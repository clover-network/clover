#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::Get};
use frame_system::pallet_prelude::*;
use sp_runtime::{
    traits::{SaturatedConversion, Zero},
    DispatchError,
};
use sp_std::prelude::*;

use clover_primitives::{Balance, CurrencyId, Share};
use clover_traits::{IncentiveOps, IncentivePoolAccountInfo, RewardPoolOps};
use reward_pool::traits::RewardHandler;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RewardPool: RewardPoolOps<Self::AccountId, PoolId, Share, Balance>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn dex_incentive_rewards)]
    pub type DexIncentiveRewards<T> = StorageMap<_, Twox64Concat, PoolId, Balance, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig {
        pub dex_rewards: Vec<(CurrencyId, CurrencyId, Balance)>,
    }

    #[cfg(feature = "std")]
    impl Default for GenesisConfig {
        fn default() -> Self {
            Self { dex_rewards: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig {
        fn build(&self) {
            for (left, right, reward_per_block) in &self.dex_rewards {
                if let Some(pair_key) = PairKey::try_from(*left, *right) {
                    assert!(!reward_per_block.is_zero());
                    DexIncentiveRewards::<T>::insert(PoolId::Dex(pair_key), reward_per_block);
                }
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidCurrencyPair,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {}
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PairKey {
    left: CurrencyId,
    right: CurrencyId,
}

impl PairKey {
    fn try_from(first: CurrencyId, second: CurrencyId) -> Option<Self> {
        if first == second {
            None
        } else if first < second {
            Some(PairKey { left: first, right: second })
        } else {
            Some(PairKey { left: second, right: first })
        }
    }
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum PoolId {
    Dex(PairKey),
}

impl<T: Config> Pallet<T> {
    fn get_dex_id(first: &CurrencyId, second: &CurrencyId) -> Result<PoolId, DispatchError> {
        PairKey::try_from(*first, *second)
            .map(PoolId::Dex)
            .ok_or(Error::<T>::InvalidCurrencyPair.into())
    }
}

impl<T: Config> RewardHandler<T::AccountId, T::BlockNumber, Balance, Share, PoolId> for Pallet<T>
where
    T::BlockNumber: SaturatedConversion,
{
    fn caculate_reward(
        pool_id: &PoolId,
        total_share: &Share,
        last_update_block: T::BlockNumber,
        now: T::BlockNumber,
    ) -> Balance {
        if total_share.is_zero() || last_update_block >= now {
            return Balance::zero();
        }

        if let Some(reward_ratio) = DexIncentiveRewards::<T>::get(pool_id) {
            if !reward_ratio.is_zero() {
                let blocks = now - last_update_block;
                return reward_ratio.saturating_mul(blocks.saturated_into());
            }
        }

        Balance::zero()
    }
}

impl<T: Config> IncentiveOps<T::AccountId, CurrencyId, Share, Balance> for Pallet<T> {
    fn add_share(
        who: &T::AccountId,
        currency_first: &CurrencyId,
        currency_second: &CurrencyId,
        amount: &Share,
    ) -> Result<Share, DispatchError> {
        let pool_id = Self::get_dex_id(currency_first, currency_second)?;
        T::RewardPool::add_share(who, pool_id, *amount)
    }

    fn remove_share(
        who: &T::AccountId,
        currency_first: &CurrencyId,
        currency_second: &CurrencyId,
        amount: &Share,
    ) -> Result<Share, DispatchError> {
        let pool_id = Self::get_dex_id(currency_first, currency_second)?;
        T::RewardPool::remove_share(who, pool_id, *amount)
    }

    fn get_account_shares(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> Share {
        Self::get_dex_id(left, right)
            .map(|id| T::RewardPool::get_account_shares(who, &id))
            .unwrap_or_else(|_| Zero::zero())
    }

    fn get_accumlated_rewards(who: &T::AccountId, left: &CurrencyId, right: &CurrencyId) -> Share {
        Self::get_dex_id(left, right)
            .map(|id| T::RewardPool::get_accumlated_rewards(who, &id))
            .unwrap_or_else(|_| Zero::zero())
    }

    fn get_account_info(
        who: &T::AccountId,
        left: &CurrencyId,
        right: &CurrencyId,
    ) -> IncentivePoolAccountInfo<Share, Balance> {
        Self::get_dex_id(left, right)
            .map(|pool_id| {
                let shares = T::RewardPool::get_account_shares(who, &pool_id);
                let accumlated_rewards = T::RewardPool::get_accumlated_rewards(who, &pool_id);
                IncentivePoolAccountInfo { shares, accumlated_rewards }
            })
            .unwrap_or_else(|_| IncentivePoolAccountInfo {
                shares: Zero::zero(),
                accumlated_rewards: Zero::zero(),
            })
    }

    fn claim_rewards(
        who: &T::AccountId,
        left: &CurrencyId,
        right: &CurrencyId,
    ) -> Result<Balance, DispatchError> {
        let pool_id = Self::get_dex_id(left, right)?;
        T::RewardPool::claim_rewards(who, &pool_id)
    }

    fn get_all_incentive_pools() -> Vec<(CurrencyId, CurrencyId, Share, Balance)> {
        T::RewardPool::get_all_pools()
            .into_iter()
            .filter_map(|(pool_id, shares, balance)| match pool_id {
                PoolId::Dex(k) => Some((k.left, k.right, shares, balance)),
            })
            .collect()
    }
}
