
use sp_arithmetic::{traits::{BaseArithmetic, Unsigned}};
use sp_runtime::traits::Convert;
use sp_runtime::{ DispatchResult, FixedPointNumber, Perquintill, Perbill, };
use frame_support::transactional;
use frame_support::traits::{Get, OnUnbalanced, Currency, ReservableCurrency, };
use frame_support::weights::{
    WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};
use pallet_transaction_payment::{Multiplier, MultiplierUpdate, };
use crate::{AccountId, Balances, Authorship, NegativeImbalance};
use clover_traits::account::MergeAccount;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
  fn on_nonzero_unbalanced(amount: NegativeImbalance) {
    Balances::resolve_creating(&Authorship::author(), amount);
  }
}

pub struct MergeAccountEvm;
impl MergeAccount<AccountId> for MergeAccountEvm {
#[transactional]
fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
     // unreserve all reserved currency
     <Balances as ReservableCurrency<_>>::unreserve(source, Balances::reserved_balance(source));

     // transfer all free to dest
     match Balances::transfer(Some(source.clone()).into(), dest.clone().into(), Balances::free_balance(source)) {
       Ok(_) => Ok(()),
       Err(e) => Err(e.error),
     }
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
pub struct StaticFeeMultiplierUpdate<T, S, V, M>(sp_std::marker::PhantomData<(T, S, V, M)>);

impl<T, S, V, M> MultiplierUpdate for StaticFeeMultiplierUpdate<T, S, V, M>
  where T: frame_system::Config, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>,
{
  fn min() -> Multiplier {
    M::get()
  }
  fn target() -> Perquintill {
    S::get()
  }
  fn variability() -> Multiplier {
    V::get()
  }
}

impl<T, S, V, M> Convert<Multiplier, Multiplier> for StaticFeeMultiplierUpdate<T, S, V, M>
  where T: frame_system::Config, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>,
{
  fn convert(_previous: Multiplier) -> Multiplier {
    Multiplier::saturating_from_integer(1)
  }
}
