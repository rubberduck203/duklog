//! Integration tests that validate generated ADIF files using adif-multitool.
//!
//! These tests write ADIF files to temporary directories and invoke:
//!   adif-multitool validate <file>
//!
//! `adif-multitool` must be installed and on PATH. Install with:
//!   go install github.com/flwyd/adif-multitool@latest

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{TimeZone, Utc};
use tempfile::tempdir;

use duklog::adif::format_adif;
use duklog::model::{
    Band, FdClass, FdPowerCategory, FieldDayLog, GeneralLog, Log, Mode, PotaLog, Qso, WfdClass,
    WfdLog,
};
use duklog::storage::{LogManager, export_adif};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run `adif-multitool validate <path>` and panic with a diagnostic if it fails.
fn validate_adif(path: &Path) {
    let output = Command::new("adif-multitool")
        .arg("validate")
        .arg(path)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to run adif-multitool: {e}\n\
                 Install with: go install github.com/flwyd/adif-multitool@latest"
            )
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let content = fs::read_to_string(path).unwrap_or_default();
        panic!("adif-multitool validate FAILED for {path:?}\nstderr: {stderr}\nFile:\n{content}");
    }
}

/// Write a formatted ADIF log to a temp directory and return (dir_guard, path).
fn write_adif(log: &Log) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let content = format_adif(log).expect("format_adif");
    let path = dir.path().join("log.adif");
    fs::write(&path, content).expect("write adif");
    (dir, path)
}

/// Find the single `.adif` file written by `LogManager` in the given directory.
fn find_adif(dir: &Path) -> PathBuf {
    fs::read_dir(dir)
        .expect("read_dir")
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == "adif"))
        .expect("no .adif file found in storage dir")
}

fn make_qso(call: &str) -> Qso {
    Qso::new(
        call.to_string(),
        "59".to_string(),
        "59".to_string(),
        Band::M20,
        Mode::Ssb,
        Utc.with_ymd_and_hms(2026, 2, 16, 14, 30, 0).unwrap(),
        String::new(),
        None,
        None,
        None,
    )
    .unwrap()
}

fn make_cw_qso(call: &str) -> Qso {
    Qso::new(
        call.to_string(),
        "599".to_string(),
        "599".to_string(),
        Band::M40,
        Mode::Cw,
        Utc.with_ymd_and_hms(2026, 2, 16, 15, 0, 0).unwrap(),
        String::new(),
        None,
        None,
        Some(7074),
    )
    .unwrap()
}

fn make_p2p_qso() -> Qso {
    Qso::new(
        "N0CALL".to_string(),
        "59".to_string(),
        "59".to_string(),
        Band::M20,
        Mode::Ssb,
        Utc.with_ymd_and_hms(2026, 2, 16, 16, 0, 0).unwrap(),
        "P2P contact".to_string(),
        Some("K-1234".to_string()),
        None,
        None,
    )
    .unwrap()
}

fn make_fd_qso() -> Qso {
    Qso::new(
        "KD9XYZ".to_string(),
        "59".to_string(),
        "59".to_string(),
        Band::M20,
        Mode::Ssb,
        Utc.with_ymd_and_hms(2026, 6, 21, 14, 30, 0).unwrap(),
        String::new(),
        None,
        Some("3A CT".to_string()),
        Some(14225),
    )
    .unwrap()
}

fn make_pota_log() -> Log {
    Log::Pota(
        PotaLog::new(
            "W1AW".to_string(),
            None,
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap(),
    )
}

fn make_general_log() -> Log {
    Log::General(GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap())
}

fn make_fd_log() -> Log {
    Log::FieldDay(
        FieldDayLog::new(
            "W1AW".to_string(),
            None,
            2,
            FdClass::B,
            "EPA".to_string(),
            FdPowerCategory::Low,
            "FN31".to_string(),
        )
        .unwrap(),
    )
}

fn make_wfd_log() -> Log {
    Log::WinterFieldDay(
        WfdLog::new(
            "W1AW".to_string(),
            None,
            1,
            WfdClass::H,
            "EPA".to_string(),
            "FN31".to_string(),
        )
        .unwrap(),
    )
}

// ---------------------------------------------------------------------------
// Header-only (no QSOs)
// ---------------------------------------------------------------------------

#[test]
fn pota_log_header_only_is_valid_adif() {
    let (_dir, path) = write_adif(&make_pota_log());
    validate_adif(&path);
}

#[test]
fn general_log_header_only_is_valid_adif() {
    let (_dir, path) = write_adif(&make_general_log());
    validate_adif(&path);
}

#[test]
fn field_day_log_header_only_is_valid_adif() {
    let (_dir, path) = write_adif(&make_fd_log());
    validate_adif(&path);
}

#[test]
fn wfd_log_header_only_is_valid_adif() {
    let (_dir, path) = write_adif(&make_wfd_log());
    validate_adif(&path);
}

// ---------------------------------------------------------------------------
// With QSOs
// ---------------------------------------------------------------------------

#[test]
fn pota_log_with_ssb_qso_is_valid_adif() {
    let mut log = make_pota_log();
    log.add_qso(make_qso("KD9XYZ"));
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

#[test]
fn pota_log_with_cw_qso_and_frequency_is_valid_adif() {
    let mut log = make_pota_log();
    log.add_qso(make_cw_qso("W3ABC"));
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

#[test]
fn pota_log_with_p2p_qso_is_valid_adif() {
    let mut log = make_pota_log();
    log.add_qso(make_qso("KD9XYZ"));
    log.add_qso(make_p2p_qso());
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

#[test]
fn general_log_with_multiple_qsos_is_valid_adif() {
    let mut log = make_general_log();
    log.add_qso(make_qso("KD9XYZ"));
    log.add_qso(make_cw_qso("W3ABC"));
    log.add_qso(make_qso("N0CALL"));
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

#[test]
fn field_day_log_with_qsos_is_valid_adif() {
    let mut log = make_fd_log();
    log.add_qso(make_fd_qso());
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

#[test]
fn wfd_log_with_qsos_is_valid_adif() {
    let mut log = make_wfd_log();
    log.add_qso(make_fd_qso());
    let (_dir, path) = write_adif(&log);
    validate_adif(&path);
}

// ---------------------------------------------------------------------------
// Full storage pipeline: save_log → append_qso → export → validate
// ---------------------------------------------------------------------------

#[test]
fn exported_pota_log_is_valid_adif() {
    let store_dir = tempdir().unwrap();
    let export_dir = tempdir().unwrap();

    let manager = LogManager::with_path(store_dir.path()).unwrap();
    let log = make_pota_log();
    manager.save_log(&log).unwrap();
    manager.append_qso(&log, &make_qso("KD9XYZ")).unwrap();
    manager.append_qso(&log, &make_p2p_qso()).unwrap();

    let internal = find_adif(store_dir.path());
    let export_path = export_dir.path().join("exported.adif");
    export_adif(&internal, &export_path).unwrap();

    validate_adif(&export_path);
}

#[test]
fn exported_field_day_log_is_valid_adif() {
    let store_dir = tempdir().unwrap();
    let export_dir = tempdir().unwrap();

    let manager = LogManager::with_path(store_dir.path()).unwrap();
    let log = make_fd_log();
    manager.save_log(&log).unwrap();
    manager.append_qso(&log, &make_fd_qso()).unwrap();

    let internal = find_adif(store_dir.path());
    let export_path = export_dir.path().join("fd_exported.adif");
    export_adif(&internal, &export_path).unwrap();

    validate_adif(&export_path);
}
