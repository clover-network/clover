#![cfg(test)]
use super::*;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use sp_std::cell::RefCell;
use std::collections::HashMap;
pub use pallet_balances::Call as BalancesCall;
use clover_traits::{IncentiveOps, IncentivePoolAccountInfo, };

pub use primitives::{
  AccountId, AccountIndex, Amount, Balance,
  CurrencyId,
  EraIndex, Hash, Index, Moment,
  Rate, Share,
  Signature,
};

use orml_currencies::{BasicCurrencyAdapter};

pub type BlockNumber = u64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestRuntime;

mod cloverdex {
  pub use super::super::*;
}

impl_outer_event! {
  pub enum TestEvent for TestRuntime {
    frame_system<T>,
    cloverdex<T>,
    orml_tokens<T>,
    orml_currencies<T>,
    pallet_balances<T>,
  }
}
impl_outer_origin! {
  pub enum Origin for TestRuntime {}
}

parameter_types! {
  pub const BlockHashCount: u64 = 250;
  pub const MaximumBlockWeight: u32 = 1024;
  pub const MaximumBlockLength: u32 = 2 * 1024;
  pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl frame_system::Trait for TestRuntime {
  type Origin = Origin;
  type Index = u64;
  type BlockNumber = BlockNumber;
  type Call = ();
  type Hash = H256;
  type Hashing = ::sp_runtime::traits::BlakeTwo256;
  type AccountId = AccountId;
  type Lookup = IdentityLookup<Self::AccountId>;
  type Header = Header;
  type Event = TestEvent;
  type BlockHashCount = BlockHashCount;
  type MaximumBlockWeight = MaximumBlockWeight;
  type MaximumBlockLength = MaximumBlockLength;
  type AvailableBlockRatio = AvailableBlockRatio;
  type Version = ();
  type PalletInfo = ();
  type AccountData = pallet_balances::AccountData<Balance>;
  type OnNewAccount = ();
  type OnKilledAccount = ();
  type DbWeight = ();
  type BlockExecutionWeight = ();
  type ExtrinsicBaseWeight = ();
  type MaximumExtrinsicWeight = ();
  type BaseCallFilter = ();
  type SystemWeightInfo = ();
}

pub type System = frame_system::Module<TestRuntime>;

parameter_types! {
  pub const ExistentialDeposit: u128 = 500;
  pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Trait for TestRuntime {
  /// The type for recording an account's balance.
  type Balance = Balance;
  /// The ubiquitous event type.
  type Event = TestEvent;
  type DustRemoval = ();
  type ExistentialDeposit = ExistentialDeposit;
  type AccountStore = System;
  type MaxLocks = MaxLocks;
  type WeightInfo = ();
}

pub type Balances = pallet_balances::Module<TestRuntime>;

impl orml_tokens::Trait for TestRuntime {
  type Event = TestEvent;
  type Balance = Balance;
  type Amount = Amount;
  type CurrencyId = CurrencyId;
  type OnReceived = ();
  type WeightInfo = ();
}

pub type Tokens = orml_tokens::Module<TestRuntime>;

parameter_types! {
  pub const GetNativeCurrencyId: CurrencyId = CurrencyId::CLV;
}

impl orml_currencies::Trait for TestRuntime {
  type Event = TestEvent;
  type MultiCurrency = Tokens;
  type NativeCurrency = BasicCurrencyAdapter<TestRuntime, Balances, Amount, BlockNumber>;
  type GetNativeCurrencyId = GetNativeCurrencyId;
  type WeightInfo = ();
}

pub type Currencies = orml_currencies::Module<TestRuntime>;

parameter_types! {
  pub GetExchangeFee: Rate = Rate::saturating_from_rational(1, 100);
  pub const CloverdexModuleId: ModuleId = ModuleId(*b"clv/dexm");
}

impl Trait for TestRuntime {
  type Event = TestEvent;
  type Currency = Currencies;
  type Share = Share;
  type GetExchangeFee = GetExchangeFee;
  type ModuleId = CloverdexModuleId;
  type OnAddLiquidity = ();
  type OnRemoveLiquidity = ();
  type IncentiveOps = IncentiveOpsHandler;
}

pub type CloverdexModule = Module<TestRuntime>;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const CLV: CurrencyId = CurrencyId::CLV;
pub const CUSDT: CurrencyId = CurrencyId::CUSDT;
pub const DOT: CurrencyId = CurrencyId::DOT;
pub const CETH: CurrencyId = CurrencyId::CETH;

thread_local! {
  pub static SHARES_STAKED: RefCell<HashMap<(AccountId, PairKey), Balance>> = RefCell::new(HashMap::new());
}

pub struct IncentiveOpsHandler;

impl IncentiveOps<AccountId, CurrencyId, Share, Balance> for IncentiveOpsHandler {
  fn add_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> Result<Share, DispatchError> {
    let t = SHARES_STAKED.with(|v| {
      let total;
      let mut old_map = v.borrow().clone();
      let key = CloverdexModule::get_pair_key(left, right);
      if let Some(before) = old_map.get_mut(&(who.clone(), key)) {
        *before += amount;
        total = before.clone();
      } else {
        old_map.insert((who.clone(), key), amount.clone());
        total = amount.clone();
      };
      *v.borrow_mut() = old_map;
      total
    });
    Ok(t)
  }

  fn remove_share(who: &AccountId, left: &CurrencyId, right: &CurrencyId, amount: &Share) -> Result<Share, DispatchError> {
    let total = SHARES_STAKED.with(|v| {
      let total;
      let mut old_map = v.borrow().clone();
      let key = CloverdexModule::get_pair_key(left, right);
      if let Some(before) = old_map.get_mut(&(who.clone(), key)) {
        *before -= amount;
        total = before.clone();
      } else {
        old_map.insert((who.clone(), key), amount.clone());
        total = amount.clone();
      };
      *v.borrow_mut() = old_map;
      total
    });
    Ok(total)
  }

  fn get_account_shares(who: &AccountId, left: &CurrencyId, right: &CurrencyId) -> Share {
    SHARES_STAKED.with(|v| {
      let key = CloverdexModule::get_pair_key(left, right);
      v.borrow().get(&(who.clone(), key)).unwrap_or(&0).clone()
    })
  }

  // todo implement it
  fn get_accumlated_rewards(_who: &AccountId, _left: &CurrencyId, _right: &CurrencyId) -> Balance {
    0
  }

  fn get_account_info(_who: &AccountId, _left: &CurrencyId, _right: &CurrencyId) -> IncentivePoolAccountInfo<Share, Balance> {
    IncentivePoolAccountInfo { shares: 0, accumlated_rewards: 0 }
  }

  fn claim_rewards(_who: &AccountId, _left: &CurrencyId, _right: &CurrencyId) -> Result<Balance, DispatchError> {
    Ok(Zero::zero())
  }

  fn get_all_incentive_pools() -> Vec<(CurrencyId, CurrencyId, Share, Balance)> {
    vec![]
  }
}

pub struct ExtBuilder {
  endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
  fn default() -> Self {
    let alice = AccountId::from(ALICE);
    let bob = AccountId::from(BOB);

    Self {
      endowed_accounts: vec![
        (alice.clone(), CLV, 1_000_000_000_000_000_000u128),
        (bob.clone(), CLV, 1_000_000_000_000_000_000u128),
        (alice.clone(), CUSDT, 1_000_000_000_000_000_000u128),
        (bob.clone(), CUSDT, 1_000_000_000_000_000_000u128),
        (alice.clone(), DOT, 1_000_000_000_000_000_000u128),
        (bob.clone(), DOT, 1_000_000_000_000_000_000u128),
        (alice.clone(), CETH, 1_000_000_000_000_000_000u128),
        (bob.clone(), CETH, 1_000_000_000_000_000_000u128),
      ],
    }
  }
}

impl ExtBuilder {
  pub fn build(self) -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
      .build_storage::<TestRuntime>()
      .unwrap();

    pallet_balances::GenesisConfig::<TestRuntime> {
      balances: self
        .endowed_accounts
        .clone()
        .into_iter()
        .filter(|(_, currency_id, _)| *currency_id == CLV)
        .map(|(account_id, _, initial_balance)| (account_id, initial_balance))
        .collect::<Vec<_>>(),
    }
    .assimilate_storage(&mut t)
      .unwrap();


    orml_tokens::GenesisConfig::<TestRuntime> {
      endowed_accounts: self
        .endowed_accounts
        .into_iter()
        .filter(|(_, currency_id, _)| *currency_id != CLV)
        .collect::<Vec<_>>(),
    }
    .assimilate_storage(&mut t).unwrap();

    // cloverdex::GenesisConfig {
    //   initial_pairs: vec![
    //     (CLV, CETH, Some(0), Some(0)),
    //     (CUSDT, CETH, Some(0), Some(0)),
    //     (CUSDT, DOT, Some(0), Some(0)),
    //     (DOT, CETH, Some(0), Some(0)),
    //   ],
    // }.assimilate_storage::<TestRuntime>(&mut t).unwrap();

    t.into()
  }
}
