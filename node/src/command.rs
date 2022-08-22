use crate::{
	parachain,
	cli::{Cli, Subcommand, RelayChainCli},
	parachain::service::{self, frontier_database_dir, start_node, dev, agung},
	cli_opt::{EthApi, RpcConfig},
	primitives::Block,
};

use sp_runtime::traits::Block as BlockT;
use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli, Result};
use sc_service::PartialComponents;
use frame_benchmarking_cli::BenchmarkCmd;

// Parachain
use codec::Encode;
use sp_core::hexdisplay::HexDisplay;
use cumulus_client_service::genesis::generate_genesis_block;
use log::info;
use cumulus_primitives_core::ParaId;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
	CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, SharedParams,
};
use sc_service::{
	config::{BasePath, PrometheusConfig},
};
use std::{io::Write, net::SocketAddr};

trait IdentifyChain {
	fn is_dev(&self) -> bool;
	fn is_agung(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
	fn is_agung(&self) -> bool {
		self.id().starts_with("agung")
	}
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
	fn is_dev(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_dev(self)
	}
	fn is_agung(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_agung(self)
	}
}

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"PEAQ Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"Peaq Node\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		peaq-node [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/peaqnetwork/peaq-network-node/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(parachain::dev_chain_spec::get_chain_spec(self.run.parachain_id)?),
			"agung" => Box::new(parachain::agung_chain_spec::get_chain_spec(self.run.parachain_id)?),
			path => Box::new(parachain::dev_chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if chain_spec.is_agung() {
			&agung_runtime::VERSION
		} else {
			&peaq_dev_runtime::VERSION
		}
	}
}

fn validate_trace_environment(cli: &Cli) -> sc_cli::Result<()> {
	if (cli.run.ethapi.contains(&EthApi::Debug) || cli.run.ethapi.contains(&EthApi::Trace))
		&& cli
			.run
			.base
			.base
			.import_params
			.wasm_runtime_overrides
			.is_none()
	{
		return Err(
			"`debug` or `trace` namespaces requires `--wasm-runtime-overrides /path/to/overrides`."
				.into(),
		);
	}
	Ok(())
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"Peaq Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"Peaq Node\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		peaq-node [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/peaqnetwork/peaq-network-node/issues/new".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter())
			.load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

#[allow(clippy::borrowed_box)]
fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}


/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();
	let _ = validate_trace_environment(&cli)?;

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		}
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			if runner.config().chain_spec.is_agung() {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						import_queue,
						..
					} = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			} else {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						import_queue,
						..
					} = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			if runner.config().chain_spec.is_agung() {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						..
					} = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, config.database), task_manager))
				})
			} else {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						..
					} = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, config.database), task_manager))
				})
			}
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			if runner.config().chain_spec.is_agung() {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						..
					} = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			} else {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						..
					} = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			}
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			if runner.config().chain_spec.is_agung() {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						import_queue,
						..
					} = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			} else {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						import_queue,
						..
					} = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			}
		}
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				// Remove Frontier offchain db
				let frontier_database_config = sc_service::DatabaseSource::RocksDb {
					path: frontier_database_dir(&config),
					cache_size: 0,
				};
				cmd.run(frontier_database_config)?;
				cmd.run(config.database)
			})
		}
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			if runner.config().chain_spec.is_agung() {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						backend,
						..
					} = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, backend, None), task_manager))
				})
			} else {
				runner.async_run(|config| {
					let PartialComponents {
						client,
						task_manager,
						backend,
						..
					} = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
						&config,
						parachain::build_import_queue,
						cli.run.target_gas_price)?;
					Ok((cmd.run(client, backend, None), task_manager))
				})
			}
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if chain_spec.is_agung() {
							return runner.sync_run(|config| {
								cmd.run::<Block, agung::Executor>(config)
							})
						} else {
							return runner.sync_run(|config| {
								cmd.run::<Block, dev::Executor>(config)
							})
						}
					}
					BenchmarkCmd::Block(cmd) => {
						if chain_spec.is_agung() {
							return runner.sync_run(|config| {
								let params = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
									&config,
									parachain::build_import_queue,
									cli.run.target_gas_price)?;

								cmd.run(params.client)
							})
						} else {
							return runner.sync_run(|config| {
								let params = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
									&config,
									parachain::build_import_queue,
									cli.run.target_gas_price)?;

								cmd.run(params.client)
							})
						}
					}
					BenchmarkCmd::Storage(cmd) => {
						if chain_spec.is_agung() {
							return runner.sync_run(|config| {
								let params = service::new_partial::<agung::RuntimeApi, agung::Executor, _>(
									&config,
									parachain::build_import_queue,
									cli.run.target_gas_price)?;

									let db = params.backend.expose_db();
									let storage = params.backend.expose_storage();

									cmd.run(config, params.client, db, storage)
							})
						} else {
							return runner.sync_run(|config| {
								let params = service::new_partial::<dev::RuntimeApi, dev::Executor, _>(
									&config,
									parachain::build_import_queue,
									cli.run.target_gas_price)?;

									let db = params.backend.expose_db();
									let storage = params.backend.expose_storage();

									cmd.run(config, params.client, db, storage)
							})
						}
					}
					BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
				}
			} else {
				Err("Benchmarking wasn't enabled when building the node. You can enable it with \
					 `--features runtime-benchmarks`."
					.into()
				)
			}
		}
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let spec = cli.load_spec(&params.chain.clone().unwrap_or_default())?;
			let state_version = Cli::native_runtime_version(&spec).state_version();

			let block: Block = generate_genesis_block(&spec, state_version)?;
			let raw_header = block.header().encode();
			let output_buf = if params.raw {
				raw_header
			} else {
				format!("0x{:?}", HexDisplay::from(&block.header().encode())).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		Some(Subcommand::ExportGenesisWasm(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let raw_wasm_blob =
				extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let output_buf = if params.raw {
				raw_wasm_blob
			} else {
				format!("0x{:?}", HexDisplay::from(&raw_wasm_blob)).into_bytes()
			};

			if let Some(output) = &params.output {
				std::fs::write(output, output_buf)?;
			} else {
				std::io::stdout().write_all(&output_buf)?;
			}

			Ok(())
		}
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;

			runner.run_node_until_exit(|config| async move {

				let rpc_config = RpcConfig {
					ethapi: cli.run.ethapi.clone(),
					ethapi_max_permits: cli.run.ethapi_max_permits,
					ethapi_trace_max_count: cli.run.ethapi_trace_max_count,
					ethapi_trace_cache_duration: cli.run.ethapi_trace_cache_duration,
					eth_log_block_cache: cli.run.eth_log_block_cache,
					eth_statuses_cache: cli.run.eth_statuses_cache,
					fee_history_limit: cli.run.fee_history_limit,
					max_past_logs: cli.run.max_past_logs,
					relay_chain_rpc_url: cli.run.base.relay_chain_rpc_url,
				};

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name().to_string()]
						.iter()
						.chain(cli.relaychain_args.iter()),
				);

				let id = ParaId::from(cli.run.parachain_id);

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::v2::AccountId>::into_account(&id);

				let state_version = Cli::native_runtime_version(&config.chain_spec).state_version();
				let block: Block = generate_genesis_block(&config.chain_spec, state_version)
					.map_err(|e| format!("{:?}", e))?;
				let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

				let polkadot_config = SubstrateCli::create_configuration(
					&polkadot_cli,
					&polkadot_cli,
					config.tokio_handle.clone(),
				)
				.map_err(|err| format!("Relay chain argument error: {}", err))?;

				info!("Parachain id: {:?}", id);
				info!("Parachain Account: {}", parachain_account);
				info!("Parachain genesis state: {}", genesis_state);
				info!(
					"Is collating: {}",
					if config.role.is_authority() {
						"yes"
					} else {
						"no"
					}
				);

				if config.chain_spec.is_agung() {
					start_node::<agung::RuntimeApi, agung::Executor>(
						config, polkadot_config, id, rpc_config, cli.run.target_gas_price)
						.await
						.map(|r| r.0)
						.map_err(Into::into)
				} else {
					start_node::<dev::RuntimeApi, dev::Executor>(
						config, polkadot_config, id, rpc_config, cli.run.target_gas_price)
						.await
						.map(|r| r.0)
						.map_err(Into::into)
				}
			})
		}
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base
			.base
			.prometheus_config(default_listen_port, chain_spec)
	}

	fn init<F>(
		&self,
		_support_url: &String,
		_impl_version: &String,
		_logger_hook: F,
		_config: &sc_service::Configuration,
	) -> Result<()>
	where
		F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
	{
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() {
			self.chain_id.clone().unwrap_or_default()
		} else {
			chain_id
		})
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}

	fn telemetry_endpoints(
		&self,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<sc_telemetry::TelemetryEndpoints>> {
		self.base.base.telemetry_endpoints(chain_spec)
	}
}
