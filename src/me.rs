// Copyright (c) 2017-2018, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use context::BlockOffset;
use context::BLOCK_TO_PLANE_SHIFT;
use partition::*;
use plane::*;
use FrameInvariants;
use FrameState;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn get_sad_avx2(
  plane_org: &mut PlaneSlice, plane_ref: &mut PlaneSlice, blk_h: usize,
  blk_w: usize
) -> u32 {
  #[cfg(target_arch = "x86")]
  use std::arch::x86::*;
  #[cfg(target_arch = "x86_64")]
  use std::arch::x86_64::*;
  let org_stride = plane_org.plane.cfg.stride;
  let ref_stride = plane_ref.plane.cfg.stride;
  let org_ptr = plane_org.as_slice().as_ptr();
  let ref_ptr = plane_ref.as_slice().as_ptr();
  let mut sums = _mm256_setzero_si256();
  for r in (0..blk_h).step_by(4) {
    for c in (0..blk_w).step_by(16) {
      let a = (
        _mm256_loadu_si256(org_ptr.offset((r * org_stride + c) as isize) as *const _),
        _mm256_loadu_si256(org_ptr.offset(((r + 1) * org_stride + c) as isize) as *const _),
        _mm256_loadu_si256(org_ptr.offset(((r + 2) * org_stride + c) as isize) as *const _),
        _mm256_loadu_si256(org_ptr.offset(((r + 3) * org_stride + c) as isize) as *const _)
      );
      let b = (
        _mm256_loadu_si256(ref_ptr.offset((r * ref_stride + c) as isize) as *const _),
        _mm256_loadu_si256(ref_ptr.offset(((r + 1) * ref_stride + c) as isize) as *const _),
        _mm256_loadu_si256(ref_ptr.offset(((r + 2) * ref_stride + c) as isize) as *const _),
        _mm256_loadu_si256(ref_ptr.offset(((r + 3) * ref_stride + c) as isize) as *const _)
      );
      let abs_diff = (
        _mm256_abs_epi16(_mm256_sub_epi16(a.0, b.0)),
        _mm256_abs_epi16(_mm256_sub_epi16(a.1, b.1)),
        _mm256_abs_epi16(_mm256_sub_epi16(a.2, b.2)),
        _mm256_abs_epi16(_mm256_sub_epi16(a.3, b.3)),
      );
      let sums16 = _mm256_add_epi16(_mm256_add_epi16(abs_diff.0, abs_diff.1), _mm256_add_epi16(abs_diff.2, abs_diff.3));

      sums = _mm256_add_epi32(sums, _mm256_add_epi32(_mm256_unpacklo_epi16(sums16, _mm256_setzero_si256()),
                                                     _mm256_unpackhi_epi16(sums16, _mm256_setzero_si256())));
    }
  }
  sums = _mm256_add_epi32(sums, _mm256_bsrli_epi128(sums, 8));
  sums = _mm256_add_epi32(sums, _mm256_bsrli_epi128(sums, 4));
  _mm_cvtsi128_si32(_mm_add_epi32(_mm256_castsi256_si128(sums), _mm256_extracti128_si256(sums, 1))) as u32
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "ssse3")]
unsafe fn get_sad_ssse3(
  plane_org: &mut PlaneSlice, plane_ref: &mut PlaneSlice, blk_h: usize,
  blk_w: usize
) -> u32 {
  #[cfg(target_arch = "x86")]
  use std::arch::x86::*;
  #[cfg(target_arch = "x86_64")]
  use std::arch::x86_64::*;
  let org_stride = plane_org.plane.cfg.stride;
  let ref_stride = plane_ref.plane.cfg.stride;
  let org_ptr = plane_org.as_slice().as_ptr();
  let ref_ptr = plane_ref.as_slice().as_ptr();
  let mut sums = _mm_setzero_si128();
  for r in (0..blk_h).step_by(4) {
    for c in (0..blk_w).step_by(8) {
      let a = (
        _mm_loadu_si128(org_ptr.offset((r * org_stride + c) as isize) as *const _),
        _mm_loadu_si128(org_ptr.offset(((r + 1) * org_stride + c) as isize) as *const _),
        _mm_loadu_si128(org_ptr.offset(((r + 2) * org_stride + c) as isize) as *const _),
        _mm_loadu_si128(org_ptr.offset(((r + 3) * org_stride + c) as isize) as *const _)
      );
      let b = (
        _mm_loadu_si128(ref_ptr.offset((r * ref_stride + c) as isize) as *const _),
        _mm_loadu_si128(ref_ptr.offset(((r + 1) * ref_stride + c) as isize) as *const _),
        _mm_loadu_si128(ref_ptr.offset(((r + 2) * ref_stride + c) as isize) as *const _),
        _mm_loadu_si128(ref_ptr.offset(((r + 3) * ref_stride + c) as isize) as *const _)
      );
      let abs_diff = (
        _mm_abs_epi16(_mm_sub_epi16(a.0, b.0)),
        _mm_abs_epi16(_mm_sub_epi16(a.1, b.1)),
        _mm_abs_epi16(_mm_sub_epi16(a.2, b.2)),
        _mm_abs_epi16(_mm_sub_epi16(a.3, b.3)),
      );
      let sums16 = _mm_add_epi16(_mm_add_epi16(abs_diff.0, abs_diff.1), _mm_add_epi16(abs_diff.2, abs_diff.3));

      sums = _mm_add_epi32(sums, _mm_add_epi32(_mm_unpacklo_epi16(sums16, _mm_setzero_si128()),
                                               _mm_unpackhi_epi16(sums16, _mm_setzero_si128())));
    }
  }
  sums = _mm_add_epi32(sums, _mm_bsrli_si128(sums, 8));
  sums = _mm_add_epi32(sums, _mm_bsrli_si128(sums, 4));
  _mm_cvtsi128_si32(sums) as u32
}

#[inline(always)]
pub fn get_sad(
  plane_org: &mut PlaneSlice, plane_ref: &mut PlaneSlice, blk_h: usize,
  blk_w: usize
) -> u32 {
  let mut sum = 0 as u32;

  #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
  {
    if is_x86_feature_detected!("avx2") && blk_w >= 16 && blk_h >= 4 {
        return unsafe { get_sad_avx2(plane_org, plane_ref, blk_h, blk_w) };
    }
    if is_x86_feature_detected!("ssse3") && blk_w >= 8 && blk_h >= 4 {
        return unsafe { get_sad_ssse3(plane_org, plane_ref, blk_h, blk_w) };
    }
  }

  for _r in 0..blk_h {
    {
      let slice_org = plane_org.as_slice_w_width(blk_w);
      let slice_ref = plane_ref.as_slice_w_width(blk_w);
      sum += slice_org
        .iter()
        .zip(slice_ref)
        .map(|(&a, &b)| (a as i16 - b as i16).abs() as u32)
        .sum::<u32>();
    }
    plane_org.y += 1;
    plane_ref.y += 1;
  }

  sum
}

pub fn motion_estimation(fi: &FrameInvariants, fs: &mut FrameState, bsize: BlockSize,
                         bo: &BlockOffset, ref_frame: usize) -> MotionVector {

  match fi.rec_buffer.frames[fi.ref_frames[ref_frame - LAST_FRAME]] {
    Some(ref rec) => {
      let po = PlaneOffset { x: (bo.x as isize) << BLOCK_TO_PLANE_SHIFT, y: (bo.y as isize) << BLOCK_TO_PLANE_SHIFT };
      let range = 32 as isize;
      let blk_w = bsize.width();
      let blk_h = bsize.height();
      let x_lo = po.x - range;
      let x_hi = po.x + range;
      let y_lo = po.y - range;
      let y_hi = po.y + range;

      let mut lowest_sad = 128*128*4096 as u32;
      let mut best_mv = MotionVector { row: 0, col: 0 };

      for y in (y_lo..y_hi).step_by(8) {
        for x in (x_lo..x_hi).step_by(8) {
          let mut plane_org = fs.input.planes[0].slice(&po);
          let mut plane_ref = rec.frame.planes[0].slice(&PlaneOffset { x: x, y: y });

          let sad = get_sad(&mut plane_org, &mut plane_ref, blk_h, blk_w);

          if sad < lowest_sad {
            lowest_sad = sad;
            best_mv = MotionVector { row: 8*(y as i16 - po.y as i16), col: 8*(x as i16 - po.x as i16) }
          }

        }
      }

      let mode = PredictionMode::NEWMV;
      let mut tmp_plane = Plane::new(blk_w, blk_h, 0, 0, 0, 0);

      let mut steps = vec![32, 16, 8, 4, 2];
      if fi.allow_high_precision_mv {
        steps.push(1);
      }

      for step in steps {
        let center_mv_h = best_mv;
        for i in 0..3 {
          for j in 0..3 {
            // Skip the center point that was already tested
            if i == 1 && j == 1 { continue; }

            let cand_mv = MotionVector { row: center_mv_h.row + step*(i as i16 - 1),
            col: center_mv_h.col + step*(j as i16 - 1) };

            {
              let tmp_slice = &mut tmp_plane.mut_slice(&PlaneOffset { x:0, y:0 });

              mode.predict_inter(fi, 0, &po, tmp_slice, blk_w, blk_h, ref_frame, &cand_mv, 8);
            }

            let mut plane_org = fs.input.planes[0].slice(&po);
            let mut plane_ref = tmp_plane.slice(&PlaneOffset { x:0, y:0 });

            let sad = get_sad(&mut plane_org, &mut plane_ref, blk_h, blk_w);

            if sad < lowest_sad {
              lowest_sad = sad;
              best_mv = cand_mv;
            }
          }
        }
      }

      best_mv
    },

    None => MotionVector { row: 0, col : 0 }
  }
}
