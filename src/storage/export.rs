use std::fs;
use std::path::{Path, PathBuf};

use super::error::StorageError;
use crate::adif;
use crate::model::{DefaultFilename, Log};

/// Exports a log as an ADIF file at the given path.
///
/// Creates any missing parent directories before writing.
pub fn export_adif(log: &Log, path: &Path) -> Result<(), StorageError> {
    let content = adif::format_adif(log)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Returns the default export path for a log.
///
/// Files are written to `~/Documents/duklog/`, falling back to `~/duklog/` if
/// the documents directory is unavailable.
///
/// Filename formats by log type:
/// - POTA: `{CALLSIGN}@{PARK}-{YYYYMMDD}.adif`
/// - General: `{CALLSIGN}-{YYYYMMDD}.adif`
/// - Field Day: `{CALLSIGN}-FD-{YYYYMMDD}.adif`
/// - Winter Field Day: `{CALLSIGN}-WFD-{YYYYMMDD}.adif`
///
/// `/` in callsigns is replaced with `_` (e.g. `W1AW/P` → `W1AW_P`).
///
/// Returns `StorageError::NoHomeDir` if no suitable directory can be
/// determined.
pub fn default_export_path(log: &Log) -> Result<PathBuf, StorageError> {
    let base = dirs::document_dir()
        .or_else(dirs::home_dir)
        .map(|d| d.join("duklog"))
        .ok_or(StorageError::NoHomeDir)?;
    Ok(base.join(log.default_filename()))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use super::*;
    use crate::model::{
        Band, FdClass, FdPowerCategory, FieldDayLog, GeneralLog, Mode, PotaLog, Qso, WfdClass,
        WfdLog,
    };

    fn make_pota_log() -> Log {
        let mut log = PotaLog::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            "K-0001".to_string(),
            "FN31".to_string(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::Pota(log)
    }

    fn make_general_log() -> Log {
        let mut log = GeneralLog::new("W1AW".to_string(), None, "FN31".to_string()).unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::General(log)
    }

    fn make_fd_log() -> Log {
        let mut log = FieldDayLog::new(
            "W1AW".to_string(),
            None,
            1,
            FdClass::A,
            "EPA".to_string(),
            FdPowerCategory::Low,
            String::new(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::FieldDay(log)
    }

    fn make_wfd_log() -> Log {
        let mut log = WfdLog::new(
            "W1AW".to_string(),
            None,
            1,
            WfdClass::H,
            "EPA".to_string(),
            String::new(),
        )
        .unwrap();
        log.header.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        Log::WinterFieldDay(log)
    }

    fn make_qso() -> Qso {
        Qso::new(
            "KD9XYZ".to_string(),
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

    // --- export_adif tests ---

    #[test]
    fn export_creates_file_with_header_and_records() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.adif");

        let mut log = make_pota_log();
        log.add_qso(make_qso());
        export_adif(&log, &path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let eoh_pos = content.find("<eoh>").expect("missing <eoh>");
        let eor_pos = content.find("<eor>").expect("missing <eor>");
        assert!(eoh_pos < eor_pos, "header must precede records");
        assert!(content.contains("<CALL:6>KD9XYZ"));
    }

    #[test]
    fn export_empty_log_produces_header_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty.adif");

        export_adif(&make_pota_log(), &path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("<eoh>"));
        assert!(!content.contains("<eor>"));
    }

    #[test]
    fn export_creates_missing_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("nested").join("out.adif");

        export_adif(&make_pota_log(), &path).unwrap();

        assert!(path.exists());
    }

    // --- default_export_path tests ---

    #[test]
    fn default_path_with_park_ref() {
        let log = make_pota_log();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW@K-0001-20260216.adif");
    }

    #[test]
    fn default_path_sanitizes_portable_callsign_with_park_ref() {
        let mut log = make_pota_log();
        log.header_mut().station_callsign = "W1AW/P".to_string();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW_P@K-0001-20260216.adif");
    }

    #[test]
    fn default_path_general_sanitizes_portable_callsign() {
        let mut log = make_general_log();
        log.header_mut().station_callsign = "W1AW/P".to_string();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW_P-20260216.adif");
    }

    #[test]
    fn default_path_general_log() {
        let log = make_general_log();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW-20260216.adif");
    }

    #[test]
    fn default_path_fd_log() {
        let log = make_fd_log();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW-FD-20260216.adif");
    }

    #[test]
    fn default_path_wfd_log() {
        let log = make_wfd_log();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "W1AW-WFD-20260216.adif");
    }

    #[test]
    fn default_path_is_in_duklog_subdirectory() {
        let log = make_pota_log();
        let path = default_export_path(&log).unwrap();
        // Parent is always a `duklog/` directory, whether under Documents or home.
        assert_eq!(path.parent().unwrap().file_name().unwrap(), "duklog");
    }
}
