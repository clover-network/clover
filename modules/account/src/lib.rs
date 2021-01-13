#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, transactional,
    decl_storage, dispatch::DispatchResult, debug
};
use frame_system::ensure_signed;
use sp_core::{H160, H256};
use ethereum::TransactionSignature;
use sha3::{Digest, Keccak256};

#[cfg(test)]
mod tests;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Account {
		pub ClvToEth get(fn clv_to_eth): map hasher(blake2_128_concat) T::AccountId => H160;
        pub EthToClv get(fn eth_to_clv): map hasher(blake2_128_concat) H160 => T::AccountId;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId {
		ClaimAccount(AccountId, H160),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait> {
		AccountIdHasMapped,
		EthAddressHasMapped,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;
		fn deposit_event() = default;

		#[weight = 10_000]
		#[transactional]
		fn bind(origin, eth_address: H160, signature: TransactionSignature) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			debug::info!("from: {:?}, to: {:?}, input: {:?}", sender, eth_address, signature);
			let decoded_eth_address = Self::recover_signer(&signature, &sender);
			debug::info!("decoded: {:?}", decoded_eth_address);
			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
    fn recover_signer(signature: &TransactionSignature, data: &T::AccountId) -> Option<H160> {
        let mut sig = [0u8; 65];
        let mut msg = [0u8; 32];
        sig[0..32].copy_from_slice(&signature.r()[..]);
        sig[32..64].copy_from_slice(&signature.s()[..]);
        sig[64] = signature.standard_v();
        msg.copy_from_slice("123".as_ref());
        let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg).ok()?;
        Some(H160::from(H256::from_slice(Keccak256::digest(&pubkey).as_slice())))
    }
}