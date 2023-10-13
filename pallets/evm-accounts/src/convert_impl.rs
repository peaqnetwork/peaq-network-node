use sp_std::{marker::PhantomData};
use sp_runtime::traits::Convert;
use peaq_primitives_xcm::EvmAddress;
use crate::Config;
use frame_support::pallet_prelude::IsType;
use sp_core::crypto::AccountId32;
use pallet_evm::{
	AddressMapping as PalletEVMAddressMapping,
};
use crate::Accounts;
use crate::EvmAddresses;

// For avoid the convert fail, we have to setup the AccountId32 direct
pub struct EVMAddressToAccountId<T>(PhantomData<T>);

impl<T: Config> Convert<EvmAddress, T::AccountId>
	for EVMAddressToAccountId<T>
where
	T::AccountId: IsType<AccountId32>,
	T::OriginAddressMapping: PalletEVMAddressMapping<T::AccountId>,
{
	fn convert(address: EvmAddress) -> T::AccountId {
		if let Some(acc) = Accounts::<T>::get(address) {
			acc
		} else {
			T::OriginAddressMapping::into_account_id(address)
		}
	}
}

pub struct AccountIdToEVMAddress<T>(PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<EvmAddress>>
	for AccountIdToEVMAddress<T>
where
	T::AccountId: IsType<AccountId32>,
	T::OriginAddressMapping: PalletEVMAddressMapping<T::AccountId>,
{
	fn convert(account_id: T::AccountId) -> Option<EvmAddress> {
		// Return the EvmAddress if a mapping to account_id exists
		EvmAddresses::<T>::get(account_id)
	}
}
