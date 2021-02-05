#![recursion_limit = "128"]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// #[macro_use]
// pub extern crate rlp_derive;
#[macro_use]
pub mod utils;
pub mod header;
pub mod pow;
pub mod error;
pub mod receipt;
pub mod log;
pub mod network_type;


pub use utils::hex_bytes_unchecked;

pub use ethbloom::{Bloom, Input as BloomInput};
pub use ethereum_types::H64;
pub use primitive_types::{H160, H256, U128, U256, U512};

use codec::{Decode, Encode};
use sp_std::prelude::*;

pub type Bytes = Vec<u8>;
pub type EthereumAddress = H160;
pub type EthereumBlockNumber = u64;

