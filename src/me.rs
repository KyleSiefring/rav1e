// Copyright (c) 2017-2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use crate::context::{
  BlockOffset, PlaneBlockOffset, SuperBlockOffset, TileBlockOffset,
  TileSuperBlockOffset, BLOCK_TO_PLANE_SHIFT, MAX_MIB_SIZE_LOG2, MI_SIZE,
  MI_SIZE_LOG2,
};
use crate::dist::*;
use crate::encoder::ReferenceFrame;
use crate::frame::*;
use crate::mc::MotionVector;
use crate::partition::*;
use crate::predict::PredictionMode;
use crate::tiling::*;
use crate::util::{clamp, Pixel};
use crate::FrameInvariants;

use arrayvec::*;

use crate::api::InterConfig;
use crate::util::ILog;
use std::convert::identity;
use std::iter;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FrameMotionVectors {
  mvs: Box<[MotionVector]>,
  pub cols: usize,
  pub rows: usize,
}

impl FrameMotionVectors {
  pub fn new(cols: usize, rows: usize) -> Self {
    Self {
      // dynamic allocation: once per frame
      mvs: vec![MotionVector::default(); cols * rows].into_boxed_slice(),
      cols,
      rows,
    }
  }
}

impl Index<usize> for FrameMotionVectors {
  type Output = [MotionVector];
  #[inline]
  fn index(&self, index: usize) -> &Self::Output {
    &self.mvs[index * self.cols..(index + 1) * self.cols]
  }
}

impl IndexMut<usize> for FrameMotionVectors {
  #[inline]
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.mvs[index * self.cols..(index + 1) * self.cols]
  }
}

#[derive(Debug, Copy, Clone)]
pub struct MEStats {
  pub mv: MotionVector,
  pub sad: u32,
}

impl Default for MEStats {
  fn default() -> Self {
    Self { mv: MotionVector::default(), sad: 0 }
  }
}

#[derive(Debug, Clone)]
pub struct FrameMEStats {
  stats: Box<[MEStats]>,
  pub cols: usize,
  pub rows: usize,
}

impl FrameMEStats {
  pub fn new(cols: usize, rows: usize) -> Self {
    Self {
      // dynamic allocation: once per frame
      stats: vec![MEStats::default(); cols * rows].into_boxed_slice(),
      cols,
      rows,
    }
  }
}

impl Index<usize> for FrameMEStats {
  type Output = [MEStats];
  #[inline]
  fn index(&self, index: usize) -> &Self::Output {
    &self.stats[index * self.cols..(index + 1) * self.cols]
  }
}

impl IndexMut<usize> for FrameMEStats {
  #[inline]
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.stats[index * self.cols..(index + 1) * self.cols]
  }
}

#[derive(Debug, Copy, Clone)]
struct MVSearchResult {
  mv: MotionVector,
  cost: u64,
}

#[derive(Debug, Copy, Clone)]
struct FullpelSearchResult {
  mv: MotionVector,
  cost: u64,
  sad: u32,
}

#[derive(Eq, PartialEq)]
enum BlockCorner {
  INIT,
  NW,
  NE,
  SW,
  SE,
}

pub fn prep_tile_motion_estimation<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &mut TileStateMut<'_, T>,
  inter_cfg: &InterConfig,
) {
  for sby in 0..ts.sb_height {
    for sbx in 0..ts.sb_width {
      prep_square_block_motion_estimation(
        fi,
        ts,
        inter_cfg,
        BlockSize::BLOCK_64X64.width_mi_log2(),
        TileSuperBlockOffset(SuperBlockOffset { x: sbx, y: sby })
          .block_offset(0, 0),
        true,
      );
    }
  }

  for sby in 0..ts.sb_height {
    for sbx in 0..ts.sb_width {
      prep_square_block_motion_estimation(
        fi,
        ts,
        inter_cfg,
        // TODO: Awkward with starting with splitting into 32x32
        BlockSize::BLOCK_64X64.width_mi_log2(),
        TileSuperBlockOffset(SuperBlockOffset { x: sbx, y: sby })
          .block_offset(0, 0),
        false,
      );
    }
  }
}

fn prep_square_block_motion_estimation<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &mut TileStateMut<'_, T>,
  inter_cfg: &InterConfig, size_mi_log2: usize, tile_bo: TileBlockOffset,
  init: bool,
) {
  let size_mi = 1 << size_mi_log2;
  let mut mv_size_log2 = size_mi_log2 - if init { 0 } else { 1 };
  let h_in_b: usize = size_mi.min(ts.mi_height - tile_bo.0.y);
  let w_in_b: usize = size_mi.min(ts.mi_width - tile_bo.0.x);
  // TODO: change to while loop since mv_size_log2 subtracted if not init
  loop {
    let mv_size = 1 << mv_size_log2;
    let bsize = BlockSize::from_width_and_height(
      mv_size << MI_SIZE_LOG2,
      mv_size << MI_SIZE_LOG2,
    );

    for &r in inter_cfg.allowed_ref_frames() {
      for y in (0..h_in_b).step_by(mv_size) {
        for x in (0..w_in_b).step_by(mv_size) {
          let corner: BlockCorner = match (init, y & mv_size == mv_size, x & mv_size == mv_size) {
            (true, _, _) => BlockCorner::INIT,
            (_, false, false) => BlockCorner::NW,
            (_, false, true) => BlockCorner::NE,
            (_, true, false) => BlockCorner::SW,
            (_, true, true) => BlockCorner::SE,
          };
          let bo = tile_bo.with_offset(x as isize, y as isize);
          if let Some(results) =
            estimate_motion_alt(fi, ts, bsize, bo, r, corner, init)
          {
            let sad = results.sad << (MAX_MIB_SIZE_LOG2 - mv_size_log2) * 2;
            save_me_stats(ts, bsize, bo, r, MEStats { mv: results.mv, sad });
          }
        }
      }
    }

    if init || mv_size_log2 == 2 {
      break;
    }
    mv_size_log2 -= 1;
  }
}

fn save_me_stats<T: Pixel>(
  ts: &mut TileStateMut<'_, T>, bsize: BlockSize, tile_bo: TileBlockOffset,
  ref_frame: RefType, stats: MEStats,
) {
  let tile_me_stats = &mut ts.me_stats[ref_frame.to_index()];
  let tile_mvs = &mut ts.mvs[ref_frame.to_index()];
  let tile_bo_x_end = (tile_bo.0.x + bsize.width_mi()).min(ts.mi_width);
  let tile_bo_y_end = (tile_bo.0.y + bsize.height_mi()).min(ts.mi_height);
  for mi_y in tile_bo.0.y..tile_bo_y_end {
    for a in tile_me_stats[mi_y][tile_bo.0.x..tile_bo_x_end].iter_mut() {
      *a = stats;
    }
    for a in tile_mvs[mi_y][tile_bo.0.x..tile_bo_x_end].iter_mut() {
      *a = stats.mv;
    }
  }
}

fn estimate_motion_alt<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
  tile_bo: TileBlockOffset, ref_frame: RefType, corner: BlockCorner,
  can_full_search: bool,
) -> Option<FullpelSearchResult> {
  if let Some(ref rec) =
    fi.rec_buffer.frames[fi.ref_frames[ref_frame.to_index()] as usize]
  {
    let blk_w = bsize.width();
    let blk_h = bsize.height();
    let ssdec = if blk_w == 64 { 2 } else if blk_w == 32 { 1 } else { 0 };
    let tile_bo_adj =
      adjust_bo(tile_bo, ts.mi_width, ts.mi_height, blk_w, blk_h);
    let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
    let (mvx_min, mvx_max, mvy_min, mvy_max) =
      get_mv_range(fi.w_in_b, fi.h_in_b, frame_bo_adj, blk_w, blk_h);

    let global_mv = [MotionVector { row: 0, col: 0 }; 2];

    // 0.5 and 0.125 are a fudge factors
    let lambda = (fi.me_lambda * 256.0 * if blk_w <= 16 { 0.5 } else { 0.125 } ) as u32;

    let po = frame_bo_adj.to_luma_plane_offset();

    let (mvx_min, mvx_max, mvy_min, mvy_max) = (mvx_min >> ssdec, mvx_max >> ssdec, mvy_min >> ssdec, mvy_max >> ssdec);
    let bsize = BlockSize::from_width_and_height(blk_w >> ssdec, blk_h >> ssdec);
    let po = PlaneOffset { x: po.x >> ssdec, y: po.y >> ssdec };
    let p_ref = match ssdec {
      0 => &rec.frame.planes[0],
      1 => &rec.input_hres,
      2 => &rec.input_qres,
      _ => unimplemented!()
    };

    let org_region = &match ssdec {
      0 => &ts.input.planes[0],
      1 => ts.input_hres,
      2 => ts.input_qres,
      _ => unimplemented!()
    }.region(Area::StartingAt { x: po.x, y: po.y });

    let mut results: FullpelSearchResult = full_pixel_me_alt(
      fi,
      ts,
      org_region,
      p_ref,
      tile_bo_adj,
      po,
      lambda,
      global_mv,
      mvx_min,
      mvx_max,
      mvy_min,
      mvy_max,
      bsize,
      ref_frame,
      corner,
      can_full_search,
      ssdec,
    );

    results.sad <<= ssdec * 2;
    results.mv = MotionVector { col: results.mv.col << ssdec, row: results.mv.row << ssdec };

    Some(results)
  } else {
    None
  }
}

fn full_pixel_me_alt<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
  org_region: &PlaneRegion<T>, p_ref: &Plane<T>, tile_bo: TileBlockOffset,
  po: PlaneOffset,
  lambda: u32, pmv: [MotionVector; 2], mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize, ref_frame: RefType,
  corner: BlockCorner, can_full_search: bool, ssdec: u8,
) -> FullpelSearchResult {
  let tile_me_stats = &ts.me_stats[ref_frame.to_index()].as_const();
  let frame_ref =
    fi.rec_buffer.frames[fi.ref_frames[0] as usize].as_ref().map(Arc::as_ref);
  let mut subsets = get_subset_predictors_alt(
    tile_bo,
    tile_me_stats,
    frame_ref,
    ref_frame.to_index(),
    bsize,
    mvx_min,
    mvx_max,
    mvy_min,
    mvy_max,
    corner,
    ssdec,
  );

  let thresh = (subsets.min_sad as f32 * 1.2) as u32
    + (1 << (bsize.height_log2() + bsize.width_log2()));

  let mut best = FullpelSearchResult {
    mv: Default::default(),
    cost: u64::MAX,
    sad: u32::MAX,
  };

  let try_cand = |predictors: &[MotionVector],
                  best: &mut FullpelSearchResult| {
    let mut results = get_best_predictor(
      fi,
      po,
      org_region,
      p_ref,
      predictors,
      fi.sequence.bit_depth,
      pmv,
      lambda,
      mvx_min,
      mvx_max,
      mvy_min,
      mvy_max,
      bsize,
    );
    fullpel_diamond_me_search_alt(
      fi,
      po,
      org_region,
      p_ref,
      &mut results,
      fi.sequence.bit_depth,
      pmv,
      lambda,
      mvx_min,
      mvx_max,
      mvy_min,
      mvy_max,
      bsize,
    );

    if results.cost < best.cost {
      *best = results;
    }
  };

  if !can_full_search {
    let allmvs: ArrayVec<[MotionVector; 11]> = subsets.median.into_iter().chain(subsets.subset_b).chain(subsets.subset_c).collect();
    try_cand(&allmvs, &mut best);
    best
  } else {
    if let Some(median) = subsets.median {
      try_cand(&[median], &mut best);

      if best.sad < thresh {
        return best;
      }
    }

    try_cand(&subsets.subset_b, &mut best);

    if best.sad < thresh {
      return best;
    }

    try_cand(&subsets.subset_c, &mut best);

    if best.sad < thresh {
      return best;
    }

    {
      let range_x = 192 * fi.me_range_scale as isize >> ssdec;
      let range_y = 64 * fi.me_range_scale as isize >> ssdec;
      let x_lo = po.x + (-range_x).max(mvx_min / 8);
      let x_hi = po.x + (range_x).min(mvx_max / 8);
      let y_lo = po.y + (-range_y).max(mvy_min / 8);
      let y_hi = po.y + (range_y).min(mvy_max / 8);

      let results = full_search(
        fi,
        x_lo,
        x_hi,
        y_lo,
        y_hi,
        bsize,
        org_region,
        p_ref,
        po,
        4 >> ssdec,
        lambda,
        [MotionVector::default(); 2],
      );

      if results.cost < best.cost {
        results
      }
      else {
        best
      }
    }
  }

}

struct MotionEstimationSubsets {
  min_sad: u32,
  median: Option<MotionVector>,
  subset_b: ArrayVec<[MotionVector; 5]>,
  subset_c: ArrayVec<[MotionVector; 5]>,
}

fn get_subset_predictors_alt<T: Pixel>(
  tile_bo: TileBlockOffset, tile_me_stats: &TileMEStats<'_>,
  frame_ref_opt: Option<&ReferenceFrame<T>>, ref_frame_id: usize,
  bsize: BlockSize, mvx_min: isize, mvx_max: isize, mvy_min: isize,
  mvy_max: isize, corner: BlockCorner, ssdec: u8
) -> MotionEstimationSubsets {
  let mut min_sad: u32 = u32::MAX;
  let mut subset_b = ArrayVec::<[MotionVector; 5]>::new();
  let mut subset_c = ArrayVec::<[MotionVector; 5]>::new();
  let w = bsize.width_mi() << ssdec;
  let h = bsize.height_mi() << ssdec;

  // EPZS subset A and B predictors.
  // Since everything is being pushed, a median doesn't need to be calculated
  // for subset A.
  // Sample the middle of bordering side of the left and top blocks.

  let mut process_cand = |stats: MEStats| -> MotionVector {
    min_sad = min_sad.min(stats.sad);
    let mv = stats.mv.quantize_to_fullpel();
    MotionVector {
      col: clamp(mv.col as isize, mvx_min, mvx_max) as i16,
      row: clamp(mv.row as isize, mvy_min, mvy_max) as i16,
    }
  };

  match corner {
    BlockCorner::NW | BlockCorner::SW => {
      if tile_bo.0.x < tile_me_stats.cols() - w {
        // right
        subset_b.push(process_cand(
          tile_me_stats[tile_bo.0.y + (h >> 1)][tile_bo.0.x + w],
        ));
      }
    }
    _ => {}
  }

  match corner {
    BlockCorner::SW | BlockCorner::SE => {
      if tile_bo.0.y < tile_me_stats.rows() - h {
        // bottom
        subset_b.push(process_cand(
          tile_me_stats[tile_bo.0.y + h][tile_bo.0.x + (w >> 1)],
        ));
      }
    }
    _ => {}
  }

  if tile_bo.0.x > 0 {
    // left
    subset_b.push(process_cand(
      tile_me_stats[tile_bo.0.y + (h >> 1)][tile_bo.0.x - 1],
    ));
  }
  if tile_bo.0.y > 0 {
    // top
    subset_b.push(process_cand(
      tile_me_stats[tile_bo.0.y - 1][tile_bo.0.x + (w >> 1)],
    ));
  }

  let median = if corner != BlockCorner::INIT {
    Some(process_cand(tile_me_stats[tile_bo.0.y][tile_bo.0.x]))
  } else {
    if tile_bo.0.y > 0 && tile_bo.0.x < tile_me_stats.cols() - w {
      // top right
      subset_b
        .push(process_cand(tile_me_stats[tile_bo.0.y - 1][tile_bo.0.x + w]));
    }

    if subset_b.len() < 3 {
      None
    } else {
      let mut rows: ArrayVec<[i16; 4]> =
        subset_b.iter().map(|&a| a.row).collect();
      let mut cols: ArrayVec<[i16; 4]> =
        subset_b.iter().map(|&a| a.col).collect();
      rows.as_mut_slice().sort();
      cols.as_mut_slice().sort();
      Some(MotionVector { row: rows[1], col: cols[1] })
    }
  };

  // Try to propagate from outside adjacent blocks
  if corner == BlockCorner::NW
    && tile_bo.0.x < tile_me_stats.cols() - (w << 1)
    && tile_bo.0.y < tile_me_stats.rows() - (h << 1)
  {
    // far bottom right
    subset_b.push(process_cand(tile_me_stats[tile_bo.0.y + (h << 1)][tile_bo.0.x + (w << 1)]));
  }

  // Zero motion vector, don't use add_cand since it skips zero vectors.
  subset_b.push(MotionVector::default());

  // EPZS subset C predictors.
  // Sample the middle of bordering side of the left, right, top and bottom
  // blocks of the previous frame.
  // Sample the middle of this block in the previous frame.

  if let Some(frame_ref) = frame_ref_opt {
    let prev_frame = &frame_ref.frame_me_stats[ref_frame_id];

    let frame_bo = PlaneBlockOffset(BlockOffset {
      x: tile_me_stats.x() + tile_bo.0.x,
      y: tile_me_stats.y() + tile_bo.0.y,
    });
    if frame_bo.0.x > 0 {
      // left
      subset_c.push(process_cand(
        prev_frame[frame_bo.0.y + (h >> 1)][frame_bo.0.x - 1],
      ));
    }
    if frame_bo.0.y > 0 {
      // top
      subset_c.push(process_cand(
        prev_frame[frame_bo.0.y - 1][frame_bo.0.x + (w >> 1)],
      ));
    }
    if frame_bo.0.x < prev_frame.cols - w {
      // right
      subset_c.push(process_cand(
        prev_frame[frame_bo.0.y + (h >> 1)][frame_bo.0.x + w],
      ));
    }
    if frame_bo.0.y < prev_frame.rows - h {
      // bottom
      subset_c.push(process_cand(
        prev_frame[frame_bo.0.y + h][frame_bo.0.x + (w >> 1)],
      ));
    }

    subset_c.push(process_cand(
      prev_frame[frame_bo.0.y + (h >> 1)][frame_bo.0.x + (w >> 1)],
    ));
  }

  let min_sad = min_sad
    >> (MAX_MIB_SIZE_LOG2 * 2
      - (bsize.width_mi_log2() + bsize.height_mi_log2() + ssdec as usize * 2));

  let dec_mv = |mv: MotionVector| MotionVector { col: mv.col >> ssdec, row: mv.row >> ssdec };
  let median = median.and_then(|mv| Some(dec_mv(mv)) );
  for mv in subset_b.iter_mut() {
    *mv = dec_mv(*mv);
  }
  for mv in subset_c.iter_mut() {
    *mv = dec_mv(*mv);
  }

  MotionEstimationSubsets { min_sad, median, subset_b, subset_c }
}

fn fullpel_diamond_me_search_alt<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, center: &mut FullpelSearchResult, bit_depth: usize,
  pmv: [MotionVector; 2], lambda: u32, mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize,
) {
  let diamond_pattern = [(1i16, 0i16), (0, 1), (-1, 0), (0, -1)];
  let (mut diamond_radius, diamond_radius_end) = (4u8, 3u8);

  loop {
    let mut best_diamond: FullpelSearchResult = FullpelSearchResult {
      mv: MotionVector::default(),
      sad: u32::MAX,
      cost: u64::MAX,
    };

    for p in diamond_pattern.iter() {
      let cand_mv = MotionVector {
        row: center.mv.row + (p.0 << diamond_radius),
        col: center.mv.col + (p.1 << diamond_radius),
      };

      let rd_cost = get_fullpel_mv_rd_cost(
        fi, po, org_region, p_ref, bit_depth, pmv, lambda, false, mvx_min,
        mvx_max, mvy_min, mvy_max, bsize, cand_mv,
      );

      if rd_cost.0 < best_diamond.cost {
        best_diamond.mv = cand_mv;
        best_diamond.cost = rd_cost.0;
        best_diamond.sad = rd_cost.1;
      }
    }

    if center.cost <= best_diamond.cost {
      if diamond_radius == diamond_radius_end {
        break;
      } else {
        diamond_radius -= 1;
      }
    } else {
      *center = best_diamond;
    }
  }

  assert!(center.cost < std::u64::MAX);
}

const fn get_mv_range(
  w_in_b: usize, h_in_b: usize, bo: PlaneBlockOffset, blk_w: usize,
  blk_h: usize,
) -> (isize, isize, isize, isize) {
  let border_w = 128 + blk_w as isize * 8;
  let border_h = 128 + blk_h as isize * 8;
  let mvx_min = -(bo.0.x as isize) * (8 * MI_SIZE) as isize - border_w;
  let mvx_max = (w_in_b - bo.0.x - blk_w / MI_SIZE) as isize
    * (8 * MI_SIZE) as isize
    + border_w;
  let mvy_min = -(bo.0.y as isize) * (8 * MI_SIZE) as isize - border_h;
  let mvy_max = (h_in_b - bo.0.y - blk_h / MI_SIZE) as isize
    * (8 * MI_SIZE) as isize
    + border_h;

  (mvx_min, mvx_max, mvy_min, mvy_max)
}

pub fn get_subset_predictors<T: Pixel>(
  tile_bo: TileBlockOffset, cmvs: ArrayVec<[MotionVector; 7]>,
  tile_mvs: &TileMotionVectors<'_>, frame_ref_opt: Option<&ReferenceFrame<T>>,
  ref_frame_id: usize, bsize: BlockSize,
) -> ArrayVec<[MotionVector; 16]> {
  let mut predictors = ArrayVec::<[_; 16]>::new();
  let w = bsize.width_mi();
  let h = bsize.height_mi();

  // Add a candidate predictor, aligning to fullpel and filtering out zero mvs.
  let add_cand = |predictors: &mut ArrayVec<[MotionVector; 16]>,
                  cand_mv: MotionVector| {
    let cand_mv = cand_mv.quantize_to_fullpel();
    if !cand_mv.is_zero() {
      predictors.push(cand_mv)
    }
  };

  let corner: BlockCorner = match (tile_bo.0.y & h == h, tile_bo.0.x & h == h) {
    (false, false) => BlockCorner::NW,
    (false, true) => BlockCorner::NE,
    (true, false) => BlockCorner::SW,
    (true, true) => BlockCorner::SE,
  };

  // Zero motion vector, don't use add_cand since it skips zero vectors.
  predictors.push(MotionVector::default());

  // Coarse motion estimation.
  for mv in cmvs {
    add_cand(&mut predictors, mv);
  }

  // EPZS subset A and B predictors.
  // Since everything is being pushed, a median doesn't need to be calculated
  // for subset A.
  // Sample the middle of bordering side of the left and top blocks.

  match corner {
    BlockCorner::NW | BlockCorner::SW => {
      if tile_bo.0.x < tile_mvs.cols() - w {
        // right
        add_cand(&mut predictors, tile_mvs[tile_bo.0.y + (h >> 1)][tile_bo.0.x + w]);
      }
    }
    _ => {}
  }

  match corner {
    BlockCorner::SW | BlockCorner::SE => {
      if tile_bo.0.y < tile_mvs.rows() - h {
        // bottom
        add_cand(&mut predictors, tile_mvs[tile_bo.0.y + h][tile_bo.0.x + (w >> 1)]);
      }
    }
    _ => {}
  }

  if tile_bo.0.x > 0 {
    // left
    add_cand(&mut predictors, tile_mvs[tile_bo.0.y + (h >> 1)][tile_bo.0.x - 1]);
  }
  if tile_bo.0.y > 0 {
    // top
    add_cand(&mut predictors, tile_mvs[tile_bo.0.y - 1][tile_bo.0.x + (w >> 1)]);
  }

  // median or middle sample
  add_cand(&mut predictors, tile_mvs[tile_bo.0.y][tile_bo.0.x]);

  /*if tile_bo.0.x > 0 {
    let left = tile_mvs[tile_bo.0.y + (h >> 1)][tile_bo.0.x - 1];
    add_cand(&mut predictors, left);
  }
  if tile_bo.0.y > 0 {
    let top = tile_mvs[tile_bo.0.y - 1][tile_bo.0.x + (w >> 1)];
    add_cand(&mut predictors, top);

    if tile_bo.0.x < tile_mvs.cols() - w {
      let top_right = tile_mvs[tile_bo.0.y - 1][tile_bo.0.x + w];
      add_cand(&mut predictors, top_right);
    }
  }*/

  // EPZS subset C predictors.
  // Sample the middle of bordering side of the left, right, top and bottom
  // blocks of the previous frame.
  // Sample the middle of this block in the previous frame.

  if let Some(frame_ref) = frame_ref_opt {
    let prev_frame_mvs = &frame_ref.frame_mvs[ref_frame_id];

    let frame_bo = PlaneBlockOffset(BlockOffset {
      x: tile_mvs.x() + tile_bo.0.x,
      y: tile_mvs.y() + tile_bo.0.y,
    });
    if frame_bo.0.x > 0 {
      let left = prev_frame_mvs[frame_bo.0.y + (h >> 1)][frame_bo.0.x - 1];
      add_cand(&mut predictors, left);
    }
    if frame_bo.0.y > 0 {
      let top = prev_frame_mvs[frame_bo.0.y - 1][frame_bo.0.x + (w >> 1)];
      add_cand(&mut predictors, top);
    }
    if frame_bo.0.x < prev_frame_mvs.cols - w {
      let right = prev_frame_mvs[frame_bo.0.y + (h >> 1)][frame_bo.0.x + w];
      add_cand(&mut predictors, right);
    }
    if frame_bo.0.y < prev_frame_mvs.rows - h {
      let bottom = prev_frame_mvs[frame_bo.0.y + h][frame_bo.0.x + (w >> 1)];
      add_cand(&mut predictors, bottom);
    }

    let previous =
      prev_frame_mvs[frame_bo.0.y + (h >> 1)][frame_bo.0.x + (w >> 1)];
    add_cand(&mut predictors, previous);
  }

  predictors
}

pub fn motion_estimation<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
  tile_bo: TileBlockOffset, ref_frame: RefType, cmv: MotionVector,
  pmv: [MotionVector; 2],
) -> (MotionVector, u32) {
  match fi.rec_buffer.frames[fi.ref_frames[ref_frame.to_index()] as usize] {
    Some(ref rec) => {
      let blk_w = bsize.width();
      let blk_h = bsize.height();
      let frame_bo = ts.to_frame_block_offset(tile_bo);
      let (mvx_min, mvx_max, mvy_min, mvy_max) =
        get_mv_range(fi.w_in_b, fi.h_in_b, frame_bo, blk_w, blk_h);

      // 0.5 is a fudge factor
      let lambda = (fi.me_lambda * 256.0 * 0.5) as u32;

      // Full-pixel motion estimation

      let po = frame_bo.to_luma_plane_offset();
      let org_region: &PlaneRegion<T> =
        &ts.input.planes[0].region(Area::StartingAt { x: po.x, y: po.y });
      let p_ref: &Plane<T> = &rec.frame.planes[0];

      let best = full_pixel_me(
        fi,
        ts,
        org_region,
        p_ref,
        tile_bo,
        lambda,
        iter::once(cmv).collect(),
        pmv,
        mvx_min,
        mvx_max,
        mvy_min,
        mvy_max,
        bsize,
        ref_frame,
      );

      let sad = best.sad;

      let mut best = MVSearchResult { mv: best.mv, cost: best.cost };

      let use_satd: bool = fi.config.speed_settings.use_satd_subpel;
      if use_satd {
        best.cost = get_fullpel_mv_rd_cost(
          fi,
          po,
          org_region,
          p_ref,
          fi.sequence.bit_depth,
          pmv,
          lambda,
          use_satd,
          mvx_min,
          mvx_max,
          mvy_min,
          mvy_max,
          bsize,
          best.mv,
        )
        .0;
      }

      sub_pixel_me(
        fi, po, org_region, p_ref, lambda, pmv, mvx_min, mvx_max, mvy_min,
        mvy_max, bsize, use_satd, &mut best, ref_frame,
      );

      (best.mv, sad)
    }

    None => (MotionVector::default(), u32::MAX),
  }
}

pub fn estimate_motion_ss2<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
  tile_bo: TileBlockOffset, pmvs: &[Option<MotionVector>; 3],
  ref_frame: RefType,
) -> Option<MotionVector> {
  if let Some(ref rec) =
    fi.rec_buffer.frames[fi.ref_frames[ref_frame.to_index()] as usize]
  {
    let blk_w = bsize.width();
    let blk_h = bsize.height();
    let tile_bo_adj =
      adjust_bo(tile_bo, ts.mi_width, ts.mi_height, blk_w, blk_h);
    let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
    let (mvx_min, mvx_max, mvy_min, mvy_max) =
      get_mv_range(fi.w_in_b, fi.h_in_b, frame_bo_adj, blk_w, blk_h);

    let global_mv = [MotionVector { row: 0, col: 0 }; 2];

    // Divide by 4 to account for subsampling, 0.125 is a fudge factor
    let lambda = (fi.me_lambda * 256.0 / 4.0 * 0.125) as u32;

    let best = me_ss2(
      fi,
      ts,
      pmvs,
      tile_bo_adj,
      rec,
      global_mv,
      lambda,
      mvx_min,
      mvx_max,
      mvy_min,
      mvy_max,
      bsize,
      ref_frame,
    );

    Some(MotionVector { row: best.mv.row * 2, col: best.mv.col * 2 })
  } else {
    None
  }
}

pub fn estimate_motion<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
  tile_bo: TileBlockOffset, pmvs: &[Option<MotionVector>], ref_frame: RefType,
) -> Option<MotionVector> {
  debug_assert!(pmvs.len() <= 7);

  if let Some(ref rec) =
    fi.rec_buffer.frames[fi.ref_frames[ref_frame.to_index()] as usize]
  {
    let blk_w = bsize.width();
    let blk_h = bsize.height();
    let tile_bo_adj =
      adjust_bo(tile_bo, ts.mi_width, ts.mi_height, blk_w, blk_h);
    let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
    let (mvx_min, mvx_max, mvy_min, mvy_max) =
      get_mv_range(fi.w_in_b, fi.h_in_b, frame_bo_adj, blk_w, blk_h);

    let global_mv = [MotionVector { row: 0, col: 0 }; 2];

    // 0.5 is a fudge factor
    let lambda = (fi.me_lambda * 256.0 * 0.5) as u32;

    let po = frame_bo_adj.to_luma_plane_offset();
    let org_region =
      &ts.input.planes[0].region(Area::StartingAt { x: po.x, y: po.y });

    let FullpelSearchResult { mv: best_mv, .. } = full_pixel_me(
      fi,
      ts,
      org_region,
      &rec.frame.planes[0],
      tile_bo_adj,
      lambda,
      pmvs.iter().cloned().filter_map(identity).collect(),
      global_mv,
      mvx_min,
      mvx_max,
      mvy_min,
      mvy_max,
      bsize,
      ref_frame,
    );

    Some(MotionVector { row: best_mv.row, col: best_mv.col })
  } else {
    None
  }
}

fn full_pixel_me<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
  org_region: &PlaneRegion<T>, p_ref: &Plane<T>, tile_bo: TileBlockOffset,
  lambda: u32, cmvs: ArrayVec<[MotionVector; 7]>, pmv: [MotionVector; 2],
  mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
  bsize: BlockSize, ref_frame: RefType,
) -> FullpelSearchResult {
  let tile_mvs = &ts.mvs[ref_frame.to_index()].as_const();
  let frame_ref =
    fi.rec_buffer.frames[fi.ref_frames[0] as usize].as_ref().map(Arc::as_ref);
  let predictors = get_subset_predictors(
    tile_bo,
    cmvs,
    tile_mvs,
    frame_ref,
    ref_frame.to_index(),
    bsize,
  );

  let frame_bo = ts.to_frame_block_offset(tile_bo);
  let po = frame_bo.to_luma_plane_offset();
  fullpel_diamond_me_search(
    fi,
    po,
    org_region,
    p_ref,
    &predictors,
    fi.sequence.bit_depth,
    pmv,
    lambda,
    mvx_min,
    mvx_max,
    mvy_min,
    mvy_max,
    bsize,
  )
}

fn sub_pixel_me<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, lambda: u32, pmv: [MotionVector; 2], mvx_min: isize,
  mvx_max: isize, mvy_min: isize, mvy_max: isize, bsize: BlockSize,
  use_satd: bool, best: &mut MVSearchResult, ref_frame: RefType,
) {
  subpel_diamond_me_search(
    fi,
    po,
    org_region,
    p_ref,
    fi.sequence.bit_depth,
    pmv,
    lambda,
    mvx_min,
    mvx_max,
    mvy_min,
    mvy_max,
    bsize,
    use_satd,
    best,
    ref_frame,
  );
}

fn me_ss2<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
  pmvs: &[Option<MotionVector>; 3], tile_bo_adj: TileBlockOffset,
  rec: &ReferenceFrame<T>, global_mv: [MotionVector; 2], lambda: u32,
  mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
  bsize: BlockSize, ref_frame: RefType,
) -> FullpelSearchResult {
  let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
  let po = PlaneOffset {
    x: (frame_bo_adj.0.x as isize) << BLOCK_TO_PLANE_SHIFT >> 1,
    y: (frame_bo_adj.0.y as isize) << BLOCK_TO_PLANE_SHIFT >> 1,
  };
  let org_region =
    &ts.input_hres.region(Area::StartingAt { x: po.x, y: po.y });

  let tile_mvs = &ts.mvs[ref_frame.to_index()].as_const();
  let frame_ref =
    fi.rec_buffer.frames[fi.ref_frames[0] as usize].as_ref().map(Arc::as_ref);

  let mut predictors = get_subset_predictors::<T>(
    tile_bo_adj,
    pmvs.iter().cloned().filter_map(identity).collect(),
    tile_mvs,
    frame_ref,
    ref_frame.to_index(),
    bsize,
  );

  for predictor in &mut predictors {
    predictor.row >>= 1;
    predictor.col >>= 1;
  }

  fullpel_diamond_me_search(
    fi,
    po,
    org_region,
    &rec.input_hres,
    &predictors,
    fi.sequence.bit_depth,
    global_mv,
    lambda,
    mvx_min >> 1,
    mvx_max >> 1,
    mvy_min >> 1,
    mvy_max >> 1,
    BlockSize::from_width_and_height(bsize.width() >> 1, bsize.height() >> 1),
  )
}

fn get_best_predictor<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, predictors: &[MotionVector], bit_depth: usize,
  pmv: [MotionVector; 2], lambda: u32, mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize,
) -> FullpelSearchResult {
  let mut best: FullpelSearchResult = FullpelSearchResult {
    mv: MotionVector::default(),
    cost: u64::MAX,
    sad: u32::MAX,
  };

  for &init_mv in predictors.iter() {
    let cost = get_fullpel_mv_rd_cost(
      fi, po, org_region, p_ref, bit_depth, pmv, lambda, false, mvx_min,
      mvx_max, mvy_min, mvy_max, bsize, init_mv,
    );

    if cost.0 < best.cost {
      best.mv = init_mv;
      best.cost = cost.0;
      best.sad = cost.1;
    }
  }

  best
}

fn fullpel_diamond_me_search<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, predictors: &[MotionVector], bit_depth: usize,
  pmv: [MotionVector; 2], lambda: u32, mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize,
) -> FullpelSearchResult {
  let diamond_pattern = [(1i16, 0i16), (0, 1), (-1, 0), (0, -1)];
  let (mut diamond_radius, diamond_radius_end) = (4u8, 3u8);

  let mut center = get_best_predictor(
    fi, po, org_region, p_ref, predictors, bit_depth, pmv, lambda, mvx_min,
    mvx_max, mvy_min, mvy_max, bsize,
  );

  loop {
    let mut best_diamond: FullpelSearchResult = FullpelSearchResult {
      mv: MotionVector::default(),
      sad: u32::MAX,
      cost: u64::MAX,
    };

    for p in diamond_pattern.iter() {
      let cand_mv = MotionVector {
        row: center.mv.row + (p.0 << diamond_radius),
        col: center.mv.col + (p.1 << diamond_radius),
      };

      let rd_cost = get_fullpel_mv_rd_cost(
        fi, po, org_region, p_ref, bit_depth, pmv, lambda, false, mvx_min,
        mvx_max, mvy_min, mvy_max, bsize, cand_mv,
      );

      if rd_cost.0 < best_diamond.cost {
        best_diamond.mv = cand_mv;
        best_diamond.cost = rd_cost.0;
        best_diamond.sad = rd_cost.1;
      }
    }

    if center.cost <= best_diamond.cost {
      if diamond_radius == diamond_radius_end {
        break;
      } else {
        diamond_radius -= 1;
      }
    } else {
      center = best_diamond;
    }
  }

  assert!(center.cost < std::u64::MAX);

  center
}

fn subpel_diamond_me_search<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  _p_ref: &Plane<T>, bit_depth: usize, pmv: [MotionVector; 2], lambda: u32,
  mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
  bsize: BlockSize, use_satd: bool, center: &mut MVSearchResult,
  ref_frame: RefType,
) {
  use crate::util::Aligned;

  let cfg = PlaneConfig::new(
    bsize.width(),
    bsize.height(),
    0,
    0,
    0,
    0,
    std::mem::size_of::<T>(),
  );

  let mut buf: Aligned<[T; 128 * 128]> = Aligned::uninitialized();

  let diamond_pattern = [(1i16, 0i16), (0, 1), (-1, 0), (0, -1)];
  let (mut diamond_radius, diamond_radius_end, mut tmp_region) = {
    let rect = Rect { x: 0, y: 0, width: cfg.width, height: cfg.height };

    // start at 1/2 pel and end at 1/4 or 1/8 pel
    (
      2u8,
      if fi.allow_high_precision_mv { 0u8 } else { 1u8 },
      PlaneRegionMut::from_slice(&mut buf.data, &cfg, rect),
    )
  };

  loop {
    let mut best_diamond: MVSearchResult =
      MVSearchResult { mv: MotionVector::default(), cost: u64::MAX };

    for p in diamond_pattern.iter() {
      let cand_mv = MotionVector {
        row: center.mv.row + (p.0 << diamond_radius),
        col: center.mv.col + (p.1 << diamond_radius),
      };

      let rd_cost = get_subpel_mv_rd_cost(
        fi,
        po,
        org_region,
        bit_depth,
        pmv,
        lambda,
        use_satd,
        mvx_min,
        mvx_max,
        mvy_min,
        mvy_max,
        bsize,
        cand_mv,
        &mut tmp_region,
        ref_frame,
      );

      if rd_cost < best_diamond.cost {
        best_diamond.mv = cand_mv;
        best_diamond.cost = rd_cost;
      }
    }

    if center.cost <= best_diamond.cost {
      if diamond_radius == diamond_radius_end {
        break;
      } else {
        diamond_radius -= 1;
      }
    } else {
      *center = best_diamond;
    }
  }

  assert!(center.cost < std::u64::MAX);
}

#[inline]
fn get_fullpel_mv_rd_cost<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, bit_depth: usize, pmv: [MotionVector; 2], lambda: u32,
  use_satd: bool, mvx_min: isize, mvx_max: isize, mvy_min: isize,
  mvy_max: isize, bsize: BlockSize, cand_mv: MotionVector,
) -> (u64, u32) {
  if (cand_mv.col as isize) < mvx_min
    || (cand_mv.col as isize) > mvx_max
    || (cand_mv.row as isize) < mvy_min
    || (cand_mv.row as isize) > mvy_max
  {
    return (u64::MAX, u32::MAX);
  }

  // Full pixel motion vector
  let plane_ref = p_ref.region(Area::StartingAt {
    x: po.x + (cand_mv.col / 8) as isize,
    y: po.y + (cand_mv.row / 8) as isize,
  });
  compute_mv_rd_cost(
    fi, pmv, lambda, use_satd, bit_depth, bsize, cand_mv, org_region,
    &plane_ref,
  )
}

fn get_subpel_mv_rd_cost<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  bit_depth: usize, pmv: [MotionVector; 2], lambda: u32, use_satd: bool,
  mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
  bsize: BlockSize, cand_mv: MotionVector, tmp_region: &mut PlaneRegionMut<T>,
  ref_frame: RefType,
) -> u64 {
  if (cand_mv.col as isize) < mvx_min
    || (cand_mv.col as isize) > mvx_max
    || (cand_mv.row as isize) < mvy_min
    || (cand_mv.row as isize) > mvy_max
  {
    return std::u64::MAX;
  }

  let tile_rect = TileRect {
    x: 0,
    y: 0,
    width: tmp_region.plane_cfg.width,
    height: tmp_region.plane_cfg.height,
  };
  PredictionMode::NEWMV.predict_inter_single(
    fi,
    tile_rect,
    0,
    po,
    tmp_region,
    bsize.width(),
    bsize.height(),
    ref_frame,
    cand_mv,
  );
  let plane_ref = tmp_region.as_const();
  compute_mv_rd_cost(
    fi, pmv, lambda, use_satd, bit_depth, bsize, cand_mv, org_region,
    &plane_ref,
  )
  .0
}

#[inline(always)]
fn compute_mv_rd_cost<T: Pixel>(
  fi: &FrameInvariants<T>, pmv: [MotionVector; 2], lambda: u32,
  use_satd: bool, bit_depth: usize, bsize: BlockSize, cand_mv: MotionVector,
  plane_org: &PlaneRegion<'_, T>, plane_ref: &PlaneRegion<'_, T>,
) -> (u64, u32) {
  let sad = if use_satd {
    get_satd(plane_org, plane_ref, bsize, bit_depth, fi.cpu_feature_level)
  } else {
    get_sad(plane_org, plane_ref, bsize, bit_depth, fi.cpu_feature_level)
  };

  let rate1 = get_mv_rate(cand_mv, pmv[0], fi.allow_high_precision_mv);
  let rate2 = get_mv_rate(cand_mv, pmv[1], fi.allow_high_precision_mv);
  let rate = rate1.min(rate2 + 1);

  (256 * sad as u64 + rate as u64 * lambda as u64, sad)
}

fn full_search<T: Pixel>(
  fi: &FrameInvariants<T>, x_lo: isize, x_hi: isize, y_lo: isize, y_hi: isize,
  bsize: BlockSize, org_region: &PlaneRegion<T>, p_ref: &Plane<T>,
  po: PlaneOffset, step: usize, lambda: u32, pmv: [MotionVector; 2],
) -> FullpelSearchResult {
  let blk_w = bsize.width();
  let blk_h = bsize.height();
  let search_region = p_ref.region(Area::Rect {
    x: x_lo,
    y: y_lo,
    width: (x_hi - x_lo) as usize + blk_w,
    height: (y_hi - y_lo) as usize + blk_h,
  });

  let mut best: FullpelSearchResult = FullpelSearchResult {
    mv: MotionVector::default(),
    sad: u32::MAX,
    cost: u64::MAX,
  };

  // Select rectangular regions within search region with vert+horz windows
  for vert_window in search_region.vert_windows(blk_h).step_by(step) {
    for ref_window in vert_window.horz_windows(blk_w).step_by(step) {
      let &Rect { x, y, .. } = ref_window.rect();

      let mv = MotionVector {
        row: 8 * (y as i16 - po.y as i16),
        col: 8 * (x as i16 - po.x as i16),
      };

      let cost_sad = compute_mv_rd_cost(
        fi,
        pmv,
        lambda,
        false,
        fi.sequence.bit_depth,
        bsize,
        mv,
        &org_region,
        &ref_window,
      );

      if cost_sad.0 < best.cost {
        best.cost = cost_sad.0;
        best.mv = mv;
      }
    }
  }

  best
}

// Adjust block offset such that entire block lies within boundaries
// Align to block width, to admit aligned SAD instructions
fn adjust_bo(
  bo: TileBlockOffset, mi_width: usize, mi_height: usize, blk_w: usize,
  blk_h: usize,
) -> TileBlockOffset {
  TileBlockOffset(BlockOffset {
    x: (bo.0.x as isize).min(mi_width as isize - blk_w as isize / 4).max(0)
      as usize
      & !(blk_w / 4 - 1),
    y: (bo.0.y as isize).min(mi_height as isize - blk_h as isize / 4).max(0)
      as usize,
  })
}

#[inline(always)]
fn get_mv_rate(
  a: MotionVector, b: MotionVector, allow_high_precision_mv: bool,
) -> u32 {
  #[inline(always)]
  fn diff_to_rate(diff: i16, allow_high_precision_mv: bool) -> u32 {
    let d = if allow_high_precision_mv { diff } else { diff >> 1 };
    2 * d.abs().ilog() as u32
  }

  diff_to_rate(a.row - b.row, allow_high_precision_mv)
    + diff_to_rate(a.col - b.col, allow_high_precision_mv)
}

pub fn estimate_motion_ss4<T: Pixel>(
  fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
  ref_idx: usize, tile_bo: TileBlockOffset,
) -> Option<MotionVector> {
  if let Some(ref rec) = fi.rec_buffer.frames[ref_idx] {
    let blk_w = bsize.width();
    let blk_h = bsize.height();
    let tile_bo_adj =
      adjust_bo(tile_bo, ts.mi_width, ts.mi_height, blk_w, blk_h);
    let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
    let po = PlaneOffset {
      x: (frame_bo_adj.0.x as isize) << BLOCK_TO_PLANE_SHIFT >> 2,
      y: (frame_bo_adj.0.y as isize) << BLOCK_TO_PLANE_SHIFT >> 2,
    };

    let range_x = 192 * fi.me_range_scale as isize;
    let range_y = 64 * fi.me_range_scale as isize;
    let (mvx_min, mvx_max, mvy_min, mvy_max) =
      get_mv_range(fi.w_in_b, fi.h_in_b, frame_bo_adj, blk_w, blk_h);
    let x_lo = po.x + (((-range_x).max(mvx_min / 8)) >> 2);
    let x_hi = po.x + (((range_x).min(mvx_max / 8)) >> 2);
    let y_lo = po.y + (((-range_y).max(mvy_min / 8)) >> 2);
    let y_hi = po.y + (((range_y).min(mvy_max / 8)) >> 2);

    // Divide by 16 to account for subsampling, 0.125 is a fudge factor
    let lambda = (fi.me_lambda * 256.0 / 16.0 * 0.125) as u32;

    let FullpelSearchResult { mv: best_mv, .. } = full_search(
      fi,
      x_lo,
      x_hi,
      y_lo,
      y_hi,
      BlockSize::from_width_and_height(blk_w >> 2, blk_h >> 2),
      &ts.input_qres.region(Area::StartingAt { x: po.x, y: po.y }),
      &rec.input_qres,
      po,
      1,
      lambda,
      [MotionVector::default(); 2],
    );

    Some(MotionVector { row: best_mv.row * 4, col: best_mv.col * 4 })
  } else {
    None
  }
}
