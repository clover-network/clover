use std::sync::Arc;
use std::fmt::Display;

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

//pub use self::gen_client::Client as CurrencyBalanceClient;
pub use bitdex_rpc_runtime_api::CurrencyBalanceApi as CurrencyBalanceRuntimeApi;

#[rpc]
pub trait CurrencyBalanceApi<BlockHash, AccountId, CurrencyId, Balance> {
  #[rpc(name = "currencyBalance_balance")]
  fn account_balance(&self, account: AccountId, currency_id: CurrencyId, at: Option<BlockHash>) -> Result<String>;
}

pub mod currency;

/// A struct that implements the [`CurrencyBalanceApi`].
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

impl<C, Block, AccountId, CurrencyId, Balance> CurrencyBalanceApi<<Block as BlockT>::Hash, AccountId, CurrencyId, Balance> for CurrencyBalance<C, Block>
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
    currency_id: CurrencyId, 
    at: Option<<Block as BlockT>::Hash>
  ) -> Result<String> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.account_balance(&at, account, currency_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get value.".into(),
			data: Some(format!("{:?}", e).into()),
		}).map(|v| {
			format!("{}", v)
		})
	}
}