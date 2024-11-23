#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use core::convert::{TryFrom, TryInto};
use frame_election_provider_support::{NoElection, VoteWeight};
use frame_support::traits::fungible::HoldConsideration;
use frame_support::traits::{
    EitherOfDiverse, EqualPrivilegeOnly, LinearStoragePrice, Nothing, WithdrawReasons,
};
use frame_support::weights::ConstantMultiplier;
use frame_support::PalletId;
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_session::historical as pallet_session_historical;
pub use pallet_transaction_payment::{
    FeeDetails, Multiplier, RuntimeDispatchInfo, TargetedFeeAdjustment,
};
use sp_api::impl_runtime_apis;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_core::crypto::KeyTypeId;
use sp_core::{ConstBool, ConstU32, ConstU64, ConstU8, OpaqueMetadata};
use sp_runtime::curve::PiecewiseLinear;
use sp_runtime::traits::{
    BlakeTwo256, Block as BlockT, Bounded, Convert, ConvertInto, NumberFor, OpaqueKeys,
    SaturatedConversion, StaticLookup,
};
use sp_runtime::transaction_validity::{
    TransactionPriority, TransactionSource, TransactionValidity,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult, FixedPointNumber, Percent,
    Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub use pallet_staking::StakerStatus;

use evm_accounts::EvmAddressMapping;
pub use frame_support::dispatch::DispatchClass;
pub use frame_support::traits::{
    Currency, FindAuthor, Imbalance, KeyOwnerProofSystem, LockIdentifier, OnUnbalanced, Randomness,
};
pub use frame_support::weights::constants::{
    BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
};
pub use frame_support::weights::Weight;
pub use frame_support::{
    construct_runtime, debug, ensure, parameter_types, transactional, ConsensusEngineId,
    StorageValue,
};
use frame_system::{limits, EnsureRoot, EnsureSigned, EnsureWithSuccess};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use parity_scale_codec::Encode;
pub use sp_runtime::{Perbill, Permill};

pub use clover_primitives::currency::*;
pub use clover_primitives::{
    AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, EraIndex, Hash, Index,
    Moment, Price, Rate, Share, Signature,
};

pub use constants::time::*;
use impls::{Author, MergeAccountEvm, WeightToFee};

mod constants;
mod impls;
mod mock;
mod tests;
mod voter_bags;
mod weights;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
  pub struct SessionKeys {
    pub grandpa: Grandpa,
    pub babe: Babe,
    pub im_online: ImOnline,
    pub authority_discovery: AuthorityDiscovery,
  }
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
    WASM_BINARY.expect(
        "Development wasm binary is not available. This means the client is built with \
		 `SKIP_WASM_BUILD` flag and it is only usable for production chains. Please rebuild with \
		 the flag disabled.",
    )
}

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
    };

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("clover"),
    impl_name: create_runtime_str!("clover"),
    state_version: 1,
    authoring_version: 1,
    spec_version: 18,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

/// We allow for 2000ms of compute with a 6 second average block time.
pub const WEIGHT_MILLISECS_PER_BLOCK: u64 = 2000;
/// We allow for 2 seconds of compute with a 6 second average block time, with maximum proof size.
pub const MAXIMUM_BLOCK_WEIGHT: Weight =
    Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_perthousand(25);

parameter_types! {
  pub BlockLength: limits::BlockLength =
    limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
  pub const BlockHashCount: BlockNumber = 2400;
  /// We allow for 2 seconds of compute with a 6 second average block time.
  pub BlockWeights: limits::BlockWeights = limits::BlockWeights::builder()
  .base_block(BlockExecutionWeight::get())
  .for_class(DispatchClass::all(), |weights| {
    weights.base_extrinsic = ExtrinsicBaseWeight::get();
  })
  .for_class(DispatchClass::Normal, |weights| {
    weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
  })
  .for_class(DispatchClass::Operational, |weights| {
    weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
    // Operational transactions have an extra reserved space, so that they
    // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
    weights.reserved = Some(
      MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT,
    );
  })
  .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
  .build_or_panic();
  pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
  pub const Version: RuntimeVersion = VERSION;
  pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = ();
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Indices;
    /// The index type for storing how many extrinsics an account has signed.
    type Nonce = Index;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    type BlockWeights = BlockWeights;
    type BlockLength = BlockLength;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = (evm_accounts::CallKillAccount<Runtime>,);
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    type SS58Prefix = SS58Prefix;
    type Block = Block;
    type MaxConsumers = ConstU32<16>;
    type OnSetCode = ();
}

parameter_types! {
  // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
  pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
  pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
  pub const ReportLongevity: u64 =
   BondingDuration::get() as u64 * SessionsPerEra::get() as u64 * EpochDuration::get();

  pub const MaxAuthorities: u32 = 100;
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<256>;
    type KeyOwnerProof =
        <Historical as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

    type EquivocationReportSystem =
        pallet_babe::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
    type WeightInfo = ();
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
  pub const MaxSetIdSessionEntries: u32 = BondingDuration::get() * SessionsPerEra::get();
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type KeyOwnerProof = <Historical as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type EquivocationReportSystem =
        pallet_grandpa::EquivocationReportSystem<Self, Offences, Historical, ReportLongevity>;
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = ConstU32<256>;
    type MaxSetIdSessionEntries = ConstU64<4>;
    type WeightInfo = ();
}

parameter_types! {
  pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
  pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
  pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
  pub const MaxSubAccounts: u32 = 100;
  pub const MaxAdditionalFields: u32 = 100;
  pub const MaxRegistrars: u32 = 20;
}

type EnsureRootOrHalfCouncil = EitherOfDiverse<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>,
>;

impl pallet_identity::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BasicDeposit = BasicDeposit;
    type FieldDeposit = FieldDeposit;
    type SubAccountDeposit = SubAccountDeposit;
    type MaxSubAccounts = MaxSubAccounts;
    type MaxAdditionalFields = MaxAdditionalFields;
    type MaxRegistrars = MaxRegistrars;
    type Slashed = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type RegistrarOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const MinVestedTransfer: Balance = 100 * DOLLARS;
  pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
  WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
  pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type EventHandler = (Staking, ImOnline);
}

parameter_types! {
  pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

/// clover account
impl evm_accounts::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type KillAccount = frame_system::Consumer<Runtime>;
    type AddressMapping = EvmAddressMapping<Runtime>;
    type MergeAccount = MergeAccountEvm;
    type WeightInfo = weights::evm_accounts::WeightInfo<Runtime>;
}

#[cfg(feature = "clover-mainnet")]
const CHAIN_ID: u64 = 1024;
#[cfg(feature = "clover-testnet")]
const CHAIN_ID: u64 = 1023;

parameter_types! {
  pub const ChainId: u64 = CHAIN_ID;
}

/// Struct that handles the conversion of Balance -> `u64`. This is used for
/// staking's election calculation.
pub struct CurrencyToVoteHandler;

impl Convert<u64, u64> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u64 {
        x
    }
}
impl Convert<u128, u128> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u128 {
        x
    }
}
impl Convert<u128, u64> for CurrencyToVoteHandler {
    fn convert(x: u128) -> u64 {
        x.saturated_into()
    }
}

impl Convert<u64, u128> for CurrencyToVoteHandler {
    fn convert(x: u64) -> u128 {
        x as u128
    }
}

pallet_staking_reward_curve::build! {
  const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
    min_inflation: 0_025_000,
    max_inflation: 0_100_000,
    ideal_stake: 0_500_000,
    falloff: 0_050_000,
    max_piece_count: 40,
    test_precision: 0_005_000,
  );
}

parameter_types! {
  // session: 10 minutes
  pub const SessionsPerEra: sp_staking::SessionIndex = 36;  // 36 sessions in an era, (6 hours)
  pub const BondingDuration: sp_staking::EraIndex = 48; // 48 era for unbouding (48 * 6 hours)
  pub const SlashDeferDuration: sp_staking::EraIndex = 24; // 1/2 bonding duration
  pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;
  pub const MaxNominatorRewardedPerValidator: u32 = 64;
  pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
  pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
  pub const MaxIterations: u32 = 10;
  // 0.05%. The higher the value, the more strict solution acceptance becomes.
  pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
  pub OffchainSolutionWeightLimit: Weight = BlockWeights::get()
    .get(DispatchClass::Normal)
    .max_extrinsic
    .expect("Normal extrinsics have weight limit configured by default; qed")
    .saturating_sub(BlockExecutionWeight::get());
    pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
  pub HistoryDepth: u32 = 84;
  pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * BlockWeights::get().max_block;
}

// pub struct OnChainSeqPhragmen;
// impl onchain::Config for OnChainSeqPhragmen {
// 	type System = Runtime;
// 	type Solver = SequentialPhragmen<
// 		AccountId,
// 		pallet_election_provider_multi_phase::SolutionAccuracyOf<Runtime>,
// 	>;
// 	type DataProvider = <Runtime as pallet_election_provider_multi_phase::Config>::DataProvider;
// 	type WeightInfo = frame_election_provider_support::weights::SubstrateWeight<Runtime>;
// 	type MaxWinners = <Runtime as pallet_election_provider_multi_phase::Config>::MaxWinners;
// 	type Bounds = ElectionBoundsOnChain;
// }

/// Upper limit on the number of NPOS nominations.
const MAX_QUOTA_NOMINATIONS: u32 = 16;

pub struct StakingBenchmarkingConfig;
impl pallet_staking::BenchmarkingConfig for StakingBenchmarkingConfig {
    type MaxNominators = ConstU32<1000>;
    type MaxValidators = ConstU32<1000>;
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type CurrencyBalance = Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
    type RewardRemainder = Treasury;
    type RuntimeEvent = RuntimeEvent;
    type Slash = Treasury;
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    /// A super-majority of the council can cancel the slash.
    type AdminOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
    >;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    /// Replace with multi-phase election provider
    type ElectionProvider = NoElection<(AccountId, BlockNumber, Staking, ConstU32<10>)>;
    /// We are way past genesis, no need to run election.
    /// TODO: check how it works in the local testnet
    type GenesisElectionProvider = NoElection<(AccountId, BlockNumber, Staking, ConstU32<10>)>;
    type VoterList = VoterList;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
    type NominationsQuota = pallet_staking::FixedNominationsQuota<MAX_QUOTA_NOMINATIONS>;
    type MaxUnlockingChunks = ConstU32<32>;
    type TargetList = pallet_staking::UseValidatorsMap<Self>;
    type HistoryDepth = HistoryDepth;
    type EventListeners = ();
    type BenchmarkingConfig = StakingBenchmarkingConfig;
}

parameter_types! {
    pub const BagThresholds: &'static [u64] = &voter_bags::THRESHOLDS;
}

type VoterBagsListInstance = pallet_bags_list::Instance1;
impl pallet_bags_list::Config<VoterBagsListInstance> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    /// The voter bags-list is loosely kept up to date, and the real source of truth for the score
    /// of each node is the staking pallet.
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
    type WeightInfo = pallet_bags_list::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const ExistentialDeposit: u128 = 0;
  pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type MaxLocks = MaxLocks;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type RuntimeHoldReason = RuntimeHoldReason;
    type MaxHolds = ConstU32<2>;
}

parameter_types! {
  pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_BLOCKS as _;
  pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
  pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type RuntimeEvent = RuntimeEvent;
    type ValidatorSet = Historical;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
    type NextSessionRotation = Babe;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

parameter_types! {
  pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MAXIMUM_BLOCK_WEIGHT;
}

impl pallet_offences::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

parameter_types! {
    pub const PreimageBaseDeposit: Balance = 1 * DOLLARS;
    pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ManagerOrigin = EnsureRoot<AccountId>;
    type Consideration = HoldConsideration<
        AccountId,
        Balances,
        PreimageHoldReason,
        LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
    >;
}

parameter_types! {
  pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * MAXIMUM_BLOCK_WEIGHT;
  pub const MaxScheduledPerBlock: u32 = 50;
}

// democracy
impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    #[cfg(feature = "runtime-benchmarks")]
    type MaxScheduledPerBlock = ConstU32<512>;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MaxScheduledPerBlock = ConstU32<50>;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type Preimages = Preimage;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const LaunchPeriod: BlockNumber = 7 * DAYS;
  pub const VotingPeriod: BlockNumber = 7 * DAYS;
  pub const FastTrackVotingPeriod: BlockNumber = 1 * DAYS;
  pub const MinimumDeposit: Balance = 100 * DOLLARS;
  pub const EnactmentPeriod: BlockNumber = 8 * DAYS;
  pub const CooloffPeriod: BlockNumber = 7 * DAYS;
  // One cent: $10,000 / MB
  pub const PreimageByteDeposit: Balance = 10 * MILLICENTS;
  pub const InstantAllowed: bool = false;
  pub const MaxVotes: u32 = 100;
  pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin =
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>;
    /// A super-majority can have the next scheduled referendum be a straight
    /// majority-carries vote.
    type ExternalMajorityOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 4, 5>;
    /// A unanimous council can have the next scheduled referendum be a straight
    /// default-carries (NTB) vote.
    type ExternalDefaultOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
    /// Full of the technical committee can have an
    /// ExternalMajority/ExternalDefault vote be tabled immediately and with a
    /// shorter voting/enactment period.
    type FastTrackOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
    type InstantOrigin = frame_system::EnsureNever<AccountId>;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    /// To cancel a proposal which has been passed, all of the council must
    /// agree to it.
    type CancellationOrigin =
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
    type CancelProposalOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
    >;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    /// Any single technical committee member may veto a coming council
    /// proposal, however they can only do it once and it lasts only for the
    /// cooloff period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type MaxVotes = MaxVotes;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
    type Preimages = Preimage;
    type MaxDeposits = ConstU32<100>;
    type MaxBlacklisted = ConstU32<100>;
    type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
    type SubmitOrigin = EnsureSigned<AccountId>;
}

impl pallet_utility::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
    type PalletsOrigin = OriginCaller;
}

parameter_types! {
  // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
  pub const DepositBase: Balance = deposit(1, 88);
  // Additional storage item size of 32 bytes.
  pub const DepositFactor: Balance = deposit(0, 32);
  pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
  pub const CouncilMaxProposals: u32 = 100;
  pub const GeneralCouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = GeneralCouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxCollectivesProposalWeight;
}

/// Converter for currencies to votes.
pub struct CurrencyToVoteHandler2<R>(sp_std::marker::PhantomData<R>);

impl<R> CurrencyToVoteHandler2<R>
where
    R: pallet_balances::Config,
    R::Balance: Into<u128>,
{
    fn factor() -> u128 {
        let issuance: u128 = <pallet_balances::Pallet<R>>::total_issuance().into();
        (issuance / u64::max_value() as u128).max(1)
    }
}

impl<R> Convert<u128, u64> for CurrencyToVoteHandler2<R>
where
    R: pallet_balances::Config,
    R::Balance: Into<u128>,
{
    fn convert(x: u128) -> u64 {
        (x / Self::factor()) as u64
    }
}

impl<R> Convert<u128, u128> for CurrencyToVoteHandler2<R>
where
    R: pallet_balances::Config,
    R::Balance: Into<u128>,
{
    fn convert(x: u128) -> u128 {
        x * Self::factor()
    }
}

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 15 * CENTS + (bytes as Balance) * 6 * CENTS
}

parameter_types! {
  pub const CandidacyBond: Balance = 1 * DOLLARS;
  // 1 storage item created, key size is 32 bytes, value size is 16+16.
  pub const VotingBondBase: Balance = deposit(1, 64);
  // additional data per vote is 32 bytes (account id).
  pub const VotingBondFactor: Balance = deposit(0, 32);
  /// Daily council elections.
  pub const TermDuration: BlockNumber = 3 * DAYS;
  pub const DesiredMembers: u32 = 7;
  pub const DesiredRunnersUp: u32 = 30;
  pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
  pub const MaxVotesPerVoter: u32 = 16;
}

impl pallet_elections_phragmen::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type ChangeMembers = Council;
    type InitializeMembers = Council;
    type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = Treasury;
    type KickedMember = Treasury;
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
    type PalletId = ElectionsPhragmenModuleId;
    type MaxCandidates = ConstU32<100>;
    type MaxVoters = ConstU32<100>;
    type MaxVotesPerVoter = MaxVotesPerVoter;
}

parameter_types! {
  pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
  pub const TechnicalMaxProposals: u32 = 100;
  pub const TechnicalMaxMembers:u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type MaxProposalWeight = MaxCollectivesProposalWeight;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddOrigin = frame_system::EnsureRoot<AccountId>;
    type RemoveOrigin = frame_system::EnsureRoot<AccountId>;
    type SwapOrigin = frame_system::EnsureRoot<AccountId>;
    type ResetOrigin = frame_system::EnsureRoot<AccountId>;
    type PrimeOrigin = frame_system::EnsureRoot<AccountId>;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const ProposalBond: Permill = Permill::from_percent(5);
  pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
  pub const SpendPeriod: BlockNumber = 1 * DAYS;
  pub const Burn: Permill = Permill::from_percent(1);
  pub const TreasuryModuleId: PalletId = PalletId(*b"py/trsry");

  pub const TipCountdown: BlockNumber = 1 * DAYS;
  pub const TipFindersFee: Percent = Percent::from_percent(20);
  pub const TipReportDepositBase: Balance = 1 * DOLLARS;
  pub const DataDepositPerByte: Balance = 10 * MILLICENTS;

  pub const MaximumReasonLength: u32 = 16384;
  pub const BountyDepositBase: Balance = 1 * DOLLARS;
  pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
  pub const BountyUpdatePeriod: BlockNumber = 7 * DAYS;
  pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
  pub const BountyValueMinimum: Balance = 5 * DOLLARS;

  pub const MaxApprovals: u32 = 100;
  pub const MaxBalance: Balance = Balance::max_value();
}

impl pallet_treasury::Config for Runtime {
    type Currency = Balances;
    type ApproveOrigin =
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 2>;
    type RejectOrigin =
        pallet_collective::EnsureProportionMoreThan<AccountId, CouncilCollective, 1, 5>;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type BurnDestination = ();
    type SpendFunds = Bounties;
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type PalletId = TreasuryModuleId;
    type MaxApprovals = MaxApprovals;
    type SpendOrigin = EnsureWithSuccess<EnsureRoot<AccountId>, AccountId, MaxBalance>;
    type ProposalBondMaximum = ();
}

parameter_types! {
  pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
    pub const CuratorDepositMin: Balance = 1 * DOLLARS;
    pub const CuratorDepositMax: Balance = 100 * DOLLARS;
}

impl pallet_bounties::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type CuratorDepositMultiplier = CuratorDepositMultiplier;
    type CuratorDepositMin = CuratorDepositMin;
    type CuratorDepositMax = CuratorDepositMax;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
    type ChildBountyManager = ();
}

impl pallet_tips::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type Tippers = ElectionsPhragmen;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 80% to treasury, 20% to author
            let mut split = fees.ration(80, 20);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 80% to treasury, 20% to author (though this can be anything)
                tips.ration_merge_into(80, 20, &mut split);
            }
            Treasury::on_unbalanced(split.0);
            Author::on_unbalanced(split.1);
        }
    }
}

parameter_types! {
  pub const TransactionByteFee: Balance = MILLICENTS;
  pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
  pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
  pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub MaximumMultiplier: Multiplier = Bounded::max_value();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, DealWithFees>;
    type WeightToFee = WeightToFee<Balance>;
    type FeeMultiplierUpdate = TargetedFeeAdjustment<
        Self,
        TargetBlockFullness,
        AdjustmentVariable,
        MinimumMultiplier,
        MaximumMultiplier,
    >;
    type OperationalFeeMultiplier = ConstU8<5>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
    type AccountIndex = AccountIndex;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Deposit = IndexDeposit;
    type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    RuntimeCall: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: RuntimeCall,
        public: <Signature as sp_runtime::traits::Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(
        RuntimeCall,
        <UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
    )> {
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let tip = 0;
        let extra: SignedExtra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra)
            .map_err(|e| {
                log::warn!("Unable to create signed payload: {:?}", e);
            })
            .ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Indices::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature, extra)))
    }
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as sp_runtime::traits::Verify>::Signer;
    type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    RuntimeCall: From<C>,
{
    type OverarchingCall = RuntimeCall;
    type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
  pub const TombstoneDeposit: Balance = 16 * MILLICENTS;
  pub const SurchargeReward: Balance = 150 * MILLICENTS;
  pub const SignedClaimHandicap: u32 = 2;
  pub const MaxDepth: u32 = 32;
  pub const MaxValueSize: u32 = 16 * 1024;
  pub const RentByteFee: Balance = 4 * MILLICENTS;
  pub const RentDepositOffset: Balance = 1000 * MILLICENTS;
  pub const DepositPerContract: Balance = TombstoneDeposit::get();
  pub const DepositPerStorageByte: Balance = deposit(0, 1);
  pub const DepositPerStorageItem: Balance = deposit(1, 0);
  pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
  pub CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(30);
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
    type RuntimeCall = RuntimeCall;
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type RuntimeEvent = RuntimeEvent;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = Nothing;
    type DepositPerItem = DepositPerStorageItem;
    type DepositPerByte = DepositPerStorageByte;
    type DefaultDepositLimit = DefaultDepositLimit;
    type CallStack = [pallet_contracts::Frame<Self>; 5];
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
    type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
    type MaxStorageKeyLen = ConstU32<128>;
    type UnsafeUnstableInterface = ConstBool<false>;
    type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
    type RuntimeHoldReason = RuntimeHoldReason;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type Migrations = ();
    #[cfg(feature = "runtime-benchmarks")]
    type Migrations = pallet_contracts::migration::codegen::BenchMigrations;
    type MaxDelegateDependencies = ConstU32<32>;
    type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
    type Debug = ();
    type Environment = ();
}

parameter_types! {
  pub Prefix: &'static [u8] = b"Pay CLVs to the Clover account:";
  pub const ClaimsModuleId: PalletId = PalletId(*b"clvclaim");
}

impl clover_claims::Config for Runtime {
    type ModuleId = ClaimsModuleId;
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type Prefix = Prefix;
}

parameter_types! {
  pub const GetStableCurrencyId: CurrencyId = CurrencyId::CUSDT;
  pub StableCurrencyFixedPrice: Price = Price::saturating_from_rational(1, 1);
  pub const MinimumCount: u32 = 1;
  pub const ExpiresIn: Moment = 1000 * 60 * 60; // 60 mins
  pub ZeroAccountId: AccountId = AccountId::from([0u8; 32]);
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
    pub StatementCost: Balance = 1 * DOLLARS;
    pub StatementByteCost: Balance = 100 * MILLICENTS;
    pub const MinAllowedStatements: u32 = 4;
    pub const MaxAllowedStatements: u32 = 10;
    pub const MinAllowedBytes: u32 = 1024;
    pub const MaxAllowedBytes: u32 = 4096;
}

impl pallet_statement::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type StatementCost = StatementCost;
    type ByteCost = StatementByteCost;
    type MinAllowedStatements = MinAllowedStatements;
    type MaxAllowedStatements = MaxAllowedStatements;
    type MinAllowedBytes = MinAllowedBytes;
    type MaxAllowedBytes = MaxAllowedBytes;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
  pub struct Runtime {
    System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
    RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip::{Pallet, Storage},
    Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},

    Authorship: pallet_authorship::{Pallet, Storage},
    Babe: pallet_babe::{Pallet, Call, Storage, Config<T>, ValidateUnsigned},
    Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config<T>, Event},

    Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>},
    Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
    TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},

    Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
    Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
    Historical: pallet_session_historical::{Pallet},

    // Governance.
    Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
    Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
    TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
    ElectionsPhragmen: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>},
    TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
    Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>, Config<T>},

    // Smart contracts modules
    Contracts: pallet_contracts,

    Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},

    ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
    AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config<T>},
    Offences: pallet_offences::{Pallet, Storage, Event},

    // Utility module.
    Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
    Utility: pallet_utility::{Pallet, Call, Event},

    Identity: pallet_identity::{Pallet, Call, Storage, Event<T>},
    Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>},

    Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>},

    Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>},
    Tips: pallet_tips::{Pallet, Call, Storage, Event<T>},

    // account module
    EvmAccounts: evm_accounts::{Pallet, Call, Storage, Event<T>},

    CloverClaims: clover_claims::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},

    VoterList: pallet_bags_list::<Instance1>,
    Preimage: pallet_preimage,
    Statement: pallet_statement,
  }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
impl_runtime_apis! {
  impl sp_api::Core<Block> for Runtime {
    fn version() -> RuntimeVersion {
      VERSION
    }

    fn execute_block(block: Block) {
      Executive::execute_block(block)
    }

    fn initialize_block(header: &<Block as BlockT>::Header) {
      Executive::initialize_block(header)
    }
  }

  impl sp_api::Metadata<Block> for Runtime {
    fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
    }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> sp_std::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
  }

  impl sp_block_builder::BlockBuilder<Block> for Runtime {
    fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
      Executive::apply_extrinsic(extrinsic)
    }

    fn finalize_block() -> <Block as BlockT>::Header {
      Executive::finalize_block()
    }

    fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
      data.create_extrinsics()
    }

    fn check_inherents(
      block: Block,
      data: sp_inherents::InherentData,
    ) -> sp_inherents::CheckInherentsResult {
      data.check_extrinsics(&block)
    }
  }

  impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
    fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
    ) -> TransactionValidity {
      Executive::validate_transaction(source, tx, block_hash)
    }
  }

  impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
    fn offchain_worker(header: &<Block as BlockT>::Header) {
      Executive::offchain_worker(header)
    }
  }

  impl sp_session::SessionKeys<Block> for Runtime {
    fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
      SessionKeys::generate(seed)
    }

    fn decode_session_keys(
      encoded: Vec<u8>,
    ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
      SessionKeys::decode_into_raw_public_keys(&encoded)
    }
  }

  impl sp_statement_store::runtime_api::ValidateStatement<Block> for Runtime {
        fn validate_statement(
            source: sp_statement_store::runtime_api::StatementSource,
            statement: sp_statement_store::Statement,
        ) -> Result<sp_statement_store::runtime_api::ValidStatement, sp_statement_store::runtime_api::InvalidStatement> {
            Statement::validate_statement(source, statement)
        }
    }

  impl sp_consensus_babe::BabeApi<Block> for Runtime {
    fn configuration() -> sp_consensus_babe::BabeConfiguration {
      sp_consensus_babe::BabeConfiguration {
        slot_duration: Babe::slot_duration(),
        epoch_length: EpochDuration::get(),
        c: PRIMARY_PROBABILITY,
        authorities: Babe::authorities().to_vec(),
        randomness: Babe::randomness(),
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
      }
    }

    fn current_epoch_start() -> sp_consensus_babe::Slot {
      Babe::current_epoch_start()
    }

    fn current_epoch() -> sp_consensus_babe::Epoch {
      Babe::current_epoch()
    }

    fn next_epoch() -> sp_consensus_babe::Epoch {
      Babe::next_epoch()
    }

    fn generate_key_ownership_proof(
      _slot_number: sp_consensus_babe::Slot,
      authority_id: sp_consensus_babe::AuthorityId,
      ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
      use parity_scale_codec::Encode;

      Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
        .map(|p| p.encode())
        .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
    }

    fn submit_report_equivocation_unsigned_extrinsic(
      equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
      key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
      ) -> Option<()> {
      let key_owner_proof = key_owner_proof.decode()?;

      Babe::submit_unsigned_equivocation_report(
        equivocation_proof,
        key_owner_proof,
        )
    }
  }

  impl fg_primitives::GrandpaApi<Block> for Runtime {
    fn grandpa_authorities() -> GrandpaAuthorityList {
      Grandpa::grandpa_authorities()
    }

    fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

    fn submit_report_equivocation_unsigned_extrinsic(
      _equivocation_proof: fg_primitives::EquivocationProof<
        <Block as BlockT>::Hash,
        NumberFor<Block>,
      >,
      _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
    ) -> Option<()> {
      None
    }

    fn generate_key_ownership_proof(
      _set_id: fg_primitives::SetId,
      _authority_id: GrandpaId,
    ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
      // NOTE: this is the only implementation possible since we've
      // defined our key owner proof type as a bottom type (i.e. a type
      // with no values).
      None
    }
  }

  impl sp_authority_discovery::AuthorityDiscoveryApi<Block> for Runtime {
    fn authorities() -> Vec<AuthorityDiscoveryId> {
      AuthorityDiscovery::authorities()
    }
  }

  impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
    fn account_nonce(account: AccountId) -> Index {
      System::account_nonce(account)
    }
  }

  // impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber>
  //   for Runtime
  // {
  //   fn call(
  //     origin: AccountId,
  //     dest: AccountId,
  //     value: Balance,
  //     gas_limit: u64,
  //     input_data: Vec<u8>,
  //   ) -> pallet_contracts_primitives::ContractExecResult {
  //       Contracts::bare_call(origin, dest.into(), value, gas_limit, input_data)
  //   }

  //   fn get_storage(
  //     address: AccountId,
  //     key: [u8; 32],
  //   ) -> pallet_contracts_primitives::GetStorageResult {
  //     Contracts::get_storage(address, key)
  //   }

  //   fn rent_projection(
  //     address: AccountId,
  //   ) -> pallet_contracts_primitives::RentProjectionResult<BlockNumber> {
  //     Contracts::rent_projection(address)
  //   }
  // }

  impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
      Block,
      Balance,
    > for Runtime {
      fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
        TransactionPayment::query_info(uxt, len)
      }
      fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
        TransactionPayment::query_fee_details(uxt, len)
      }
      fn query_weight_to_fee(weight: Weight) -> Balance {
        TransactionPayment::weight_to_fee(weight)
      }
      fn query_length_to_fee(length: u32) -> Balance {
        TransactionPayment::length_to_fee(length)
      }
    }
}
