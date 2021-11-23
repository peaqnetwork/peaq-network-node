use peaq_node_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig, Signature, SudoConfig,
	SystemConfig, WASM_BINARY,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sc_network::config::MultiaddrWithPeerId;
use std::str::FromStr;
use hex_literal::hex;

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
		"PEAQ-dev",
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
		vec![
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

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "PEAQ".into());
    properties.insert("tokenDecimals".into(), 18.into());

	Ok(ChainSpec::from_genesis(
		// Name
		"PEAQ-testnet",
		// ID
		"peaq-substrate-testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					(
						AuraId::from_slice(&hex!("c0d50370df4900c0234dc2447c83c71d2cb2b9e8a2f0cbf51e0bedec11619b0a")),
						GrandpaId::from_slice(&hex!("f28e8001386c013f42c34489fa6ba3b412103640fab66cf801d57f28a2b15066"))
					),
					(
						AuraId::from_slice(&hex!("344d37ba3b5d262a75fe245f32bda90552bf3297e6c45ab49e2fee61739e6970")),
						GrandpaId::from_slice(&hex!("ace38a477865e9e35b1158e09f6312625ffa0f71cafea3c21f09945472e6e929"))
					),
					(
						AuraId::from_slice(&hex!("e6f71b2580e5eef4c6a8166976282eac9524147b65cbcbf5895e1dfc767bf840")),
						GrandpaId::from_slice(&hex!("5ef30971c5793d9072f14dbe3460b3eb7846e940ad4e81420df0880fa7775119"))
					),
					(
						AuraId::from_slice(&hex!("a60c413a74fe233132659161a102b5fc70101b7b27b4a678cf28b660d5597e6a")),
						GrandpaId::from_slice(&hex!("c2ec9279d562b043cca04a64a132a6c659ece88b46634aaabeaaeb9534395fb2"))
					),
					(
						AuraId::from_slice(&hex!("b8c2ff7e29908f6fd216c648709e1294b634e73e2aac111c948509db12a5343f")),
						GrandpaId::from_slice(&hex!("09bbcf123d668ba7e22f1915e79a0ed3a67bcacf047929d23464561c17f89018"))
					)
				],
				// Sudo account
				hex!("146d6b59fd1d6ac17312a33c2619dbf245f190a7c6a07b10d025e645ca9a5135").into(),
				// Pre-funded accounts
				vec![
					hex!("c0d50370df4900c0234dc2447c83c71d2cb2b9e8a2f0cbf51e0bedec11619b0a").into(),
					hex!("344d37ba3b5d262a75fe245f32bda90552bf3297e6c45ab49e2fee61739e6970").into(),
					hex!("e6f71b2580e5eef4c6a8166976282eac9524147b65cbcbf5895e1dfc767bf840").into(),
					hex!("a60c413a74fe233132659161a102b5fc70101b7b27b4a678cf28b660d5597e6a").into(),
					hex!("b8c2ff7e29908f6fd216c648709e1294b634e73e2aac111c948509db12a5343f").into(),
					//Sudo
					hex!("146d6b59fd1d6ac17312a33c2619dbf245f190a7c6a07b10d025e645ca9a5135").into(),
				],
				true,
			)
		},
		// Bootnodes
		vec![
			MultiaddrWithPeerId::from_str("/dns/bn1.test.peaq.network/tcp/10333/p2p/12D3KooWPPgfvmaonCgVKpRVx6FbdtDNtXJDh19ppUUGz4qS9KoY").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn2.test.peaq.network/tcp/10333/p2p/12D3KooWKrSNAVWfFjLMGizcGcNwrWK1CsYL3Mwm7wV4aXza66ka").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn3.test.peaq.network/tcp/10333/p2p/12D3KooWA3TSY7VjqqiHgumPkCUpWuzjBhDZVCSZtKg9xn85TCwR").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn4.test.peaq.network/tcp/10333/p2p/12D3KooWA3p68a55qkgZWWKcGLkUGCD6jLF8SyyPxbyeP8yckXDw").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn5.test.peaq.network/tcp/10333/p2p/12D3KooWAbz5wSnF4gecBTxUNJkibmE2cmCJRbooDUkg1Pe8Y5B6").unwrap(),
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
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		aura: AuraConfig {
			authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
		},
		grandpa: GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect(),
		},
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},
	}
}
