use std::sync::Arc;

use cubecl_common::{reader::reader_from_concrete, sync_type::SyncType};
use cubecl_runtime::storage::ComputeStorage;
use cubecl_runtime::{
    memory_management::{simple::SimpleMemoryManagement, MemoryManagement},
    server::{Binding, ComputeServer, Handle},
    storage::{BytesResource, BytesStorage},
    ExecutionMode,
};
use derive_new::new;

use super::DummyKernel;

/// The dummy server is used to test the cubecl-runtime infrastructure.
/// It uses simple memory management with a bytes storage on CPU, without asynchronous tasks.
#[derive(new, Debug)]
pub struct DummyServer<MM = SimpleMemoryManagement<BytesStorage>> {
    memory_management: MM,
}

impl<MM> ComputeServer for DummyServer<MM>
where
    MM: MemoryManagement<BytesStorage>,
{
    type DispatchOptions = ();
    type Kernel = Arc<dyn DummyKernel>;
    type Storage = BytesStorage;
    type MemoryManagement = MM;
    type FeatureSet = ();

    fn read(&mut self, binding: Binding<Self>) -> cubecl_common::reader::Reader {
        let bytes_handle = self.memory_management.get(binding.memory);
        let bytes = self.memory_management.storage().get(&bytes_handle);
        reader_from_concrete(bytes.read().to_vec())
    }

    fn get_resource(&mut self, binding: Binding<Self>) -> BytesResource {
        let handle = self.memory_management.get(binding.memory);
        self.memory_management.storage().get(&handle)
    }

    fn create(&mut self, data: &[u8]) -> Handle<Self> {
        let handle = self.empty(data.len());
        let resource = self.get_resource(handle.clone().binding());

        let bytes = resource.write();

        for (i, val) in data.iter().enumerate() {
            bytes[i] = *val;
        }

        handle
    }

    fn empty(&mut self, size: usize) -> Handle<Self> {
        Handle::new(self.memory_management.reserve(size, &[]))
    }

    unsafe fn execute(
        &mut self,
        kernel: Self::Kernel,
        _count: Self::DispatchOptions,
        bindings: Vec<Binding<Self>>,
        _mode: ExecutionMode,
    ) {
        let mut resources = bindings
            .into_iter()
            .map(|binding| self.get_resource(binding))
            .collect::<Vec<_>>();

        kernel.compute(&mut resources);
    }

    fn sync(&mut self, _: SyncType) {
        // Nothing to do with dummy backend.
    }
}
