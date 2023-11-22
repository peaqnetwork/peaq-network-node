use crate::{Accounts, Config, EvmAddresses, H160};
use pallet_evm::AddressMapping as PalletEVMAddressMapping;
use parity_scale_codec::Encode;
use peaq_primitives_xcm::EvmAddress;
use sp_core::{crypto::AccountId32, Hasher, H256};
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;

pub trait UnifyAddressMapping<AccountId> {
	fn to_set_account_id(evm: &EvmAddress) -> Option<AccountId>;
	fn to_default_account_id(evm_address: &EvmAddress) -> AccountId;

	fn to_set_evm_address(account_id: &AccountId) -> Option<EvmAddress>;
	fn to_default_evm_address(account_id: &AccountId) -> EvmAddress;
}

pub struct UnifyAddressMapper<T, H>(PhantomData<(T, H)>);

impl<T, H> UnifyAddressMapping<T::AccountId> for UnifyAddressMapper<T, H>
where
	T: Config,
	T::OriginAddressMapping: PalletEVMAddressMapping<T::AccountId>,
	H: Hasher<Out = H256>,
{
	/// Returns the AccountId used go generate the given EvmAddress.
	fn to_set_account_id(evm_address: &EvmAddress) -> Option<T::AccountId> {
		Accounts::<T>::get(evm_address)
	}

	fn to_default_account_id(evm_address: &EvmAddress) -> T::AccountId {
		T::OriginAddressMapping::into_account_id(*evm_address)
	}

	fn to_set_evm_address(account_id: &T::AccountId) -> Option<EvmAddress> {
		EvmAddresses::<T>::get(account_id)
	}

	fn to_default_evm_address(account_id: &T::AccountId) -> EvmAddress {
		let payload = (b"evm:", account_id);
		H160::from_slice(&payload.using_encoded(H::hash)[0..20])
	}
}
