#![cfg_attr(not(feature = "std"), no_std)]
use super::*;
use clover_primitives::CurrencyId;

pub use clover_rpc_runtime_api::CurrencyPairApi as CurrencyPairRuntimeApi;

#[rpc]
pub trait CurrencyPairRpc<BlockHash> {
  #[rpc(name = "clover_currencyPair")]
  fn currency_pair(&self, at: Option<BlockHash>) -> Result<sp_std::vec::Vec<(CurrencyId, CurrencyId)>>;
}

pub struct CurrencyPair<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> CurrencyPair<C, M> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

impl<C, Block> CurrencyPairRpc<<Block as BlockT>::Hash> for CurrencyPair<C, Block>
where
  Block: BlockT,
  C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
  C::Api: CurrencyPairRuntimeApi<Block>,
{
    fn currency_pair(
        &self,
        at: Option<<Block as BlockT>::Hash>
    ) -> Result<sp_std::vec::Vec<(CurrencyId, CurrencyId)>> {
    let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    api.currency_pair(&at).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    })
    }
}
