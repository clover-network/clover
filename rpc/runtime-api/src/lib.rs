#![cfg_attr(not(feature = "std"), no_std)]

sp_api::decl_runtime_apis! {
	/// The API to query account balance of a currency
	pub trait CurrencyBalanceApi<AccountId, CurrencyId, Balance> where
		AccountId: codec::Codec,
    CurrencyId: codec::Codec,
    Balance: codec::Codec,
	{
		/// Get account balance of a currency
		fn account_balance(account: AccountId, currency_id: CurrencyId) -> Balance;
	}

	pub trait CurrencyPairApi {
	    fn currency_pair() -> sp_std::vec::Vec<(primitives::CurrencyId, primitives::CurrencyId)>;
	}
}