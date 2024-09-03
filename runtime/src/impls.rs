
use sp_arithmetic::traits::{BaseArithmetic, Unsigned};
use sp_runtime::traits::Convert;
use sp_runtime::{DispatchResult, FixedPointNumber, Perquintill, Perbill};
use frame_support::{traits::ExistenceRequirement, transactional};
use frame_support::traits::{Get, OnUnbalanced, Currency, ReservableCurrency};
use frame_support::weights::{
    WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};
use pallet_transaction_payment::{Multiplier, MultiplierUpdate, };
use crate::{AccountId, Balances, Authorship, NegativeImbalance};
use clover_traits::account::MergeAccount;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
  fn on_nonzero_unbalanced(amount: NegativeImbalance) {
    if let Some(author) = &Authorship::author() {
      Balances::resolve_creating(author, amount);
    }
  }
}

pub struct MergeAccountEvm;
impl MergeAccount<AccountId> for MergeAccountEvm {
#[transactional]
fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
     // unreserve all reserved currency
     <Balances as ReservableCurrency<_>>::unreserve(source, Balances::reserved_balance(source));

     // transfer all free to dest
    Balances::transfer(&source, &dest, Balances::free_balance(source), ExistenceRequirement::KeepAlive)
  }
}

pub struct WeightToFee<T>(sp_std::marker::PhantomData<T>);

impl<T> WeightToFeePolynomial for WeightToFee<T> where
  T: BaseArithmetic + From<u32> + Copy + Unsigned
{
  type Balance = T;

  fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
    smallvec::smallvec!(WeightToFeeCoefficient {
      coeff_integer: 10_000u32.into(),
      coeff_frac: Perbill::zero(),
      negative: false,
      degree: 1,
    })
  }
}

/// Reset the fee multiplier to the fixed value
/// this is required to perform the upgrade from a previously running chain
/// without applying the static fee multiplier
/// the value is incorrect (1_000_000_000 in clover testnet, spec version4).
#[allow(dead_code)]
pub struct StaticFeeMultiplierUpdate<T, S, V, M, N>(sp_std::marker::PhantomData<(T, S, V, M, N)>);

impl<T, S, V, M, N> MultiplierUpdate for StaticFeeMultiplierUpdate<T, S, V, M, N>
  where T: frame_system::Config, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>, N: Get<Multiplier>,
{
  fn min() -> Multiplier {
    M::get()
  }
  fn max() -> Multiplier {
    N::get()
  }
  fn target() -> Perquintill {
    S::get()
  }
  fn variability() -> Multiplier {
    V::get()
  }
}

impl<T, S, V, M, N> Convert<Multiplier, Multiplier> for StaticFeeMultiplierUpdate<T, S, V, M, N>
  where T: frame_system::Config, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>, N: Get<Multiplier>,
{
  fn convert(_previous: Multiplier) -> Multiplier {
    Multiplier::saturating_from_integer(1)
  }
}
