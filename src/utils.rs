use std::ops::Mul;

use anchor_lang::AccountDeserialize;
use anyhow::Result;
use raydium_amm_v3::libraries::fixed_point_64;
use solana_sdk::account::Account;
use spl_token_2022::extension::{
  transfer_fee::{TransferFeeConfig, MAX_FEE_BASIS_POINTS},
  BaseState,
  BaseStateWithExtensions,
  StateWithExtensionsMut,
};

pub fn deserialize_anchor_account<T: AccountDeserialize>(account: &Account) -> Result<T> {
  let mut data: &[u8] = &account.data;
  T::try_deserialize(&mut data).map_err(Into::into)
}

/// Calculate the fee for output amount
pub fn get_transfer_inverse_fee<'data, S: BaseState>(
  account_state: &StateWithExtensionsMut<'data, S>,
  epoch: u64,
  post_fee_amount: u64,
) -> u64 {
  let fee = if let Ok(transfer_fee_config) = account_state.get_extension::<TransferFeeConfig>() {
    let transfer_fee = transfer_fee_config.get_epoch_fee(epoch);
    if u16::from(transfer_fee.transfer_fee_basis_points) == MAX_FEE_BASIS_POINTS {
      u64::from(transfer_fee.maximum_fee)
    } else {
      transfer_fee_config
        .calculate_inverse_epoch_fee(epoch, post_fee_amount)
        .unwrap()
    }
  } else {
    0
  };
  fee
}

pub fn price_to_sqrt_price_x64(price: f64, decimals_0: u8, decimals_1: u8) -> u128 {
  let price_with_decimals = price * multiplier(decimals_1) / multiplier(decimals_0);
  price_to_x64(price_with_decimals.sqrt())
}

pub fn multiplier(decimals: u8) -> f64 {
  (10_i32).checked_pow(decimals.try_into().unwrap()).unwrap() as f64
}

pub fn price_to_x64(price: f64) -> u128 {
  (price * fixed_point_64::Q64 as f64) as u128
}

pub fn amount_with_slippage(amount: u64, slippage: f64, round_up: bool) -> u64 {
  if round_up {
    (amount as f64).mul(1_f64 + slippage).ceil() as u64
  } else {
    (amount as f64).mul(1_f64 - slippage).floor() as u64
  }
}
