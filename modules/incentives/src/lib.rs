//! Clover Incentives Module
//!
//! ##Overview
//! Implements clover incentives based on reward pool
#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
  decl_module, decl_event, decl_storage,
  traits::{EnsureOrigin, Get, Happened},
  IterableStorageMap,
};
use sp_runtime::{RuntimeDebug};
use sp_std::prelude::*;

/// PoolId for various rewards pools
#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PoolId {
  /// Rewards for dex module
  Dex(u64),
}

pub trait Trait: frame_system::Trait {
}
