use std::{collections::BTreeMap, str::FromStr};

use clover_primitives::currency::*;
pub use clover_primitives::{AccountId, Balance, Signature};
pub use clover_runtime::RuntimeGenesisConfig;
use clover_runtime::{
    wasm_binary_unwrap, BabeConfig, BalancesConfig, Block, CouncilConfig, DemocracyConfig,
    EVMConfig, EthereumConfig, ImOnlineConfig, IndicesConfig, SessionConfig, SessionKeys,
    StakerStatus, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
};
use fp_evm::GenesisAccount;
use hex_literal::hex;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainSpecExtension;
use sc_service::{ChainType, Properties};
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public, H160, U256};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};
type AccountPublic = <Signature as Verify>::Signer;

/// Telemetry URL.
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

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

/// Session keys for a chain.
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

/// Helper function to generate a chain spec for staging testnet.
fn staging_testnet_config_genesis() -> RuntimeGenesisConfig {
    #[rustfmt::skip]
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	//
	// and
	//
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)> = vec![
		(
			// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
			array_bytes::hex_n_into_unchecked("9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"),
			// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
			array_bytes::hex_n_into_unchecked("781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"),
			// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
			array_bytes::hex2array_unchecked("9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332")
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			array_bytes::hex2array_unchecked("6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106")
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			array_bytes::hex2array_unchecked("6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106")
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			array_bytes::hex2array_unchecked("6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106")
				.unchecked_into(),
		),
		(
			// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
			array_bytes::hex_n_into_unchecked("68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"),
			// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
			array_bytes::hex_n_into_unchecked("c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"),
			// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
			array_bytes::hex2array_unchecked("7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f")
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			array_bytes::hex2array_unchecked("482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e")
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			array_bytes::hex2array_unchecked("482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e")
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			array_bytes::hex2array_unchecked("482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e")
				.unchecked_into(),
		),
		(
			// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
			array_bytes::hex_n_into_unchecked("547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"),
			// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
			array_bytes::hex_n_into_unchecked("9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"),
			// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
			array_bytes::hex2array_unchecked("5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440")
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			array_bytes::hex2array_unchecked("482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a")
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			array_bytes::hex2array_unchecked("482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a")
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			array_bytes::hex2array_unchecked("482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a")
				.unchecked_into(),
		),
		(
			// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
			array_bytes::hex_n_into_unchecked("f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"),
			// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
			array_bytes::hex_n_into_unchecked("66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"),
			// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
			array_bytes::hex2array_unchecked("3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef")
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			array_bytes::hex2array_unchecked("00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378")
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			array_bytes::hex2array_unchecked("00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378")
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			array_bytes::hex2array_unchecked("00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378")
				.unchecked_into(),
		),
	];

    // generated with secret: subkey inspect "$secret"/fir
    let root_key: AccountId = array_bytes::hex_n_into_unchecked(
        // 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
        "9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809",
    );

    let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

    testnet_genesis(
        initial_authorities,
        vec![],
        root_key,
        Some(endowed_accounts),
        dev_endowed_evm_accounts(),
    )
}

/// Staging testnet config.
pub fn staging_testnet_config() -> ChainSpec {
    let boot_nodes = vec![];
    ChainSpec::from_genesis(
        "Staging Testnet",
        "staging_testnet",
        ChainType::Live,
        staging_testnet_config_genesis,
        boot_nodes,
        Some(
            TelemetryEndpoints::new(vec![(TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        ),
        None,
        None,
        None,
        Default::default(),
    )
}

/// Helper function to generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed.
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

/// Get hard coded endowed account for EVM.
fn endowed_evm_account() -> BTreeMap<H160, GenesisAccount> {
    let endowed_account = vec![
        // clover fauct
        H160::from_str("9157f0316f375e4ccf67f8d21ae0881d0abcbb21").unwrap(),
    ];
    get_endowed_evm_accounts(endowed_account)
}

/// Get hard coded endowed accounts for EVM.
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

/// Helper function to convert endowed accounts to EVM genesis accounts.
fn get_endowed_evm_accounts(endowed_accounts: Vec<H160>) -> BTreeMap<H160, GenesisAccount> {
    let mut evm_accounts = BTreeMap::new();
    for account in endowed_accounts {
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

/// Development testnet genesis
fn development_config_genesis() -> RuntimeGenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        dev_endowed_evm_accounts(),
    )
}

/// Development config (single validator Alice).
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        Some("cloverlocal"),
        None,
        Some(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        ),
        Default::default(),
    )
}

/// Generic local testnet genesis
fn local_testnet_genesis() -> RuntimeGenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        // Pre-funded accounts
        Some(vec![
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
        ]),
        endowed_evm_account(),
    )
}

/// Local testnet config (multivalidator Alice + Bob).
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        Some("cloverlocal"),
        None,
        Some(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        ),
        Default::default(),
    )
}

/// Local rose testnet genesis
fn local_rose_testnet_genesis() -> RuntimeGenesisConfig {
    testnet_genesis(
        vec![
            (
                hex!["222c5fa244583b1734ceb6515916efc5e103f65b869ebec4e56b989d9dbb446e"].into(),
                hex!["222c5fa244583b1734ceb6515916efc5e103f65b869ebec4e56b989d9dbb446e"].into(),
                hex!["005b5b120aabe29673ca587a738ff1032437a388b006b51a9d6ea16f3dee6349"]
                    .unchecked_into(), // babe key
                hex!["6575c1155089f6653206ffa533757ef71a9efb2738fb86bcc89128b1517788c0"]
                    .unchecked_into(), // grandpa
                hex!["f8bc696eadcba0561c7a19af387b11f7db04e1d6457d344aa626476d6152a612"]
                    .unchecked_into(), // imonline
                hex!["64f317d45163a8b4c1960c60550ea1f70506768a96eac2881f7805b9141d1b11"]
                    .unchecked_into(), // discovery
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
                hex!["dcb5d89f40d57b9da9cd1f677c789584e4e88e1cdfd7a91d561757e23e73aa45"]
                    .unchecked_into(), // babe
                hex!["c7925c95410d4ad451f9bc995852127f169bef4fd75f2c23f9472620ddd59f91"]
                    .unchecked_into(), // grandpa
                hex!["14e2ecd186552e1dfb1f2d5233657b69e0b398d7ec405bb68071ee19d3009f04"]
                    .unchecked_into(), // imonline
                hex!["e404b380c6bd7ab0577a5e6809a3338d28d191137e7581bdd23eb3e893ca9e6a"]
                    .unchecked_into(), // discovery
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
                hex!["c08908eb1a58eb1df74e54415cdd4977c20023cc7f5dff771c38f26491367b6e"]
                    .unchecked_into(), // babe
                hex!["0ec2a175b1efc3835a8d1497f914ef39ec4ba0ea7a60cf4cb440586fa74fcd99"]
                    .unchecked_into(), // grandpa
                hex!["f49fda7f7db9af41fd4095a7bf37745e4cc30f9b592c1563ecc5fe2292e9f309"]
                    .unchecked_into(), // imonline
                hex!["e0520566773304de5fd0d448b0ca76b6a2c7edd66d90b4dba36785e64ba65949"]
                    .unchecked_into(), // discovery
            ),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        Some(vec![
            // 5CPQQYs3wf32fr5PhmmfFQEeVzD1Zy9Hdo8LFzQYuhP8XHW6
            hex!["0e42eb6f65a8ef5e3f3c3cdb5b2c3be646e791abd76e2224d5847cde786b2e01"].into(),
        ]),
        endowed_evm_account(),
    )
}

/// Local rose testnet config (multivalidator Alice + Bob).
pub fn local_rose_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Clover",
        "rose",
        ChainType::Custom(String::from("rose")),
        local_testnet_genesis,
    vec![
      "/dns/seed1.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWPb5VY3dzydLFVh4Bn8sk73QvicvVoYcHQawRgicuMNwJ"
        .parse()
        .unwrap(),
      "/dns/seed2.rose-cc2.clovernode.com/tcp/30333/p2p/12D3KooWG4jPV345wrEE23tdRh69i9YH5BtSo8RhToPj4fTaJgkZ"
        .parse()
        .unwrap(),
    ],
    TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),

    Some("rose"),
        None,
        Some(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        ),
        Default::default(),
    )
}

/// Helper function to generate a testnet genesis config.
fn iris_testnet_genesis() -> RuntimeGenesisConfig {
    testnet_genesis(
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
                hex!["005b5b120aabe29673ca587a738ff1032437a388b006b51a9d6ea16f3dee6349"]
                    .unchecked_into(), // babe key
                hex!["6575c1155089f6653206ffa533757ef71a9efb2738fb86bcc89128b1517788c0"]
                    .unchecked_into(), // grandpa
                hex!["f8bc696eadcba0561c7a19af387b11f7db04e1d6457d344aa626476d6152a612"]
                    .unchecked_into(), // imonline
                hex!["64f317d45163a8b4c1960c60550ea1f70506768a96eac2881f7805b9141d1b11"]
                    .unchecked_into(), // discovery
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
                hex!["dcb5d89f40d57b9da9cd1f677c789584e4e88e1cdfd7a91d561757e23e73aa45"]
                    .unchecked_into(), // babe
                hex!["c7925c95410d4ad451f9bc995852127f169bef4fd75f2c23f9472620ddd59f91"]
                    .unchecked_into(), // grandpa
                hex!["14e2ecd186552e1dfb1f2d5233657b69e0b398d7ec405bb68071ee19d3009f04"]
                    .unchecked_into(), // imonline
                hex!["e404b380c6bd7ab0577a5e6809a3338d28d191137e7581bdd23eb3e893ca9e6a"]
                    .unchecked_into(), // discovery
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
                hex!["c08908eb1a58eb1df74e54415cdd4977c20023cc7f5dff771c38f26491367b6e"]
                    .unchecked_into(), // babe
                hex!["0ec2a175b1efc3835a8d1497f914ef39ec4ba0ea7a60cf4cb440586fa74fcd99"]
                    .unchecked_into(), // grandpa
                hex!["f49fda7f7db9af41fd4095a7bf37745e4cc30f9b592c1563ecc5fe2292e9f309"]
                    .unchecked_into(), // imonline
                hex!["e0520566773304de5fd0d448b0ca76b6a2c7edd66d90b4dba36785e64ba65949"]
                    .unchecked_into(), // discovery
            ),
        ],
        vec![],
        // 5CPQQYs3wf32fr5PhmmfFQEeVzD1Zy9Hdo8LFzQYuhP8XHW6
        // subkey inspect "$SECRET//clover//root"
        hex!["0e42eb6f65a8ef5e3f3c3cdb5b2c3be646e791abd76e2224d5847cde786b2e01"].into(),
        Some(vec![
            // 5CPQQYs3wf32fr5PhmmfFQEeVzD1Zy9Hdo8LFzQYuhP8XHW6
            hex!["0e42eb6f65a8ef5e3f3c3cdb5b2c3be646e791abd76e2224d5847cde786b2e01"].into(),
        ]),
        endowed_evm_account(),
    )
}

/// Iris testnet config.
pub fn iris_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Clover",
        "iris",
        ChainType::Custom(String::from("iris")),
        local_testnet_genesis,
        vec![
            "/dns/seed1.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWFtshqoFL1hAwseGc4WuFeREKicjFR15JiVEaJiHnDvn2"
              .parse()
              .unwrap(),
            "/dns/seed2.iris-cc2.clover.finance/tcp/30333/p2p/12D3KooWBcU1EShS2azLwQhKVKyeXU2cc3CWyhuN8wJwEKaRVNe8"
              .parse()
              .unwrap(),
          ],
    TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),

    Some("iris"),
        None,
        Some(
            json!({
              "tokenDecimals": 18,
              "tokenSymbol": "CLV"
            })
            .as_object()
            .expect("Created an object")
            .clone(),
        ),
        Default::default(),
    )
}

/// Helper function to create RuntimeGenesisConfig for testing.
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    endowed_eth_accounts: BTreeMap<H160, GenesisAccount>,
) -> RuntimeGenesisConfig {
    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
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
        ]
    });
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(x) {
                endowed_accounts.push(x.clone())
            }
        });

    // stakers: all validators and nominators.
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.0.clone(), x.0.clone(), STASH, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = initial_authorities.len();
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .into_iter()
                .map(|choice| choice.0.clone())
                .collect::<Vec<_>>();
            (
                x.clone(),
                x.clone(),
                STASH,
                StakerStatus::Nominator(nominations),
            )
        }))
        .collect::<Vec<_>>();

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 1_000 * DOLLARS;
    const STASH: Balance = 100 * DOLLARS;

    RuntimeGenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            ..Default::default()
        },
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect(),
        },
        evm: EVMConfig {
            accounts: endowed_eth_accounts,
            ..Default::default()
        },
        ethereum: EthereumConfig::default(),
        im_online: ImOnlineConfig { keys: vec![] },
        indices: IndicesConfig { indices: vec![] },
        session: SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        staking: StakingConfig {
            validator_count: initial_authorities.len() as u32,
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers,
            ..Default::default()
        },
        democracy: DemocracyConfig::default(),
        council: CouncilConfig::default(),
        technical_committee: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },
        sudo: SudoConfig {
            key: Some(root_key),
        },
        babe: BabeConfig {
            epoch_config: Some(clover_runtime::BABE_GENESIS_EPOCH_CONFIG),
            ..Default::default()
        },
        authority_discovery: Default::default(),
        grandpa: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        vesting: Default::default(),
        elections_phragmen: Default::default(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full_base, NewFullBase};
    use sc_service_test;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> RuntimeGenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            vec![],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            None,
            dev_endowed_evm_accounts(),
        )
    }

    /// Local testnet config (single validator - Alice).
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            None,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob).
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            None,
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        sp_tracing::try_init_simple();

        sc_service_test::connectivity(integration_test_config_with_two_authorities(), |config| {
            let NewFullBase {
                task_manager,
                client,
                network,
                sync,
                transaction_pool,
                ..
            } = new_full_base(config, false, |_, _| ())?;
            Ok(sc_service_test::TestNetComponents::new(
                task_manager,
                client,
                network,
                sync,
                transaction_pool,
            ))
        });
    }

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }

    #[test]
    fn test_staging_test_net_chain_spec() {
        staging_testnet_config().build_storage().unwrap();
    }
}
