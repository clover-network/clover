
use crate::*;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use ethereum_types::{H160, H256};
use rlp_derive::{RlpDecodable, RlpEncodable};
use sp_runtime::RuntimeDebug;

#[derive(Clone, Encode, Decode, RuntimeDebug, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct LogEntry {
    pub address: EthereumAddress,
    pub topics: Vec<H256>,
    pub data: Bytes,
}

impl LogEntry {
    /// Calculates the bloom of this log entry.
    pub fn bloom(&self) -> Bloom {
        self.topics.iter().fold(
            Bloom::from(BloomInput::Raw(self.address.as_bytes())),
            |mut b, t| {
                b.accrue(BloomInput::Raw(t.as_bytes()));
                b
            },
        )
    }
}