use crate::parachain::Extensions;
use cumulus_primitives_core::ParaId;
use peaq_krest_runtime::{
	staking, BalancesConfig, BlockRewardConfig, CouncilConfig, EVMConfig, EthereumConfig,
	GenesisAccount, ParachainInfoConfig, ParachainStakingConfig, PeaqPrecompiles, Runtime,
	RuntimeGenesisConfig, StakingCoefficientRewardCalculatorConfig, SudoConfig, WASM_BINARY,
};
use peaq_primitives_xcm::{AccountId, Balance};
use runtime_common::TOKEN_DECIMALS;
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::Perbill;

use crate::parachain::dev_chain_spec::{authority_keys_from_seed, get_account_id_from_seed};

use sp_core::sr25519;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

fn session_keys(aura: AuraId) -> peaq_krest_runtime::opaque::SessionKeys {
	peaq_krest_runtime::opaque::SessionKeys { aura }
}

pub fn get_chain_spec() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../chain-specs/krest-raw.json")[..])
}

pub fn get_chain_spec_local_testnet(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "KREST".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	#[allow(deprecated)]
	Ok(ChainSpec::from_genesis(
		"krest-network",
		"krest-local",
		ChainType::Local,
		move || {
			configure_genesis(
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
		// code
		wasm_binary,
	))
}

/// Configure initial storage state for FRAME modules.
fn configure_genesis(
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	initial_authorities: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	parachain_id: ParaId,
) -> RuntimeGenesisConfig {
	// This is supposed the be the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	RuntimeGenesisConfig {
		system: Default::default(),
		parachain_info: ParachainInfoConfig { parachain_id, ..Default::default() },
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
			max_candidate_stake: staking::MAX_COLLATOR_STAKE,
		},
		staking_coefficient_reward_calculator: StakingCoefficientRewardCalculatorConfig {
			coefficient: staking::coefficient(),
			..Default::default()
		},
		inflation_manager: Default::default(),
		block_reward: BlockRewardConfig {
			// Make sure sum is 100
			reward_config: pallet_block_reward::RewardDistributionConfig {
				treasury_percent: Perbill::from_percent(25),
				collators_delegators_percent: Perbill::from_percent(40),
				coretime_percent: Perbill::from_percent(10),
				subsidization_pool_percent: Perbill::from_percent(5),
				depin_staking_percent: Perbill::from_percent(5),
				depin_incentivization_percent: Perbill::from_percent(15),
			},
			_phantom: Default::default(),
		},
		vesting: Default::default(),
		aura: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		aura_ext: Default::default(),
		evm: EVMConfig {
			accounts: PeaqPrecompiles::<Runtime>::used_addresses()
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
			..Default::default()
		},
		ethereum: EthereumConfig { ..Default::default() },
		dynamic_fee: Default::default(),
		base_fee: Default::default(),
		polkadot_xcm: peaq_krest_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		treasury: Default::default(),
		council: CouncilConfig::default(),
		assets: Default::default(),
		// async_backing_vesting_block_provider: Default::default(),
	}
}
