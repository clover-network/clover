#![cfg_attr(not(feature = "std"), no_std)]
use super::*;
use codec::{Codec, };
use std::fmt::Display;

pub use clover_rpc_runtime_api::IncentivePoolApi as IncentivePoolRuntimeApi;

pub struct IncentivePool<C, B> {
  client: Arc<C>,
  _marker: std::marker::PhantomData<B>,
}

impl<C, B> IncentivePool<C, B> {
  pub fn new(client: Arc<C>) -> Self {
    IncentivePool{
      client,
      _marker: Default::default(),
    }
  }
}

#[rpc]
pub trait IncentivePoolRpc<BlockHash, AccountId, CurrencyId, Balance, Share> {
  #[rpc(name = "incentive_getAllPools")]
  fn get_all_incentive_pools(&self, at: Option<BlockHash>) -> Result<Vec<(CurrencyId, CurrencyId, String, String)>>;
}

impl<C, Block, AccountId, CurrencyId, Balance, Share> IncentivePoolRpc<<Block as BlockT>::Hash, AccountId, CurrencyId, Balance, Share> for IncentivePool<C, Block>
where
  Block: BlockT,
  C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
  C::Api: IncentivePoolRuntimeApi<Block, AccountId, CurrencyId, Balance, Share>,
  AccountId: Codec,
  CurrencyId: Codec,
  Balance: Codec + Display,
  Share: Codec + Display, {
  fn get_all_incentive_pools(&self,
                             at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(CurrencyId, CurrencyId, String, String)>> {
    let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

    api.get_all_incentive_pools(&at).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).map(|data|
           data.into_iter().map(|(c1, c2, share, balance)| {
             (c1, c2, format!("{}", share), format!("{}", balance))
           })
           .collect())
  }
}

