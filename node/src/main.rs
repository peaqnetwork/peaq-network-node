//! Substrate Node Template CLI library.
#![warn(missing_docs)]

// [TODO]
// mod chain_spec;
mod parachain;
mod primitives;
#[macro_use]
mod service;
mod cli;
mod cli_opt;
mod command;
mod rpc;

fn main() -> sc_cli::Result<()> {
	command::run()
}
