use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;
use sp_runtime::traits::{Bounded, Convert};
use sp_std::{borrow::Borrow, marker::PhantomData};
use xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};
use xcm::latest::{
	prelude::{Fungibility, MultiAsset, MultiLocation, XcmError},
	Weight,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::WeightTrader;

/// A MultiLocation-AssetId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct PeaqAssetIdConvert<AssetId, AssetMapper>(PhantomData<(AssetId, AssetMapper)>);

impl<AssetId, AssetMapper> xcm_executor::traits::Convert<MultiLocation, AssetId>
	for PeaqAssetIdConvert<AssetId, AssetMapper>
where
	AssetId: Clone + Eq,
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
	 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location,
	 * relay_location, aca_loaction, bnc_location);         if now == peaq_location {
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
	 *         // log::error!("PeaqAssetIdConvert: {:?}, {:?}, {:?}, {:?}", peaq_location,
	 * relay_location, aca_loaction, bnc_location);         // log::error!("id {:?}",
	 * id.borrow().clone());         match id.borrow().clone() {
	 *             0 => Ok(peaq_location),
	 *             1 => {
	 *                 // log::error!("Reverse: id {:?}, relay_location {:?}",
	 * id.borrow().clone(), relay_location);                 Ok(relay_location)
	 *             },
	 *             2 => Ok(aca_loaction),
	 *             3 => {
	 *                 // log::error!("Reverse: id {:?}, bnc_location {:?}", id.borrow().clone(),
	 * bnc_location);                 Ok(bnc_location)
	 *             },
	 *             _ => Err(()),
	 *         }
	 *     }
	 */
}

impl<AssetId, AssetMapper> Convert<AssetId, Option<MultiLocation>>
	for PeaqAssetIdConvert<AssetId, AssetMapper>
where
	AssetId: Clone + Eq,
	AssetMapper: XcAssetLocation<AssetId>,
{
	fn convert(id: AssetId) -> Option<MultiLocation> {
		<PeaqAssetIdConvert<AssetId, AssetMapper> as xcm_executor::traits::Convert<
			MultiLocation,
			AssetId,
		>>::reverse(id)
		.ok()
	}
}

// [TODO] Need to check
/// Used as weight trader for foreign assets.
///
/// In case foreigin asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
	/// Total used weight
	weight: Weight,
	/// Total consumed assets
	consumed: u128,
	/// Asset Id (as MultiLocation) and units per second for payment
	asset_location_and_units_per_second: Option<(MultiLocation, u128)>,
	_pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
	fn new() -> Self {
		Self {
			weight: Weight::zero(),
			consumed: 0,
			asset_location_and_units_per_second: None,
			_pd: PhantomData,
		}
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: xcm_executor::Assets,
	) -> Result<xcm_executor::Assets, XcmError> {
		log::trace!(
			target: "xcm::weight",
			"FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
			weight, payment,
		);

		// Atm in pallet, we only support one asset so this should work
		let payment_asset = payment.fungible_assets_iter().next().ok_or(XcmError::TooExpensive)?;

		match payment_asset {
			MultiAsset {
				id: xcm::latest::AssetId::Concrete(asset_location),
				fun: Fungibility::Fungible(_),
			} => {
				if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
					let amount = units_per_second.saturating_mul(weight.ref_time() as u128) // TODO: change this to u64?
                        / (WEIGHT_REF_TIME_PER_SECOND as u128);
					if amount == 0 {
						return Ok(payment)
					}

					let unused = payment
						.checked_sub((asset_location.clone(), amount).into())
						.map_err(|_| XcmError::TooExpensive)?;

					self.weight = self.weight.saturating_add(weight);

					// If there are multiple calls to `BuyExecution` but with different assets, we
					// need to be able to handle that. Current primitive implementation will just
					// keep total track of consumed asset for the FIRST consumed asset. Others will
					// just be ignored when refund is concerned.
					if let Some((old_asset_location, _)) =
						self.asset_location_and_units_per_second.clone()
					{
						if old_asset_location == asset_location {
							self.consumed = self.consumed.saturating_add(amount);
						}
					} else {
						self.consumed = self.consumed.saturating_add(amount);
						self.asset_location_and_units_per_second =
							Some((asset_location, units_per_second));
					}

					Ok(unused)
				} else {
					Err(XcmError::TooExpensive)
				}
			},
			_ => Err(XcmError::TooExpensive),
		}
	}

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

		if let Some((asset_location, units_per_second)) =
			self.asset_location_and_units_per_second.clone()
		{
			let weight = weight.min(self.weight);
			let amount = units_per_second.saturating_mul(weight.ref_time() as u128) /
				(WEIGHT_REF_TIME_PER_SECOND as u128);

			self.weight = self.weight.saturating_sub(weight);
			self.consumed = self.consumed.saturating_sub(amount);

			if amount > 0 {
				Some((asset_location, amount).into())
			} else {
				None
			}
		} else {
			None
		}
	}
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
	fn drop(&mut self) {
		if let Some((asset_location, _)) = self.asset_location_and_units_per_second.clone() {
			if self.consumed > 0 {
				R::take_revenue((asset_location, self.consumed).into());
			}
		}
	}
}

/*
 * impl<AssetId, AssetMapper> Convert<MultiLocation, Option<AssetId>> for
 * PeaqAssetIdConvert<AssetId, AssetMapper> where
 *     AssetId: Clone + Eq + Bounded,
 *     AssetMapper: XcAssetLocation<AssetId>,
 * {
 *     fn convert(location: MultiLocation) -> Option<AssetId> {
 *         <PeaqAssetIdConvert<AssetId, AssetMapper> as
 * xcm_executor::traits::Convert<MultiLocation, AssetId>>::convert(location).ok()     }
 * }
 *
 */
// impl<T> Convert<PeaqAssetId, Option<MultiLocation>> for PeaqAssetIdConvert<T>
// where
// 	T: SysConfig + ParaSysConfig,
// {
// 	fn convert(id: PeaqAssetId) -> Option<MultiLocation> {
// 		<PeaqAssetIdConvert<T> as xcm_executor::traits::Convert<MultiLocation,
// PeaqAssetId>>::reverse(id).ok() 	}
// }
