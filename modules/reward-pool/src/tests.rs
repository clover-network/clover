#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	ALICE, Currencies, PoolId, RewardPoolModule,
  run_to_block,
  ExtBuilder, Origin, TestRuntime, System,
};

pub use primitives::{ AccountId, currency::*, };

use RewardPoolModule as RPM;

fn check_pool_data (pool_id: &PoolId, account: &AccountId,
                    total_shares: Share,
                    total_rewards: Balance, total_rewards_useable: Balance,
                    alice_shares: Share, alice_borrow: Balance,) {
  let pool_info = RPM::get_pool_info(&pool_id);
  assert_eq!(pool_info.total_shares, total_shares);
  assert_eq!(pool_info.total_rewards, total_rewards);
  assert_eq!(pool_info.total_rewards_useable, total_rewards_useable);

  let alice_info = RPM::get_pool_account_info(&pool_id, &account);
  assert_eq!(alice_info.shares, alice_shares);
  assert_eq!(alice_info.borrowed_amount, alice_borrow);
}


#[test]
fn test_reward_calc_on_no_shares() {
  ExtBuilder::default().build().execute_with(|| {
    let pool_id = PoolId::Swap(1);
    run_to_block(10);
    let r = RPM::update_pool_reward(&pool_id);
    assert!(r.is_ok());
    let pool_info = RPM::get_pool_info(&pool_id);
    assert_eq!(pool_info.total_shares, 0, "should be no shares");
    assert_eq!(pool_info.total_rewards, 0, "should be no rewards");
    assert_eq!(pool_info.total_rewards_useable, 0, "should be no rewards usable");

    // sometime passed...
    run_to_block(20);
    let r = RPM::update_pool_reward(&pool_id);
    assert_eq!(r.is_ok(), true);
    let pool_info = RPM::get_pool_info(&pool_id);
    assert_eq!(pool_info.total_shares, 0, "should be no shares");
    assert_eq!(pool_info.total_rewards, 0, "should be no rewards");
    assert_eq!(pool_info.total_rewards_useable, 0, "should be no rewards usable");
  });
}

#[test]
fn test_reward_single_account() {
  let pool_id = PoolId::Swap(1);
  let alice = AccountId::from(ALICE);
  let pool_account = RPM::sub_account_id(pool_id.clone());

  ExtBuilder::default().build().execute_with(|| {
    let initial_balance = Currencies::total_balance(CurrencyId::BXB, &alice);
    run_to_block(10);
    assert!(RPM::add_share(&alice, pool_id, 100).is_ok(), "should add shares to the pool");
    run_to_block(20);
    assert!(RPM::update_pool_reward(&pool_id).is_ok());
    let pool_info = RPM::get_pool_info(&pool_id);
    assert_eq!(pool_info.last_update_block, 20);
    check_pool_data(&pool_id, &alice, 100, 10 * DOLLARS, 10 * DOLLARS, 100, 0);

    run_to_block(30);
    assert!(RPM::add_share(&alice, pool_id, 100).is_ok(), "should add shares to the pool");
    let pool_info = RPM::get_pool_info(&pool_id);
    assert_eq!(pool_info.last_update_block, 30);
    check_pool_data(&pool_id, &alice, 200, 40 * DOLLARS, 20 * DOLLARS, 200, 20 * DOLLARS);

    run_to_block(40);

    // before remove, total: 50, useable: 30
    let r = RPM::remove_share(&alice, pool_id, 100);
    assert!(r.is_ok(), "should add shares to the pool");

    check_pool_data(&pool_id, &alice, 100, 25 * DOLLARS, 15 * DOLLARS, 100, 10 * DOLLARS);

    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &pool_account), 15 * DOLLARS);

    run_to_block(50);
    assert!(RPM::remove_share(&alice, pool_id, 100).is_ok(), "should remove shares to the pool");
    check_pool_data(&pool_id, &alice, 0, 0, 0, 0, 0);
    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &alice), initial_balance + 40 * DOLLARS);
    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &pool_account), 0);
  });
}

#[test]
fn test_reward_single_account_existential() {
  let pool_id = PoolId::Swap(1);
  let alice = AccountId::from(ALICE);
  let pool_account = RPM::sub_account_id(pool_id.clone());

  ExtBuilder::default().build().execute_with(|| {
    let initial_balance = Currencies::total_balance(CurrencyId::BXB, &alice);
    run_to_block(10);
    assert!(RPM::add_share(&alice, pool_id, 1_000_000_000_000).is_ok(), "should add shares to the pool");
    run_to_block(11);
    assert!(RPM::remove_share(&alice, pool_id, 1).is_ok());
    // 1 share's reward is 1, which is too small to send
    check_pool_data(&pool_id, &alice, 999_999_999_999, 1 * DOLLARS, 1 * DOLLARS, 999_999_999_999, 0);

    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &alice), initial_balance);
    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &pool_account), 1 * DOLLARS);

    run_to_block(20);
    assert!(RPM::remove_share(&alice, pool_id, 999_999_999_999).is_ok());
    // remove all shares, all reward should send to alice
    check_pool_data(&pool_id, &alice, 0, 0, 0, 0, 0);

    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &alice), initial_balance + 10 * DOLLARS);
    assert_eq!(Currencies::total_balance(CurrencyId::BXB, &pool_account), 0);
  });
}
