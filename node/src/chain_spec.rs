use clover_runtime::{
  AccountId, AuraId, Balance, BalancesConfig, EVMConfig, EthereumConfig, GenesisConfig,
  IndicesConfig, SessionKeys, Signature, /*SudoConfig, */SystemConfig, DOLLARS, WASM_BINARY,
};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use pallet_evm::GenesisAccount;
use primitive_types::H160;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};

use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public, U256};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::collections::BTreeMap;
use std::str::FromStr;

// The URL for the telemetry server.
// const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

fn session_keys(aura_id: AuraId) -> SessionKeys {
  SessionKeys { aura: aura_id }
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
  TPublic::Pair::from_string(&format!("//{}", seed), None)
    .expect("static values are valid; qed")
    .public()
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
//pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
//  get_from_seed::<AuraId>(seed)
//}

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
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
  AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
  AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate authority key.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId) {
  (
    get_account_id_from_seed::<sr25519::Public>(s),
    get_from_seed::<AuraId>(s),
  )
}

fn endowed_evm_account() -> BTreeMap<H160, GenesisAccount> {
  let endowed_account = vec![
    // clover fauct
    H160::from_str("9157f0316f375e4ccf67f8d21ae0881d0abcbb21").unwrap(),
  ];
  get_endowed_evm_accounts(endowed_account)
}

fn dev_endowed_evm_accounts() -> BTreeMap<H160, GenesisAccount> {
  let endowed_account = vec![
    H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b").unwrap(),
    H160::from_str("e6206C7f064c7d77C6d8e3eD8601c9AA435419cE").unwrap(),
    // the dev account key
    // seed: bottom drive obey lake curtain smoke basket hold race lonely fit walk
    // private key: 0x03183f27e9d78698a05c24eb6732630eb17725fcf2b53ee3a6a635d6ff139680
    H160::from_str("aed40f2261ba43b4dffe484265ce82d8ffe2b4db").unwrap(),
  ];

  get_endowed_evm_accounts(endowed_account)
}

fn get_endowed_evm_accounts(endowed_account: Vec<H160>) -> BTreeMap<H160, GenesisAccount> {
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
    move || {
      testnet_genesis(
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
          //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
          get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        ],
        true,
        dev_endowed_evm_accounts(),
        id,
      )
    },
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    None,
    // Properties
    None,
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
    move || {
      testnet_genesis(
        wasm_binary,
        // Initial PoA authorities
        vec![
          authority_keys_from_seed("Alice"),
          authority_keys_from_seed("Bob"),
          authority_keys_from_seed("Charlie"),
          authority_keys_from_seed("Dave"),
        ],
        // Sudo account
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        vec![
          // get_account_id_from_seed::<sr25519::Public>("Alice"),
          // get_account_id_from_seed::<sr25519::Public>("Bob"),
          // get_account_id_from_seed::<sr25519::Public>("Charlie"),
          // get_account_id_from_seed::<sr25519::Public>("Dave"),
          get_account_id_from_seed::<sr25519::Public>("Eve"),
          get_account_id_from_seed::<sr25519::Public>("Ferdie"),
          //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
          //get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
          //get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
          //get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
          get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
          get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
        ],
        true,
        endowed_evm_account(),
        id,
      )
    },
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    None,
    // Properties
    None,
    // Extensions
    Extensions {
      relay_chain: "westend-dev".into(),
      para_id: id.into(),
    },
  ))
}

pub fn clover_mainnet_config(id: ParaId) -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Clover",
    // ID
    "clover",
    ChainType::Custom(String::from("clover")),
    move || {
      testnet_genesis(
        wasm_binary,
        // Initial PoA authorities
        vec![
          (
            // subkey inspect "$SECRET//clover//1//aura"
            hex!("a4b3d4f4394427cce13a5b294ac38e667d55dbac58ffc4d443e9bd198c6b342b").into(),
            hex!("a4b3d4f4394427cce13a5b294ac38e667d55dbac58ffc4d443e9bd198c6b342b")
              .unchecked_into(),
          ),
          (
            // subkey inspect "$SECRET//clover//2//aura"
            hex!("564c0e297ca1e490d7101a321383e4f1556a3d07aa14f5a2f50925693bb3a351").into(),
            hex!("564c0e297ca1e490d7101a321383e4f1556a3d07aa14f5a2f50925693bb3a351")
              .unchecked_into(),
          ),
          (
            // subkey inspect "$SECRET//clover//3//aura"
            hex!("06f7a3ade48d14be541dfe94ca64ec479350126e0bf7050b3da1ce4734ea6a7e").into(),
            hex!("06f7a3ade48d14be541dfe94ca64ec479350126e0bf7050b3da1ce4734ea6a7e")
              .unchecked_into(),
          ),
          (
            // subkey inspect "$SECRET//clover//4//aura"
            hex!("3a7824fdf9bb3717aafe3b1b3c21f8e3ea3239dfd80e97ab55621df33576cb7d").into(),
            hex!("3a7824fdf9bb3717aafe3b1b3c21f8e3ea3239dfd80e97ab55621df33576cb7d")
              .unchecked_into(),
          ),
          (
            // subkey inspect "$SECRET//clover//5//aura"
            hex!("aaee08e31c484817239fde12a32b9be6207ab8001102117cd086132ce18ecb79").into(),
            hex!("aaee08e31c484817239fde12a32b9be6207ab8001102117cd086132ce18ecb79")
              .unchecked_into(),
          ),
        ],
        // subkey inspect "$SECRET//clover//root"
        hex!("b6bede6cb32acce92409a782541fa2d8f3edaeeeab74ef28fb002cbec206db1e").into(),
        // Pre-funded accounts
        vec![hex!("b6bede6cb32acce92409a782541fa2d8f3edaeeeab74ef28fb002cbec206db1e").into()],
        true,
        endowed_evm_account(),
        id,
      )
    },
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    None,
    Some("clover"),
    // Properties
    None,
    // Extensions
    Extensions {
      relay_chain: "polkadot-local".into(),
      para_id: id.into(),
    },
  ))
}


pub fn sakura_mainnet_config(id: ParaId) -> Result<ChainSpec, String> {
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
        (
          // subkey inspect "$SECRET//clover//1//aura"
          hex!("60cd3f03542883d516031f18b6e6dc9c3eb5d58aebdd01f52485a2e4a7d5a958").into(),
          hex!("60cd3f03542883d516031f18b6e6dc9c3eb5d58aebdd01f52485a2e4a7d5a958").unchecked_into()
        ),
        (
          // subkey inspect "$SECRET//clover//2//aura"
          hex!("d066e8c5ec33604991169d213d44a86575e664893e02382351dc8c1b0148811c").into(),
          hex!("d066e8c5ec33604991169d213d44a86575e664893e02382351dc8c1b0148811c").unchecked_into()
        ),
        (
          // subkey inspect "$SECRET//clover//3//aura"
          hex!("206564a45a6ea6f32c5e3280a834b698b309f6031e15d7646e6ec0a2c162a00d").into(),
          hex!("206564a45a6ea6f32c5e3280a834b698b309f6031e15d7646e6ec0a2c162a00d").unchecked_into()
        ),
        (
          // subkey inspect "$SECRET//clover//4//aura"
          hex!("e4ce8803ffe0f08dbbf645e540d5b8bbd1d5862f2aa8441d3eb1e73c7d0c0b7e").into(),
          hex!("e4ce8803ffe0f08dbbf645e540d5b8bbd1d5862f2aa8441d3eb1e73c7d0c0b7e").unchecked_into()
        ),
        (
          // subkey inspect "$SECRET//clover//5//aura"
          hex!("ee64a09b2457926ca96a86d53a215a92987eb043284a233390193b85a0753667").into(),
          hex!("ee64a09b2457926ca96a86d53a215a92987eb043284a233390193b85a0753667").unchecked_into()
        ),
      ],
      // subkey inspect "$SECRET//clover//root"
      hex!("7aee8f965656f7280d97fc684e3bf0d3bb6d9a32d4021bbcbf3cfa3b7da5626a").into(),
      // Pre-funded accounts
      vec![
        hex!("7aee8f965656f7280d97fc684e3bf0d3bb6d9a32d4021bbcbf3cfa3b7da5626a").into()
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
    Some("skarua"),
    None,
    // Properties
    None,
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
  initial_authorities: Vec<(AccountId, AuraId)>,
  root_key: AccountId,
  endowed_accounts: Vec<AccountId>,
  _enable_println: bool,
  endowed_eth_accounts: BTreeMap<H160, GenesisAccount>,
  id: ParaId,
) -> GenesisConfig {
  let _enable_println = true;

  const ENDOWMENT: Balance = 1_000 * DOLLARS;
  // const STASH: Balance = 100 * DOLLARS;
  const AUTHOR_BALANCE: Balance = 200 * DOLLARS;

  GenesisConfig {
    system: SystemConfig {
      // Add Wasm runtime to storage.
      code: wasm_binary.to_vec(),
    },
    balances: BalancesConfig {
      // Configure endowed accounts with initial balance of 1 << 60.
      balances: endowed_accounts
        .iter()
        .cloned()
        .map(|k| (k, ENDOWMENT))
        .chain(
          initial_authorities
            .iter()
            .map(|x| (x.0.clone(), AUTHOR_BALANCE)),
        )
        .collect(),
    },
    // pallet_contracts: ContractsConfig {
    //   current_schedule: pallet_contracts::Schedule::default()
    //   .enable_println(enable_println),
    // },
    evm: EVMConfig {
      accounts: endowed_eth_accounts,
    },
    ethereum: EthereumConfig {},
    indices: IndicesConfig { indices: vec![] },
    //sudo: SudoConfig {
    //  // Assign network admin rights.
    //  key: Some(root_key),
    //},
    parachain_info: clover_runtime::ParachainInfoConfig { parachain_id: id },
    collator_selection: clover_runtime::CollatorSelectionConfig {
      invulnerables: initial_authorities
        .iter()
        .cloned()
        .map(|(acc, _)| acc)
        .collect(),
      candidacy_bond: 1 * DOLLARS,
      ..Default::default()
    },
    council: Default::default(),
    technical_committee: Default::default(),
    democracy: Default::default(),
    treasury: Default::default(),
    elections_phragmen: Default::default(),
    technical_membership: Default::default(),
    vesting: Default::default(),
    session: clover_runtime::SessionConfig {
      keys: initial_authorities
        .iter()
        .cloned()
        .map(|(acc, aura)| {
          (
            acc.clone(),        // account id
            acc.clone(),        // validator id
            session_keys(aura), // session keys
          )
        })
        .collect(),
    },
    aura: Default::default(),
    aura_ext: Default::default(),
    base_fee: Default::default(),
  }
}
