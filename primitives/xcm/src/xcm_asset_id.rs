use sp_std::{borrow::Borrow, marker::PhantomData};
use xc_asset_config::{XcAssetLocation};
use xcm::latest::prelude::MultiLocation;
use sp_runtime::traits::Bounded;
use sp_runtime::traits::Convert;

/// A MultiLocation-AssetId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct PeaqAssetIdConvert<AssetId, AssetMapper>(PhantomData<(AssetId, AssetMapper)>);

impl<AssetId, AssetMapper> xcm_executor::traits::Convert<MultiLocation, AssetId>
    for PeaqAssetIdConvert<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
    fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<AssetId, ()> {
        if let Some(asset_id) = AssetMapper::get_asset_id(location.borrow().clone()) {
            Ok(asset_id)
        } else {
            Err(())
        }
    }

    fn reverse_ref(id: impl Borrow<AssetId>) -> Result<MultiLocation, ()> {
        if let Some(multilocation) = AssetMapper::get_xc_asset_location(id.borrow().clone()) {
            Ok(multilocation)
        } else {
            Err(())
        }
    }

/*
 *     fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<PeaqAssetId, ()> {
 *         let	peaq_location = native_currency_location(
 *                 <T as ParaSysConfig>::SelfParaId::get().into(),
 *                 [0, 0].encode()
 *             ).expect("Fail").into_versioned();
 *         let relay_location = MultiLocation::parent().into_versioned();
 *         let aca_loaction = native_currency_location(
 *                 parachain::acala::ID,
 *                 parachain::acala::ACA_KEY.to_vec()).expect("Fail").into_versioned();
 *         let bnc_location = native_currency_location(
 *                 parachain::bifrost::ID,
 *                 parachain::bifrost::BNC_KEY.to_vec()).expect("Fail").into_versioned();
 *         let now = location.borrow().clone().into_versioned();
 *
 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location, relay_location, aca_loaction, bnc_location);
 *         if now == peaq_location {
 *             // log::error!("Convert: now PeaqAssetIdConvert: {:?}", peaq_location);
 *             Ok(0)
 *         } else if now == relay_location {
 *             Ok(1)
 *         } else if now == aca_loaction {
 *             Ok(2)
 *         } else if now == bnc_location {
 *             // log::error!("Convert: bnc PeaqAssetIdConvert: {:?}", bnc_location);
 *             Ok(3)
 *         } else {
 *             Err(())
 *         }
 *     }
 *
 *     fn reverse_ref(id: impl Borrow<PeaqAssetId>) -> Result<MultiLocation, ()> {
 *         let	peaq_location = native_currency_location(
 *                 <T as ParaSysConfig>::SelfParaId::get().into(),
 *                 [0, 0].encode()
 *             ).expect("Fail");
 *         let relay_location = MultiLocation::parent();
 *         let aca_loaction = native_currency_location(
 *                 parachain::acala::ID,
 *                 parachain::acala::ACA_KEY.to_vec()).expect("Fail");
 *         let bnc_location = native_currency_location(
 *                 parachain::bifrost::ID,
 *                 parachain::bifrost::BNC_KEY.to_vec()).expect("Fail");
 *
 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location, relay_location, aca_loaction, bnc_location);
 *         // log::error!("id {:?}", id.borrow().clone());
 *         match id.borrow().clone() {
 *             0 => Ok(peaq_location),
 *             1 => {
 *                 // log::error!("Reverse: id {:?}, relay_location {:?}", id.borrow().clone(), relay_location);
 *                 Ok(relay_location)
 *             },
 *             2 => Ok(aca_loaction),
 *             3 => {
 *                 // log::error!("Reverse: id {:?}, bnc_location {:?}", id.borrow().clone(), bnc_location);
 *                 Ok(bnc_location)
 *             },
 *             _ => Err(()),
 *         }
 *     }
 */
}

impl<AssetId, AssetMapper> Convert<AssetId, Option<MultiLocation>> for PeaqAssetIdConvert<AssetId, AssetMapper>
where
    AssetId: Clone + Eq + Bounded,
    AssetMapper: XcAssetLocation<AssetId>,
{
	fn convert(id: AssetId) -> Option<MultiLocation> {
		<PeaqAssetIdConvert<AssetId, AssetMapper> as xcm_executor::traits::Convert<MultiLocation, AssetId>>::reverse(id).ok()
	}
}

/*
 * impl<AssetId, AssetMapper> Convert<MultiLocation, Option<AssetId>> for PeaqAssetIdConvert<AssetId, AssetMapper>
 * where
 *     AssetId: Clone + Eq + Bounded,
 *     AssetMapper: XcAssetLocation<AssetId>,
 * {
 *     fn convert(location: MultiLocation) -> Option<AssetId> {
 *         <PeaqAssetIdConvert<AssetId, AssetMapper> as xcm_executor::traits::Convert<MultiLocation, AssetId>>::convert(location).ok()
 *     }
 * }
 *
 */
// impl<T> Convert<PeaqAssetId, Option<MultiLocation>> for PeaqAssetIdConvert<T>
// where
// 	T: SysConfig + ParaSysConfig,
// {
// 	fn convert(id: PeaqAssetId) -> Option<MultiLocation> {
// 		<PeaqAssetIdConvert<T> as xcm_executor::traits::Convert<MultiLocation, PeaqAssetId>>::reverse(id).ok()
// 	}
// }
