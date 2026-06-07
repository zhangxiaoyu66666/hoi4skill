//! Shared filesystem and path helpers used by the CLI commands.
//!
//! This module is deliberately small: command code decides what to read or
//! write, while these helpers keep path normalization and recursive traversal
//! behavior consistent.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(crate) fn write_if_missing(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}

pub(crate) fn relative_slash_path(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}

pub(crate) fn write_or_print(text: &str, output: Option<&str>) -> Result<(), String> {
    if let Some(path) = output {
        let path = normalize_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
        println!("{}", path.display());
    } else {
        print!("{text}");
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn normalize_path(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let p = if p.is_absolute() {
        p
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(p)
    };
    Ok(p)
}

pub(crate) fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect_files_inner(root, &mut out)?;
    Ok(out)
}

fn collect_files_inner(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| format!("read dir {}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_inner(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub(crate) fn read_utf8_lossy(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .to_string())
}

pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
