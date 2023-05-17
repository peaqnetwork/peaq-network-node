//! Submodule for Zenlink-DEX-Module integration

use super::{
	AccountId, Balances, Currencies, CurrencyId, ParachainInfo, Runtime, RuntimeEvent,
	ZenlinkProtocol,
};
use frame_support::{parameter_types, pallet_prelude::*, PalletId};
// use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_std::{vec, vec::Vec};
use xcm::latest::prelude::*;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, MultiAssetsHandler, PairLpGenerate,
	ZenlinkMultiAssets,
};



// Zenlink-DEX Parameter definitions
parameter_types! {
	pub const ZenlinkDexPalletId: PalletId = PalletId(*b"zenlinkd");
	pub SelfParaId: u32 = ParachainInfo::parachain_id().into();

	pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
		// Krest local and live, 0.01 BNC
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(2241))), 10_000_000_000),
		// TODO: More to be added

		// Zenlink local 1 for test
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(200))), 1_000_000),
		// Zenlink local 2 for test
		(MultiLocation::new(1, Junctions::X1(Junction::Parachain(300))), 1_000_000),
	];
}

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local> LocalAssetAdaptor<Local> {
	// ZenlinkAssetId{
	// 	chain_id: u32,
	// 	asset_type: u8,
	// 	asset_index: u64,
	// }

	fn currency_into_asset(assed_id: ZenlinkAssetId) -> Result<CurrencyId, ()> {
		Err(())
	}
}

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		// if let Ok(currency_id) = asset_id.try_into() {
		if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, &who))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		// if let Ok(currency_id) = asset_id.try_into() {
		if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		// let currency_id: Result<CurrencyId, ()> = asset_id.try_into();
		let currency_id: Result<CurrencyId, ()> =
			LocalAssetAdaptor::<Local>::currency_into_asset(asset_id);
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
		// if let Ok(currency_id) = asset_id.try_into() {
		if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
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
		// if let Ok(currency_id) = asset_id.try_into() {
		if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
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
		// if let Ok(currency_id) = asset_id.try_into() {
		if let Ok(currency_id) = LocalAssetAdaptor::<Local>::currency_into_asset(asset_id) {
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
