// Copyright (c) 2019-2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

// Some of the documentation for this class is modified from the rust library.
// The rust library is licenced under MIT (http://opensource.org/licenses/MIT)
// or Apache 2.0 (http://www.apache.org/licenses/LICENSE-2.0)).

#![allow(unused)]

use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};
use std::{fmt, mem, slice};

pub struct Slice2DRawParts<T> {
  // TODO: It would be desirable to use NonNull in place of a simple pointer,
  // but we rely on using 0x0 vecs with no allocation, thus we can't be
  // guaranteed a non-null pointer.
  pub ptr: *mut T,
  pub width: usize,
  pub height: usize,
  pub stride: usize,
}

// Implement copy and clone regardless of whether T implements them.
impl<T> Copy for Slice2DRawParts<T> {}
impl<T> Clone for Slice2DRawParts<T> {
  fn clone(&self) -> Slice2DRawParts<T> {
    *self
  }
}

impl<T> Slice2DRawParts<T> {
  /// Inspired by split_at for slices.
  ///
  /// Horizontally divides one slice into two at index.
  ///
  /// The first will contains rows from `[0, mid)` (excluding the index `mid`
  /// itself) and the second will contain all columns from `[mid, height)`
  /// (excluding the index `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  unsafe fn horizontal_split(
    self, mid: usize,
  ) -> (Slice2DRawParts<T>, Slice2DRawParts<T>) {
    // Out of bounds
    assert!(mid <= self.height);

    let mut top = self;
    let mut bottom = self;
    top.height = mid;
    bottom.ptr = bottom.ptr.add(mid * bottom.stride);
    bottom.height = bottom.height - mid;
    (top, bottom)
  }

  /// Inspired by split_at for slices.
  ///
  /// Vertically divides one slice into two at an index.
  ///
  /// The first will contains columns from `[0, mid)` (excluding the index `mid`
  /// itself) and the second will contain all columns from `[mid, height)`
  /// (excluding the index `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > width`.
  #[inline(always)]
  unsafe fn vertical_split(
    self, mid: usize,
  ) -> (Slice2DRawParts<T>, Slice2DRawParts<T>) {
    // Out of bounds
    assert!(mid <= self.width);

    let mut left = self;
    let mut right = self;
    left.width = mid;
    right.ptr = right.ptr.add(mid);
    right.width = right.width - mid;
    (left, right)
  }
}

/// Inspired by std::slice::SliceIndex
pub trait SliceIndex2D<T>: Copy {
  type Output: ?Sized;

  unsafe fn get_raw_unchecked<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a Self::Output;
  unsafe fn get_raw_mut_unchecked<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a mut Self::Output;

  fn check_assert(index: &Self, data: Slice2DRawParts<T>);
  fn check(index: &Self, data: Slice2DRawParts<T>) -> bool;

  #[inline(always)]
  unsafe fn get_raw<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> Option<&'a Self::Output> {
    if Self::check(&index, data) {
      Some(Self::get_raw_unchecked(index, data))
    } else {
      None
    }
  }

  #[inline(always)]
  unsafe fn get_raw_mut<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> Option<&'a mut Self::Output> {
    if Self::check(&index, data) {
      Some(Self::get_raw_mut_unchecked(index, data))
    } else {
      None
    }
  }

  #[inline(always)]
  unsafe fn index_raw<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a Self::Output {
    Self::check_assert(&index, data);
    Self::get_raw_unchecked(index, data)
  }

  #[inline(always)]
  unsafe fn index_raw_mut<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a mut Self::Output {
    Self::check_assert(&index, data);
    Self::get_raw_mut_unchecked(index, data)
  }
}

impl<T> SliceIndex2D<T> for usize {
  type Output = [T];

  #[inline(always)]
  unsafe fn get_raw_unchecked<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a Self::Output {
    slice::from_raw_parts(data.ptr.add(index * data.stride), data.width)
  }

  #[inline(always)]
  unsafe fn get_raw_mut_unchecked<'a>(
    index: Self, data: Slice2DRawParts<T>,
  ) -> &'a mut Self::Output {
    slice::from_raw_parts_mut(data.ptr.add(index * data.stride), data.width)
  }

  #[inline(always)]
  fn check_assert(index: &Self, data: Slice2DRawParts<T>) {
    assert!(*index < data.height);
  }

  #[inline(always)]
  fn check(index: &Self, data: Slice2DRawParts<T>) -> bool {
    *index < data.height
  }
}

pub struct Slice2D<'a, T> {
  raw_parts: Slice2DRawParts<T>,
  phantom: PhantomData<&'a T>,
}

pub struct Slice2DMut<'a, T> {
  raw_parts: Slice2DRawParts<T>,
  phantom: PhantomData<&'a mut T>,
}

impl<'a, T> Slice2D<'a, T> {
  #[inline(always)]
  pub fn empty(backing: [T; 0]) -> Slice2D<'a, T> {
    Self {
      raw_parts: Slice2DRawParts {
        ptr: backing.as_ptr() as *mut T,
        width: 0,
        height: 0,
        stride: 0,
      },
      phantom: PhantomData,
    }
  }

  // TODO: Get rid of once splitting of Slices is handled elsewhere.
  #[inline(always)]
  pub unsafe fn new(
    ptr: *const T, width: usize, height: usize, stride: usize,
  ) -> Self {
    assert!(width <= stride);
    Self {
      raw_parts: Slice2DRawParts { ptr: ptr as *mut T, width, height, stride },
      phantom: PhantomData,
    }
  }

  #[inline(always)]
  pub unsafe fn from_raw_parts(raw_parts: Slice2DRawParts<T>) -> Self {
    Self { raw_parts, phantom: PhantomData }
  }

  #[inline(always)]
  pub const fn as_ptr(&self) -> *const T {
    self.raw_parts.ptr
  }

  #[inline(always)]
  pub const fn width(&self) -> usize {
    self.raw_parts.width
  }

  #[inline(always)]
  pub const fn height(&self) -> usize {
    self.raw_parts.height
  }

  #[inline(always)]
  pub const fn cols(&self) -> usize {
    self.raw_parts.width
  }

  #[inline(always)]
  pub const fn rows(&self) -> usize {
    self.raw_parts.height
  }

  #[inline(always)]
  pub const fn stride(&self) -> usize {
    self.raw_parts.stride
  }

  /// Inspired by split_at for slices.
  ///
  /// Horizontally divides one slice into two at index.
  ///
  /// The first will contains rows from `[0, mid)` (excluding the index `mid`
  /// itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the index `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn horizontal_split(
    self, mid: usize,
  ) -> (Slice2D<'a, T>, Slice2D<'a, T>) {
    unsafe {
      let (top, bottom) = self.raw_parts.horizontal_split(mid);
      (Slice2D::from_raw_parts(top), Slice2D::from_raw_parts(bottom))
    }
  }

  /// Inspired by split_at for slices.
  ///
  /// Vertically divides one slice into two at index.
  ///
  /// The first will contains columns from `[0, mid)` (excluding the columns
  /// `mid` itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the column `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn vertical_split(self, mid: usize) -> (Slice2D<'a, T>, Slice2D<'a, T>) {
    unsafe {
      let (left, right) = self.raw_parts.vertical_split(mid);
      (Slice2D::from_raw_parts(left), Slice2D::from_raw_parts(right))
    }
  }

  #[inline(always)]
  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    unsafe { RowsIter::new(self.raw_parts) }
  }

  pub fn tmp_subslice(
    &mut self, index: (Range<usize>, Range<usize>),
  ) -> Slice2D<'a, T> {
    let data = self.raw_parts;
    assert!(index.0.end <= data.height && index.1.end <= data.width);
    unsafe {
      Slice2D::from_raw_parts(Slice2DRawParts {
        ptr: data.ptr.add(index.0.start * data.stride + index.1.start),
        width: index.1.end - index.1.start,
        height: index.0.end - index.0.start,
        stride: data.stride,
      })
    }
  }
}

impl<'a, T, I: SliceIndex2D<T>> Index<I> for Slice2D<'a, T> {
  type Output = I::Output;
  #[inline(always)]
  fn index(&self, index: I) -> &Self::Output {
    unsafe { I::index_raw(index, self.raw_parts) }
  }
}

unsafe impl<T> Send for Slice2D<'_, T> where T: Sync {}
unsafe impl<T> Sync for Slice2D<'_, T> where T: Sync {}

impl<T> fmt::Debug for Slice2D<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Slice2D {{ ptr: {:?}, size: {}({})x{} }}",
      self.as_ptr(),
      self.width(),
      self.stride(),
      self.height()
    )
  }
}

// Functions shared with Slice2D
impl<'a, T> Slice2DMut<'a, T> {
  #[inline(always)]
  pub unsafe fn from_raw_parts(raw_parts: Slice2DRawParts<T>) -> Self {
    Self { raw_parts, phantom: PhantomData }
  }

  #[inline(always)]
  pub const fn as_ptr(&self) -> *const T {
    self.raw_parts.ptr
  }

  #[inline(always)]
  pub const fn width(&self) -> usize {
    self.raw_parts.width
  }

  #[inline(always)]
  pub const fn height(&self) -> usize {
    self.raw_parts.height
  }

  #[inline(always)]
  pub const fn cols(&self) -> usize {
    self.raw_parts.width
  }

  #[inline(always)]
  pub const fn rows(&self) -> usize {
    self.raw_parts.height
  }

  #[inline(always)]
  pub const fn stride(&self) -> usize {
    self.raw_parts.stride
  }

  /// Inspired by split_at for slices.
  ///
  /// Horizontally divides one slice into two at index.
  ///
  /// The first will contains rows from `[0, mid)` (excluding the index `mid`
  /// itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the index `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn horizontal_split(
    self, mid: usize,
  ) -> (Slice2D<'a, T>, Slice2D<'a, T>) {
    unsafe {
      let (top, bottom) = self.raw_parts.horizontal_split(mid);
      (Slice2D::from_raw_parts(top), Slice2D::from_raw_parts(bottom))
    }
  }

  /// Inspired by split_at for slices.
  ///
  /// Vertically divides one slice into two at index.
  ///
  /// The first will contains columns from `[0, mid)` (excluding the columns
  /// `mid` itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the column `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn vertical_split(self, mid: usize) -> (Slice2D<'a, T>, Slice2D<'a, T>) {
    unsafe {
      let (left, right) = self.raw_parts.vertical_split(mid);
      (Slice2D::from_raw_parts(left), Slice2D::from_raw_parts(right))
    }
  }

  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    unsafe { RowsIter::new(self.raw_parts) }
  }
}

// Mutable functions
impl<'a, T> Slice2DMut<'a, T> {
  #[inline(always)]
  pub fn empty(mut backing: [T; 0]) -> Slice2DMut<'a, T> {
    Self {
      raw_parts: Slice2DRawParts {
        ptr: backing.as_mut_ptr(),
        width: 0,
        height: 0,
        stride: 0,
      },
      phantom: PhantomData,
    }
  }

  #[inline(always)]
  pub unsafe fn new(
    ptr: *mut T, width: usize, height: usize, stride: usize,
  ) -> Self {
    assert!(width <= stride);
    Self {
      raw_parts: Slice2DRawParts { ptr: ptr as *mut T, width, height, stride },
      phantom: PhantomData,
    }
  }

  #[inline(always)]
  pub const fn as_const(&self) -> Slice2D<'_, T> {
    Slice2D { raw_parts: self.raw_parts, phantom: PhantomData }
  }

  #[inline(always)]
  pub fn as_mut_ptr(&mut self) -> *mut T {
    self.raw_parts.ptr
  }

  /// Inspired by split_at for slices.
  ///
  /// Horizontally divides one mutable slice into two at index.
  ///
  /// The first will contains rows from `[0, mid)` (excluding the row `mid`
  /// itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the row `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn horizontal_split_mut(
    self, mid: usize,
  ) -> (Slice2DMut<'a, T>, Slice2DMut<'a, T>) {
    unsafe {
      let (top, bottom) = self.raw_parts.horizontal_split(mid);
      (Slice2DMut::from_raw_parts(top), Slice2DMut::from_raw_parts(bottom))
    }
  }

  /// Inspired by split_at for slices.
  ///
  /// Vertically divides one mutable slice into two at index.
  ///
  /// The first will contains columns from `[0, mid)` (excluding the columns
  /// `mid` itself) and the second will contain all rows from `[mid, height)`
  /// (excluding the column `height` itself).
  ///
  /// # Panic
  ///
  /// Panics if `mid > height`.
  #[inline(always)]
  pub fn vertical_split_mut(
    self, mid: usize,
  ) -> (Slice2DMut<'a, T>, Slice2DMut<'a, T>) {
    unsafe {
      let (left, right) = self.raw_parts.vertical_split(mid);
      (Slice2DMut::from_raw_parts(left), Slice2DMut::from_raw_parts(right))
    }
  }

  #[inline(always)]
  pub fn tmp_subslice(
    &mut self, index: (Range<usize>, Range<usize>),
  ) -> Slice2DMut<'a, T> {
    let data = self.raw_parts;
    assert!(index.0.end <= data.height && index.1.end <= data.width);
    unsafe {
      Slice2DMut::from_raw_parts(Slice2DRawParts {
        ptr: data.ptr.add(index.0.start * data.stride + index.1.start),
        width: index.1.end - index.1.start,
        height: index.0.end - index.0.start,
        stride: data.stride,
      })
    }
  }

  #[inline(always)]
  pub fn rows_iter_mut(&mut self) -> RowsIterMut<'_, T> {
    unsafe { RowsIterMut::new(self.raw_parts) }
  }
}

unsafe impl<T> Send for Slice2DMut<'_, T> where T: Send {}
unsafe impl<T> Sync for Slice2DMut<'_, T> where T: Sync {}

impl<'a, T, I: SliceIndex2D<T>> Index<I> for Slice2DMut<'a, T> {
  type Output = I::Output;
  #[inline(always)]
  fn index(&self, index: I) -> &Self::Output {
    unsafe { SliceIndex2D::index_raw(index, self.raw_parts) }
  }
}

impl<'a, T, I: SliceIndex2D<T>> IndexMut<I> for Slice2DMut<'a, T> {
  #[inline(always)]
  fn index_mut(&mut self, index: I) -> &mut Self::Output {
    unsafe { SliceIndex2D::index_raw_mut(index, self.raw_parts) }
  }
}

impl<T> fmt::Debug for Slice2DMut<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Slice2D {{ ptr: {:?}, size: {}({})x{} }}",
      self.as_ptr(),
      self.width(),
      self.stride(),
      self.height()
    )
  }
}

/// Iterator over rows
pub struct RowsIter<'a, T> {
  /// Represent the iterators state in a 2d slice.
  /// Width and stride are constant. The pointer tracks the current row and the
  /// height tracks the remaining rows.
  slice: Slice2DRawParts<T>,
  phantom: PhantomData<&'a T>,
}

/// Mutable iterator over rows
pub struct RowsIterMut<'a, T> {
  /// Represent the iterators state in a 2d slice.
  /// Width and stride are constant. The pointer tracks the current row and the
  /// height tracks the remaining rows.
  slice: Slice2DRawParts<T>,
  phantom: PhantomData<&'a mut T>,
}

impl<'a, T> RowsIter<'a, T> {
  #[inline(always)]
  pub unsafe fn new(slice: Slice2DRawParts<T>) -> Self {
    Self { slice, phantom: PhantomData }
  }
}

impl<'a, T> RowsIterMut<'a, T> {
  #[inline(always)]
  pub unsafe fn new(slice: Slice2DRawParts<T>) -> Self {
    Self { slice, phantom: PhantomData }
  }
}

impl<'a, T> Iterator for RowsIter<'a, T> {
  type Item = &'a [T];

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    unsafe { SliceIndex2D::get_raw(0, self.slice) }.and_then(|row| {
      self.slice.ptr = unsafe { self.slice.ptr.add(self.slice.stride) };
      self.slice.height -= 1;
      Some(row)
    })
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.slice.height, Some(self.slice.height))
  }
}

impl<'a, T> Iterator for RowsIterMut<'a, T> {
  type Item = &'a mut [T];

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    unsafe { SliceIndex2D::get_raw_mut(0, self.slice) }.and_then(|row| {
      self.slice.ptr = unsafe { self.slice.ptr.add(self.slice.stride) };
      self.slice.height -= 1;
      Some(row)
    })
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    (self.slice.height, Some(self.slice.height))
  }
}

impl<T> ExactSizeIterator for RowsIter<'_, T> {}
impl<T> FusedIterator for RowsIter<'_, T> {}
impl<T> ExactSizeIterator for RowsIterMut<'_, T> {}
impl<T> FusedIterator for RowsIterMut<'_, T> {}

pub struct VerticalChunks2D<'a, T> {
  slice: Slice2D<'a, T>,
  chunk_size: usize,
}

pub struct VerticalChunks2DMut<'a, T> {
  slice: Slice2DMut<'a, T>,
  chunk_size: usize,
}

pub struct HorizontalChunks2D<'a, T> {
  slice: Slice2D<'a, T>,
  chunk_size: usize,
}

pub struct HorizontalChunks2DMut<'a, T> {
  slice: Slice2DMut<'a, T>,
  chunk_size: usize,
}

impl<'a, T> Iterator for VerticalChunks2D<'a, T> {
  type Item = Slice2D<'a, T>;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.slice.height() == 0 {
      None
    } else {
      let chunksz = self.slice.height().min(self.chunk_size);
      let tmp = mem::replace(&mut self.slice, Slice2D::empty([]));
      let (head, tail) = tmp.horizontal_split(chunksz);
      self.slice = tail;
      Some(head)
    }
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    if self.slice.height() == 0 {
      (0, Some(0))
    } else {
      let n = self.slice.height() / self.chunk_size;
      let rem = self.slice.height() % self.chunk_size;
      let n = if rem > 0 { n + 1 } else { n };
      (n, Some(n))
    }
  }
}

impl<'a, T> Iterator for VerticalChunks2DMut<'a, T> {
  type Item = Slice2DMut<'a, T>;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.slice.height() == 0 {
      None
    } else {
      let chunksz = self.slice.height().min(self.chunk_size);
      let tmp = mem::replace(&mut self.slice, Slice2DMut::empty([]));
      let (head, tail) = tmp.horizontal_split_mut(chunksz);
      self.slice = tail;
      Some(head)
    }
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    if self.slice.height() == 0 {
      (0, Some(0))
    } else {
      let n = self.slice.height() / self.chunk_size;
      let rem = self.slice.height() % self.chunk_size;
      let n = if rem > 0 { n + 1 } else { n };
      (n, Some(n))
    }
  }
}

impl<'a, T> Iterator for HorizontalChunks2D<'a, T> {
  type Item = Slice2D<'a, T>;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.slice.width() == 0 {
      None
    } else {
      let chunksz = self.slice.width().min(self.chunk_size);
      let tmp = mem::replace(&mut self.slice, Slice2D::empty([]));
      let (head, tail) = tmp.vertical_split(chunksz);
      self.slice = tail;
      Some(head)
    }
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    if self.slice.height() == 0 {
      (0, Some(0))
    } else {
      let n = self.slice.width() / self.chunk_size;
      let rem = self.slice.width() % self.chunk_size;
      let n = if rem > 0 { n + 1 } else { n };
      (n, Some(n))
    }
  }
}

impl<'a, T> Iterator for HorizontalChunks2DMut<'a, T> {
  type Item = Slice2DMut<'a, T>;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.slice.width() == 0 {
      None
    } else {
      let chunksz = self.slice.width().min(self.chunk_size);
      let tmp = mem::replace(&mut self.slice, Slice2DMut::empty([]));
      let (head, tail) = tmp.vertical_split_mut(chunksz);
      self.slice = tail;
      Some(head)
    }
  }

  #[inline(always)]
  fn size_hint(&self) -> (usize, Option<usize>) {
    if self.slice.height() == 0 {
      (0, Some(0))
    } else {
      let n = self.slice.width() / self.chunk_size;
      let rem = self.slice.width() % self.chunk_size;
      let n = if rem > 0 { n + 1 } else { n };
      (n, Some(n))
    }
  }
}

impl<T> ExactSizeIterator for VerticalChunks2D<'_, T> {}
impl<T> FusedIterator for VerticalChunks2D<'_, T> {}
impl<T> ExactSizeIterator for HorizontalChunks2D<'_, T> {}
impl<T> FusedIterator for HorizontalChunks2D<'_, T> {}
