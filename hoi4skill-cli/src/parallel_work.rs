//! Bounded scheduling helpers for filesystem-heavy CPU work.
//!
//! HOI4 projects contain many tiny files mixed with a few very large files.
//! These helpers keep the worker count owned by Rayon while creating enough
//! byte-balanced work for modern CPUs without spawning a task per file.

use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const TARGET_CHUNKS_PER_WORKER: usize = 8;
const MIN_TARGET_BYTES: u64 = 256 * 1024;
const MAX_TARGET_BYTES: u64 = 4 * 1024 * 1024;
const MAX_FILES_PER_CHUNK: usize = 512;

#[derive(Clone, Debug)]
pub(crate) struct FileWork {
    pub(crate) path: PathBuf,
    pub(crate) bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct WorkChunk {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn collect_file_work_parallel(
    root: &Path,
    extensions: &[&str],
) -> Result<Vec<FileWork>, String> {
    let mut entries = fs::read_dir(root)
        .map_err(|error| format!("read dir {}: {error}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_unstable_by_key(|entry| entry.path());

    let partials = entries
        .into_par_iter()
        .map(|entry| {
            let mut files = Vec::new();
            collect_entry_work(entry, extensions, &mut files)?;
            Ok(files)
        })
        .collect::<Vec<Result<Vec<FileWork>, String>>>();

    let mut files = Vec::new();
    for partial in partials {
        files.extend(partial?);
    }
    files.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn collect_entry_work(
    entry: fs::DirEntry,
    extensions: &[&str],
    files: &mut Vec<FileWork>,
) -> Result<(), String> {
    let path = entry.path();
    let Ok(file_type) = entry.file_type() else {
        return Ok(());
    };
    let metadata = if file_type.is_symlink() {
        fs::metadata(&path).ok()
    } else {
        None
    };
    let is_directory = file_type.is_dir() || metadata.as_ref().is_some_and(|item| item.is_dir());
    let is_file = file_type.is_file() || metadata.as_ref().is_some_and(|item| item.is_file());
    if is_directory {
        let entries =
            fs::read_dir(&path).map_err(|error| format!("read dir {}: {error}", path.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            collect_entry_work(entry, extensions, files)?;
        }
    } else if is_file
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|candidate| extension.eq_ignore_ascii_case(candidate))
            })
    {
        let bytes = metadata
            .map(|item| item.len())
            .or_else(|| entry.metadata().ok().map(|item| item.len()));
        if let Some(bytes) = bytes {
            files.push(FileWork { path, bytes });
        }
    }
    Ok(())
}

pub(crate) fn plan_byte_balanced_chunks(files: &[FileWork]) -> Vec<WorkChunk> {
    if files.is_empty() {
        return Vec::new();
    }
    let workers = rayon::current_num_threads().max(1);
    let target_chunks = workers.saturating_mul(TARGET_CHUNKS_PER_WORKER).max(1);
    let total_bytes = files
        .iter()
        .fold(0_u64, |total, file| total.saturating_add(file.bytes));
    let target_bytes = total_bytes
        .div_ceil(target_chunks as u64)
        .clamp(MIN_TARGET_BYTES, MAX_TARGET_BYTES);

    let mut chunks = Vec::with_capacity(target_chunks);
    let mut start = 0;
    let mut bytes = 0_u64;
    for (index, file) in files.iter().enumerate() {
        bytes = bytes.saturating_add(file.bytes);
        let file_count = index + 1 - start;
        if bytes >= target_bytes || file_count >= MAX_FILES_PER_CHUNK {
            chunks.push(WorkChunk {
                start,
                end: index + 1,
            });
            start = index + 1;
            bytes = 0;
        }
    }
    if start < files.len() {
        chunks.push(WorkChunk {
            start,
            end: files.len(),
        });
    }
    chunks
}

pub(crate) fn files_per_cpu_chunk(file_count: usize) -> usize {
    if file_count == 0 {
        return 1;
    }
    let target_chunks = rayon::current_num_threads()
        .max(1)
        .saturating_mul(TARGET_CHUNKS_PER_WORKER)
        .max(1);
    file_count.div_ceil(target_chunks).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn work(bytes: u64) -> FileWork {
        FileWork {
            path: PathBuf::new(),
            bytes,
        }
    }

    #[test]
    fn byte_balanced_chunks_cover_each_file_once() {
        let files = vec![work(1), work(512 * 1024), work(8 * 1024 * 1024), work(1)];
        let chunks = plan_byte_balanced_chunks(&files);
        assert_eq!(chunks.first().map(|chunk| chunk.start), Some(0));
        assert_eq!(chunks.last().map(|chunk| chunk.end), Some(files.len()));
        assert!(chunks.windows(2).all(|pair| pair[0].end == pair[1].start));
    }

    #[test]
    fn zero_byte_files_are_still_bounded() {
        let files = (0..(MAX_FILES_PER_CHUNK + 1))
            .map(|_| work(0))
            .collect::<Vec<_>>();
        let chunks = plan_byte_balanced_chunks(&files);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].end - chunks[0].start, MAX_FILES_PER_CHUNK);
    }
}
