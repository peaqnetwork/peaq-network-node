//! peaq_runtime's XCM executor records locally executed XCM once recording is switched on, which is
//! what `DryRunApi::dry_run_call` reads back as `local_xcm`.

use peaq_runtime::{xcm_config::XcmConfig, PolkadotXcm, RuntimeCall};
use xcm::latest::prelude::*;
use xcm_executor::{traits::RecordXcm, XcmExecutor};

// The outcome is not checked: the executor records the message before the barrier runs, so a
// barrier rejection does not matter here.
fn execute_clear_origin() -> Xcm<()> {
	let message = Xcm::<RuntimeCall>(vec![ClearOrigin]);
	let mut hash = [0u8; 32];
	let _ = XcmExecutor::<XcmConfig>::prepare_and_execute(
		Here,
		message,
		&mut hash,
		Weight::MAX,
		Weight::zero(),
	);
	Xcm(vec![ClearOrigin])
}

#[test]
fn records_local_xcm_when_enabled() {
	sp_io::TestExternalities::default().execute_with(|| {
		PolkadotXcm::set_record_xcm(true);
		let executed = execute_clear_origin();
		assert_eq!(PolkadotXcm::recorded_xcm(), Some(executed));
	});
}

#[test]
fn records_nothing_when_disabled() {
	sp_io::TestExternalities::default().execute_with(|| {
		execute_clear_origin();
		assert_eq!(PolkadotXcm::recorded_xcm(), None);
	});
}
