pub mod stack;
pub mod builtin;

use sp_std::vec::Vec;
use sp_core::{H160, U256, H256};
use fp_evm::{CallInfo, CreateInfo};
use crate::Trait;

pub trait Runner<T: Trait> {
	type Error: Into<sp_runtime::DispatchError>;

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u32,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CallInfo, Self::Error>;

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u32,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error>;

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u32,
		gas_price: Option<U256>,
		nonce: Option<U256>,
		config: &evm::Config,
	) -> Result<CreateInfo, Self::Error>;
}
