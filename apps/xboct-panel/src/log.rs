//! The tail of a service log: the last lines, read from the end of the file.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const CHUNK: u64 = 8 * 1024;
const LIMIT: usize = 256 * 1024;

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum LogError {
    #[error("failed to open the log {path}: {message}")]
    Open { path: PathBuf, message: String },
    #[error("failed to read the log {path}: {message}")]
    Read { path: PathBuf, message: String },
}

pub fn tail(path: &Path, lines: usize) -> Result<String, LogError> {
    if lines == 0 {
        return Ok(String::new());
    }
    let mut file = File::open(path).map_err(|failure| LogError::Open {
        path: path.to_owned(),
        message: failure.to_string(),
    })?;
    let length = file
        .metadata()
        .map_err(|failure| LogError::Read {
            path: path.to_owned(),
            message: failure.to_string(),
        })?
        .len();
    let read = |file: &mut File, from: u64, size: u64| {
        let mut chunk = vec![0u8; size as usize];
        file.seek(SeekFrom::Start(from))
            .and_then(|_| file.read_exact(&mut chunk))
            .map(|()| chunk)
            .map_err(|failure| LogError::Read {
                path: path.to_owned(),
                message: failure.to_string(),
            })
    };

    let mut position = length;
    let mut buffer: Vec<u8> = Vec::new();
    let mut breaks = 0;
    while position > 0 && breaks <= lines && buffer.len() < LIMIT {
        let size = CHUNK.min(position);
        position -= size;
        let mut chunk = read(&mut file, position, size)?;
        breaks += chunk.iter().filter(|byte| **byte == b'\n').count();
        chunk.extend_from_slice(&buffer);
        buffer = chunk;
    }

    let text = String::from_utf8_lossy(&buffer);
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines);
    Ok(all[start..].join("\n"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time after the epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("panel-log-{name}-{stamp}.log"))
    }

    fn written(name: &str, text: &str) -> PathBuf {
        let path = temp_path(name);
        fs::write(&path, text).expect("log file");
        path
    }

    #[test]
    fn the_last_lines_are_returned() {
        let path = written("last", "one\ntwo\nthree\nfour\n");
        assert_eq!(tail(&path, 2).expect("tail"), "three\nfour");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_short_log_is_returned_whole() {
        let path = written("short", "one\ntwo\n");
        assert_eq!(tail(&path, 10).expect("tail"), "one\ntwo");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn carriage_returns_do_not_leak_into_lines() {
        let path = written("crlf", "one\r\ntwo\r\n");
        assert_eq!(tail(&path, 1).expect("tail"), "two");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn an_empty_log_has_no_lines() {
        let path = written("empty", "");
        assert_eq!(tail(&path, 5).expect("tail"), "");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn a_missing_log_names_its_path() {
        let path = temp_path("missing");
        let failure = tail(&path, 5).expect_err("missing log");
        assert!(failure.to_string().contains("panel-log-missing"));
    }

    #[test]
    fn a_long_log_is_read_from_the_end() {
        let mut text = String::new();
        for number in 0..5000 {
            text.push_str(&format!("line {number}\n"));
        }
        let path = written("long", &text);
        assert_eq!(tail(&path, 2).expect("tail"), "line 4998\nline 4999");
        let _ = fs::remove_file(&path);
    }
}
