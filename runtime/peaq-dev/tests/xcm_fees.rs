//! XCM execution fees on peaq-dev's `XcmConfig`. The behaviour these tests lock in: the trader
//! moves the fee from the payer to the fee destination without minting or burning it, and a second
//! `BuyExecution` in another asset is paid for and handed to `TakeRevenue` rather than dropped.

use frame_support::traits::{fungible, fungibles};
use peaq_dev_runtime::{
	xcm_config::XcmConfig, AccountId, Assets, Balances, BlockRewardMigrationSinks, PeaqPotAccount,
	Runtime, RuntimeCall, RuntimeOrigin, XcAssetConfig,
};
use peaq_primitives_xcm::Balance;
use xcm::{latest::prelude::*, VersionedLocation};
use xcm_executor::XcmExecutor;

const PAYER: [u8; 32] = [1; 32];
const BENEFICIARY: [u8; 32] = [2; 32];
const FOREIGN_ASSET_ID: u64 = 1;
const FOREIGN_UNITS_PER_SECOND: u128 = 1_000_000_000_000;
const NATIVE_WITHDRAW: Balance = 1_000_000_000_000_000_000;
const FOREIGN_WITHDRAW: Balance = 1_000_000_000_000;

fn account(id: [u8; 32]) -> AccountId {
	id.into()
}

fn local(id: [u8; 32]) -> Location {
	AccountId32 { network: None, id }.into()
}

// The relay token stands in for the second, non-native fee asset.
fn foreign_location() -> Location {
	Location::parent()
}

fn native_balance(who: &AccountId) -> Balance {
	<Balances as fungible::Inspect<AccountId>>::balance(who)
}

fn native_issuance() -> Balance {
	<Balances as fungible::Inspect<AccountId>>::total_issuance()
}

fn foreign_balance(who: &AccountId) -> Balance {
	<Assets as fungibles::Inspect<AccountId>>::balance(FOREIGN_ASSET_ID, who)
}

// What the block-reward pallet's sinks hold in total; native fees from `UsingComponents` go there.
fn block_reward_pots() -> Balance {
	BlockRewardMigrationSinks::get()
		.iter()
		.map(|sink| native_balance(&pallet_block_reward::Pallet::<Runtime>::resolve(&sink.target)))
		.sum()
}

fn new_ext() -> sp_io::TestExternalities {
	let mut ext = sp_io::TestExternalities::default();
	ext.execute_with(|| {
		frame_system::Pallet::<Runtime>::set_block_number(1);
		pallet_block_reward::Sinks::<Runtime>::put(
			frame_support::BoundedVec::try_from(BlockRewardMigrationSinks::get()).unwrap(),
		);

		let payer = account(PAYER);
		<Balances as fungible::Mutate<AccountId>>::mint_into(&payer, 10 * NATIVE_WITHDRAW).unwrap();

		<Assets as fungibles::Create<AccountId>>::create(FOREIGN_ASSET_ID, payer.clone(), true, 1)
			.unwrap();
		<Assets as fungibles::Mutate<AccountId>>::mint_into(
			FOREIGN_ASSET_ID,
			&payer,
			10 * FOREIGN_WITHDRAW,
		)
		.unwrap();
		XcAssetConfig::register_asset_location(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::V5(foreign_location())),
			FOREIGN_ASSET_ID,
		)
		.unwrap();
		XcAssetConfig::set_asset_units_per_second(
			RuntimeOrigin::root(),
			Box::new(VersionedLocation::V5(foreign_location())),
			FOREIGN_UNITS_PER_SECOND,
		)
		.unwrap();
	});
	ext
}

fn execute(message: Vec<Instruction<RuntimeCall>>) -> Outcome {
	let mut hash = [0u8; 32];
	XcmExecutor::<XcmConfig>::prepare_and_execute(
		local(PAYER),
		Xcm(message),
		&mut hash,
		Weight::MAX,
		Weight::zero(),
	)
}

/// Pays a native fee through one `BuyExecution` and returns the fee. Asserts that the message
/// completes and that the payer is debited exactly the withdrawn amount.
fn pay_native_fee() -> Balance {
	let (payer, beneficiary) = (account(PAYER), account(BENEFICIARY));
	let payer_before = native_balance(&payer);

	let outcome = execute(vec![
		WithdrawAsset((Here, NATIVE_WITHDRAW).into()),
		BuyExecution { fees: (Here, NATIVE_WITHDRAW).into(), weight_limit: Unlimited },
		DepositAsset { assets: Wild(AllCounted(1)), beneficiary: local(BENEFICIARY) },
	]);

	assert!(matches!(outcome, Outcome::Complete { .. }), "{outcome:?}");
	assert_eq!(payer_before - native_balance(&payer), NATIVE_WITHDRAW);
	let fee = NATIVE_WITHDRAW - native_balance(&beneficiary);
	assert!(fee > 0);
	fee
}

#[test]
fn native_fee_goes_to_block_reward_pots() {
	new_ext().execute_with(|| {
		let pots_before = block_reward_pots();
		let fee = pay_native_fee();
		assert_eq!(block_reward_pots() - pots_before, fee);
	});
}

#[test]
fn native_fee_does_not_change_issuance() {
	new_ext().execute_with(|| {
		let issuance_before = native_issuance();
		pay_native_fee();
		assert_eq!(native_issuance(), issuance_before);
	});
}

#[test]
fn second_buy_execution_in_another_asset_is_paid_to_take_revenue() {
	new_ext().execute_with(|| {
		let (payer, beneficiary, pot) =
			(account(PAYER), account(BENEFICIARY), PeaqPotAccount::get());
		let payer_before = foreign_balance(&payer);
		let pot_before = foreign_balance(&pot);
		let supply_before =
			<Assets as fungibles::Inspect<AccountId>>::total_issuance(FOREIGN_ASSET_ID);

		let outcome = execute(vec![
			WithdrawAsset(
				vec![(Here, NATIVE_WITHDRAW).into(), (foreign_location(), FOREIGN_WITHDRAW).into()]
					.into(),
			),
			BuyExecution { fees: (Here, NATIVE_WITHDRAW).into(), weight_limit: Unlimited },
			BuyExecution {
				fees: (foreign_location(), FOREIGN_WITHDRAW).into(),
				weight_limit: Limited(Weight::from_parts(1_000_000_000, 1024)),
			},
			DepositAsset { assets: Wild(AllCounted(2)), beneficiary: local(BENEFICIARY) },
		]);

		assert!(matches!(outcome, Outcome::Complete { .. }), "{outcome:?}");
		assert_eq!(payer_before - foreign_balance(&payer), FOREIGN_WITHDRAW);
		let fee = FOREIGN_WITHDRAW - foreign_balance(&beneficiary);
		assert!(fee > 0);
		assert_eq!(foreign_balance(&pot) - pot_before, fee);
		assert_eq!(
			<Assets as fungibles::Inspect<AccountId>>::total_issuance(FOREIGN_ASSET_ID),
			supply_before
		);
	});
}
