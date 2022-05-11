//! Support for Astar ecosystem parachains.

/// Shell to Aura consensus upgrades.
mod shell_upgrade;

/// Parachain specified service.
pub mod service;

/// Parachain specs.
pub mod chain_spec;

pub use chain_spec::{
	development_config, ChainSpec,
	// agung_net_config
};

pub use service::{
    build_import_queue, new_partial, dev, start_dev_node,
};
