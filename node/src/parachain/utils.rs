pub use peaq_primitives_xcm::AccountId;
use peaq_primitives_xcm::Signature;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_core::sr25519::Public as PublicType;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// The default XCM version to set in genesis config.
pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

type AccountPublic = <Signature as Verify>::Signer;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId) {
	(get_account_id_from_seed::<PublicType>(s), get_from_seed::<AuraId>(s))
}
