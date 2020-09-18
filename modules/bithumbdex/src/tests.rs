#![cfg(test)]

use super::*;
use frame_support::assert_ok;
use mock::{
	BithumbDexModule, ExtBuilder, Origin, BXB, ALICE, BUSD, BOB, DOT, BETH,
};

pub use primitives::{ AccountId };

use BithumbDexModule as BDM;

fn format_routes<T>(routes: &vec::Vec<T>) -> String
where
    T: sp_std::fmt::Debug {
    let mut s: String = "".to_owned();

    for r in routes {
        let f = format!("{:?},", r);
        s  = s + &f;
    }
    s
}

#[test]
fn test_compound_rote() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(BDM::add_liquidity(
			Origin::signed(AccountId::from(ALICE)),
			BXB,
			BETH,
			500000000000,
			100000000000000000
		));
		assert_ok!(BDM::add_liquidity(
			Origin::signed(AccountId::from(BOB)),
			BETH,
			DOT,
			80000000000,
			4000000000000
		));

	  	let source_amount_bxb = 90000000;
		let (_, route) = BDM::get_target_amount_available(BXB, DOT, source_amount_bxb);
		assert_eq!(route, [BETH, DOT]);
	});
}

#[test]
fn pair_id_encoding() {
  let test_currency = |small, large| {
    let pair_key = BDM::get_pair_key(&small, &large);
    let pair_key2 = BDM::get_pair_key(&large, &small);
    assert_eq!(pair_key, pair_key2);
    let (currency_left, currency_right) = BDM::pair_key_to_ids(pair_key).unwrap();
    assert_eq!(currency_left, small);
    assert_eq!(currency_right, large);
  };

  let pair_key = BDM::get_pair_key(&BXB, &BUSD);
  // littel endian
  // [0, 0, 0, 0, b1000000, 0, 0, 0]
  assert_eq!(pair_key, (2 as u64).pow(32));
  let pair_key = BDM::get_pair_key(&BXB, &DOT);
  // [0, 0, 0, 0, b01000000, 0, 0, 0]
  assert_eq!(pair_key, (2 as u64).pow(33));
  let pair_key = BDM::get_pair_key(&BUSD, &DOT);
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
	// target pool is drain
	assert_eq!(
		BDM::calculate_swap_target_amount(
			1_000_000_000_000_000_000,
			0,
			1_000_000_000_000_000_000,
			Rate::zero()
		),
		0
	);
  // supply pool is drain
  assert_eq!(
    BDM::calculate_swap_target_amount(
      0,
      1_000_000_000_000_000_000,
      1_000_000_000_000_000_000,
      Rate::zero()
    ),
    0
  );

	// supply amount is zero
	assert_eq!(
		BDM::calculate_swap_target_amount(
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			0,
			Rate::zero()
		),
		0
	);

  // fee rate >= 100%
	assert_eq!(
		BDM::calculate_swap_target_amount(
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			Rate::one()
		),
		0
	);

  // target pool <= target amount
	assert_eq!(
		BDM::calculate_swap_supply_amount(
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			Rate::zero()
		),
		0
	);
	assert_eq!(
		BDM::calculate_swap_supply_amount(
			0,
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			Rate::zero()
		),
		0
	);

	// fee rate >= 100%
	assert_eq!(
		BDM::calculate_swap_supply_amount(
			1_000_000_000_000_000_000,
			1_000_000_000_000_000_000,
			1_000_000_000_000,
			Rate::one()
		),
		0
	);

  let supply_pool = 1_000_000_000_000_000_000_000_000;
  let target_pool = 1_000_000_000_000_000_000_000_000;
  let fee_rate = Rate::saturating_from_rational(1, 100);
  let supply_amount = 1_000_000_000_000_000_000;
  let target_amount = BDM::calculate_swap_target_amount(supply_pool, target_pool, supply_amount, fee_rate);
  let supply_amount_at_least =
    BDM::calculate_swap_supply_amount(supply_pool, target_pool, target_amount, fee_rate);
  assert!(supply_amount_at_least >= supply_amount);

  let supply_pool = 1_000_000;
  let target_pool = 1_000_000_000_000_000_000_000_000;
  let fee_rate = Rate::saturating_from_rational(1, 100);
  let supply_amount = 1_000_000_000_000_000_000;
  let target_amount = BDM::calculate_swap_target_amount(supply_pool, target_pool, supply_amount, fee_rate);
  let supply_amount_at_least =
    BDM::calculate_swap_supply_amount(supply_pool, target_pool, target_amount, fee_rate);
  assert!(supply_amount_at_least >= supply_amount);

  let supply_pool = 195_703_422_673_811_993_405_238u128;
  let target_pool = 8_303_589_956_323_875_342_979u128;
  let fee_rate = Rate::saturating_from_rational(1, 1000); // 0.1%
  let target_amount = 1_000_000_000_000_000u128;
  let supply_amount_at_least =
    BDM::calculate_swap_supply_amount(supply_pool, target_pool, target_amount, fee_rate);
  let actual_target_amount =
    BDM::calculate_swap_target_amount(supply_pool, target_pool, supply_amount_at_least, fee_rate);
  assert!(actual_target_amount >= target_amount);
}

#[test]
fn make_sure_get_supply_amount_needed_can_affort_target() {
	ExtBuilder::default().build().execute_with(|| {
    assert_ok!(BDM::add_liquidity(
			Origin::signed(AccountId::from(ALICE)),
			BXB,
			BETH,
			500000000000,
			100000000000000000
		));

		assert_ok!(BDM::add_liquidity(
			Origin::signed(AccountId::from(ALICE)),
			BUSD,
			BETH,
			500000000000,
			100000000000000000
		));
		assert_ok!(BDM::add_liquidity(
			Origin::signed(AccountId::from(BOB)),
			BUSD,
			DOT,
			80000000000,
			4000000000000
		));

	  let target_amount_busd_beth = 90000000;
    // supply: 80000000000
    // target: 4000000000000
    // new target_pool = 4000000000000 - 90000000 / (1 - 0.01)
    //                 = 4000000000000 - (90,909,090.90909091 ~ 90,909,091)
    //                 = 3999909090909
    // supply_amount = (4000000000000 * 80000000000 ) / 3999909090909- 80000000000
    //               = 1818223.14326 ~ 1818224
		let (amount, route)= BDM::get_supply_amount_needed(BUSD, DOT, target_amount_busd_beth);
    assert_eq!(format_routes(&route), "CurrencyId::DOT,");
    assert_eq!(amount, 1818224, "supply amount should match expected");

    // new_target_pool = 4000000000000 * 80000000000 / (1818224 + 80000000000)
    //                 = 3999909090866.165 ~ 3999909090867
    // target_amount = (4000000000000 - 3999909090867) * 0.99
    //               = 90000041.67 ~ 90000042
		let (target_amount, _)= BDM::get_target_amount_available(BUSD, DOT, amount);
    assert_eq!(target_amount, 90000042);
		assert!(target_amount >= amount);
	});
}
