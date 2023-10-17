use frame_support::{
	traits::{
		fungibles, Get,
	},
};
use frame_system::Config as SysConfig;
use pallet_assets::Config as AssetsConfig;
use sp_runtime::traits::{
	Convert,
};
use sp_std::{marker::PhantomData};
use zenlink_protocol::GenerateLpAssetId;

use peaq_primitives_xcm::{PeaqCurrencyId, PeaqCurrencyIdToZenlinkId};
use zenlink_protocol::{
	AssetId as ZenlinkAssetId, Config as ZenProtConfig,
};



/// This is the Peaq's default GenerateLpAssetId implementation.
pub struct PeaqAssetZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>(
	PhantomData<(T, Local, ExistentialDeposit, AdminAccount)>,
);

impl<T, Local, ExistentialDeposit, AdminAccount> GenerateLpAssetId<ZenlinkAssetId>
	for PeaqAssetZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>
where
	Local: fungibles::Create<T::AccountId, AssetId = PeaqCurrencyId, Balance = T::Balance>
		+ fungibles::Inspect<T::AccountId, AssetId = PeaqCurrencyId, Balance = T::Balance>,
	T: SysConfig + AssetsConfig + ZenProtConfig,
	ExistentialDeposit: Get<T::Balance>,
	AdminAccount: Get<T::AccountId>,
{
	fn generate_lp_asset_id(
		asset0: ZenlinkAssetId,
		asset1: ZenlinkAssetId,
	) -> Option<ZenlinkAssetId> {
		let asset_id0: PeaqCurrencyId = asset0.try_into().ok()?;
		let asset_id1: PeaqCurrencyId = asset1.try_into().ok()?;

		match (asset_id0, asset_id1) {
			(PeaqCurrencyId::Token(symbol0), PeaqCurrencyId::Token(symbol1)) => {
				PeaqCurrencyIdToZenlinkId::<T::SelfParaId>::convert(PeaqCurrencyId::LPToken(
					symbol0, symbol1,
				))
			},

			(_, _) => None,
		}
	}

	fn create_lp_asset(asset0: &ZenlinkAssetId, asset1: &ZenlinkAssetId) -> Option<()> {
		let asset_id0: PeaqCurrencyId = (*asset0).try_into().ok()?;
		let asset_id1: PeaqCurrencyId = (*asset1).try_into().ok()?;

		match (asset_id0, asset_id1) {
			(PeaqCurrencyId::Token(symbol0), PeaqCurrencyId::Token(symbol1)) => {
				let lp_currency = PeaqCurrencyId::LPToken(symbol0, symbol1);
				Local::create(lp_currency, AdminAccount::get(), true, ExistentialDeposit::get())
					.ok()?;
				Some(())
			},
			(_, _) => None,
		}
	}
}
