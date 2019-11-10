// Copyright (c) 2018, The rav1e contributors. All rights reserved
//
// This source code is subject to the terms of the BSD 2 Clause License and
// the Alliance for Open Media Patent License 1.0. If the BSD 2 Clause License
// was not distributed with this source code in the LICENSE file, you can
// obtain it at www.aomedia.org/license/software. If the Alliance for Open
// Media Patent License 1.0 was not distributed with this source code in the
// PATENTS file, you can obtain it at www.aomedia.org/license/patent.

use super::*;

type TxfmShift = [i8; 3];
type TxfmShifts = [TxfmShift; 3];

// Shift so that the first shift is 4 - (bd - 8) to align with the initial
// design of daala_tx
// 8 bit 4x4 is an exception and only shifts by 3 in the first stage
const FWD_SHIFT_4X4: TxfmShifts = [[3, 0, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_8X8: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_16X16: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_32X32: TxfmShifts = [[4, -2, 0], [2, 0, 0], [0, 0, 2]];
const FWD_SHIFT_64X64: TxfmShifts = [[4, -1, -2], [2, 0, -1], [0, 0, 1]];
const FWD_SHIFT_4X8: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_8X4: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_8X16: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_16X8: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_16X32: TxfmShifts = [[4, -2, 0], [2, 0, 0], [0, 0, 2]];
const FWD_SHIFT_32X16: TxfmShifts = [[4, -2, 0], [2, 0, 0], [0, 0, 2]];
const FWD_SHIFT_32X64: TxfmShifts = [[4, -1, -2], [2, 0, -1], [0, 0, 1]];
const FWD_SHIFT_64X32: TxfmShifts = [[4, -1, -2], [2, 0, -1], [0, 0, 1]];
const FWD_SHIFT_4X16: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_16X4: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_8X32: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_32X8: TxfmShifts = [[4, -1, 0], [2, 0, 1], [0, 0, 3]];
const FWD_SHIFT_16X64: TxfmShifts = [[4, -2, 0], [2, 0, 0], [0, 0, 2]];
const FWD_SHIFT_64X16: TxfmShifts = [[4, -2, 0], [2, 0, 0], [0, 0, 2]];

const FWD_TXFM_SHIFT_LS: [TxfmShifts; TxSize::TX_SIZES_ALL] = [
  FWD_SHIFT_4X4,
  FWD_SHIFT_8X8,
  FWD_SHIFT_16X16,
  FWD_SHIFT_32X32,
  FWD_SHIFT_64X64,
  FWD_SHIFT_4X8,
  FWD_SHIFT_8X4,
  FWD_SHIFT_8X16,
  FWD_SHIFT_16X8,
  FWD_SHIFT_16X32,
  FWD_SHIFT_32X16,
  FWD_SHIFT_32X64,
  FWD_SHIFT_64X32,
  FWD_SHIFT_4X16,
  FWD_SHIFT_16X4,
  FWD_SHIFT_8X32,
  FWD_SHIFT_32X8,
  FWD_SHIFT_16X64,
  FWD_SHIFT_64X16,
];

type TxfmFunc = dyn Fn(&[i32], &mut [i32]);
type TxfmFuncI32X8 = unsafe fn(&[I32X8], &mut [I32X8]);

use std::ops::*;

/*pub trait TxOperations: Copy {
  fn zero() -> Self;

  fn tx_mul(self, _: (i32, i32)) -> Self;
  fn rshift1(self) -> Self;
  fn add(self, b: Self) -> Self;
  fn sub(self, b: Self) -> Self;
  fn add_avg(self, b: Self) -> Self;
  fn sub_avg(self, b: Self) -> Self;

  fn copy_fn(self) -> Self {
    self
  }
}

impl TxOperations for i32 {
  fn zero() -> Self {
    0
  }

  fn tx_mul(self, mul: (i32, i32)) -> Self {
    ((self * mul.0) + (1 << mul.1 >> 1)) >> mul.1
  }

  fn rshift1(self) -> Self {
    (self + if self < 0 { 1 } else { 0 }) >> 1
  }

  fn add(self, b: Self) -> Self {
    self + b
  }

  fn sub(self, b: Self) -> Self {
    self - b
  }

  fn add_avg(self, b: Self) -> Self {
    (self + b) >> 1
  }

  fn sub_avg(self, b: Self) -> Self {
    (self - b) >> 1
  }
}*/

#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub trait TxOperations: Copy {
  unsafe fn zero() -> Self;

  unsafe fn tx_mul(self, _: (i32, i32)) -> Self;
  unsafe fn rshift1(self) -> Self;
  unsafe fn add(self, b: Self) -> Self;
  unsafe fn sub(self, b: Self) -> Self;
  unsafe fn add_avg(self, b: Self) -> Self;
  unsafe fn sub_avg(self, b: Self) -> Self;

  unsafe fn copy_fn(self) -> Self {
    self
  }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
struct I32X8 {
  data: [i32; 8],
}

impl I32X8 {
  #[target_feature(enable = "avx2")]
  unsafe fn vec(self) -> __m256i {
    std::mem::transmute(self.data)
  }

  #[target_feature(enable = "avx2")]
  unsafe fn set(&mut self, i: usize, val: i32) {
    self.data[i] = val;
  }

  #[target_feature(enable = "avx2")]
  unsafe fn get(self, i: usize) -> i32 {
    self.data[i]
  }

  #[target_feature(enable = "avx2")]
  unsafe fn new(a: __m256i) -> I32X8 {
    I32X8 { data: std::mem::transmute(a) }
  }
}

impl TxOperations for I32X8 {
  #[target_feature(enable = "avx2")]
  unsafe fn zero() -> Self {
    I32X8::new(_mm256_setzero_si256())
  }

  #[target_feature(enable = "avx2")]
  unsafe fn tx_mul(self, mul: (i32, i32)) -> Self {
      I32X8::new(_mm256_srav_epi32(
        _mm256_add_epi32(
          _mm256_mullo_epi32(self.vec(), _mm256_set1_epi32(mul.0)),
          _mm256_set1_epi32(1 << mul.1 >> 1),
        ),
        _mm256_set1_epi32(mul.1),
      ))
  }

  #[target_feature(enable = "avx2")]
  unsafe fn rshift1(self) -> Self {
      I32X8::new(_mm256_srai_epi32(
        _mm256_sub_epi32(
          self.vec(),
          _mm256_cmpgt_epi32(_mm256_setzero_si256(), self.vec()),
        ),
        1,
      ))
  }

  unsafe fn add(self, b: Self) -> Self {
    I32X8::new(_mm256_add_epi32(self.vec(), b.vec()))
  }

  unsafe fn sub(self, b: Self) -> Self {
    I32X8::new(_mm256_sub_epi32(self.vec(), b.vec()))
  }

  #[target_feature(enable = "avx2")]
  unsafe fn add_avg(self, b: Self) -> Self {
    I32X8::new(_mm256_srai_epi32(_mm256_add_epi32(self.vec(), b.vec()), 1))
  }

  unsafe fn sub_avg(self, b: Self) -> Self {
    I32X8::new(_mm256_srai_epi32(_mm256_sub_epi32(self.vec(), b.vec()), 1))
  }
}

impl_1d_tx!(target_feature(enable = "avx2"), unsafe);

#[derive(Debug, Clone, Copy, PartialEq)]
enum TxfmType {
  DCT4,
  DCT8,
  DCT16,
  DCT32,
  DCT64,
  ADST4,
  ADST8,
  ADST16,
  Identity4,
  Identity8,
  Identity16,
  Identity32,
  Invalid,
}

impl TxfmType {
  const TX_TYPES_1D: usize = 4;
  const AV1_TXFM_TYPE_LS: [[TxfmType; Self::TX_TYPES_1D]; 5] = [
    [TxfmType::DCT4, TxfmType::ADST4, TxfmType::ADST4, TxfmType::Identity4],
    [TxfmType::DCT8, TxfmType::ADST8, TxfmType::ADST8, TxfmType::Identity8],
    [
      TxfmType::DCT16,
      TxfmType::ADST16,
      TxfmType::ADST16,
      TxfmType::Identity16,
    ],
    [
      TxfmType::DCT32,
      TxfmType::Invalid,
      TxfmType::Invalid,
      TxfmType::Identity32,
    ],
    [TxfmType::DCT64, TxfmType::Invalid, TxfmType::Invalid, TxfmType::Invalid],
  ];

  /*fn get_func(self) -> &'static TxfmFunc {
    use self::TxfmType::*;
    match self {
      DCT4 => &daala_fdct4,
      DCT8 => &daala_fdct8,
      DCT16 => &daala_fdct16,
      DCT32 => &daala_fdct32,
      DCT64 => &daala_fdct64,
      ADST4 => &daala_fdst_vii_4,
      ADST8 => &daala_fdst8,
      ADST16 => &daala_fdst16,
      Identity4 => &fidentity4,
      Identity8 => &fidentity8,
      Identity16 => &fidentity16,
      Identity32 => &fidentity32,
      _ => unreachable!(),
    }
  }*/

  fn get_func_i32x8(self) -> TxfmFuncI32X8 {
    use self::TxfmType::*;
    match self {
      DCT4 => daala_fdct4,
      DCT8 => daala_fdct8,
      DCT16 => daala_fdct16,
      DCT32 => daala_fdct32,
      DCT64 => daala_fdct64,
      ADST4 => daala_fdst_vii_4,
      ADST8 => daala_fdst8,
      ADST16 => daala_fdst16,
      Identity4 => fidentity4,
      Identity8 => fidentity8,
      Identity16 => fidentity16,
      Identity32 => fidentity32,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy)]
struct Txfm2DFlipCfg {
  tx_size: TxSize,
  /// Flip upside down
  ud_flip: bool,
  /// Flip left to right
  lr_flip: bool,
  shift: TxfmShift,
  txfm_type_col: TxfmType,
  txfm_type_row: TxfmType,
}

impl Txfm2DFlipCfg {
  fn fwd(tx_type: TxType, tx_size: TxSize, bd: usize) -> Self {
    let tx_type_1d_col = VTX_TAB[tx_type as usize];
    let tx_type_1d_row = HTX_TAB[tx_type as usize];
    let txw_idx = tx_size.width_index();
    let txh_idx = tx_size.height_index();
    let txfm_type_col =
      TxfmType::AV1_TXFM_TYPE_LS[txh_idx][tx_type_1d_col as usize];
    let txfm_type_row =
      TxfmType::AV1_TXFM_TYPE_LS[txw_idx][tx_type_1d_row as usize];
    assert_ne!(txfm_type_col, TxfmType::Invalid);
    assert_ne!(txfm_type_row, TxfmType::Invalid);
    let (ud_flip, lr_flip) = Self::get_flip_cfg(tx_type);

    Txfm2DFlipCfg {
      tx_size,
      ud_flip,
      lr_flip,
      shift: FWD_TXFM_SHIFT_LS[tx_size as usize][(bd - 8) / 2],
      txfm_type_col,
      txfm_type_row,
    }
  }

  /// Determine the flip config, returning (ud_flip, lr_flip)
  fn get_flip_cfg(tx_type: TxType) -> (bool, bool) {
    use self::TxType::*;
    match tx_type {
      DCT_DCT | ADST_DCT | DCT_ADST | ADST_ADST | IDTX | V_DCT | H_DCT
      | V_ADST | H_ADST => (false, false),
      FLIPADST_DCT | FLIPADST_ADST | V_FLIPADST => (true, false),
      DCT_FLIPADST | ADST_FLIPADST | H_FLIPADST => (false, true),
      FLIPADST_FLIPADST => (true, true),
    }
  }
}

#[target_feature(enable = "avx2")]
unsafe fn transpose_8x8_avx2(
  input: (I32X8, I32X8, I32X8, I32X8, I32X8, I32X8, I32X8, I32X8),
) -> (I32X8, I32X8, I32X8, I32X8, I32X8, I32X8, I32X8, I32X8) {
  let stage1 = (
    _mm256_unpacklo_epi32(input.0.vec(), input.1.vec()),
    _mm256_unpackhi_epi32(input.0.vec(), input.1.vec()),
    _mm256_unpacklo_epi32(input.2.vec(), input.3.vec()),
    _mm256_unpackhi_epi32(input.2.vec(), input.3.vec()),
    _mm256_unpacklo_epi32(input.4.vec(), input.5.vec()),
    _mm256_unpackhi_epi32(input.4.vec(), input.5.vec()),
    _mm256_unpacklo_epi32(input.6.vec(), input.7.vec()),
    _mm256_unpackhi_epi32(input.6.vec(), input.7.vec()),
  );

  let stage2 = (
    _mm256_unpacklo_epi64(stage1.0, stage1.2),
    _mm256_unpackhi_epi64(stage1.0, stage1.2),
    _mm256_unpacklo_epi64(stage1.1, stage1.3),
    _mm256_unpackhi_epi64(stage1.1, stage1.3),
    _mm256_unpacklo_epi64(stage1.4, stage1.6),
    _mm256_unpackhi_epi64(stage1.4, stage1.6),
    _mm256_unpacklo_epi64(stage1.5, stage1.7),
    _mm256_unpackhi_epi64(stage1.5, stage1.7),
  );

  const LO: i32 = (2 << 4) | 0;
  const HI: i32 = (3 << 4) | 1;
  (
    I32X8::new(_mm256_permute2x128_si256(stage2.0, stage2.4, LO)),
    I32X8::new(_mm256_permute2x128_si256(stage2.1, stage2.5, LO)),
    I32X8::new(_mm256_permute2x128_si256(stage2.2, stage2.6, LO)),
    I32X8::new(_mm256_permute2x128_si256(stage2.3, stage2.7, LO)),
    I32X8::new(_mm256_permute2x128_si256(stage2.0, stage2.4, HI)),
    I32X8::new(_mm256_permute2x128_si256(stage2.1, stage2.5, HI)),
    I32X8::new(_mm256_permute2x128_si256(stage2.2, stage2.6, HI)),
    I32X8::new(_mm256_permute2x128_si256(stage2.3, stage2.7, HI)),
  )
}

#[target_feature(enable = "avx2")]
#[inline]
unsafe fn shift_left(a: I32X8, shift: u8) -> I32X8 {
  I32X8::new(_mm256_sllv_epi32(a.vec(), _mm256_set1_epi32(shift as i32)))
}

#[target_feature(enable = "avx2")]
#[inline]
unsafe fn shift_right(a: I32X8, shift: u8) -> I32X8 {
  I32X8::new(_mm256_srav_epi32(
    _mm256_add_epi32(a.vec(), _mm256_set1_epi32(1 << (shift as i32) >> 1)),
    _mm256_set1_epi32(shift as i32),
  ))
}

#[target_feature(enable = "avx2")]
#[inline]
unsafe fn round_shift_array_avx2(arr: &mut [I32X8], size: usize, bit: i8) {
  if bit == 0 {
    return;
  }
  if bit > 0 {
    let shift = bit as u8;
    for i in (0..size).step_by(4) {
      let s = &mut arr[i..i + 4];
      s[0] = shift_right(s[0], shift);
      s[1] = shift_right(s[1], shift);
      s[2] = shift_right(s[2], shift);
      s[3] = shift_right(s[3], shift);
    }
  } else {
    let shift = (-bit) as u8;
    for i in (0..size).step_by(4) {
      let s = &mut arr[i..i + 4];
      s[0] = shift_left(s[0], shift);
      s[1] = shift_left(s[1], shift);
      s[2] = shift_left(s[2], shift);
      s[3] = shift_left(s[3], shift);
    }
  }
}

trait FwdTxfm2D: Dim {
  fn fwd_txfm2d_daala(
    input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
    bd: usize,
  ) {
    unsafe {
      Self::fwd_txfm2d_daala_avx2(input, output, stride, tx_type, bd);
    }
  }

  #[target_feature(enable = "avx2")]
  unsafe fn fwd_txfm2d_daala_avx2(
    input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
    bd: usize,
  ) {
    //let mut tmp: AlignedArray<[i32; 64 * 64]> = AlignedArray::uninitialized();
    //let buf = &mut tmp.array[..Self::W * Self::H];
    let mut tmp: AlignedArray<[I32X8; 64 * 64 / 8]> =
      AlignedArray::uninitialized();
    let buf = &mut tmp.array[..Self::W * (Self::H / 8).max(1)];
    let temp_out = &mut [I32X8::zero(); 128];
    let cfg =
      Txfm2DFlipCfg::fwd(tx_type, TxSize::by_dims(Self::W, Self::H), bd);

    // Note when assigning txfm_size_col, we use the txfm_size from the
    // row configuration and vice versa. This is intentionally done to
    // accurately perform rectangular transforms. When the transform is
    // rectangular, the number of columns will be the same as the
    // txfm_size stored in the row cfg struct. It will make no difference
    // for square transforms.
    let txfm_size_col = TxSize::width(cfg.tx_size);
    let txfm_size_row = TxSize::height(cfg.tx_size);

    let txfm_func_col = cfg.txfm_type_col.get_func_i32x8();
    let txfm_func_row = cfg.txfm_type_row.get_func_i32x8();

    // Columns

    /*for c in 0..txfm_size_col {
    let mut col_flip_backing: AlignedArray<[i32; 64 * 64]> =
      AlignedArray::uninitialized();
    let col_flip = &mut col_flip_backing.array[..txfm_size_row];
    if cfg.ud_flip {
      // flip upside down
      for r in 0..txfm_size_row {
        col_flip[r] = (input[(txfm_size_row - r - 1) * stride + c]).into();
      }
    } else {
      for r in 0..txfm_size_row {
        col_flip[r] = (input[r * stride + c]).into();
      }
    }*/
    for cg in (0..txfm_size_col).step_by(8) {
      let shift = cfg.shift[0] as u8;
      #[target_feature(enable = "avx2")]
      #[inline]
      unsafe fn load_columns(input_ptr: *const i16, shift: u8) -> I32X8 {
        shift_left(
          I32X8::new(_mm256_cvtepi16_epi32(_mm_loadu_si128(input_ptr as *const _))),
          shift
        )
      }
      if cfg.ud_flip {
        // flip upside down
        for r in 0..txfm_size_row {
          for c in 0..txfm_size_col.min(8) {
            temp_out[r].set(c,
              input[(txfm_size_row - r - 1) * stride + c + cg].into());
          }
        }
      } else {
        // TODO: load less of x4
        for r in (0..txfm_size_row).step_by(4) {
          /*for c in 0..txfm_size_col.min(8) {
            temp_out[r].data[c] = (input[r * stride + c + cg]).into();
          }*/
          let output = &mut temp_out[r..r + 4];
          let input_ptr = input[r * stride + cg..].as_ptr();
          output[0] = load_columns(input_ptr, shift);
          output[1] = load_columns(input_ptr.add(stride), shift);
          output[2] = load_columns(input_ptr.add(2 * stride), shift);
          output[3] = load_columns(input_ptr.add(3 * stride), shift);
        }
      }
      //av1_round_shift_array(output, txfm_size_row, -cfg.shift[0]);
      //round_shift_array_avx2(temp_out, txfm_size_row, -cfg.shift[0]);
      /*txfm_func_col(
        &output[..txfm_size_row].to_vec(),
        &mut output[txfm_size_row..]
      );*/
      /*for r in 0..txfm_size_row {
        temp_out[r] = set1(output[r]);
      }*/
      txfm_func_col(
        &temp_out[..txfm_size_row].to_vec(),
        &mut temp_out[txfm_size_row..],
      );
      /*for r in 0..txfm_size_row {
        output[r + txfm_size_row] = get1(temp_out[r + txfm_size_row]);
      }*/
      round_shift_array_avx2(
        &mut temp_out[txfm_size_row..],
        txfm_size_row,
        -cfg.shift[1],
      );
      /*av1_round_shift_array(
        &mut output[txfm_size_row..],
        txfm_size_row,
        -cfg.shift[1]
      );*/
      /*if cfg.lr_flip {
        for r in 0..txfm_size_row {
          // flip from left to right
          buf[r * txfm_size_col + (txfm_size_col - c - 1)] =
            output[txfm_size_row + r];
        }
      } else {
        for r in 0..txfm_size_row {
          buf[r * txfm_size_col + c] = output[txfm_size_row + r];
        }
      }*/
      if cfg.lr_flip {
        for rg in (0..txfm_size_row).step_by(8) {
          for c in 0..txfm_size_col.min(8) {
            for r in 0..txfm_size_row.min(8) {
              buf[(rg / 8 * txfm_size_col) + (txfm_size_col - (c + cg) - 1)]
                .set(r, temp_out[txfm_size_row + r + rg].get(c));
            }
          }
        }
      } else {
        for rg in (0..txfm_size_row).step_by(8) {
          if txfm_size_row >= 8 && txfm_size_col >= 8 {
            let buf = &mut buf[(rg / 8 * txfm_size_col) + cg..];
            let buf = &mut buf[..8];
            let input = &temp_out[txfm_size_row + rg..];
            let input = &input[..8];
            let transposed = transpose_8x8_avx2((
              input[0], input[1], input[2], input[3], input[4], input[5],
              input[6], input[7],
            ));

            buf[0] = transposed.0;
            buf[1] = transposed.1;
            buf[2] = transposed.2;
            buf[3] = transposed.3;
            buf[4] = transposed.4;
            buf[5] = transposed.5;
            buf[6] = transposed.6;
            buf[7] = transposed.7;
          } else {
            for c in 0..txfm_size_col.min(8) {
              for r in 0..txfm_size_row.min(8) {
                buf[(rg / 8 * txfm_size_col) + c + cg].set(r, temp_out[txfm_size_row + r + rg].get(c));
              }
            }
          }
        }
      }
    }

    // Rows
    /*for r in 0..txfm_size_row {
      txfm_func_row(
        &buf[r * txfm_size_col..],
        &mut output[r * txfm_size_col..],
      );
      av1_round_shift_array(
        &mut output[r * txfm_size_col..],
        txfm_size_col,
        -cfg.shift[2],
      );
    }*/
    // Rows
    for rg in (0..txfm_size_row).step_by(8) {
      /*for r in 0..txfm_size_row.min(8) {
        for c in 0..txfm_size_col {
          temp_out[c].data[r] = buf[(r + rg) * txfm_size_col + c];
        }
      }*/
      txfm_func_row(
        //  &temp_out[..txfm_size_col].to_vec(),
        &buf[rg / 8 * txfm_size_col..],
        &mut temp_out[..],
      );
      round_shift_array_avx2(temp_out, txfm_size_col, -cfg.shift[2]);
      /*for r in 0..txfm_size_row.min(8) {
        for c in 0..txfm_size_col {
          output[(r + rg) * txfm_size_col + c] = temp_out[c].data[r];
        }
      }*/
      for cg in (0..txfm_size_col).step_by(8) {
        if txfm_size_row >= 8 && txfm_size_col >= 8 {
          let output_ptr = output[rg * txfm_size_col + cg..].as_mut_ptr();
          let input = &temp_out[cg..];
          let transposed = transpose_8x8_avx2((
            input[0], input[1], input[2], input[3], input[4], input[5],
            input[6], input[7],
          ));
          /*for r in 0..txfm_size_row.min(8) {
            for c in 0..txfm_size_col.min(8) {
              output[(r + rg) * txfm_size_col + c + cg] =
                temp_out[c + cg].data[r];
            }
          }*/

          _mm256_storeu_si256(
            output_ptr.add(0 * txfm_size_col) as *mut _,
            transposed.0.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(1 * txfm_size_col) as *mut _,
            transposed.1.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(2 * txfm_size_col) as *mut _,
            transposed.2.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(3 * txfm_size_col) as *mut _,
            transposed.3.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(4 * txfm_size_col) as *mut _,
            transposed.4.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(5 * txfm_size_col) as *mut _,
            transposed.5.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(6 * txfm_size_col) as *mut _,
            transposed.6.vec(),
          );
          _mm256_storeu_si256(
            output_ptr.add(7 * txfm_size_col) as *mut _,
            transposed.7.vec(),
          );
        } else {
          for r in 0..txfm_size_row.min(8) {
            for c in 0..txfm_size_col.min(8) {
              output[(r + rg) * txfm_size_col + c + cg] = temp_out[c + cg].get(r);
            }
          }
        }
      }
      /*for r in 0..txfm_size_row.min(8) {
        for c in 0..txfm_size_col {
          print!("{} {}", output[(r + rg) * txfm_size_col + c], temp_out[c].data[r]);
          //output[(r + rg) * txfm_size_col + c] = temp_out[c].data[r];
        }
      }*/
    }
  }
}

macro_rules! impl_fwd_txs {
  ($(($W:expr, $H:expr)),+) => {
    $(
      paste::item! {
        impl FwdTxfm2D for [<Block $W x $H>] {}
      }
    )*
  }
}

impl_fwd_txs! { (4, 4), (8, 8), (16, 16), (32, 32), (64, 64) }
impl_fwd_txs! { (4, 8), (8, 16), (16, 32), (32, 64) }
impl_fwd_txs! { (8, 4), (16, 8), (32, 16), (64, 32) }
impl_fwd_txs! { (4, 16), (8, 32), (16, 64) }
impl_fwd_txs! { (16, 4), (32, 8), (64, 16) }

pub fn fht4x4(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block4x4::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht8x8(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block8x8::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht16x16(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block16x16::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht32x32(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block32x32::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht64x64(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT);
  let mut aligned: AlignedArray<[i32; 4096]> = AlignedArray::uninitialized();
  let tmp = &mut aligned.array;

  //Block64x64::fwd_txfm2d(input, &mut tmp, stride, tx_type, bit_depth);
  Block64x64::fwd_txfm2d_daala(input, tmp, stride, tx_type, bit_depth);

  for i in 0..2 {
    for (row_out, row_in) in
      output[2048 * i..].chunks_mut(32).zip(tmp[32 * i..].chunks(64)).take(64)
    {
      row_out.copy_from_slice(&row_in[..32]);
    }
  }
}

pub fn fht4x8(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block4x8::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht8x4(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block8x4::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht8x16(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block8x16::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht16x8(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block16x8::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht16x32(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT || tx_type == TxType::IDTX);
  Block16x32::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht32x16(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT || tx_type == TxType::IDTX);
  Block32x16::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht32x64(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT);
  let mut aligned: AlignedArray<[i32; 2048]> = AlignedArray::uninitialized();
  let tmp = &mut aligned.array;

  Block32x64::fwd_txfm2d_daala(input, tmp, stride, tx_type, bit_depth);

  for (row_out, row_in) in output.chunks_mut(32).zip(tmp.chunks(32)).take(64) {
    row_out.copy_from_slice(&row_in[..32]);
  }
}

pub fn fht64x32(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT);
  let mut aligned: AlignedArray<[i32; 2048]> = AlignedArray::uninitialized();
  let tmp = &mut aligned.array;

  Block64x32::fwd_txfm2d_daala(input, tmp, stride, tx_type, bit_depth);

  for i in 0..2 {
    for (row_out, row_in) in
      output[1024 * i..].chunks_mut(32).zip(tmp[32 * i..].chunks(64)).take(32)
    {
      row_out.copy_from_slice(&row_in[..32]);
    }
  }
}

pub fn fht4x16(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block4x16::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht16x4(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  Block16x4::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht8x32(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT || tx_type == TxType::IDTX);
  Block8x32::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht32x8(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT || tx_type == TxType::IDTX);
  Block32x8::fwd_txfm2d_daala(input, output, stride, tx_type, bit_depth);
}

pub fn fht16x64(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT);
  let mut aligned: AlignedArray<[i32; 1024]> = AlignedArray::uninitialized();
  let tmp = &mut aligned.array;

  Block16x64::fwd_txfm2d_daala(input, tmp, stride, tx_type, bit_depth);

  for (row_out, row_in) in output.chunks_mut(16).zip(tmp.chunks(16)).take(64) {
    row_out.copy_from_slice(&row_in[..16]);
  }
}

pub fn fht64x16(
  input: &[i16], output: &mut [i32], stride: usize, tx_type: TxType,
  bit_depth: usize,
) {
  assert!(tx_type == TxType::DCT_DCT);
  let mut aligned: AlignedArray<[i32; 1024]> = AlignedArray::uninitialized();
  let tmp = &mut aligned.array;

  Block64x16::fwd_txfm2d_daala(input, tmp, stride, tx_type, bit_depth);

  for i in 0..2 {
    for (row_out, row_in) in
      output[512 * i..].chunks_mut(32).zip(tmp[32 * i..].chunks(64)).take(16)
    {
      row_out.copy_from_slice(&row_in[..32]);
    }
  }
}
