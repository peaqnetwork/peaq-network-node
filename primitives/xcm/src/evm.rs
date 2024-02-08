use crate::{AccountId, AssetId};
use frame_support::ensure;
use pallet_assets::AssetsCallback;
use sp_core::{H160, U256};
use sp_std::marker::PhantomData;

/// Evm Address.
pub type EvmAddress = sp_core::H160;

/// Convert any type that implements Into<U256> into byte representation ([u8, 32])
pub fn to_bytes<T: Into<U256>>(value: T) -> [u8; 32] {
	Into::<[u8; 32]>::into(value.into())
}

/// Revert opt code. It's inserted at the precompile addresses, to make them functional in EVM.
pub const EVM_REVERT_CODE: &[u8] = &[0x60, 0x00, 0x60, 0x00, 0xfd];

/// This trait ensure we can convert EVM address to AssetIds
/// We will require Runtime to have this trait implemented
pub trait EVMAddressToAssetId<AssetId> {
	// Get assetId from address
	fn address_to_asset_id(address: H160) -> Option<AssetId>;

	// Get address from AssetId
	fn asset_id_to_address(asset_id: AssetId) -> H160;
}

pub struct EvmRevertCodeHandler<A, R>(PhantomData<(A, R)>);
impl<A, R> AssetsCallback<AssetId, AccountId> for EvmRevertCodeHandler<A, R>
where
	A: EVMAddressToAssetId<AssetId>,
	R: pallet_evm::Config,
{
	fn created(id: &AssetId, _: &AccountId) -> Result<(), ()> {
		let address = A::asset_id_to_address(*id);
		ensure!(!pallet_evm::AccountCodes::<R>::contains_key(address), ());
		pallet_evm::AccountCodes::<R>::insert(address, EVM_REVERT_CODE.to_vec());
		Ok(())
	}

	fn destroyed(id: &AssetId) -> Result<(), ()> {
		let address = A::asset_id_to_address(*id);
		pallet_evm::AccountCodes::<R>::remove(address);
		Ok(())
	}
}
