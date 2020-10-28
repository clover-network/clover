use super::*;

use std::fmt::Display;
use codec::Codec;

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
pub use clover_rpc_runtime_api::CurrencyBalanceApi as CurrencyBalanceRuntimeApi;

#[rpc]
pub trait CurrencyBalanceRpc<BlockHash, AccountId, CurrencyId, Balance> {
  #[rpc(name = "clover_getBalance")]
  fn account_balance(&self, account: AccountId, currency_id: Option<CurrencyId>, at: Option<BlockHash>) -> Result<sp_std::vec::Vec<(CurrencyId, String)>>;
}

pub struct CurrencyBalance<C, B> {
  client: Arc<C>,
  _marker: std::marker::PhantomData<B>,
}

impl<C, B> CurrencyBalance<C, B> {
  pub fn new(client: Arc<C>) -> Self {
    CurrencyBalance{
      client,
      _marker: Default::default(),
    }
  }
}

pub enum Error {
  RuntimeError,
}

impl From<Error> for i64 {
  fn from(e: Error) -> i64 {
    match e {
      Error::RuntimeError => 1,
    }
  }
}

impl<C, Block, AccountId, CurrencyId, Balance> CurrencyBalanceRpc<<Block as BlockT>::Hash, AccountId, CurrencyId, Balance> for CurrencyBalance<C, Block>
where
  Block: BlockT,
  C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
  C::Api: CurrencyBalanceRuntimeApi<Block, AccountId, CurrencyId, Balance>,
  AccountId: Codec,
  CurrencyId: Codec,
  Balance: Codec + Display,
{

  fn account_balance(&self,
    account: AccountId,
    currency_id: Option<CurrencyId>,
    at: Option<<Block as BlockT>::Hash>
  ) -> Result<sp_std::vec::Vec<(CurrencyId, String)>> {
    let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    let balances = api.account_balance(&at, account, currency_id).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).unwrap().into_iter().map(|(cid, balance)| {
      (cid, format!("{}", balance))
    }).collect();
      Ok(balances)
  }
}
