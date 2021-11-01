use impl_trait_for_tuples::impl_for_tuples;
use sp_runtime::{ DispatchResult, DispatchError };
use frame_support::storage::{with_transaction, TransactionOutcome};

fn with_transaction_result<R>(f: impl FnOnce() -> Result<R, DispatchError>) -> Result<R, DispatchError> {
	with_transaction(|| {
		let res = f();
		if res.is_ok() {
			TransactionOutcome::Commit(res)
		} else {
			TransactionOutcome::Rollback(res)
		}
	})
}

pub trait MergeAccount<AccountId> {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult;
}

#[impl_for_tuples(5)]
impl<AccountId> MergeAccount<AccountId> for Tuple {
	fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
		with_transaction_result(|| {
			for_tuples!( #( {
                Tuple::merge_account(source, dest)?;
            } )* );
			Ok(())
		})
	}
}
