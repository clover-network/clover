#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
  decl_event, decl_module, decl_storage,
  traits::{EnsureOrigin, Get},
  weights::DispatchClass,
};
use frame_system::{self as system};
use orml_traits::{DataFeeder, DataProvider};
use orml_utilities::with_transaction_result;
use primitives::{CurrencyId, Price};
use sp_runtime::traits::CheckedDiv;
use clover_traits::PriceProvider;

// mod mock;
// mod tests;

pub trait Trait: system::Trait {
  type Event: From<Event> + Into<<Self as system::Trait>::Event>;
  type Source: DataProvider<CurrencyId, Price> + DataFeeder<CurrencyId, Price, Self::AccountId>;
  type GetStableCurrencyId: Get<CurrencyId>;
  type StableCurrencyFixedPrice: Get<Price>;
  type LockOrigin: EnsureOrigin<Self::Origin>;
}

decl_event!(
  pub enum Event {
    LockPrice(CurrencyId, Price),
    UnlockPrice(CurrencyId),
  }
);

decl_storage! {
  trait Store for Module<T: Trait> as Prices {
    LockedPrice get(fn locked_price): map hasher(twox_64_concat) CurrencyId => Option<Price>;
  }
}

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    fn deposit_event() = default;

    const GetStableCurrencyId: CurrencyId = T::GetStableCurrencyId::get();
    const StableCurrencyFixedPrice: Price = T::StableCurrencyFixedPrice::get();

    #[weight = (10_000, DispatchClass::Operational)]
    fn lock_price(origin, currency_id: CurrencyId) {
      with_transaction_result(|| {
        T::LockOrigin::ensure_origin(origin)?;
        <Module<T> as PriceProvider<CurrencyId, Price>>::lock_price(currency_id);
        Ok(())
      })?;
    }

    #[weight = (10_000, DispatchClass::Operational)]
    fn unlock_price(origin, currency_id: CurrencyId) {
      with_transaction_result(|| {
        T::LockOrigin::ensure_origin(origin)?;
        <Module<T> as PriceProvider<CurrencyId, Price>>::unlock_price(currency_id);
        Ok(())
      })?;
    }
  }
}

impl<T: Trait> Module<T> {}

impl<T: Trait> PriceProvider<CurrencyId, Price> for Module<T> {
  fn get_relative_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
    if let (Some(base_price), Some(quote_price)) =
      (Self::get_price(base_currency_id), Self::get_price(quote_currency_id))
    {
      base_price.checked_div(&quote_price)
    } else {
      None
    }
  }

  /// get price in USD
  fn get_price(currency_id: CurrencyId) -> Option<Price> {
    if currency_id == T::GetStableCurrencyId::get() {
      // if is stable currency, return fixed price
      Some(T::StableCurrencyFixedPrice::get())
    } else {
      // if locked price exists, return it, otherwise return latest price from oracle.
      Self::locked_price(currency_id).or_else(|| T::Source::get(&currency_id))
    }
  }

  fn lock_price(currency_id: CurrencyId) {
    // lock price when get valid price from source
    if let Some(val) = T::Source::get(&currency_id) {
      LockedPrice::insert(currency_id, val);
      <Module<T>>::deposit_event(Event::LockPrice(currency_id, val));
    }
  }

  fn unlock_price(currency_id: CurrencyId) {
    LockedPrice::remove(currency_id);
    <Module<T>>::deposit_event(Event::UnlockPrice(currency_id));
  }
}
