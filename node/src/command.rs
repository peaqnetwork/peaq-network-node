use cumulus_client_cli::generate_genesis_block;
use cumulus_primitives_core::ParaId;
#[cfg(feature = "frame-benchmarking-cli")]
use frame_benchmarking_cli::BenchmarkCmd;
use log::info;
use parity_scale_codec::Encode;
use peaq_primitives_xcm::*;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams,
	NetworkParams, Result, RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::{
	config::{BasePath, PrometheusConfig},
	DatabaseSource, PartialComponents,
};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{AccountIdConversion, Block as BlockT};
use std::io::Write;

use crate::{
	cli::{Cli, RelayChainCli, Subcommand},
	cli_opt::{EthApi, RpcConfig},
	parachain,
	parachain::service::{self, agung, dev, frontier_database_dir, krest, peaq, start_node},
};

trait IdentifyChain {
	fn is_dev(&self) -> bool;
	fn is_agung(&self) -> bool;
	fn is_krest(&self) -> bool;
	fn is_peaq(&self) -> bool;
}

impl IdentifyChain for dyn sc_service::ChainSpec {
	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
	fn is_agung(&self) -> bool {
		self.id().starts_with("agung")
	}
	fn is_krest(&self) -> bool {
		self.id().starts_with("krest")
	}
	fn is_peaq(&self) -> bool {
		self.id().starts_with("peaq")
	}
}

macro_rules! with_runtime_or_err {
	($chain_spec:expr, { $( $code:tt )* }) => {
		if $chain_spec.is_dev() {
			#[allow(unused_imports)]
			use dev::{RuntimeApi, Executor};
			$( $code )*
		} else if $chain_spec.is_agung() {
			#[allow(unused_imports)]
			use agung::{RuntimeApi, Executor};
			$( $code )*
		} else if $chain_spec.is_krest() {
			#[allow(unused_imports)]
			use krest::{RuntimeApi, Executor};
			$( $code )*
		} else if $chain_spec.is_peaq() {
			#[allow(unused_imports)]
			use peaq::{RuntimeApi, Executor};
			$( $code )*
		} else {
			return Err("Wrong chain_spec".into());
		}
	}
}

impl<T: sc_service::ChainSpec + 'static> IdentifyChain for T {
	fn is_dev(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_dev(self)
	}
	fn is_agung(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_agung(self)
	}
	fn is_krest(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_krest(self)
	}
	fn is_peaq(&self) -> bool {
		<dyn sc_service::ChainSpec>::is_peaq(self)
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
			"dev" => Box::new(parachain::dev_chain_spec::get_chain_spec()?),
			"dev-local" => Box::new(parachain::dev_chain_spec::get_chain_spec_local_testnet(
				self.run.parachain_id,
			)?),
			"agung-local" => Box::new(parachain::agung_chain_spec::get_chain_spec_local_testnet(
				self.run.parachain_id,
			)?),
			"krest" => Box::new(parachain::krest_chain_spec::get_chain_spec()?),
			"krest-local" => Box::new(parachain::krest_chain_spec::get_chain_spec_local_testnet(
				self.run.parachain_id,
			)?),
			"peaq" => Box::new(parachain::peaq_chain_spec::get_chain_spec()?),
			"peaq-local" => Box::new(parachain::peaq_chain_spec::get_chain_spec_local_testnet(
				self.run.parachain_id,
			)?),
			path => {
				let chain_spec = parachain::agung_chain_spec::ChainSpec::from_json_file(
					std::path::PathBuf::from(path),
				)?;
				if chain_spec.is_dev() {
					Box::new(parachain::dev_chain_spec::ChainSpec::from_json_file(
						std::path::PathBuf::from(path),
					)?)
				} else if chain_spec.is_agung() {
					Box::new(parachain::agung_chain_spec::ChainSpec::from_json_file(
						std::path::PathBuf::from(path),
					)?)
				} else if chain_spec.is_krest() {
					Box::new(parachain::krest_chain_spec::ChainSpec::from_json_file(
						std::path::PathBuf::from(path),
					)?)
				} else if chain_spec.is_peaq() {
					Box::new(parachain::peaq_chain_spec::ChainSpec::from_json_file(
						std::path::PathBuf::from(path),
					)?)
				} else {
					return Err(format!("Wrong chain_spec, {}", path));
				}
			},
		})
	}
}

impl Cli {
	fn runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		if chain_spec.is_agung() {
			&peaq_agung_runtime::VERSION
		} else if chain_spec.is_krest() {
			&peaq_krest_runtime::VERSION
		} else if chain_spec.is_peaq() {
			&peaq_runtime::VERSION
		} else {
			&peaq_dev_runtime::VERSION
		}
	}
}

fn validate_trace_environment(cli: &Cli) -> sc_cli::Result<()> {
	if (cli.run.ethapi.contains(&EthApi::Debug) || cli.run.ethapi.contains(&EthApi::Trace))
		&& cli.run.base.base.import_params.wasm_runtime_overrides.is_none()
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
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name()].iter()).load_spec(id)
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
	validate_trace_environment(&cli)?;

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			with_runtime_or_err!(runner.config().chain_spec, {
				runner.async_run(|mut config| {
					let PartialComponents { client, task_manager, import_queue, .. } =
						service::new_partial::<RuntimeApi, Executor, _>(
							&mut config,
							parachain::build_import_queue,
							cli.run.target_gas_price,
						)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			with_runtime_or_err!(runner.config().chain_spec, {
				runner.async_run(|mut config| {
					let PartialComponents { client, task_manager, .. } =
						service::new_partial::<RuntimeApi, Executor, _>(
							&mut config,
							parachain::build_import_queue,
							cli.run.target_gas_price,
						)?;
					Ok((cmd.run(client, config.database), task_manager))
				})
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			with_runtime_or_err!(runner.config().chain_spec, {
				runner.async_run(|mut config| {
					let PartialComponents { client, task_manager, .. } =
						service::new_partial::<RuntimeApi, Executor, _>(
							&mut config,
							parachain::build_import_queue,
							cli.run.target_gas_price,
						)?;
					Ok((cmd.run(client, config.chain_spec), task_manager))
				})
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			with_runtime_or_err!(runner.config().chain_spec, {
				runner.async_run(|mut config| {
					let PartialComponents { client, task_manager, import_queue, .. } =
						service::new_partial::<RuntimeApi, Executor, _>(
							&mut config,
							parachain::build_import_queue,
							cli.run.target_gas_price,
						)?;
					Ok((cmd.run(client, import_queue), task_manager))
				})
			})
		},
		// [TODO] Revert
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				// Remove Frontier offchain db
				let frontier_database_config = match config.database {
					DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
						path: frontier_database_dir(&config, "db"),
						cache_size: 0,
					},
					DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
						path: frontier_database_dir(&config, "paritydb"),
					},
					_ => {
						return Err(format!("Cannot purge `{:?}` database", config.database).into())
					},
				};
				cmd.run(frontier_database_config)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			with_runtime_or_err!(runner.config().chain_spec, {
				runner.async_run(|mut config| {
					let PartialComponents { client, task_manager, backend, .. } =
						service::new_partial::<RuntimeApi, Executor, _>(
							&mut config,
							parachain::build_import_queue,
							cli.run.target_gas_price,
						)?;
					Ok((cmd.run(client, backend, None), task_manager))
				})
			})
		},
		#[cfg(feature = "frame-benchmarking-cli")]
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				let chain_spec = &runner.config().chain_spec;
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						with_runtime_or_err!(chain_spec, {
							runner.sync_run(|config| cmd.run::<Block, Executor>(config))
						})
					},
					BenchmarkCmd::Block(cmd) => {
						with_runtime_or_err!(chain_spec, {
							runner.sync_run(|mut config| {
								let params = service::new_partial::<RuntimeApi, Executor, _>(
									&mut config,
									parachain::build_import_queue,
									cli.run.target_gas_price,
								)?;

								cmd.run(params.client)
							})
						})
					},
					BenchmarkCmd::Storage(cmd) => {
						with_runtime_or_err!(chain_spec, {
							runner.sync_run(|mut config| {
								let params = service::new_partial::<RuntimeApi, Executor, _>(
									&mut config,
									parachain::build_import_queue,
									cli.run.target_gas_price,
								)?;

								let db = params.backend.expose_db();
								let storage = params.backend.expose_storage();

								cmd.run(config, params.client, db, storage)
							})
						})
					},
					BenchmarkCmd::Extrinsic(_) => Err("Unsupported benchmarking command".into()),
					BenchmarkCmd::Overhead(_) => Err("Unsupported benchmarking command".into()),
					BenchmarkCmd::Machine(cmd) => runner.sync_run(|config| {
						cmd.run(
							&config,
							frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE.clone(),
						)
					}),
				}
			} else {
				Err("Benchmarking wasn't enabled when building the node. You can enable it with \
					 `--features runtime-benchmarks`."
					.into())
			}
		},
		Some(Subcommand::ExportGenesisState(params)) => {
			let mut builder = sc_cli::LoggerBuilder::new("");
			builder.with_profiling(sc_tracing::TracingReceiver::Log, "");
			let _ = builder.init();

			let spec = cli.load_spec(&params.chain.clone().unwrap_or_default())?;
			let state_version = Cli::runtime_version(&spec).state_version();

			let block: Block = generate_genesis_block(&*spec, state_version)?;
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
		},
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
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime will not be supported anymore by the \
            peaq-node. Instead please use the provided CLI tool by Substrate! Have a look at crate \
            `try-runtime-cli`."
			.into()),
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(_)) => Ok(()),
		None => {
			let runner = cli.create_runner(&cli.run.normalize())?;
			let collator_options = cli.run.collator_options();

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
					relay_chain_rpc_urls: cli.run.base.relay_chain_rpc_urls,
					tracing_raw_max_memory_usage: cli.run.tracing_raw_max_memory_usage,
				};

				let polkadot_cli = RelayChainCli::new(
					&config,
					[RelayChainCli::executable_name()].iter().chain(cli.relaychain_args.iter()),
				);

				let id = ParaId::from(cli.run.parachain_id);

				let parachain_account =
					AccountIdConversion::<polkadot_primitives::AccountId>::into_account_truncating(
						&id,
					);

				let state_version = Cli::runtime_version(&config.chain_spec).state_version();
				let block: Block = generate_genesis_block(&*config.chain_spec, state_version)
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
				info!("Is collating: {}", if config.role.is_authority() { "yes" } else { "no" });

				with_runtime_or_err!(config.chain_spec, {
					info!("{} network start", config.chain_spec.id());
					start_node::<RuntimeApi, Executor>(
						config,
						polkadot_config,
						collator_options,
						id,
						rpc_config,
						cli.run.target_gas_price,
					)
					.await
					.map(|r| r.0)
					.map_err(Into::into)
				})
			})
		},
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_listen_port() -> u16 {
		9945
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
			.base_path()?
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn prometheus_config(
		&self,
		default_listen_port: u16,
		chain_spec: &Box<dyn ChainSpec>,
	) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port, chain_spec)
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

		Ok(if chain_id.is_empty() { self.chain_id.clone().unwrap_or_default() } else { chain_id })
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self, is_dev: bool) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool(is_dev)
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
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
