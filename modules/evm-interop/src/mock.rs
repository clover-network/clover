// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate as clover_evm_interop;

use frame_support::parameter_types;
use hex_literal::hex;
use sp_core::H160;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};
use std::str::FromStr;

parameter_types! {
    pub const BlockHashCount: u32 = 250;
}
impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
}

impl Config for Test {
    type Event = ();
    type Currency = Balances;
    type AddressMapping = AddressMappingHandler;
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub struct AddressMappingHandler;
impl AddressMapping<u64> for AddressMappingHandler {
    fn into_account_id(address: H160) -> u64 {
        match address {
            a if a == H160::from_str("2200000000000000000000000000000000000000").unwrap() => 8u64,
            a if a == H160::from_str("2200000000000000000000000000000000000001").unwrap() => 9u64,
            a if a == H160::from_str("2200000000000000000000000000000000000002").unwrap() => 10u64,
            a if a == H160::from_str("2200000000000000000000000000000000000003").unwrap() => 11u64,
            _ => 128u64,
        }
    }
    fn to_evm_address(_account: &u64) -> Option<sp_core::H160> {
        None
    }
}

frame_support::construct_runtime!(
  pub enum Test where
    Block = Block,
    NodeBlock = Block,
    UncheckedExtrinsic = UncheckedExtrinsic
  {
    System: frame_system::{Module, Call, Config, Storage, Event<T>},
    Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
    CloverEvmInterOp: clover_evm_interop::{Module, Call, Storage, Event<T>, },
  }
);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(4, 100_000_000), (5, 100_000_000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
