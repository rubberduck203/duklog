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

    /// Deletes a log file.
    pub fn delete_log(&self, log_id: &str) -> Result<(), StorageError> {
        let path = self.log_path(log_id);
        fs::remove_file(&path)?;
        Ok(())
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
