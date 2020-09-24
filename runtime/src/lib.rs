#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_std::prelude::*;
use sp_core::{
	crypto::KeyTypeId,
	OpaqueMetadata,
};
use sp_runtime::{
	ApplyExtrinsicResult, generic, create_runtime_str, FixedPointNumber, impl_opaque_keys, Percent,
	ModuleId, transaction_validity::{TransactionPriority, TransactionValidity, TransactionSource},
};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, Convert, NumberFor, OpaqueKeys, SaturatedConversion, Saturating,
	StaticLookup,
};
use sp_runtime::curve::PiecewiseLinear;
use enum_iterator::IntoEnumIterator;

use sp_api::impl_runtime_apis;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_grandpa::fg_primitives;
use pallet_session::historical as pallet_session_historical;
use sp_version::RuntimeVersion;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_core::{u32_trait::{_1, _2, _4, _5}};

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

use orml_traits::{MultiCurrency};
use orml_currencies::{BasicCurrencyAdapter};

pub use pallet_staking::StakerStatus;

pub use pallet_timestamp::Call as TimestampCall;
pub use pallet_balances::Call as BalancesCall;
pub use sp_runtime::{Permill, Perbill};
use frame_system::{EnsureRoot};
pub use frame_support::{
	construct_runtime, debug, parameter_types, StorageValue,
	traits::{KeyOwnerProofSystem, Randomness, LockIdentifier},
	weights::{
		Weight, IdentityFee,
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
	},
};
use codec::{Encode};

pub use primitives::{
	AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, EraIndex, Hash, Index,
	Moment, Rate, Share, Signature,
  currency::*,
};

pub use constants::{time::*, };

mod constants;
mod mock;
mod tests;

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
  }
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bitdex"),
	impl_name: create_runtime_str!("bitdex"),
	authoring_version: 1,
	spec_version: 1,
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

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight = AvailableBlockRatio::get()
		.saturating_sub(Perbill::from_percent(10)) * MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Trait for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = ();
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
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = BlockExecutionWeight;
	/// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
	/// The maximum weight that a single extrinsic of `Normal` dispatch class can have,
	/// idependent of the logic of that extrinsics. (Roughly max block weight - average on
	/// initialize cost).
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type ModuleToIndex = ModuleToIndex;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl pallet_babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;

	type KeyOwnerProofSystem = Historical;

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::IdentificationTuple;

	type HandleEquivocation = pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, ()>; // Offences
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = Historical;

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
		KeyTypeId,
		GrandpaId,
	)>>::IdentificationTuple;

	type HandleEquivocation = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Trait for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ()); // ImOnline
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

/// Struct that handles the conversion of Balance -> `u64`. This is used for
/// staking's election calculation.
pub struct CurrencyToVoteHandler;

impl CurrencyToVoteHandler {
	fn factor() -> Balance {
		(Balances::total_issuance() / u64::max_value() as Balance).max(1)
	}
}

impl Convert<Balance, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}

impl Convert<u128, Balance> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
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
	pub const SessionsPerEra: sp_staking::SessionIndex = 3;  // 3 sessions in an era, (3 hours)
	pub const BondingDuration: pallet_staking::EraIndex = 28; // 28 era for unbouding (28 * 3 hours)
	pub const SlashDeferDuration: pallet_staking::EraIndex = 14; // 1/2 bonding duration
	pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxIterations: u32 = 10;
	// 0.05%. The higher the value, the more strict solution acceptance becomes.
	pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
}

impl pallet_staking::Trait for Runtime {
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVoteHandler;
	type RewardRemainder = Treasury;
	type Event = Event;
	type Slash = Treasury;
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;

	type SlashCancelOrigin = EnsureRoot<AccountId>;

	type SessionInterface = Self;
	type RewardCurve = RewardCurve;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

// democracy
impl pallet_scheduler::Trait for Runtime {
	type Event = Event;
	type Origin = Origin;
	type Call = Call;
	type MaximumWeight = MaximumBlockWeight;
	type PalletsOrigin = OriginCaller;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 7 * MINUTES;
	pub const VotingPeriod: BlockNumber = 7 * MINUTES;
	pub const FastTrackVotingPeriod: BlockNumber = 1 * MINUTES;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 8 * MINUTES;
	pub const CooloffPeriod: BlockNumber = 7 * MINUTES;
	// One cent: $10,000 / MB
	pub const PreimageByteDeposit: Balance = 10 * MILLICENTS;
	pub const InstantAllowed: bool = false;
	pub const MaxVotes: u32 = 100;
}

impl pallet_democracy::Trait for Runtime {
	type Proposal = Call;
	type Event = Event;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	/// A super-majority can have the next scheduled referendum be a straight
	/// majority-carries vote.
	type ExternalMajorityOrigin = pallet_collective::EnsureProportionAtLeast<_4, _5, AccountId, CouncilCollective>;
	/// A unanimous council can have the next scheduled referendum be a straight
	/// default-carries (NTB) vote.
	type ExternalDefaultOrigin = pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
	/// Full of the technical committee can have an
	/// ExternalMajority/ExternalDefault vote be tabled immediately and with a
	/// shorter voting/enactment period.
	type FastTrackOrigin = pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
	type InstantOrigin = frame_system::EnsureNever<AccountId>;
	type InstantAllowed = InstantAllowed;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	/// To cancel a proposal which has been passed, all of the council must
	/// agree to it.
	type CancellationOrigin = pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
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
	type WeightInfo = ();
}

impl pallet_utility::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = ();
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Trait<CouncilCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type WeightInfo = ();
}

/// Converter for currencies to votes.
pub struct CurrencyToVoteHandler2<R>(sp_std::marker::PhantomData<R>);

impl<R> CurrencyToVoteHandler2<R>
where
	R: pallet_balances::Trait,
	R::Balance: Into<u128>,
{
	fn factor() -> u128 {
		let issuance: u128 = <pallet_balances::Module<R>>::total_issuance().into();
		(issuance / u64::max_value() as u128).max(1)
	}
}

impl<R> Convert<u128, u64> for CurrencyToVoteHandler2<R>
where
	R: pallet_balances::Trait,
	R::Balance: Into<u128>,
{
	fn convert(x: u128) -> u64 { (x / Self::factor()) as u64 }
}

impl<R> Convert<u128, u128> for CurrencyToVoteHandler2<R>
where
	R: pallet_balances::Trait,
	R::Balance: Into<u128>,
{
	fn convert(x: u128) -> u128 { x * Self::factor() }
}

parameter_types! {
	pub const CandidacyBond: Balance = 1 * DOLLARS;
	pub const VotingBond: Balance = 5 * CENTS;
	/// Daily council elections.
	pub const TermDuration: BlockNumber = 24 * HOURS;
	pub const DesiredMembers: u32 = 17;
	pub const DesiredRunnersUp: u32 = 30;
	pub const ElectionsPhragmenModuleId: LockIdentifier = *b"phrelect";
}

impl pallet_elections_phragmen::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
	type ChangeMembers = Council;
	type InitializeMembers = Council;
	type CurrencyToVote = CurrencyToVoteHandler2<Self>;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type LoserCandidate = Treasury;
	type BadReport = Treasury;
	type KickedMember = Treasury;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type TermDuration = TermDuration;
	type ModuleId = ElectionsPhragmenModuleId;
	type WeightInfo = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Trait<TechnicalCollective> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = TechnicalMotionDuration;
	type MaxProposals = TechnicalMaxProposals;
	type WeightInfo = ();
}

impl pallet_membership::Trait<pallet_membership::Instance1> for Runtime {
	type Event = Event;
	type AddOrigin = frame_system::EnsureRoot<AccountId>;
	type RemoveOrigin = frame_system::EnsureRoot<AccountId>;
	type SwapOrigin = frame_system::EnsureRoot<AccountId>;
	type ResetOrigin = frame_system::EnsureRoot<AccountId>;
	type PrimeOrigin = frame_system::EnsureRoot<AccountId>;
	type MembershipInitialized = TechnicalCommittee;
	type MembershipChanged = TechnicalCommittee;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 20 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const TipReportDepositPerByte: Balance = 10 * MILLICENTS;
}

impl pallet_treasury::Trait for Runtime {
	type Currency = Balances;
	type ApproveOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
	type RejectOrigin = pallet_collective::EnsureProportionMoreThan<_1, _5, AccountId, CouncilCollective>;
	type Tippers = ElectionsPhragmen;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = Event;
	type ProposalRejection = Treasury;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type ModuleId = TreasuryModuleId;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = Balances;
	type OnTransactionPayment = ();
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Trait for Runtime {
	type AccountIndex = AccountIndex;
	type Event = Event;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type WeightInfo = ();
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
				debug::warn!("Unable to create signed payload: {:?}", e);
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

impl orml_tokens::Trait for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = (); // todo: do we need it?
	type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::BXB;
}

impl orml_currencies::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Balances, Balance, Balance, Amount, BlockNumber>;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const RewardModuleId: ModuleId = ModuleId(*b"bxb/repm");
	pub const ExistentialReward: u128 = 100;
}

impl reward_pool::Trait for Runtime {
  type Event = Event;
  type PoolId = clover_incentives::PoolId;
	type ModuleId = RewardModuleId;
  type Currency = Currencies;
  type GetNativeCurrencyId = GetNativeCurrencyId;
	type ExistentialReward = ExistentialReward;
  type Handler = Incentives;
}

impl clover_incentives::Trait for Runtime {
  type RewardPool = RewardPool;
}

parameter_types! {
	pub GetExchangeFee: Rate = Rate::saturating_from_rational(1, 1000);
	pub const BithumbDexModuleId: ModuleId = ModuleId(*b"bxb/dexm");
}

impl bithumbdex::Trait for Runtime {
	type Event = Event;
	type Currency = Currencies;
	type Share = Share;
	type GetExchangeFee = GetExchangeFee;
	type ModuleId = BithumbDexModuleId;
	type OnAddLiquidity = ();
	type OnRemoveLiquidity = ();
  type IncentiveOps = Incentives;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},

		Authorship: pallet_authorship::{Module, Call, Storage},
		Babe: pallet_babe::{Module, Call, Storage, Config, Inherent, ValidateUnsigned},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},

		Indices: pallet_indices::{Module, Call, Storage, Config<T>, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},

		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},

		Staking: pallet_staking::{Module, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
		Historical: pallet_session_historical::{Module},

		Currencies: orml_currencies::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Event<T>, Config<T>},

		// Governance.
		Democracy: pallet_democracy::{Module, Call, Storage, Config, Event<T>},
		Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		TechnicalCommittee: pallet_collective::<Instance2>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		ElectionsPhragmen: pallet_elections_phragmen::{Module, Call, Storage, Event<T>, Config<T>},
		TechnicalMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
		Treasury: pallet_treasury::{Module, Call, Storage, Event<T>, Config},

		// Utility module.
		Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>},
		Utility: pallet_utility::{Module, Call, Event},
		BithumbDex: bithumbdex::{Module, Storage, Call, Event<T>, Config},
		RewardPool: reward_pool::{Module, Storage, Call, Event<T>,},
		Incentives: clover_incentives::{Module, Storage, Call, },
	}
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
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
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
>;

pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
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
			Runtime::metadata().into()
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

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
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

  impl sp_consensus_babe::BabeApi<Block> for Runtime {
  	fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
  		sp_consensus_babe::BabeGenesisConfiguration {
  			slot_duration: Babe::slot_duration(),
  			epoch_length: EpochDuration::get(),
  			c: PRIMARY_PROBABILITY,
  			genesis_authorities: Babe::authorities(),
  			randomness: Babe::randomness(),
  			allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
  		}
  	}

  	fn current_epoch_start() -> sp_consensus_babe::SlotNumber {
  		Babe::current_epoch_start()
  	}

  	fn generate_key_ownership_proof(
  		_slot_number: sp_consensus_babe::SlotNumber,
  		authority_id: sp_consensus_babe::AuthorityId,
  		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
  		use codec::Encode;

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

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	}

	impl bitdex_rpc_runtime_api::CurrencyBalanceApi<Block, AccountId, CurrencyId, Balance> for Runtime {
		fn account_balance(account: AccountId, currency_id: Option<CurrencyId>) -> sp_std::vec::Vec<(CurrencyId, Balance)> {
			let mut balances = sp_std::vec::Vec::new();
			match currency_id {
				None => {
					for cid in CurrencyId::into_enum_iter() {
						balances.push((cid, Currencies::total_balance(cid, &account)));
					}
				},
				Some(cid) => balances.push((cid, Currencies::total_balance(cid, &account)))
			}
			balances
		}
	}

	impl bitdex_rpc_runtime_api::CurrencyPairApi<Block> for Runtime {
		fn currency_pair() -> sp_std::vec::Vec<(CurrencyId, CurrencyId)> {
		 	let pair = BithumbDex::get_existing_currency_pairs().0;
		 	pair
		}
	}

	impl bitdex_rpc_runtime_api::CurrencyExchangeApi<Block, AccountId, CurrencyId, Balance, Rate, Share> for Runtime {
		fn target_amount_available(source: CurrencyId, target: CurrencyId, amount: Balance) -> (Balance, sp_std::vec::Vec<CurrencyId>) {
			let balance = BithumbDex::get_target_amount_available(source, target, amount);
			balance
		}

		fn supply_amount_needed(source: CurrencyId, target: CurrencyId, amount: Balance) -> (Balance, sp_std::vec::Vec<CurrencyId>) {
			let balance = BithumbDex::get_supply_amount_needed(source, target, amount);
			balance
		}

		fn get_liquidity(account: Option<AccountId>) -> sp_std::vec::Vec<(CurrencyId, CurrencyId, Balance, Balance, Balance, Balance)> {
			let result = BithumbDex::get_liquidity(account);
			result
		}

		fn get_exchange_rate() -> Rate {
			let result = BithumbDex::get_exchange_fee();
			result
		}

		fn to_add_liquidity(source: CurrencyId, target: CurrencyId, source_amount: Balance, target_amount: Balance) -> (Share, Share) {
			let result = BithumbDex::to_add_liquidity(source, target, source_amount, target_amount);
			result
		}
	}
}
