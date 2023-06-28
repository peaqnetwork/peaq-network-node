#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

use sp_runtime::Perbill;
use sp_std::marker::PhantomData;

use frame_support::{parameter_types, traits::Get};
use orml_traits::currency::MutationHooks;

/// Balance of an account.
pub type Balance = peaq_primitives_xcm::Balance;

// Contracts price units.
pub const TOKEN_DECIMALS: u32 = 18;
pub const NANOCENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2 - 9);
pub const MILLICENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2 - 3);
pub const CENTS: Balance = 10_u128.pow(TOKEN_DECIMALS - 2);
pub const DOLLARS: Balance = 10_u128.pow(TOKEN_DECIMALS);

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const EoTFeeFactor: Perbill = Perbill::from_percent(50);
}

pub struct CurrencyHooks<T, DustAccount>(PhantomData<T>, DustAccount);
impl<T, DustAccount> MutationHooks<T::AccountId, T::CurrencyId, T::Balance>
	for CurrencyHooks<T, DustAccount>
where
	T: orml_tokens::Config,
	DustAccount: Get<<T as frame_system::Config>::AccountId>,
{
	type OnDust = orml_tokens::TransferDust<T, DustAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}
