// Copyright (c) 2019-2020, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

#![allow(unused)]

use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
use std::ptr::NonNull;
use std::{fmt, slice};

#[derive(Copy, Clone)]
pub struct Slice2DRawParts<T> {
  pub ptr: *mut T,
  pub width: usize,
  pub height: usize,
  pub stride: usize,
}

/// Inspired by std::slice::SliceIndex
pub trait SliceIndex2D<T>: Sized {
  type Output: ?Sized;

  unsafe fn get_raw_unchecked<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a Self::Output;
  unsafe fn get_raw_mut_unchecked<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a mut Self::Output;

  fn check_assert(index: &Self, data: &Slice2DRawParts<T>);
  fn check(index: &Self, data: &Slice2DRawParts<T>) -> bool;

  #[inline(always)]
  unsafe fn get_raw<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> Option<&'a Self::Output> {
    if Self::check(&index, &data) {
      Some(Self::get_raw_unchecked(index, data))
    } else {
      None
    }
  }

  #[inline(always)]
  unsafe fn get_raw_mut<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> Option<&'a mut Self::Output> {
    if Self::check(&index, &data) {
      Some(Self::get_raw_mut_unchecked(index, data))
    } else {
      None
    }
  }

  #[inline(always)]
  unsafe fn index_raw<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a Self::Output {
    Self::check_assert(&index, &data);
    Self::get_raw_unchecked(index, data)
  }

  #[inline(always)]
  unsafe fn index_raw_mut<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a mut Self::Output {
    Self::check_assert(&index, &data);
    Self::get_raw_mut_unchecked(index, data)
  }
}

impl<T> SliceIndex2D<T> for usize {
  type Output = [T];

  #[inline(always)]
  unsafe fn get_raw_unchecked<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a Self::Output {
    slice::from_raw_parts(data.ptr.add(index * data.stride), data.width)
  }

  #[inline(always)]
  unsafe fn get_raw_mut_unchecked<'a>(
    index: Self, data: &Slice2DRawParts<T>,
  ) -> &'a mut Self::Output {
    slice::from_raw_parts_mut(data.ptr.add(index * data.stride), data.width)
  }

  #[inline(always)]
  fn check_assert(index: &Self, data: &Slice2DRawParts<T>) {
    assert!(*index < data.height);
  }

  #[inline(always)]
  fn check(index: &Self, data: &Slice2DRawParts<T>) -> bool {
    *index < data.height
  }
}

#[derive(Copy, Clone)]
pub struct Slice2D<'a, T> {
  // TODO: It would be desirable to use NonNull in place of a simple pointer,
  // but we rely on using 0x0 vecs with no allocation, thus we can't be
  // guaranteed a non-null pointer.
  ptr: *const T,
  width: usize,
  height: usize,
  stride: usize,
  phantom: PhantomData<&'a T>,
}

#[derive(Copy, Clone)]
pub struct Slice2DMut<'a, T> {
  ptr: *mut T,
  width: usize,
  height: usize,
  stride: usize,
  phantom: PhantomData<&'a mut T>,
}

impl<'a, T> Slice2D<'a, T> {
  // TODO: If we ever move to Nonnull pointers, it would make sense to use that
  // as a parameter here.
  #[inline(always)]
  pub unsafe fn new(
    ptr: *const T, width: usize, height: usize, stride: usize,
  ) -> Self {
    assert!(width <= stride);
    Self { ptr, width, height, stride, phantom: PhantomData }
  }

  #[inline(always)]
  pub const fn as_ptr(&self) -> *const T {
    self.ptr
  }

  #[inline(always)]
  pub const fn width(&self) -> usize {
    self.width
  }

  #[inline(always)]
  pub const fn height(&self) -> usize {
    self.height
  }

  #[inline(always)]
  pub const fn stride(&self) -> usize {
    self.stride
  }

  pub fn to_raw_parts(&self) -> Slice2DRawParts<T> {
    Slice2DRawParts {
      ptr: self.ptr as *mut T,
      width: self.width,
      height: self.height,
      stride: self.stride,
    }
  }

  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    RowsIter { slice: self.to_raw_parts(), phantom: PhantomData }
  }
}

impl<'a, T, I: SliceIndex2D<T>> Index<I> for Slice2D<'a, T> {
  type Output = I::Output;
  #[inline(always)]
  fn index(&self, index: I) -> &Self::Output {
    unsafe { I::index_raw(index, &self.to_raw_parts()) }
  }
}

impl<T> fmt::Debug for Slice2D<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Slice2D {{ ptr: {:?}, size: {}({})x{} }}",
      self.ptr, self.width, self.stride, self.height
    )
  }
}

// Functions shared with Slice2D
impl<'a, T> Slice2DMut<'a, T> {
  #[inline(always)]
  pub const fn as_ptr(&self) -> *const T {
    self.ptr
  }

  #[inline(always)]
  pub const fn width(&self) -> usize {
    self.width
  }

  #[inline(always)]
  pub const fn height(&self) -> usize {
    self.height
  }

  #[inline(always)]
  pub const fn stride(&self) -> usize {
    self.stride
  }

  pub fn to_raw_parts(&self) -> Slice2DRawParts<T> {
    Slice2DRawParts {
      ptr: self.ptr as *mut T,
      width: self.width,
      height: self.height,
      stride: self.stride,
    }
  }

  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    RowsIter { slice: self.to_raw_parts(), phantom: PhantomData }
  }
}

// Mutable functions
impl<'a, T> Slice2DMut<'a, T> {
  #[inline(always)]
  pub unsafe fn new(
    ptr: *mut T, width: usize, height: usize, stride: usize,
  ) -> Self {
    assert!(width <= stride);
    Self { ptr, width, height, stride, phantom: PhantomData }
  }

  pub const fn as_const(self) -> Slice2D<'a, T> {
    Slice2D {
      ptr: self.ptr,
      width: self.width,
      height: self.height,
      stride: self.stride,
      phantom: PhantomData,
    }
  }

  pub fn as_mut_ptr(&mut self) -> *mut T {
    self.ptr
  }

  pub fn rows_iter_mut(&mut self) -> RowsIterMut<'_, T> {
    RowsIterMut { slice: self.to_raw_parts(), phantom: PhantomData }
  }
}

impl<'a, T, I: SliceIndex2D<T>> Index<I> for Slice2DMut<'a, T> {
  type Output = I::Output;
  #[inline(always)]
  fn index(&self, index: I) -> &Self::Output {
    unsafe { I::index_raw(index, &self.to_raw_parts()) }
  }
}

impl<'a, T, I: SliceIndex2D<T>> IndexMut<I> for Slice2DMut<'a, T> {
  #[inline(always)]
  fn index_mut(&mut self, index: I) -> &mut Self::Output {
    unsafe { I::index_raw_mut(index, &self.to_raw_parts()) }
  }
}

impl<T> fmt::Debug for Slice2DMut<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Slice2D {{ ptr: {:?}, size: {}({})x{} }}",
      self.ptr, self.width, self.stride, self.height
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
    unsafe { SliceIndex2D::get_raw(0, &self.slice) }.and_then(|row| {
      self.slice.ptr =
        unsafe { self.slice.ptr.offset(self.slice.stride as isize) };
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
    unsafe { SliceIndex2D::get_raw_mut(0, &self.slice) }.and_then(|row| {
      self.slice.ptr =
        unsafe { self.slice.ptr.offset(self.slice.stride as isize) };
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
