use crate::{
    compiler::wgsl,
    compute::{WgpuServer, WgpuStorage},
    AutoGraphicsApi, GraphicsApi, WgpuDevice,
};
use alloc::sync::Arc;
use cubecl_core::{Feature, FeatureSet, Runtime};
use cubecl_runtime::{
    channel::MutexComputeChannel,
    client::ComputeClient,
    memory_management::dynamic::{DynamicMemoryManagement, DynamicMemoryManagementOptions},
    ComputeRuntime,
};
use wgpu::DeviceDescriptor;

/// Runtime that uses the [wgpu] crate with the wgsl compiler. This is used in the Wgpu backend.
/// For advanced configuration, use [`init_sync`] to pass in runtime options or to select a
/// specific graphics API.
#[derive(Debug)]
pub struct WgpuRuntime;

/// The compute instance is shared across all [wgpu runtimes](WgpuRuntime).
static RUNTIME: ComputeRuntime<WgpuDevice, Server, MutexComputeChannel<Server>> =
    ComputeRuntime::new();

type Server = WgpuServer<DynamicMemoryManagement<WgpuStorage>>;

impl Runtime for WgpuRuntime {
    type Compiler = wgsl::WgslCompiler;
    type Server = WgpuServer<DynamicMemoryManagement<WgpuStorage>>;

    type Channel = MutexComputeChannel<WgpuServer<DynamicMemoryManagement<WgpuStorage>>>;
    type Device = WgpuDevice;

    fn client(device: &Self::Device) -> ComputeClient<Self::Server, Self::Channel> {
        RUNTIME.client(device, move || {
            let (adapter, device_wgpu, queue) =
                pollster::block_on(create_wgpu_setup::<AutoGraphicsApi>(device));
            create_client(adapter, device_wgpu, queue, RuntimeOptions::default())
        })
    }

    fn name() -> &'static str {
        "wgpu"
    }
}

/// The values that control how a WGPU Runtime will perform its calculations.
pub struct RuntimeOptions {
    /// Control the amount of compute tasks to be aggregated into a single GPU command.
    pub tasks_max: usize,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        #[cfg(test)]
        const DEFAULT_MAX_TASKS: usize = 1;
        #[cfg(not(test))]
        const DEFAULT_MAX_TASKS: usize = 16;

        let tasks_max = match std::env::var("CUBECL_WGPU_MAX_TASKS") {
            Ok(value) => value
                .parse::<usize>()
                .expect("CUBECL_WGPU_MAX_TASKS should be a positive integer."),
            Err(_) => DEFAULT_MAX_TASKS,
        };

        Self { tasks_max }
    }
}

pub fn init_existing_device(
    adapter: Arc<wgpu::Adapter>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    options: RuntimeOptions,
) -> WgpuDevice {
    let device_id = WgpuDevice::Existing(device.as_ref().global_id());
    let client = create_client(adapter, device, queue, options);
    RUNTIME.register(&device_id, client);
    device_id
}

/// Initialize a client on the given device with the given options. This function is useful to configure the runtime options
/// or to pick a different graphics API. On wasm, it is necessary to use [`init_async`] instead.
pub fn init_sync<G: GraphicsApi>(device: &WgpuDevice, options: RuntimeOptions) {
    pollster::block_on(init_async::<G>(device, options));
}

/// Like [`init_sync`], but async, necessary for wasm.
pub async fn init_async<G: GraphicsApi>(device: &WgpuDevice, options: RuntimeOptions) {
    let (adapter, device_wgpu, queue) = create_wgpu_setup::<G>(device).await;
    let client = create_client(adapter, device_wgpu, queue, options);
    RUNTIME.register(device, client)
}

async fn create_wgpu_setup<G: GraphicsApi>(
    device: &WgpuDevice,
) -> (Arc<wgpu::Adapter>, Arc<wgpu::Device>, Arc<wgpu::Queue>) {
    let (device_wgpu, queue, adapter) = select_device::<G>(device).await;

    log::info!(
        "Created wgpu compute server on device {:?} => {:?}",
        device,
        adapter.get_info()
    );
    (Arc::new(adapter), Arc::new(device_wgpu), Arc::new(queue))
}

fn create_client(
    adapter: Arc<wgpu::Adapter>,
    device_wgpu: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    options: RuntimeOptions,
) -> ComputeClient<
    WgpuServer<DynamicMemoryManagement<WgpuStorage>>,
    MutexComputeChannel<WgpuServer<DynamicMemoryManagement<WgpuStorage>>>,
> {
    let limits = device_wgpu.limits();
    let storage = WgpuStorage::new(device_wgpu.clone());
    let memory_management = DynamicMemoryManagement::new(
        storage,
        DynamicMemoryManagementOptions::preset(
            limits.max_storage_buffer_binding_size as usize,
            limits.min_storage_buffer_offset_alignment as usize,
        ),
    );
    let server = WgpuServer::new(memory_management, device_wgpu, queue, options.tasks_max);
    let channel = MutexComputeChannel::new(server);

    let features = adapter.features();
    let mut features_cube = FeatureSet::default();

    if features.contains(wgpu::Features::SUBGROUP) {
        features_cube.register(Feature::Subcube);
    }

    ComputeClient::new(channel, Arc::new(features_cube))
}

/// Select the wgpu device and queue based on the provided [device](WgpuDevice).
pub async fn select_device<G: GraphicsApi>(
    device: &WgpuDevice,
) -> (wgpu::Device, wgpu::Queue, wgpu::Adapter) {
    #[cfg(target_family = "wasm")]
    let adapter = select_adapter::<G>(device).await;

    #[cfg(not(target_family = "wasm"))]
    let adapter = select_adapter::<G>(device);
    let limits = adapter.limits();

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: adapter.features(),
                required_limits: limits,
                // The default is MemoryHints::Performance, which tries to do some bigger
                // block allocations. However, we already batch allocations, so we
                // can use MemoryHints::MemoryUsage to lower memory usage.
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        )
        .await
        .map_err(|err| {
            format!(
                "Unable to request the device with the adapter {:?}, err {:?}",
                adapter.get_info(),
                err
            )
        })
        .unwrap();

    (device, queue, adapter)
}

#[cfg(target_family = "wasm")]
async fn select_adapter<G: GraphicsApi>(_device: &WgpuDevice) -> wgpu::Adapter {
    let instance = wgpu::Instance::default();

    instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
        .await
        .unwrap()
}

#[cfg(not(target_family = "wasm"))]
fn select_adapter<G: GraphicsApi>(device: &WgpuDevice) -> wgpu::Adapter {
    use wgpu::DeviceType;

    let instance = wgpu::Instance::default();
    let mut adapters_other = Vec::new();
    let mut adapters = Vec::new();

    instance
        .enumerate_adapters(G::backend().into())
        .into_iter()
        .for_each(|adapter| {
            let device_type = adapter.get_info().device_type;

            if let DeviceType::Other = device_type {
                adapters_other.push(adapter);
                return;
            }

            let is_same_type = match device {
                WgpuDevice::DiscreteGpu(_) => device_type == DeviceType::DiscreteGpu,
                WgpuDevice::IntegratedGpu(_) => device_type == DeviceType::IntegratedGpu,
                WgpuDevice::VirtualGpu(_) => device_type == DeviceType::VirtualGpu,
                WgpuDevice::Cpu => device_type == DeviceType::Cpu,
                WgpuDevice::BestAvailable => true,
                WgpuDevice::Existing(_) => {
                    unreachable!("Cannot select an adapter for an existing device.")
                }
            };

            if is_same_type {
                adapters.push(adapter);
            }
        });

    fn select(
        num: usize,
        error: &str,
        mut adapters: Vec<wgpu::Adapter>,
        mut adapters_other: Vec<wgpu::Adapter>,
    ) -> wgpu::Adapter {
        if adapters.len() <= num {
            if adapters_other.len() <= num {
                panic!(
                    "{}, adapters {:?}, other adapters {:?}",
                    error,
                    adapters
                        .into_iter()
                        .map(|adapter| adapter.get_info())
                        .collect::<Vec<_>>(),
                    adapters_other
                        .into_iter()
                        .map(|adapter| adapter.get_info())
                        .collect::<Vec<_>>(),
                );
            }

            return adapters_other.remove(num);
        }

        adapters.remove(num)
    }

    let adapter = match device {
        WgpuDevice::DiscreteGpu(num) => select(
            *num,
            "No Discrete GPU device found",
            adapters,
            adapters_other,
        ),
        WgpuDevice::IntegratedGpu(num) => select(
            *num,
            "No Integrated GPU device found",
            adapters,
            adapters_other,
        ),
        WgpuDevice::VirtualGpu(num) => select(
            *num,
            "No Virtual GPU device found",
            adapters,
            adapters_other,
        ),
        WgpuDevice::Cpu => select(0, "No CPU device found", adapters, adapters_other),
        WgpuDevice::BestAvailable => {
            let mut most_performant_adapter = None;
            let mut current_score = -1;

            adapters
                .into_iter()
                .chain(adapters_other)
                .for_each(|adapter| {
                    let info = adapter.get_info();
                    let score = match info.device_type {
                        DeviceType::DiscreteGpu => 5,
                        DeviceType::Other => 4, // Let's be optimistic with the Other device, it's
                        // often a Discrete Gpu.
                        DeviceType::IntegratedGpu => 3,
                        DeviceType::VirtualGpu => 2,
                        DeviceType::Cpu => 1,
                    };

                    if score > current_score {
                        most_performant_adapter = Some(adapter);
                        current_score = score;
                    }
                });

            if let Some(adapter) = most_performant_adapter {
                adapter
            } else {
                panic!("No adapter found for graphics API {:?}", G::default());
            }
        }
        WgpuDevice::Existing(_) => unreachable!("Cannot select an adapter for an existing device."),
    };

    log::info!("Using adapter {:?}", adapter.get_info());

    adapter
}
