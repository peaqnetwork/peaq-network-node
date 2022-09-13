//! Parachain Service and ServiceFactory implementation.
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_consensus_common::{ParachainBlockImport, ParachainConsensus};
use cumulus_client_consensus_relay_chain::Verifier as RelayChainVerifier;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_rpc_interface::RelayChainRPCInterface;
use fc_consensus::FrontierBlockImport;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use futures::StreamExt;
use polkadot_service::CollatorPair;
use sc_client_api::{BlockchainEvents, ExecutorProvider};
use sc_consensus::import_queue::BasicQueue;
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{Configuration, PartialComponents, Role, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use substrate_prometheus_endpoint::Registry;

use super::shell_upgrade::*;
use crate::primitives::*;
use sc_service::BasePath;

use crate::cli_opt::EthApi as EthApiCmd;
use crate::cli_opt::RpcConfig;
use fc_rpc::EthTask;
use sc_cli::SubstrateCli;

use sp_core::U256;

/// dev network runtime executor.
pub mod dev {
	pub use peaq_dev_runtime::RuntimeApi;

	pub type HostFunctions = (
		frame_benchmarking::benchmarking::HostFunctions,
		peaq_primitives_ext::peaq_ext::HostFunctions,
	);
	// Our native executor instance.
	pub struct Executor;

	impl sc_executor::NativeExecutionDispatch for Executor {
		type ExtendHostFunctions = HostFunctions;

		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			peaq_dev_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			peaq_dev_runtime::native_version()
		}
	}
}

pub mod agung {
	pub use peaq_agung_runtime::RuntimeApi;

	pub type HostFunctions = (
		frame_benchmarking::benchmarking::HostFunctions,
		peaq_primitives_ext::peaq_ext::HostFunctions,
	);
	// Our native executor instance.
	pub struct Executor;

	impl sc_executor::NativeExecutionDispatch for Executor {
		type ExtendHostFunctions = HostFunctions;

		fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
			peaq_agung_runtime::api::dispatch(method, data)
		}

		fn native_version() -> sc_executor::NativeVersion {
			peaq_agung_runtime::native_version()
		}
	}
}

type FullClient<RuntimeApi, Executor> =
	TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>;
type FullBackend = TFullBackend<Block>;


pub fn frontier_database_dir(config: &Configuration) -> std::path::PathBuf {
	let config_dir = config
		.base_path
		.as_ref()
		.map(|base_path| base_path.config_dir(config.chain_spec.id()))
		.unwrap_or_else(|| {
			BasePath::from_project("", "", &crate::cli::Cli::executable_name())
				.config_dir(config.chain_spec.id())
		});
	config_dir.join("frontier").join("db")
}

pub fn open_frontier_backend(config: &Configuration) -> Result<Arc<fc_db::Backend<Block>>, String> {
	Ok(Arc::new(fc_db::Backend::<Block>::new(
		&fc_db::DatabaseSettings {
			source: fc_db::DatabaseSettingsSrc::RocksDb {
				path: frontier_database_dir(&config),
				cache_size: 0,
			},
		},
	)?))
}
/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
pub fn new_partial<RuntimeApi, Executor, BIQ>(
	config: &Configuration,
	fn_build_import_queue: BIQ,
	target_gas_price: u64,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		(),
		sc_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			FrontierBlockImport<
				Block,
				Arc<FullClient<RuntimeApi, Executor>>,
				FullClient<RuntimeApi, Executor>,
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
	RuntimeApi:
		ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIQ: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		FrontierBlockImport<
			Block,
			Arc<FullClient<RuntimeApi, Executor>>,
			FullClient<RuntimeApi, Executor>,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
		u64,
	) -> Result<
		sc_consensus::DefaultImportQueue<
			Block,
			FullClient<RuntimeApi, Executor>,
		>,
		sc_service::Error,
	>,
{
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

	let executor = sc_executor::NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
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

	let frontier_backend = open_frontier_backend(config)?;
	let frontier_block_import =
		FrontierBlockImport::new(client.clone(), client.clone(), frontier_backend.clone());

	let import_queue = fn_build_import_queue(
		client.clone(),
		frontier_block_import.clone(),
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
			frontier_block_import,
			filter_pool,
			telemetry,
			telemetry_worker_handle,
			frontier_backend,
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
	relay_chain_rpc_url: Option<url::Url>,
) -> RelayChainResult<(
	Arc<(dyn RelayChainInterface + 'static)>,
	Option<CollatorPair>,
)> {
	match relay_chain_rpc_url {
		Some(relay_chain_url) => Ok((
			Arc::new(RelayChainRPCInterface::new(relay_chain_url).await?) as Arc<_>,
			None,
		)),
		None => build_inprocess_relay_chain(
			polkadot_config,
			parachain_config,
			telemetry_worker_handle,
			task_manager,
		),
	}
}

/// Start a node with the given parachain `Configuration` and relay chain `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the runtime api.
///
/// NOTE: for runtimes that supports pallet_contracts_rpc
#[sc_tracing::logging::prefix_logs_with("Parachain")]
async fn start_contracts_node_impl<RuntimeApi, Executor, BIQ, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	rpc_config: RpcConfig,
	target_gas_price: u64,
	fn_build_import_queue: BIQ,
	fn_build_consensus: BIC,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<FullClient<RuntimeApi, Executor>>,
)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ peaq_rpc_primitives_debug::DebugRuntimeApi<Block>
		+ peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>
		+ pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIQ: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		FrontierBlockImport<
			Block,
			Arc<FullClient<RuntimeApi, Executor>>,
			FullClient<RuntimeApi, Executor>,
		>,
		&Configuration,
		Option<TelemetryHandle>,
		&TaskManager,
		u64,
	) -> Result<
		sc_consensus::DefaultImportQueue<
			Block,
			FullClient<RuntimeApi, Executor>,
		>,
		sc_service::Error,
	>,
	BIC: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		Arc<dyn RelayChainInterface>,
		Arc<
			sc_transaction_pool::FullPool<
				Block,
				FullClient<RuntimeApi, Executor>,
			>,
		>,
		Arc<NetworkService<Block, Hash>>,
		SyncCryptoStorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);
	let params = new_partial::<RuntimeApi, Executor, BIQ>(
		&parachain_config, fn_build_import_queue, target_gas_price)?;
	let (
		_block_import,
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
		rpc_config.relay_chain_rpc_url.clone(),
	)
	.await
	.map_err(|e| match e {
		RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
		s => format!("{}", s).into(),
	})?;
	let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

	let force_authoring = parachain_config.force_authoring;
	let is_authority = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
	let (network, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: import_queue.clone(),
			block_announce_validator_builder: Some(Box::new(|_| {
				Box::new(block_announce_validator)
			})),
			warp_sync: None,
		})?;

	let subscription_task_executor =
		sc_rpc::SubscriptionTaskExecutor::new(task_manager.spawn_handle());
	let fee_history_limit = rpc_config.fee_history_limit;

	let overrides = crate::rpc::overrides_handle(client.clone());

	// Frontier offchain DB task. Essential.
	// Maps emulated ethereum data to substrate native data.
	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		Some("frontier"),
		fc_mapping_sync::MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
			3,
			0,
			fc_mapping_sync::SyncStrategy::Parachain,
		)
		.for_each(|()| futures::future::ready(())),
	);

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool_2) = filter_pool.clone() {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(
				Arc::clone(&client),
				filter_pool_2,
				FILTER_RETAIN_THRESHOLD
			),
		);
	}

	task_manager.spawn_essential_handle().spawn(
		"frontier-schema-cache-task",
		Some("frontier"),
		EthTask::ethereum_schema_cache_task(Arc::clone(&client), Arc::clone(&frontier_backend)),
	);

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
			crate::rpc::tracing::RpcRequesters {
				debug: None,
				trace: None,
			}
		};

	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
		rpc_config.eth_log_block_cache as u64,
		rpc_config.eth_statuses_cache as u64,
		prometheus_registry.clone(),
	));

	// variable `rpc_config` will be moved in next code block, we need to
	// save param `relay_chain_rpc_url` to be able to use it later.
	let relay_chain_rpc_url = rpc_config.relay_chain_rpc_url.clone();

	let rpc_extensions_builder = {
		let client = client.clone();
		let network = network.clone();
		let pool = transaction_pool.clone();

		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let _backend = backend.clone();
		let ethapi_cmd = ethapi_cmd.clone();
		let max_past_logs = rpc_config.max_past_logs;
		let overrides = overrides.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = block_data_cache.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				backend: frontier_backend.clone(),
				client: client.clone(),
				deny_unsafe,
				ethapi_cmd: ethapi_cmd.clone(),
				filter_pool: filter_pool.clone(),
				graph: pool.pool().clone(),
				pool: pool.clone(),
				is_authority,
				max_past_logs,
				fee_history_limit,
				fee_history_cache: fee_history_cache.clone(),
				network: network.clone(),
				block_data_cache: block_data_cache.clone(),
				overrides: overrides.clone(),
			};
			#[allow(unused_mut)]
			let mut io = crate::rpc::create_full(deps, subscription_task_executor.clone());
			// This node support WASM contracts
			io.extend_with(pallet_contracts_rpc::ContractsApi::to_delegate(
				pallet_contracts_rpc::Contracts::new(client.clone()),
			));
			if ethapi_cmd.contains(&EthApiCmd::Debug) || ethapi_cmd.contains(&EthApiCmd::Trace) {
				crate::rpc::tracing::extend_with_tracing(
					client.clone(),
					tracing_requesters.clone(),
					rpc_config.ethapi_trace_max_count,
					&mut io,
				);
			}
			Ok(io)
		})
	};

	// Spawn basic services.
	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_extensions_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.sync_keystore(),
		backend: backend.clone(),
		network: network.clone(),
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	if is_authority {
		let parachain_consensus = fn_build_consensus(
			client.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			network,
			params.keystore_container.sync_keystore(),
			force_authoring,
		)?;

		let spawner = task_manager.spawn_handle();

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_interface,
			spawner,
			parachain_consensus,
			import_queue,
			collator_key: collator_key.ok_or(sc_service::error::Error::Other(
				"Collator Key is None".to_string(),
			))?,
			relay_chain_slot_duration,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_interface,
			relay_chain_slot_duration,
			import_queue,
			collator_options: CollatorOptions {
				relay_chain_rpc_url,
			},
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Build the import queue.
pub fn build_import_queue<RuntimeApi, Executor>(
	client: Arc<FullClient<RuntimeApi, Executor>>,
	block_import: FrontierBlockImport<
		Block,
		Arc<FullClient<RuntimeApi, Executor>>,
		FullClient<RuntimeApi, Executor>,
	>,
	config: &Configuration,
	telemetry_handle: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	target_gas_price: u64,
) -> Result<
	sc_consensus::DefaultImportQueue<
		Block,
		FullClient<RuntimeApi, Executor>,
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	let client2 = client.clone();

	let aura_verifier = move || {
		let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client2).unwrap();

		Box::new(cumulus_client_consensus_aura::build_verifier::<
			sp_consensus_aura::sr25519::AuthorityPair,
			_,
			_,
			_,
		>(
			cumulus_client_consensus_aura::BuildVerifierParams {
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

					Ok((time, slot, dynamic_fee))
				},
				can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(
					client2.executor().clone(),
				),
				telemetry: telemetry_handle,
			},
		)) as Box<_>
	};

	let relay_chain_verifier = Box::new(RelayChainVerifier::new(client.clone(), |_, _| async {
		Ok(())
	})) as Box<_>;

	let verifier = Verifier {
		client,
		relay_chain_verifier,
		aura_verifier: BuildOnAccess::Uninitialized(Some(Box::new(aura_verifier))),
	};

	let registry = config.prometheus_registry().clone();
	let spawner = task_manager.spawn_essential_handle();

	Ok(BasicQueue::new(
		verifier,
		Box::new(ParachainBlockImport::new(block_import)),
		None,
		&spawner,
		registry,
	))
}

pub async fn start_node<RuntimeApi, Executor>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	rpc_config: RpcConfig,
	target_gas_price: u64,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<FullClient<RuntimeApi, Executor>>,
)>
where
	RuntimeApi:
		ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>
		+ peaq_rpc_primitives_debug::DebugRuntimeApi<Block>
		+ peaq_rpc_primitives_txpool::TxPoolRuntimeApi<Block>
		+ sp_consensus_aura::AuraApi<Block, AuraId>
		+ pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>
		+ cumulus_primitives_core::CollectCollationInfo<Block>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	start_contracts_node_impl::<RuntimeApi, Executor, _, _>(
		parachain_config,
		polkadot_config,
		id,
		rpc_config,
		target_gas_price,
		|client,
		 block_import,
		 config,
		 telemetry,
		 task_manager,
		 target_gas_price| {
			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;
			let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

			cumulus_client_consensus_aura::import_queue::<
				sp_consensus_aura::sr25519::AuthorityPair,
				_,
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

					Ok((time, slot, dynamic_fee))
				},
				registry: config.prometheus_registry().clone(),
				can_author_with,
				spawner: &task_manager.spawn_essential_handle(),
				telemetry,
			})
			.map_err(Into::into)
		},
		|client,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_interface,
		 transaction_pool,
		 sync_oracle,
		 keystore,
		 force_authoring| {
			let spawn_handle = task_manager.spawn_handle();

			let slot_duration =
				cumulus_client_consensus_aura::slot_duration(&*client).unwrap();

			let proposer_factory =
				sc_basic_authorship::ProposerFactory::with_proof_recording(
					spawn_handle,
					client.clone(),
					transaction_pool,
					prometheus_registry,
					telemetry.clone(),
				);

			Ok(AuraConsensus::build::<
				sp_consensus_aura::sr25519::AuthorityPair,
				_,
				_,
				_,
				_,
				_,
				_,
			>(BuildAuraConsensusParams {
				proposer_factory,
				create_inherent_data_providers:
					move |_, (relay_parent, validation_data)| {
						let relay_chain_for_aura = relay_chain_interface.clone();
						async move {
							let parachain_inherent =
								cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
									relay_parent,
									&relay_chain_for_aura,
									&validation_data,
									id,
								).await;
							let time = sp_timestamp::InherentDataProvider::from_system_time();
							let slot =
								sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
									*time,
									slot_duration,
								);

							let parachain_inherent = parachain_inherent.ok_or_else(|| {
								Box::<dyn std::error::Error + Send + Sync>::from(
									"Failed to create parachain inherent",
								)
							})?;
							let dynamic_fee =
								fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));

							Ok((time, slot, parachain_inherent, dynamic_fee))
						}
					},
				block_import: client.clone(),
				para_client: client.clone(),
				backoff_authoring_blocks: Option::<()>::None,
				sync_oracle,
				keystore,
				force_authoring,
				slot_duration,
				// We got around 500ms for proposing
				block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
				// And a maximum of 750ms if slots are skipped
				max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
				telemetry,
			})
		)
	})
	.await
}
