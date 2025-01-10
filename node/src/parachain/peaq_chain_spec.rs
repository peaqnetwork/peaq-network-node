use crate::parachain::Extensions;
use cumulus_primitives_core::ParaId;
use peaq_primitives_xcm::{AccountId, Balance};
use peaq_runtime::{
	staking, BalancesConfig, BlockRewardConfig, CouncilConfig, EVMConfig, EthereumConfig,
	GenesisAccount, ParachainInfoConfig, ParachainStakingConfig, PeaqPrecompiles, Runtime,
	RuntimeGenesisConfig, SudoConfig, WASM_BINARY,
};
use runtime_common::TOKEN_DECIMALS;
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::Perbill;

use crate::parachain::dev_chain_spec::{authority_keys_from_seed, get_account_id_from_seed};

use sp_core::sr25519;

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

fn session_keys(aura: AuraId) -> peaq_runtime::opaque::SessionKeys {
	peaq_runtime::opaque::SessionKeys { aura }
}

pub fn get_chain_spec() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../chain-specs/peaq-raw.json")[..])
}

pub fn get_chain_spec_local_testnet(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

    Ok(ChainSpec::builder(
		wasm_binary,
        Extensions {
			bad_blocks: Default::default(),
            relay_chain: "polkadot-local".into(),
            para_id: para_id,
        },
    )
    .with_name("peaq-network")
    .with_id("peaq-local")
    .with_chain_type(ChainType::Local)
	.with_genesis_config_patch(configure_genesis(
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
	))
	.with_properties(properties)
	.build())
}

/// Configure initial storage state for FRAME modules.
fn configure_genesis(
	stakers: Vec<(AccountId, Option<AccountId>, Balance)>,
	initial_authorities: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	parachain_id: ParaId,
) -> serde_json::Value {
	// This is supposed the be the simplest bytecode to revert without returning any data.
	// We will pre-deploy it under all of our precompiles to ensure they can be called from
	// within contracts.
	// (PUSH1 0x00 PUSH1 0x00 REVERT)
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	serde_json::json!({
		"parachainInfo": {
			"parachainId": parachain_id,
		},
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u128 << 78)).collect::<Vec<_>>(),
		},
		"session": {
			"keys": initial_authorities.iter().map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone()))).collect::<Vec<_>>(),
		},
		"parachainStaking": {
			"stakers": stakers,
			"maxCandidateStake": staking::MAX_COLLATOR_STAKE,
		},
		"blockReward": {
			"rewardConfig": {
				"treasuryPercent": Perbill::from_percent(25),
				"collatorsDelegatorsPercent": Perbill::from_percent(40),
				"coretimePercent": Perbill::from_percent(10),
				"subsidizationPoolPercent": Perbill::from_percent(5),
				"depinStakingPercent": Perbill::from_percent(5),
				"depinIncentivizationPercent": Perbill::from_percent(15),
			},
		},
		"sudo": {
			"key": Some(root_key),
		},
		"evm": {
			"accounts": PeaqPrecompiles::<Runtime>::used_addresses().map(|addr| {
				(addr, GenesisAccount {
					nonce: Default::default(),
					balance: Default::default(),
					storage: Default::default(),
					code: revert_bytecode.clone(),
				})
			}).collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
	})
}
