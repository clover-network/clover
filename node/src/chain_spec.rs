use serde_json::json;
use sp_core::{Pair, Public, sr25519, U256};
use clover_runtime::{
  AccountId, BabeConfig, Balance, AuthorityDiscoveryConfig, BalancesConfig, ContractsConfig, IndicesConfig, GenesisConfig, ImOnlineId,
  GrandpaConfig, SessionConfig, SessionKeys, StakingConfig, SudoConfig, SystemConfig, WASM_BINARY,
  Signature, StakerStatus,
  EVMConfig, EthereumConfig, DOLLARS
};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::{traits::{IdentifyAccount, Verify}, Perbill};
use sc_service::ChainType;
use hex_literal::hex;
use sc_telemetry::TelemetryEndpoints;
use sp_core::crypto::UncheckedInto;
use std::collections::BTreeMap;
use pallet_evm::GenesisAccount;
use primitive_types::H160;
use std::str::FromStr;

// The URL for the telemetry server.
const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

fn session_keys(
  grandpa: GrandpaId,
  babe: BabeId,
  im_online: ImOnlineId,
  authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
  SessionKeys { grandpa, babe, im_online, authority_discovery, }
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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId) {
  (
    get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
    get_account_id_from_seed::<sr25519::Public>(s),
    get_from_seed::<BabeId>(s),
    get_from_seed::<GrandpaId>(s),
    get_from_seed::<ImOnlineId>(s),
    get_from_seed::<AuthorityDiscoveryId>(s),
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
        //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
      ],
      true,
      dev_endowed_evm_accounts()
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
        //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        //get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
      ],
      true,
      endowed_evm_account()
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
          hex!["005b5b120aabe29673ca587a738ff1032437a388b006b51a9d6ea16f3dee6349"].unchecked_into(), // babe key
          hex!["6575c1155089f6653206ffa533757ef71a9efb2738fb86bcc89128b1517788c0"].unchecked_into(), // grandpa
          hex!["f8bc696eadcba0561c7a19af387b11f7db04e1d6457d344aa626476d6152a612"].unchecked_into(), // imonline
          hex!["64f317d45163a8b4c1960c60550ea1f70506768a96eac2881f7805b9141d1b11"].unchecked_into(), // discovery
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
          hex!["dcb5d89f40d57b9da9cd1f677c789584e4e88e1cdfd7a91d561757e23e73aa45"].unchecked_into(), // babe
          hex!["c7925c95410d4ad451f9bc995852127f169bef4fd75f2c23f9472620ddd59f91"].unchecked_into(), // grandpa
          hex!["14e2ecd186552e1dfb1f2d5233657b69e0b398d7ec405bb68071ee19d3009f04"].unchecked_into(), // imonline
          hex!["e404b380c6bd7ab0577a5e6809a3338d28d191137e7581bdd23eb3e893ca9e6a"].unchecked_into(), // discovery
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
          hex!["c08908eb1a58eb1df74e54415cdd4977c20023cc7f5dff771c38f26491367b6e"].unchecked_into(), // babe
          hex!["0ec2a175b1efc3835a8d1497f914ef39ec4ba0ea7a60cf4cb440586fa74fcd99"].unchecked_into(), // grandpa
          hex!["f49fda7f7db9af41fd4095a7bf37745e4cc30f9b592c1563ecc5fe2292e9f309"].unchecked_into(), // imonline
          hex!["e0520566773304de5fd0d448b0ca76b6a2c7edd66d90b4dba36785e64ba65949"].unchecked_into(), // discovery
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
      endowed_evm_account()
    ),
    // Bootnodes
    vec![
      "/dns/seed1.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWPb5VY3dzydLFVh4Bn8sk73QvicvVoYcHQawRgicuMNwJ"
        .parse()
        .unwrap(),
      "/dns/seed2.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWG4jPV345wrEE23tdRh69i9YH5BtSo8RhToPj4fTaJgkZ"
        .parse()
        .unwrap(),
    ],
    // Telemetry
    TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
    // Protocol ID
    Some("rose"),
    // Properties
    Some(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    None,
  ))
}

pub fn iris_testnet_config() -> Result<ChainSpec, String> {
  let wasm_binary = WASM_BINARY.ok_or("Development wasm binary not available".to_string())?;

  Ok(ChainSpec::from_genesis(
    // Name
    "Clover",
    // ID
    "iris",
    ChainType::Custom(String::from("iris")),
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
          hex!["005b5b120aabe29673ca587a738ff1032437a388b006b51a9d6ea16f3dee6349"].unchecked_into(), // babe key
          hex!["6575c1155089f6653206ffa533757ef71a9efb2738fb86bcc89128b1517788c0"].unchecked_into(), // grandpa
          hex!["f8bc696eadcba0561c7a19af387b11f7db04e1d6457d344aa626476d6152a612"].unchecked_into(), // imonline
          hex!["64f317d45163a8b4c1960c60550ea1f70506768a96eac2881f7805b9141d1b11"].unchecked_into(), // discovery
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
          hex!["dcb5d89f40d57b9da9cd1f677c789584e4e88e1cdfd7a91d561757e23e73aa45"].unchecked_into(), // babe
          hex!["c7925c95410d4ad451f9bc995852127f169bef4fd75f2c23f9472620ddd59f91"].unchecked_into(), // grandpa
          hex!["14e2ecd186552e1dfb1f2d5233657b69e0b398d7ec405bb68071ee19d3009f04"].unchecked_into(), // imonline
          hex!["e404b380c6bd7ab0577a5e6809a3338d28d191137e7581bdd23eb3e893ca9e6a"].unchecked_into(), // discovery
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
          hex!["c08908eb1a58eb1df74e54415cdd4977c20023cc7f5dff771c38f26491367b6e"].unchecked_into(), // babe
          hex!["0ec2a175b1efc3835a8d1497f914ef39ec4ba0ea7a60cf4cb440586fa74fcd99"].unchecked_into(), // grandpa
          hex!["f49fda7f7db9af41fd4095a7bf37745e4cc30f9b592c1563ecc5fe2292e9f309"].unchecked_into(), // imonline
          hex!["e0520566773304de5fd0d448b0ca76b6a2c7edd66d90b4dba36785e64ba65949"].unchecked_into(), // discovery
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
      endowed_evm_account()
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
    TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
    // Protocol ID
    Some("iris"),
    // Properties
    Some(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone()),
    // Extensions
    None,
  ))
}


/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
  wasm_binary: &[u8],
  initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId,)>,
  root_key: AccountId,
  endowed_accounts: Vec<AccountId>,
  _enable_println: bool,
  endowed_eth_accounts: BTreeMap<H160, GenesisAccount>,
) -> GenesisConfig {
  let enable_println = true;

  const ENDOWMENT: Balance = 1_000 * DOLLARS;
  const STASH: Balance = 100 * DOLLARS;
  const AUTHOR_BALANCE: Balance = 200 * DOLLARS;

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
            .chain(initial_authorities.iter().map(|x| (x.0.clone(), AUTHOR_BALANCE)))
            .collect(),
    }),
    pallet_contracts: Some(ContractsConfig {
      current_schedule: pallet_contracts::Schedule {
        enable_println, // this should only be enabled on development chains
        ..Default::default()
      },
    }),
    pallet_evm: Some(EVMConfig {
      accounts: endowed_eth_accounts,
    }),
    pallet_ethereum: Some(EthereumConfig {}),
    pallet_indices: Some(IndicesConfig {
      indices: vec![],
    }),
    pallet_session: Some(SessionConfig {
      keys: initial_authorities.iter().map(|x| {
        (x.0.clone(), x.0.clone(), session_keys(
          x.3.clone(),
          x.2.clone(),
          x.4.clone(),
          x.5.clone(),
        ))
      }).collect::<Vec<_>>(),
    }),
    pallet_staking: Some(StakingConfig {
      validator_count: initial_authorities.len() as u32,
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
    pallet_im_online: Some(Default::default()),
    pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
      keys: vec![],
    }),
    pallet_sudo: Some(SudoConfig {
      // Assign network admin rights.
      key: root_key,
    }),
    pallet_collective_Instance1: Some(Default::default()),
    pallet_collective_Instance2: Some(Default::default()),
    pallet_democracy: Some(Default::default()),
    pallet_treasury: Some(Default::default()),
    pallet_elections_phragmen: Some(Default::default()),
    pallet_membership_Instance1: Some(Default::default()),
    pallet_vesting: Some(Default::default()),
  }
}
