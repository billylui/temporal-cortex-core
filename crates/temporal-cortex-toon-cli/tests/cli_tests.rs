//! Integration tests for the `toon` CLI binary.
//!
//! These tests use `assert_cmd` and `predicates` to exercise the encode, decode,
//! and stats subcommands through the actual binary, including stdin/stdout piping,
//! file I/O, error handling, and roundtrip correctness.

// `Command::cargo_bin` was deprecated in assert_cmd 2.1.2 in favor of
// `cargo::cargo_bin_cmd!`. Allow it until we migrate.
#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

/// Helper: path to the sample.json fixture.
fn sample_json_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.json")
}

/// Helper: path to the calendar.json fixture.
fn calendar_json_path() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/calendar.json")
}

/// Helper: read the sample.json fixture as a string.
fn sample_json() -> String {
    std::fs::read_to_string(sample_json_path()).expect("sample.json fixture must exist")
}

/// Helper: read the calendar.json fixture as a string.
fn calendar_json() -> String {
    std::fs::read_to_string(calendar_json_path()).expect("calendar.json fixture must exist")
}

// ─────────────────────────────────────────────────────────────────────────────
// Encode subcommand
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn encode_stdin_to_stdout() {
    // Test 1: pipe JSON via stdin, get TOON on stdout
    let input = r#"{"name":"Alice","age":30}"#;

    Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("name:"))
        .stdout(predicate::str::contains("age:"));
}

#[test]
fn encode_file_to_stdout() {
    // Test 2: read from file via -i, output to stdout
    Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "-i", sample_json_path()])
        .assert()
        .success()
        .stdout(predicate::str::contains("name:"))
        .stdout(predicate::str::contains("scores"));
}

#[test]
fn encode_file_to_file() {
    // Test 3: read from file via -i, write to file via -o
    let output_path = "/tmp/toon-test-encode-output.toon";

    // Clean up from any prior run
    let _ = std::fs::remove_file(output_path);

    Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "-i", sample_json_path(), "-o", output_path])
        .assert()
        .success();

    // Verify the output file was created and contains TOON content
    let content = std::fs::read_to_string(output_path).expect("output file must exist");
    assert!(
        content.contains("name:"),
        "TOON output should contain 'name:'"
    );
    assert!(!content.is_empty(), "Output file should not be empty");

    // Clean up
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn encode_invalid_json_fails() {
    // Test 4: invalid JSON input should produce non-zero exit
    Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin("this is not valid json {{{")
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Failed to encode")
                .or(predicate::str::contains("error").or(predicate::str::contains("Error"))),
        );
}

// ─────────────────────────────────────────────────────────────────────────────
// Decode subcommand
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn decode_stdin_to_stdout() {
    // Test 5: pipe TOON via stdin, get JSON on stdout
    // First, encode some JSON to get valid TOON
    let input_json = r#"{"name":"Alice","age":30}"#;
    let encode_output = Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin(input_json)
        .output()
        .expect("encode should succeed");

    let toon = String::from_utf8(encode_output.stdout).expect("TOON should be valid UTF-8");

    // Now decode the TOON back to JSON
    Command::cargo_bin("toon")
        .unwrap()
        .arg("decode")
        .write_stdin(toon)
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("30"));
}

#[test]
fn decode_file_to_file() {
    // Test 6: file I/O for decode (-i and -o flags)
    let toon_path = "/tmp/toon-test-decode-input.toon";
    let json_path = "/tmp/toon-test-decode-output.json";

    // Clean up from any prior run
    let _ = std::fs::remove_file(toon_path);
    let _ = std::fs::remove_file(json_path);

    // First create a TOON file by encoding
    Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "-i", sample_json_path(), "-o", toon_path])
        .assert()
        .success();

    // Now decode from the TOON file to a JSON file
    Command::cargo_bin("toon")
        .unwrap()
        .args(["decode", "-i", toon_path, "-o", json_path])
        .assert()
        .success();

    // Verify the output JSON file was created and contains expected content
    let content = std::fs::read_to_string(json_path).expect("output JSON file must exist");
    assert!(
        content.contains("Alice"),
        "Decoded JSON should contain 'Alice'"
    );
    assert!(
        content.contains("Portland"),
        "Decoded JSON should contain 'Portland'"
    );

    // Clean up
    let _ = std::fs::remove_file(toon_path);
    let _ = std::fs::remove_file(json_path);
}

#[test]
fn decode_invalid_toon_fails() {
    // Test 7: invalid TOON input should produce an error
    // Multiline input with an unterminated quoted key triggers ToonParse error
    // because parse_object_from_lines calls parse_key_from_content which errors
    Command::cargo_bin("toon")
        .unwrap()
        .arg("decode")
        .write_stdin("\"unterminated: value\nother: line")
        .assert()
        .failure();
}

// ─────────────────────────────────────────────────────────────────────────────
// Stats subcommand
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn stats_from_file() {
    // Test 8: stats from a file shows sizes and reduction
    Command::cargo_bin("toon")
        .unwrap()
        .args(["stats", "-i", sample_json_path()])
        .assert()
        .success()
        .stdout(predicate::str::contains("bytes"))
        .stdout(predicate::str::contains("%"));
}

#[test]
fn stats_output_format() {
    // Test 9: stats output contains the expected labels
    Command::cargo_bin("toon")
        .unwrap()
        .args(["stats", "-i", sample_json_path()])
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON size:"))
        .stdout(predicate::str::contains("TOON size:"))
        .stdout(predicate::str::contains("Reduction:"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Roundtrip
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn roundtrip_encode_decode_pipeline() {
    // Test 10: encode then decode produces JSON equivalent to input
    let input_json = sample_json();

    // Encode
    let encode_output = Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin(input_json.clone())
        .output()
        .expect("encode should succeed");
    assert!(encode_output.status.success(), "encode must succeed");
    let toon = String::from_utf8(encode_output.stdout).expect("TOON should be valid UTF-8");

    // Decode
    let decode_output = Command::cargo_bin("toon")
        .unwrap()
        .arg("decode")
        .write_stdin(toon)
        .output()
        .expect("decode should succeed");
    assert!(decode_output.status.success(), "decode must succeed");
    let result_json = String::from_utf8(decode_output.stdout).expect("JSON should be valid UTF-8");

    // Parse both and compare as serde_json::Value for structural equality
    let original: serde_json::Value =
        serde_json::from_str(&input_json).expect("input is valid JSON");
    let roundtripped: serde_json::Value =
        serde_json::from_str(&result_json).expect("roundtrip result is valid JSON");

    assert_eq!(
        original, roundtripped,
        "Roundtrip should preserve JSON semantics"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn encode_empty_object() {
    // Test 11: empty JSON object encodes without error
    Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin("{}")
        .assert()
        .success();
}

#[test]
fn large_input_roundtrip() {
    // Test 12: calendar fixture roundtrip works
    let input_json = calendar_json();

    // Encode
    let encode_output = Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin(input_json.clone())
        .output()
        .expect("encode should succeed");
    assert!(
        encode_output.status.success(),
        "encode of large input must succeed: {}",
        String::from_utf8_lossy(&encode_output.stderr)
    );
    let toon = String::from_utf8(encode_output.stdout).expect("TOON should be valid UTF-8");
    assert!(
        !toon.is_empty(),
        "TOON output should not be empty for calendar fixture"
    );

    // Decode
    let decode_output = Command::cargo_bin("toon")
        .unwrap()
        .arg("decode")
        .write_stdin(toon)
        .output()
        .expect("decode should succeed");
    assert!(
        decode_output.status.success(),
        "decode of large input must succeed: {}",
        String::from_utf8_lossy(&decode_output.stderr)
    );
    let result_json = String::from_utf8(decode_output.stdout).expect("JSON should be valid UTF-8");

    // Structural equality
    let original: serde_json::Value =
        serde_json::from_str(&input_json).expect("calendar fixture is valid JSON");
    let roundtripped: serde_json::Value =
        serde_json::from_str(&result_json).expect("roundtrip result is valid JSON");

    assert_eq!(
        original, roundtripped,
        "Calendar fixture roundtrip should preserve JSON semantics"
    );
}

#[test]
fn help_flag_shows_usage() {
    // Test 13: --help shows usage information
    Command::cargo_bin("toon")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TOON"))
        .stdout(predicate::str::contains("encode"))
        .stdout(predicate::str::contains("decode"))
        .stdout(predicate::str::contains("stats"));
}

#[test]
fn unknown_subcommand_fails() {
    // Test 14: unknown subcommand produces an error
    Command::cargo_bin("toon")
        .unwrap()
        .arg("frobnicate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("unrecognized")));
}

// ─────────────────────────────────────────────────────────────────────────────
// --filter flag on encode subcommand
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn encode_with_filter_strips_fields() {
    // Test 15: --filter strips specified fields before encoding
    let input = r#"{"name":"Event","etag":"abc","kind":"event"}"#;

    let output = Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "--filter", "etag,kind"])
        .write_stdin(input)
        .output()
        .expect("encode with --filter should succeed");

    assert!(output.status.success(), "encode with --filter must succeed");
    let toon = String::from_utf8(output.stdout).expect("output should be UTF-8");

    // The filtered TOON should contain "name" but NOT "etag" or "kind"
    assert!(
        toon.contains("name:"),
        "filtered output should contain 'name:'"
    );
    assert!(
        !toon.contains("etag"),
        "filtered output should NOT contain 'etag'"
    );
    assert!(
        !toon.contains("kind"),
        "filtered output should NOT contain 'kind'"
    );
}

#[test]
fn encode_with_filter_preset_google() {
    // Test 16: --filter-preset google strips Google Calendar noise fields
    let input = r#"{"summary":"Team Meeting","etag":"\"abc123\"","kind":"calendar#event","htmlLink":"https://calendar.google.com/event?eid=123","iCalUID":"abc@google.com","sequence":0,"start":{"dateTime":"2025-01-01T10:00:00Z"}}"#;

    let output = Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "--filter-preset", "google"])
        .write_stdin(input)
        .output()
        .expect("encode with --filter-preset google should succeed");

    assert!(
        output.status.success(),
        "encode with --filter-preset google must succeed"
    );
    let toon = String::from_utf8(output.stdout).expect("output should be UTF-8");

    // Google preset strips etag, kind, htmlLink, iCalUID, sequence
    assert!(toon.contains("summary:"), "should keep summary");
    assert!(toon.contains("start"), "should keep start");
    assert!(!toon.contains("etag"), "should strip etag");
    assert!(!toon.contains("kind"), "should strip kind");
    assert!(!toon.contains("htmlLink"), "should strip htmlLink");
    assert!(!toon.contains("iCalUID"), "should strip iCalUID");
    assert!(!toon.contains("sequence"), "should strip sequence");
}

#[test]
fn encode_filter_empty_pattern_preserves_all() {
    // Test 17: --filter with empty string preserves all fields
    let input = r#"{"name":"Alice","age":30}"#;

    let output_filtered = Command::cargo_bin("toon")
        .unwrap()
        .args(["encode", "--filter", ""])
        .write_stdin(input)
        .output()
        .expect("encode with empty --filter should succeed");

    let output_normal = Command::cargo_bin("toon")
        .unwrap()
        .arg("encode")
        .write_stdin(input)
        .output()
        .expect("encode without filter should succeed");

    assert!(output_filtered.status.success());
    assert!(output_normal.status.success());

    let toon_filtered = String::from_utf8(output_filtered.stdout).unwrap();
    let toon_normal = String::from_utf8(output_normal.stdout).unwrap();

    // Empty filter pattern should produce same output as no filter
    assert_eq!(
        toon_filtered, toon_normal,
        "empty filter should preserve all fields"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// --managed-cortex flag (stub)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn managed_cortex_without_api_key_shows_error() {
    // Test 18: --managed-cortex without --api-key shows an error message
    Command::cargo_bin("toon")
        .unwrap()
        .arg("--managed-cortex")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "--managed-cortex requires --api-key",
        ))
        .stdout(predicate::str::contains("https://temporal-cortex.dev"));
}

#[test]
fn managed_cortex_with_api_key_shows_not_available() {
    // Test 19: --managed-cortex with --api-key shows "not yet available" message
    Command::cargo_bin("toon")
        .unwrap()
        .args(["--managed-cortex", "--api-key", "test-key-123"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Managed Cortex mode is not yet available",
        ));
}
