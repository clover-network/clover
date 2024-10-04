//! Clover Reward Pool module
//!
//! ##Overview
//! Reward pooling based on shares,
//! Add shares to the pool, receive native currency reward
//! Allow add shares, withdraw shares and coressponding native currency
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_runtime::{
    traits::{AccountIdConversion, UniqueSaturatedInto, Zero},
    DispatchResult, DispatchError, FixedPointNumber, ModuleId,
};
use sp_std::{cmp::{Eq, PartialEq}, vec};

use clover_primitives::{Balance, CurrencyId, Price, Share, Ratio};
use crate::traits::RewardHandler;
use clover_traits::RewardPoolOps;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type PoolId: Parameter + Member + Copy + MaxEncodedLen;
        type ModuleId: Get<ModuleId>;
        type Handler: RewardHandler<Self::AccountId, Self::BlockNumber, Balance, Share, Self::PoolId>;
        type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId, Balance = Balance>;
        type GetNativeCurrencyId: Get<CurrencyId>;
        #[pallet::constant]
        type ExistentialReward: Get<Balance>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn get_pool)]
    pub type Pools<T: Config> = StorageMap<_, Twox64Concat, T::PoolId, PoolInfo<Share, Balance, T::BlockNumber>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pool_account_data)]
    pub type PoolAccountData<T: Config> = StorageDoubleMap<
        _, 
        Twox64Concat, T::PoolId, 
        Twox64Concat, T::AccountId, 
        PoolAccountInfo<Share, Balance>, 
        ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RewardUpdated(T::PoolId, Balance),
        ShareRemoved(T::PoolId, T::AccountId, Share),
    }

    #[pallet::error]
    pub enum Error<T> {
        RewardCalculationError,
        InsufficientShares,
        InvalidAmount,
        InvalidRewards,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default, TypeInfo)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact, Block: HasCompact> {
    #[codec(compact)]
    pub total_shares: Share,
    #[codec(compact)]
    pub total_rewards: Balance,
    #[codec(compact)]
    pub total_rewards_useable: Balance,
    #[codec(compact)]
    pub last_update_block: Block,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default, TypeInfo)]
pub struct PoolAccountInfo<Share: HasCompact, Balance: HasCompact> {
    #[codec(compact)]
    pub shares: Share,
    #[codec(compact)]
    pub borrowed_amount: Balance,
}

impl<T: Config> Pallet<T> {
    pub fn sub_account_id(pool_id: T::PoolId) -> T::AccountId {
        T::ModuleId::get().into_sub_account(pool_id)
    }

    pub fn get_pool_info(pool_id: &T::PoolId) -> PoolInfo<Share, Balance, T::BlockNumber> {
        Self::get_pool(pool_id)
    }

    pub fn get_pool_account_info(pool_id: &T::PoolId, account: &T::AccountId) -> PoolAccountInfo<Share, Balance> {
        Self::pool_account_data(pool_id, account)
    }

    fn get_rewards_by_account_shares(
        pool_info: PoolInfo<Share, Balance, T::BlockNumber>,
        account_info: PoolAccountInfo<Share, Balance>,
        amount: Share
    ) -> Result<(PoolInfo<Share, Balance, T::BlockNumber>, PoolAccountInfo<Share, Balance>, Balance), DispatchError> {
        if amount <= Zero::zero() || pool_info.total_shares < amount || account_info.shares < amount {
            return Err(Error::<T>::InsufficientShares.into());
        }

        let reward_with_virtual = Self::calc_reward_by_shares(&pool_info, &amount)?;

        let PoolInfo { total_shares, total_rewards, total_rewards_useable, ..} = pool_info;
        let PoolAccountInfo { shares, borrowed_amount } = account_info;

        let account_balance_to_remove = Ratio::checked_from_rational(amount, shares)
            .and_then(|n| n.checked_mul_int(borrowed_amount))
            .ok_or(Error::<T>::RewardCalculationError)?;

        let new_balance = borrowed_amount.checked_sub(account_balance_to_remove)
            .ok_or(Error::<T>::RewardCalculationError)?;

        let reward = reward_with_virtual.checked_sub(account_balance_to_remove)
            .ok_or(Error::<T>::RewardCalculationError)?;

        if reward > total_rewards_useable || new_balance > borrowed_amount {
            log::error!("got wrong reward for account: {:?}, pool info: {:?}, shares: {:?}", account_info, pool_info, amount);
            return Err(Error::<T>::RewardCalculationError.into());
        }

        let total_shares = total_shares.checked_sub(amount)
            .ok_or(Error::<T>::InsufficientShares)?;

        let (reward, total_rewards, total_rewards_useable) = if reward <= T::ExistentialReward::get() {
            log::warn!("reward {:?} is less than existential reward, don't send the reward", reward);
            (0, total_rewards, total_rewards_useable)
        } else {
            let rewards = total_rewards.checked_sub(reward_with_virtual)
                .ok_or(Error::<T>::RewardCalculationError)?;
            let rewards_useable = total_rewards_useable.checked_sub(reward)
                .ok_or(Error::<T>::RewardCalculationError)?;
            (reward, rewards, rewards_useable)
        };

        let pool_info = PoolInfo {
            total_rewards, total_rewards_useable, total_shares,
            ..pool_info
        };
        let shares = shares.checked_sub(amount)
            .ok_or(Error::<T>::RewardCalculationError)?;

        let account_info = PoolAccountInfo {
            shares, borrowed_amount: new_balance,
            ..account_info
        };

        Ok((pool_info, account_info, reward))
    }

    fn calc_reward_by_shares(
        pool_info: &PoolInfo<Share, Balance, T::BlockNumber>,
        amount: &Share
    ) -> Result<Balance, DispatchError> {
        let PoolInfo { total_shares, total_rewards, ..} = pool_info;

        if total_shares.is_zero() || amount.is_zero() {
            return Ok(Zero::zero());
        }

        if amount > total_shares {
            return Err(Error::<T>::InsufficientShares.into());
        }

        let reward_with_virtual = Ratio::checked_from_rational::<Balance, _>(amount.clone().into(), total_shares.clone())
            .and_then(|n| n.checked_mul_int(total_rewards.clone()))
            .ok_or(Error::<T>::RewardCalculationError)?;

        if &reward_with_virtual > total_rewards {
            log::error!("got wrong reward for pool info: {:?}, shares: {:?}", pool_info, amount);
            return Err(Error::<T>::RewardCalculationError.into());
        }

        Ok(reward_with_virtual)
    }

    fn update_pool_reward(pool: &T::PoolId) -> Result<PoolInfo<Share, Balance, T::BlockNumber>, DispatchError> {
        let (pool_info, balance_change) = Self::calc_pool_reward(pool)?;

        if !balance_change.is_zero() {
            let sub_account = Self::sub_account_id(pool.clone());
            log::info!("updating reward pool {:?}, account {:?} balance by: {:?}", pool, sub_account, balance_change);

            let amount = balance_change.unique_saturated_into();
            T::Currency::update_balance(T::GetNativeCurrencyId::get(), &sub_account, amount)?;
        }
        Pools::<T>::mutate(pool, |info| {
            *info = pool_info.clone();
        });

        Ok(pool_info)
    }

    fn calc_pool_reward(
        pool: &T::PoolId,
    ) -> Result<(PoolInfo<Share, Balance, T::BlockNumber>, Balance), DispatchError> {
        let pool_info = Self::get_pool(pool);
        let cur_block = <frame_system::Pallet<T>>::block_number();
        Self::calc_pool_reward_at_block(pool, &pool_info, &cur_block)
    }

    fn calc_pool_reward_at_block(
        pool: &T::PoolId,
        pool_info: &PoolInfo<Share, Balance, T::BlockNumber>,
        cur_block: &T::BlockNumber
    ) -> Result<(PoolInfo<Share, Balance, T::BlockNumber>, Balance), DispatchError> {
        let last_update_block  = pool_info.last_update_block;
        if cur_block <= &last_update_block {
            log::info!("ignore update pool reward: {:?} at block: {:?}, already updated at: {:?}", pool, cur_block, last_update_block);
            return Ok((pool_info.clone(), 0));
        }

        let reward = T::Handler::caculate_reward(pool, &pool_info.total_shares, last_update_block, cur_block.clone());

        let mut new_info = pool_info.clone();
        new_info.last_update_block = cur_block.clone();

        if reward == 0 {
            log::warn!("0 reward: {:?}, pool: {:?}, between {:?} - {:?}", reward, pool, last_update_block, cur_block);
            return Ok((new_info, 0));
        }

        new_info.total_rewards = new_info.total_rewards.checked_add(reward).ok_or(Error::<T>::RewardCalculationError)?;
        new_info.total_rewards_useable = new_info.total_rewards_useable.checked_add(reward).ok_or(Error::<T>::RewardCalculationError)?;

        Ok((new_info, reward))
    }
}

impl<T: Config> RewardPoolOps<T::AccountId, T::PoolId, Share, Balance> for Pallet<T> {
    fn add_share(who: &T::AccountId, pool: T::PoolId, amount: Share) -> Result<Share, DispatchError> {
        if amount.is_zero() {
            return Err(Error::<T>::InvalidAmount.into());
        }

        let mut pool_info = Self::update_pool_reward(&pool)?;

        let price = if pool_info.total_shares.is_zero() {
            Ok(Price::zero())
        } else {
            Price::checked_from_rational(pool_info.total_rewards, pool_info.total_shares)
                .ok_or(Error::<T>::RewardCalculationError)
        }?;

        let virtual_reward_amount = price
            .checked_mul_int(amount)
            .ok_or(Error::<T>::RewardCalculationError)?;

        pool_info.total_shares = pool_info.total_shares.checked_add(amount)
            .ok_or(Error::<T>::RewardCalculationError)?;
        pool_info.total_rewards = pool_info.total_rewards.checked_add(virtual_reward_amount.into())
            .ok_or(Error::<T>::RewardCalculationError)?;

        Pools::<T>::mutate(pool, |info| {
            *info = pool_info;
        });

        let mut total_shares = 0;
        PoolAccountData::<T>::try_mutate(pool, who, |data| -> DispatchResult {
            data.shares = data.shares.checked_add(amount).ok_or(Error::<T>::RewardCalculationError)?;
            data.borrowed_amount = data.borrowed_amount.checked_add(virtual_reward_amount.into()).ok_or(Error::<T>::RewardCalculationError)?;
            total_shares = data.shares;
            Ok(())
        })?;

        Ok(total_shares)
    }

    fn remove_share(who: &T::AccountId, pool: T::PoolId, amount: Share) -> Result<Share, DispatchError> {
        let pool_info = Self::update_pool_reward(&pool)?;
        let account_info = Self::pool_account_data(&pool, &who);
        
        log::info!("to remove shares: {:?}, amount: {:?}", account_info.shares, amount);
        if account_info.shares < amount {
            return Err(Error::<T>::InsufficientShares.into());
        }

        let (pool_info, account_info, reward) = Self::get_rewards_by_account_shares(pool_info, account_info, amount)?;

        Pools::<T>::mutate(pool, |info| {
            *info = pool_info;
        });

        PoolAccountData::<T>::mutate(pool, &who, |data| {
            *data = account_info.clone();
        });

        let sub_account = Self::sub_account_id(pool);
        T::Currency::transfer(T::GetNativeCurrencyId::get(), &sub_account, &who, reward)?;

        Ok(account_info.shares)
    }

    fn get_account_shares(who: &T::AccountId, pool: &T::PoolId) -> Share {
        let PoolAccountInfo { shares, .. } = Self::get_pool_account_info(pool, who);
        shares
    }

    fn get_accumlated_rewards(who: &T::AccountId, pool: &T::PoolId) -> Balance {
        let account_info  = Self::get_pool_account_info(pool, who);
        if account_info.shares.is_zero() {
            return 0;
        }

        let calc_reward = || -> Result<Balance, DispatchError> {
            let (pool_info, _) = Self::calc_pool_reward(pool)?;
            let shares = account_info.shares.clone();
            let (_, _, reward) = Self::get_rewards_by_account_shares(pool_info, account_info, shares)?;
            Ok(reward)
        };
        match calc_reward() {
            Ok(reward) => reward,
            Err(e) => {
                log::error!("failed to calculate reward for account: {:?}, pool: {:?}, error: {:?}", who, pool, e);
                Zero::zero()
            }
        }
    }

  fn claim_rewards(who: &T::AccountId, pool: &T::PoolId) -> Result<Balance, DispatchError> {
    // update accumlated rewards for the pool
    let pool_info = Self::update_pool_reward(&pool)?;
    let account_info  = Self::get_pool_account_info(&pool, who);

    if account_info.shares.is_zero() {
      return Ok(Zero::zero());
    }

    let reward_with_virtual = Self::calc_reward_by_shares(&pool_info, &account_info.shares)?;

    let PoolInfo { total_rewards_useable, ..} = pool_info;
    let PoolAccountInfo { borrowed_amount, ..} = account_info;

    let actual_reward = reward_with_virtual.checked_sub(borrowed_amount)
      .ok_or(Error::<T>::RewardCaculationError)?;
    // don't have enough rewards to claim
    if actual_reward < T::ExistentialReward::get() {
      return Ok(Zero::zero());
    }
    let total_rewards_useable = total_rewards_useable.checked_sub(actual_reward)
      .ok_or(Error::<T>::RewardCaculationError)?;

    // another check, total rewards should be greater than borrowed amount
    if borrowed_amount > reward_with_virtual {
      return Err(Error::<T>::RewardCaculationError.into());
    }
    // since we've claimed all available rewards, we should borrow the reward from the pool, the claimable rewards is zero
    let borrowed_amount = reward_with_virtual;

    let sub_account = Self::sub_account_id(pool.clone());

    T::Currency::transfer(T::GetNativeCurrencyId::get(), &sub_account, &who, actual_reward.unique_saturated_into())?;

    <Pools<T>>::mutate(pool, |info| {
      info.total_rewards_useable = total_rewards_useable;
    });

    <PoolAccountData<T>>::mutate(pool, who, |data| {
      data.borrowed_amount = borrowed_amount;
    });

    Ok(actual_reward)
  }

  fn get_all_pools() -> vec::Vec<(T::PoolId, Share, Balance)> {
    let cur_block = <frame_system::Pallet<T>>::block_number();
    <Pools<T>>::iter()
      .map(|(pool_id, info)| {
        let result = Self::calc_pool_reward_at_block(&pool_id, &info, &cur_block);
        match result {
          Ok((new_info, _)) => (pool_id, new_info.total_shares, new_info.total_rewards_useable),
          Err(e) => {
            debug::error!("failed to get pool info for {:?}, error: {:?}", pool_id, e);
            (pool_id, info.total_shares, Zero::zero())
          },
        }
      }).collect()
  }
}

