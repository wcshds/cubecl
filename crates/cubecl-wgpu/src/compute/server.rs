use std::num::NonZero;

use super::WgpuStorage;
use alloc::{borrow::Cow, sync::Arc};
use cubecl_common::{reader::Reader, sync_type::SyncType};
use cubecl_core::{compute::DebugInformation, prelude::*, server::Handle, FeatureSet, KernelId};
use cubecl_runtime::{
    debug::DebugLogger,
    memory_management::MemoryManagement,
    server::{self, ComputeServer},
    storage::{ComputeStorage, StorageId},
    ExecutionMode,
};
use hashbrown::HashMap;
use wgpu::{CommandEncoder, ComputePass, ComputePipeline, ShaderModuleDescriptor};

/// Wgpu compute server.
#[derive(Debug)]
pub struct WgpuServer<MM: MemoryManagement<WgpuStorage>> {
    memory_management: MM,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    encoder: CommandEncoder,
    current_pass: Option<ComputePass<'static>>,
    tasks_count: usize,
    compute_storage_used: Vec<StorageId>,
    pipelines: HashMap<KernelId, Arc<ComputePipeline>>,
    tasks_max: usize,
    logger: DebugLogger,
}

fn create_encoder(device: &wgpu::Device) -> CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("CubeCL Command Encoder"),
    })
}

impl<MM> WgpuServer<MM>
where
    MM: MemoryManagement<WgpuStorage>,
{
    /// Create a new server.
    pub fn new(
        memory_management: MM,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        tasks_max: usize,
    ) -> Self {
        Self {
            memory_management,
            device: device.clone(),
            queue: queue.clone(),
            encoder: create_encoder(&device),
            current_pass: None,
            tasks_count: 0,
            compute_storage_used: Vec::new(),
            pipelines: HashMap::new(),
            tasks_max,
            logger: DebugLogger::new(),
        }
    }

    fn pipeline(
        &mut self,
        kernel: <Self as ComputeServer>::Kernel,
        mode: ExecutionMode,
    ) -> Arc<ComputePipeline> {
        let mut kernel_id = kernel.id();
        kernel_id.mode(mode);

        if let Some(pipeline) = self.pipelines.get(&kernel_id) {
            return pipeline.clone();
        }

        let mut compile = kernel.compile(mode);
        if self.logger.is_activated() {
            compile.debug_info = Some(DebugInformation::new("wgsl", kernel_id.clone()));
        }

        let compile = self.logger.debug(compile);
        let pipeline = self.compile_source(&compile.source, mode);

        self.pipelines.insert(kernel_id.clone(), pipeline.clone());

        pipeline
    }

    fn compile_source(&self, source: &str, mode: ExecutionMode) -> Arc<ComputePipeline> {
        let module = match mode {
            ExecutionMode::Checked => self.device.create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
            }),
            ExecutionMode::Unchecked => unsafe {
                self.device
                    .create_shader_module_unchecked(ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
                    })
            },
        };

        Arc::new(
            self.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: None,
                    module: &module,
                    entry_point: "main",
                    compilation_options: Default::default(),
                    cache: None,
                }),
        )
    }

    fn clear_compute_pass(&mut self) {
        self.current_pass = None;
    }
}

impl<MM> ComputeServer for WgpuServer<MM>
where
    MM: MemoryManagement<WgpuStorage>,
{
    type Kernel = Box<dyn CubeTask>;
    type DispatchOptions = CubeCount<Self>;
    type Storage = WgpuStorage;
    type MemoryManagement = MM;
    type FeatureSet = FeatureSet;

    fn read(&mut self, binding: server::Binding<Self>) -> Reader {
        let resource = self.get_resource(binding);

        let size = resource.size();
        let read_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.clear_compute_pass();

        self.encoder.copy_buffer_to_buffer(
            &resource.buffer,
            resource.offset(),
            &read_buffer,
            0,
            size,
        );

        // Flush all commands to the queue, so GPU gets started on copying to the staging buffer.
        self.sync(SyncType::Flush);

        let (sender, receiver) = async_channel::bounded(1);
        let slice = read_buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, move |v| {
            sender
                .try_send(v)
                .expect("Unable to send buffer slice result to async channel.");
        });

        let device = self.device.clone();

        Box::pin(async move {
            // Now wait for the GPU to finish.
            device.poll(wgpu::Maintain::Wait);

            let slice = read_buffer.slice(..);

            receiver
                .recv()
                .await
                .expect("Unable to receive buffer slice result.")
                .expect("Failed to map buffer");

            let data = slice.get_mapped_range();
            let result = bytemuck::cast_slice(&data).to_vec();

            drop(data);
            read_buffer.unmap();

            result
        })
    }

    fn get_resource(
        &mut self,
        binding: server::Binding<Self>,
    ) -> <Self::Storage as cubecl_runtime::storage::ComputeStorage>::Resource {
        let handle = self.memory_management.get(binding.memory);
        self.memory_management.storage().get(&handle)
    }

    /// When we create a new handle from existing data, we use custom allocations so that we don't
    /// have to execute the current pending tasks.
    ///
    /// This is important, otherwise the compute passes are going to be too small and we won't be able to
    /// fully utilize the GPU.
    fn create(&mut self, data: &[u8]) -> server::Handle<Self> {
        // Reserve memory on some storage we haven't yet used this command queue.
        let memory = self
            .memory_management
            .reserve(data.len(), &self.compute_storage_used);

        let handle = Handle::new(memory);

        if let Some(len) = NonZero::new(data.len() as u64) {
            let resource = self.get_resource(handle.clone().binding());

            // Write to the staging buffer. Next queue submission this will copy the data to the GPU.
            self.queue
                .write_buffer_with(&resource.buffer, resource.offset(), len)
                .expect("Failed to write to staging buffer.")
                .copy_from_slice(data);
        }

        handle
    }

    fn empty(&mut self, size: usize) -> server::Handle<Self> {
        server::Handle::new(self.memory_management.reserve(size, &[]))
    }

    unsafe fn execute(
        &mut self,
        kernel: Self::Kernel,
        count: Self::DispatchOptions,
        bindings: Vec<server::Binding<Self>>,
        mode: ExecutionMode,
    ) {
        let pipeline = self.pipeline(kernel, mode);
        let group_layout = pipeline.get_bind_group_layout(0);

        // Store all the resources we'll be using. This could be eliminated if
        // there was a way to tie the lifetime of the resource to the memory handle.
        let resources: Vec<_> = bindings
            .iter()
            .map(|binding| {
                let resource_handle = self.memory_management.get(binding.memory.clone());
                // Keep track of the storage we've used so far.
                self.compute_storage_used.push(resource_handle.id);

                self.memory_management.storage().get(&resource_handle)
            })
            .collect();

        let entries = &resources
            .iter()
            .enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r.as_binding(),
            })
            .collect::<Vec<_>>();

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &group_layout,
            entries,
        });

        // First resolve the dispatch buffer if needed. The weird ordering is because the lifetime of this
        // needs to be longer than the compute pass, so we can't do this just before dispatching.
        let dispatch_resource = match count.clone() {
            CubeCount::Dynamic(binding) => Some(self.get_resource(binding)),
            _ => None,
        };

        self.tasks_count += 1;

        // Start a new compute pass if needed. The forget_lifetime allows
        // to store this with a 'static lifetime, but the compute pass must
        // be dropped before the encoder. This isn't unsafe - it's still checked at runtime.
        let pass = self.current_pass.get_or_insert_with(|| {
            self.encoder
                .begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                })
                .forget_lifetime()
        });

        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);

        match count {
            CubeCount::Static(x, y, z) => {
                pass.dispatch_workgroups(x, y, z);
            }
            CubeCount::Dynamic(_) => {
                let resource = dispatch_resource.as_ref().unwrap();
                pass.dispatch_workgroups_indirect(&resource.buffer, resource.offset());
            }
        }

        if self.tasks_count >= self.tasks_max {
            self.sync(SyncType::Flush);
        }
    }

    fn sync(&mut self, sync_type: SyncType) {
        // End the current compute pass.
        self.clear_compute_pass();
        let new_encoder = create_encoder(&self.device);
        let encoder = std::mem::replace(&mut self.encoder, new_encoder);
        self.queue.submit([encoder.finish()]);

        self.tasks_count = 0;
        self.compute_storage_used.clear();

        if sync_type == SyncType::Wait {
            self.device.poll(wgpu::Maintain::Wait);
        }

        // Cleanup allocations and deallocations.
        self.memory_management.storage().perform_deallocations();
    }
}
