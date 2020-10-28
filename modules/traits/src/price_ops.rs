pub trait PriceProvider<CurrencyId, Price> {
  fn get_relative_price(base: CurrencyId, quote: CurrencyId) -> Option<Price>;
  fn get_price(currency_id: CurrencyId) -> Option<Price>;
  fn lock_price(currency_id: CurrencyId);
  fn unlock_price(currency_id: CurrencyId);
}
