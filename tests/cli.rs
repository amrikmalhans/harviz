use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

fn fixture_path(name: &str) -> String {
    Path::new("tests")
        .join("fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn default_output_has_expected_sections() {
    let fixture = fixture_path("sample.har");

    let mut cmd = Command::cargo_bin("perf_tool").expect("binary should build");
    cmd.arg(&fixture)
        .assert()
        .success()
        .stdout(predicate::str::contains("entries: 4"))
        .stdout(predicate::str::contains("total_time_ms: 565.75"))
        .stdout(predicate::str::contains("total_bytes: 3.29 KB"))
        .stdout(predicate::str::contains("slowest 4:"))
        .stdout(predicate::str::contains("largest 4 by bytes:"));
}

#[test]
fn json_output_is_valid_and_structured() {
    let fixture = fixture_path("sample.har");

    let output = Command::cargo_bin("perf_tool")
        .expect("binary should build")
        .arg("--json")
        .arg("--top")
        .arg("2")
        .arg(&fixture)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).expect("must be valid JSON");

    assert_eq!(report["entries"], 4);
    assert_eq!(report["top_requested"], 2);
    assert_eq!(report["top_returned"], 2);
    assert!(report["group_by"].is_null());
    assert!(report["top_slowest"].is_array());
    assert!(report["top_largest"].is_array());
    assert!(report["top_groups"].is_array());
    assert_eq!(report["top_slowest"].as_array().expect("array").len(), 2);
    assert_eq!(report["top_largest"].as_array().expect("array").len(), 2);
    assert_eq!(report["top_groups"].as_array().expect("array").len(), 0);
}

#[test]
fn top_limits_both_lists() {
    let fixture = fixture_path("sample.har");

    let output = Command::cargo_bin("perf_tool")
        .expect("binary should build")
        .arg("--json")
        .arg("--top")
        .arg("1")
        .arg(&fixture)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).expect("must be valid JSON");
    assert_eq!(report["top_slowest"].as_array().expect("array").len(), 1);
    assert_eq!(report["top_largest"].as_array().expect("array").len(), 1);
}

#[test]
fn missing_input_path_fails() {
    let mut cmd = Command::cargo_bin("perf_tool").expect("binary should build");
    cmd.arg("tests/fixtures/does-not-exist.har")
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read file"));
}

#[test]
fn help_output_exposes_core_usage_and_options() {
    let mut cmd = Command::cargo_bin("perf_tool").expect("binary should build");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Analyze HAR files"))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("--top"))
        .stdout(predicate::str::contains("--json"))
        .stdout(predicate::str::contains("--group-by"))
        .stdout(predicate::str::contains("--help"));
}

#[test]
fn json_output_with_group_by_host_has_group_metrics() {
    let fixture = fixture_path("grouped.har");

    let output = Command::cargo_bin("perf_tool")
        .expect("binary should build")
        .arg("--json")
        .arg("--top")
        .arg("2")
        .arg("--group-by")
        .arg("host")
        .arg(&fixture)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let report: serde_json::Value = serde_json::from_slice(&output).expect("must be valid JSON");
    assert_eq!(report["group_by"], "host");
    assert_eq!(report["top_groups"].as_array().expect("array").len(), 2);
    assert_eq!(report["top_groups"][0]["key"], "cdn.example.com");
    assert_eq!(report["top_groups"][0]["count"], 2);
    assert_eq!(report["top_groups"][0]["total_time_ms"], 350.0);
    assert_eq!(report["top_groups"][0]["avg_time_ms"], 175.0);
    assert_eq!(report["top_groups"][0]["p95_time_ms"], 300.0);
    assert_eq!(report["top_groups"][0]["total_bytes"], 390);
}

#[test]
fn text_output_with_group_by_host_has_group_section() {
    let fixture = fixture_path("grouped.har");

    let mut cmd = Command::cargo_bin("perf_tool").expect("binary should build");
    cmd.arg("--top")
        .arg("2")
        .arg("--group-by")
        .arg("host")
        .arg(&fixture)
        .assert()
        .success()
        .stdout(predicate::str::contains("groups by host (top 2):"))
        .stdout(predicate::str::contains("cdn.example.com"))
        .stdout(predicate::str::contains("api.example.com"));
}
