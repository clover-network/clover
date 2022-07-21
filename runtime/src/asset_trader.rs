use frame_support::{
  traits::{tokens::fungibles::Mutate, Get},
  weights::{constants::WEIGHT_PER_SECOND, Weight},
};
use sp_runtime::traits::Zero;
use sp_std::marker::PhantomData;

use clover_traits::AssetIdWeightGetter;
use xcm::latest::{
  AssetId as xcmAssetId, Error as XcmError, Fungibility, MultiAsset, MultiLocation,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::{Convert, MatchesFungibles, WeightTrader};

pub struct FungibleAssetTrader<
  AssetId: From<u64> + Clone,
  LocationConverter: Convert<MultiLocation, AssetId>,
  AssetWeightGetter: AssetIdWeightGetter<AssetId>,
  R: TakeRevenue,
>(
  Weight,
  Option<(MultiLocation, u128, u128)>,
  PhantomData<(AssetId, LocationConverter, AssetWeightGetter, R)>,
);

impl<
    AssetId: From<u64> + Clone,
    LocationConverter: Convert<MultiLocation, AssetId>,
    AssetWeightGetter: AssetIdWeightGetter<AssetId>,
    R: TakeRevenue,
  > WeightTrader for FungibleAssetTrader<AssetId, LocationConverter, AssetWeightGetter, R>
{
  fn new() -> Self {
    FungibleAssetTrader(0, None, PhantomData)
  }

  fn buy_weight(
    &mut self,
    weight: Weight,
    payment: xcm_executor::Assets,
  ) -> Result<xcm_executor::Assets, XcmError> {
    let first_fungible = payment
      .clone()
      .fungible_assets_iter()
      .next()
      .ok_or(XcmError::TooExpensive)?;

    match (first_fungible.id, first_fungible.fun) {
      (xcmAssetId::Concrete(id), Fungibility::Fungible(_)) => {
        let asset_id: AssetId =
          LocationConverter::convert(id.clone()).map_err(|_| XcmError::AssetNotFound)?;
        // we only support assets that have weight price configured
        let units_per_second =
          AssetWeightGetter::get_units_per_second(asset_id).ok_or(XcmError::TooExpensive)?;
        let amount = units_per_second.saturating_mul(weight as u128) / (WEIGHT_PER_SECOND as u128);
        // the fee is too low
        if amount.is_zero() {
          return Ok(payment);
        }
        let fee = MultiAsset {
          fun: Fungibility::Fungible(amount),
          id: xcmAssetId::Concrete(id.clone()),
        };
        let unused = payment
          .checked_sub(fee)
          .map_err(|_| XcmError::TooExpensive)?;

        self.0 = self.0.saturating_add(weight);
        let new_info = match self.1.clone() {
          Some((prev_loc, prev_amount, prev_units_per_second)) => {
            if prev_loc == id.clone() {
              // this should not happen really
              if units_per_second != prev_units_per_second {
                return Err(XcmError::TooExpensive);
              }
              // some payment happened previously and the payment asset is the same as this round,
              // add the new amount to the payment
              Some((id, prev_amount.saturating_add(amount), units_per_second))
            } else {
              // some payment made before, but it's not the same payment as this round
              // ignore the refund for this round
              None
            }
          }
          None => Some((id, amount, units_per_second)),
        };

        if let Some(new_info) = new_info {
          self.0 = self.0.saturating_add(weight);
          self.1 = Some(new_info);
        };

        Ok(unused)
      }
      _ => Err(XcmError::TooExpensive),
    }
  }

  fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
    if let Some((id, prev_amount, units_per_second)) = self.1.clone() {
      // ensure we refund at most the weight we paid
      let weight = weight.min(self.0);
      self.0 -= weight;
      let amount = units_per_second * (weight as u128) / (WEIGHT_PER_SECOND as u128);
      self.1 = Some((
        id.clone(),
        prev_amount.saturating_sub(amount),
        units_per_second,
      ));
      Some(MultiAsset {
        fun: Fungibility::Fungible(amount),
        id: xcmAssetId::Concrete(id.clone()),
      })
    } else {
      None
    }
  }
}

/// helper to handle transaction fee payments
impl<
    AssetId: From<u64> + Clone,
    LocationConverter: Convert<MultiLocation, AssetId>,
    AssetWeightGetter: AssetIdWeightGetter<AssetId>,
    R: TakeRevenue,
  > Drop for FungibleAssetTrader<AssetId, LocationConverter, AssetWeightGetter, R>
{
  fn drop(&mut self) {
    if let Some((id, amount, _)) = self.1.clone() {
      R::take_revenue((id, amount).into());
    }
  }
}

/// helper to pay xcm transactions fees to the Beneficial account
pub struct XcmPayToAccount<Assets, Matcher, AccountId, Beneficial>(
  PhantomData<(Assets, Matcher, AccountId, Beneficial)>,
);
impl<
    Assets: Mutate<AccountId>,
    Matcher: MatchesFungibles<Assets::AssetId, Assets::Balance>,
    AccountId: Clone,
    Beneficial: Get<AccountId>,
  > TakeRevenue for XcmPayToAccount<Assets, Matcher, AccountId, Beneficial>
{
  fn take_revenue(revenue: MultiAsset) {
    match Matcher::matches_fungibles(&revenue) {
      Ok((asset_id, amount)) => {
        if !amount.is_zero() {
          let ok = Assets::mint_into(asset_id, &Beneficial::get(), amount).is_ok();
          debug_assert!(ok, "`mint_into` cannot fail; qed");
        }
      }
      Err(_) => log::debug!(
        target: "xcm-fee",
        "no fungible found for `take_revenue`"
      ),
    }
  }
}
