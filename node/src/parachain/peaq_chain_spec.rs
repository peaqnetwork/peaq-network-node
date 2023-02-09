use crate::parachain::Extensions;
use cumulus_primitives_core::ParaId;
use peaq_runtime::{
	staking, AccountId, Balance, BalancesConfig, BlockRewardConfig, CouncilConfig, EVMConfig,
	EthereumConfig, GenesisAccount, GenesisConfig, ParachainInfoConfig, ParachainStakingConfig,
	Precompiles, SudoConfig, SystemConfig, DOLLARS, MILLICENTS, TOKEN_DECIMALS, WASM_BINARY,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::Perbill;

use hex_literal::hex;
use sc_network_common::config::MultiaddrWithPeerId;
use std::str::FromStr;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

fn session_keys(aura: AuraId) -> peaq_runtime::opaque::SessionKeys {
	peaq_runtime::opaque::SessionKeys { aura }
}

pub fn get_chain_spec(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		"peaq-network",
		"peaq",
		ChainType::Live,
		move || {
			configure_genesis(
				wasm_binary,
				// stakers
				vec![
					(
						AccountId::try_from(
							&hex!("4ac0ce21b77a91f361be6ac5b72a4e61c20eb90a5eb99a962cd1288d9e62b529") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("d2906b26d5502690fcc4f2a60930d9a6543373051b5c0da6ba2025008e57b23c") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("82040ef2f4c23c6d9102415c964c853c3b249019539ae9ed6d84386780701b35") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					)
				],
				// Initial PoA authorities
				vec![
					(
						AccountId::try_from(
							&hex!("4ac0ce21b77a91f361be6ac5b72a4e61c20eb90a5eb99a962cd1288d9e62b529") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("4ac0ce21b77a91f361be6ac5b72a4e61c20eb90a5eb99a962cd1288d9e62b529") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("d2906b26d5502690fcc4f2a60930d9a6543373051b5c0da6ba2025008e57b23c") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("d2906b26d5502690fcc4f2a60930d9a6543373051b5c0da6ba2025008e57b23c") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("82040ef2f4c23c6d9102415c964c853c3b249019539ae9ed6d84386780701b35") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("82040ef2f4c23c6d9102415c964c853c3b249019539ae9ed6d84386780701b35") as &[u8]
						).unwrap()
					)
				],
				// Sudo account
				hex!("baa6e3c1c492a2324f2ce9bd7f05418597d2e8319924c54e827e52cf51b0747a").into(),
				// Pre-funded accounts
				vec![
					hex!("4ac0ce21b77a91f361be6ac5b72a4e61c20eb90a5eb99a962cd1288d9e62b529").into(),
					hex!("d2906b26d5502690fcc4f2a60930d9a6543373051b5c0da6ba2025008e57b23c").into(),
					hex!("82040ef2f4c23c6d9102415c964c853c3b249019539ae9ed6d84386780701b35").into(),
					//Sudo
					hex!("baa6e3c1c492a2324f2ce9bd7f05418597d2e8319924c54e827e52cf51b0747a").into(),
				],
				para_id.into(),
			)
		},
		// Bootnodes
		vec![
			MultiaddrWithPeerId::from_str("/dns/cn1.peaq.network/tcp/30333/p2p/12D3KooWSiUfLFErmp281eGjXCKZeg1unCGndeq68VF53jbysWrJ").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/cn2.peaq.network/tcp/30333/p2p/12D3KooWFSRsiL6c5VF2NZmp6vnCw8tWPnj1jArR3AWaAuCyN5pb").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/cn3.peaq.network/tcp/30333/p2p/12D3KooW9r1ED5GNvAtNpgeraFgVAQRVikJWQmXEoXWmkmkGS6cD").unwrap(),
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
			relay_chain: "polkadot".into(),
			para_id,
		},
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
		session: peaq_runtime::SessionConfig {
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
			hard_cap: 4_200_000_000 * DOLLARS,
		},
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
		polkadot_xcm: peaq_runtime::PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
		tokens: Default::default(),
		treasury: Default::default(),
		council: CouncilConfig::default(),
	}
}
