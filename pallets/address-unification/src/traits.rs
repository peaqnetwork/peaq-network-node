use peaq_primitives_xcm::evm::EvmAddress;

/// A mapping between `AccountId` and `EvmAddress`.
pub trait EVMAddressMapping<AccountId> {
	/// Returns the AccountId used go generate the given EvmAddress.
	fn get_account_id_or_default(evm: &EvmAddress) -> AccountId;

	fn get_detault_account_id(evm: &EvmAddress) -> AccountId;

	fn get_evm_address_or_default(account_id: &AccountId) -> EvmAddress;

	fn get_detault_evm_address(account_id: &AccountId) -> EvmAddress;
	/// Returns true if a given AccountId is associated with a given EvmAddress
	/// and false if is not.
	fn is_linked(account_id: &AccountId, evm: &EvmAddress) -> bool;
}
