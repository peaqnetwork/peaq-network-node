use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	service::{self, frontier_database_dir},
	cli_opt::{EthApi, RpcConfig},
};
use peaq_node_runtime::Block;
use sp_runtime::traits::Block as _;
use sc_cli::{ChainSpec, RuntimeVersion, SubstrateCli, Result};
use sc_service::PartialComponents;
use frame_benchmarking_cli::BenchmarkCmd;

// Parachain
use codec::Encode;
use sp_core::hexdisplay::HexDisplay;
use cumulus_client_service::genesis::generate_genesis_block;
use std::io::Write;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"PEAQ Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		//[TODO]
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		//[TODO]
		"support.anonymous.an".into()
	}

	fn copyright_start_year() -> i32 {
		2021
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(chain_spec::development_config()?),
			"agung" => Box::new(chain_spec::agung_net_config()?),
			path => Box::new(chain_spec::ChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}

	fn native_runtime_version(_: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		&peaq_node_runtime::VERSION
	}
}

fn validate_trace_environment(cli: &Cli) -> sc_cli::Result<()> {
	if (cli.run.ethapi.contains(&EthApi::Debug) || cli.run.ethapi.contains(&EthApi::Trace))
		&& cli
			.run
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
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial(&config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		}
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = service::new_partial(&config, &cli)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		}
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					..
				} = service::new_partial(&config, &cli)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		}
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					import_queue,
					..
				} = service::new_partial(&config, &cli)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
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
			runner.async_run(|config| {
				let PartialComponents {
					client,
					task_manager,
					backend,
					..
				} = service::new_partial(&config, &cli)?;
				Ok((cmd.run(client, backend, None), task_manager))
			})
		}
		Some(Subcommand::Benchmark(cmd)) => {
			if cfg!(feature = "runtime-benchmarks") {
				let runner = cli.create_runner(cmd)?;
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						return runner.sync_run(|config| {
							cmd.run::<Block, service::ExecutorDispatch>(config)
						})
					}
					BenchmarkCmd::Block(cmd) => {
						return runner.sync_run(|mut config| {
							let params = service::new_partial(&mut config, &cli)?;

							cmd.run(params.client)
						})
					}
					BenchmarkCmd::Storage(cmd) => {
							return runner.sync_run(|mut config| {
								let params = service::new_partial(&mut config, &cli)?;

								let db = params.backend.expose_db();
								let storage = params.backend.expose_storage();

								cmd.run(config, params.client, db, storage)
						})
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

			// [TODO]??? moonbeam
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
			let runner = cli.create_runner(&cli.run.base)?;
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
			};

				service::new_full(config, &cli, rpc_config).map_err(sc_cli::Error::Service)
			})
		}
	}
}
