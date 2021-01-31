

// --- crates ---
use codec::{Decode, Encode};
// --- github ---
use ethbloom::{Bloom, Input as BloomInput};
use primitive_types::{H256, U256};
use rlp::*;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[cfg(any(feature = "deserialize", test))]
use crate::header::bytes_from_string;
use crate::{error::EthereumError, *};
use merkle_patricia_trie::{trie::Trie, MerklePatriciaTrie, Proof};
use log::LogEntry;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum TransactionOutcome {
    /// Status and state root are unknown under EIP-98 rules.
    Unknown,
    /// State root is known. Pre EIP-98 and EIP-658 rules.
    StateRoot(H256),
    /// Status code is known. EIP-658 rules.
    StatusCode(u8),
}


#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthereumReceipt {
    /// The total gas used in the block following execution of the transaction.
    pub gas_used: U256,
    /// The OR-wide combination of all logs' blooms for this transaction.
    pub log_bloom: Bloom,
    /// The logs stemming from this transaction.
    pub logs: Vec<LogEntry>,
    /// Transaction outcome.
    pub outcome: TransactionOutcome,
}

#[cfg_attr(any(feature = "deserialize", test), derive(serde::Deserialize))]
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct EthereumReceiptProof {
    pub index: u64,
    #[cfg_attr(
    any(feature = "deserialize", test),
    serde(deserialize_with = "bytes_from_string")
    )]
    pub proof: Vec<u8>,
    pub header_hash: H256,
}

impl EthereumReceipt {
    /// Create a new receipt.
    pub fn new(outcome: TransactionOutcome, gas_used: U256, logs: Vec<LogEntry>) -> Self {
        Self {
            gas_used,
            log_bloom: logs.iter().fold(Bloom::default(), |mut b, l| {
                b.accrue_bloom(&l.bloom());
                b
            }),
            logs,
            outcome,
        }
    }

    pub fn verify_proof_and_generate(
        receipt_root: &H256,
        proof_record: &EthereumReceiptProof,
    ) -> Result<Self, EthereumError> {
        let proof: Proof =
            rlp::decode(&proof_record.proof).map_err(|_| EthereumError::InvalidReceiptProof)?;
        let key = rlp::encode(&proof_record.index);

        let value = MerklePatriciaTrie::verify_proof(receipt_root.0.to_vec(), &key, proof)
            .map_err(|_| EthereumError::InvalidReceiptProof)?
            .ok_or(EthereumError::InvalidReceiptProof)?;
        let receipt = rlp::decode(&value).map_err(|_| EthereumError::InvalidReceiptProof)?;

        Ok(receipt)
    }
}

impl Encodable for EthereumReceipt {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self.outcome {
            TransactionOutcome::Unknown => {
                s.begin_list(3);
            }
            TransactionOutcome::StateRoot(ref root) => {
                s.begin_list(4);
                s.append(root);
            }
            TransactionOutcome::StatusCode(ref status_code) => {
                s.begin_list(4);
                s.append(status_code);
            }
        }
        s.append(&self.gas_used);
        s.append(&self.log_bloom);
        s.append_list(&self.logs);
    }
}

impl Decodable for EthereumReceipt {
    fn decode(rlp: &Rlp) -> Result<Self, DecoderError> {
        if rlp.item_count()? == 3 {
            Ok(EthereumReceipt {
                outcome: TransactionOutcome::Unknown,
                gas_used: rlp.val_at(0)?,
                log_bloom: rlp.val_at(1)?,
                logs: rlp.list_at(2)?,
            })
        } else {
            Ok(EthereumReceipt {
                gas_used: rlp.val_at(1)?,
                log_bloom: rlp.val_at(2)?,
                logs: rlp.list_at(3)?,
                outcome: {
                    let first = rlp.at(0)?;
                    if first.is_data() && first.data()?.len() <= 1 {
                        TransactionOutcome::StatusCode(first.as_val()?)
                    } else {
                        TransactionOutcome::StateRoot(first.as_val()?)
                    }
                },
            })
        }
    }
}

pub type EthereumTransactionIndex = (H256, u64);

#[cfg(test)]
mod tests {
    // --- std ---
    use std::str::FromStr;
    // --- crates ---
    use keccak_hasher::KeccakHasher;
    use super::*;
    use array_bytes::{fixed_hex_bytes_unchecked, hex_bytes_unchecked};

    #[inline]
    fn construct_receipts(
        root: Option<H256>,
        gas_used: U256,
        status: Option<u8>,
        log_entries: Vec<LogEntry>,
    ) -> EthereumReceipt {
        EthereumReceipt::new(
            if root.is_some() {
                TransactionOutcome::StateRoot(root.unwrap())
            } else {
                TransactionOutcome::StatusCode(status.unwrap())
            },
            gas_used,
            log_entries,
        )
    }

    /// ropsten tx hash: 0xce62c3d1d2a43cfcc39707b98de53e61a7ef7b7f8853e943d85e511b3451aa7e
    #[test]
    fn test_basic() {
        // https://ropsten.etherscan.io/tx/0xce62c3d1d2a43cfcc39707b98de53e61a7ef7b7f8853e943d85e511b3451aa7e#eventlog
        let log_entries = vec![LogEntry {
            address: EthereumAddress::from_str("ad52e0f67b6f44cd5b9a6f4fbc7c0f78f37e094b").unwrap(),
            topics: vec![
                fixed_hex_bytes_unchecked!(
					"0x6775ce244ff81f0a82f87d6fd2cf885affb38416e3a04355f713c6f008dd126a",
					32
				)
                    .into(),
                fixed_hex_bytes_unchecked!(
					"0x0000000000000000000000000000000000000000000000000000000000000006",
					32
				)
                    .into(),
                fixed_hex_bytes_unchecked!(
					"0x0000000000000000000000000000000000000000000000000000000000000000",
					32
				)
                    .into(),
            ],
            data: hex_bytes_unchecked("0x00000000000000000000000074241db5f3ebaeecf9506e4ae9881860933416048eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48000000000000000000000000000000000000000000000000002386f26fc10000"),
        }];

        let r = construct_receipts(None, 1123401.into(), Some(1), log_entries);
        // TODO: Check the log bloom generation logic
        assert_eq!(r.log_bloom, Bloom::from_str(
            "00000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000820000000000000020000000000000000000800000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000200000000020000000000000000000000000000080000000000000800000000000000000000000"
        ).unwrap());
    }

    /// kovan tx hash: 0xaaf52845694258509cbdd30ea21894b4e685eb4cdbb13dd298f925fe6e51db35
    /// block number: 13376543 (only a tx in this block, which is above)
    /// from: 0x4aea6cfc5bd14f2308954d544e1dc905268357db
    /// to: 0xa24df0420de1f3b8d740a52aaeb9d55d6d64478e (a contract)
    /// receipts_root in block#13376543: 0xc789eb8b7f5876f4df4f8ae16f95c9881eabfb700ee7d8a00a51fb4a71afbac9
    /// to check if receipts_root in block-header can be pre-computed.
    #[test]
    fn check_receipts() {
        let expected_root = fixed_hex_bytes_unchecked!(
			"0xc789eb8b7f5876f4df4f8ae16f95c9881eabfb700ee7d8a00a51fb4a71afbac9",
			32
		)
            .into();

        let log_entries = vec![LogEntry {
            address: EthereumAddress::from_str("a24df0420de1f3b8d740a52aaeb9d55d6d64478e").unwrap(),
            topics: vec![fixed_hex_bytes_unchecked!(
				"0xf36406321d51f9ba55d04e900c1d56caac28601524e09d53e9010e03f83d7d00",
				32
			)
                .into()],
            data: hex_bytes_unchecked("0x0000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000363384a3868b9000000000000000000000000000000000000000000000000000000005d75f54f0000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000e53504f5450582f4241542d455448000000000000000000000000000000000000"),
        }];
        let receipts = vec![EthereumReceipt::new(
            TransactionOutcome::StatusCode(1),
            73705.into(),
            log_entries,
        )];

        let receipts_root: H256 = H256(triehash::ordered_trie_root::<KeccakHasher, _>(
            receipts.iter().map(|x| ::rlp::encode(x)),
        ));

        assert_eq!(receipts_root, expected_root);
    }
}
