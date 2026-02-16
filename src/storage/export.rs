use std::fs;
use std::path::{Path, PathBuf};

use super::error::StorageError;
use crate::adif;
use crate::model::Log;

/// Exports a log as an ADIF file at the given path.
pub fn export_adif(log: &Log, path: &Path) -> Result<(), StorageError> {
    let content = adif::format_adif(log)?;
    fs::write(path, content)?;
    Ok(())
}

/// Returns the default export path for a log.
///
/// Format: `~/duklog-{PARK}-{YYYYMMDD}.adif` when a park ref is set,
/// or `~/duklog-{CALLSIGN}-{YYYYMMDD}.adif` otherwise.
///
/// Returns `StorageError::NoHomeDir` if the home directory cannot be
/// determined.
pub fn default_export_path(log: &Log) -> Result<PathBuf, StorageError> {
    let prefix = log.park_ref.as_deref().unwrap_or(&log.station_callsign);
    let date = log.created_at.format("%Y%m%d");
    let filename = format!("duklog-{prefix}-{date}.adif");

    let home = dirs::home_dir().ok_or(StorageError::NoHomeDir)?;
    Ok(home.join(filename))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use super::*;
    use crate::model::{Band, Mode, Qso};

    fn make_log() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            "W1AW".to_string(),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_log_without_park() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            "W1AW".to_string(),
            None,
            "FN31".to_string(),
        )
        .unwrap();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
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
        )
        .unwrap()
    }

    // --- export_adif tests ---

    #[test]
    fn export_creates_file_with_header_and_records() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.adif");

        let mut log = make_log();
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

        export_adif(&make_log(), &path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("<eoh>"));
        assert!(!content.contains("<eor>"));
    }

    // --- default_export_path tests ---

    #[test]
    fn default_path_with_park_ref() {
        let log = make_log();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "duklog-K-0001-20260216.adif");
    }

    #[test]
    fn default_path_without_park_ref() {
        let log = make_log_without_park();
        let path = default_export_path(&log).unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "duklog-W1AW-20260216.adif");
    }

    #[test]
    fn default_path_is_in_home_directory() {
        let log = make_log();
        let path = default_export_path(&log).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(path.parent().unwrap(), home);
    }
}
