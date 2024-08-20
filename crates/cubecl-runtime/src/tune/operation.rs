use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{Debug, Display};
use core::hash::Hash;

/// Default checksum for an operation set
#[cfg(autotune_persistent_cache)]
pub fn compute_checksum<Out>(autotunables: &[Box<dyn AutotuneOperation<Out>>]) -> String {
    let mut checksum = String::new();
    autotunables.iter().for_each(|op| {
        checksum += op.name();
    });
    format!("{:x}", md5::compute(checksum))
}

/// Groups operations of the same type for autotune
pub trait AutotuneOperationSet<K, Output = ()>: Send {
    /// The key used in the tune cache
    fn key(&self) -> K;

    /// All candidate operations for autotuning this operation type
    /// Operations can run on toy tensors of relevant size
    fn autotunables(&self) -> Vec<Box<dyn AutotuneOperation<Output>>>;

    /// Returns the operation for the given index, matching the order
    /// returned by autotunables. Operation obtained here runs on original tensors
    fn fastest(self: Box<Self>, fastest_index: usize) -> Box<dyn AutotuneOperation<Output>>;

    /// Compute a checksum that can invalidate outdated cached auto-tune results.
    #[cfg(autotune_persistent_cache)]
    fn compute_checksum(&self) -> String {
        compute_checksum(&self.autotunables())
    }
}

/// Contains operation to run and inputs on which to run it
pub trait AutotuneOperation<Output = ()> {
    /// Runs the operation
    fn execute(self: Box<Self>) -> Output;

    /// The name of the operation.
    fn name(&self) -> &str {
        core::any::type_name::<Self>()
    }

    /// Clones the operation and inputs
    fn clone(&self) -> Box<dyn AutotuneOperation<Output>>;
}

#[cfg(autotune_persistent_cache)]
/// Trait alias with support for persistent caching
pub trait AutotuneKey:
    Clone
    + Debug
    + PartialEq
    + Eq
    + Hash
    + Display
    + serde::Serialize
    + serde::de::DeserializeOwned
    + Send
    + Sync
{
}
#[cfg(not(autotune_persistent_cache))]
/// Trait alias
pub trait AutotuneKey: Clone + Debug + PartialEq + Eq + Hash + Display {}

impl AutotuneKey for String {}
