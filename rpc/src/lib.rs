use std::sync::Arc;

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

pub mod currency;
pub mod pair;
pub mod balance;
pub mod exchange;
pub mod incentive_pool;

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
