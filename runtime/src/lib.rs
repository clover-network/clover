#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::Decode;
use sp_arithmetic::PerThing;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::curve::PiecewiseLinear;
use sp_runtime::traits::{
  BlakeTwo256, Block as BlockT, Convert, ConvertInto, Dispatchable, PostDispatchInfoOf,
  SaturatedConversion, StaticLookup, UniqueSaturatedInto, Verify,
};
use sp_runtime::{
  create_runtime_str, generic, impl_opaque_keys,
  transaction_validity::{
    TransactionPriority, TransactionSource, TransactionValidity, TransactionValidityError,
  },
  ApplyExtrinsicResult, FixedPointNumber, OpaqueExtrinsic, Percent, Perquintill,
};
use sp_std::{marker::PhantomData, prelude::*};

use sp_api::impl_runtime_apis;

use pallet_contracts::weights::WeightInfo;
use pallet_ethereum::{Call::transact, Transaction as EthereumTransaction};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
pub use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_core::u32_trait::{_1, _2, _4, _5};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

pub use pallet_staking::StakerStatus;

use codec::Encode;
use evm_accounts::EvmAddressMapping;
use fp_rpc::TransactionStatus;
pub use frame_support::{
  construct_runtime, debug, ensure, log, parameter_types,
  traits::{
    Currency, EnsureOneOf, EqualPrivilegeOnly, Everything, FindAuthor, Imbalance,
    KeyOwnerProofSystem, LockIdentifier, Nothing, Randomness, U128CurrencyToVote,
  },
  transactional,
  weights::{
    constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
    DispatchClass, Weight,
  },
  ConsensusEngineId, PalletId, StorageValue,
};
use frame_system::{limits, EnsureRoot};
pub use pallet_balances::Call as BalancesCall;
use pallet_evm::{Account as EVMAccount, EnsureAddressTruncated, FeeCalculator, Runner};
pub use pallet_timestamp::Call as TimestampCall;
pub use sp_runtime::{Perbill, Permill, RuntimeAppPublic};

pub use primitives::{
  currency::*, AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, EraIndex, Hash,
  Index, Moment, Price, Rate, Share, Signature,
};

// Polkadot imports
use pallet_xcm::{EnsureXcm, IsMajorityOfBody};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use xcm::latest::BodyId;
use xcm_executor::XcmExecutor;

pub use constants::time::*;
use impls::{MergeAccountEvm, WeightToFee};
use xcm_config::{DotLocation, XcmConfig, XcmOriginToTransactDispatchOrigin};

mod clover_evm_config;
mod constants;
mod impls;
mod mock;
mod parachains_common;
mod tests;
mod weights;

mod precompiles;
use precompiles::CloverPrecompiles;

mod xcm_config;

pub type AuraId = sp_consensus_aura::sr25519::AuthorityId;

pub type AssetId = u32;

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
    pub aura: Aura,
  }
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
  spec_name: create_runtime_str!("clover-mainnet"),
  impl_name: create_runtime_str!("clover-mainnet"),
  authoring_version: 1,
  spec_version: 20,
  impl_version: 1,
  apis: RUNTIME_API_VERSIONS,
  transaction_version: 1,
  state_version: 0,
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

pub const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;
pub const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
pub const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_perthousand(25);

parameter_types! {
  pub BlockLength: limits::BlockLength =
    limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
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
  pub const SS58Prefix: u8 = 128;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
  /// The basic call filter to use in dispatchable.
  type BaseCallFilter = Everything;
  /// The identifier used to distinguish between accounts.
  type AccountId = AccountId;
  /// The aggregated dispatch type that is available for extrinsics.
  type Call = Call;
  /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
  type Lookup = Indices;
  /// The index type for storing how many extrinsics an account has signed.
  type Index = Index;
  /// The index type for blocks.
  type BlockNumber = BlockNumber;
  /// The type for hashing blocks and tries.
  type Hash = Hash;
  /// The hashing algorithm used.
  type Hashing = BlakeTwo256;
  /// The header type.
  type Header = generic::Header<BlockNumber, BlakeTwo256>;
  /// The ubiquitous event type.
  type Event = Event;
  /// The ubiquitous origin type.
  type Origin = Origin;
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
  type OnKilledAccount = (
    // pallet_evm::CallKillAccount<Runtime>,
    evm_accounts::CallKillAccount<Runtime>,
  );
  /// The data to be stored in an account.
  type AccountData = pallet_balances::AccountData<Balance>;
  /// Weight information for the extrinsics of this pallet.
  type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
  type SS58Prefix = SS58Prefix;
  /// The set code logic of the parachain.
  type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
  type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
  pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
  pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
  pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
  pub const MaxSubAccounts: u32 = 100;
  pub const MaxAdditionalFields: u32 = 100;
  pub const MaxRegistrars: u32 = 20;
}

type EnsureRootOrHalfCouncil = EnsureOneOf<
  EnsureRoot<AccountId>,
  pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>,
>;

impl pallet_identity::Config for Runtime {
  type Event = Event;
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
}

impl pallet_vesting::Config for Runtime {
  type Event = Event;
  type Currency = Balances;
  type BlockNumberToBalance = ConvertInto;
  type MinVestedTransfer = MinVestedTransfer;
  type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
  const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
  pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
  /// A timestamp: milliseconds since the unix epoch.
  type Moment = u64;
  type OnTimestampSet = ();
  type MinimumPeriod = MinimumPeriod;
  type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const UncleGenerations: BlockNumber = 0;
}

impl pallet_authorship::Config for Runtime {
  type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
  type UncleGenerations = UncleGenerations;
  type FilterUncle = ();
  type EventHandler = ();
}

/// clover account
impl evm_accounts::Config for Runtime {
  type Event = Event;
  type Currency = Balances;
  type KillAccount = frame_system::Consumer<Runtime>;
  type AddressMapping = EvmAddressMapping<Runtime>;
  type MergeAccount = MergeAccountEvm;
  type WeightInfo = weights::evm_accounts::WeightInfo<Runtime>;
}

/// clover evm
pub struct FixedGasPrice;

impl FeeCalculator for FixedGasPrice {
  fn min_gas_price() -> U256 {
    50_000_000_000u64.into()
  }
}

const CHAIN_ID: u64 = 1024;

parameter_types! {
  pub const ChainId: u64 = CHAIN_ID;
}

static CLOVER_EVM_CONFIG: evm::Config = clover_evm_config::CloverEvmConfig::config();

parameter_types! {
  pub BlockGasLimit: U256 = U256::from(30_000_000); // double the ethereum block limit
  pub PrecompilesValue: CloverPrecompiles<Runtime> = CloverPrecompiles::<_>::new();
}

impl pallet_evm::Config for Runtime {
  type FeeCalculator = FixedGasPrice;
  type BlockHashMapping = pallet_ethereum::EthereumBlockHashMapping<Self>;
  type GasWeightMapping = ();
  type CallOrigin = EnsureAddressTruncated;
  type WithdrawOrigin = EnsureAddressTruncated;
  type AddressMapping = EvmAddressMapping<Runtime>;
  type Currency = Balances;
  type Event = Event;
  type Runner = pallet_evm::runner::stack::Runner<Self>;
  type PrecompilesType = CloverPrecompiles<Self>;
  type PrecompilesValue = PrecompilesValue;
  type ChainId = ChainId;
  type FindAuthor = EthereumFindAuthor<Aura>;
  type BlockGasLimit = BlockGasLimit;
  type OnChargeTransaction = ();
  fn config() -> &'static evm::Config {
    &CLOVER_EVM_CONFIG
  }
}

pub struct EthereumFindAuthor<F>(PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for EthereumFindAuthor<F> {
  fn find_author<'a, I>(digests: I) -> Option<H160>
  where
    I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
  {
    if let Some(author_index) = F::find_author(digests) {
      let authority_id = Aura::authorities()[author_index as usize].clone();
      return Some(H160::from_slice(&authority_id.to_raw_vec()[4..24]));
    }
    None
  }
}

/// parachain doesn't have an authorship support currently
pub struct PhantomMockAuthorship;

impl FindAuthor<u32> for PhantomMockAuthorship {
  fn find_author<'a, I>(_digests: I) -> Option<u32>
  where
    I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
  {
    Some(0 as u32)
  }
}

impl pallet_ethereum::Config for Runtime {
  type Event = Event;
  // type FindAuthor = EthereumFindAuthor<PhantomMockAuthorship>;
  type StateRoot = pallet_ethereum::IntermediateStateRoot;
}

pub struct TransactionConverter;

impl fp_rpc::ConvertTransaction<UncheckedExtrinsic> for TransactionConverter {
  fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> UncheckedExtrinsic {
    UncheckedExtrinsic::new_unsigned(
      pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
    )
  }
}

impl fp_rpc::ConvertTransaction<OpaqueExtrinsic> for TransactionConverter {
  fn convert_transaction(&self, transaction: pallet_ethereum::Transaction) -> OpaqueExtrinsic {
    let extrinsic = UncheckedExtrinsic::new_unsigned(
      pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
    );
    let encoded = extrinsic.encode();
    OpaqueExtrinsic::decode(&mut &encoded[..]).expect("Encoded extrinsic is always valid")
  }
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
  pub const SessionsPerEra: sp_staking::SessionIndex = 36;  // 18 sessions in an era, (6 hours)
  pub const BondingDuration: sp_staking::EraIndex = 24; // 24 era for unbouding (24 * 6 hours)
  pub const SlashDeferDuration: sp_staking::EraIndex = 12; // 1/2 bonding duration
  pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;
  pub const MaxNominatorRewardedPerValidator: u32 = 64;
  pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
  pub const MaxIterations: u32 = 10;
  // 0.05%. The higher the value, the more strict solution acceptance becomes.
  pub MinSolutionScoreBump: Perbill = PerThing::from_rational(5u32, 10_000);
  pub OffchainSolutionWeightLimit: Weight = BlockWeights::get()
    .get(DispatchClass::Normal)
    .max_extrinsic
    .expect("Normal extrinsics have weight limit configured by default; qed")
    .saturating_sub(BlockExecutionWeight::get());
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
  type Event = Event;
  type DustRemoval = ();
  type ExistentialDeposit = ExistentialDeposit;
  type AccountStore = System;
  type MaxLocks = MaxLocks;
  type MaxReserves = MaxReserves;
  type ReserveIdentifier = [u8; 8];
  type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_BLOCKS as _;
  pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
}

parameter_types! {
  pub MaximumSchedulerWeight: Weight = Perbill::from_percent(10) * MAXIMUM_BLOCK_WEIGHT;
  pub const MaxScheduledPerBlock: u32 = 50;
    // Retry a scheduled item every 10 blocks (2 minute) until the preimage exists.
  pub const NoPreimagePostponement: Option<u32> = Some(10);
}

// democracy
impl pallet_scheduler::Config for Runtime {
  type Event = Event;
  type Origin = Origin;
  type Call = Call;
  type MaximumWeight = MaximumSchedulerWeight;
  type PalletsOrigin = OriginCaller;
  type ScheduleOrigin = EnsureRoot<AccountId>;
  type MaxScheduledPerBlock = MaxScheduledPerBlock;
  type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
  type OriginPrivilegeCmp = EqualPrivilegeOnly;
  type PreimageProvider = ();
  type NoPreimagePostponement = NoPreimagePostponement;
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
  type Proposal = Call;
  type Event = Event;
  type Currency = Balances;
  type EnactmentPeriod = EnactmentPeriod;
  type VoteLockingPeriod = EnactmentPeriod;
  type LaunchPeriod = LaunchPeriod;
  type VotingPeriod = VotingPeriod;
  type MinimumDeposit = MinimumDeposit;
  /// A straight majority of the council can decide what their next motion is.
  type ExternalOrigin =
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
  /// A super-majority can have the next scheduled referendum be a straight
  /// majority-carries vote.
  type ExternalMajorityOrigin =
    pallet_collective::EnsureProportionAtLeast<_4, _5, AccountId, CouncilCollective>;
  /// A unanimous council can have the next scheduled referendum be a straight
  /// default-carries (NTB) vote.
  type ExternalDefaultOrigin =
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
  /// Full of the technical committee can have an
  /// ExternalMajority/ExternalDefault vote be tabled immediately and with a
  /// shorter voting/enactment period.
  type FastTrackOrigin =
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
  type InstantOrigin = frame_system::EnsureNever<AccountId>;
  type InstantAllowed = InstantAllowed;
  type FastTrackVotingPeriod = FastTrackVotingPeriod;
  /// To cancel a proposal which has been passed, all of the council must
  /// agree to it.
  type CancellationOrigin =
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
  type CancelProposalOrigin = EnsureOneOf<
    EnsureRoot<AccountId>,
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
  >;
  type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
  type BlacklistOrigin = EnsureRoot<AccountId>;
  /// Any single technical committee member may veto a coming council
  /// proposal, however they can only do it once and it lasts only for the
  /// cooloff period.
  type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
  type CooloffPeriod = CooloffPeriod;
  type PreimageByteDeposit = PreimageByteDeposit;
  type Slash = Treasury;
  type Scheduler = Scheduler;
  type MaxVotes = MaxVotes;
  type PalletsOrigin = OriginCaller;
  type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
  type MaxProposals = MaxProposals;
}

parameter_types! {
  pub const AssetDeposit: Balance = 10_000 * DOLLARS; // 10_000 DOLLARS deposit to create asset
  pub const AssetAccountDeposit: Balance = deposit(1, 16);
  pub const ApprovalDeposit: Balance = 0;
  pub const AssetsStringLimit: u32 = 50;
  /// Key = 32 bytes, Value = 36 bytes (32+1+1+1+1)
  // https://github.com/paritytech/substrate/blob/069917b/frame/assets/src/lib.rs#L257L271
  pub const MetadataDepositBase: Balance = deposit(1, 68);
  pub const MetadataDepositPerByte: Balance = deposit(0, 1);
  pub const ExecutiveBody: BodyId = BodyId::Executive;
}

/// We allow root and the Relay Chain council to execute privileged asset operations.
pub type AssetsForceOrigin =
  EnsureOneOf<EnsureRoot<AccountId>, EnsureXcm<IsMajorityOfBody<DotLocation, ExecutiveBody>>>;

impl pallet_assets::Config for Runtime {
  type Event = Event;
  type Balance = Balance;
  type AssetId = AssetId;
  type Currency = Balances;
  type ForceOrigin = AssetsForceOrigin;
  type AssetDeposit = AssetDeposit;
  type MetadataDepositBase = MetadataDepositBase;
  type MetadataDepositPerByte = MetadataDepositPerByte;
  type ApprovalDeposit = ApprovalDeposit;
  type StringLimit = AssetsStringLimit;
  type Freezer = ();
  type Extra = ();
  type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
  type AssetAccountDeposit = AssetAccountDeposit;
}

impl pallet_utility::Config for Runtime {
  type Event = Event;
  type Call = Call;
  type PalletsOrigin = OriginCaller;
  type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
  pub const DepositBase: Balance = deposit(1, 88);
  // Additional storage item size of 32 bytes.
  pub const DepositFactor: Balance = deposit(0, 32);
  pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
  type Event = Event;
  type Call = Call;
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
  type Origin = Origin;
  type Proposal = Call;
  type Event = Event;
  type MotionDuration = CouncilMotionDuration;
  type MaxProposals = CouncilMaxProposals;
  type MaxMembers = GeneralCouncilMaxMembers;
  type DefaultVote = pallet_collective::PrimeDefaultVote;
  type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
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
}

impl pallet_elections_phragmen::Config for Runtime {
  type Event = Event;
  type Currency = Balances;
  type ChangeMembers = Council;
  type InitializeMembers = Council;
  type CurrencyToVote = U128CurrencyToVote;
  type CandidacyBond = CandidacyBond;
  type VotingBondBase = VotingBondBase;
  type VotingBondFactor = VotingBondFactor;
  type LoserCandidate = Treasury;
  type KickedMember = Treasury;
  type DesiredMembers = DesiredMembers;
  type DesiredRunnersUp = DesiredRunnersUp;
  type TermDuration = TermDuration;
  type PalletId = ElectionsPhragmenModuleId;
  type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
  pub const TechnicalMaxProposals: u32 = 100;
  pub const TechnicalMaxMembers:u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
  type Origin = Origin;
  type Proposal = Call;
  type Event = Event;
  type MotionDuration = TechnicalMotionDuration;
  type MaxProposals = TechnicalMaxProposals;
  type MaxMembers = TechnicalMaxMembers;
  type DefaultVote = pallet_collective::PrimeDefaultVote;
  type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
  type Event = Event;
  type AddOrigin = frame_system::EnsureRoot<AccountId>;
  type RemoveOrigin = frame_system::EnsureRoot<AccountId>;
  type SwapOrigin = frame_system::EnsureRoot<AccountId>;
  type ResetOrigin = frame_system::EnsureRoot<AccountId>;
  type PrimeOrigin = frame_system::EnsureRoot<AccountId>;
  type MaxMembers = GeneralCouncilMaxMembers;
  type MembershipInitialized = TechnicalCommittee;
  type MembershipChanged = TechnicalCommittee;
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
}

impl pallet_treasury::Config for Runtime {
  type Currency = Balances;
  type ApproveOrigin =
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
  type RejectOrigin =
    pallet_collective::EnsureProportionMoreThan<_1, _5, AccountId, CouncilCollective>;
  type Event = Event;
  type OnSlash = ();
  type ProposalBond = ProposalBond;
  type ProposalBondMinimum = ProposalBondMinimum;
  type ProposalBondMaximum = ();
  type SpendPeriod = SpendPeriod;
  type Burn = Burn;
  type BurnDestination = ();
  type SpendFunds = Bounties;
  type PalletId = TreasuryModuleId;
  type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
  type MaxApprovals = MaxApprovals;
}

impl pallet_bounties::Config for Runtime {
  type Event = Event;
  type BountyDepositBase = BountyDepositBase;
  type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
  type BountyUpdatePeriod = BountyUpdatePeriod;
  type BountyCuratorDeposit = BountyCuratorDeposit;
  type BountyValueMinimum = BountyValueMinimum;
  type DataDepositPerByte = DataDepositPerByte;
  type MaximumReasonLength = MaximumReasonLength;
  type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
  type ChildBountyManager = ();
}

impl pallet_tips::Config for Runtime {
  type Event = Event;
  type DataDepositPerByte = DataDepositPerByte;
  type MaximumReasonLength = MaximumReasonLength;
  type Tippers = ElectionsPhragmen;
  type TipCountdown = TipCountdown;
  type TipFindersFee = TipFindersFee;
  type TipReportDepositBase = TipReportDepositBase;
  type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
  pub const TransactionByteFee: Balance = MILLICENTS;
  pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
  pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
  pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
  pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Runtime {
  type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
  type TransactionByteFee = TransactionByteFee;
  type OperationalFeeMultiplier = OperationalFeeMultiplier;
  type WeightToFee = WeightToFee<Balance>;
  type FeeMultiplierUpdate =
    TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

impl pallet_sudo::Config for Runtime {
  type Event = Event;
  type Call = Call;
}

parameter_types! {
  pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
  type AccountIndex = AccountIndex;
  type Event = Event;
  type Currency = Balances;
  type Deposit = IndexDeposit;
  type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
  Call: From<LocalCall>,
{
  fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
    call: Call,
    public: <Signature as sp_runtime::traits::Verify>::Signer,
    account: AccountId,
    nonce: Index,
  ) -> Option<(
    Call,
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
  Call: From<C>,
{
  type OverarchingCall = Call;
  type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
  pub const DepositPerItem: Balance = deposit(1, 0);
  pub const DepositPerByte: Balance = deposit(0, 1);
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
  pub RentFraction: Perbill = PerThing::from_rational(1u32, 30 * DAYS);
  // The lazy deletion runs inside on_initialize.
  pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
    BlockWeights::get().max_block;
  // The weight needed for decoding the queue should be less or equal than a fifth
  // of the overall weight dedicated to the lazy deletion.
  pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
      <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
      <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
    )) / 5) as u32;

  pub const MaxCodeSize: u32 = 2 * 1024 * 1024;
  pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
  type Time = Timestamp;
  type Randomness = RandomnessCollectiveFlip;
  type Currency = Balances;
  type Event = Event;
  type Call = Call;
  type CallFilter = Nothing;
  // type RentPayment = ();
  type DepositPerItem = DepositPerItem;
  type DepositPerByte = DepositPerByte;
  // type SignedClaimHandicap = SignedClaimHandicap;
  // type TombstoneDeposit = TombstoneDeposit;
  // type DepositPerContract = DepositPerContract;
  // type DepositPerStorageByte = DepositPerStorageByte;
  // type DepositPerStorageItem = DepositPerStorageItem;
  // type RentFraction = RentFraction;
  // type SurchargeReward = SurchargeReward;
  type CallStack = [pallet_contracts::Frame<Self>; 31];
  // type MaxValueSize = MaxValueSize;
  type WeightPrice = pallet_transaction_payment::Pallet<Self>;
  type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
  type ChainExtension = ();
  type DeletionQueueDepth = DeletionQueueDepth;
  type DeletionWeightLimit = DeletionWeightLimit;
  // type MaxCodeSize = MaxCodeSize;
  type Schedule = Schedule;
  type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
}

parameter_types! {
  pub Prefix: &'static [u8] = b"Pay CLVs to the Clover account:";
  pub const ClaimsModuleId: PalletId = PalletId(*b"clvclaim");
}

impl clover_claims::Config for Runtime {
  type PalletId = ClaimsModuleId;
  type Event = Event;
  type Currency = Balances;
  type Prefix = Prefix;
}

parameter_types! {
  pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
  pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
  type Event = Event;
  // type OnValidationData = ();
  type OnSystemEvent = ();
  type SelfParaId = parachain_info::Pallet<Runtime>;
  type DmpMessageHandler = DmpQueue;
  type ReservedDmpWeight = ReservedDmpWeight;
  type OutboundXcmpMessageSource = XcmpQueue;
  type XcmpMessageHandler = XcmpQueue;
  type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
  pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
  pub const Period: u32 = 6 * HOURS;
  pub const Offset: u32 = 0;
  pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_session::Config for Runtime {
  type Event = Event;
  type ValidatorId = <Self as frame_system::Config>::AccountId;
  // we don't have stash and controller, thus we don't need the convert as well.
  type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
  type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
  type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
  type SessionManager = CollatorSelection;
  // Essentially just Aura, but lets be pedantic.
  type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
  type Keys = SessionKeys;
  // type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
  type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
  type AuthorityId = AuraId;
  type DisabledValidators = ();
  type MaxAuthorities = MaxAuthorities;
}

pub mod currency {
  use super::Balance;

  pub const SUPPLY_FACTOR: Balance = 100;

  pub const WEI: Balance = 1;
  pub const KILOWEI: Balance = 1_000;
  pub const MEGAWEI: Balance = 1_000_000;
  pub const GIGAWEI: Balance = 1_000_000_000;
}

parameter_types! {
  // Tells `pallet_base_fee` whether to calculate a new BaseFee `on_finalize` or not.
  pub IsActive: bool = false;
  pub DefaultBaseFeePerGas: U256 = (1 * currency::GIGAWEI * currency::SUPPLY_FACTOR).into();
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
  fn lower() -> Permill {
    Permill::zero()
  }
  fn ideal() -> Permill {
    Permill::from_parts(500_000)
  }
  fn upper() -> Permill {
    Permill::from_parts(1_000_000)
  }
}

impl pallet_base_fee::Config for Runtime {
  type Event = Event;
  type Threshold = BaseFeeThreshold;
  type IsActive = IsActive;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
  pub const PotId: PalletId = PalletId(*b"PotStake");
  pub const MaxCandidates: u32 = 1000;
  pub const MinCandidates: u32 = 2;
  pub const SessionLength: BlockNumber = 6 * HOURS;
  pub const MaxInvulnerables: u32 = 100;
}

//
pub type CollatorSelectionUpdateOrigin = EnsureRootOrHalfCouncil;

impl pallet_collator_selection::Config for Runtime {
  type Event = Event;
  type Currency = Balances;
  type UpdateOrigin = CollatorSelectionUpdateOrigin;
  type PotId = PotId;
  type MaxCandidates = MaxCandidates;
  type MinCandidates = MinCandidates;
  type MaxInvulnerables = MaxInvulnerables;
  type ValidatorId = <Self as frame_system::Config>::AccountId;
  type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
  type ValidatorRegistration = Session;
  type KickThreshold = Period;
  type WeightInfo = ();
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
  pub enum Runtime where
    Block = Block,
    NodeBlock = opaque::Block,
    UncheckedExtrinsic = UncheckedExtrinsic
  {
    System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
    RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 1,
    Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,

    Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 3,
    Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,

    ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned} = 5,

    TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 6,

    ParachainInfo: parachain_info::{Pallet, Storage, Config} = 7,

    // Collator support. the order of these 4 are important and shall not change.
    Authorship: pallet_authorship::{Pallet, Call, Storage} = 8,
    CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 9,
    Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 10,
    Aura: pallet_aura::{Pallet, Config<T>} = 11,
    AuraExt: cumulus_pallet_aura_ext::{Pallet, Config} = 12,

    // Governance.
    Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 13,
    Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 14,
    TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 15,
    ElectionsPhragmen: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 16,
    TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 17,
    Treasury: pallet_treasury::{Pallet, Call, Storage, Event<T>, Config} = 18,

    // Smart contracts modules
    Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>} = 19,
    EVM: pallet_evm::{Pallet, Config, Call, Storage, Event<T>} = 20,
    Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Origin, Config, } = 21,

    Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 22,

    // Utility module.
    Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 23,
    Utility: pallet_utility::{Pallet, Call, Event} = 24,

    Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 25,
    Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 26,

    Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 27,

    Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 28,
    Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 29,

    // account module
    EvmAccounts: evm_accounts::{Pallet, Call, Storage, Event<T>} = 30,

    CloverClaims: clover_claims::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 31,
    BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 32,

    XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 33,
    PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin} = 34,
    CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 35,
    DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 36,

    // The main stage.
    Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 50,

    Spambot: cumulus_ping::{Pallet, Call, Storage, Event<T>} = 99,
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
  fp_self_contained::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = fp_self_contained::CheckedExtrinsic<AccountId, Call, SignedExtra, H160>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
  Runtime,
  Block,
  frame_system::ChainContext<Runtime>,
  Runtime,
  AllPalletsWithSystem,
>;

pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

impl fp_self_contained::SelfContainedCall for Call {
  type SignedInfo = H160;

  fn is_self_contained(&self) -> bool {
    match self {
      Call::Ethereum(call) => call.is_self_contained(),
      _ => false,
    }
  }

  fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
    match self {
      Call::Ethereum(call) => call.check_self_contained(),
      _ => None,
    }
  }

  fn validate_self_contained(&self, info: &Self::SignedInfo) -> Option<TransactionValidity> {
    match self {
      Call::Ethereum(call) => call.validate_self_contained(info),
      _ => None,
    }
  }

  fn pre_dispatch_self_contained(
    &self,
    info: &Self::SignedInfo,
  ) -> Option<Result<(), TransactionValidityError>> {
    match self {
      Call::Ethereum(call) => call.pre_dispatch_self_contained(info),
      _ => None,
    }
  }

  fn apply_self_contained(
    self,
    info: Self::SignedInfo,
  ) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
    match self {
      call @ Call::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
        Origin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info)),
      )),
      _ => None,
    }
  }
}

impl_runtime_apis! {
  impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
    fn slot_duration() -> sp_consensus_aura::SlotDuration {
      sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
    }

    fn authorities() -> Vec<AuraId> {
      Aura::authorities().into_inner()
    }
  }

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

  impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
    fn account_nonce(account: AccountId) -> Index {
      System::account_nonce(account)
    }
  }

  impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>
    for Runtime
  {
    fn call(
      origin: AccountId,
      dest: AccountId,
      value: Balance,
      gas_limit: u64,
      storage_deposit_limit: Option<Balance>,
      input_data: Vec<u8>,
    ) -> pallet_contracts_primitives::ContractExecResult<Balance> {
        Contracts::bare_call(origin, dest.into(), value, gas_limit, storage_deposit_limit, input_data, true)
    }

    fn instantiate(
      origin: AccountId,
      endowment: Balance,
      gas_limit: u64,
      storage_deposit_limit: Option<Balance>,
      code: pallet_contracts_primitives::Code<Hash>,
      data: Vec<u8>,
      salt: Vec<u8>,
    ) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId, Balance>
    {
      Contracts::bare_instantiate(origin, endowment, gas_limit, storage_deposit_limit, code, data, salt, true)
    }

    fn upload_code(
      origin: AccountId,
      code: Vec<u8>,
      storage_deposit_limit: Option<Balance>,
    ) -> pallet_contracts_primitives::CodeUploadResult<Hash, Balance>
    {
      Contracts::bare_upload_code(origin, code, storage_deposit_limit)
    }

    fn get_storage(
      address: AccountId,
      key: [u8; 32],
    ) -> pallet_contracts_primitives::GetStorageResult {
      Contracts::get_storage(address, key)
    }

//    fn rent_projection(
//      address: AccountId,
//    ) -> pallet_contracts_primitives::RentProjectionResult<BlockNumber> {
//      Contracts::rent_projection(address)
//    }
  }

  impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
    fn query_info(
      uxt: <Block as BlockT>::Extrinsic,
      len: u32,
    ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
      TransactionPayment::query_info(uxt, len)
    }

    fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::FeeDetails<Balance> {
      TransactionPayment::query_fee_details(uxt, len)
    }
  }

  impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
    fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
      ParachainSystem::collect_collation_info(header)
    }
  }

  impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
    fn chain_id() -> u64 {
        <Runtime as pallet_evm::Config>::ChainId::get()
    }

    fn account_basic(address: H160) -> EVMAccount {
        EVM::account_basic(&address)
    }

    fn gas_price() -> U256 {
        <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price()
    }

    fn account_code_at(address: H160) -> Vec<u8> {
        EVM::account_codes(address)
    }

    fn author() -> H160 {
        <pallet_evm::Pallet<Runtime>>::find_author()
    }

    fn storage_at(address: H160, index: U256) -> H256 {
        let mut tmp = [0u8; 32];
        index.to_big_endian(&mut tmp);
        EVM::account_storages(address, H256::from_slice(&tmp[..]))
    }

    fn call(
        from: H160,
        to: H160,
        data: Vec<u8>,
        value: U256,
        gas_limit: U256,
        max_fee_per_gas: Option<U256>,
        max_priority_fee_per_gas: Option<U256>,
        nonce: Option<U256>,
        estimate: bool,
        access_list: Option<Vec<(H160, Vec<H256>)>>,
    ) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
        let config = if estimate {
            let mut config = <Runtime as pallet_evm::Config>::config().clone();
            config.estimate = true;
            Some(config)
        } else {
            None
        };

        <Runtime as pallet_evm::Config>::Runner::call(
            from,
            to,
            data,
            value,
            gas_limit.unique_saturated_into(),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            nonce,
            access_list.unwrap_or_default(),
            config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
        ).map_err(|err| err.into())
    }

    fn create(
        from: H160,
        data: Vec<u8>,
        value: U256,
        gas_limit: U256,
        max_fee_per_gas: Option<U256>,
        max_priority_fee_per_gas: Option<U256>,
        nonce: Option<U256>,
        estimate: bool,
        access_list: Option<Vec<(H160, Vec<H256>)>>,
    ) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
        let config = if estimate {
            let mut config = <Runtime as pallet_evm::Config>::config().clone();
            config.estimate = true;
            Some(config)
        } else {
            None
        };

        <Runtime as pallet_evm::Config>::Runner::create(
            from,
            data,
            value,
            gas_limit.unique_saturated_into(),
            max_fee_per_gas,
            max_priority_fee_per_gas,
            nonce,
            access_list.unwrap_or_default(),
            config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
        ).map_err(|err| err.into())
    }

    fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
        Ethereum::current_transaction_statuses()
    }

    fn current_block() -> Option<pallet_ethereum::Block> {
        Ethereum::current_block()
    }

    fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
        Ethereum::current_receipts()
    }

    fn current_all() -> (
        Option<pallet_ethereum::Block>,
        Option<Vec<pallet_ethereum::Receipt>>,
        Option<Vec<TransactionStatus>>
    ) {
        (
            Ethereum::current_block(),
            Ethereum::current_receipts(),
            Ethereum::current_transaction_statuses()
        )
    }

    fn extrinsic_filter(
      xts: Vec<<Block as BlockT>::Extrinsic>,
    ) -> Vec<EthereumTransaction> {
      xts.into_iter().filter_map(|xt| match xt.0.function {
        Call::Ethereum(transact{transaction}) => Some(transaction),
        _ => None
      }).collect::<Vec<EthereumTransaction>>()
    }

    fn elasticity() -> Option<Permill> {
      Some(BaseFee::elasticity())
    }
  }

  impl fp_trace_apis::DebugRuntimeApi<Block> for Runtime {
    fn trace_transaction(
      _extrinsics: Vec<<Block as BlockT>::Extrinsic>,
      _traced_transaction: &pallet_ethereum::Transaction,
    ) -> Result<
      (),
      sp_runtime::DispatchError,
    > {
      use fp_tracer::tracer::EvmTracer;
      use pallet_ethereum::Call::transact;
      // Apply the a subset of extrinsics: all the substrate-specific or ethereum
      // transactions that preceded the requested transaction.
      for ext in _extrinsics.into_iter() {
        let _ = match &ext.0.function {
          Call::Ethereum(transact { transaction }) => {
            if transaction == _traced_transaction {
              EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
              return Ok(());
            } else {
              Executive::apply_extrinsic(ext)
            }
          }
          _ => Executive::apply_extrinsic(ext),
        };
      }

      Err(sp_runtime::DispatchError::Other(
        "Failed to find Ethereum transaction among the extrinsics.",
      ))
    }

    fn trace_block(
      _extrinsics: Vec<<Block as BlockT>::Extrinsic>,
      _known_transactions: Vec<H256>,
    ) -> Result<
      (),
      sp_runtime::DispatchError,
    > {
      use fp_tracer::tracer::EvmTracer;
      use sha3::{Digest, Keccak256};
      use pallet_ethereum::Call::transact;

      let mut config = <Runtime as pallet_evm::Config>::config().clone();
      config.estimate = true;

      // Apply all extrinsics. Ethereum extrinsics are traced.
      for ext in _extrinsics.into_iter() {
        match &ext.0.function {
          Call::Ethereum(transact { transaction }) => {
            let eth_extrinsic_hash =
              H256::from_slice(Keccak256::digest(&rlp::encode(transaction)).as_slice());
            if _known_transactions.contains(&eth_extrinsic_hash) {
              // Each known extrinsic is a new call stack.
              EvmTracer::emit_new();
              EvmTracer::new().trace(|| Executive::apply_extrinsic(ext));
            } else {
              let _ = Executive::apply_extrinsic(ext);
            }
          }
          _ => {
            let _ = Executive::apply_extrinsic(ext);
          }
        };
      }

      Ok(())
    }
  }
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
  fn check_inherents(
    block: &Block,
    relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
  ) -> sp_inherents::CheckInherentsResult {
    let relay_chain_slot = relay_state_proof
      .read_slot()
      .expect("Could not read the relay chain slot from the proof");

    let inherent_data =
      cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
        relay_chain_slot,
        sp_std::time::Duration::from_secs(6),
      )
      .create_inherent_data()
      .expect("Could not create the timestamp inherent data");

    inherent_data.check_extrinsics(&block)
  }
}

cumulus_pallet_parachain_system::register_validate_block!(
  Runtime = Runtime,
  BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
  CheckInherents = CheckInherents,
);
