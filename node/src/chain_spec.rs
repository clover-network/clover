use clover_runtime::{
    wasm_binary_unwrap, AccountId, Balance, Block, ImOnlineId, SessionKeys, Signature,
    StakerStatus, DOLLARS,
};
use fp_evm::GenesisAccount;
use hex_literal::hex;
use primitive_types::H160;
use sc_chain_spec::ChainSpecExtension;
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::UncheckedInto;
use sp_core::{sr25519, Pair, Public, U256};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::Perbill;
use std::collections::BTreeMap;
use std::str::FromStr;

// The URL for the telemetry server.
const TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
    /// The light sync state extension used by the sync-state rpc.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Babe authority key.
pub fn authority_keys_from_seed(
    s: &str,
) -> (
    AccountId,
    AccountId,
    BabeId,
    GrandpaId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<BabeId>(s),
        get_from_seed::<GrandpaId>(s),
        get_from_seed::<ImOnlineId>(s),
        get_from_seed::<AuthorityDiscoveryId>(s),
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

pub fn development_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_protocol_id("cloverlocal")
        .with_properties(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        )
        .with_genesis_config_patch(testnet_genesis(
            // Initial PoA authorities
            vec![authority_keys_from_seed("Alice")],
            // Sudo account
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            // Pre-funded accounts
            vec![
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                get_account_id_from_seed::<sr25519::Public>("Bob"),
                //get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            ],
            dev_endowed_evm_accounts(),
        ))
        .build()
}

pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
        .with_name("Clover")
        .with_id("local_testnet")
        .with_chain_type(ChainType::Local)
        .with_protocol_id("cloverlocal")
        .with_properties(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        )
        .with_genesis_config_patch(testnet_genesis(
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
            endowed_evm_account(),
        ))
        .build()
}

pub fn local_rose_testnet_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
    .with_name("Clover")
    .with_id("rose")
    .with_chain_type(ChainType::Custom("rose".to_string()))
    .with_properties(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone())
    .with_genesis_config_patch(testnet_genesis(
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
        endowed_evm_account()
      ))
      .with_boot_nodes(
        vec![
          "/dns/seed1.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWPb5VY3dzydLFVh4Bn8sk73QvicvVoYcHQawRgicuMNwJ"
            .parse()
            .unwrap(),
          "/dns/seed2.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWG4jPV345wrEE23tdRh69i9YH5BtSo8RhToPj4fTaJgkZ"
            .parse()
            .unwrap(),
        ],)
      .with_telemetry_endpoints(TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).expect("Valid telemetry url"))
      .with_protocol_id("rose")
      .build()
}

pub fn iris_testnet_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
  .with_name("Clover")
  .with_id("iris")
  .with_chain_type(ChainType::Custom("iris".to_string()))
  .with_properties(json!({
    "tokenDecimals": 18,
    "tokenSymbol": "CLV"
  }).as_object().expect("Created an object").clone())
  .with_genesis_config_patch(testnet_genesis(
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
      endowed_evm_account()
    ))
    .with_boot_nodes(
      vec![
        "/dns/seed1.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWFtshqoFL1hAwseGc4WuFeREKicjFR15JiVEaJiHnDvn2"
          .parse()
          .unwrap(),
        "/dns/seed2.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWBcU1EShS2azLwQhKVKyeXU2cc3CWyhuN8wJwEKaRVNe8"
          .parse()
          .unwrap(),
      ])
    .with_telemetry_endpoints(TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).expect("Valid telemetry url"))
    .with_protocol_id("iris")
    .build()
}

pub fn ivy_config() -> ChainSpec {
    ChainSpec::builder(wasm_binary_unwrap(), Default::default())
    .with_name("Clover Mainnet")
    .with_id("clover_ivy")
    .with_chain_type(ChainType::Live)
    .with_properties(json!({
      "tokenDecimals": 18,
      "tokenSymbol": "CLV"
    }).as_object().expect("Created an object").clone())
    .with_genesis_config_patch(testnet_genesis(
      // Initial PoA authorities
      vec![
        // SECRET="..."
        // 5H14XVgazykT5sz2hUPkEBmj3N4wzj5Zej9QVmVwCzN4iYVL
        // subkey inspect "$SECRET//clover//1//validator"
        // subkey inspect "$SECRET//clover//1//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//1//grandpa"
        // subkey inspect "$SECRET//clover//1//imonline"
        // subkey inspect "$SECRET//clover//1//discovery"
        (
          hex!["da65c3df9f86fbfb2b301282a5c0807d501e265354b5badd475eb4e28960aa55"].into(),
          hex!["da65c3df9f86fbfb2b301282a5c0807d501e265354b5badd475eb4e28960aa55"].into(),
          hex!["e254d4a170f065a4c76128d5b70da167b93550e26f61c16c36268fd0703d7e0b"].unchecked_into(), // babe key
          hex!["730ef78d56adfbbd2889a794dd358b35d0b5573415b79baccbc213c688ee3e32"].unchecked_into(), // grandpa
          hex!["8815cb09de848860956ee58c6900161b9491ed955d83c10d6f11bbc4bf5a3627"].unchecked_into(), // imonline
          hex!["321514a7b080dfc016c134fe531fbd338a5ce4128ebc9f09611b14a42c2b9e6a"].unchecked_into(), // discovery
        ),
        // 5H6bu5PdTY2WovY4tqUWLi9pHyTuXXkRwR4XZ4N5YNfUJdpY
        // subkey inspect "$SECRET//clover//2//validator"
        // subkey inspect "$SECRET//clover//2//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//2//grandpa"
        // subkey inspect "$SECRET//clover//2//imonline"
        // subkey inspect "$SECRET//clover//2//discovery"
        (
          hex!["de9f999ad12043c5793eddbb4189b0a82d3225c4b65468ef689a8b42bacb261a"].into(),
          hex!["de9f999ad12043c5793eddbb4189b0a82d3225c4b65468ef689a8b42bacb261a"].into(),
          hex!["4a86c546231758db8c648f616d3c5c45d5851275dd03ef174e9d42b06b657c6b"].unchecked_into(), // babe
          hex!["2f2534068447782f49f51827355c069c3b61cf3cea9bff5060d45cfe754dd386"].unchecked_into(), // grandpa
          hex!["a6a4614a1bc934d19c15fec123d1bac6c3d1e67bf88439567ea7444114e47c12"].unchecked_into(), // imonline
          hex!["10d5fede129511e883414f6a17457eca1bae6e5cc8c8d8a59efa810e68f4326e"].unchecked_into(), // discovery
        ),
        // 5Gdi2r42Sx2Kka7Us4sJePdRLaogZH4MRCvKLqYVuEmLwrjC
        // subkey inspect "$SECRET//clover//3//validator"
        // subkey inspect "$SECRET//clover//3//babe"
        // subkey inspect --scheme ed25519 "$SECRET//clover//3//grandpa"
        // subkey inspect "$SECRET//clover//3//imonline"
        // subkey inspect "$SECRET//clover//3//discovery"
        (
          hex!["ca1c9f747fe1f7ea25c521e63bc521d825922ae1724c569556616ee6d173284b"].into(),
          hex!["ca1c9f747fe1f7ea25c521e63bc521d825922ae1724c569556616ee6d173284b"].into(),
          hex!["84bfc5961ccce24c4348e96cba5ce476c3864d1b37523105c908433517413e0f"].unchecked_into(), // babe
          hex!["bd1a9c279e81f75b682e67e9a75f4c466af318a6ab0cd7721ceb7a8bee6a1376"].unchecked_into(), // grandpa
          hex!["106bfca82987d1c44638e2b7b7a433d62c38b8e463471b4a6770a23ee5af3b0a"].unchecked_into(), // imonline
          hex!["c0ff707d2849e3ea00803824ba50ed38f284bd84841cc5c8ff890406d048f90d"].unchecked_into(), // discovery
        ),
      ],
      // 5HToWGk4935P9VWS8VCTKLbCiG1jT9g2JM6b6ThcS79GJ1xT
      // subkey inspect "$SECRET//clover//root"
      hex!["eecad357d3e3f702947770e84211cf47eaf2d62611eb5642a7e266a829bfb35c"].into(),
      // Pre-funded accounts
      vec![
        // 5HToWGk4935P9VWS8VCTKLbCiG1jT9g2JM6b6ThcS79GJ1xT
        hex!["eecad357d3e3f702947770e84211cf47eaf2d62611eb5642a7e266a829bfb35c"].into(),
      ],
      BTreeMap::new(), // evm accounts
    ))
    .with_boot_nodes(
      vec![
        "/dns/seed1.ivy.clover.finance/tcp/30333/p2p/12D3KooWAw6GLPuBsatTjmwhqq4vjEEiVMmttSS3V56ZWX7J9Yh5"
          .parse()
          .unwrap(),
        "/dns/seed2.ivy.clover.finance/tcp/30333/p2p/12D3KooWCNnKzKHrPB5LcddhjA9epkq81pdkcaxm2berU7pgYmEN"
          .parse()
          .unwrap(),
      ]
    )
    .with_telemetry_endpoints(TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).expect("Valid telemetry url"))
    .with_protocol_id("ivy")
    .build()
}

/// Configure initial storage state for FRAME modules.
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    endowed_eth_accounts: BTreeMap<H160, GenesisAccount>,
) -> serde_json::Value {
    const ENDOWMENT: Balance = 1_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;
    const AUTHOR_BALANCE: Balance = 200 * DOLLARS;

    let num_endowed_accounts = endowed_accounts.len();

    serde_json::json!({
      "balances": {
        "balances": endowed_accounts.iter().chain(initial_authorities.iter().map(|a| &a.0)).map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
      },
      "session": {
        "keys": initial_authorities.iter()
          .map(|x| (x.0.clone(), x.0.clone(), session_keys(
            x.3.clone(),
            x.2.clone(),
            x.4.clone(),
            x.5.clone(),
          )))
          .collect::<Vec<_>>(),
      },
      "staking": {
        "validatorCount": initial_authorities.len() as u32,
        "minimumValidatorCount": initial_authorities.len() as u32,
        "invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        "slashRewardFraction": Perbill::from_percent(10),
        "stakers":  initial_authorities.iter().map(|x| {
          (x.0.clone(), x.0.clone(), STASH, StakerStatus::<AccountId>::Validator)
        }).collect::<Vec<_>>(),
      },
      "babe": {
        "epochConfig": Some(clover_runtime::BABE_GENESIS_EPOCH_CONFIG),
      },
      "sudo": { "key": Some(root_key.clone()) },
      "evm": {
        "accounts": Some(endowed_eth_accounts),
      },
      "electionsPhragmen": {
        "members": endowed_accounts
          .iter()
          .take((num_endowed_accounts + 1) / 2)
          .cloned()
          .map(|member| (member, STASH))
          .collect::<Vec<_>>(),
      },
    })
}
