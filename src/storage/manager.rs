use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::StorageError;
use crate::model::{Log, Qso};

/// Serializable log metadata (everything except QSOs).
///
/// Used as the first line of each JSONL log file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LogMetadata {
    station_callsign: String,
    operator: Option<String>,
    park_ref: Option<String>,
    grid_square: String,
    created_at: DateTime<Utc>,
    log_id: String,
}

impl LogMetadata {
    fn from_log(log: &Log) -> Self {
        Self {
            station_callsign: log.station_callsign.clone(),
            operator: log.operator.clone(),
            park_ref: log.park_ref.clone(),
            grid_square: log.grid_square.clone(),
            created_at: log.created_at,
            log_id: log.log_id.clone(),
        }
    }

    fn into_log(self, qsos: Vec<Qso>) -> Log {
        Log {
            station_callsign: self.station_callsign,
            operator: self.operator,
            park_ref: self.park_ref,
            grid_square: self.grid_square,
            qsos,
            created_at: self.created_at,
            log_id: self.log_id,
        }
    }
}

/// Manages JSONL-based log persistence.
///
/// Each log is stored as a single `.jsonl` file: line 1 contains
/// [`LogMetadata`], lines 2+ contain individual [`Qso`] records.
pub struct LogManager {
    base_path: PathBuf,
}

impl LogManager {
    /// Creates a manager using the XDG data directory.
    ///
    /// The logs directory (`~/.local/share/duklog/logs/`) is created if it
    /// does not already exist.
    pub fn new() -> Result<Self, StorageError> {
        let data_dir = dirs::data_dir().ok_or(StorageError::NoDataDir)?;
        let base_path = data_dir.join("duklog").join("logs");
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    /// Creates a manager rooted at the given path.
    #[cfg(test)]
    pub(crate) fn with_path(path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let base_path = path.into();
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    /// Returns the file path for a given log ID.
    ///
    /// Replaces `/` in the log ID with `_` to prevent path traversal
    /// (callsigns may contain `/`, e.g. `W1AW/P`).
    fn log_path(&self, log_id: &str) -> PathBuf {
        let safe_id = log_id.replace('/', "_");
        self.base_path.join(format!("{safe_id}.jsonl"))
    }

    /// Writes a complete log to disk (metadata + all QSOs).
    ///
    /// Overwrites any existing file for this log ID.
    pub fn save_log(&self, log: &Log) -> Result<(), StorageError> {
        let path = self.log_path(&log.log_id);
        let mut file = fs::File::create(&path)?;

        let metadata = LogMetadata::from_log(log);
        serde_json::to_writer(&mut file, &metadata)?;
        writeln!(file)?;

        for qso in &log.qsos {
            serde_json::to_writer(&mut file, qso)?;
            writeln!(file)?;
        }

        Ok(())
    }

    /// Appends a single QSO to an existing log file.
    ///
    /// The log must have been previously created with [`save_log`](Self::save_log).
    /// Returns `StorageError::Io` if the file does not exist.
    pub fn append_qso(&self, log_id: &str, qso: &Qso) -> Result<(), StorageError> {
        let path = self.log_path(log_id);
        let mut file = OpenOptions::new().append(true).open(&path)?;

        serde_json::to_writer(&mut file, qso)?;
        writeln!(file)?;

        Ok(())
    }

    /// Loads a log from its JSONL file.
    ///
    /// The first line is parsed as metadata, remaining lines as QSOs.
    pub fn load_log(&self, log_id: &str) -> Result<Log, StorageError> {
        let path = self.log_path(log_id);
        load_log_from_path(&path)
    }

    /// Lists all logs sorted by `created_at` descending (newest first).
    pub fn list_logs(&self) -> Result<Vec<Log>, StorageError> {
        let mut logs: Vec<Log> = fs::read_dir(&self.base_path)?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .filter(|entry| {
                let path = entry.path();
                path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl")
            })
            .map(|entry| load_log_from_path(&entry.path()))
            .collect::<Result<Vec<_>, _>>()?;

        logs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(logs)
    }

    /// Creates a new log, checking for duplicates before saving.
    ///
    /// Returns [`StorageError::DuplicateLog`] if an existing log already has the
    /// same station callsign, operator, park reference, and grid square on the
    /// same UTC day. All comparisons are case-insensitive. Logs with different
    /// park references are never considered duplicates, allowing multiple park
    /// activations from the same location on the same day.
    ///
    /// The caller must ensure `log` has a unique `log_id`; this method compares
    /// on fields rather than identity.
    pub fn create_log(&self, log: &Log) -> Result<(), StorageError> {
        let new_date = log.created_at.date_naive();
        for existing in self.list_logs()? {
            if existing.created_at.date_naive() == new_date
                && existing.station_callsign.to_lowercase() == log.station_callsign.to_lowercase()
                && operator_eq(&existing.operator, &log.operator)
                && park_ref_eq(&existing.park_ref, &log.park_ref)
                && existing.grid_square.to_lowercase() == log.grid_square.to_lowercase()
            {
                return Err(StorageError::DuplicateLog {
                    callsign: log.station_callsign.clone(),
                    date: new_date,
                });
            }
        }
        self.save_log(log)
    }

    /// Deletes a log file.
    pub fn delete_log(&self, log_id: &str) -> Result<(), StorageError> {
        let path = self.log_path(log_id);
        fs::remove_file(&path)?;
        Ok(())
    }
}

/// Returns `true` if two operator fields represent the same operator.
///
/// `None` matches `None`; two `Some` values are compared case-insensitively.
fn operator_eq(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x.to_lowercase() == y.to_lowercase(),
        _ => false,
    }
}

/// Returns `true` if two park reference fields represent the same park.
///
/// `None` matches `None`; two `Some` values are compared case-insensitively.
/// Logs with different park references are always considered distinct, even
/// if every other field matches — operators may activate multiple parks from
/// the same location on the same day.
fn park_ref_eq(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => x.to_lowercase() == y.to_lowercase(),
        _ => false,
    }
}

/// Loads a log from the given JSONL file path.
fn load_log_from_path(path: &Path) -> Result<Log, StorageError> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let metadata_line = lines
        .next()
        .ok_or_else(|| StorageError::EmptyLogFile(path.to_path_buf()))?
        .map_err(StorageError::Io)?;

    let metadata: LogMetadata = serde_json::from_str(&metadata_line)?;

    let qsos = lines
        .map(|line| {
            let line = line?;
            serde_json::from_str(&line).map_err(StorageError::Json)
        })
        .collect::<Result<Vec<Qso>, StorageError>>()?;

    Ok(metadata.into_log(qsos))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use chrono::{TimeZone, Utc};
    use quickcheck_macros::quickcheck;
    use tempfile::tempdir;

    use super::*;
    use crate::model::{Band, Mode};

    fn make_log() -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.log_id = "test-log".to_string();
        log.created_at = Utc.with_ymd_and_hms(2026, 2, 16, 12, 0, 0).unwrap();
        log
    }

    fn make_log_with_id(id: &str, year: i32) -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.log_id = id.to_string();
        log.created_at = Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0).unwrap();
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

    fn make_p2p_qso() -> Qso {
        Qso::new(
            "N0CALL".to_string(),
            "59".to_string(),
            "59".to_string(),
            Band::M40,
            Mode::Cw,
            Utc.with_ymd_and_hms(2026, 2, 16, 15, 0, 0).unwrap(),
            "P2P".to_string(),
            Some("K-1234".to_string()),
        )
        .unwrap()
    }

    fn make_manager() -> (tempfile::TempDir, LogManager) {
        let dir = tempdir().unwrap();
        let manager = LogManager::with_path(dir.path()).unwrap();
        (dir, manager)
    }

    // --- Round-trip tests ---

    #[test]
    fn save_and_load_empty_log() {
        let (_dir, manager) = make_manager();
        let log = make_log();
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log(&log.log_id).unwrap();
        assert_eq!(log, loaded);
    }

    #[test]
    fn save_and_load_log_with_qsos() {
        let (_dir, manager) = make_manager();
        let mut log = make_log();
        log.add_qso(make_qso());
        log.add_qso(make_p2p_qso());
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log(&log.log_id).unwrap();
        assert_eq!(log, loaded);
    }

    #[quickcheck]
    fn round_trip_preserves_qso_count(n: u8) -> bool {
        let n = n.min(20) as usize;
        let (_dir, manager) = make_manager();
        let mut log = make_log();
        for _ in 0..n {
            log.add_qso(make_qso());
        }
        manager.save_log(&log).unwrap();
        let loaded = manager.load_log(&log.log_id).unwrap();
        loaded.qsos.len() == n
    }

    // --- Append tests ---

    #[test]
    fn append_qso_adds_to_existing_log() {
        let (_dir, manager) = make_manager();
        let log = make_log();
        manager.save_log(&log).unwrap();

        manager.append_qso(&log.log_id, &make_qso()).unwrap();
        manager.append_qso(&log.log_id, &make_p2p_qso()).unwrap();

        let loaded = manager.load_log(&log.log_id).unwrap();
        assert_eq!(loaded.qsos.len(), 2);
        assert_eq!(loaded.qsos[0], make_qso());
        assert_eq!(loaded.qsos[1], make_p2p_qso());
    }

    #[quickcheck]
    fn append_n_qsos_yields_n_total(n: u8) -> bool {
        let n = n.min(20) as usize;
        let (_dir, manager) = make_manager();
        let log = make_log();
        manager.save_log(&log).unwrap();

        for _ in 0..n {
            manager.append_qso(&log.log_id, &make_qso()).unwrap();
        }

        let loaded = manager.load_log(&log.log_id).unwrap();
        loaded.qsos.len() == n
    }

    // --- List tests ---

    #[test]
    fn list_logs_returns_all_sorted_by_created_at_desc() {
        let (_dir, manager) = make_manager();

        let older = make_log_with_id("older", 2025);
        let newer = make_log_with_id("newer", 2026);

        manager.save_log(&older).unwrap();
        manager.save_log(&newer).unwrap();

        let logs = manager.list_logs().unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].log_id, "newer");
        assert_eq!(logs[1].log_id, "older");
    }

    #[test]
    fn list_logs_empty_directory() {
        let (_dir, manager) = make_manager();
        let logs = manager.list_logs().unwrap();
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn list_logs_ignores_non_jsonl_files() {
        let (dir, manager) = make_manager();
        fs::write(dir.path().join("notes.txt"), "not a log").unwrap();

        let log = make_log();
        manager.save_log(&log).unwrap();

        let logs = manager.list_logs().unwrap();
        assert_eq!(logs.len(), 1);
    }

    // --- Delete tests ---

    #[test]
    fn delete_removes_log_file() {
        let (_dir, manager) = make_manager();
        let log = make_log();
        manager.save_log(&log).unwrap();

        manager.delete_log(&log.log_id).unwrap();

        let result = manager.load_log(&log.log_id);
        assert!(matches!(result, Err(StorageError::Io(_))));
    }

    #[test]
    fn delete_nonexistent_log_returns_error() {
        let (_dir, manager) = make_manager();
        let result = manager.delete_log("nonexistent");
        assert!(matches!(result, Err(StorageError::Io(_))));
    }

    // --- Error cases ---

    #[test]
    fn load_nonexistent_log_returns_error() {
        let (_dir, manager) = make_manager();
        let result = manager.load_log("nonexistent");
        assert!(matches!(result, Err(StorageError::Io(_))));
    }

    #[test]
    fn load_empty_file_returns_empty_log_error() {
        let (dir, manager) = make_manager();
        fs::write(dir.path().join("empty.jsonl"), "").unwrap();

        let result = manager.load_log("empty");
        assert!(matches!(result, Err(StorageError::EmptyLogFile(_))));
    }

    #[test]
    fn load_corrupt_json_returns_error() {
        let (dir, manager) = make_manager();
        let path = dir.path().join("bad.jsonl");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "{{not valid json}}").unwrap();

        let result = manager.load_log("bad");
        assert!(matches!(result, Err(StorageError::Json(_))));
    }

    #[test]
    fn load_corrupt_qso_line_returns_error() {
        let (dir, manager) = make_manager();
        let log = make_log();
        manager.save_log(&log).unwrap();

        // Append a corrupt QSO line
        let path = dir.path().join("test-log.jsonl");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(file, "{{bad qso}}").unwrap();

        let result = manager.load_log("test-log");
        assert!(matches!(result, Err(StorageError::Json(_))));
    }

    #[test]
    fn save_overwrites_existing_file() {
        let (_dir, manager) = make_manager();
        let mut log = make_log();
        log.add_qso(make_qso());
        log.add_qso(make_qso());
        manager.save_log(&log).unwrap();

        // Save again without QSOs
        let log = make_log();
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log(&log.log_id).unwrap();
        assert_eq!(loaded.qsos.len(), 0);
    }

    #[test]
    fn with_path_creates_directory() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c");
        let _manager = LogManager::with_path(&nested).unwrap();
        assert!(nested.exists());
    }

    // --- Metadata round-trip ---

    #[test]
    fn metadata_preserves_optional_park_ref() {
        let (_dir, manager) = make_manager();
        let mut log = make_log();
        log.park_ref = None;
        log.log_id = "no-park".to_string();
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log("no-park").unwrap();
        assert_eq!(loaded.park_ref, None);
    }

    #[test]
    fn old_format_operator_string_deserializes_to_some() {
        let (dir, manager) = make_manager();
        let json = r#"{"station_callsign":"W1AW","operator":"W1AW","park_ref":null,"grid_square":"FN31","created_at":"2026-02-16T12:00:00Z","log_id":"compat"}"#;
        fs::write(dir.path().join("compat.jsonl"), format!("{json}\n")).unwrap();
        let loaded = manager.load_log("compat").unwrap();
        assert_eq!(loaded.operator, Some("W1AW".to_string()));
    }

    #[test]
    fn metadata_preserves_none_operator() {
        let (_dir, manager) = make_manager();
        let mut log = Log::new(
            "W1AW".to_string(),
            None,
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.log_id = "no-op".to_string();
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log("no-op").unwrap();
        assert_eq!(loaded.operator, None);
    }

    // --- create_log duplicate prevention ---

    fn make_log_for_today(id: &str) -> Log {
        let mut log = Log::new(
            "W1AW".to_string(),
            Some("W1AW".to_string()),
            Some("K-0001".to_string()),
            "FN31".to_string(),
        )
        .unwrap();
        log.log_id = id.to_string();
        // created_at defaults to Utc::now() — same day as the new log in create_log
        log
    }

    fn make_log_for_yesterday(id: &str) -> Log {
        let mut log = make_log_for_today(id);
        let yesterday = Utc::now().date_naive().pred_opt().unwrap();
        log.created_at = Utc.from_utc_datetime(&yesterday.and_hms_opt(12, 0, 0).unwrap());
        log
    }

    #[test]
    fn create_log_succeeds_when_no_existing_logs() {
        let (_dir, manager) = make_manager();
        let log = make_log_for_today("new");
        manager.create_log(&log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 1);
    }

    #[test]
    fn create_log_rejects_exact_duplicate_same_day() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
        // Existing log must not be overwritten
        assert_eq!(manager.list_logs().unwrap().len(), 1);
    }

    #[test]
    fn create_log_allows_different_utc_day() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_yesterday("old");
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_allows_different_callsign() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.station_callsign = "KD9XYZ".to_string();
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_allows_different_operator() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.operator = Some("KD9XYZ".to_string());
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_allows_different_grid() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.grid_square = "EM10".to_string();
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_rejects_duplicate_case_insensitive_callsign() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.station_callsign = "w1aw".to_string();
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        // new_log has "W1AW" — differs only in case from "w1aw"
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
    }

    #[test]
    fn create_log_allows_none_vs_some_operator() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.operator = None;
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        // new_log has operator = Some("W1AW"), existing has None → not a duplicate
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_allows_some_vs_none_operator() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        // existing has operator = Some("W1AW")
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.operator = None;
        // new_log has operator = None → not a duplicate
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_rejects_duplicate_none_operators() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.operator = None;
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.operator = None;
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
    }

    #[test]
    fn create_log_rejects_duplicate_case_insensitive_grid() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.grid_square = "fn31".to_string();
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        // new_log has "FN31" — differs only in case from "fn31"
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
    }

    #[test]
    fn create_log_error_contains_callsign_and_date() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        let err = manager.create_log(&new_log).unwrap_err();
        // Verify the structured error fields are formatted into the message
        let msg = err.to_string();
        assert!(msg.contains("W1AW"), "error should contain callsign");
        assert!(msg.contains("UTC"), "error should reference UTC");
    }

    #[test]
    fn create_log_propagates_list_logs_error() {
        let (dir, manager) = make_manager();
        let log = make_log_for_today("new");
        // Write a corrupt JSONL file to make list_logs fail
        fs::write(dir.path().join("corrupt.jsonl"), "{bad json}\n").unwrap();
        let result = manager.create_log(&log);
        assert!(matches!(result, Err(StorageError::Json(_))));
    }

    #[test]
    fn create_log_allows_different_park_ref() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        // existing has park_ref = Some("K-0001")
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.park_ref = Some("K-0002".to_string());
        // Different park on same day — not a duplicate
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_allows_park_ref_vs_no_park_ref() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        // existing has park_ref = Some("K-0001")
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.park_ref = None;
        // One is a POTA log, one is a generic log — not a duplicate
        manager.create_log(&new_log).unwrap();
        assert_eq!(manager.list_logs().unwrap().len(), 2);
    }

    #[test]
    fn create_log_rejects_duplicate_same_park_ref() {
        let (_dir, manager) = make_manager();
        let existing = make_log_for_today("existing");
        manager.save_log(&existing).unwrap();

        let new_log = make_log_for_today("new");
        // Same park_ref = Some("K-0001") — is a duplicate
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
    }

    #[test]
    fn create_log_rejects_duplicate_case_insensitive_operator() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.station_callsign = "W3DUK".to_string();
        existing.operator = Some("w3duk".to_string());
        existing.park_ref = None;
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.station_callsign = "W3DUK".to_string();
        new_log.operator = Some("W3DUK".to_string());
        new_log.park_ref = None;
        let result = manager.create_log(&new_log);
        assert!(
            matches!(result, Err(StorageError::DuplicateLog { .. })),
            "duplicate with different-case operator should be blocked"
        );
    }

    #[test]
    fn create_log_rejects_duplicate_none_park_refs() {
        let (_dir, manager) = make_manager();
        let mut existing = make_log_for_today("existing");
        existing.park_ref = None;
        manager.save_log(&existing).unwrap();

        let mut new_log = make_log_for_today("new");
        new_log.park_ref = None;
        // Both have no park ref — is a duplicate
        let result = manager.create_log(&new_log);
        assert!(matches!(result, Err(StorageError::DuplicateLog { .. })));
    }

    // --- operator_eq unit tests ---

    mod operator_eq {
        use super::*;

        #[test]
        fn none_and_none_are_equal() {
            assert!(operator_eq(&None, &None));
        }

        #[test]
        fn some_and_none_differ() {
            assert!(!operator_eq(&Some("W1AW".into()), &None));
        }

        #[test]
        fn none_and_some_differ() {
            assert!(!operator_eq(&None, &Some("W1AW".into())));
        }

        #[test]
        fn same_case_some_are_equal() {
            assert!(operator_eq(&Some("W1AW".into()), &Some("W1AW".into())));
        }

        #[test]
        fn different_case_some_are_equal() {
            assert!(operator_eq(&Some("W1AW".into()), &Some("w1aw".into())));
        }

        #[test]
        fn different_callsign_some_differ() {
            assert!(!operator_eq(&Some("W1AW".into()), &Some("KD9XYZ".into())));
        }
    }

    mod park_ref_eq {
        use super::*;

        #[test]
        fn none_and_none_are_equal() {
            assert!(park_ref_eq(&None, &None));
        }

        #[test]
        fn some_and_none_differ() {
            assert!(!park_ref_eq(&Some("K-0001".into()), &None));
        }

        #[test]
        fn none_and_some_differ() {
            assert!(!park_ref_eq(&None, &Some("K-0001".into())));
        }

        #[test]
        fn same_park_ref_equal() {
            assert!(park_ref_eq(&Some("K-0001".into()), &Some("K-0001".into())));
        }

        #[test]
        fn different_case_park_ref_equal() {
            assert!(park_ref_eq(&Some("k-0001".into()), &Some("K-0001".into())));
        }

        #[test]
        fn different_park_ref_differ() {
            assert!(!park_ref_eq(&Some("K-0001".into()), &Some("K-0002".into())));
        }
    }

    // --- Path safety ---

    #[test]
    fn log_id_with_slash_round_trips() {
        let (_dir, manager) = make_manager();
        let mut log = make_log();
        log.log_id = "W1AW/P-20260216-120000".to_string();
        manager.save_log(&log).unwrap();

        let loaded = manager.load_log("W1AW/P-20260216-120000").unwrap();
        assert_eq!(loaded.log_id, "W1AW/P-20260216-120000");
    }
}
