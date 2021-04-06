// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate as clover_claims;

use sp_core::H256;
use frame_support::parameter_types;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup}, testing::Header,
};
use hex_literal::hex;

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

parameter_types!{
    pub Prefix: &'static [u8] = b"Pay CLVs to the TEST account:";
}
impl Config for Test {
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
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

    pallet_balances::GenesisConfig::<Test> {
      balances: vec![(4, 100), (5, 100)],
    }.assimilate_storage(&mut t).unwrap();

    t.into()
}

pub fn get_legal_tx_hash() -> EthereumTxHash {
    EthereumTxHash(hex!["4c5adaad6ca9cd2ae9f372b59ff6765fb66082c08caf6e61e6fbc39c35e82bec"])
}

pub fn get_legal_eth_addr() -> EthereumAddress {
    // seed words: cancel cage help flag aisle icon grocery govern include roof kit nut
    EthereumAddress(hex!["243E34C336F3D2c08BBC79b99E6BCA1fA7c58595"])
}

pub fn get_legal_eth_sig() -> EcdsaSignature {
    // `243E34C336F3D2c08BBC79b99E6BCA1fA7c58595`'s sig
    // data: Pay CLVs to the TEST account:0100000000000000
    EcdsaSignature(hex!["ba7b9daeb565e43bebbdb9a7dbfd35091c52586369ae061e8d0d2e33e85d505543402fa317ba950238d057efbeed07cd4bd072600549916fd1a724c7cc06a0e71c"])
}

pub fn get_another_account_eth_sig() -> EcdsaSignature {
    // account: divert balcony sick decorate like plate faith ivory surface where peanut way
    // `f189cdFF5C1A903e20397b110f5D99B0460C67DC`'s sig
    // data: Pay CLVs to the TEST account:0100000000000000
    EcdsaSignature(hex!["a78348a71e178e4865e6afa541d8b692e0f94a750d03b3f632274c877ca4450f0b2b51a2a4858e36415d7eec87e8dc20dd7ef9fb60b75df0adf88a9057396d311b"])
}

pub fn get_wrong_msg_eth_sig() -> EcdsaSignature {
    // `0x243E34C336F3D2c08BBC79b99E6BCA1fA7c58595`'s sig
    // data: Pay CLVs to the TEST account2:010000000000000
    EcdsaSignature(hex!["048e01508d19174608c4b65d3c87c0b9f44911361b362a9d30650113cd907b646c3c29ff1f0882d9db2563ca9afd276fe5cd94337a330a3fb19303fa0b3012301b"])

}
