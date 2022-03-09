#[cfg(feature = "manual-seal")]
use structopt::clap::arg_enum;
use structopt::StructOpt;
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
#[derive(Debug, StructOpt)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub base: sc_cli::RunCmd,

	#[cfg(feature = "manual-seal")]
	/// Choose sealing method.
	#[structopt(long = "sealing")]
	pub sealing: Sealing,

	/// Enable EVM tracing module on a non-authority node.
	#[structopt(
		long,
		conflicts_with = "validator",
		require_delimiter = true
	)]
	pub ethapi: Vec<EthApi>,

	/// Number of concurrent tracing tasks. Meant to be shared by both "debug" and "trace" modules.
	#[structopt(long, default_value = "10")]
	pub ethapi_max_permits: u32,

	/// Maximum number of trace entries a single request of `trace_filter` is allowed to return.
	/// A request asking for more or an unbounded one going over this limit will both return an
	/// error.
	#[structopt(long, default_value = "500")]
	pub ethapi_trace_max_count: u32,

	/// Duration (in seconds) after which the cache of `trace_filter` for a given block will be
	/// discarded.
	#[structopt(long, default_value = "300")]
	pub ethapi_trace_cache_duration: u64,

	/// Size of the LRU cache for block data and their transaction statuses.
	#[structopt(long, default_value = "3000")]
	pub eth_log_block_cache: usize,

	#[structopt(long = "enable-dev-signer")]
	pub enable_dev_signer: bool,

	/// Maximum number of logs in a query.
	#[structopt(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum fee history cache size.
	#[structopt(long, default_value = "2048")]
	pub fee_history_limit: u64,

	/// The dynamic-fee pallet target gas price set by block author
	#[structopt(long, default_value = "1")]
	pub target_gas_price: u64,
}

#[derive(Debug, StructOpt)]
pub struct Cli {
	#[structopt(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[structopt(flatten)]
	pub run: RunCmd,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
	/// Key management cli utilities
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

	/// The custom benchmark subcommand benchmarking runtime pallets.
	#[structopt(name = "benchmark", about = "Benchmark runtime pallets.")]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),
}
