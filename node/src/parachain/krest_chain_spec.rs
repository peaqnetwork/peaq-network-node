use crate::parachain::Extensions;
use cumulus_primitives_core::ParaId;
use peaq_krest_runtime::{
	staking, AccountId, Balance, BalancesConfig, BlockRewardConfig, CouncilConfig, EVMConfig,
	EthereumConfig, GenesisAccount, GenesisConfig, ParachainInfoConfig, ParachainStakingConfig,
	Precompiles, SudoConfig, SystemConfig, DOLLARS, MILLICENTS, TOKEN_DECIMALS, WASM_BINARY,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::Perbill;

use crate::parachain::dev_chain_spec::{authority_keys_from_seed, get_account_id_from_seed};
use hex_literal::hex;
use sc_network_common::config::MultiaddrWithPeerId;
use sp_core::sr25519;
use std::str::FromStr;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

fn session_keys(aura: AuraId) -> peaq_krest_runtime::opaque::SessionKeys {
	peaq_krest_runtime::opaque::SessionKeys { aura }
}

pub fn get_chain_spec() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../chain-specs/krest.json")[..])
}

pub fn get_chain_spec_testnet(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "KREST".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		"krest-network",
		"krest",
		ChainType::Development,
		move || {
			configure_genesis(
				wasm_binary,
				// stakers
				vec![
					(
						AccountId::try_from(
							&hex!("d0724eec97826a56d1d0aa61f667025d81e047aa6408d4b1a82569d86c643e14") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("b8a553fc8364134c2856e229fe67d2bbc10c7a3575935b809c4299df08c8bd37") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("44771aff6488cfc95016804e23fd6fd370d8cc1a975d4178ad2b78a8656b2f46") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					)
				],
				// Initial PoA authorities
				vec![
					(
						AccountId::try_from(
							&hex!("d0724eec97826a56d1d0aa61f667025d81e047aa6408d4b1a82569d86c643e14") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("d0724eec97826a56d1d0aa61f667025d81e047aa6408d4b1a82569d86c643e14") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("b8a553fc8364134c2856e229fe67d2bbc10c7a3575935b809c4299df08c8bd37") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("b8a553fc8364134c2856e229fe67d2bbc10c7a3575935b809c4299df08c8bd37") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("44771aff6488cfc95016804e23fd6fd370d8cc1a975d4178ad2b78a8656b2f46") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("44771aff6488cfc95016804e23fd6fd370d8cc1a975d4178ad2b78a8656b2f46") as &[u8]
						).unwrap()
					)
				],
				// Sudo account
				hex!("baa6e3c1c492a2324f2ce9bd7f05418597d2e8319924c54e827e52cf51b0747a").into(),
				// Pre-funded accounts
				vec![
					hex!("d0724eec97826a56d1d0aa61f667025d81e047aa6408d4b1a82569d86c643e14").into(),
					hex!("b8a553fc8364134c2856e229fe67d2bbc10c7a3575935b809c4299df08c8bd37").into(),
					hex!("44771aff6488cfc95016804e23fd6fd370d8cc1a975d4178ad2b78a8656b2f46").into(),
					//Sudo
					hex!("baa6e3c1c492a2324f2ce9bd7f05418597d2e8319924c54e827e52cf51b0747a").into(),
				],
				para_id.into(),
			)
		},
		// Bootnodes
		vec![
			MultiaddrWithPeerId::from_str("/dns/cn1krest.peaq.network/tcp/30333/p2p/12D3KooWSiUfLFErmp281eGjXCKZeg1unCGndeq68VF53jbysWrJ").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/cn2krest.peaq.network/tcp/30333/p2p/12D3KooWFSRsiL6c5VF2NZmp6vnCw8tWPnj1jArR3AWaAuCyN5pb").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/cn3krest.peaq.network/tcp/30333/p2p/12D3KooW9r1ED5GNvAtNpgeraFgVAQRVikJWQmXEoXWmkmkGS6cD").unwrap(),
		],
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
			relay_chain: "kusama".into(),
			para_id,
		},
	))
}

pub fn get_chain_spec_local_testnet(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "KREST".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		"krest-network",
		"krest-local",
		ChainType::Local,
		move || {
			configure_genesis(
				wasm_binary,
				// stakers
				vec![(
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					None,
					2 * staking::MinCollatorStake::get(),
				)],
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
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
		Extensions { bad_blocks: Default::default(), relay_chain: "kusama-local".into(), para_id },
	))
}

/// Configure initial storage state for FRAME modules.
fn configure_genesis(
	wasm_binary: &[u8],
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
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
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 78)).collect(),
		},
		session: peaq_krest_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone())))
				.collect::<Vec<_>>(),
		},
		parachain_staking: ParachainStakingConfig {
			stakers,
			reward_rate_config: staking::reward_rate_config(),
			max_candidate_stake: staking::MAX_COLLATOR_STAKE,
		},
		block_reward: BlockRewardConfig {
			// Make sure sum is 100
			reward_config: pallet_block_reward::RewardDistributionConfig {
				treasury_percent: Perbill::from_percent(20),
				dapps_percent: Perbill::from_percent(25),
				collators_percent: Perbill::from_percent(10),
				lp_percent: Perbill::from_percent(25),
				machines_percent: Perbill::from_percent(10),
				machines_subsidization_percent: Perbill::from_percent(10),
			},
			block_issue_reward: 7_909_867 * MILLICENTS,
			max_currency_supply: 4_200_000_000 * DOLLARS,
		},
		vesting: peaq_krest_runtime::VestingConfig { vesting: vec![] },
		aura: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		aura_ext: Default::default(),
		evm: EVMConfig {
			accounts: Precompiles::used_addresses()
				.map(|addr| {
					(
						addr,
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
		polkadot_xcm: peaq_krest_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
		},
		tokens: Default::default(),
		treasury: Default::default(),
		council: CouncilConfig::default(),
	}
}
