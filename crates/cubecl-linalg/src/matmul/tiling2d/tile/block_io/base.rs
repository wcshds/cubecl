use cubecl_core as cubecl;
use cubecl_core::prelude::*;

use crate::matmul::tiling2d::config::CubeTiling2dConfig;
use crate::matmul::tiling2d::tile::loader::{CheckBounds, ReadTileInfo};
use crate::matmul::tiling2d::tile::memory_access::ContiguousAccess;
use crate::matmul::tiling2d::write_output::WriteTileInfo;

#[cube]
pub(crate) trait BlockLoader<F: Float>: Send + Sync + 'static {
    fn load_tile_plain<A: ContiguousAccess<F>>(
        tensor: &Tensor<F>,
        shared_memory: &mut SharedMemory<F>,
        read_tile_info: ReadTileInfo,
        config: Comptime<CubeTiling2dConfig>,
        check_bounds: CheckBounds,
    );

    fn load_tile_transposed(
        tensor: &Tensor<F>,
        shared_memory: &mut SharedMemory<F>,
        read_tile_info: ReadTileInfo,
        config: Comptime<CubeTiling2dConfig>,
        check_bounds: CheckBounds,
    );
}

#[cube]
pub(crate) trait BlockWriter<F: Float>: Send + Sync + 'static {
    fn write_output<A: ContiguousAccess<F>>(
        out: &mut Tensor<F>,
        results: &Array<F>,
        write_tile_info: WriteTileInfo,
        config: Comptime<CubeTiling2dConfig>,
        check_bounds: CheckBounds,
    );
}

#[cube]
pub(crate) fn all_zeros_runtime<F: Float>(
    shared_memory: &mut SharedMemory<F>,
    start: UInt,
    sm_position_base: UInt,
    sm_stride: UInt,
    config: Comptime<CubeTiling2dConfig>,
) {
    let tile_size = Comptime::map(config, |c| c.tile_size);
    let zeros = F::vectorized(0., Comptime::get(tile_size));

    for i in range(start, Comptime::get(tile_size), Comptime::new(false)) {
        let sm_position = (sm_position_base + i * sm_stride) / Comptime::runtime(tile_size);

        shared_memory[sm_position] = zeros;
    }
}

#[cube]
pub(crate) fn all_zeros_comptime<F: Float>(
    shared_memory: &mut SharedMemory<F>,
    sm_position_base: UInt,
    sm_stride: UInt,
    config: Comptime<CubeTiling2dConfig>,
) {
    let tile_size = Comptime::map(config, |c| c.tile_size);
    let unroll = Comptime::map(config, |c| c.unroll_tile);
    let zeros = F::vectorized(0., Comptime::get(tile_size));

    for i in range(0u32, Comptime::get(tile_size), unroll) {
        let sm_position = (sm_position_base + i * sm_stride) / Comptime::runtime(tile_size);

        shared_memory[sm_position] = zeros;
    }
}
