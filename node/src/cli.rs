#[cfg(feature = "manual-seal")]
use structopt::clap::arg_enum;

use clap::Parser;
use crate::cli_opt::EthApi;

#[cfg(feature = "manual-seal")]
arg_enum! {
	/// Available Sealing methods.
	#[derive(Debug, Copy, Clone, StructOpt)]
	pub enum Sealing {
		// Seal using rpc method.
		Manual,
		// Seal when transaction is executed.
		Instant,
	}
}

#[cfg(feature = "manual-seal")]
impl Default for Sealing {
	fn default() -> Sealing {
		Sealing::Manual
	}
}

#[allow(missing_docs)]
#[derive(Debug, Parser)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,

	#[cfg(feature = "manual-seal")]
	/// Choose sealing method.
	#[clap(long = "sealing")]
	pub sealing: Sealing,

	/// Enable EVM tracing module on a non-authority node.
	#[clap(
		long,
		conflicts_with = "validator",
		use_value_delimiter = true,
		require_value_delimiter = true,
		multiple_values = true
	)]
	pub ethapi: Vec<EthApi>,

	/// Number of concurrent tracing tasks. Meant to be shared by both "debug" and "trace" modules.
	#[clap(long, default_value = "10")]
	pub ethapi_max_permits: u32,

	/// Size in bytes of data a raw tracing request is allowed to use.
	/// Bound the size of memory, stack and storage data.
	#[clap(long, default_value = "20000000")]
	pub tracing_raw_max_memory_usage: usize,

	/// Maximum number of trace entries a single request of `trace_filter` is allowed to return.
	/// A request asking for more or an unbounded one going over this limit will both return an
	/// error.
	#[clap(long, default_value = "500")]
	pub ethapi_trace_max_count: u32,

	/// Duration (in seconds) after which the cache of `trace_filter` for a given block will be
	/// discarded.
	#[clap(long, default_value = "300")]
	pub ethapi_trace_cache_duration: u64,

	/// Size of the LRU cache for block data and their transaction statuses.
	#[clap(long, default_value = "3000")]
	pub eth_log_block_cache: usize,

	/// Size in bytes of the LRU cache for transactions statuses data.
	#[clap(long, default_value = "300000000")]
	pub eth_statuses_cache: usize,

	/// Maximum number of logs in a query.
	#[clap(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum fee history cache size.
	#[clap(long, default_value = "2048")]
	pub fee_history_limit: u64,

	/// The dynamic-fee pallet target gas price set by block author
	#[clap(long, default_value = "1")]
	pub target_gas_price: u64,
}

#[derive(Debug, Parser)]
#[clap(
	propagate_version = true,
	args_conflicts_with_subcommands = true,
	subcommand_negates_reqs = true
)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[clap(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// Key management cli utilities
	#[clap(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	/// The pallet benchmarking moved to the `pallet` sub-command.
	#[clap(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}
