// Copyright (c) 2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

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
  rav1e_sse_4x4_avx2
];


declare_asm_hbd_sse_fn![
  // AVX2
  rav1e_sse_4x4_hbd_avx2
];

#[inline(always)]
#[allow(clippy::let_and_return)]
pub fn get_sse<T: Pixel>(
    src: &PlaneRegion<'_, T>, dst: &PlaneRegion<'_, T>, w: usize, h: usize,
    bit_depth: usize, cpu: CpuFeatureLevel,
) -> u64 {
    // TODO: Remove in the future and pass BlockSize directly. Currently
    // implemented to allow testing distortion only on the visible portion of
    // the frame
    assert!(w <= 128 && h <= 128);
    let bsize = BlockSize::from_width_and_height(w, h);

    let call_rust =
        || -> u64 { rust::get_sse(dst, src, w, h, bit_depth, cpu) };

    #[cfg(feature = "check_asm")]
    let ref_dist = call_rust();

    // TODO: Remove the w and h check
    // If width and height are powers of 2 and greater than or equal 4 .
    let dist = if (w-1)&(w|3) == 0 && (h-1)&(h|3) == 0 {
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
        }
    } else {
        call_rust()
    };

    #[cfg(feature = "check_asm")]
    assert_eq!(dist, ref_dist);

    dist
}

static SSE_FNS_AVX2: [Option<SseFn>; DIST_FNS_LENGTH] = {
    let mut out: [Option<SseFn>; DIST_FNS_LENGTH] = [None; DIST_FNS_LENGTH];

    use BlockSize::*;
    out[BLOCK_4X4 as usize] = Some(rav1e_sse_4x4_avx2);

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
