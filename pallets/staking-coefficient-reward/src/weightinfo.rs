use frame_support::weights::Weight;

pub trait WeightInfo {
	fn set_coefficient() -> Weight;
}
