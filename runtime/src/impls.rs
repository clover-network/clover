
use sp_arithmetic::{traits::{BaseArithmetic, Unsigned}};
use sp_runtime::traits::Convert;
use sp_runtime::{ FixedPointNumber, Perquintill, Perbill, };
use frame_support::traits::{Get, OnUnbalanced, Currency, };
use frame_support::weights::{
    WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial,
};
use pallet_transaction_payment::{Multiplier, MultiplierUpdate, };
use crate::{Balances, Authorship, NegativeImbalance};

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
  fn on_nonzero_unbalanced(amount: NegativeImbalance) {
    Balances::resolve_creating(&Authorship::author(), amount);
  }
}

pub struct WeightToFee<T>(sp_std::marker::PhantomData<T>);

impl<T> WeightToFeePolynomial for WeightToFee<T> where
  T: BaseArithmetic + From<u32> + Copy + Unsigned
{
  type Balance = T;

  fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
    smallvec::smallvec!(WeightToFeeCoefficient {
      coeff_integer: 10_000_000u32.into(),
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
pub struct StaticFeeMultiplierUpdate<T, S, V, M>(sp_std::marker::PhantomData<(T, S, V, M)>);

impl<T, S, V, M> MultiplierUpdate for StaticFeeMultiplierUpdate<T, S, V, M>
  where T: frame_system::Trait, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>,
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
  where T: frame_system::Trait, S: Get<Perquintill>, V: Get<Multiplier>, M: Get<Multiplier>,
{
  fn convert(_previous: Multiplier) -> Multiplier {
    Multiplier::saturating_from_integer(1)
  }
}
