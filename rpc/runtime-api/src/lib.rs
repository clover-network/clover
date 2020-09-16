#![cfg_attr(not(feature = "std"), no_std)]

sp_api::decl_runtime_apis! {
	pub trait CurrencyBalanceApi<AccountId, CurrencyId, Balance> where
		AccountId: codec::Codec,
        CurrencyId: codec::Codec,
        Balance: codec::Codec {
		// Get account balance of a currency
		fn account_balance(account: AccountId, currency_id: Option<CurrencyId>) -> sp_std::vec::Vec<(CurrencyId, Balance)>;
	}

	pub trait CurrencyPairApi<> {
		fn currency_pair() -> sp_std::vec::Vec<(primitives::CurrencyId, primitives::CurrencyId)>;
	}

	pub trait CurrencyExchangeApi<CurrencyId, Balance> where 
		CurrencyId: codec::Codec,
		Balance: codec::Codec {
		fn target_amount_available(source: CurrencyId, target: CurrencyId, amount: Balance) -> (Balance, sp_std::vec::Vec<CurrencyId>);
		fn supply_amount_needed(source: CurrencyId, target: CurrencyId, amount: Balance) -> (Balance, sp_std::vec::Vec<CurrencyId>);
	}
}