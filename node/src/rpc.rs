//! A collection of node-specific RPC methods.

use std::sync::Arc;

use fc_rpc::{EthBlockDataCacheTask, OverrideHandle};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use jsonrpsee::RpcModule;
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use sc_network::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
	Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_runtime::traits::BlakeTwo256;
// use sc_consensus_manual_seal::rpc::{ManualSeal, ManualSealApiServer};

use sc_service::TaskManager;
use sp_runtime::traits::Block as BlockT;
pub mod tracing;
use crate::cli_opt::EthApi as EthApiCmd;

use crate::primitives::*;

pub struct SpawnTasksParams<'a, B: BlockT, C, BE> {
	pub task_manager: &'a TaskManager,
	pub client: Arc<C>,
	pub substrate_backend: Arc<BE>,
	pub frontier_backend: Arc<fc_db::Backend<B>>,
	pub filter_pool: Option<FilterPool>,
	pub overrides: Arc<OverrideHandle<B>>,
	pub fee_history_limit: u64,
	pub fee_history_cache: FeeHistoryCache,
}

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
	pub is_authority: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// Backend.
	pub backend: Arc<fc_db::Backend<Block>>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Maximum fee history cache size.
	pub fee_history_limit: u64,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// The list of optional RPC extensions.
	pub ethapi_cmd: Vec<EthApiCmd>,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

pub struct TracingConfig {
	pub tracing_requesters: crate::rpc::tracing::RpcRequesters,
	pub trace_filter_max_count: u32,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, BE, A>(
	deps: FullDeps<C, P, A>,
	subscription_task_executor: SubscriptionTaskExecutor,
	maybe_tracing_config: Option<TracingConfig>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	// BE::Blockchain: BlockchainBackend<Block>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: CallApiAt<Block>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: peaq_pallet_did_rpc::PeaqDIDRuntimeApi<Block, AccountId, BlockNumber, Moment>,
	C::Api: peaq_pallet_rbac_rpc::PeaqRBACRuntimeApi<Block, AccountId, EntityId>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: peaq_rpc_primitives_debug::DebugRuntimeApi<Block>,
	C::Api: peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>,
	C::Api: peaq_pallet_storage_rpc::PeaqStorageRuntimeApi<Block, AccountId>,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,

	BE::Blockchain: BlockchainBackend<Block>,
{
	use fc_rpc::{
		Eth, EthApiServer, EthFilter, EthFilterApiServer, EthPubSub, EthPubSubApiServer, Net,
		NetApiServer, Web3, Web3ApiServer,
	};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use peaq_pallet_did_rpc::{PeaqDID, PeaqDIDApiServer};
	use peaq_pallet_rbac_rpc::{PeaqRBAC, PeaqRBACApiServer};
	use peaq_pallet_storage_rpc::{PeaqStorage, PeaqStorageApiServer};
	use peaq_rpc_debug::{Debug, DebugServer};
	use peaq_rpc_trace::{Trace, TraceServer};
	use peaq_rpc_txpool::{TxPool, TxPoolServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		deny_unsafe,
		is_authority,
		network,
		filter_pool,
		backend,
		max_past_logs,
		fee_history_limit,
		fee_history_cache,
		ethapi_cmd,
		overrides,
		block_data_cache,
	} = deps;

	io.merge(System::new(Arc::clone(&client), Arc::clone(&pool), deny_unsafe).into_rpc())?;
	io.merge(TransactionPayment::new(Arc::clone(&client)).into_rpc())?;

	enum Never {}
	impl<T> fp_rpc::ConvertTransaction<T> for Never {
		fn convert_transaction(&self, _transaction: pallet_ethereum::Transaction) -> T {
			// The Never type is not instantiable, but this method requires the type to be
			// instantiated to be called (`&self` parameter), so if the code compiles we have the
			// guarantee that this function will never be called.
			unreachable!()
		}
	}
	let no_tx_converter: Option<fp_rpc::NoTransactionConverter> = None;

	io.merge(
		Eth::new(
			Arc::clone(&client),
			Arc::clone(&pool),
			graph.clone(),
			no_tx_converter,
			Arc::clone(&network),
			Default::default(),
			Arc::clone(&overrides),
			Arc::clone(&backend),
			is_authority,
			Arc::clone(&block_data_cache),
			fee_history_cache,
			fee_history_limit,
			10_u64,
		)
		.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		io.merge(
			EthFilter::new(
				client.clone(),
				backend,
				filter_pool,
				500_usize, // max stored filters
				max_past_logs,
				block_data_cache,
			)
			.into_rpc(),
		)?;
	}

	io.merge(
		Net::new(
			Arc::clone(&client),
			network.clone(),
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
		.into_rpc(),
	)?;

	io.merge(PeaqStorage::new(Arc::clone(&client)).into_rpc())?;
	io.merge(PeaqDID::new(Arc::clone(&client)).into_rpc())?;
	io.merge(PeaqRBAC::new(Arc::clone(&client)).into_rpc())?;
	io.merge(Web3::new(Arc::clone(&client)).into_rpc())?;
	io.merge(
		EthPubSub::new(pool, Arc::clone(&client), network, subscription_task_executor, overrides)
			.into_rpc(),
	)?;
	if ethapi_cmd.contains(&EthApiCmd::Txpool) {
		io.merge(TxPool::new(Arc::clone(&client), graph).into_rpc())?;
	}

	if let Some(tracing_config) = maybe_tracing_config {
		if let Some(trace_filter_requester) = tracing_config.tracing_requesters.trace {
			io.merge(
				Trace::new(client, trace_filter_requester, tracing_config.trace_filter_max_count)
					.into_rpc(),
			)?;
		}

		if let Some(debug_requester) = tracing_config.tracing_requesters.debug {
			io.merge(Debug::new(debug_requester).into_rpc())?;
		}
	}

	Ok(io)
}
