#![cfg_attr(not(feature = "std"), no_std)]
use super::*;
use std::fmt::Display;

use codec::{Codec, Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use clover_rpc_runtime_api::CurrencyExchangeApi as CurrencyExchangeRuntimeApi;

#[derive(Encode, Decode, Eq, PartialEq, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ExchangeInfo<CurrencyId> {
    balance: String,
    routes: Vec<CurrencyId>
}

#[rpc]
pub trait CurrencyExchangeRpc<BlockHash, AccountId, CurrencyId, Balance, Rate, Share> {
  #[rpc(name = "clover_targetAmountAvailable")]
  fn target_amount_available(&self, source: CurrencyId, target: CurrencyId, amount: Balance, at: Option<BlockHash>) -> Result<ExchangeInfo<CurrencyId>>;

  #[rpc(name = "clover_supplyAmountNeeded")]
  fn supply_amount_needed(&self, source: CurrencyId, target: CurrencyId, amount: Balance, at: Option<BlockHash>) -> Result<ExchangeInfo<CurrencyId>>;

  #[rpc(name = "clover_getLiquidity")]
  fn get_liquidity(&self, account: Option<AccountId>, at: Option<BlockHash>) -> Result<Vec<(CurrencyId, CurrencyId, String, String, String, String, String)>>;

  #[rpc(name = "clover_getExchangeRate")]
  fn get_exchange_rate(&self, at: Option<BlockHash>) -> Result<Rate>;

  #[rpc(name = "clover_toAddLiquidity")]
  fn to_add_liquidity(&self, source: CurrencyId, target: CurrencyId, source_amount: Balance, target_amount: Balance, at: Option<BlockHash>) -> Result<(String, String)>;

  #[rpc(name="clover_getAccountStakingInfo")]
  fn get_account_staking_info(&self,
                              account: AccountId,
                              currency_first: CurrencyId,
                              currency_second: CurrencyId,
                              at: Option<BlockHash>) -> Result<(String, String)>;
}

pub struct CurrencyExchange<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> CurrencyExchange<C, M> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _marker: Default::default() }
    }
}

impl<C, Block, AccountId, CurrencyId, Balance, Rate, Share> CurrencyExchangeRpc<<Block as BlockT>::Hash, AccountId, CurrencyId, Balance, Rate, Share> for CurrencyExchange<C, Block>
where
  Block: BlockT,
  C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: CurrencyExchangeRuntimeApi<Block, AccountId, CurrencyId, Balance, Rate, Share>,
    AccountId: Codec,
    CurrencyId: Codec,
  Balance: Codec + Display,
    Rate: Codec,
    Share: Codec + Display,
{
    fn target_amount_available(&self, source: CurrencyId, target: CurrencyId, amount: Balance, at: Option<<Block as BlockT>::Hash>) -> Result<ExchangeInfo<CurrencyId>> {
        let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    api.target_amount_available(&at, source, target, amount).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).map(|(b, r)| {
            ExchangeInfo {
                balance: format!("{}", b),
                routes: r
            }
    })
    }

    fn supply_amount_needed(&self, source: CurrencyId, target: CurrencyId, amount: Balance, at: Option<<Block as BlockT>::Hash>) -> Result<ExchangeInfo<CurrencyId>> {
        let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    api.supply_amount_needed(&at, source, target, amount).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).map(|(b, r)| {
            ExchangeInfo {
                balance: format!("{}", b),
                routes: r
            }
    })
    }

    fn get_liquidity(&self, account: Option<AccountId>, at: Option<<Block as BlockT>::Hash>) -> Result<Vec<(CurrencyId, CurrencyId, String, String, String, String, String)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));
        let info = api.get_liquidity(&at, account).map_err(|e| RpcError {
            code: ErrorCode::ServerError(Error::RuntimeError.into()),
            message: "Unable to get value.".into(),
            data: Some(format!("{:?}", e).into()),
        }).unwrap().into_iter().map(|(c1, c2, b1, b2, s1, s2, s3)| {
            (c1, c2, format!("{}", b1), format!("{}", b2), format!("{}", s1), format!("{}", s2), format!("{}", s3))
        }).collect();
        Ok(info)
    }

    fn get_exchange_rate(&self, at: Option<<Block as BlockT>::Hash>) -> Result<Rate> {
        let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    api.get_exchange_rate(&at).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    })
    }


    fn to_add_liquidity(&self, source: CurrencyId, target: CurrencyId, source_amount: Balance, target_amount: Balance, at: Option<<Block as BlockT>::Hash>) -> Result<(String, String)> {
        let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
      // If the block hash is not supplied assume the best block.
      self.client.info().best_hash));
    api.to_add_liquidity(&at, source, target, source_amount, target_amount).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).map(|(s1, s2)| {
            (format!("{}", s1), format!("{}", s2))
    })
    }

  fn get_account_staking_info(&self, account: AccountId,
                              currency_left: CurrencyId,
                              currency_right: CurrencyId,
                              at: Option<<Block as BlockT>::Hash>) -> Result<(String, String)> {
    let api = self.client.runtime_api();
    let at = BlockId::hash(at.unwrap_or_else(||
                                             // If the block hash is not supplied assume the best block.
                                             self.client.info().best_hash));

    api.get_staking_info(&at, account, currency_left, currency_right).map_err(|e| RpcError {
      code: ErrorCode::ServerError(Error::RuntimeError.into()),
      message: "Unable to get value.".into(),
      data: Some(format!("{:?}", e).into()),
    }).map(|(s1, s2)| {
      (format!("{}", s1), format!("{}", s2))
    })
  }
}
