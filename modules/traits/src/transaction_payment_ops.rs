use sp_runtime::{
  DispatchError,
};

/// trasaction payment trait which treats tx fee and tip separately.
/// so we can implement different charging model for fee and tip.
pub trait TransactionPaymentOps<AccountId, Balance, PositiveImbalance, NegativeImbalance> {
  /// pay transaction fee from the account
  /// returns a pair of imbalance
  /// first is deductable fee(e.g. free transaction discount, staking transaction discount)
  /// the second part is the fee charges from the account balance
  fn payfee(who: &AccountId, tx_fee: Balance, tip: Balance) -> Result<(NegativeImbalance, NegativeImbalance), DispatchError>;
  fn on_payback(who: &AccountId, tx_fee: Balance, tip: Balance) -> Result<(PositiveImbalance, PositiveImbalance), DispatchError>;
}
