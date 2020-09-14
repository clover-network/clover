#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	BithumbDexModule, ExtBuilder, Origin, TestRuntime, System, TestEvent, Tokens, BXB, ALICE, BUSD, BOB, DOT, BETH,
};

#[test]
fn pair_id_encoding() {
  let test_currency = |small, large| {
    let pair_key = BithumbDexModule::get_pair_key(&small, &large);
    let pair_key2 = BithumbDexModule::get_pair_key(&large, &small);
    assert_eq!(pair_key, pair_key2);
    let (currency_left, currency_right) = BithumbDexModule::pair_key_to_ids(pair_key).unwrap();
    assert_eq!(currency_left, small);
    assert_eq!(currency_right, large);
  };

  let pair_key = BithumbDexModule::get_pair_key(&BXB, &BUSD);
  // littel endian
  // [0, 0, 0, 0, b1000000, 0, 0, 0]
  assert_eq!(pair_key, (2 as u64).pow(32));
  let pair_key = BithumbDexModule::get_pair_key(&BXB, &DOT);
  // [0, 0, 0, 0, b01000000, 0, 0, 0]
  assert_eq!(pair_key, (2 as u64).pow(33));
  let pair_key = BithumbDexModule::get_pair_key(&BUSD, &DOT);
  // [1, 0, 0, 0, b01000000, 0, 0, 0]
  assert_eq!(pair_key, 1 + (2 as u64).pow(33));
  test_currency(BXB, BUSD);
  test_currency(BXB, DOT);
  test_currency(BXB, BETH);
  test_currency(BUSD, DOT);
  test_currency(BUSD, BETH);
  test_currency(DOT, BETH);
}

#[test]
fn target_and_supply_amount_calculation() {
  ExtBuilder::default().build().execute_with(|| {
    let (pairs, info) = BithumbDexModule::get_existing_currency_pairs();
    assert_eq!(pairs.len(), 3);
  })
}
