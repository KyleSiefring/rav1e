use crate::util::{RowsIter, RowsIterMut, Slice2D, Slice2DMut};
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

  #[inline(always)]
  pub fn slice(&self) -> Slice2D<'_, T> {
    unsafe {
      Slice2D::new_unchecked(
        self.data.as_ptr(),
        self.width,
        self.height,
        self.width,
      )
    }
  }

  #[inline(always)]
  pub fn mut_slice(&mut self) -> Slice2DMut<'_, T> {
    unsafe {
      Slice2DMut::new_unchecked(
        self.data.as_mut_ptr(),
        self.width,
        self.height,
        self.width,
      )
    }
  }

  #[inline(always)]
  pub fn rows_iter(&self) -> RowsIter<'_, T> {
    unsafe {
      RowsIter::new(self.data.as_ptr(), self.width, self.width, self.height)
    }
  }

  #[inline(always)]
  pub fn rows_iter_mut(&mut self) -> RowsIterMut<'_, T> {
    unsafe {
      RowsIterMut::new(
        self.data.as_mut_ptr(),
        self.width,
        self.width,
        self.height,
      )
    }
  }
}

impl<T> Index<usize> for Data2D<T> {
  type Output = [T];
  #[inline(always)]
  fn index(&self, index: usize) -> &Self::Output {
    &self.data[index * self.width..(index + 1) * self.width]
  }
}

impl<T> IndexMut<usize> for Data2D<T> {
  #[inline(always)]
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.data[index * self.width..(index + 1) * self.width]
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
