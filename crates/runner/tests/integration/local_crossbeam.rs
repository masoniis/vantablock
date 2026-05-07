use crate::common::CrossbeamClientServerTestEnvironment;
use std::time::Duration;

#[test]
#[ignore]
fn empty() {
    let mut env = CrossbeamClientServerTestEnvironment::default();

    // empty test for now

    env.wait_until(|_e| true, Duration::from_secs(5));

    env.step();
}
