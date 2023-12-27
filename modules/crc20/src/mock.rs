// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate as clover_claims;

use frame_support::parameter_types;
use hex_literal::hex;
use sp_core::H256;
use sp_runtime::{
  testing::Header,
  traits::{BlakeTwo256, IdentityLookup},
};

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

parameter_types! {
    pub Prefix: &'static [u8] = b"Pay CLVs to the TEST account:";
    pub const ClaimsModuleId: ModuleId = ModuleId(*b"clvclaim");
}
impl Config for Test {
  type ModuleId = ClaimsModuleId;
  type Event = ();
  type Currency = Balances;
  type Prefix = Prefix;
}
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
  pub enum Test where
    Block = Block,
    NodeBlock = Block,
    UncheckedExtrinsic = UncheckedExtrinsic
  {
    System: frame_system::{Module, Call, Config, Storage, Event<T>},
    Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
    CloverClaims: clover_claims::{Module, Call, Storage, Event<T>, ValidateUnsigned},
  }
);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
  let mut t = frame_system::GenesisConfig::default()
    .build_storage::<Test>()
    .unwrap();

  pallet_balances::GenesisConfig::<Test> {
    balances: vec![(4, 100), (5, 100)],
  }
  .assimilate_storage(&mut t)
  .unwrap();

  t.into()
}

pub fn get_legal_tx_hash() -> EthereumTxHash {
  EthereumTxHash(hex![
    "4c5adaad6ca9cd2ae9f372b59ff6765fb66082c08caf6e61e6fbc39c35e82bec"
  ])
}

pub fn get_legal_eth_addr() -> EthereumAddress {
  // seed words: cancel cage help flag aisle icon grocery govern include roof kit nut
  EthereumAddress(hex!["243E34C336F3D2c08BBC79b99E6BCA1fA7c58595"])
}

pub fn get_legal_eth_sig() -> EcdsaSignature {
  // `243E34C336F3D2c08BBC79b99E6BCA1fA7c58595`'s sig
  // data: Pay CLVs to the TEST account:01000000000000004c5adaad6ca9cd2ae9f372b59ff6765fb66082c08caf6e61e6fbc39c35e82bec
  EcdsaSignature(hex!["c179736cc655655e14f8b66d386045df26f5f441ba6e58d153dfa1ffdd329ccc602bf23833371f87f408ecb093dc9839f24fa35473c3693ce5395780309f2a7f1b"])
}

pub fn get_another_account_eth_sig() -> EcdsaSignature {
  // account: divert balcony sick decorate like plate faith ivory surface where peanut way
  // `f189cdFF5C1A903e20397b110f5D99B0460C67DC`'s sig
  // data: Pay CLVs to the TEST account:01000000000000004c5adaad6ca9cd2ae9f372b59ff6765fb66082c08caf6e61e6fbc39c35e82bec
  EcdsaSignature(hex!["7dc3cd6d99fb0dd1f8fbc4fae9aec8399e913496e3dbd33ddd83f723665ecf4569b715c430239750ffb2973d094cfa4fbb808b3f0ec1ef2caff5d5e473b2332a1b"])
}

pub fn get_wrong_msg_eth_sig() -> EcdsaSignature {
  // `0x243E34C336F3D2c08BBC79b99E6BCA1fA7c58595`'s sig
  // data: Pay CLVs to the TEST account2:010000000000000
  EcdsaSignature(hex!["7dc3cd6d99fb0dd1f8fbc4fae9aec8399e913496e3dbd33ddd83f723665ecf4569b715c430239750ffb2973d094cfa4fbb808b3f0ec1ef2caff5d5e473b2332a1c"])
}
