//! Parachain Service and ServiceFactory implementation.
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::collators::lookahead as async_aura;
use cumulus_client_consensus_common::ParachainBlockImport;
use cumulus_client_consensus_relay_chain::Verifier as RelayChainVerifier;
use cumulus_client_service::{
	prepare_node_config, start_relay_chain_tasks, BuildNetworkParams, DARecoveryProfile,
	StartRelayChainTasksParams,
};
use cumulus_primitives_core::{
	relay_chain::{CollatorPair, ValidationCode},
	ParaId,
};
use sc_client_api::{AuxStore, Backend, StateBackend, StorageProvider};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;

use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node_with_rpc;
use fc_consensus::FrontierBlockImport;
use fc_db::DatabaseSource;
use fc_rpc::EthTask;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use futures::StreamExt;
use peaq_primitives_xcm::*;
// use polkadot_service::CollatorPair;
use sc_client_api::BlockchainEvents;
use sc_consensus::import_queue::BasicQueue;
use sc_executor::WasmExecutor;
use sc_network::{config::FullNetworkConfiguration, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{
	Configuration, ImportQueue, PartialComponents, TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::U256;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;
use zenlink_protocol::AssetId as ZenlinkAssetId;

use super::shell_upgrade::*;
use crate::cli_opt::{EthApi as EthApiCmd, RpcConfig};

macro_rules! declare_executor {
	($mod_type:tt, $runtime_ns:tt) => {
		pub mod $mod_type {
			pub use $runtime_ns::RuntimeApi;
		}
	};
}

declare_executor!(dev, peaq_dev_runtime);
declare_executor!(krest, peaq_krest_runtime);
declare_executor!(peaq, peaq_runtime);

type FullClient<RuntimeApi> = TFullClient<
	Block,
	RuntimeApi,
	WasmExecutor<(
		sp_io::SubstrateHostFunctions,
		frame_benchmarking::benchmarking::HostFunctions,
		peaq_primitives_ext::peaq_ext::HostFunctions,
	)>,
>;
type FullBackend = TFullBackend<Block>;

pub fn frontier_database_dir(config: &Configuration, path: &str) -> std::path::PathBuf {
	config.base_path.config_dir(config.chain_spec.id()).join("frontier").join(path)
}

// TODO This is copied from frontier. It should be imported instead after
// https://github.com/paritytech/frontier/issues/333 is solved
pub fn open_frontier_backend<C, BE>(
	client: Arc<C>,
	config: &Configuration,
) -> Result<fc_db::Backend<Block>, String>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	let frontier_backend = fc_db::Backend::KeyValue(fc_db::kv::Backend::<Block>::new(
		client,
		&fc_db::kv::DatabaseSettings {
			source: match config.database {
				DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
					path: frontier_database_dir(config, "db"),
					cache_size: 0,
				},
				DatabaseSource::ParityDb { .. } =>
					DatabaseSource::ParityDb { path: frontier_database_dir(config, "paritydb") },
				DatabaseSource::Auto { .. } => DatabaseSource::Auto {
					rocksdb_path: frontier_database_dir(config, "db"),
					paritydb_path: frontier_database_dir(config, "paritydb"),
					cache_size: 0,
				},
				_ =>
					return Err("Supported db sources: `rocksdb` | `paritydb` | `auto`".to_string()),
			},
		},
	)?);

	Ok(frontier_backend)
}

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
#[allow(clippy::type_complexity)]
pub fn new_partial<RuntimeApi, BIQ>(
	config: &mut Configuration,
	fn_build_import_queue: BIQ,
	target_gas_price: u64,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi>,
		FullBackend,
		(),
		sc_consensus::DefaultImportQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi>>,
		(
			ParachainBlockImport<
				Block,
				FrontierBlockImport<Block, Arc<FullClient<RuntimeApi>>, FullClient<RuntimeApi>>,
				FullBackend,
			>,
			Option<FilterPool>,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
			Arc<fc_db::Backend<Block>>,
			FeeHistoryCache,
		),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>:
		sc_client_api::backend::StateBackend<BlakeTwo256>,
	BIQ: FnOnce(
		Arc<FullClient<RuntimeApi>>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<Block, Arc<FullClient<RuntimeApi>>, FullClient<RuntimeApi>>,
			FullBackend,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
		u64,
	) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
{
	// Use ethereum style for subscription ids
	config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = sc_service::new_wasm_executor(config);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let filter_pool: Option<FilterPool> = Some(Arc::new(std::sync::Mutex::new(BTreeMap::new())));
	let fee_history_cache: FeeHistoryCache = Arc::new(std::sync::Mutex::new(BTreeMap::new()));

	let frontier_backend = open_frontier_backend(client.clone(), config)?;
	let frontier_block_import = FrontierBlockImport::new(client.clone(), client.clone());

	let parachain_block_import: ParachainBlockImport<_, _, _> =
		ParachainBlockImport::new(frontier_block_import, backend.clone());

	let import_queue = fn_build_import_queue(
		client.clone(),
		parachain_block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
		target_gas_price,
	)?;

	let params = PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain: (),
		other: (
			parachain_block_import,
			filter_pool,
			telemetry,
			telemetry_worker_handle,
			Arc::new(frontier_backend),
			fee_history_cache,
		),
	};

	Ok(params)
}

async fn build_relay_chain_interface(
	polkadot_config: Configuration,
	parachain_config: &Configuration,
	telemetry_worker_handle: Option<TelemetryWorkerHandle>,
	task_manager: &mut TaskManager,
	collator_options: CollatorOptions,
) -> RelayChainResult<(Arc<(dyn RelayChainInterface + 'static)>, Option<CollatorPair>)> {
	if let cumulus_client_cli::RelayChainMode::ExternalRpc(rpc_target_urls) =
		collator_options.relay_chain_mode
	{
		build_minimal_relay_chain_node_with_rpc(polkadot_config, task_manager, rpc_target_urls)
			.await
	} else {
		build_inprocess_relay_chain(
			polkadot_config,
			parachain_config,
			telemetry_worker_handle,
			task_manager,
			None,
		)
	}
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
#[allow(clippy::too_many_arguments)]
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_contracts_node_impl<RuntimeApi, BIQ, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	id: ParaId,
	rpc_config: RpcConfig,
	target_gas_price: u64,
	fn_build_import_queue: BIQ,
	fn_build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ peaq_pallet_did_rpc::PeaqDIDRuntimeApi<Block, AccountId, BlockNumber, Moment>
		+ peaq_pallet_rbac_rpc::PeaqRBACRuntimeApi<Block, AccountId, RbacEntityId>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ peaq_rpc_primitives_debug::DebugRuntimeApi<Block>
		+ peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>
		+ peaq_pallet_storage_rpc::PeaqStorageRuntimeApi<Block, AccountId>
		+ zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId, ZenlinkAssetId>
		+ cumulus_primitives_aura::AuraUnincludedSegmentApi<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>:
		sc_client_api::backend::StateBackend<BlakeTwo256>,
	BIQ: FnOnce(
		Arc<FullClient<RuntimeApi>>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<Block, Arc<FullClient<RuntimeApi>>, FullClient<RuntimeApi>>,
			FullBackend,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
		u64,
	) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>,
	BIC: FnOnce(
		Arc<FullClient<RuntimeApi>>,
		Arc<FullBackend>,
		ParachainBlockImport<
			Block,
			FrontierBlockImport<Block, Arc<FullClient<RuntimeApi>>, FullClient<RuntimeApi>>,
			FullBackend,
		>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		Arc<dyn RelayChainInterface>,
		Arc<sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi>>>,
		Arc<SyncingService<Block>>,
		KeystorePtr,
		ParaId,
		CollatorPair,
	) -> Result<(), sc_service::Error>,
{
	let mut parachain_config = prepare_node_config(parachain_config);
	let params = new_partial::<RuntimeApi, BIQ>(
		&mut parachain_config,
		fn_build_import_queue,
		target_gas_price,
	)?;
	let (
		parachain_block_import,
		filter_pool,
		mut telemetry,
		telemetry_worker_handle,
		frontier_backend,
		fee_history_cache,
	) = params.other;

	let client = params.client.clone();
	let backend = params.backend.clone();

	let mut task_manager = params.task_manager;
	let (relay_chain_interface, collator_key) = build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
	)
	.await
	.map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

	let is_authority = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();
	let network_config = FullNetworkConfiguration::new(&parachain_config.network);
	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		cumulus_client_service::build_network(BuildNetworkParams {
			parachain_config: &parachain_config,
			net_config: network_config,
			para_id: id,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: params.import_queue,
			relay_chain_interface: relay_chain_interface.clone(),
			sybil_resistance_level: cumulus_client_service::CollatorSybilResistance::Resistant,
		})
		.await?;

	let fee_history_limit = rpc_config.fee_history_limit;

	let overrides = fc_storage::overrides_handle(client.clone());

	let pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	> = Default::default();

	// Frontier offchain DB task. Essential.
	// Maps emulated ethereum data to substrate native data.
	match frontier_backend.as_ref() {
		fc_db::Backend::KeyValue(b) => task_manager.spawn_essential_handle().spawn(
			"frontier-mapping-sync-worker",
			Some("frontier"),
			fc_mapping_sync::kv::MappingSyncWorker::new(
				client.import_notification_stream(),
				Duration::new(6, 0),
				client.clone(),
				backend.clone(),
				overrides.clone(),
				Arc::new(b.clone()),
				3,
				0,
				fc_mapping_sync::SyncStrategy::Parachain,
				sync_service.clone(),
				pubsub_notification_sinks.clone(),
			)
			.for_each(|()| futures::future::ready(())),
		),
	};

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool_2) = filter_pool.clone() {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(Arc::clone(&client), filter_pool_2, FILTER_RETAIN_THRESHOLD),
		);
	}

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		Some("frontier"),
		EthTask::fee_history_task(
			Arc::clone(&client),
			Arc::clone(&overrides),
			fee_history_cache.clone(),
			fee_history_limit,
		),
	);

	let ethapi_cmd = rpc_config.ethapi.clone();
	let tracing_requesters =
		if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
			crate::rpc::tracing::spawn_tracing_tasks(
				&rpc_config,
				prometheus_registry.clone(),
				crate::rpc::SpawnTasksParams {
					task_manager: &task_manager,
					client: client.clone(),
					substrate_backend: backend.clone(),
					frontier_backend: frontier_backend.clone(),
					filter_pool: filter_pool.clone(),
					overrides: overrides.clone(),
					fee_history_limit,
					fee_history_cache: fee_history_cache.clone(),
				},
			)
		} else {
			crate::rpc::tracing::RpcRequesters { debug: None, trace: None }
		};

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
		rpc_config.eth_log_block_cache,
		rpc_config.eth_statuses_cache,
		prometheus_registry.clone(),
	));

	let rpc_builder = {
		let client = client.clone();
		let network = network.clone();
		let sync = sync_service.clone();
		let pool = transaction_pool.clone();
		let backend = backend.clone();

		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let ethapi_cmd = ethapi_cmd.clone();
		let max_past_logs = rpc_config.max_past_logs;
		let overrides = overrides.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = block_data_cache.clone();

		move |deny_unsafe, subscription_task_executor| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				graph: pool.pool().clone(),
				deny_unsafe,
				is_authority,
				network: network.clone(),
				sync: sync.clone(),
				filter_pool: filter_pool.clone(),
				ethapi_cmd: ethapi_cmd.clone(),
				frontier_backend: match frontier_backend.as_ref() {
					fc_db::Backend::KeyValue(b) => Arc::new(b.clone()),
				},
				backend: backend.clone(),
				command_sink: None,
				max_past_logs,
				fee_history_limit,
				fee_history_cache: fee_history_cache.clone(),
				xcm_senders: None,
				overrides: overrides.clone(),
				block_data_cache: block_data_cache.clone(),
				forced_parent_hashes: None,
			};

			if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
				crate::rpc::create_full(
					deps,
					subscription_task_executor,
					Some(crate::rpc::TracingConfig {
						tracing_requesters: tracing_requesters.clone(),
						trace_filter_max_count: rpc_config.ethapi_trace_max_count,
					}),
				)
				.map_err(Into::into)
			} else {
				crate::rpc::create_full(deps, subscription_task_executor, None).map_err(Into::into)
			}
		}
	};

	// Spawn basic services.
	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder: Box::new(rpc_builder),
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);
	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		task_manager: &mut task_manager,
		para_id: id,
		relay_chain_interface: relay_chain_interface.clone(),
		relay_chain_slot_duration,
		import_queue: import_queue_service,
		recovery_handle: Box::new(overseer_handle.clone()),
		sync_service: sync_service.clone(),
		da_recovery_profile: if is_authority {
			DARecoveryProfile::Collator
		} else {
			DARecoveryProfile::FullNode
		},
	})?;

	if is_authority {
		fn_build_consensus(
			client.clone(),
			backend.clone(),
			parachain_block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			sync_service.clone(),
			params.keystore_container.keystore(),
			id,
			collator_key.expect("Command line arguments do not allow this. qed"),
		)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Build the import queue.
#[allow(clippy::type_complexity)]
pub fn build_import_queue<RuntimeApi>(
	client: Arc<FullClient<RuntimeApi>>,
	block_import: ParachainBlockImport<
		Block,
		FrontierBlockImport<Block, Arc<FullClient<RuntimeApi>>, FullClient<RuntimeApi>>,
		FullBackend,
	>,
	config: &Configuration,
	telemetry_handle: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	target_gas_price: u64,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>,
	sc_client_api::StateBackendFor<FullBackend, Block>:
		sc_client_api::backend::StateBackend<BlakeTwo256>,
{
	let client2 = client.clone();

	let aura_verifier = move || {
		let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client2).unwrap();

		Box::new(cumulus_client_consensus_aura::build_verifier::<
			sp_consensus_aura::sr25519::AuthorityPair,
			_,
			_,
			_,
		>(cumulus_client_consensus_aura::BuildVerifierParams {
			client: client2.clone(),
			create_inherent_data_providers: move |_, _| async move {
				let time = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
					sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*time,
						slot_duration,
						);
				let dynamic_fee =
					fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));

				Ok((slot, time, dynamic_fee))
			},
			telemetry: telemetry_handle,
		})) as Box<_>
	};

	let relay_chain_verifier =
		Box::new(RelayChainVerifier::new(client.clone(), |_, _| async { Ok(()) })) as Box<_>;

	let verifier = Verifier {
		client,
		relay_chain_verifier,
		aura_verifier: BuildOnAccess::Uninitialized(Some(Box::new(aura_verifier))),
	};

	let registry = config.prometheus_registry();
	let spawner = task_manager.spawn_essential_handle();

	Ok(BasicQueue::new(verifier, Box::new(block_import), None, &spawner, registry))
}

pub async fn start_node<RuntimeApi>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	id: ParaId,
	rpc_config: RpcConfig,
	target_gas_price: u64,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ peaq_pallet_did_rpc::PeaqDIDRuntimeApi<Block, AccountId, BlockNumber, Moment>
		+ peaq_pallet_rbac_rpc::PeaqRBACRuntimeApi<Block, AccountId, RbacEntityId>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ peaq_rpc_primitives_debug::DebugRuntimeApi<Block>
		+ peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ cumulus_primitives_core::CollectCollationInfo<Block>
		+ peaq_pallet_storage_rpc::PeaqStorageRuntimeApi<Block, AccountId>
		+ zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId, ZenlinkAssetId>
		+ cumulus_primitives_aura::AuraUnincludedSegmentApi<Block>,
{
	start_contracts_node_impl::<RuntimeApi, _, _>(
		parachain_config,
		polkadot_config,
		collator_options,
		id,
		rpc_config,
		target_gas_price,
		|client, block_import, config, telemetry, task_manager, target_gas_price| {
			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

			cumulus_client_consensus_aura::import_queue::<
				sp_consensus_aura::sr25519::AuthorityPair,
				_,
				_,
				_,
				_,
				_,
			>(cumulus_client_consensus_aura::ImportQueueParams {
				block_import,
				client,
				create_inherent_data_providers: move |_, _| async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*time,
							slot_duration,
						);

					let dynamic_fee =
						fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));

					Ok((slot, time, dynamic_fee))
				},
				registry: config.prometheus_registry(),
				spawner: &task_manager.spawn_essential_handle(),
				telemetry,
			})
			.map_err(Into::into)
		},
		|client,
		 backend,
		 block_import,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_interface,
		 transaction_pool,
		 sync_oracle,
		 keystore,
		 para_id,
		 collator_key| {
			let spawn_handle = task_manager.spawn_handle();

			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

			let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
				spawn_handle,
				client.clone(),
				transaction_pool,
				prometheus_registry,
				telemetry.clone(),
			);

			let overseer_handle = relay_chain_interface
				.overseer_handle()
				.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

			let announce_block = {
				let sync_service = sync_oracle.clone();
				Arc::new(move |hash, data| sync_service.announce_block(hash, data))
			};

			let collator_service = cumulus_client_collator::service::CollatorService::new(
				client.clone(),
				Arc::new(task_manager.spawn_handle()),
				announce_block,
				client.clone(),
			);

			let fut =
				async_aura::run::<Block, AuraPair, _, _, _, _, _, _, _, _, _>(async_aura::Params {
					create_inherent_data_providers: move |_, ()| async move { Ok(()) },
					block_import: block_import.clone(),
					para_client: client.clone(),
					para_backend: backend.clone(),
					relay_client: relay_chain_interface.clone(),
					code_hash_provider: move |block_hash| {
						client.code_at(block_hash).ok().map(|c| ValidationCode::from(c).hash())
					},
					sync_oracle: sync_oracle.clone(),
					keystore,
					collator_key,
					para_id,
					overseer_handle,
					slot_duration,
					relay_chain_slot_duration: Duration::from_secs(6),
					proposer: cumulus_client_consensus_proposer::Proposer::new(proposer_factory),
					collator_service,
					// We got around 1500ms for proposing
					authoring_duration: Duration::from_millis(1500),
					// collation_request_receiver: None,
					reinitialize: false,
				});

			task_manager.spawn_essential_handle().spawn("aura", None, fut);

			Ok(())
		},
	)
	.await
}
