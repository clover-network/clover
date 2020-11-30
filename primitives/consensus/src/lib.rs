#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use sp_std::vec::Vec;
use sp_core::H256;
use sp_runtime::ConsensusEngineId;

pub const FRONTIER_ENGINE_ID: ConsensusEngineId = [b'f', b'r', b'o', b'n'];

#[derive(Decode, Encode, Clone, PartialEq, Eq)]
pub enum ConsensusLog {
	#[codec(index = "1")]
	EndBlock {
		/// Ethereum block hash.
		block_hash: H256,
		/// Transaction hashes of the Ethereum block.
		transaction_hashes: Vec<H256>,
	},
}
