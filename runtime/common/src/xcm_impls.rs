use frame_support::{pallet_prelude::Get, weights::constants::WEIGHT_REF_TIME_PER_SECOND};
use peaq_primitives_xcm::{
	PeaqCurrencyId,
	NATIVE_CURRNECY_ID,
};
use sp_runtime::traits::Convert;
use sp_std::{borrow::Borrow, marker::PhantomData};
use xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};
use xcm::latest::{
	prelude::{Fungibility, GeneralKey, MultiAsset, MultiLocation, XcmError, X1},
	Weight,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::WeightTrader;

pub fn self_native_currency_location() -> MultiLocation {
	MultiLocation::new(0, X1(GeneralKey { data: [0; 32], length: 2 }))
}

/// A MultiLocation-AssetId converter for XCM, Zenlink-Protocol and similar stuff.
pub struct PeaqCurrencyIdConvert<AssetMapper, SelfReserve>(PhantomData<(AssetMapper, SelfReserve)>);

impl<AssetMapper, SelfReserve> xcm_executor::traits::Convert<MultiLocation, PeaqCurrencyId>
	for PeaqCurrencyIdConvert<AssetMapper, SelfReserve>
where
	AssetMapper: XcAssetLocation<PeaqCurrencyId>,
	SelfReserve: Get<MultiLocation>,
{
	fn convert_ref(location: impl Borrow<MultiLocation>) -> Result<PeaqCurrencyId, ()> {
		let location = location.borrow().clone();
		// [TODO] Should I move to the AssetMapping?
		if location == SelfReserve::get() {
			return Ok(NATIVE_CURRNECY_ID);
		}
		if let Some(asset_id) = AssetMapper::get_asset_id(location) {
			Ok(asset_id)
		} else {
			Err(())
		}
	}

	fn reverse_ref(id: impl Borrow<PeaqCurrencyId>) -> Result<MultiLocation, ()> {
		let id = id.borrow().clone();
		if id == NATIVE_CURRNECY_ID {
			return Ok(SelfReserve::get())
		}
		if let Some(multilocation) = AssetMapper::get_xc_asset_location(id) {
			Ok(multilocation)
		} else {
			Err(())
		}
	}
}

impl<AssetMapper, SelfReserve> Convert<PeaqCurrencyId, Option<MultiLocation>>
	for PeaqCurrencyIdConvert<AssetMapper, SelfReserve>
where
	AssetMapper: XcAssetLocation<PeaqCurrencyId>,
	SelfReserve: Get<MultiLocation>,
{
	fn convert(id: PeaqCurrencyId) -> Option<MultiLocation> {
		<PeaqCurrencyIdConvert<AssetMapper, SelfReserve> as xcm_executor::traits::Convert<
			MultiLocation,
			PeaqCurrencyId,
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
