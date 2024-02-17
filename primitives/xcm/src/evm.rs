use crate::AccountId;
use frame_support::ensure;
use pallet_assets::AssetsCallback;
use pallet_evm_precompile_assets_erc20::EVMAddressToAssetId;
use sp_core::U256;
use sp_std::marker::PhantomData;

/// Evm Address.
pub type EvmAddress = sp_core::H160;

/// Convert any type that implements Into<U256> into byte representation ([u8, 32])
pub fn to_bytes<T: Into<U256>>(value: T) -> [u8; 32] {
	Into::<[u8; 32]>::into(value.into())
}

/// Revert opt code. It's inserted at the precompile addresses, to make them functional in EVM.
pub const EVM_REVERT_CODE: &[u8] = &[0x60, 0x00, 0x60, 0x00, 0xfd];

pub struct EvmRevertCodeHandler<A, R>(PhantomData<(A, R)>);
impl<A, R> AssetsCallback<R::AssetId, AccountId> for EvmRevertCodeHandler<A, R>
where
	A: EVMAddressToAssetId<R::AssetId>,
	R: pallet_evm::Config + pallet_assets::Config,
{
	fn created(id: &R::AssetId, _: &AccountId) -> Result<(), ()> {
		match A::asset_id_to_address((*id).clone()) {
			None => return Ok(()),
			Some(address) => {
				ensure!(!pallet_evm::AccountCodes::<R>::contains_key(address), ());
				pallet_evm::AccountCodes::<R>::insert(address, EVM_REVERT_CODE.to_vec());
				Ok(())
			},
		}
	}

	fn destroyed(id: &R::AssetId) -> Result<(), ()> {
		match A::asset_id_to_address((*id).clone()) {
			None => return Ok(()),
			Some(address) => {
				pallet_evm::AccountCodes::<R>::remove(address);
				Ok(())
			},
		}
	}
}
