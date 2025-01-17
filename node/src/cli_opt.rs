use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EthApi {
	Txpool,
	Debug,
	Trace,
}

impl FromStr for EthApi {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"txpool" => Self::Txpool,
			"debug" => Self::Debug,
			"trace" => Self::Trace,
			_ => return Err(format!("`{}` is not recognized as a supported Ethereum Api", s)),
		})
	}
}

pub struct RpcConfig {
	pub ethapi: Vec<EthApi>,
	pub ethapi_max_permits: u32,
	pub ethapi_trace_max_count: u32,
	pub ethapi_trace_cache_duration: u64,
	pub eth_log_block_cache: usize,
	pub eth_statuses_cache: usize,
	pub fee_history_limit: u64,
	pub max_past_logs: u32,
	pub relay_chain_rpc_urls: Vec<url::Url>,
	pub tracing_raw_max_memory_usage: usize,
}

#[derive(Clone)]
/// To add additional config to start_xyz_node functions
pub struct AdditionalConfig {
	// We don't need to have evm_tracing_config because we are already get from other place
	// We don't need to have enable_evm_rpc because we are always enabling it
	/// Maxium allowed block size limit to propose
	pub proposer_block_size_limit: usize,

	/// Soft deadline limit used by `Proposer`
	pub proposer_soft_deadline_percent: u8,
	/// Hardware benchmarks score
	pub hwbench: Option<sc_sysinfo::HwBench>,
}
