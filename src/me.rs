// Copyright (c) 2017-2019, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use crate::context::{
  BlockOffset, PlaneBlockOffset, TileBlockOffset, BLOCK_TO_PLANE_SHIFT,
  MI_SIZE,
};
use crate::dist::*;
use crate::encoder::ReferenceFrame;
use crate::frame::*;
use crate::mc::MotionVector;
use crate::partition::*;
use crate::predict::PredictionMode;
use crate::tiling::*;
use crate::util::Pixel;
use crate::FrameInvariants;

use arrayvec::*;

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
pub struct MVSearchResult {
  mv: MotionVector,
  cost: u64,
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
  ref_frame_id: usize,
) -> ArrayVec<[MotionVector; 17]> {
  let mut predictors = ArrayVec::<[_; 17]>::new();

  // Add a candidate predictor, aligning to fullpel and filtering out zero mvs.
  let add_cand = |predictors: &mut ArrayVec<[MotionVector; 17]>,
                  cand_mv: MotionVector| {
    let cand_mv = cand_mv.quantize_to_fullpel();
    if !cand_mv.is_zero() {
      predictors.push(cand_mv)
    }
  };

  // Zero motion vector, don't use add_cand since it skips zero vectors.
  predictors.push(MotionVector::default());

  // Coarse motion estimation.
  for mv in cmvs {
    add_cand(&mut predictors, mv);
  }

  // EPZS subset A and B predictors.

  let mut median_preds = ArrayVec::<[_; 3]>::new();
  if tile_bo.0.x > 0 {
    let left = tile_mvs[tile_bo.0.y][tile_bo.0.x - 1];
    median_preds.push(left);
    add_cand(&mut predictors, left);
  }
  if tile_bo.0.y > 0 {
    let top = tile_mvs[tile_bo.0.y - 1][tile_bo.0.x];
    median_preds.push(top);
    add_cand(&mut predictors, top);

    if tile_bo.0.x < tile_mvs.cols() - 1 {
      let top_right = tile_mvs[tile_bo.0.y - 1][tile_bo.0.x + 1];
      median_preds.push(top_right);
      add_cand(&mut predictors, top_right);
    }
  }

  if !median_preds.is_empty() {
    let mut median_mv = MotionVector::default();
    for mv in median_preds.iter() {
      median_mv = median_mv + *mv;
    }
    median_mv = median_mv / (median_preds.len() as i16);
    add_cand(&mut predictors, median_mv);
  }

  // EPZS subset C predictors.

  if let Some(frame_ref) = frame_ref_opt {
    let prev_frame_mvs = &frame_ref.frame_mvs[ref_frame_id];

    let frame_bo = PlaneBlockOffset(BlockOffset {
      x: tile_mvs.x() + tile_bo.0.x,
      y: tile_mvs.y() + tile_bo.0.y,
    });
    if frame_bo.0.x > 0 {
      let left = prev_frame_mvs[frame_bo.0.y][frame_bo.0.x - 1];
      add_cand(&mut predictors, left);
    }
    if frame_bo.0.y > 0 {
      let top = prev_frame_mvs[frame_bo.0.y - 1][frame_bo.0.x];
      add_cand(&mut predictors, top);
    }
    if frame_bo.0.x < prev_frame_mvs.cols - 1 {
      let right = prev_frame_mvs[frame_bo.0.y][frame_bo.0.x + 1];
      add_cand(&mut predictors, right);
    }
    if frame_bo.0.y < prev_frame_mvs.rows - 1 {
      let bottom = prev_frame_mvs[frame_bo.0.y + 1][frame_bo.0.x];
      add_cand(&mut predictors, bottom);
    }

    let previous = prev_frame_mvs[frame_bo.0.y][frame_bo.0.x];
    add_cand(&mut predictors, previous);
  }

  predictors
}

pub trait MotionEstimation {
  fn full_pixel_me<T: Pixel>(
    fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
    org_region: &PlaneRegion<T>, p_ref: &Plane<T>, tile_bo: TileBlockOffset,
    lambda: u32, cmvs: ArrayVec<[MotionVector; 7]>, pmv: [MotionVector; 2],
    mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
    bsize: BlockSize, ref_frame: RefType,
  ) -> MVSearchResult;

  fn sub_pixel_me<T: Pixel>(
    fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
    p_ref: &Plane<T>, lambda: u32, pmv: [MotionVector; 2], mvx_min: isize,
    mvx_max: isize, mvy_min: isize, mvy_max: isize, bsize: BlockSize,
    use_satd: bool, best: &mut MVSearchResult, ref_frame: RefType,
  );

  fn motion_estimation<T: Pixel>(
    fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
    tile_bo: TileBlockOffset, ref_frame: RefType, cmv: MotionVector,
    pmv: [MotionVector; 2],
  ) -> MotionVector {
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

        let mut best = Self::full_pixel_me(
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

        let use_satd: bool = fi.config.speed_settings.use_satd_subpel;
        if use_satd {
          best.cost = get_mv_rd_cost(
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
            None,
            ref_frame,
          );
        }

        Self::sub_pixel_me(
          fi, po, org_region, p_ref, lambda, pmv, mvx_min, mvx_max, mvy_min,
          mvy_max, bsize, use_satd, &mut best, ref_frame,
        );

        best.mv
      }

      None => MotionVector::default(),
    }
  }

  fn estimate_motion_ss2<T: Pixel>(
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

      let best = Self::me_ss2(
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

  fn me_ss2<T: Pixel>(
    fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
    pmvs: &[Option<MotionVector>; 3], tile_bo_adj: TileBlockOffset,
    rec: &ReferenceFrame<T>, global_mv: [MotionVector; 2], lambda: u32,
    mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
    bsize: BlockSize, ref_frame: RefType,
  ) -> MVSearchResult;

  fn estimate_motion<T: Pixel>(
    fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>, bsize: BlockSize,
    tile_bo: TileBlockOffset, pmvs: &[Option<MotionVector>],
    ref_frame: RefType,
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

      let MVSearchResult { mv: best_mv, .. } = Self::full_pixel_me(
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
}

pub struct DiamondSearch {}

impl MotionEstimation for DiamondSearch {
  fn full_pixel_me<T: Pixel>(
    fi: &FrameInvariants<T>, ts: &TileStateMut<'_, T>,
    org_region: &PlaneRegion<T>, p_ref: &Plane<T>, tile_bo: TileBlockOffset,
    lambda: u32, cmvs: ArrayVec<[MotionVector; 7]>, pmv: [MotionVector; 2],
    mvx_min: isize, mvx_max: isize, mvy_min: isize, mvy_max: isize,
    bsize: BlockSize, ref_frame: RefType,
  ) -> MVSearchResult {
    let tile_mvs = &ts.mvs[ref_frame.to_index()].as_const();
    let frame_ref = fi.rec_buffer.frames[fi.ref_frames[0] as usize]
      .as_ref()
      .map(Arc::as_ref);
    let predictors = get_subset_predictors(
      tile_bo,
      cmvs,
      tile_mvs,
      frame_ref,
      ref_frame.to_index(),
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
      ref_frame,
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
  ) -> MVSearchResult {
    let frame_bo_adj = ts.to_frame_block_offset(tile_bo_adj);
    let po = PlaneOffset {
      x: (frame_bo_adj.0.x as isize) << BLOCK_TO_PLANE_SHIFT >> 1,
      y: (frame_bo_adj.0.y as isize) << BLOCK_TO_PLANE_SHIFT >> 1,
    };
    let org_region =
      &ts.input_hres.region(Area::StartingAt { x: po.x, y: po.y });

    let tile_mvs = &ts.mvs[ref_frame.to_index()].as_const();
    let frame_ref = fi.rec_buffer.frames[fi.ref_frames[0] as usize]
      .as_ref()
      .map(Arc::as_ref);

    let mut predictors = get_subset_predictors::<T>(
      tile_bo_adj,
      pmvs.iter().cloned().filter_map(identity).collect(),
      tile_mvs,
      frame_ref,
      ref_frame.to_index(),
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
      BlockSize::from_width_and_height(
        bsize.width() >> 1,
        bsize.height() >> 1,
      ),
      ref_frame,
    )
  }
}

fn get_best_predictor<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, predictors: &[MotionVector], bit_depth: usize,
  pmv: [MotionVector; 2], lambda: u32, mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize, ref_frame: RefType,
) -> MVSearchResult {
  let mut best: MVSearchResult =
    MVSearchResult { mv: MotionVector::default(), cost: u64::MAX };

  for &init_mv in predictors.iter() {
    let cost = get_mv_rd_cost(
      fi, po, org_region, p_ref, bit_depth, pmv, lambda, false, mvx_min,
      mvx_max, mvy_min, mvy_max, bsize, init_mv, None, ref_frame,
    );

    if cost < best.cost {
      best.mv = init_mv;
      best.cost = cost;
    }
  }

  best
}

fn fullpel_diamond_me_search<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, predictors: &[MotionVector], bit_depth: usize,
  pmv: [MotionVector; 2], lambda: u32, mvx_min: isize, mvx_max: isize,
  mvy_min: isize, mvy_max: isize, bsize: BlockSize, ref_frame: RefType,
) -> MVSearchResult {
  let diamond_pattern = [(1i16, 0i16), (0, 1), (-1, 0), (0, -1)];
  let (mut diamond_radius, diamond_radius_end) = (16i16, 8i16);

  let mut center = get_best_predictor(
    fi, po, org_region, p_ref, predictors, bit_depth, pmv, lambda, mvx_min,
    mvx_max, mvy_min, mvy_max, bsize, ref_frame,
  );

  loop {
    let mut best_diamond: MVSearchResult =
      MVSearchResult { mv: MotionVector::default(), cost: u64::MAX };

    for p in diamond_pattern.iter() {
      let cand_mv = MotionVector {
        row: center.mv.row + diamond_radius * p.0,
        col: center.mv.col + diamond_radius * p.1,
      };

      if !((cand_mv.col as isize) < mvx_min
        || (cand_mv.col as isize) > mvx_max
        || (cand_mv.row as isize) < mvy_min
        || (cand_mv.row as isize) > mvy_max)
      {
        let ref_region = p_ref.region(Area::StartingAt {
          x: po.x + (cand_mv.col / 8) as isize,
          y: po.y + (cand_mv.row / 8) as isize,
        });

        let rd_cost = compute_mv_rd_cost(
          fi,
          pmv,
          lambda,
          false,
          bit_depth,
          bsize,
          cand_mv,
          org_region,
          &ref_region,
        );

        if rd_cost < best_diamond.cost {
          best_diamond.mv = cand_mv;
          best_diamond.cost = rd_cost;
        }
      }
    }

    if center.cost <= best_diamond.cost {
      if diamond_radius == diamond_radius_end {
        break;
      } else {
        diamond_radius /= 2;
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
  p_ref: &Plane<T>, bit_depth: usize, pmv: [MotionVector; 2], lambda: u32,
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
  let (mut diamond_radius, diamond_radius_end, mut tmp_region_opt) = {
    let rect = Rect { x: 0, y: 0, width: cfg.width, height: cfg.height };

    // Sub-pixel motion estimation
    (
      4i16,
      if fi.allow_high_precision_mv { 1i16 } else { 2i16 },
      Some(PlaneRegionMut::from_slice(&mut buf.data, &cfg, rect)),
    )
  };

  loop {
    let mut best_diamond: MVSearchResult =
      MVSearchResult { mv: MotionVector::default(), cost: u64::MAX };

    for p in diamond_pattern.iter() {
      let cand_mv = MotionVector {
        row: center.mv.row + diamond_radius * p.0,
        col: center.mv.col + diamond_radius * p.1,
      };

      let rd_cost = get_mv_rd_cost(
        fi,
        po,
        org_region,
        p_ref,
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
        tmp_region_opt.as_mut(),
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
        diamond_radius /= 2;
      }
    } else {
      *center = best_diamond;
    }
  }

  assert!(center.cost < std::u64::MAX);
}

fn get_mv_rd_cost<T: Pixel>(
  fi: &FrameInvariants<T>, po: PlaneOffset, org_region: &PlaneRegion<T>,
  p_ref: &Plane<T>, bit_depth: usize, pmv: [MotionVector; 2], lambda: u32,
  use_satd: bool, mvx_min: isize, mvx_max: isize, mvy_min: isize,
  mvy_max: isize, bsize: BlockSize, cand_mv: MotionVector,
  tmp_region_opt: Option<&mut PlaneRegionMut<T>>, ref_frame: RefType,
) -> u64 {
  if (cand_mv.col as isize) < mvx_min || (cand_mv.col as isize) > mvx_max {
    return std::u64::MAX;
  }
  if (cand_mv.row as isize) < mvy_min || (cand_mv.row as isize) > mvy_max {
    return std::u64::MAX;
  }

  if let Some(region) = tmp_region_opt {
    let tile_rect = TileRect {
      x: 0,
      y: 0,
      width: region.plane_cfg.width,
      height: region.plane_cfg.height,
    };
    PredictionMode::NEWMV.predict_inter_single(
      fi,
      tile_rect,
      0,
      po,
      region,
      bsize.width(),
      bsize.height(),
      ref_frame,
      cand_mv,
    );
    let plane_ref = region.as_const();
    compute_mv_rd_cost(
      fi, pmv, lambda, use_satd, bit_depth, bsize, cand_mv, org_region,
      &plane_ref,
    )
  } else {
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
}

#[inline(always)]
fn compute_mv_rd_cost<T: Pixel>(
  fi: &FrameInvariants<T>, pmv: [MotionVector; 2], lambda: u32,
  use_satd: bool, bit_depth: usize, bsize: BlockSize, cand_mv: MotionVector,
  plane_org: &PlaneRegion<'_, T>, plane_ref: &PlaneRegion<'_, T>,
) -> u64 {
  let sad = if use_satd {
    get_satd(plane_org, plane_ref, bsize, bit_depth, fi.cpu_feature_level)
  } else {
    get_sad(plane_org, plane_ref, bsize, bit_depth, fi.cpu_feature_level)
  };

  let rate1 = get_mv_rate(cand_mv, pmv[0], fi.allow_high_precision_mv);
  let rate2 = get_mv_rate(cand_mv, pmv[1], fi.allow_high_precision_mv);
  let rate = rate1.min(rate2 + 1);

  256 * sad as u64 + rate as u64 * lambda as u64
}

fn full_search<T: Pixel>(
  fi: &FrameInvariants<T>, x_lo: isize, x_hi: isize, y_lo: isize, y_hi: isize,
  bsize: BlockSize, p_org: &Plane<T>, p_ref: &Plane<T>,
  best_mv: &mut MotionVector, lowest_cost: &mut u64, po: PlaneOffset,
  step: usize, lambda: u32, pmv: [MotionVector; 2],
) {
  let blk_w = bsize.width();
  let blk_h = bsize.height();
  let plane_org = p_org.region(Area::StartingAt { x: po.x, y: po.y });
  let search_region = p_ref.region(Area::Rect {
    x: x_lo,
    y: y_lo,
    width: (x_hi - x_lo) as usize + blk_w,
    height: (y_hi - y_lo) as usize + blk_h,
  });

  // Select rectangular regions within search region with vert+horz windows
  for vert_window in search_region.vert_windows(blk_h).step_by(step) {
    for ref_window in vert_window.horz_windows(blk_w).step_by(step) {
      let &Rect { x, y, .. } = ref_window.rect();

      let mv = MotionVector {
        row: 8 * (y as i16 - po.y as i16),
        col: 8 * (x as i16 - po.x as i16),
      };

      let cost = compute_mv_rd_cost(
        fi,
        pmv,
        lambda,
        false,
        fi.sequence.bit_depth,
        bsize,
        mv,
        &plane_org,
        &ref_window,
      );

      if cost < *lowest_cost {
        *lowest_cost = cost;
        *best_mv = mv;
      }
    }
  }
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
    if d == 0 {
      0
    } else {
      2 * (16 - d.abs().leading_zeros())
    }
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

    let mut lowest_cost = std::u64::MAX;
    let mut best_mv = MotionVector::default();

    // Divide by 16 to account for subsampling, 0.125 is a fudge factor
    let lambda = (fi.me_lambda * 256.0 / 16.0 * 0.125) as u32;

    full_search(
      fi,
      x_lo,
      x_hi,
      y_lo,
      y_hi,
      BlockSize::from_width_and_height(blk_w >> 2, blk_h >> 2),
      ts.input_qres,
      &rec.input_qres,
      &mut best_mv,
      &mut lowest_cost,
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
