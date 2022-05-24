use peaq_dev_runtime::{
	AccountId, BalancesConfig, EVMConfig, EthereumConfig, GenesisAccount, GenesisConfig,
	Signature, SudoConfig, SystemConfig, WASM_BINARY, Precompiles, ParachainInfoConfig,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use cumulus_primitives_core::ParaId;
use crate::parachain::Extensions;

// [TODO] Agung
// use hex_literal::hex;
// use sc_network::config::MultiaddrWithPeerId;
// use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId) {
	(get_account_id_from_seed::<sr25519::Public>(s), get_from_seed::<AuraId>(s))
}

pub fn development_config(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), 18.into());

	Ok(ChainSpec::from_genesis(
		"peaq-dev",
		"peaq-substrate-dev",
		ChainType::Development,
		move || {
			configure_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					authority_keys_from_seed("Alice"),
				],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
				],
				para_id.into(),
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Fork ID
		None,
		// Properties
		Some(properties),
		// Extensions
		Extensions {
			bad_blocks: Default::default(),
			relay_chain: "test-service".into(),
			para_id: para_id,
		},
	))
}

/*
 * pub fn agung_net_config() -> Result<ChainSpec, String> {
 *     let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;
 *
 *     let mut properties = Properties::new();
 *     properties.insert("tokenSymbol".into(), "AGNG".into());
 *     properties.insert("tokenDecimals".into(), 18.into());
 *
 *     Ok(ChainSpec::from_genesis(
 *         // Name
 *         "agung-network",
 *         // ID
 *         "agung-substrate-testnet",
 *         ChainType::Local,
 *         move || {
 *             configure_genesis(
 *                 wasm_binary,
 *                 // Initial PoA authorities
 *                 vec![
 *                     (
 *                         AuraId::try_from(
 *                             &hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416") as &[u8]
 *                         ).unwrap(),
 *                         AccountId::try_from(
 *                             &hex!("f45dc8a48fd2cd4e59bf53c4a36a36c0964a14ab76742d891d837731af2c60cc") as &[u8]
 *                         ).unwrap()
 *                     ),
 *                     (
 *                         AuraId::try_from(
 *                             &hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e") as &[u8]
 *                         ).unwrap(),
 *                         AccountId::try_from(
 *                             &hex!("a4af12973c4c027600fd198e9226781f1ce3755a74ae5efc726dbd4ebf958854") as &[u8]
 *                         ).unwrap()
 *                     ),
 *                     (
 *                         AuraId::try_from(
 *                             &hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750") as &[u8]
 *                         ).unwrap(),
 *                         AccountId::try_from(
 *                             &hex!("3f6fca05fe7ba7f7625d855dfd2b0af911c192294111530f24df7d0b28892885") as &[u8]
 *                         ).unwrap()
 *                     ),
 *                     (
 *                         AuraId::try_from(
 *                             &hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a") as &[u8]
 *                         ).unwrap(),
 *                         AccountId::try_from(
 *                             &hex!("43f6612faccb685e36009a63d5000aab6551b901bcda9ae708923d19dd033128") as &[u8]
 *                         ).unwrap()
 *                     ),
 *                     (
 *                         AuraId::try_from(
 *                             &hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073") as &[u8]
 *                         ).unwrap(),
 *                         AccountId::try_from(
 *                             &hex!("050f8fe5db72dcba0ea5f7d976a042d2899d696714464838ba431a806b5dd7d6") as &[u8]
 *                         ).unwrap()
 *                     )
 *                 ],
 *                 // Sudo account
 *                 hex!("f6f16b29e9ba748f41c1bf361d1925359b256edc99ba5c57541e07cc79465202").into(),
 *                 // Pre-funded accounts
 *                 vec![
 *                     hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416").into(),
 *                     hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e").into(),
 *                     hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750").into(),
 *                     hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a").into(),
 *                     hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073").into(),
 *                     //Sudo
 *                     hex!("f6f16b29e9ba748f41c1bf361d1925359b256edc99ba5c57541e07cc79465202").into(),
 *                 ],
 *                 true,
 *             )
 *         },
 *         // Bootnodes
 *         vec![
 *             MultiaddrWithPeerId::from_str("/dns/bn1.agung.peaq.network/tcp/10333/p2p/12D3KooWMJps5U6mBp2JUJpewuSAz2wbEf59Rm98kgwDj5LZd5rW").unwrap(),
 *             MultiaddrWithPeerId::from_str("/dns/bn2.agung.peaq.network/tcp/10333/p2p/12D3KooW9pipmGRbECY9gwD6ArJVvC9CP24XxdoCUSq8eY2dJ5Cd").unwrap(),
 *             MultiaddrWithPeerId::from_str("/dns/bn3.agung.peaq.network/tcp/10333/p2p/12D3KooWPafWKojir1pCGmN5DwW6yQ33exdEMSUz5xX6t3EPAwqS").unwrap(),
 *             MultiaddrWithPeerId::from_str("/dns/bn4.agung.peaq.network/tcp/10333/p2p/12D3KooWCumywbmYpGLBTDXxcbw7rzM9WNU1Hyqs6P6ftCsHBxmV").unwrap(),
 *             MultiaddrWithPeerId::from_str("/dns/bn5.agung.peaq.network/tcp/10333/p2p/12D3KooWAo4D2rXY3kZM4zqSkxHNSS8Q1eonYX4F5x811Qd9y4gb").unwrap(),
 *         ],
 *         // Telemetry
 *         None,
 *         // Protocol ID
 *         None,
 *         // Fork ID
 *         None,
 *         // Properties
 *         Some(properties),
 *         // Extensions
 *         None,
 *     ))
 * }
 */

fn session_keys(aura: AuraId) -> peaq_dev_runtime::opaque::SessionKeys {
    peaq_dev_runtime::opaque::SessionKeys { aura }
}

/// Configure initial storage state for FRAME modules.
fn configure_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
    parachain_id: ParaId,
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
		parachain_info: ParachainInfoConfig { parachain_id },
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 78.
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 78))
				.collect(),
		},
		// [TODO]...
		// block_reward
		session: peaq_dev_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone())))
				.collect::<Vec<_>>(),
		},
		aura: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		aura_ext: Default::default(),
		collator_selection: peaq_dev_runtime::CollatorSelectionConfig {
			desired_candidates: 200,
			// [TODO]...
			candidacy_bond: 32_000,
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		evm: EVMConfig {
			accounts: Precompiles::used_addresses()
				.map(|addr| {
					(
						addr.into(),
						GenesisAccount {
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
