use hex_literal::hex;
use peaq_node_runtime::{
	AccountId, AuraConfig, BalancesConfig, EVMConfig, EthereumConfig, GenesisConfig, GrandpaConfig,
	Signature, SudoConfig, SystemConfig, WASM_BINARY, Precompiles,
};
use sc_network::config::MultiaddrWithPeerId;
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
	(get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), 18.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"peaq-dev",
		// ID
		"peaq-substrate-dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), 18.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"peaq-testnet",
		// ID
		"peaq-substrate-testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					(
						AuraId::from_slice(&hex!("c0a03ff255c2db2ddb33acc9885c3607eea411006cbe5cc1511c29762c8f8e0c")),
						GrandpaId::from_slice(&hex!("0c4f41c73ede92f54c621da69e434310d53b59c37b5f7148f23e33167115770b"))
					),
					(
						AuraId::from_slice(&hex!("3661f26abbaa07d7df01e4c1348457ad9ede2f96c14f271beef7da0faadbe532")),
						GrandpaId::from_slice(&hex!("69b44ae0c55c8284a4157141dc6bed6ea2d1e9d5b16a88b715a0dfada1659faa"))
					),
					(
						AuraId::from_slice(&hex!("243d9dacf4835501479ec16b3a3c44661ee967b26de75dfeb3af2c8660e0f80a")),
						GrandpaId::from_slice(&hex!("8107c34c8f6a5f968a5311c9911d92d78432d03be8f9bf7ed913cc089b3c6db1"))
					),
					(
						AuraId::from_slice(&hex!("dc2318d3925aa5cb69f47219f31480a73d709ffaca323d06874f75fbed00e527")),
						GrandpaId::from_slice(&hex!("1534240b466e40d055d67e1477f4fb4e04655d8405c7abe0e6d52844735c3e7d"))
					),
					(
						AuraId::from_slice(&hex!("920ff8bb3da346bdde5e1a43e05379651ef853df188499d53861cb2e221e1e6d")),
						GrandpaId::from_slice(&hex!("23a16d750210fc8d6f3301f06345140c0356bbba4df52452d92a1b87b8af0c37"))
					)
				],
				// Sudo account
				hex!("e43082fa42efb0b22be8991f3f62c84b9e3411ef23a25b6e95c2da0937167226").into(),
				// Pre-funded accounts
				vec![
					hex!("c0a03ff255c2db2ddb33acc9885c3607eea411006cbe5cc1511c29762c8f8e0c").into(),
					hex!("3661f26abbaa07d7df01e4c1348457ad9ede2f96c14f271beef7da0faadbe532").into(),
					hex!("243d9dacf4835501479ec16b3a3c44661ee967b26de75dfeb3af2c8660e0f80a").into(),
					hex!("dc2318d3925aa5cb69f47219f31480a73d709ffaca323d06874f75fbed00e527").into(),
					hex!("920ff8bb3da346bdde5e1a43e05379651ef853df188499d53861cb2e221e1e6d").into(),
					//Sudo
					hex!("e43082fa42efb0b22be8991f3f62c84b9e3411ef23a25b6e95c2da0937167226").into(),
				],
				true,
			)
		},
		// Bootnodes
		vec![
			MultiaddrWithPeerId::from_str("/dns/bn1.test.peaq.network/tcp/10333/p2p/12D3KooWAfuyTS1eM1aa14XaQUZ9Q17J5Po4mg8ccQQVsY6Mf1kg").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn2.test.peaq.network/tcp/10333/p2p/12D3KooWQ6SoNpbCtwTnDAS5Wj6z7h7jvnUWYN1PN3L2NH1xrWKB").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn3.test.peaq.network/tcp/10333/p2p/12D3KooWMZZkKXMRtHkjJDYcAMAZj9496oQ8tcaYCEA1FvJKgFtX").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn4.test.peaq.network/tcp/10333/p2p/12D3KooWPM4L2ijaf5mLmHF5KDsubKYTy9RbWvFcWiYLKSeLA2nE").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn5.test.peaq.network/tcp/10333/p2p/12D3KooWCvRzKLxJXTVZqEvNGtRHU2DzuW6bV54jW3LhRUWngaJS").unwrap(),
		],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	// This is supposed the be the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 78.
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 78))
				.collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities
				.iter()
				.map(|x| (x.1.clone(), 1))
				.collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},
		evm: EVMConfig {
			accounts: Precompiles::used_addresses()
				.map(|addr| {
					(
						addr.into(),
						pallet_evm::GenesisAccount {
							nonce: Default::default(),
							balance: Default::default(),
							storage: Default::default(),
							code: revert_bytecode.clone(),
						},
					)
				})
				.collect(),
		},
		ethereum: EthereumConfig {},
		dynamic_fee: Default::default(),
		base_fee: Default::default(),
	}
}
