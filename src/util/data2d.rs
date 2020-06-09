// Copyright (c) 2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use crate::util::{
  RowsIter, RowsIterMut, Slice2D, Slice2DMut, Slice2DRawParts, SliceIndex2D,
};
use std::fmt;
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct Data2D<T> {
  data: Vec<T>,
  width: usize,
  height: usize,
}

impl<T> Data2D<T>
where
  T: Default,
{
  #[inline(always)]
  pub fn new(width: usize, height: usize) -> Self {
    let len = width * height;
    let mut data = Vec::with_capacity(len);
    data.resize_with(len, Default::default);
    Self { data, width, height }
  }
}

impl<T> Data2D<T> {
  #[inline(always)]
  pub const fn width(&self) -> usize {
    self.width
  }

  #[inline(always)]
  pub const fn height(&self) -> usize {
    self.height
  }

  /// Same as the width() function.
  #[inline(always)]
  pub const fn cols(&self) -> usize {
    self.width
  }

  /// Same as the height() function.
  #[inline(always)]
  pub const fn rows(&self) -> usize {
    self.height
  }

  fn slice_raw_parts(&self) -> Slice2DRawParts<T> {
    Slice2DRawParts {
      ptr: self.data.as_ptr() as *mut T,
      width: self.width,
      height: self.height,
      stride: self.width,
    }
  }

  #[inline(always)]
  pub fn slice(&self) -> Slice2D<'_, T> {
    unsafe { Slice2D::from_raw_parts(self.slice_raw_parts()) }
  }

  #[inline(always)]
  pub fn mut_slice(&mut self) -> Slice2DMut<'_, T> {
    unsafe { Slice2DMut::from_raw_parts(self.slice_raw_parts()) }
  }

  #[inline(always)]
  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    unsafe { RowsIter::new(self.slice_raw_parts()) }
  }

  #[inline(always)]
  pub fn rows_iter_mut(&mut self) -> RowsIterMut<'_, T> {
    unsafe { RowsIterMut::new(self.slice_raw_parts()) }
  }
}

impl<T, I: SliceIndex2D<T>> Index<I> for Data2D<T> {
  type Output = I::Output;
  #[inline(always)]
  fn index(&self, index: I) -> &Self::Output {
    unsafe { SliceIndex2D::index_raw(index, self.slice_raw_parts()) }
  }
}

impl<T, I: SliceIndex2D<T>> IndexMut<I> for Data2D<T> {
  #[inline(always)]
  fn index_mut(&mut self, index: I) -> &mut Self::Output {
    unsafe { SliceIndex2D::index_raw_mut(index, self.slice_raw_parts()) }
  }
}

impl<T> fmt::Debug for Data2D<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Data2D {{ ptr: {:?}, size: {}x{} }}",
      self.data.as_ptr(),
      self.width,
      self.height
    )
  }
}