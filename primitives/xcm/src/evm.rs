use sp_core::U256;

/// Evm Address.
pub type EvmAddress = sp_core::H160;

/// Convert any type that implements Into<U256> into byte representation ([u8, 32])
pub fn to_bytes<T: Into<U256>>(value: T) -> [u8; 32] {
	Into::<[u8; 32]>::into(value.into())
}
