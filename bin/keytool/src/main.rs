use structopt::StructOpt;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use bip39::{Language, Mnemonic, MnemonicType, Seed};
use tiny_hderive::bip32::ExtendedPrivKey;
use sp_core::{H256, H160, crypto::{Ss58Codec, Ss58AddressFormat}};
use sha3::{Keccak256, Digest};
use sp_runtime::{MultiSigner, traits::IdentifyAccount};

#[derive(Debug, StructOpt)]
pub struct AccountBinding {
    /// Specify the 12 words seed phrase
    #[structopt(long, short = "s")]
    seeds: Option<String>,
}

impl AccountBinding {
    pub fn generate(&self) {
        let seeds = match &self.seeds {
            Some(words) => Mnemonic::from_phrase(&words, Language::English).unwrap(),
            _ => Mnemonic::new(MnemonicType::Words12, Language::English)
        };
        let secp = Secp256k1::new();
        // use the default derivation path. example: https://docs.rs/tiny-hderive/latest/tiny_hderive/
        let ext = ExtendedPrivKey::derive(Seed::new(&seeds, "").as_bytes(), "m/44'/60'/0'/0/0").unwrap();
        let evm_private_key = SecretKey::from_slice(&ext.secret()).unwrap();
        let evm_public_key = PublicKey::from_secret_key(&secp, &evm_private_key);

        let mut pubkey = [0u8; 64];
        let serialized = &evm_public_key.serialize_uncompressed();
        // need to drop the first byte
        pubkey.copy_from_slice(&serialized[1..65]);
        let account = H160::from(H256::from_slice(Keccak256::digest(&pubkey).as_slice()));

        // by default, clover use sr25519 for account address
        let clover_address = generate_clover_address::<sp_core::sr25519::Pair>(seeds.phrase());

        println!("Seed phrase:      {}", seeds.phrase());
        println!("Clover address:   {}", clover_address);
        println!("EVM address:      0x{:x}", account);
        println!("EVM private key:  0x{:x}", evm_private_key);
    }
}

fn generate_clover_address<Pair>(seed: &str) -> String
    where Pair: sp_core::Pair, Pair::Public: Into<MultiSigner> {
    let (pair, _) = Pair::from_phrase(seed, None).unwrap();
    let account = pair.public().into().into_account();
    account.to_ss58check_with_version(Ss58AddressFormat::SubstrateAccount)
}

fn main() {
    let cmd = AccountBinding::from_args();
    cmd.generate();
}
