use crate::parachain::Extensions;
use cumulus_primitives_core::ParaId;
use peaq_dev_runtime::{
	staking, BalancesConfig, BlockRewardConfig, CouncilConfig, EVMConfig, EthereumConfig,
	GenesisAccount, MorConfig, ParachainInfoConfig, ParachainStakingConfig, PeaqMorConfig,
	PeaqPrecompiles, Runtime, RuntimeGenesisConfig, SudoConfig, WASM_BINARY,
};
use peaq_primitives_xcm::{AccountId, Balance, Signature};
use runtime_common::{CENTS, DOLLARS, MILLICENTS, TOKEN_DECIMALS};
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	Perbill,
};

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

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

pub fn get_chain_spec() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../chain-specs/peaq-dev-raw.json")[..])
}

pub fn get_chain_spec_local_testnet(para_id: u32) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	let mut properties = Properties::new();
	properties.insert("tokenSymbol".into(), "PEAQ".into());
	properties.insert("tokenDecimals".into(), TOKEN_DECIMALS.into());

	#[allow(deprecated)]
	Ok(ChainSpec::from_genesis(
		"peaq-dev",
		"dev-testnet",
		ChainType::Development,
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
		Extensions { bad_blocks: Default::default(), relay_chain: "rococo-local".into(), para_id },
		// code
		wasm_binary,
	))
}

fn session_keys(aura: AuraId) -> peaq_dev_runtime::opaque::SessionKeys {
	peaq_dev_runtime::opaque::SessionKeys { aura }
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
		session: peaq_dev_runtime::SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.1.clone())))
				.collect::<Vec<_>>(),
		},
		parachain_staking: ParachainStakingConfig {
			stakers,
			max_candidate_stake: staking::MAX_COLLATOR_STAKE,
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
		polkadot_xcm: peaq_dev_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		treasury: Default::default(),
		council: CouncilConfig::default(),
		peaq_mor: PeaqMorConfig {
			mor_config: MorConfig {
				registration_reward: 10 * CENTS,
				machine_usage_fee_min: MILLICENTS,
				machine_usage_fee_max: 3 * DOLLARS,
				track_n_block_rewards: 200,
			},
		},
		assets: Default::default(),
	}
}
