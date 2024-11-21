//! Frontier is currently not compatible with the BABE consensus provider. So we need to manually
//! implement the BABE consensus provider for Frontier. This is a temporary solution until Frontier
//! is compatible with the BABE consensus provider.

#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(clippy::clone_on_copy)]
use fc_rpc::pending::ConsensusDataProvider;
use sc_client_api::backend::{AuxStore, Backend, StorageProvider};
use sc_client_api::UsageProvider;
use sc_service::error::Error as ServiceError;
use sc_service::{
    ChainSpec, Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager,
};
use schnorrkel::PublicKey;
use sp_api::{ApiExt, ApiRef, Core, ProvideRuntimeApi};
use sp_application_crypto::{AppCrypto, ByteArray};
use sp_consensus_babe::digests::PreDigest;
use sp_consensus_babe::inherents::BabeInherentData;
use sp_consensus_babe::{
    make_vrf_transcript, AllowedSlots, AuthorityId, AuthorityPair, BabeApi, BabeConfiguration,
    Epoch, Randomness, Slot, VrfSignature,
};
use sp_core::{Encode, H256};
use sp_inherents::{CreateInherentDataProviders, InherentData, InherentDataProvider};
use sp_keystore::{Keystore, KeystorePtr};
use sp_runtime::generic::{Digest, DigestItem};
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, One};
use sp_runtime::TransactionOutcome;
use std::marker::PhantomData;
use std::sync::Arc;

pub struct BabeConsensusDataProvider<B, C> {
    client: Arc<C>,
    keystore: Arc<dyn Keystore>,
    _phantom: PhantomData<B>,
}
use sp_consensus_babe::SlotDuration;

impl<B, C> BabeConsensusDataProvider<B, C>
where
    B: sp_runtime::traits::Block<Hash = sp_core::H256>,
    C: AuxStore + ProvideRuntimeApi<B> + UsageProvider<B>,
    C::Api: BabeApi<B>,
{
    pub fn new(client: Arc<C>, keystore: Arc<dyn Keystore>) -> Self {
        Self {
            client,
            keystore,
            _phantom: Default::default(),
        }
    }
}

pub fn make_primary_pre_digest(
    authority_index: sp_consensus_babe::AuthorityIndex,
    slot: sp_consensus_babe::Slot,
    vrf_signature: VrfSignature,
) -> Digest {
    let digest_data = sp_consensus_babe::digests::PreDigest::Primary(
        sp_consensus_babe::digests::PrimaryPreDigest {
            authority_index,
            slot,
            vrf_signature,
        },
    );
    let log = DigestItem::PreRuntime(sp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
    Digest { logs: vec![log] }
}

pub fn make_secondary_plain_pre_digest(
    authority_index: sp_consensus_babe::AuthorityIndex,
    slot: sp_consensus_babe::Slot,
) -> Digest {
    let digest_data = sp_consensus_babe::digests::PreDigest::SecondaryPlain(
        sp_consensus_babe::digests::SecondaryPlainPreDigest {
            authority_index,
            slot,
        },
    );
    let log = DigestItem::PreRuntime(sp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
    Digest { logs: vec![log] }
}

pub fn make_secondary_vrf_pre_digest(
    authority_index: sp_consensus_babe::AuthorityIndex,
    slot: sp_consensus_babe::Slot,
    vrf_signature: VrfSignature,
) -> Digest {
    let digest_data = sp_consensus_babe::digests::PreDigest::SecondaryVRF(
        sp_consensus_babe::digests::SecondaryVRFPreDigest {
            authority_index,
            slot,
            vrf_signature,
        },
    );
    let log = DigestItem::PreRuntime(sp_consensus_babe::BABE_ENGINE_ID, digest_data.encode());
    Digest { logs: vec![log] }
}

fn make_vrf_signature(
    randomness: &Randomness,
    slot: Slot,
    epoch: u64,
    key: sp_consensus_babe::AuthorityId,
    keystore: &KeystorePtr,
) -> Option<VrfSignature> {
    let transcript = make_vrf_transcript(randomness, slot, epoch);
    let try_sign = Keystore::sr25519_vrf_sign(
        &**keystore,
        sp_consensus_babe::KEY_TYPE,
        key.as_ref(),
        &transcript.clone().into_sign_data(),
    );
    if let Ok(Some(signature)) = try_sign {
        let public = PublicKey::from_bytes(&key.to_raw_vec()).ok()?;
        if signature
            .output
            .0
            .attach_input_hash(&public, transcript.0.clone())
            .is_err()
        {
            // VRF signature cannot be validated using key and transcript
            return None;
        }
        return Some(signature);
    } else {
        // VRF key not found in keystore or VRF signing failed
        None
    }
}

impl<B: sp_runtime::traits::Block<Hash = sp_core::H256>, C: Send + Sync + ProvideRuntimeApi<B>>
    ConsensusDataProvider<B> for BabeConsensusDataProvider<B, C>
where
    B: sp_runtime::traits::Block<Hash = sp_core::H256>,
    C: sp_api::ProvideRuntimeApi<B>,
    C::Api: BabeApi<B>,
{
    fn create_digest(
        &self,
        parent: &B::Header,
        data: &InherentData,
    ) -> Result<Digest, sp_inherents::Error> {
        let best_block_hash: sp_core::H256 = parent.hash();
        let slot = data
            .babe_inherent_data()
            .expect("Timestamp is always present; qed");
        let runtime_api = self.client.runtime_api();
        let current_epoch: Epoch = runtime_api.current_epoch(best_block_hash).unwrap();
        let allowed_slots = current_epoch.config.allowed_slots.clone();
        let authorities: Vec<_> = current_epoch.authorities.clone();
        let randomness: Randomness = current_epoch.randomness.clone();
        let epoch_index: u64 = current_epoch.epoch_index.clone();
        let public_keys = self
            .keystore
            .sr25519_public_keys(sp_consensus_babe::KEY_TYPE);
        if public_keys.len() > 0 && slot.is_some() {
            let validator_public_key: sp_consensus_babe::AuthorityId = public_keys[0].into();
            let maybe_pos = authorities
                .iter()
                .position(|a| &validator_public_key == &a.0);
            if let Some(authority_index) = maybe_pos {
                match allowed_slots {
                    AllowedSlots::PrimaryAndSecondaryPlainSlots => {
                        return Ok(make_secondary_plain_pre_digest(
                            authority_index as u32,
                            slot.unwrap(),
                        ));
                    }
                    AllowedSlots::PrimaryAndSecondaryVRFSlots => {
                        if let Some(vrf_signature) = make_vrf_signature(
                            &randomness,
                            slot.unwrap(),
                            epoch_index,
                            validator_public_key,
                            &self.keystore,
                        ) {
                            return Ok(make_secondary_vrf_pre_digest(
                                authority_index as u32,
                                slot.unwrap(),
                                vrf_signature,
                            ));
                        }
                    }
                    _ => {
                        if let Some(vrf_signature) = make_vrf_signature(
                            &randomness,
                            slot.unwrap(),
                            epoch_index,
                            validator_public_key,
                            &self.keystore,
                        ) {
                            return Ok(make_primary_pre_digest(
                                authority_index as u32,
                                slot.unwrap(),
                                vrf_signature,
                            ));
                        }
                    }
                }
            }
        }

        Ok(Default::default())
    }
}
