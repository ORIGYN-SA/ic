#[rustfmt::skip]

use anyhow::Result;

use ic_tests::driver::new::group::SystemTestGroup;
use ic_tests::driver::test_env::TestEnv;
use ic_tests::systest;
use slog::info;

pub fn config(env: TestEnv) {
    info!(env.logger(), ">>> config");
}

pub fn test_a(env: TestEnv) {
    info!(env.logger(), ">>> test_a");
}

pub fn test_b(env: TestEnv) {
    info!(env.logger(), ">>> test_b");
}

fn main() -> Result<()> {
    SystemTestGroup::new()
        .with_setup(config)
        .add_test(systest!(test_a))
        .add_test(systest!(test_b))
        .execute_from_args()?;

    Ok(())
}
