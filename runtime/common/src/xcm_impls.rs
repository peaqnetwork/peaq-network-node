use cumulus_primitives_core::XcmContext;
use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;
use sp_std::marker::PhantomData;
use xc_asset_config::ExecutionPaymentRate;
use xcm::latest::{
	prelude::{Asset, AssetId, Fungibility, Location, XcmError},
	Weight,
};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, AssetsInHolding};

/// Used as weight trader for foreign assets.
///
/// In case foreigin asset is supported as payment asset, XCM execution time
/// on-chain can be paid by the foreign asset, using the configured rate.
pub struct FixedRateOfForeignAsset<T: ExecutionPaymentRate, R: TakeRevenue> {
	/// Total used weight
	weight: Weight,
	/// Assets taken as payment. The holding register carries real imbalances, so the trader has
	/// to hold on to them until it is dropped and can hand them over to `R`.
	consumed: AssetsInHolding,
	/// Asset Id (as Location) and units per second for payment
	asset_location_and_units_per_second: Option<(Location, u128)>,
	_pd: PhantomData<(T, R)>,
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> WeightTrader for FixedRateOfForeignAsset<T, R> {
	fn new() -> Self {
		Self {
			weight: Weight::zero(),
			consumed: AssetsInHolding::new(),
			asset_location_and_units_per_second: None,
			_pd: PhantomData,
		}
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		mut payment: AssetsInHolding,
		_context: &XcmContext,
	) -> Result<AssetsInHolding, (AssetsInHolding, XcmError)> {
		log::trace!(
			target: "xcm::weight",
			"FixedRateOfForeignAsset::buy_weight weight: {:?}, payment: {:?}",
			weight, payment,
		);

		// Atm in pallet, we only support one asset so this should work
		let Some(Asset { id: AssetId(asset_location), fun: Fungibility::Fungible(_) }) =
			payment.fungible_assets_iter().next()
		else {
			return Err((payment, XcmError::TooExpensive));
		};

		let Some(units_per_second) = T::get_units_per_second(asset_location.clone()) else {
			return Err((payment, XcmError::TooExpensive));
		};

		let amount = units_per_second.saturating_mul(weight.ref_time() as u128) // TODO: change this to u64?
            / (WEIGHT_REF_TIME_PER_SECOND as u128);
		if amount == 0 {
			return Ok(payment);
		}

		let to_charge: Asset = (asset_location.clone(), amount).into();
		let Ok(taken) = payment.try_take(to_charge.into()) else {
			return Err((payment, XcmError::TooExpensive));
		};

		self.weight = self.weight.saturating_add(weight);
		// Every taken imbalance has to be kept, otherwise dropping it here would silently revert
		// the withdrawal. Refunds are still priced off the FIRST asset only, which matches the
		// behaviour of the previous implementation.
		self.consumed.subsume_assets(taken);
		if self.asset_location_and_units_per_second.is_none() {
			self.asset_location_and_units_per_second = Some((asset_location, units_per_second));
		}

		Ok(payment)
	}

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<AssetsInHolding> {
		log::trace!(target: "xcm::weight", "FixedRateOfForeignAsset::refund_weight weight: {:?}", weight);

		let (asset_location, units_per_second) = self.asset_location_and_units_per_second.clone()?;

		let weight = weight.min(self.weight);
		let amount = units_per_second.saturating_mul(weight.ref_time() as u128) /
			(WEIGHT_REF_TIME_PER_SECOND as u128);
		if amount == 0 {
			return None;
		}

		self.weight = self.weight.saturating_sub(weight);

		let refund: Asset = (asset_location, amount).into();
		let refunded = self.consumed.saturating_take(refund.into());
		if refunded.is_empty() {
			None
		} else {
			Some(refunded)
		}
	}
}

impl<T: ExecutionPaymentRate, R: TakeRevenue> Drop for FixedRateOfForeignAsset<T, R> {
	fn drop(&mut self) {
		if !self.consumed.is_empty() {
			let mut taken = AssetsInHolding::new();
			core::mem::swap(&mut self.consumed, &mut taken);
			R::take_revenue(taken);
		}
	}
}
