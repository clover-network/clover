//! Clover Incentives Module
//!
//! ##Overview
//! Implements clover incentives based on reward pool
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
  decl_module, decl_error, decl_storage, debug,
};
use sp_runtime::{
  DispatchError,
  RuntimeDebug,
  traits::{
    SaturatedConversion,
    Zero,
  }
};
use sp_std::prelude::*;
use sp_std::vec;
use primitives::{Balance, CurrencyId, Share, };
use clover_traits::{RewardPoolOps, IncentiveOps, IncentivePoolAccountInfo, };
use reward_pool::traits::RewardHandler;

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PairKey {
  left: CurrencyId,
  right: CurrencyId,
}

impl PairKey {
  fn try_from(first: CurrencyId, second: CurrencyId) -> Option<Self> {
    if first == second {
      None
    } else if first < second {
      Some(PairKey { left: first, right: second, })
    } else {
      Some(PairKey { left: second, right: first, })
    }
  }
}

/// PoolId for various rewards pools
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PoolId {
  /// Rewards for dex module
  Dex(PairKey),
}

pub trait Trait: frame_system::Config{
  type RewardPool:  RewardPoolOps<Self::AccountId, PoolId, Share, Balance>;
}

decl_storage! {
  trait Store for Module<T: Trait> as Incentives {
    // mapping from pool id to its incentive reward per block
    pub DexIncentiveRewards get(fn dex_incentive_rewards): map hasher(twox_64_concat) PoolId => Balance;
  }

  add_extra_genesis {
    config(dex_rewards): Vec<(CurrencyId, CurrencyId, Balance)>;

    build(|config: &GenesisConfig| {
      debug::info!("got incentives config: {:?}", config.dex_rewards);
      for (left, right, reward_per_block) in &config.dex_rewards {
        let pair_key = PairKey::try_from(*left, *right).unwrap();
        assert!(!reward_per_block.is_zero());
        DexIncentiveRewards::insert(PoolId::Dex(pair_key), reward_per_block);
      }
    })
  }
}

decl_error! {
  /// Error for incentive module.
  pub enum Error for Module<T: Trait> {
    /// invalid currency pair
    InvalidCurrencyPair,
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    type Error = Error<T>;
  }
}

//
// we don't support auto staking for lp tokens
// pub struct OnAddLiquidity<T>(sp_std::marker::PhantomData<T>);
// impl<T: Trait> Happened<(T::AccountId, CurrencyId, CurrencyId, Share)> for OnAddLiquidity<T> {
// 	fn happened(info: &(T::AccountId, CurrencyId, CurrencyId, Share)) {
// 		let (who, currency_first, currency_second, increase_share) = info;
//     if currency_first == currency_second {
//       debug::error!("invalid currency pair for add liquidity event, currency {:?}", currency_first);
//       return;
//     }
//     let pair_key = PairKey::try_from(*currency_first, *currency_second).unwrap();
//
// 		match T::RewardPool::add_share(who, PoolId::Dex(pair_key), *increase_share) {
//       Ok(_) => (),
//       Err(e) => {
//         debug::error!("failed remove share from pool!, {:?}", e);
//       }
//     }
// 	}
// }
//
//
// pub struct OnRemoveLiquidity<T>(sp_std::marker::PhantomData<T>);
// impl<T: Trait> Happened<(T::AccountId, CurrencyId, CurrencyId, Share)> for OnRemoveLiquidity<T> {
// 	fn happened(info: &(T::AccountId, CurrencyId, CurrencyId, Share)) {
// 		let (who, currency_first, currency_second, decrease_share) = info;
//     if currency_first == currency_second {
//       debug::error!("invalid currency pair for remove liquidity event, currency {:?}", currency_first);
//       return;
//     }
//     let pair_key = PairKey::try_from(*currency_first, *currency_second).unwrap();
// 		match T::RewardPool::remove_share(who, PoolId::Dex(pair_key), *decrease_share) {
//       Ok(_) => (),
//       Err(e) => {
//         debug::error!("failed remove share from pool!, {:?}", e);
//       }
//     }
// 	}
// }


impl <T: Trait> Module<T> {
  fn get_dex_id(first: &CurrencyId, second: &CurrencyId) -> Result<PoolId, DispatchError> {
    let pair_key = PairKey::try_from(*first, *second)
      .ok_or(Error::<T>::InvalidCurrencyPair)?;
    Ok(PoolId::Dex(pair_key))
  }
}

impl <T: Trait> RewardHandler<T::AccountId, T::BlockNumber, Balance, Share, PoolId> for Module<T>
where T::BlockNumber: SaturatedConversion, {
  fn caculate_reward(pool_id: &PoolId,
                     total_share: &Share,
                     last_update_block: T::BlockNumber,
                     now: T::BlockNumber) -> Balance {
    // no shares in the pool, should not pay the reward
    if total_share.is_zero() {
      return Balance::zero();
    }

    if !DexIncentiveRewards::contains_key(pool_id) || last_update_block >= now {
      return Balance::zero();
    }
    let reward_ratio = Self::dex_incentive_rewards(pool_id);
    if reward_ratio.is_zero() {
      return Balance::zero();
    }
    let blocks = now - last_update_block;
    reward_ratio.saturating_mul(blocks.saturated_into())
  }
}

impl<T: Trait> IncentiveOps<T::AccountId, CurrencyId, Share, Balance> for Module<T> {

  fn add_share(who: &T::AccountId,
               currency_first: &CurrencyId,
               currency_second: &CurrencyId,
               amount: &Share) -> Result<Share, DispatchError>{
    let pair_key = PairKey::try_from(*currency_first, *currency_second)
      .ok_or(Error::<T>::InvalidCurrencyPair)?;
    T::RewardPool::add_share(who, PoolId::Dex(pair_key), *amount)
  }

  fn remove_share(who: &T::AccountId,
                  currency_first: &CurrencyId,
                  currency_second: &CurrencyId,
                  amount: &Share) -> Result<Share, DispatchError> {
    let pair_key = PairKey::try_from(*currency_first, *currency_second)
      .ok_or(Error::<T>::InvalidCurrencyPair)?;
    T::RewardPool::remove_share(who, PoolId::Dex(pair_key), *amount)
  }

  fn get_account_shares(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> Share {
    if let Ok(id) = Self::get_dex_id(left, right) {
      T::RewardPool::get_account_shares(who, &id)
    } else {
      Zero::zero()
    }
  }

  fn get_accumlated_rewards(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> Share {
    if let Ok(id) = Self::get_dex_id(left, right) {
      T::RewardPool::get_accumlated_rewards(who, &id)
    } else {
      Zero::zero()
    }
  }

  fn get_account_info(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> IncentivePoolAccountInfo<Share, Balance> {
    if let Ok(pool_id) = Self::get_dex_id(left, right) {
      let shares = T::RewardPool::get_account_shares(who, &pool_id);
      let accumlated_rewards = T::RewardPool::get_accumlated_rewards(who, &pool_id);
      IncentivePoolAccountInfo { shares, accumlated_rewards, }
    } else {
      IncentivePoolAccountInfo { shares: Zero::zero(), accumlated_rewards: Zero::zero(), }
    }
  }

  fn claim_rewards(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> Result<Balance, DispatchError> {
    Self::get_dex_id(left, right)
      .and_then(|pool_id| T::RewardPool::claim_rewards(who, &pool_id))
  }

  fn get_all_incentive_pools() -> vec::Vec<(CurrencyId, CurrencyId, Share, Balance)>{
    T::RewardPool::get_all_pools()
      .iter()
      .filter(|(pool_id, _, _)| match pool_id {
        PoolId::Dex(_) => true,
      })
      .map(|(pool_id, shares, balance)| match pool_id {
        PoolId::Dex(k) => (k.left, k.right, shares.clone(), balance.clone()),
      })
      .collect()
  }
}
