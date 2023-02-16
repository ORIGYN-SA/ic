use anyhow::{bail, Result};
use std::{env, path::PathBuf, process::Command};

fn execute_test_driver_with_scenario(scenario_name: &str) -> Result<()> {
    let binary_path = env::current_dir()?.join("rs/tests/test-driver-e2e-scenarios");
    let working_dir = PathBuf::from(env::var("TEST_TMPDIR")?).join(scenario_name);
    let status = Command::new(binary_path)
        .env("TEST_SCENARIO_NAME", scenario_name)
        .args(["--working-dir", working_dir.to_str().unwrap(), "run"])
        .status()
        .expect("failed to execute process");
    if status.success() {
        Ok(())
    } else {
        bail!("execution of test scenario failed.")
    }
}

// these tests don't work in the cargo case

#[test]
fn test_scenario_without_errors_succeeds() {
    let result = execute_test_driver_with_scenario("test_without_errors");
    assert!(result.is_ok())
}

#[test]
fn test_scenario_with_test_panic_fails() {
    let result = execute_test_driver_with_scenario("test_with_panic");
    assert!(result.is_err())
}

#[test]
fn test_scenario_with_setup_panic_fails() {
    let result = execute_test_driver_with_scenario("test_with_setup_panic");
    assert!(result.is_err())
}

#[test]
fn test_that_runs_out_of_time() {
    let result = execute_test_driver_with_scenario("test_that_runs_out_of_time");
    assert!(result.is_err())
}

#[test]
fn test_duplicate_tasks() {
    let result = execute_test_driver_with_scenario("test_duplicate_tasks");
    assert!(result.is_err())
}

#[test]
fn test_that_runs_1_parallel_task() {
    let result = execute_test_driver_with_scenario("test_that_runs_1_parallel_task");
    assert!(result.is_ok())
}

#[test]
fn test_that_runs_2_parallel_tasks() {
    let result = execute_test_driver_with_scenario("test_that_runs_2_parallel_tasks");
    assert!(result.is_ok())
}

#[test]
fn test_that_runs_3_parallel_tasks() {
    let result = execute_test_driver_with_scenario("test_that_runs_3_parallel_tasks");
    assert!(result.is_ok())
}

#[test]
fn test_that_runs_3_parallel_tasks_one_of_which_fails() {
    let result =
        execute_test_driver_with_scenario("test_that_runs_3_parallel_tasks_one_of_which_fails");
    assert!(result.is_err())
}

#[test]
fn test_that_runs_2_parallel_tasks_then_one_task_then_2_parallel_tasks() {
    let result = execute_test_driver_with_scenario(
        "test_that_runs_2_parallel_tasks_then_one_task_then_2_parallel_tasks",
    );
    assert!(result.is_ok())
}

#[test]
fn test_that_runs_2_parallel_tasks_then_one_failing_task_then_2_parallel_tasks() {
    let result = execute_test_driver_with_scenario(
        "test_that_runs_2_parallel_tasks_then_one_failing_task_then_2_parallel_tasks",
    );
    assert!(result.is_err())
}

#[test]
fn test_overall_group_timeout() {
    let result = execute_test_driver_with_scenario("test_overall_group_timeout");
    assert!(result.is_err())
}
