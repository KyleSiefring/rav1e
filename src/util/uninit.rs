// Copyright (c) 2019, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use std::mem::MaybeUninit;

pub trait InitSlice<T: Copy> {
  type Output;

  /// Initialize all entries in the slice to one value
  fn init_repeat(self, value: T) -> Self::Output;
}

impl<'a, T: Copy> InitSlice<T> for &'a mut [MaybeUninit<T>] {
  type Output = &'a mut [T];

  fn init_repeat(self, value: T) -> Self::Output {
    for a in self.iter_mut() {
      *a = MaybeUninit::new(value);
    }

    unsafe {
      std::mem::transmute::<&'a mut [MaybeUninit<T>], &'a mut [T]>(self)
    }
  }
}
