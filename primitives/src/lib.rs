#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate num_derive;
use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_runtime::{
  FixedU128,
  generic,
  traits::{BlakeTwo256, IdentifyAccount, Verify},
  MultiSignature, RuntimeDebug
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on
/// the chain.
pub type Signature = MultiSignature;

/// Alias to the public key used for this chain, actually a `MultiSigner`. Like
/// the signature, this also isn't a fixed size when encoded, as different
/// cryptos have different size public keys.
pub type AccountPublic = <Signature as Verify>::Signer;

/// Alias to the opaque account ID type for this chain, actually a
/// `AccountId32`. This is always 32 bytes.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them.
pub type AccountIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Opaque, encoded, unchecked extrinsic.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

#[repr(u32)]
#[derive(Encode, Decode, Eq, FromPrimitive, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, enum_iterator::IntoEnumIterator)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, strum_macros::EnumIter, strum_macros::Display, int_enum::IntEnum))]
pub enum CurrencyId {
	  CLV = 0,
	  CUSDT = 1,
	  DOT = 2,
	  CETH = 3,
}

/// dex related types
pub type Rate = FixedU128;
pub type Ratio = FixedU128;
pub type Price = FixedU128;
/// Share type
pub type Share = u128;

pub mod currency {
  use super::*;
  pub const DOLLARS: Balance = 1_000_000_000_000_000_000;
  pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000_000_000
  pub const MILLICENTS: Balance = CENTS / 1000; // 10_000_000_000_000
  pub const MICROCENTS: Balance = MILLICENTS / 1000; // 10_000_000_000
}
