// Copyright (c) 2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.
/*
use crate::tiling::PlaneRegion;
use crate::cpu_features::CpuFeatureLevel;
use crate::dist::*;
use crate::partition::BlockSize;
use crate::util::*;

// Range is [0, 255^2 * 128 * 128] which fits in u32
type SseFn = unsafe extern fn(
    src: *const u8,
    src_stride: isize,
    dst: *const u8,
    dst_stride: isize,
) -> u32;

type SseHBDFn = unsafe extern fn(
    src: *const u16,
    src_stride: isize,
    dst: *const u16,
    dst_stride: isize,
) -> u64;

macro_rules! declare_asm_sse_fn {
  ($($name: ident),+) => (
    $(
      extern { fn $name (
        src: *const u8, src_stride: isize, dst: *const u8, dst_stride: isize
      ) -> u32; }
    )+
  )
}

macro_rules! declare_asm_hbd_sse_fn {
  ($($name: ident),+) => (
    $(
      extern { fn $name (
        src: *const u16, src_stride: isize, dst: *const u16, dst_stride: isize
      ) -> u64; }
    )+
  )
}

declare_asm_sse_fn![
  // AVX2
  rav1e_sse_4x4_avx2,
  rav1e_sse_8x8_avx2,

  rav1e_sse_4x8_avx2,
  rav1e_sse_8x16_avx2,

  rav1e_sse_4x16_avx2,
  rav1e_sse_8x32_avx2
];


declare_asm_hbd_sse_fn![
  // AVX2
  rav1e_sse_4x4_hbd_avx2
];

#[inline(always)]
#[allow(clippy::let_and_return)]
pub fn get_sse<T: Pixel>(
    src: &PlaneRegion<'_, T>, dst: &PlaneRegion<'_, T>, bsize: BlockSize,
    bit_depth: usize, cpu: CpuFeatureLevel,
) -> u64 {
    let call_rust =
        || -> u64 { rust::get_sse(dst, src, bsize, bit_depth, cpu) };

        #[cfg(any(feature = "check_asm", test))]
    let ref_dist = call_rust();

    let dist =
        match T::type_enum() {
            PixelType::U8 => match SSE_FNS[cpu.as_index()][to_index(bsize)] {
                Some(func) => unsafe {
                    (func)(
                        src.data_ptr() as *const _,
                        T::to_asm_stride(src.plane_cfg.stride),
                        dst.data_ptr() as *const _,
                        T::to_asm_stride(dst.plane_cfg.stride),
                    ) as u64
                },
                None => call_rust(),
            },
            PixelType::U16 => match SSE_HBD_FNS[cpu.as_index()][to_index(bsize)] {
                Some(func) => unsafe {
                    (func)(
                        src.data_ptr() as *const _,
                        T::to_asm_stride(src.plane_cfg.stride) as isize,
                        dst.data_ptr() as *const _,
                        T::to_asm_stride(dst.plane_cfg.stride) as isize,
                    )
                },
                None => call_rust(),
            },
        };

    #[cfg(any(feature = "check_asm", test))]
    assert_eq!(dist, ref_dist, "{}", bsize);

    dist
}

static SSE_FNS_AVX2: [Option<SseFn>; DIST_FNS_LENGTH] = {
    let mut out: [Option<SseFn>; DIST_FNS_LENGTH] = [None; DIST_FNS_LENGTH];

    use BlockSize::*;
    out[BLOCK_4X4 as usize] = Some(rav1e_sse_4x4_avx2);
    out[BLOCK_8X8 as usize] = Some(rav1e_sse_8x8_avx2);

    out[BLOCK_4X8 as usize] = Some(rav1e_sse_4x8_avx2);
    out[BLOCK_8X16 as usize] = Some(rav1e_sse_8x16_avx2);

    out[BLOCK_8X32 as usize] = Some(rav1e_sse_8x32_avx2);
    out[BLOCK_4X16 as usize] = Some(rav1e_sse_4x16_avx2);

    out
};

static SSE_HBD_FNS_AVX2: [Option<SseHBDFn>; DIST_FNS_LENGTH] = {
    let mut out: [Option<SseHBDFn>; DIST_FNS_LENGTH] = [None; DIST_FNS_LENGTH];

    use BlockSize::*;
    out[BLOCK_4X4 as usize] = Some(rav1e_sse_4x4_hbd_avx2);

    out
};

cpu_function_lookup_table!(
  SSE_FNS: [[Option<SseFn>; DIST_FNS_LENGTH]],
  default: [None; DIST_FNS_LENGTH],
  [AVX2]
);

cpu_function_lookup_table!(
  SSE_HBD_FNS: [[Option<SseHBDFn>; DIST_FNS_LENGTH]],
  default: [None; DIST_FNS_LENGTH],
  [AVX2]
);


#[cfg(test)]
pub mod test {
    use super::*;
    use crate::frame::*;
    use rand::{thread_rng, Rng};
    use crate::tiling::Area;

    fn setup_planes() -> (Plane<u8>, Plane<u8>) {
        let mut rng = thread_rng();

        // Two planes with different strides
        let mut input_plane = Plane::new(640, 480, 0, 0, 128 + 8, 128 + 8);
        let mut rec_plane = Plane::new(640, 480, 0, 0, 2 * 128 + 8, 2 * 128 + 8);

        for rows in input_plane.as_region_mut().rows_iter_mut() {
            for c in rows {
                *c = rng.gen::<u8>();
            }
        }

        for rows in rec_plane.as_region_mut().rows_iter_mut() {
            for c in rows {
                *c = rng.gen::<u8>();
            }
        }

        (input_plane, rec_plane)
    }

    #[test]
    fn get_sse_x86_test() {
        use BlockSize::*;
        let blocks = vec![
            BLOCK_4X4,
        BLOCK_4X8,
        BLOCK_8X4,
        BLOCK_8X8,
        BLOCK_8X16,
        BLOCK_16X8,
        BLOCK_16X16,
        BLOCK_16X32,
        BLOCK_32X16,
        BLOCK_32X32,
        BLOCK_32X64,
        BLOCK_64X32,
        BLOCK_64X64,
        BLOCK_64X128,
        BLOCK_128X64,
        BLOCK_128X128,
        BLOCK_4X16,
        BLOCK_16X4,
        BLOCK_8X32,
        BLOCK_32X8,
        BLOCK_16X64,
        BLOCK_64X16,
        ];

        let bit_depth: usize = 8;
        let (input_plane, rec_plane) = setup_planes();

        for block in blocks {
            let area = Area::StartingAt { x: 32, y: 40 };

            let input_region = input_plane.region(area);
            let rec_region = rec_plane.region(area);

            get_sse(
                &input_region,
                &rec_region,
                block,
                bit_depth,
                CpuFeatureLevel::default()
            );
        }
    }
}
 */