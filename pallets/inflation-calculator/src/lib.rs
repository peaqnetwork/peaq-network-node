#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{IsType, OnKilledAccount},
	transactional,
};
use frame_system::{ensure_signed, pallet_prelude::*, WeightInfo, Pallet, Config};
use parity_scale_codec::Encode;

use sp_core::{H160, H256};
use sp_io::{crypto::secp256k1_ecdsa_recover, hashing::keccak_256};
use sp_runtime::{
	traits::{LookupError, StaticLookup, Zero},
	MultiAddress,
};
use sp_std::{marker::PhantomData, vec::Vec};


#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		// TODO
	}

	/// Error for evm accounts module.
	#[pallet::error]
	pub enum Error<T> {
		// TODO
	}


	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
	}
}