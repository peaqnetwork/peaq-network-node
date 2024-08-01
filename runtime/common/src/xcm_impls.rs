use cumulus_primitives_core::XcmContext;
use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;
use sp_std::marker::PhantomData;
use xc_asset_config::ExecutionPaymentRate;
use xcm::latest::{
	prelude::{Asset, Fungibility, Location, XcmError},
	Weight,
};
use xcm_builder::TakeRevenue;
use xcm_executor::traits::WeightTrader;

/// Used as weight trader for foreign assets.
///
/// In case foreigin asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
	/// Total used weight
	weight: Weight,
	/// Total consumed assets
	consumed: u128,
	/// Asset Id (as Location) and units per second for payment
	asset_location_and_units_per_second: Option<(Location, u128)>,
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
		payment: xcm_executor::AssetsInHolding,
		_context: &XcmContext,
	) -> Result<xcm_executor::AssetsInHolding, XcmError> {
		log::trace!(
			target: "xcm::weight",
			"FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
			weight, payment,
		);

		// Atm in pallet, we only support one asset so this should work
		let payment_asset = payment.fungible_assets_iter().next().ok_or(XcmError::TooExpensive)?;

		match payment_asset {
			Asset { id: xcm::latest::AssetId(asset_location), fun: Fungibility::Fungible(_) } => {
				if let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) {
					let amount = units_per_second.saturating_mul(weight.ref_time() as u128) // TODO: change this to u64?
                        / (WEIGHT_REF_TIME_PER_SECOND as u128);
					if amount == 0 {
						return Ok(payment);
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

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<Asset> {
		log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

		if let Some((asset_location, units_per_second)) =
			self.asset_location_and_units_per_second.clone()
		{
			let weight = weight.min(self.weight);
			let amount = units_per_second.saturating_mul(weight.ref_time() as u128)
				/ (WEIGHT_REF_TIME_PER_SECOND as u128);

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

// [TODO] Comment it out because we won't use that
// pub struct FeeManagerNotWaived;
// impl FeeManager for FeeManagerNotWaived {
// 	fn is_waived(_: Option<&Location>, _: FeeReason) -> bool {
// 		false
// 	}
// 	fn handle_fee(_: Assets, _: Option<&XcmContext>, _: FeeReason) {}
// }
