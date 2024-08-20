mod dummy;

use std::sync::Arc;

use crate::dummy::autotune_execute;
use crate::dummy::TEST_TUNER;
use crate::dummy::{client, DummyDevice, DummyElementwiseAddition};

#[cfg(autotune_persistent_cache)]
use crate::dummy::{TUNER_DEVICE_ID, TUNER_PREFIX};

use cubecl_runtime::ComputeRuntime;

#[allow(unused)]
use serial_test::serial;

type Runtime = ComputeRuntime<DummyDevice, dummy::DummyServer, dummy::DummyChannel>;

#[test]
fn created_resource_is_the_same_when_read() {
    let client = client(&DummyDevice);
    let resource = Vec::from([0, 1, 2]);
    let resource_description = client.create(&resource);

    let obtained_resource = client.read(resource_description.binding());

    assert_eq!(resource, obtained_resource)
}

#[test]
fn empty_allocates_memory() {
    let client = client(&DummyDevice);
    let size = 4;
    let resource_description = client.empty(size);
    let empty_resource = client.read(resource_description.binding());

    assert_eq!(empty_resource.len(), 4);
}

#[test]
fn execute_elementwise_addition() {
    let client = client(&DummyDevice);
    let lhs = client.create(&[0, 1, 2]);
    let rhs = client.create(&[4, 4, 4]);
    let out = client.empty(3);

    client.execute(
        Arc::new(DummyElementwiseAddition),
        (),
        vec![lhs.binding(), rhs.binding(), out.clone().binding()],
    );

    let obtained_resource = client.read(out.binding());

    assert_eq!(obtained_resource, Vec::from([4, 5, 6]))
}

#[test]
#[serial]
#[cfg(feature = "std")]
fn autotune_basic_addition_execution() {
    TEST_TUNER.clear();
    let client = client(&DummyDevice);

    let shapes = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs = client.create(&[0, 1, 2]);
    let rhs = client.create(&[4, 4, 4]);
    let out = client.empty(3);
    let handles = vec![lhs.binding(), rhs.binding(), out.clone().binding()];

    let addition_autotune_kernel =
        dummy::AdditionAutotuneOperationSet::new(client.clone(), shapes, handles);
    autotune_execute(&client, Box::new(addition_autotune_kernel));

    let obtained_resource = client.read(out.binding());

    // If slow kernel was selected it would output [0, 1, 2]
    assert_eq!(obtained_resource, Vec::from([4, 5, 6]));
}

#[test]
#[serial]
#[cfg(feature = "std")]
fn autotune_basic_multiplication_execution() {
    TEST_TUNER.clear();
    let client = client(&DummyDevice);

    let shapes = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs = client.create(&[0, 1, 2]);
    let rhs = client.create(&[4, 4, 4]);
    let out = client.empty(3);
    let handles = vec![lhs.binding(), rhs.binding(), out.clone().binding()];

    let multiplication_autotune_kernel =
        dummy::MultiplicationAutotuneOperationSet::new(client.clone(), shapes, handles);
    autotune_execute(&client, Box::new(multiplication_autotune_kernel));

    let obtained_resource = client.read(out.binding());

    // If slow kernel was selected it would output [0, 1, 2]
    assert_eq!(obtained_resource, Vec::from([0, 4, 8]));
}

#[test]
#[serial]
#[cfg(feature = "std")]
fn autotune_cache_same_key_return_a_cache_hit() {
    TEST_TUNER.clear();
    let runtime = Runtime::new();

    let client = runtime.client(&DummyDevice, dummy::init_client);

    // note: the key name depends on the shapes of the operation set
    // see log_shape_input_key for more info.

    // in this test both shapes [1,3] and [1,4] end up with the same key name
    // which is 'cache_test-1,4'
    let shapes_1 = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs_1 = client.create(&[0, 1, 2]);
    let rhs_1 = client.create(&[4, 4, 4]);
    let out_1 = client.empty(3);
    let handles_1 = vec![lhs_1.binding(), rhs_1.binding(), out_1.binding()];

    let shapes_2 = vec![vec![1, 4], vec![1, 4], vec![1, 4]];
    let lhs_2 = client.create(&[0, 1, 2, 3]);
    let rhs_2 = client.create(&[5, 6, 7, 8]);
    let out_2 = client.empty(4);
    let handles_2 = vec![lhs_2.binding(), rhs_2.binding(), out_2.clone().binding()];

    let cache_test_autotune_kernel_1 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_1, handles_1);
    let cache_test_autotune_kernel_2 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_2, handles_2);
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_1));
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_2));

    let obtained_resource = client.read(out_2.binding());

    // Cache should be hit, so CacheTestFastOn3 should be used, returning lhs
    assert_eq!(obtained_resource, Vec::from([0, 1, 2, 3]));
}

#[test]
#[serial]
#[cfg(autotune_persistent_cache)]
fn autotune_cache_no_cache_on_disk_return_a_cache_miss() {
    TEST_TUNER.clear();

    // delete the cache file
    let file_path = cubecl_runtime::tune::get_persistent_cache_file_path(&format!(
        "{}/{}",
        TUNER_PREFIX, TUNER_DEVICE_ID
    ));
    let _ = std::fs::remove_file(file_path);

    let compute = Runtime::new();

    let client = compute.client(&DummyDevice, dummy::init_client);

    // in this test shapes [1,3] and [1,5] ends up with different key names
    // which are 'cache_test-1,4' and 'cache_test-1,8'
    let shapes_1 = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs_1 = client.create(&[0, 1, 2]);
    let rhs_1 = client.create(&[4, 4, 4]);
    let out_1 = client.empty(3);
    let handles_1 = vec![lhs_1.binding(), rhs_1.binding(), out_1.binding()];

    let shapes_2 = vec![vec![1, 5], vec![1, 5], vec![1, 5]];
    let lhs_2 = client.create(&[0, 1, 2, 3, 4]);
    let rhs_2 = client.create(&[5, 6, 7, 8, 9]);
    let out_2 = client.empty(5);
    let handles_2 = vec![lhs_2.binding(), rhs_2.binding(), out_2.clone().binding()];

    let cache_test_autotune_kernel_1 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_1, handles_1);
    let cache_test_autotune_kernel_2 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_2, handles_2);
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_1));
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_2));

    // read the resource which should update the cache on disk
    let obtained_resource = client.read(out_2.binding());

    // Cache should be missed, so CacheTestSlowOn3 (but faster on 5) should be used, returning rhs
    assert_eq!(obtained_resource, Vec::from([5, 6, 7, 8, 9]));
}

#[test]
#[serial]
#[cfg(autotune_persistent_cache)]
fn autotune_cache_file_path_creation_works_when_path_does_not_exist_yet() {
    TEST_TUNER.clear();
    // delete the cache file

    let file_path = cubecl_runtime::tune::get_persistent_cache_file_path(&format!(
        "{}/{}",
        TUNER_PREFIX, TUNER_DEVICE_ID
    ));
    let parent_dir = file_path
        .parent()
        .expect("Cache file should have a parent directory");
    // Delete the cache file's parent directory
    let _ = std::fs::remove_dir_all(parent_dir);

    let runtime = Runtime::new();
    let client = runtime.client(&DummyDevice, dummy::init_client);

    // in this test shapes [1,3] and [1,5] ends up with different key names
    // which are 'cache_test-1,4' and 'cache_test-1,8'
    let shapes = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs = client.create(&[0, 1, 2]);
    let rhs = client.create(&[4, 4, 4]);
    let out = client.empty(3);
    let handles = vec![lhs.binding(), rhs.binding(), out.clone().binding()];

    let cache_test_autotune_kernel =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes, handles);
    autotune_execute(&client, Box::new(cache_test_autotune_kernel));

    assert!(
        parent_dir.exists(),
        "Parent directory of the cache file should exist"
    );
    assert!(file_path.exists(), "Cache file should exist");
}

#[test]
#[serial]
#[cfg(feature = "std")]
fn autotune_cache_different_keys_return_a_cache_miss() {
    TEST_TUNER.clear();
    let client = client(&DummyDevice);

    // in this test shapes [1,3] and [1,5] ends up with different key names
    // which are 'cache_test-1,4' and 'cache_test-1,8'
    let shapes_1 = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs_1 = client.create(&[0, 1, 2]);
    let rhs_1 = client.create(&[4, 4, 4]);
    let out_1 = client.empty(3);
    let handles_1 = vec![lhs_1.binding(), rhs_1.binding(), out_1.binding()];

    let shapes_2 = vec![vec![1, 5], vec![1, 5], vec![1, 5]];
    let lhs_2 = client.create(&[0, 1, 2, 3, 4]);
    let rhs_2 = client.create(&[5, 6, 7, 8, 9]);
    let out_2 = client.empty(5);
    let handles_2 = vec![lhs_2.binding(), rhs_2.binding(), out_2.clone().binding()];

    let cache_test_autotune_kernel_1 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_1, handles_1);
    let cache_test_autotune_kernel_2 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_2, handles_2);
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_1));
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_2));

    let obtained_resource = client.read(out_2.binding());

    // Cache should be missed, so CacheTestSlowOn3 (but faster on 5) should be used, returning rhs
    assert_eq!(obtained_resource, Vec::from([5, 6, 7, 8, 9]));
}

#[test]
#[serial]
#[cfg(feature = "std")]
fn autotune_cache_different_checksums_return_a_cache_miss() {
    TEST_TUNER.clear();

    let runtime = Runtime::new();
    let client = runtime.client(&DummyDevice, dummy::init_client);

    // in this test both shapes [1,3] and [1,4] end up with the same key name
    // which is 'cache_test-1,4'
    let shapes_1 = vec![vec![1, 3], vec![1, 3], vec![1, 3]];
    let lhs_1 = client.create(&[0, 1, 2]);
    let rhs_1 = client.create(&[4, 4, 4]);
    let out_1 = client.empty(3);
    let handles_1 = vec![lhs_1.binding(), rhs_1.binding(), out_1.binding()];
    let cache_test_autotune_kernel_1 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_1, handles_1);
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_1));

    TEST_TUNER.clear();

    let shapes_2 = vec![vec![1, 4], vec![1, 4], vec![1, 4]];
    let lhs_2 = client.create(&[0, 1, 2, 3]);
    let rhs_2 = client.create(&[5, 6, 7, 8]);
    let out_2 = client.empty(4);
    let handles_2 = vec![lhs_2.binding(), rhs_2.binding(), out_2.clone().binding()];

    let mut cache_test_autotune_kernel_2 =
        dummy::CacheTestAutotuneOperationSet::new(client.clone(), shapes_2, handles_2);
    cache_test_autotune_kernel_2.generate_random_checksum = true;
    autotune_execute(&client, Box::new(cache_test_autotune_kernel_2));

    let obtained_resource = client.read(out_2.binding());

    // Cache should be missed because the checksum on 4 is generated randomly
    // and thus is always different,
    // so CacheTestSlowOn3 (but faster on 4) should be used, returning rhs
    assert_eq!(obtained_resource, Vec::from([5, 6, 7, 8]));
}
