use crate as cubecl;

use cubecl::prelude::*;

#[cube(launch)]
pub fn kernel_absolute_pos(output1: &mut Array<UInt>, output2: &mut Array<UInt>) {
    if ABSOLUTE_POS >= output1.len() {
        return;
    }

    output1[ABSOLUTE_POS] = ABSOLUTE_POS;
    output2[ABSOLUTE_POS] = ABSOLUTE_POS;
}

pub fn test_kernel_topology_absolute_pos<R: Runtime>(client: ComputeClient<R::Server, R::Channel>) {
    let cube_count = (3, 5, 7);
    let cube_dim = (16, 16, 1);
    let extra: u32 = 3u32;

    let length =
        (cube_count.0 * cube_count.1 * cube_count.2 * cube_dim.0 * cube_dim.1 * cube_dim.2) + extra;
    let handle1 = client.empty(length as usize * core::mem::size_of::<u32>());
    let handle2 = client.empty(length as usize * core::mem::size_of::<u32>());

    unsafe {
        kernel_absolute_pos::launch::<R>(
            &client,
            CubeCount::Static(cube_count.0, cube_count.1, cube_count.2),
            CubeDim::new(cube_dim.0, cube_dim.1, cube_dim.2),
            ArrayArg::from_raw_parts(&handle1, length as usize, 1),
            ArrayArg::from_raw_parts(&handle2, length as usize, 1),
        )
    };

    let actual = client.read(handle1.binding());
    let actual = u32::from_bytes(&actual);
    let mut expect: Vec<u32> = (0..length - extra).collect();
    expect.push(0);
    expect.push(0);
    expect.push(0);

    assert_eq!(actual, &expect);
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! testgen_topology {
    () => {
        use super::*;

        #[test]
        fn test_topology_scalar() {
            let client = TestRuntime::client(&Default::default());
            cubecl_core::runtime_tests::topology::test_kernel_topology_absolute_pos::<TestRuntime>(
                client,
            );
        }
    };
}
