// Copyright (C) 2021 Clover Network
// This file is part of Clover.

/// An Ethereum address (i.e. 20 bytes, used to represent an Ethereum account).
///
/// This gets serialized to the 0x-prefixed hex representation.
use codec::{Encode, Decode};
use sp_runtime::{
  RuntimeDebug,
};
#[cfg(feature = "std")]
use serde::{self, Serialize, Deserialize, Serializer, Deserializer};

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug)]
pub struct EthereumAddress(pub [u8; 20]);

#[cfg(feature = "std")]
impl Serialize for EthereumAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for EthereumAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let base_string = String::deserialize(deserializer)?;
        let offset = if base_string.starts_with("0x") { 2 } else { 0 };
        let s = &base_string[offset..];
        if s.len() != 40 {
            Err(serde::de::Error::custom("Bad length of Ethereum address (should be 42 including '0x')"))?;
        }
        let raw: Vec<u8> = rustc_hex::FromHex::from_hex(s)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        let mut r = Self::default();
        r.0.copy_from_slice(&raw);
        Ok(r)
    }
}

/// An Ethereum signature
#[derive(Encode, Decode, Clone)]
pub struct EcdsaSignature(pub [u8; 65]);

impl PartialEq for EcdsaSignature {
    fn eq(&self, other: &Self) -> bool {
        &self.0[..] == &other.0[..]
    }
}

impl sp_std::fmt::Debug for EcdsaSignature {
    fn fmt(&self, f: &mut sp_std::fmt::Formatter<'_>) -> sp_std::fmt::Result {
        write!(f, "EcdsaSignature({:?})", &self.0[..])
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug)]
pub struct EthereumTxHash(pub [u8; 32]);

#[cfg(feature = "std")]
impl Serialize for EthereumTxHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

/// Converts the given binary data into ASCII-encoded hex. It will be twice the length.
pub fn to_ascii_hex(data: &[u8]) -> Vec<u8> {
  let mut r = Vec::with_capacity(data.len() * 2);
  let mut push_nibble = |n| r.push(if n < 10 { b'0' + n } else { b'a' - 10 + n });
  for &b in data.iter() {
      push_nibble(b / 16);
      push_nibble(b % 16);
  }
  r
}
