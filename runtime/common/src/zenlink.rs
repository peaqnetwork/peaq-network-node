use frame_support::traits::{fungibles, Get};
use frame_system::Config as SysConfig;
use pallet_assets::Config as AssetsConfig;
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;
use zenlink_protocol::GenerateLpAssetId;

use peaq_primitives_xcm::{AssetId as PeaqAssetId, AssetIdToZenlinkId};
use zenlink_protocol::{AssetId as ZenlinkAssetId, Config as ZenProtConfig};
type AssetId = u64;

/// This is the Peaq's default GenerateLpAssetId implementation.
pub struct PeaqAssetZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>(
	PhantomData<(T, Local, ExistentialDeposit, AdminAccount)>,
);

impl<T, Local, ExistentialDeposit, AdminAccount> GenerateLpAssetId<ZenlinkAssetId>
	for PeaqAssetZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>
where
	Local: fungibles::Create<T::AccountId, AssetId = AssetId, Balance = T::Balance>
		+ fungibles::Inspect<T::AccountId, AssetId = AssetId, Balance = T::Balance>,
	T: SysConfig + AssetsConfig + ZenProtConfig,
	ExistentialDeposit: Get<T::Balance>,
	AdminAccount: Get<T::AccountId>,
{
	fn generate_lp_asset_id(
		asset0: ZenlinkAssetId,
		asset1: ZenlinkAssetId,
	) -> Option<ZenlinkAssetId> {
		let asset_id0: PeaqAssetId = asset0.try_into().ok()?;
		let asset_id1: PeaqAssetId = asset1.try_into().ok()?;

		match (asset_id0, asset_id1) {
			(PeaqAssetId::Token(symbol0), PeaqAssetId::Token(symbol1)) =>
				AssetIdToZenlinkId::<T::SelfParaId>::convert(
					PeaqAssetId::LPToken(symbol0, symbol1).try_into().ok()?,
				),

			(_, _) => None,
		}
	}

	fn create_lp_asset(asset0: &ZenlinkAssetId, asset1: &ZenlinkAssetId) -> Option<()> {
		let asset_id0: PeaqAssetId = (*asset0).try_into().ok()?;
		let asset_id1: PeaqAssetId = (*asset1).try_into().ok()?;

		match (asset_id0, asset_id1) {
			(PeaqAssetId::Token(symbol0), PeaqAssetId::Token(symbol1)) => {
				let lp_currency = PeaqAssetId::LPToken(symbol0, symbol1);
				Local::create(
					lp_currency.try_into().ok()?,
					AdminAccount::get(),
					true,
					ExistentialDeposit::get(),
				)
				.ok()?;
				// Cannot setup the metadata for the LP asset because admin account doesn't have enough balance.
				Some(())
			},
			(_, _) => None,
		}
	}
}
