use peaq_agung_runtime::{TOKEN_DECIMALS, MILLICENTS, DOLLARS};
use peaq_agung_runtime::{
	AccountId, BalancesConfig, EVMConfig, EthereumConfig, GenesisAccount, GenesisConfig,
	SudoConfig, SystemConfig, WASM_BINARY, Precompiles, ParachainInfoConfig,
	staking, Balance, ParachainStakingConfig,
	BlockRewardConfig,
};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use cumulus_primitives_core::ParaId;
use crate::parachain::Extensions;
use sp_runtime::{
	Perbill,
};

use hex_literal::hex;
use sc_network_common::config::MultiaddrWithPeerId;
use std::str::FromStr;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

fn session_keys(aura: AuraId) -> peaq_agung_runtime::opaque::SessionKeys {
    peaq_agung_runtime::opaque::SessionKeys { aura }
}

pub fn get_chain_spec(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "AGNG".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	Ok(ChainSpec::from_genesis(
		"agung-network",
		"agung-testnet",
		ChainType::Local,
		move || {
			configure_genesis(
				wasm_binary,
				// stakers
				vec![
					(
						AccountId::try_from(
							&hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					), (
						AccountId::try_from(
							&hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073") as &[u8]
						).unwrap(),
						None,
						2 * staking::MinCollatorStake::get(),
					)
				],
				// Initial PoA authorities
				vec![
					(
						AccountId::try_from(
							&hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a") as &[u8]
						).unwrap()
					),
					(
						AccountId::try_from(
							&hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073") as &[u8]
						).unwrap(),
						AuraId::try_from(
							&hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073") as &[u8]
						).unwrap()
					)
				],
				// Sudo account
				hex!("f6f16b29e9ba748f41c1bf361d1925359b256edc99ba5c57541e07cc79465202").into(),
				// Pre-funded accounts
				vec![
					hex!("086732fee8cfbcdc9c9ac3931d85d0a997d88602bdaa7a137c9c4c43101fe416").into(),
					hex!("be9889f446dbb0a2fba44932a2ae7f1d3d6b34a186d8901875ecfce8970b395e").into(),
					hex!("f610c763f6c8c282a700a87f48e46b955630b56c284a2ffb2b83d1f8548bb750").into(),
					hex!("bec3d9d0cb9991e3f87ac2b8c03184c060aafa964593af74feb70381d11dd97a").into(),
					hex!("c4b6a019eef3471a0825fe69ed0205c056e7ce1d3560c93f083c5d6cf8305073").into(),
					//Sudo
					hex!("f6f16b29e9ba748f41c1bf361d1925359b256edc99ba5c57541e07cc79465202").into(),
				],
				para_id.into(),
			)
		},
		// Bootnodes
		vec![
			MultiaddrWithPeerId::from_str("/dns/bn1.agung.peaq.network/tcp/10333/p2p/12D3KooWMJps5U6mBp2JUJpewuSAz2wbEf59Rm98kgwDj5LZd5rW").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn2.agung.peaq.network/tcp/10333/p2p/12D3KooW9pipmGRbECY9gwD6ArJVvC9CP24XxdoCUSq8eY2dJ5Cd").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn3.agung.peaq.network/tcp/10333/p2p/12D3KooWPafWKojir1pCGmN5DwW6yQ33exdEMSUz5xX6t3EPAwqS").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn4.agung.peaq.network/tcp/10333/p2p/12D3KooWCumywbmYpGLBTDXxcbw7rzM9WNU1Hyqs6P6ftCsHBxmV").unwrap(),
			MultiaddrWithPeerId::from_str("/dns/bn5.agung.peaq.network/tcp/10333/p2p/12D3KooWAo4D2rXY3kZM4zqSkxHNSS8Q1eonYX4F5x811Qd9y4gb").unwrap(),
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
			relay_chain: "rococo-local".into(),
			para_id: para_id,
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
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 78))
				.collect(),
		},
		session: peaq_agung_runtime::SessionConfig {
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
			block_issue_reward: 79_09_867 * MILLICENTS,
			hard_cap: 4_200_000_000 * DOLLARS,
		},
		asset_registry: Default::default(),
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
		polkadot_xcm: peaq_agung_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
		},
		tokens: Default::default(),
	}
}
