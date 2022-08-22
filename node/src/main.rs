//! Peaq Node CLI library.
#![warn(missing_docs)]

mod parachain;
mod primitives;
mod cli;
mod cli_opt;
mod command;
mod rpc;

fn main() -> sc_cli::Result<()> {
	command::run()
}
