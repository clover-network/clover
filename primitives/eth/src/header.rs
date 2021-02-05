#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use ethbloom::Bloom;
use keccak_hash::{keccak, KECCAK_EMPTY_LIST_RLP, KECCAK_NULL_RLP};
pub use primitive_types::{H160, H256, U128, U256, U512};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use rlp_derive::{RlpEncodable, RlpDecodable};

#[cfg(any(feature = "deserialize", test))]
use serde::Deserialize;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
// --- substrate ---
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

use crate::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
enum Seal {
    /// The seal/signature is included.
    With,
    /// The seal/signature is not included.
    Without,
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthereumHeader {
    pub parent_hash: H256,
    pub timestamp: u64,
    pub number: EthereumBlockNumber,
    pub author: EthereumAddress,
    pub transactions_root: H256,
    pub uncles_hash: H256,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "bytes_from_string")
    )]
    pub extra_data: Bytes,
    pub state_root: H256,
    pub receipts_root: H256,
    pub log_bloom: Bloom,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "u256_from_u64")
    )]
    pub gas_used: U256,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "u256_from_u64")
    )]
    pub gas_limit: U256,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "u256_from_u64")
    )]
    pub difficulty: U256,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "bytes_array_from_string")
    )]
    pub seal: Vec<Bytes>,
    pub hash: Option<H256>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderStr(Vec<u8>);

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct HeaderCandidate {
    hash: H256,
    parent_hash: H256,
    total_difficulty: U256
}

impl HeaderStr {
    pub fn new(slice: Vec<u8>) -> Self {
        HeaderStr(slice)
    }
}

impl EthereumHeader {
    #[cfg(any(feature = "deserialize", test))]
    pub fn from_scale_codec_str<S: AsRef<str>>(s: S) -> Option<Self> {
        if let Ok(eth_header) = <Self as Decode>::decode(&mut &hex_bytes_unchecked(s.as_ref())[..])
        {
            Some(eth_header)
        } else {
            None
        }
    }
}

impl Default for EthereumHeader {
    fn default() -> Self {
        EthereumHeader {
            parent_hash: H256::zero(),
            timestamp: 0,
            number: 0,
            author: EthereumAddress::zero(),
            transactions_root: KECCAK_NULL_RLP,
            uncles_hash: KECCAK_EMPTY_LIST_RLP,
            extra_data: vec![],
            state_root: KECCAK_NULL_RLP,
            receipts_root: KECCAK_NULL_RLP,
            log_bloom: Bloom::default(),
            gas_used: U256::default(),
            gas_limit: U256::default(),
            difficulty: U256::default(),
            seal: vec![],
            hash: None,
        }
    }
}

impl PartialEq for EthereumHeader {
    fn eq(&self, c: &EthereumHeader) -> bool {
        if let (&Some(ref h1), &Some(ref h2)) = (&self.hash, &c.hash) {
            // More strict check even if hashes equal since EthereumHeader could be decoded from dispatch call by external
            // Note that this is different implementation compared to Open Ethereum
            // Refer: https://github.com/openethereum/openethereum/blob/v3.0.0-alpha.1/ethcore/types/src/header.rs#L93
            if h1 != h2 {
                return false;
            }
        }

        self.parent_hash == c.parent_hash
            && self.timestamp == c.timestamp
            && self.number == c.number
            && self.author == c.author
            && self.transactions_root == c.transactions_root
            && self.uncles_hash == c.uncles_hash
            && self.extra_data == c.extra_data
            && self.state_root == c.state_root
            && self.receipts_root == c.receipts_root
            && self.log_bloom == c.log_bloom
            && self.gas_used == c.gas_used
            && self.gas_limit == c.gas_limit
            && self.difficulty == c.difficulty
            && self.seal == c.seal
    }
}

impl Decodable for EthereumHeader {
    fn decode(r: &Rlp) -> Result<Self, DecoderError> {
        let mut blockheader = EthereumHeader {
            parent_hash: r.val_at(0)?,
            uncles_hash: r.val_at(1)?,
            author: r.val_at(2)?,
            state_root: r.val_at(3)?,
            transactions_root: r.val_at(4)?,
            receipts_root: r.val_at(5)?,
            log_bloom: r.val_at(6)?,
            difficulty: r.val_at(7)?,
            number: r.val_at(8)?,
            gas_limit: r.val_at(9)?,
            gas_used: r.val_at(10)?,
            timestamp: r.val_at(11)?,
            extra_data: r.val_at(12)?,
            seal: vec![],
            hash: keccak(r.as_raw()).into(),
        };

        for i in 13..r.item_count()? {
            blockheader.seal.push(r.at(i)?.as_raw().to_vec())
        }

        Ok(blockheader)
    }
}

impl Encodable for EthereumHeader {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.stream_rlp(s, Seal::With);
    }
}

/// Alter value of given field, reset memoised hash if changed.
fn change_field<T>(hash: &mut Option<H256>, field: &mut T, value: T)
    where
        T: PartialEq<T>,
{
    if field != &value {
        *field = value;
        *hash = None;
    }
}

impl EthereumHeader {
    /// Create a new, default-valued, header.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the parent_hash field of the header.
    pub fn parent_hash(&self) -> &H256 {
        &self.parent_hash
    }

    /// Get the timestamp field of the header.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Get the number field of the header.
    pub fn number(&self) -> EthereumBlockNumber {
        self.number
    }

    /// Get the author field of the header.
    pub fn author(&self) -> &EthereumAddress {
        &self.author
    }

    /// Get the extra data field of the header.
    pub fn extra_data(&self) -> &Bytes {
        &self.extra_data
    }

    /// Get the state root field of the header.
    pub fn state_root(&self) -> &H256 {
        &self.state_root
    }

    /// Get the receipts root field of the header.
    pub fn receipts_root(&self) -> &H256 {
        &self.receipts_root
    }

    /// Get the log bloom field of the header.
    pub fn log_bloom(&self) -> &Bloom {
        &self.log_bloom
    }

    /// Get the transactions root field of the header.
    pub fn transactions_root(&self) -> &H256 {
        &self.transactions_root
    }

    /// Get the uncles hash field of the header.
    pub fn uncles_hash(&self) -> &H256 {
        &self.uncles_hash
    }

    /// Get the gas used field of the header.
    pub fn gas_used(&self) -> &U256 {
        &self.gas_used
    }

    /// Get the gas limit field of the header.
    pub fn gas_limit(&self) -> &U256 {
        &self.gas_limit
    }

    /// Get the difficulty field of the header.
    pub fn difficulty(&self) -> &U256 {
        &self.difficulty
    }

    /// Get the seal field of the header.
    pub fn seal(&self) -> &[Bytes] {
        &self.seal
    }

    /// Set the seal field of the header.
    pub fn set_seal(&mut self, a: Vec<Bytes>) {
        change_field(&mut self.hash, &mut self.seal, a)
    }

    /// Set the difficulty field of the header.
    pub fn set_difficulty(&mut self, a: U256) {
        change_field(&mut self.hash, &mut self.difficulty, a);
    }

    /// Get & memoize the hash of this header (keccak of the RLP with seal).
    pub fn compute_hash(&mut self) -> H256 {
        let hash = self.hash();
        self.hash = Some(hash);
        hash
    }

    pub fn re_compute_hash(&self) -> H256 {
        keccak_hash::keccak(self.rlp(Seal::With))
    }

    /// Get the hash of this header (keccak of the RLP with seal).
    pub fn hash(&self) -> H256 {
        self.hash
            .unwrap_or_else(|| keccak_hash::keccak(self.rlp(Seal::With)))
    }

    /// Get the hash of the header excluding the seal
    pub fn bare_hash(&self) -> H256 {
        keccak_hash::keccak(self.rlp(Seal::Without))
    }

    /// Encode the header, getting a type-safe wrapper around the RLP.
    pub fn encoded(&self) -> HeaderStr {
        HeaderStr::new(self.rlp(Seal::With))
    }

    /// Get the RLP representation of this Header.
    fn rlp(&self, with_seal: Seal) -> Bytes {
        let mut s = RlpStream::new();
        self.stream_rlp(&mut s, with_seal);
        s.out()
    }

    /// Place this header into an RLP stream `s`, optionally `with_seal`.
    fn stream_rlp(&self, s: &mut RlpStream, with_seal: Seal) {
        if let Seal::With = with_seal {
            s.begin_list(13 + self.seal.len());
        } else {
            s.begin_list(13);
        }

        s.append(&self.parent_hash);
        s.append(&self.uncles_hash);
        s.append(&self.author);
        s.append(&self.state_root);
        s.append(&self.transactions_root);
        s.append(&self.receipts_root);
        s.append(&self.log_bloom);
        s.append(&self.difficulty);
        s.append(&self.number);
        s.append(&self.gas_limit);
        s.append(&self.gas_used);
        s.append(&self.timestamp);
        s.append(&self.extra_data);

        if let Seal::With = with_seal {
            for b in &self.seal {
                s.append_raw(b, 1);
            }
        }
    }
}

#[cfg(any(feature = "deserialize", test))]
pub fn str_to_u64(s: &str) -> u64 {
    if s.starts_with("0x") {
        u64::from_str_radix(&s[2..], 16).unwrap_or_default()
    } else {
        s.parse().unwrap_or_default()
    }
}

#[cfg(any(feature = "deserialize", test))]
pub fn bytes_from_string<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    Ok(hex_bytes_unchecked(&String::deserialize(deserializer)?))
}

#[cfg(any(feature = "deserialize", test))]
fn u256_from_u64<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    Ok(u64::deserialize(deserializer)?.into())
}

#[cfg(any(feature = "deserialize", test))]
fn bytes_array_from_string<'de, D>(deserializer: D) -> Result<Vec<Bytes>, D::Error>
    where
        D: serde::Deserializer<'de>,
{
    Ok(<Vec<String>>::deserialize(deserializer)?
        .into_iter()
        .map(|s| hex_bytes_unchecked(&s))
        .collect())
}
