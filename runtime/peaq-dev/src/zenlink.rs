//! Submodule for Zenlink-DEX-Module integration

use super::{
	Balance, Balances, Currencies, CurrencyId, LpPoolId, ParachainInfo, Runtime, RuntimeEvent,
	Tokens, Timestamp, ZenlinkProtocol, // ZenlinkStableAmm,
};
use frame_support::{parameter_types, pallet_prelude::*, PalletId};
use orml_traits::MultiCurrency;
use sp_std::{vec, vec::Vec};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, PairLpGenerate,
	ZenlinkMultiAssets,
};
// use zenlink_stable_amm::traits::{StablePoolLpCurrencyIdGenerate, ValidateCurrency};


// Zenlink-DEX Parameter definitions
parameter_types! {
	pub SelfParaId: u32 = ParachainInfo::parachain_id().into();

	pub const ZenlinkDexPalletId: PalletId = PalletId(*b"zenlkpro");
	// pub const StableAmmPalletId: PalletId = PalletId(*b"zenlkamm");
	pub const StringLimit: u32 = 50;

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
    type WeightInfo = ();
}

// impl zenlink_stable_amm::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type CurrencyId = CurrencyId;
// 	type MultiCurrency = Currencies;
// 	type PoolId = LpPoolId;
// 	type TimeProvider = Timestamp;
// 	type EnsurePoolAsset = StableAmmVerifyPoolAsset;
// 	type LpGenerate = PoolLpGenerate;
// 	type PoolCurrencySymbolLimit = StringLimit;
// 	type PalletId = StableAmmPalletId;
// 	type WeightInfo = ();
// }

// impl zenlink_swap_router::Config for Runtime {
// 	type RuntimeEvent = RuntimeEvent;
// 	type StablePoolId = LpPoolId;
// 	type Balance = Balance;
// 	type StableCurrencyId = CurrencyId;
// 	type NormalCurrencyId = ZenlinkAssetId;
// 	type NormalAmm = ZenlinkProtocol;
// 	type StableAMM = ZenlinkStableAmm;
// 	type WeightInfo = zenlink_swap_router::weights::SubstrateWeight<Runtime>;
// }


/// Short form for our individual configuration of Zenlink's MultiAssets.
pub type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;


/// A local adaptor to convert between Zenlink-Assets and our local currency.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, &who))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		let currency_id: Result<CurrencyId, ()> = asset_id.try_into();
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


// /// A very simple Liquidity-Pool generator to transform a LpPoolId into CurrencyId.
// pub struct PoolLpGenerate;

// impl StablePoolLpCurrencyIdGenerate<CurrencyId, LpPoolId> for PoolLpGenerate {
// 	fn generate_by_pool_id(pool_id: LpPoolId) -> CurrencyId {
// 		CurrencyId::StableLpToken(pool_id)
// 	}
// }


// /// TODO documentation
// pub struct StableAmmVerifyPoolAsset;

// impl ValidateCurrency<CurrencyId> for StableAmmVerifyPoolAsset {
// 	fn validate_pooled_currency(_currencies: &[CurrencyId]) -> bool {
// 		true
// 	}

// 	fn validate_pool_lp_currency(_currency_id: CurrencyId) -> bool {
// 		if Tokens::total_issuance(_currency_id) > 0 {
// 			return false
// 		}
// 		true
// 	}
// }
