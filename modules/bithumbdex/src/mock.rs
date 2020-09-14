#![cfg(test)]
use super::*;
use frame_support::{impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use sp_std::cell::RefCell;
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

mod bithumbdex {
	pub use super::super::*;
}

impl_outer_event! {
	pub enum TestEvent for TestRuntime {
		frame_system<T>,
		bithumbdex<T>,
		orml_tokens<T>,
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
	type ModuleToIndex = ();
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
}

impl pallet_balances::Trait for TestRuntime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = TestEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}


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
	pub GetExchangeFee: Rate = Rate::saturating_from_rational(1, 100);
	pub const BithumbDexModuleId: ModuleId = ModuleId(*b"bxb/dexm");
}

impl Trait for TestRuntime {
	type Event = TestEvent;
	type Currency = Tokens;
	type Share = Share;
	type GetExchangeFee = GetExchangeFee;
	type ModuleId = BithumbDexModuleId;
	type OnAddLiquidity = ();
	type OnRemoveLiquidity = ();
}

pub type BithumbDexModule = Module<TestRuntime>;

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];
pub const DAVE: [u8; 32] = [2u8; 32];
pub const BXB: CurrencyId = CurrencyId::BXB;
pub const BUSD: CurrencyId = CurrencyId::BUSD;
pub const DOT: CurrencyId = CurrencyId::DOT;
pub const BETH: CurrencyId = CurrencyId::BETH;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
    let alice = AccountId::from(ALICE);
    let bob = AccountId::from(BOB);

		Self {
			endowed_accounts: vec![
        (alice.clone(), BXB, 1_000_000_000_000_000_000u128),
        (bob.clone(), BXB, 1_000_000_000_000_000_000u128),
        (alice.clone(), BUSD, 1_000_000_000_000_000_000u128),
        (bob.clone(), BUSD, 1_000_000_000_000_000_000u128),
        (alice.clone(), DOT, 1_000_000_000_000_000_000u128),
        (bob.clone(), DOT, 1_000_000_000_000_000_000u128),
        (alice.clone(), BETH, 1_000_000_000_000_000_000u128),
        (bob.clone(), BETH, 1_000_000_000_000_000_000u128),
      ],
    }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<TestRuntime>()
			.unwrap();

    //pallet_balances::GenesisConfig::<TestRuntime> {
		//	// Configure endowed accounts with initial balance of 1 << 60.
		//	balances: self.endowed_accounts.iter().cloned()
		//		.map(|k| (k, 1_000_000_000_000_000_000u128))
		//		.collect(),
		//}
		//.assimilate_storage(&mut t)
		//.unwrap();

		orml_tokens::GenesisConfig::<TestRuntime> {
			endowed_accounts: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != BXB)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
      .unwrap();

    bithumbdex::GenesisConfig {
      initial_pairs: vec![
        (BXB, BETH),
        (BXB, BUSD),
        (BUSD, DOT),
      ],
    }
    .assimilate_storage::<TestRuntime>(&mut t)
      .unwrap();

    t.into()
  }
}
