use serde_json::json;
use sp_core::{Pair, Public, sr25519};
use clover_runtime::{
  AccountId, BabeConfig, Balance, BalancesConfig, ContractsConfig, CurrencyId, IndicesConfig, GenesisConfig,
  GrandpaConfig, SessionConfig, SessionKeys, StakingConfig, SudoConfig, SystemConfig, WASM_BINARY,
  Signature, StakerStatus, TokensConfig, IncentivesConfig, CloverDexConfig, BandOracleConfig,
  CloverOracleConfig, EVMConfig, EthereumConfig
};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{traits::{IdentifyAccount, Verify}, Perbill};
use sc_service::ChainType;
use hex_literal::hex;
use sc_telemetry::TelemetryEndpoints;
use sp_core::crypto::UncheckedInto;
use std::collections::BTreeMap;

// The URL for the telemetry server.
const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

fn session_keys(
  grandpa: GrandpaId,
  babe: BabeId
) -> SessionKeys {
  SessionKeys { grandpa, babe}
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
  TPublic::Pair::from_string(&format!("//{}", seed), None)
    .expect("static values are valid; qed")
    .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
  AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
  AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Babe authority key.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, BabeId, GrandpaId) {
  (
    get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
    get_account_id_from_seed::<sr25519::Public>(s),
    get_from_seed::<BabeId>(s),
    get_from_seed::<GrandpaId>(s),
  )
}

pub fn development_config() -> Result<ChainSpec, String> {
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
        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
      ],
      true,
      ),
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    // Properties
    Some(json!({
      "tokenDecimals": 12,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    None,
  ))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
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
        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
      ],
      true,
    ),
    // Bootnodes
    vec![],
    // Telemetry
    None,
    // Protocol ID
    Some("cloverlocal"),
    // Properties
    Some(json!({
      "tokenDecimals": 12,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    None,
  ))
}

pub fn local_rose_testnet_config() -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Clover",
    // ID
    "rose",
    ChainType::Custom(String::from("rose")),
    move || testnet_genesis(
      wasm_binary,
      // Initial PoA authorities
      vec![
        // 5FErVqdraDMVVHRYHzFVubtvaMaaySxtgFpmdX9WEpx6zwai
        (
          hex!["8c723dff02c2e2e578609e5caa0eda0572913f73b1c330ad7a2aa3819453762e"].into(),
          hex!["8c723dff02c2e2e578609e5caa0eda0572913f73b1c330ad7a2aa3819453762e"].into(),
          hex!["3210617311f52ac55feead02772acb048234d16e04c33ada32d3faa6c3eecc44"].unchecked_into(),
          hex!["3d145911dd713e50f7cea8f65eec1dec5e7cba466b4a16ad6b75ce3c11b1ad0b"].unchecked_into(),
        ),
        // 5F6Qp4EEbg8KQdpaE1E4dQBuRPk5WRc9imvbEgCYMfruxwDz
        (
          hex!["8601cffcc5836815e60092831cb79b9242b995bcf5cd90589c21092811e3e859"].into(),
          hex!["8601cffcc5836815e60092831cb79b9242b995bcf5cd90589c21092811e3e859"].into(),
          hex!["80a3099c09a963dee18fc99f1455ba6666ab8efc6576e64b1330e33b994cfd4a"].unchecked_into(),
          hex!["9ae3442373c948b8ae442e4de4633a9ce4a3d06b8d1cf3ca11916586ef46f4a6"].unchecked_into(),
        ),
        // 5DLTV9Dp1sCKdBVp8iBYi1LSFze8iv7A8EVvh9zsAs4ouRag
        (
          hex!["383fc84801261040d7a0feef51d64a06a02033b6887fdcf1f031b6f4deaba447"].into(),
          hex!["383fc84801261040d7a0feef51d64a06a02033b6887fdcf1f031b6f4deaba447"].into(),
          hex!["cc3a0b74e4a61e41ac64a0e18c22bc3bf0f0e6c9ded3c08def8b9d3f1c37d324"].unchecked_into(),
          hex!["a0be89588eefd7129641edcff28c8fcb1054fc981b27aee3e88668beee9d0d4d"].unchecked_into(),
        ),
      ],
      // 5Cwo46bWWxaZCJQYkwH62nChaiEDKY9Kh4oo8kfbS9SNesMf
      hex!["26f702ab9792cbb2ea9c23b9f7982b6f6d6e9c3561e701175f9df919cf75f01f"].into(),
      // Pre-funded accounts
      vec![
        // 5Cwo46bWWxaZCJQYkwH62nChaiEDKY9Kh4oo8kfbS9SNesMf
        hex!["26f702ab9792cbb2ea9c23b9f7982b6f6d6e9c3561e701175f9df919cf75f01f"].into(),
      ],
      true,
    ),
    // Bootnodes
    vec![
      "/dns/seed1.rose.clovernode.com/tcp/30333/p2p/12D3KooWF9dXRyooKqXCJWPyvQx2Am2Tg1Wq2pqBdgbor57g2cFQ"
        .parse()
        .unwrap(),
      "/dns/seed2.rose.clovernode.com/tcp/30333/p2p/12D3KooWPrKZgyxGniSna9yigFrqhR2nA4ZcBWH4qwUAoGsc6PSp"
        .parse()
        .unwrap(),
    ],
    // Telemetry
    TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
    // Protocol ID
    Some("rose"),
    // Properties
    Some(json!({
      "tokenDecimals": 12,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    None,
  ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
  wasm_binary: &[u8],
  initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId)>,
  root_key: AccountId,
  endowed_accounts: Vec<AccountId>,
  _enable_println: bool,
) -> GenesisConfig {
  use clover_runtime::{DOLLARS};

  let enable_println = true;

  const ENDOWMENT: Balance = 10_000 * DOLLARS;
  const STASH: Balance = 100 * DOLLARS;

  GenesisConfig {
    frame_system: Some(SystemConfig {
      // Add Wasm runtime to storage.
      code: wasm_binary.to_vec(),
      changes_trie_config: Default::default(),
    }),
    pallet_balances: Some(BalancesConfig {
      // Configure endowed accounts with initial balance of 1 << 60.
      balances: endowed_accounts.iter().cloned()
            .map(|k| (k, ENDOWMENT))
            .chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
            .collect(),
    }),
    pallet_contracts: Some(ContractsConfig {
      current_schedule: pallet_contracts::Schedule {
        enable_println, // this should only be enabled on development chains
        ..Default::default()
      },
    }),
    clover_evm: Some(EVMConfig {
      accounts: BTreeMap::new(),
    }),
    clover_ethereum: Some(EthereumConfig {}),
    pallet_indices: Some(IndicesConfig {
      indices: vec![],
    }),
    pallet_session: Some(SessionConfig {
      keys: initial_authorities.iter().map(|x| {
        (x.0.clone(), x.0.clone(), session_keys(
          x.3.clone(),
          x.2.clone(),
        ))
      }).collect::<Vec<_>>(),
    }),
    pallet_staking: Some(StakingConfig {
      validator_count: initial_authorities.len() as u32 * 2,
      minimum_validator_count: initial_authorities.len() as u32,
      stakers: initial_authorities.iter().map(|x| {
        (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
      }).collect(),
      invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
      slash_reward_fraction: Perbill::from_percent(10),
      .. Default::default()
    }),
    pallet_babe: Some(BabeConfig {
      authorities: vec![],
    }),
    pallet_grandpa: Some(GrandpaConfig {
      authorities: vec![],
    }),
    pallet_sudo: Some(SudoConfig {
      // Assign network admin rights.
      key: root_key,
    }),
    orml_oracle_Instance1: Some(CloverOracleConfig {
      members: Default::default(), // initialized by OperatorMembership
      phantom: Default::default(),
    }),
    orml_oracle_Instance2: Some(BandOracleConfig {
      members: Default::default(), // initialized by OperatorMembership
      phantom: Default::default(),
    }),
      orml_tokens: Some(TokensConfig {
      endowed_accounts: endowed_accounts
        .iter()
        .flat_map(|x| {
          vec![
            (x.clone(), CurrencyId::CETH, 10000 * DOLLARS),
            (x.clone(), CurrencyId::CUSDT, 10000 * DOLLARS),
            (x.clone(), CurrencyId::DOT, 10000 * DOLLARS),
          ]
        })
        .collect(),
    }),
    clover_incentives: Some(IncentivesConfig{
      dex_rewards: vec![
        (CurrencyId::CLV, CurrencyId::DOT, 1 * DOLLARS),
        (CurrencyId::CLV, CurrencyId::CUSDT, 2 * DOLLARS),
        (CurrencyId::CETH, CurrencyId::CUSDT, 3 * DOLLARS),
      ],
    }),
    cloverdex: Some(CloverDexConfig {
        initial_pairs: vec![
          (CurrencyId::CUSDT, CurrencyId::CETH, Some(1000 * DOLLARS), Some(500 * DOLLARS)),
          (CurrencyId::CUSDT, CurrencyId::DOT, Some(700 * DOLLARS), Some(250 * DOLLARS)),
          (CurrencyId::CUSDT, CurrencyId::CLV, Some(300 * DOLLARS), Some(600 * DOLLARS)),
        ],
    }),
    pallet_collective_Instance1: Some(Default::default()),
    pallet_collective_Instance2: Some(Default::default()),
    pallet_democracy: Some(Default::default()),
    pallet_treasury: Some(Default::default()),
    pallet_elections_phragmen: Some(Default::default()),
    pallet_membership_Instance1: Some(Default::default()),
  }
}
