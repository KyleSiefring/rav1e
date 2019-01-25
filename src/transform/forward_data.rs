
use super::TxSize;
use super::TxType;

use super::HTX_TAB;
use super::VTX_TAB;

pub type TxfmShift = [i8; 3];
pub type TxfmShifts = [TxfmShift; 3];

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

pub const FWD_TXFM_SHIFT_LS: [TxfmShifts; TxSize::TX_SIZES_ALL] = [
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TxfmType {
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
}

#[derive(Debug, Clone, Copy)]
pub struct Txfm2DFlipCfg {
  pub tx_size: TxSize,
  /// Flip upside down
  pub ud_flip: bool,
  /// Flip left to right
  pub lr_flip: bool,
  pub shift: TxfmShift,
  pub txfm_type_col: TxfmType,
  pub txfm_type_row: TxfmType,
}

impl Txfm2DFlipCfg {
  pub fn fwd(tx_type: TxType, tx_size: TxSize, bd: usize) -> Self {
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
