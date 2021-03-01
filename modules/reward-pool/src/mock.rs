#![cfg(test)]
use super::*;
use frame_support::{
  impl_outer_event, impl_outer_origin, parameter_types,
  traits::{OnFinalize, OnInitialize},
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
pub use pallet_balances::Call as BalancesCall;

pub use primitives::{
  AccountId, AccountIndex, Amount, Balance,
  CurrencyId,
  EraIndex, Hash, Index, Moment,
  Rate, Share,
  Signature,
  currency::*,
};

use orml_currencies::{BasicCurrencyAdapter};

pub type BlockNumber = u64;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TestRuntime;

mod reward_pool{
  pub use super::super::*;
}

impl_outer_event! {
  pub enum TestEvent for TestRuntime {
    frame_system<T>,
    reward_pool<T>,
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

impl frame_system::Config for TestRuntime {
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

impl pallet_balances::Config for TestRuntime {
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

impl orml_tokens::Config for TestRuntime {
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

impl orml_currencies::Config for TestRuntime {
  type Event = TestEvent;
  type MultiCurrency = Tokens;
  type NativeCurrency = BasicCurrencyAdapter<TestRuntime, Balances, Amount, BlockNumber>;
  type GetNativeCurrencyId = GetNativeCurrencyId;
  type WeightInfo = ();
}

pub type Currencies = orml_currencies::Module<TestRuntime>;

parameter_types! {
  pub const RewardPoolModuleId: ModuleId = ModuleId(*b"clv/repm");
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, Ord, PartialOrd)]
pub enum PoolId {
  Swap(u64),
}

pub struct Handler;
impl RewardHandler<AccountId, BlockNumber, Balance, Share, PoolId> for Handler {
  // simple reward calculation, 1 block 1 reward
  fn caculate_reward(pool_id: &PoolId, total_share: &Share, last_update_block: BlockNumber,
                     now: BlockNumber) -> Balance {
    println!("calculate reward for pool: {:?}", pool_id);
    if total_share.is_zero() {
      println!("no reward because no share in pool, pool: {:?}", pool_id);
      0
    } else {
      DOLLARS.checked_mul((now - last_update_block).into()).unwrap()
    }
  }
}

impl Trait for TestRuntime {
  type Event = TestEvent;
  type Currency = Currencies;
  type ModuleId = RewardPoolModuleId;
  type GetNativeCurrencyId = GetNativeCurrencyId;
  type PoolId = PoolId;
  type Handler = Handler;
  type ExistentialReward = ExistentialDeposit;
}

pub type RewardPoolModule = Module<TestRuntime>;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const DAVE: [u8; 32] = [2u8; 32];
pub const CLV: CurrencyId = CurrencyId::CLV;
pub const CUSDT: CurrencyId = CurrencyId::CUSDT;
pub const DOT: CurrencyId = CurrencyId::DOT;
pub const CETH: CurrencyId = CurrencyId::CETH;

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

    t.into()
  }
}

pub fn run_to_block(n: u64) {
  while System::block_number() < n {
    RewardPoolModule::on_finalize(System::block_number());
    Currencies::on_finalize(System::block_number());
    Tokens::on_finalize(System::block_number());
    Balances::on_finalize(System::block_number());
    System::on_finalize(System::block_number());
    System::set_block_number(System::block_number() + 1);
    System::on_initialize(System::block_number());
    Balances::on_initialize(System::block_number());
    Tokens::on_initialize(System::block_number());
    Currencies::on_initialize(System::block_number());
    RewardPoolModule::on_initialize(System::block_number());
  }
}
