use cumulus_primitives_core::XcmContext;
use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;
use sp_runtime::traits::{Bounded, MaybeEquivalence};
use sp_std::marker::PhantomData;
use xc_asset_config::{ExecutionPaymentRate, XcAssetLocation};
use xcm::latest::{
	prelude::{Asset, AssetId, Fungibility, Location, XcmError},
	Weight,
};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, AssetsInHolding};

/// Used to convert between cross-chain asset multilocation and local asset Id.
///
/// This implementation relies on `XcAssetConfig` pallet to handle mapping.
/// In case asset location hasn't been mapped, it means the asset isn't supported (yet).
pub struct AssetLocationIdConverter<AssetId, AssetMapper>(PhantomData<(AssetId, AssetMapper)>);
impl<AssetId, AssetMapper> MaybeEquivalence<Location, AssetId>
	for AssetLocationIdConverter<AssetId, AssetMapper>
where
	AssetId: Clone + Eq + Bounded,
	AssetMapper: XcAssetLocation<AssetId>,
{
	fn convert(location: &Location) -> Option<AssetId> {
		AssetMapper::get_asset_id(location.clone())
	}

	fn convert_back(id: &AssetId) -> Option<Location> {
		AssetMapper::get_xc_asset_location(id.clone())
	}
}

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

	/// Price `weight` in `given` without touching the holding register.
	///
	/// Callers that only want to know the fee (the `xcm-utils` precompile) used to synthesise a
	/// `u128::MAX` payment and read what `buy_weight` left over. That charges the trader for real,
	/// so dropping it hands the "fee" to `TakeRevenue`. This is the read-only path for that.
	fn quote_weight(
		&mut self,
		weight: Weight,
		given: AssetId,
		_context: &XcmContext,
	) -> Result<Asset, XcmError> {
		let AssetId(asset_location) = given;
		let units_per_second =
			T::get_units_per_second(asset_location.clone()).ok_or(XcmError::TooExpensive)?;
		let amount = units_per_second.saturating_mul(weight.ref_time() as u128) /
			(WEIGHT_REF_TIME_PER_SECOND as u128);
		Ok((asset_location, amount).into())
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
