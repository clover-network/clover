#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	ALICE, PoolId, RewardPoolModule,
  run_to_block,
  ExtBuilder, Origin, TestRuntime, System,
};

pub use primitives::{ AccountId };

use RewardPoolModule as RPM;


#[test]
fn test_reward_calc_on_no_shares() {
  ExtBuilder::default().build().execute_with(|| {
    let pool_id = PoolId::Swap(1);
    run_to_block(10);
    let r = RPM::update_pool_reward(&pool_id);
    assert_eq!(r.is_ok(), true);
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
