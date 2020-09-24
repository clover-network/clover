//! Clover Incentives Module
//!
//! ##Overview
//! Implements clover incentives based on reward pool
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
  decl_module, decl_error, decl_event, decl_storage, debug,
  traits::{
    EnsureOrigin, Get, Happened,
  },
  IterableStorageMap,
  weights::constants::WEIGHT_PER_MICROS,
};
use sp_runtime::{
  DispatchResult,
  FixedPointNumber, FixedPointOperand,
  RuntimeDebug,
  traits::{
    AtLeast32Bit, MaybeSerializeDeserialize, Member, SaturatedConversion, Saturating,
    Zero,
  }
};
use sp_std::prelude::*;
use primitives::{Balance, CurrencyId, Price, Share, Ratio};
use clover_traits::{RewardPoolOps, IncentiveOps};
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

pub trait Trait: frame_system::Trait {
  type RewardPool:  RewardPoolOps<Self::AccountId, PoolId, Share>;
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


impl <T: Trait> RewardHandler<T::AccountId, T::BlockNumber, Balance, Share, PoolId> for Module<T>
where T::BlockNumber: SaturatedConversion, {
  fn caculate_reward(pool_id: &PoolId,
                     _total_share: &Share,
                     last_update_block: T::BlockNumber,
                     now: T::BlockNumber) -> Balance {
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

impl<T: Trait> IncentiveOps<T::AccountId, CurrencyId, Share> for Module<T> {
  fn add_share(who: &T::AccountId,
               currency_first: &CurrencyId,
               currency_second: &CurrencyId,
               amount: &Share) -> DispatchResult {
    let pair_key = PairKey::try_from(*currency_first, *currency_second)
      .ok_or(Error::<T>::InvalidCurrencyPair)?;
    T::RewardPool::add_share(who, PoolId::Dex(pair_key), *amount)
  }

  fn remove_share(who: &T::AccountId,
                  currency_first: &CurrencyId,
                  currency_second: &CurrencyId,
                  amount: &Share) -> DispatchResult {
    let pair_key = PairKey::try_from(*currency_first, *currency_second)
      .ok_or(Error::<T>::InvalidCurrencyPair)?;
    T::RewardPool::remove_share(who, PoolId::Dex(pair_key), *amount)
  }
}
