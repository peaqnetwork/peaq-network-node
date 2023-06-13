//! Submodule for Zenlink-DEX-Module integration

use super::{
	Balances, Currencies, CurrencyId, ParachainInfo, Runtime, RuntimeEvent,
	Tokens, TokenSymbol, ZenlinkProtocol, Timestamp, ZenlinkStableAmm, Balance,
};
use frame_support::{log, parameter_types, pallet_prelude::*, PalletId};
use orml_traits::MultiCurrency;
use sp_std::{vec, vec::Vec};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, PairLpGenerate,
	ZenlinkMultiAssets,
};
use zenlink_stable_amm::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};
use zenlink_vault::VaultAssetGenerate;


// Zenlink-DEX Parameter definitions
parameter_types! {
	pub SelfParaId: u32 = ParachainInfo::parachain_id().into();

	pub const ZenlinkDexPalletId: PalletId = PalletId(*b"zenlkpro");
	pub const StableAmmPalletId: PalletId = PalletId(*b"zenlkamm");
	pub const StringLimit: u32 = 50;
	pub const VaultPalletId: PalletId = PalletId(*b"zenlkvau");

	pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
		// Krest local and live, 0.01 BNC
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(2000))), 10_000_000_000),
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(3000))), 10_000_000_000),
		// TODO: More to be added

		// Zenlink local 1 for test
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(200))), 1_000_000),
		// Zenlink local 2 for test
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(300))), 1_000_000),
	];
}


impl zenlink_protocol::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
    type MultiAssetsHandler = MultiAssets;
    type PalletId = ZenlinkDexPalletId;
    type AssetId = ZenlinkAssetId;
    type LpGenerate = PairLpGenerate<Self>;
    type TargetChains = ZenlinkRegistedParaChains;
    type SelfParaId = SelfParaId;
    type WeightInfo = (); //zenlink_protocol::default_weights::SubstrateWeight<Runtime>;
}

type PoolId = u32;

impl zenlink_stable_amm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Tokens;
	type PoolId = PoolId;
	type TimeProvider = Timestamp;
	type EnsurePoolAsset = StableAmmVerifyPoolAsset;
	type LpGenerate = PoolLpGenerate;
	type PoolCurrencySymbolLimit = StringLimit;
	type PalletId = StableAmmPalletId;
	type WeightInfo = ();
}

impl zenlink_swap_router::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StablePoolId = PoolId;
	type Balance = Balance;
	type StableCurrencyId = CurrencyId;
	type NormalCurrencyId = ZenlinkAssetId;
	type NormalAmm = ZenlinkProtocol;
	type StableAMM = ZenlinkStableAmm;
	type WeightInfo = ();
}

impl zenlink_vault::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = CurrencyId;
	type MultiAsset = Tokens;
	type VaultAssetGenerate = VaultAssetGenerator;
	type PalletId = VaultPalletId;
	type WeightInfo = ();
}


/// TODO documentation
pub type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;


/// TODO documentation
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

/*
 * impl<Local> LocalAssetAdaptor<Local> {
 *     // ZenlinkAssetId{
 *     // 	chain_id: u32,
 *     // 	asset_type: u8,
 *     // 	asset_index: u64,
 *     // }
 *
 *     fn currency_into_asset(asset_id: ZenlinkAssetId) -> Result<CurrencyId, ()> {
 *         log::error!("asset_id.chain_id: {:?}", asset_id.chain_id);
 *         log::error!("asset_id.asset_type: {:?}", asset_id.asset_type);
 *         log::error!("asset_id.asset_index: {:?}", asset_id.asset_index);
 *         log::error!("SelfParaId: {:?}", SelfParaId::get());
 *         log::error!("zenlink_protocol::NATIVE: {:?}", zenlink_protocol::NATIVE);
 *         if asset_id.chain_id != SelfParaId::get() {
 *             return Err(());
 *         }
 *         match asset_id.asset_type {
 *             zenlink_protocol::NATIVE => {
 *                 if asset_id.asset_index == 0 {
 *                     Ok(CurrencyId::Token(TokenSymbol::PEAQ))
 *                 } else {
 *                     Err(())
 *                 }
 *             },
 *             zenlink_protocol::LOCAL => {
 *                 if asset_id.asset_index == 0 {
 *                     Ok(CurrencyId::Token(TokenSymbol::DOT))
 *                 } else {
 *                     Err(())
 *                 }
 *             },
 *             _ => Err(()),
 *         }
 *     }
 * }
 */

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
		// if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, &who))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
		// if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		log::error!("QQQQ asset_id: {:?}", asset_id);
		let currency_id: Result<CurrencyId, ()> = asset_id.try_into();
		// let currency_id: Result<CurrencyId, ()> =
		//	LocalAssetAdaptor::<Local>::currency_into_asset(asset_id);
		match currency_id {
			Ok(_) => true,
			Err(_) => false,
		}
	}

	fn local_transfer(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = asset_id.try_into() {
		// if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			Local::transfer(
				currency_id,
				&origin,
				&target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			)
		} else {
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
		// if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			Local::deposit(
				currency_id,
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
		}

		Ok(amount)
	}

	fn local_withdraw(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
		// if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			Local::withdraw(
				currency_id,
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
		}

		Ok(amount)
	}
}


// /// TODO documentation
pub struct PoolLpGenerate;

impl StablePoolLpCurrencyIdGenerate<CurrencyId, PoolId> for PoolLpGenerate {
	fn generate_by_pool_id(pool_id: PoolId) -> CurrencyId {
		CurrencyId::StableLpToken(pool_id)
	}
}


// /// TODO documentation
pub struct StableAmmVerifyPoolAsset;

impl ValidateCurrency<CurrencyId> for StableAmmVerifyPoolAsset {
	fn validate_pooled_currency(_currencies: &[CurrencyId]) -> bool {
		true
	}

	fn validate_pool_lp_currency(_currency_id: CurrencyId) -> bool {
		if Tokens::total_issuance(_currency_id) > 0 {
			return false
		}
		true
	}
}


// /// TODO documentation
pub struct VaultAssetGenerator;

impl VaultAssetGenerate<CurrencyId> for VaultAssetGenerator {
	fn generate(asset: CurrencyId) -> Option<CurrencyId> {
		match asset {
			CurrencyId::Token(token_symbol) => Some(CurrencyId::Vault(token_symbol)),
			_ => None,
		}
	}
}

