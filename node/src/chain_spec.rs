use cumulus_primitives_core::ParaId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde_json::json;
use serde::{Deserialize, Serialize};
use sp_core::{Pair, Public, sr25519, U256};
use clover_runtime::{
  AccountId, Balance, BalancesConfig, ContractsConfig, IndicesConfig, GenesisConfig,
  SudoConfig, SystemConfig, WASM_BINARY,
  Signature,
  EVMConfig, EthereumConfig, DOLLARS
};
use sp_runtime::{traits::{IdentifyAccount, Verify}, };
use sc_service::ChainType;
use hex_literal::hex;
use std::collections::BTreeMap;
use pallet_evm::GenesisAccount;
use primitive_types::H160;
use std::str::FromStr;

// The URL for the telemetry server.
// const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

// fn session_keys(
// ) -> SessionKeys {
//   SessionKeys { }
// }

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
  TPublic::Pair::from_string(&format!("//{}", seed), None)
    .expect("static values are valid; qed")
    .public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
  AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
  AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Babe authority key.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId) {
  (
    get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
    get_account_id_from_seed::<sr25519::Public>(s),
  )
}

fn endowed_evm_account() -> BTreeMap<H160, GenesisAccount>{
  let endowed_account = vec![
    // clover fauct
    H160::from_str("9157f0316f375e4ccf67f8d21ae0881d0abcbb21").unwrap()
  ];
  get_endowed_evm_accounts(endowed_account)
}

fn dev_endowed_evm_accounts() -> BTreeMap<H160, GenesisAccount>{
  let endowed_account = vec![
    H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b").unwrap(),
    H160::from_str("e6206C7f064c7d77C6d8e3eD8601c9AA435419cE").unwrap(),
    // the dev account key
    // seed: bottom drive obey lake curtain smoke basket hold race lonely fit walk
    // private key: 0x03183f27e9d78698a05c24eb6732630eb17725fcf2b53ee3a6a635d6ff139680
    H160::from_str("aed40f2261ba43b4dffe484265ce82d8ffe2b4db").unwrap()
  ];

  get_endowed_evm_accounts(endowed_account)
}

fn get_endowed_evm_accounts(endowed_account: Vec<H160>) -> BTreeMap<H160, GenesisAccount>{
  let mut evm_accounts = BTreeMap::new();
  for account in endowed_account {
    evm_accounts.insert(
      account,
      GenesisAccount {
        nonce: U256::from(0),
        balance: U256::from(1_000 * DOLLARS),
        storage: Default::default(),
        code: vec![],
      },
    );
  }
  evm_accounts
}

pub fn development_config(id: ParaId) -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Development",
    // ID
    "dev",
    ChainType::Development,
    move || testnet_genesis(
      wasm_binary,
      // Initial PoA authorities
      vec![
        authority_keys_from_seed("Alice"),
      ],
      // Sudo account
      get_account_id_from_seed::<sr25519::Public>("Alice"),
      // Pre-funded accounts
      vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
      ],
      true,
      dev_endowed_evm_accounts(),
      id
      ),
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    // Properties
    Some(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    Extensions {
      relay_chain: "westend-dev".into(),
      para_id: id.into(),
    },
  ))
}

pub fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Clover",
    // ID
    "local_testnet",
    ChainType::Local,
    move || testnet_genesis(
      wasm_binary,
      // Initial PoA authorities
      vec![
        authority_keys_from_seed("Alice"),
        authority_keys_from_seed("Bob"),
      ],
      // Sudo account
      get_account_id_from_seed::<sr25519::Public>("Alice"),
      // Pre-funded accounts
      vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
        //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        //get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
      ],
      true,
      endowed_evm_account(),
      id,
    ),
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    // Properties
    Some(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    Extensions {
      relay_chain: "westend-dev".into(),
      para_id: id.into(),
    },
  ))
}

pub fn sakura_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Skarua",
    // ID
    "skarua",
    ChainType::Custom(String::from("skarua")),
    move || testnet_genesis(
      wasm_binary,
      // Initial PoA authorities
      vec![
        // SECRET="..."
        // 5CqWfdrRGdZe6bwxZMiHfdcNAVePjkUJpSh2rpKgcNWciTFP
        // subkey inspect "$SECRET//clover//1//validator"
        // subkey inspect "$SECRET//clover//1//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//1//grandpa"
        // subkey inspect "$SECRET//clover//1//imonline"
        // subkey inspect "$SECRET//clover//1//discovery"
        (
          hex!["222c5fa244583b1734ceb6515916efc5e103f65b869ebec4e56b989d9dbb446e"].into(),
          hex!["222c5fa244583b1734ceb6515916efc5e103f65b869ebec4e56b989d9dbb446e"].into(),
        ),
        // 5FNQoCoibJMAyqC77og9tSbhGUtaVt51SD7GdCxmMeWxPBvX
        // subkey inspect "$SECRET//clover//2//validator"5FNQoCoibJMAyqC77og9tSbhGUtaVt51SD7GdCxmMeWxPBvX
        // subkey inspect "$SECRET//clover//2//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//2//grandpa"
        // subkey inspect "$SECRET//clover//2//imonline"
        // subkey inspect "$SECRET//clover//2//discovery"
        (
          hex!["9235b080b6ca2e7b2a7af7a46ac4f677bfa394e29d83611324046c38eb14ee49"].into(),
          hex!["9235b080b6ca2e7b2a7af7a46ac4f677bfa394e29d83611324046c38eb14ee49"].into(),
        ),
        // 5HQDFanwYwt3QtkAvaBHbaaLgSRER42PWAXCJqNoxyQFZXZJ
        // subkey inspect "$SECRET//clover//3//validator"
        // subkey inspect "$SECRET//clover//3//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//3//grandpa"
        // subkey inspect "$SECRET//clover//3//imonline"
        // subkey inspect "$SECRET//clover//3//discovery"
        (
          hex!["ec0dc859299bcc7146d9ba74956ff67334454e23c0d9fd3e55302f94b09a742b"].into(),
          hex!["ec0dc859299bcc7146d9ba74956ff67334454e23c0d9fd3e55302f94b09a742b"].into(),
        ),
      ],
      // 5CPQQYs3wf32fr5PhmmfFQEeVzD1Zy9Hdo8LFzQYuhP8XHW6
      // subkey inspect "$SECRET//clover//root"
      hex!["0e42eb6f65a8ef5e3f3c3cdb5b2c3be646e791abd76e2224d5847cde786b2e01"].into(),
      // Pre-funded accounts
      vec![
        // 5CPQQYs3wf32fr5PhmmfFQEeVzD1Zy9Hdo8LFzQYuhP8XHW6
        hex!["0e42eb6f65a8ef5e3f3c3cdb5b2c3be646e791abd76e2224d5847cde786b2e01"].into(),
      ],
      true,
      endowed_evm_account(),
      id,
    ),
    // Bootnodes
    vec![
      "/dns/seed1.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWFtshqoFL1hAwseGc4WuFeREKicjFR15JiVEaJiHnDvn2"
        .parse()
        .unwrap(),
      "/dns/seed2.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWBcU1EShS2azLwQhKVKyeXU2cc3CWyhuN8wJwEKaRVNe8"
        .parse()
        .unwrap(),
    ],
    // Telemetry
    None,
    // Protocol ID
    Some("skarua"),
    // Properties
    Some(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "SKU"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    Extensions {
			relay_chain: "westend-dev".into(),
			para_id: id.into(),
		},
  ))
}


/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
  wasm_binary: &[u8],
  initial_authorities: Vec<(AccountId, AccountId, )>,
  root_key: AccountId,
  endowed_accounts: Vec<AccountId>,
  _enable_println: bool,
  endowed_eth_accounts: BTreeMap<H160, GenesisAccount>,
  id: ParaId,
) -> GenesisConfig {
  let enable_println = true;

  const ENDOWMENT: Balance = 1_000 * DOLLARS;
  // const STASH: Balance = 100 * DOLLARS;
  const AUTHOR_BALANCE: Balance = 200 * DOLLARS;

  GenesisConfig {
    frame_system: SystemConfig {
      // Add Wasm runtime to storage.
      code: wasm_binary.to_vec(),
      changes_trie_config: Default::default(),
    },
    pallet_balances: BalancesConfig {
      // Configure endowed accounts with initial balance of 1 << 60.
      balances: endowed_accounts.iter().cloned()
            .map(|k| (k, ENDOWMENT))
            .chain(initial_authorities.iter().map(|x| (x.0.clone(), AUTHOR_BALANCE)))
            .collect(),
    },
    pallet_contracts: ContractsConfig {
      current_schedule: pallet_contracts::Schedule::default()
      .enable_println(enable_println),
    },
    pallet_evm: EVMConfig {
      accounts: endowed_eth_accounts,
    },
    pallet_ethereum: EthereumConfig {},
    pallet_indices: IndicesConfig {
      indices: vec![],
    },
    pallet_sudo: SudoConfig {
      // Assign network admin rights.
      key: root_key,
    },
    parachain_info: clover_runtime::ParachainInfoConfig { parachain_id: id },
    pallet_collective_Instance1: Default::default(),
    pallet_collective_Instance2: Default::default(),
    pallet_democracy: Default::default(),
    pallet_treasury: Default::default(),
    pallet_elections_phragmen: Default::default(),
    pallet_membership_Instance1: Default::default(),
    pallet_vesting: Default::default(),
  }
}
