use cubecl_runtime::storage::{ComputeStorage, StorageHandle, StorageId, StorageUtilization};
use cudarc::driver::sys::CUstream;
use std::collections::HashMap;

/// Buffer storage for cuda.
pub struct CudaStorage {
    memory: HashMap<StorageId, cudarc::driver::sys::CUdeviceptr>,
    deallocations: Vec<StorageId>,
    stream: cudarc::driver::sys::CUstream,
    activate_slices: HashMap<ActiveResource, cudarc::driver::sys::CUdeviceptr>,
}

#[derive(new, Debug, Hash, PartialEq, Eq, Clone)]
struct ActiveResource {
    ptr: u64,
    kind: CudaResourceKind,
}

unsafe impl Send for CudaStorage {}

impl core::fmt::Debug for CudaStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("CudaStorage {{ device: {:?} }}", self.stream).as_str())
    }
}

/// Keeps actual wgpu buffer references in a hashmap with ids as key.
impl CudaStorage {
    /// Create a new storage on the given [device](wgpu::Device).
    pub fn new(stream: CUstream) -> Self {
        Self {
            memory: HashMap::new(),
            deallocations: Vec::new(),
            stream,
            activate_slices: HashMap::new(),
        }
    }

    /// Actually deallocates buffers tagged to be deallocated.
    pub fn perform_deallocations(&mut self) {
        for id in self.deallocations.drain(..) {
            if let Some(ptr) = self.memory.remove(&id) {
                unsafe {
                    cudarc::driver::result::free_async(ptr, self.stream).unwrap();
                }
            }
        }
    }

    pub fn flush(&mut self) {
        self.activate_slices.clear();
    }
}

/// The memory resource that can be allocated for wgpu.
#[derive(new, Debug)]
pub struct CudaResource {
    /// The wgpu buffer.
    pub ptr: u64,
    pub binding: *mut std::ffi::c_void,
    /// How the resource is used.
    pub kind: CudaResourceKind,
}

unsafe impl Send for CudaResource {}

pub type Binding = *mut std::ffi::c_void;

impl CudaResource {
    /// Return the binding view of the buffer.
    pub fn as_binding(&self) -> Binding {
        match self.kind {
            CudaResourceKind::Full { .. } => self.binding,
            CudaResourceKind::Slice { .. } => self.binding,
        }
    }

    /// Return the buffer size.
    pub fn size(&self) -> u64 {
        match self.kind {
            CudaResourceKind::Full { size } => size as u64,
            CudaResourceKind::Slice { size, offset: _ } => size as u64,
        }
    }

    /// Return the buffer offset.
    pub fn offset(&self) -> u64 {
        match self.kind {
            CudaResourceKind::Full { size: _ } => 0,
            CudaResourceKind::Slice { size: _, offset } => offset as u64,
        }
    }
}

/// How the resource is used, either as a slice or fully.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum CudaResourceKind {
    /// Represents an entire buffer.
    Full { size: usize },
    /// A slice over a buffer.
    Slice { size: usize, offset: usize },
}

impl ComputeStorage for CudaStorage {
    type Resource = CudaResource;

    fn get(&mut self, handle: &StorageHandle) -> Self::Resource {
        let ptr = self.memory.get(&handle.id).unwrap();

        match handle.utilization {
            StorageUtilization::Full(size) => CudaResource::new(
                *ptr,
                ptr as *const cudarc::driver::sys::CUdeviceptr as *mut std::ffi::c_void,
                CudaResourceKind::Full { size },
            ),
            StorageUtilization::Slice { offset, size } => {
                let ptr = ptr + offset as u64;
                let kind = CudaResourceKind::Slice { size, offset };
                let key = ActiveResource::new(ptr, kind.clone());

                self.activate_slices.insert(key.clone(), ptr);

                // The ptr needs to stay alive until we send the task to the server.
                let ptr = self.activate_slices.get(&key).unwrap();

                CudaResource::new(
                    *ptr,
                    ptr as *const cudarc::driver::sys::CUdeviceptr as *mut std::ffi::c_void,
                    kind,
                )
            }
        }
    }

    fn alloc(&mut self, size: usize) -> StorageHandle {
        let id = StorageId::new();
        let ptr = unsafe { cudarc::driver::result::malloc_async(self.stream, size).unwrap() };
        self.memory.insert(id, ptr);
        StorageHandle::new(id, StorageUtilization::Full(size))
    }

    fn dealloc(&mut self, id: StorageId) {
        self.deallocations.push(id);
    }
}
