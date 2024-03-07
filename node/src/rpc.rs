//! A collection of node-specific RPC methods.

use cumulus_primitives_core::ParaId;
use fc_rpc::{EthBlockDataCacheTask, OverrideHandle};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use jsonrpsee::RpcModule;
use peaq_primitives_xcm::*;
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use sc_client_api::UsageProvider;
use sc_consensus_manual_seal::rpc::EngineCommand;
use sc_network::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::{TaskManager, TransactionPool};
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{
	Backend as BlockchainBackend, Error as BlockChainError, HeaderBackend, HeaderMetadata,
};
use sp_core::H256;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::{collections::BTreeMap, sync::Arc};
use zenlink_protocol::AssetId as ZenlinkAssetId;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use polkadot_primitives::PersistedValidationData;

pub mod tracing;
use crate::cli_opt::EthApi as EthApiCmd;

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

pub type XcmSenders = Option<(flume::Sender<Vec<u8>>, flume::Sender<(ParaId, Vec<u8>)>)>;

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, BE> {
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
	/// Chain syncing service
	pub sync: Arc<SyncingService<Block>>,
	/// EthFilterApi pool.
	pub filter_pool: Option<FilterPool>,
	/// The list of optional RPC extensions.
	pub ethapi_cmd: Vec<EthApiCmd>,
	/// Frontier Backend.
	pub frontier_backend: Arc<dyn fc_db::BackendReader<Block> + Send + Sync>,
	/// Backend.
	pub backend: Arc<BE>,
	/// Manual seal command sink
	pub command_sink: Option<futures::channel::mpsc::Sender<EngineCommand<Hash>>>,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Maximum fee history cache size.
	pub fee_history_limit: u64,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Channels for manual xcm messages (downward, hrmp)
	pub xcm_senders: XcmSenders,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
	/// Mandated parent hashes for a given block hash.
	pub forced_parent_hashes: Option<BTreeMap<H256, H256>>,
}

pub struct TracingConfig {
	pub tracing_requesters: crate::rpc::tracing::RpcRequesters,
	pub trace_filter_max_count: u32,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, BE, A>(
	deps: FullDeps<C, P, A, BE>,
	subscription_task_executor: SubscriptionTaskExecutor,
	maybe_tracing_config: Option<TracingConfig>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	// BE::Blockchain: BlockchainBackend<Block>,
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore + UsageProvider<Block>,
	C: BlockchainEvents<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: CallApiAt<Block>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BlockBuilder<Block>,
	C::Api: AuraApi<Block, AuraId>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: peaq_pallet_did_rpc::PeaqDIDRuntimeApi<Block, AccountId, BlockNumber, Moment>,
	C::Api: peaq_pallet_rbac_rpc::PeaqRBACRuntimeApi<Block, AccountId, RbacEntityId>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: peaq_rpc_primitives_debug::DebugRuntimeApi<Block>,
	C::Api: peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>,
	C::Api: peaq_pallet_storage_rpc::PeaqStorageRuntimeApi<Block, AccountId>,
	C::Api: zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId, ZenlinkAssetId>,
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
	use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		graph,
		deny_unsafe,
		is_authority,
		network,
		sync,
		filter_pool,
		ethapi_cmd,
		command_sink: _,
		frontier_backend,
		backend: _,
		max_past_logs,
		fee_history_limit,
		fee_history_cache,
		xcm_senders: _,
		overrides,
		block_data_cache,
		forced_parent_hashes,
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

	let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
    let pending_create_inherent_data_providers = move |_, _| async move {
        let current = sp_timestamp::InherentDataProvider::from_system_time();
        let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
        let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
        let slot =
            sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
        // Create a dummy parachain inherent data provider which is required to pass
        // the checks by the para chain system. We use dummy values because in the 'pending context'
        // neither do we have access to the real values nor do we need them.
        let (relay_parent_storage_root, relay_chain_state) =
            RelayStateSproofBuilder::default().into_state_root_and_proof();
        let vfp = PersistedValidationData {
            // This is a hack to make `cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases`
            // happy. Relay parent number can't be bigger than u32::MAX.
            relay_parent_number: u32::MAX,
            relay_parent_storage_root,
            ..Default::default()
        };
        let parachain_inherent_data = ParachainInherentData {
            validation_data: vfp,
            relay_chain_state,
            downward_messages: Default::default(),
            horizontal_messages: Default::default(),
        };
        Ok((slot, timestamp, parachain_inherent_data))
    };

    let pending_consensus_data_provider = Box::new(
        fc_rpc::pending::AuraConsensusDataProvider::new(client.clone()),
    );

	io.merge(
		Eth::new(
			Arc::clone(&client),
			Arc::clone(&pool),
			graph.clone(),
			no_tx_converter,
			Arc::clone(&sync),
			Default::default(), // signers
			Arc::clone(&overrides),
			Arc::clone(&frontier_backend),
			is_authority,
			Arc::clone(&block_data_cache),
			fee_history_cache,
			fee_history_limit,
			10_u64,
			forced_parent_hashes,
			pending_create_inherent_data_providers,
			Some(pending_consensus_data_provider),
		)
		.into_rpc(),
	)?;

	if let Some(filter_pool) = filter_pool {
		io.merge(
			EthFilter::new(
				client.clone(),
				frontier_backend,
				graph.clone(),
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

	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	io.merge(PeaqStorage::new(Arc::clone(&client)).into_rpc())?;
	io.merge(PeaqDID::new(Arc::clone(&client)).into_rpc())?;
	io.merge(PeaqRBAC::new(Arc::clone(&client)).into_rpc())?;
	io.merge(ZenlinkProtocol::new(Arc::clone(&client)).into_rpc())?;
	io.merge(Web3::new(Arc::clone(&client)).into_rpc())?;
	io.merge(
		EthPubSub::new(
			pool,
			Arc::clone(&client),
			sync.clone(),
			subscription_task_executor,
			overrides,
			pubsub_notification_sinks.clone(),
		)
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
