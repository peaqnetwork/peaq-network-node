//! Peaq Node CLI library.
#![warn(missing_docs)]

mod cli;
mod cli_opt;
mod command;
mod parachain;
mod primitives;
mod rpc;

fn main() -> sc_cli::Result<()> {
	command::run()
}
