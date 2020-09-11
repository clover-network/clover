#![cfg(test)]

use super::*;
use mock::*;

pub const BXB: CurrencyId = CurrencyId::BXB;
pub const BUSD: CurrencyId = CurrencyId::BUSD;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};

#[test]
fn test_balance() {
	ExtBuilder::default()
		.balances(vec![
			(AccountId::from(ALICE), BXB, 1000),
			(AccountId::from(BOB), BUSD, 1000),
		])
		.build()
		.execute_with(|| {
			assert_eq!(<Currencies as MultiCurrency<_>>::free_balance(BXB, &AccountId::from(ALICE)), 500);
			assert_eq!(<Currencies as MultiCurrency<_>>::free_balance(BUSD, &AccountId::from(BOB)), 1000);

			let _ = <Currencies as MultiCurrencyExtended<_>>::update_balance(BXB, &AccountId::from(ALICE), 1000);
			assert_eq!(<Currencies as MultiCurrency<_>>::free_balance(BXB, &AccountId::from(ALICE)), 1500);
		});
}
