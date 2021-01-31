use std::{marker::PhantomData, sync::Arc};
use std::collections::BTreeMap;
use ethereum::{
	Block as EthereumBlock, Transaction as EthereumTransaction,
	TransactionMessage as EthereumTransactionMessage,
};
use frame_support::debug;
use ethereum_types::{H160, H256, H64, U256, U64, H512};
use jsonrpc_core::{BoxFuture, Result, futures::future::{self, Future}};
use futures::future::TryFutureExt;
use sp_runtime::{
	traits::{Block as BlockT, UniqueSaturatedInto, Zero, One, Saturating, BlakeTwo256},
	transaction_validity::TransactionSource
};
use sp_api::{ProvideRuntimeApi, BlockId, Core};
use sp_transaction_pool::{TransactionPool, InPoolTransaction};
use sc_client_api::backend::{StorageProvider, Backend, StateBackend, AuxStore};
use sha3::{Keccak256, Digest};
use sp_blockchain::{Error as BlockChainError, HeaderMetadata, HeaderBackend};
use sc_network::{NetworkService, ExHashT};
use fc_rpc_core::{EthApi as EthApiT, NetApi as NetApiT, Web3Api as Web3ApiT};
use fc_rpc_core::types::{
	BlockNumber, Bytes, CallRequest, Filter, FilteredParams, Index, Log, Receipt, RichBlock,
	SyncStatus, SyncInfo, Transaction, Work, Rich, Block, BlockTransactions, VariadicValue,
	TransactionRequest, InternalTransaction
};
use fp_rpc::{EthereumRuntimeRPCApi, ConvertTransaction, TransactionStatus};
use crate::{internal_err, error_on_execution_failure, EthSigner};

pub use fc_rpc_core::{EthApiServer, NetApiServer, Web3ApiServer};
use codec::{self, Encode};

pub struct EthApi<B: BlockT, C, P, CT, BE, H: ExHashT> {
	pool: Arc<P>,
	client: Arc<C>,
	convert_transaction: CT,
	network: Arc<NetworkService<B, H>>,
	is_authority: bool,
	signers: Vec<Box<dyn EthSigner>>,
	_marker: PhantomData<(B, BE)>,
}

impl<B: BlockT, C, P, CT, BE, H: ExHashT> EthApi<B, C, P, CT, BE, H> {
	pub fn new(
		client: Arc<C>,
		pool: Arc<P>,
		convert_transaction: CT,
		network: Arc<NetworkService<B, H>>,
		signers: Vec<Box<dyn EthSigner>>,
		is_authority: bool,
	) -> Self {
		Self {
			client,
			pool,
			convert_transaction,
			network,
			is_authority,
			signers,
			_marker: PhantomData,
		}
	}
}

fn rich_block_build(
	block: ethereum::Block,
	statuses: Vec<Option<TransactionStatus>>,
	hash: Option<H256>,
	full_transactions: bool
) -> RichBlock {
	Rich {
		inner: Block {
			hash: Some(hash.unwrap_or_else(|| {
				H256::from_slice(
					Keccak256::digest(&rlp::encode(&block.header)).as_slice()
				)
			})),
			parent_hash: block.header.parent_hash,
			uncles_hash: block.header.ommers_hash,
			author: block.header.beneficiary,
			miner: block.header.beneficiary,
			state_root: block.header.state_root,
			transactions_root: block.header.transactions_root,
			receipts_root: block.header.receipts_root,
			number: Some(block.header.number),
			gas_used: block.header.gas_used,
			gas_limit: block.header.gas_limit,
			extra_data: Bytes(block.header.extra_data.clone()),
			logs_bloom: Some(block.header.logs_bloom),
			timestamp: U256::from(block.header.timestamp / 1000),
			difficulty: block.header.difficulty,
			total_difficulty: None,
			seal_fields: vec![
				Bytes(block.header.mix_hash.as_bytes().to_vec()),
				Bytes(block.header.nonce.as_bytes().to_vec())
			],
			uncles: vec![],
			transactions: {
				if full_transactions {
					BlockTransactions::Full(
						block.transactions.iter().enumerate().map(|(index, transaction)|{
							transaction_build(
								transaction.clone(),
								block.clone(),
								statuses[index].clone().unwrap_or_default()
							)
						}).collect()
					)
				} else {
					BlockTransactions::Hashes(
						block.transactions.iter().map(|transaction|{
							H256::from_slice(
								Keccak256::digest(&rlp::encode(&transaction.clone())).as_slice()
							)
						}).collect()
					)
				}
			},
			size: Some(U256::from(rlp::encode(&block).len() as u32))
		},
		extra_info: BTreeMap::new()
	}
}

fn transaction_build(
	transaction: EthereumTransaction,
	block: EthereumBlock,
	status: TransactionStatus
) -> Transaction {
	let mut sig = [0u8; 65];
	let mut msg = [0u8; 32];
	sig[0..32].copy_from_slice(&transaction.signature.r()[..]);
	sig[32..64].copy_from_slice(&transaction.signature.s()[..]);
	sig[64] = transaction.signature.standard_v();
	msg.copy_from_slice(&EthereumTransactionMessage::from(transaction.clone()).hash()[..]);

	let pubkey = match sp_io::crypto::secp256k1_ecdsa_recover(&sig, &msg) {
		Ok(p) => Some(H512::from(p)),
		Err(_e) => None,
	};

	Transaction {
		hash: H256::from_slice(
			Keccak256::digest(&rlp::encode(&transaction)).as_slice()
		),
		nonce: transaction.nonce,
		block_hash: Some(H256::from_slice(
			Keccak256::digest(&rlp::encode(&block.header)).as_slice()
		)),
		block_number: Some(block.header.number),
		transaction_index: Some(U256::from(
			UniqueSaturatedInto::<u32>::unique_saturated_into(
				status.transaction_index
			)
		)),
		from: status.from,
		to: status.to,
		value: transaction.value,
		gas_price: transaction.gas_price,
		gas: transaction.gas_limit,
		input: Bytes(transaction.clone().input),
		creates: status.contract_address,
		raw: Bytes(rlp::encode(&transaction)),
		public_key: pubkey,
		chain_id: transaction.signature.chain_id().map(U64::from),
		standard_v: U256::from(transaction.signature.standard_v()),
		v: U256::from(transaction.signature.v()),
		r: U256::from(transaction.signature.r().as_bytes()),
		s: U256::from(transaction.signature.s().as_bytes()),
	}
}

impl<B, C, P, CT, BE, H: ExHashT> EthApi<B, C, P, CT, BE, H> where
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE> + AuxStore,
	C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError> + 'static,
	C::Api: EthereumRuntimeRPCApi<B>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	B: BlockT<Hash=H256> + Send + Sync + 'static,
	C: Send + Sync + 'static,
	P: TransactionPool<Block=B> + Send + Sync + 'static,
	CT: ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	fn native_block_id(&self, number: Option<BlockNumber>) -> Result<Option<BlockId<B>>> {
		Ok(match number.unwrap_or(BlockNumber::Latest) {
			BlockNumber::Hash { hash, .. } => {
				self.load_hash(hash).unwrap_or(None)
			},
			BlockNumber::Num(number) => {
				Some(BlockId::Number(number.unique_saturated_into()))
			},
			BlockNumber::Latest => {
				Some(BlockId::Hash(
					self.client.info().best_hash
				))
			},
			BlockNumber::Earliest => {
				Some(BlockId::Number(Zero::zero()))
			},
			BlockNumber::Pending => {
				None
			}
		})
	}

	// Asumes there is only one mapped canonical block in the AuxStore, otherwise something is wrong
	fn load_hash(&self, hash: H256) -> Result<Option<BlockId<B>>> {
		let hashes = match fc_consensus::load_block_hash::<B, _>(self.client.as_ref(), hash)
			.map_err(|err| internal_err(format!("fetch aux store failed: {:?}", err)))?
		{
			Some(hashes) => hashes,
			None => return Ok(None),
		};
		let out: Vec<H256> = hashes.into_iter()
			.filter_map(|h| {
				if let Ok(Some(_)) = self.client.header(BlockId::Hash(h)) {
					Some(h)
				} else {
					None
				}
			}).collect();

		if out.len() == 1 {
			return Ok(Some(
				BlockId::Hash(out[0])
			));
		}
		Ok(None)
	}
}

impl<B, C, P, CT, BE, H: ExHashT> EthApiT for EthApi<B, C, P, CT, BE, H> where
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE> + AuxStore,
	C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError> + 'static,
	C::Api: EthereumRuntimeRPCApi<B>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	B: BlockT<Hash=H256> + Send + Sync + 'static,
	C: Send + Sync + 'static,
	P: TransactionPool<Block=B> + Send + Sync + 'static,
	CT: ConvertTransaction<<B as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	fn protocol_version(&self) -> Result<u64> {
		Ok(1)
	}

	fn syncing(&self) -> Result<SyncStatus> {
		if self.network.is_major_syncing() {
			let best_number: u32 = self.client.info().best_number.clone().unique_saturated_into();
			let block_number = U256::from(best_number);
			Ok(SyncStatus::Info(SyncInfo {
				starting_block: U256::zero(),
				current_block: block_number,
				// TODO `highest_block` is not correct, should load `best_seen_block` from NetworkWorker,
				// but afaik that is not currently possible in Substrate:
				// https://github.com/paritytech/substrate/issues/7311
				highest_block: block_number,
				warp_chunks_amount: None,
				warp_chunks_processed: None,
			}))
		} else {
			Ok(SyncStatus::None)
		}
	}

	fn hashrate(&self) -> Result<U256> {
		Ok(U256::zero())
	}

	fn author(&self) -> Result<H160> {
		let hash = self.client.info().best_hash;

		Ok(
			self.client
			.runtime_api()
			.author(&BlockId::Hash(hash))
			.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?.into()
		)
	}

	fn is_mining(&self) -> Result<bool> {
		Ok(self.is_authority)
	}

	fn chain_id(&self) -> Result<Option<U64>> {
		let hash = self.client.info().best_hash;
		Ok(Some(self.client.runtime_api().chain_id(&BlockId::Hash(hash))
				.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?.into()))
	}

	fn gas_price(&self) -> Result<U256> {
		let hash = self.client.info().best_hash;
		Ok(
			self.client
				.runtime_api()
				.gas_price(&BlockId::Hash(hash))
				.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?
				.into(),
		)
	}

	fn accounts(&self) -> Result<Vec<H160>> {
		let mut accounts = Vec::new();
		for signer in &self.signers {
			accounts.append(&mut signer.accounts());
		}
		Ok(accounts)
	}

	fn block_number(&self) -> Result<U256> {
		let block_number: u32 = self.client.info().best_number.clone().unique_saturated_into();
		Ok(U256::from(block_number))
	}

	fn balance(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
		if let Ok(Some(id)) = self.native_block_id(number) {
			return Ok(
				self.client
					.runtime_api()
					.account_basic(&id, address)
					.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?
					.balance.into(),
			);
		}
		Ok(U256::zero())
	}

	fn storage_at(&self, address: H160, index: U256, number: Option<BlockNumber>) -> Result<H256> {
		if let Ok(Some(id)) = self.native_block_id(number) {
			return Ok(
				self.client
					.runtime_api()
					.storage_at(&id, address, index)
					.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?
					.into(),
			);
		}
		Ok(H256::default())
	}

	fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>> {
		let id = match self.load_hash(hash)
			.map_err(|err| internal_err(format!("{:?}", err)))?
		{
			Some(hash) => hash,
			_ => return Ok(None),
		};

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses) {
			(Some(block), Some(statuses)) => {
				Ok(Some(rich_block_build(
					block,
					statuses.into_iter().map(|s| Some(s)).collect(),
					Some(hash),
					full,
				)))
			},
			_ => {
				Ok(None)
			},
		}
	}

	fn block_by_number(&self, number: BlockNumber, full: bool) -> Result<Option<RichBlock>> {
		let id = match self.native_block_id(Some(number))? {
			Some(id) => id,
			None => return Ok(None),
		};

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses) {
			(Some(block), Some(statuses)) => {
				let hash = H256::from_slice(
					Keccak256::digest(&rlp::encode(&block.header)).as_slice(),
				);

				Ok(Some(rich_block_build(
					block,
					statuses.into_iter().map(|s| Some(s)).collect(),
					Some(hash),
					full,
				)))
			},
			_ => {
				Ok(None)
			},
		}
	}

	fn transaction_count(&self, address: H160, number: Option<BlockNumber>) -> Result<U256> {
		if let Some(BlockNumber::Pending) = number {
			// Find future nonce
			let id = BlockId::hash(self.client.info().best_hash);
			let nonce: U256 = self.client.runtime_api()
				.account_basic(&id, address)
				.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?
				.nonce;

			let mut current_nonce = nonce;
			let mut current_tag = (address, nonce).encode();
			for tx in self.pool.ready() {
				// since transactions in `ready()` need to be ordered by nonce
				// it's fine to continue with current iterator.
				if tx.provides().get(0) == Some(&current_tag) {
					current_nonce = current_nonce.saturating_add(1.into());
					current_tag = (address, current_nonce).encode();
				}
			}

			return Ok(current_nonce);
		}

		let id = match self.native_block_id(number)? {
			Some(id) => id,
			None => return Ok(U256::zero()),
		};

		let nonce = self.client.runtime_api()
			.account_basic(&id, address)
			.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?
			.nonce.into();

		Ok(nonce)
	}

	fn block_transaction_count_by_hash(&self, hash: H256) -> Result<Option<U256>> {
		let id = match self.load_hash(hash)
			.map_err(|err| internal_err(format!("{:?}", err)))?
		{
			Some(hash) => hash,
			_ => return Ok(None),
		};

		let block = self.client.runtime_api()
			.current_block(&id)
			.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?;

		match block {
			Some(block) => Ok(Some(U256::from(block.transactions.len()))),
			None => Ok(None),
		}
	}

	fn block_transaction_count_by_number(&self, number: BlockNumber) -> Result<Option<U256>> {
		let id = match self.native_block_id(Some(number))? {
			Some(id) => id,
			None => return Ok(None),
		};

		let block = self.client.runtime_api()
			.current_block(&id)
			.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?;

		match block {
			Some(block) => Ok(Some(U256::from(block.transactions.len()))),
			None => Ok(None),
		}
	}

	fn block_uncles_count_by_hash(&self, _: H256) -> Result<U256> {
		Ok(U256::zero())
	}

	fn block_uncles_count_by_number(&self, _: BlockNumber) -> Result<U256> {
		Ok(U256::zero())
	}

	fn code_at(&self, address: H160, number: Option<BlockNumber>) -> Result<Bytes> {
		if let Ok(Some(id)) = self.native_block_id(number) {
			return Ok(
				self.client
					.runtime_api()
					.account_code_at(&id, address)
					.map_err(|err| internal_err(format!("fetch runtime chain id failed: {:?}", err)))?
					.into(),
			);
		}
		Ok(Bytes(vec![]))
	}

	fn send_transaction(&self, request: TransactionRequest) -> BoxFuture<H256> {
		let from = match request.from {
			Some(from) => from,
			None => {
				let accounts = match self.accounts() {
					Ok(accounts) => accounts,
					Err(e) => return Box::new(future::result(Err(e))),
				};

				match accounts.get(0) {
					Some(account) => account.clone(),
					None => return Box::new(future::result(Err(internal_err("no signer available")))),
				}
			},
		};

		let nonce = match request.nonce {
			Some(nonce) => nonce,
			None => {
				match self.transaction_count(from, None) {
					Ok(nonce) => nonce,
					Err(e) => return Box::new(future::result(Err(e))),
				}
			},
		};

		let chain_id = match self.chain_id() {
			Ok(chain_id) => chain_id,
			Err(e) => return Box::new(future::result(Err(e))),
		};

		let message = ethereum::TransactionMessage {
			nonce,
			gas_price: request.gas_price.unwrap_or(U256::from(1)),
			gas_limit: request.gas.unwrap_or(U256::max_value()),
			value: request.value.unwrap_or(U256::zero()),
			input: request.data.map(|s| s.into_vec()).unwrap_or_default(),
			action: match request.to {
				Some(to) => ethereum::TransactionAction::Call(to),
				None => ethereum::TransactionAction::Create,
			},
			chain_id: chain_id.map(|s| s.as_u64()),
		};

		let mut transaction = None;

		for signer in &self.signers {
			if signer.accounts().contains(&from) {
				match signer.sign(message, &from) {
					Ok(t) => transaction = Some(t),
					Err(e) => return Box::new(future::result(Err(e))),
				}
				break
			}
		}

		let transaction = match transaction {
			Some(transaction) => transaction,
			None => return Box::new(future::result(Err(internal_err("no signer available")))),
		};
		let transaction_hash = H256::from_slice(
			Keccak256::digest(&rlp::encode(&transaction)).as_slice()
		);
		let hash = self.client.info().best_hash;
		Box::new(
			self.pool
				.submit_one(
					&BlockId::hash(hash),
					TransactionSource::Local,
					self.convert_transaction.convert_transaction(transaction),
				)
				.compat()
				.map(move |_| transaction_hash)
				.map_err(|err| internal_err(format!("submit transaction to pool failed: {:?}", err)))
		)
	}

	fn send_raw_transaction(&self, bytes: Bytes) -> BoxFuture<H256> {
		let transaction = match rlp::decode::<ethereum::Transaction>(&bytes.0[..]) {
			Ok(transaction) => transaction,
			Err(_) => return Box::new(
				future::result(Err(internal_err("decode transaction failed")))
			),
		};
		let transaction_hash = H256::from_slice(
			Keccak256::digest(&rlp::encode(&transaction)).as_slice()
		);
		let hash = self.client.info().best_hash;
		Box::new(
			self.pool
				.submit_one(
					&BlockId::hash(hash),
					TransactionSource::Local,
					self.convert_transaction.convert_transaction(transaction),
				)
				.compat()
				.map(move |_| transaction_hash)
				.map_err(|err| internal_err(format!("submit transaction to pool failed: {:?}", err)))
		)
	}

	fn call(&self, request: CallRequest, _: Option<BlockNumber>) -> Result<Bytes> {
		let hash = self.client.info().best_hash;

		let CallRequest {
			from,
			to,
			gas_price,
			gas,
			value,
			data,
			nonce
		} = request;

		let gas_limit = gas.unwrap_or(U256::max_value());
		let data = data.map(|d| d.0).unwrap_or_default();

		match to {
			Some(to) => {
				let info = self.client.runtime_api()
					.call(
						&BlockId::Hash(hash),
						from.unwrap_or_default(),
						to,
						data,
						value.unwrap_or_default(),
						gas_limit,
						gas_price,
						nonce,
						false,
					)
					.map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
					.map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

				error_on_execution_failure(&info.exit_reason, &info.value)?;

				Ok(Bytes(info.value))
			},
			None => {
				let info = self.client.runtime_api()
					.create(
						&BlockId::Hash(hash),
						from.unwrap_or_default(),
						data,
						value.unwrap_or_default(),
						gas_limit,
						gas_price,
						nonce,
						false,
					)
					.map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
					.map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

				error_on_execution_failure(&info.exit_reason, &[])?;

				Ok(Bytes(info.value[..].to_vec()))
			},
		}
	}

	fn estimate_gas(&self, request: CallRequest, _: Option<BlockNumber>) -> Result<U256> {
		let hash = self.client.info().best_hash;

		let CallRequest {
			from,
			to,
			gas_price,
			gas,
			value,
			data,
			nonce
		} = request;

		let gas_limit = gas.unwrap_or(U256::max_value());
		let data = data.map(|d| d.0).unwrap_or_default();
		debug::info!("estimate gas hash: {:?}, data: {:?}", hash, data);
		let used_gas = match to {
			Some(to) => {
				let info = self.client.runtime_api()
					.call(
						&BlockId::Hash(hash),
						from.unwrap_or_default(),
						to,
						data,
						value.unwrap_or_default(),
						gas_limit,
						gas_price,
						nonce,
						true,
					)
					.map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
					.map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

				error_on_execution_failure(&info.exit_reason, &info.value)?;

				info.used_gas
			},
			None => {
				let info = self.client.runtime_api()
					.create(
						&BlockId::Hash(hash),
						from.unwrap_or_default(),
						data,
						value.unwrap_or_default(),
						gas_limit,
						gas_price,
						nonce,
						true
					)
					.map_err(|err| internal_err(format!("runtime error: {:?}", err)))?
					.map_err(|err| internal_err(format!("execution fatal: {:?}", err)))?;

				error_on_execution_failure(&info.exit_reason, &[])?;

				info.used_gas
			},
		};

		Ok(used_gas)
	}

	fn fast_estimate_gas(&self, request: CallRequest, block_number: Option<BlockNumber>) -> Result<U256> {
		let mut test_request = request.clone();
		test_request.gas = Some(U256::max_value());
		let used_gas = self.fast_estimate_gas(test_request, block_number.clone())?;
		let mut fist_request = request.clone();
		fist_request.gas = Some(used_gas);
		let first_result = self.fast_estimate_gas(fist_request, block_number.clone());
		match first_result {
			// in most cases, estimate gas will work
			Ok(used_gas) => {
				Ok(used_gas)
			}
			// do binary search
			Err(_) => {
				let mut lower = used_gas;
				let mut upper = used_gas * 64 / 63;
				let mut mid = upper;
				let mut best = mid;
				while lower + 1 < upper {
					mid = (lower + upper + 1) / 2;
					let mut test_request = request.clone();
					test_request.gas = Some(mid);
					let test_result = self.fast_estimate_gas(test_request, block_number.clone());
					if test_result.is_ok() {
						upper = mid;
						best = mid;
					} else {
						lower = mid;
					}
				}
				Ok(best)
			}
		}
	}

	fn transaction_by_hash(&self, hash: H256) -> Result<Option<Transaction>> {
		let (hash, index) = match fc_consensus::load_transaction_metadata(
			self.client.as_ref(),
			hash,
		).map_err(|err| internal_err(format!("fetch aux store failed: {:?})", err)))? {
			Some((hash, index)) => (hash, index as usize),
			None => return Ok(None),
		};

		let id = match self.load_hash(hash)
			.map_err(|err| internal_err(format!("{:?}", err)))?
		{
			Some(hash) => hash,
			_ => return Ok(None),
		};

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses) {
			(Some(block), Some(statuses)) => {
				Ok(Some(transaction_build(
					block.transactions[index].clone(),
					block,
					statuses[index].clone(),
				)))
			},
			_ => Ok(None)
		}
	}

	fn transaction_by_block_hash_and_index(
		&self,
		hash: H256,
		index: Index,
	) -> Result<Option<Transaction>> {
		let id = match self.load_hash(hash)
			.map_err(|err| internal_err(format!("{:?}", err)))?
		{
			Some(hash) => hash,
			_ => return Ok(None),
		};
		let index = index.value();

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses) {
			(Some(block), Some(statuses)) => {
				Ok(Some(transaction_build(
					block.transactions[index].clone(),
					block,
					statuses[index].clone(),
				)))
			},
			_ => Ok(None)
		}
	}

	fn transaction_by_block_number_and_index(
		&self,
		number: BlockNumber,
		index: Index,
	) -> Result<Option<Transaction>> {
		let id = match self.native_block_id(Some(number))? {
			Some(id) => id,
			None => return Ok(None),
		};
		let index = index.value();

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses) {
			(Some(block), Some(statuses)) => {
				Ok(Some(transaction_build(
					block.transactions[index].clone(),
					block,
					statuses[index].clone(),
				)))
			},
			_ => Ok(None)
		}
	}

	fn transaction_receipt(&self, hash: H256) -> Result<Option<Receipt>> {
		let (hash, index) = match fc_consensus::load_transaction_metadata(
			self.client.as_ref(),
			hash,
		).map_err(|err| internal_err(format!("fetch aux store failed : {:?}", err)))? {
			Some((hash, index)) => (hash, index as usize),
			None => return Ok(None),
		};

		let id = match self.load_hash(hash)
			.map_err(|err| internal_err(format!("{:?}", err)))?
		{
			Some(hash) => hash,
			_ => return Ok(None),
		};

		let block = self.client.runtime_api().current_block(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let receipts = self.client.runtime_api().current_receipts(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;
		let statuses = self.client.runtime_api().current_transaction_statuses(&id)
			.map_err(|err| internal_err(format!("call runtime failed: {:?}", err)))?;

		match (block, statuses, receipts) {
			(Some(block), Some(statuses), Some(receipts)) => {
				let block_hash = H256::from_slice(
					Keccak256::digest(&rlp::encode(&block.header)).as_slice()
				);
				let receipt = receipts[index].clone();
				let status = statuses[index].clone();
				let mut cumulative_receipts = receipts.clone();
				cumulative_receipts.truncate((status.transaction_index + 1) as usize);

				return Ok(Some(Receipt {
					transaction_hash: Some(status.transaction_hash),
					transaction_index: Some(status.transaction_index.into()),
					block_hash: Some(block_hash),
					from: Some(status.from),
					to: status.to,
					block_number: Some(block.header.number),
					cumulative_gas_used: {
						let cumulative_gas: u32 = cumulative_receipts.iter().map(|r| {
							r.used_gas.as_u32()
						}).sum();
						U256::from(cumulative_gas)
					},
					gas_used: Some(receipt.used_gas),
					contract_address: status.contract_address,

					logs: {
						let mut pre_receipts_log_index = None;
						if cumulative_receipts.len() > 0 {
							cumulative_receipts.truncate(cumulative_receipts.len() - 1);
							pre_receipts_log_index = Some(cumulative_receipts.iter().map(|r| {
								r.logs.len() as u32
							}).sum::<u32>());
						}
						receipt.logs.iter().enumerate().map(|(i, log)| {
							Log {
								address: log.address,
								topics: log.topics.clone(),
								data: Bytes(log.data.clone()),
								block_hash: Some(block_hash),
								block_number: Some(block.header.number),
								transaction_hash: Some(hash),
								transaction_index: Some(status.transaction_index.into()),
								log_index: Some(U256::from(
									(pre_receipts_log_index.unwrap_or(0)) + i as u32
								)),
								transaction_log_index: Some(U256::from(i)),
								removed: false,
							}
						}).collect()
					},
					status_code: Some(U64::from(receipt.state_root.to_low_u64_be())),
					logs_bloom: receipt.logs_bloom,
					state_root: None,
					internal_transactions: status.internal_transactions.iter().enumerate().map(|(_i, x)| {
						InternalTransaction {
							from: Some(x.parent),
							to: Some(x.node),
							gas_used: Some(x.gas_used),
							developer: x.developer,
							developer_reward: x.developer_reward,
						}
					}).collect(),
				}))
			}
			_ => Ok(None),
		}
	}

	fn uncle_by_block_hash_and_index(&self, _: H256, _: Index) -> Result<Option<RichBlock>> {
		Ok(None)
	}

	fn uncle_by_block_number_and_index(
		&self,
		_: BlockNumber,
		_: Index,
	) -> Result<Option<RichBlock>> {
		Ok(None)
	}

	fn logs(&self, filter: Filter) -> Result<Vec<Log>> {
		let mut blocks_and_statuses = Vec::new();
		let mut ret = Vec::new();
		let params = FilteredParams::new(Some(filter.clone()));

		if let Some(hash) = filter.block_hash {
			let id = match self.load_hash(hash)
				.map_err(|err| internal_err(format!("{:?}", err)))?
			{
				Some(hash) => hash,
				_ => return Ok(Vec::new()),
			};

			let (block, _, statuses) = self.client.runtime_api()
				.current_all(&id)
				.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?;

			if let (Some(block), Some(statuses)) = (block, statuses) {
				blocks_and_statuses.push((block, statuses));
			}
		} else {
			let mut current_number = filter.to_block
				.and_then(|v| v.to_min_block_num())
				.map(|s| s.unique_saturated_into())
				.unwrap_or(
					self.client.info().best_number
				);

			let from_number = filter.from_block
				.and_then(|v| v.to_min_block_num())
				.map(|s| s.unique_saturated_into())
				.unwrap_or(
					self.client.info().best_number
				);
			while current_number >= from_number {
				let id = BlockId::Number(current_number);

				let (block, _, statuses) = self.client.runtime_api()
					.current_all(&id)
					.map_err(|err| internal_err(format!("fetch runtime account basic failed: {:?}", err)))?;

				if let (Some(block), Some(statuses)) = (block, statuses) {
					blocks_and_statuses.push((block, statuses));
				}

				if current_number == Zero::zero() {
					break
				} else {
					current_number = current_number.saturating_sub(One::one());
				}
			}
		}

		for (block, statuses) in blocks_and_statuses {
			let mut block_log_index: u32 = 0;
			let block_hash = H256::from_slice(
				Keccak256::digest(&rlp::encode(&block.header)).as_slice()
			);
			for status in statuses.iter() {
				let logs = status.logs.clone();
				let mut transaction_log_index: u32 = 0;
				let transaction_hash = status.transaction_hash;
				for ethereum_log in logs {
					let mut log = Log {
						address: ethereum_log.address.clone(),
						topics: ethereum_log.topics.clone(),
						data: Bytes(ethereum_log.data.clone()),
						block_hash: None,
						block_number: None,
						transaction_hash: None,
						transaction_index: None,
						log_index: None,
						transaction_log_index: None,
						removed: false,
					};
					let mut add: bool = false;
					if let (
						Some(VariadicValue::Single(_)),
						Some(VariadicValue::Multiple(_))
					) = (
						filter.address.clone(),
						filter.topics.clone(),
					) {
						if !params.filter_address(&log) && params.filter_topics(&log) {
							add = true;
						}
					} else if let Some(VariadicValue::Single(_)) = filter.address {
						if !params.filter_address(&log) {
							add = true;
						}
					} else if let Some(VariadicValue::Multiple(_)) = &filter.topics {
						if params.filter_topics(&log) {
							add = true;
						}
					} else {
						add = true;
					}
					if add {
						log.block_hash = Some(block_hash);
						log.block_number = Some(block.header.number.clone());
						log.transaction_hash = Some(transaction_hash);
						log.transaction_index = Some(U256::from(status.transaction_index));
						log.log_index = Some(U256::from(block_log_index));
						log.transaction_log_index = Some(U256::from(transaction_log_index));
						ret.push(log);
					}
					transaction_log_index += 1;
					block_log_index += 1;
				}
			}
		}

		Ok(ret)
	}

	fn work(&self) -> Result<Work> {
		Ok(Work {
			pow_hash: H256::default(),
			seed_hash: H256::default(),
			target: H256::default(),
			number: None,
		})
	}

	fn submit_work(&self, _: H64, _: H256, _: H256) -> Result<bool> {
		Ok(false)
	}

	fn submit_hashrate(&self, _: U256, _: H256) -> Result<bool> {
		Ok(false)
	}
}

pub struct NetApi<B: BlockT, BE, C, H: ExHashT> {
	client: Arc<C>,
	network: Arc<NetworkService<B, H>>,
	_marker: PhantomData<BE>,
}

impl<B: BlockT, BE, C, H: ExHashT> NetApi<B, BE, C, H> {
	pub fn new(
		client: Arc<C>,
		network: Arc<NetworkService<B, H>>,
	) -> Self {
		Self {
			client,
			network,
			_marker: PhantomData,
		}
	}
}

impl<B: BlockT, BE, C, H: ExHashT> NetApiT for NetApi<B, BE, C, H> where
	C: ProvideRuntimeApi<B> + StorageProvider<B, BE> + AuxStore,
	C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError> + 'static,
	C::Api: EthereumRuntimeRPCApi<B>,
	BE: Backend<B> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C: Send + Sync + 'static,
	B: BlockT<Hash=H256> + Send + Sync + 'static,
{
	fn is_listening(&self) -> Result<bool> {
		Ok(true)
	}

	fn peer_count(&self) -> Result<String> {
		Ok((self.network.num_connected() as u32).to_string())
	}

	fn version(&self) -> Result<String> {
		let hash = self.client.info().best_hash;
		Ok(self.client.runtime_api().chain_id(&BlockId::Hash(hash))
			.map_err(|_| internal_err("fetch runtime chain id failed"))?.to_string())
	}
}

pub struct Web3Api<B, C> {
	client: Arc<C>,
	_marker: PhantomData<B>,
}

impl<B, C> Web3Api<B, C> {
	pub fn new(
		client: Arc<C>,
	) -> Self {
		Self {
			client: client,
			_marker: PhantomData,
		}
	}
}

impl<B, C> Web3ApiT for Web3Api<B, C> where
	C: ProvideRuntimeApi<B> + AuxStore,
	C::Api: EthereumRuntimeRPCApi<B>,
	C: HeaderBackend<B> + HeaderMetadata<B, Error=BlockChainError> + 'static,
	C: Send + Sync + 'static,
	B: BlockT<Hash=H256> + Send + Sync + 'static,
{
	fn client_version(&self) -> Result<String> {
		let hash = self.client.info().best_hash;
		let version = self.client.runtime_api().version(&BlockId::Hash(hash))
			.map_err(|err| internal_err(format!("fetch runtime version failed: {:?}", err)))?;
		Ok(format!(
			"{spec_name}/v{spec_version}.{impl_version}/{pkg_name}-{pkg_version}",
			spec_name = version.spec_name,
			spec_version = version.spec_version,
			impl_version = version.impl_version,
			pkg_name = env!("CARGO_PKG_NAME"),
			pkg_version = env!("CARGO_PKG_VERSION")
		))
	}

	fn sha3(&self, input: Bytes) -> Result<H256> {
		Ok(H256::from_slice(
			Keccak256::digest(&input.into_vec()).as_slice()
		))
	}
}
