//! HOI4 load-order and `replace_path` aware source traversal.
//!
//! Roots are supplied from lowest to highest priority. A `replace_path`
//! declared by a higher-priority mod hides that relative subtree in every
//! lower-priority root. The normal collector prunes before descending into a
//! hidden directory. Diagnostic mode is separate: it reads hidden files for
//! update investigation without adding them to the effective source view.

use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(crate) const DEFAULT_REPLACED_DIAGNOSTIC_FILES: usize = 200;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct LayeredScanOptions {
    pub(crate) replace_path_diagnostics: bool,
    pub(crate) max_replaced_files: usize,
}

impl LayeredScanOptions {
    pub(crate) fn effective() -> Self {
        Self {
            replace_path_diagnostics: false,
            max_replaced_files: DEFAULT_REPLACED_DIAGNOSTIC_FILES,
        }
    }
}

impl Default for LayeredScanOptions {
    fn default() -> Self {
        Self::effective()
    }
}

pub(crate) fn layered_scan_options_from_args(map: &ArgMap) -> Result<LayeredScanOptions, String> {
    Ok(LayeredScanOptions {
        replace_path_diagnostics: map.flags.contains("replace-path-diagnostics"),
        max_replaced_files: parse_usize_option(
            map,
            "max-replaced-files",
            DEFAULT_REPLACED_DIAGNOSTIC_FILES,
        )?,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct LayeredSourceLayer {
    pub(crate) root: PathBuf,
    replace_paths: BTreeSet<String>,
    masked_by_higher: BTreeMap<String, PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct LayeredSourcePlan {
    layers: Vec<LayeredSourceLayer>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct LayeredScanReport {
    pub(crate) mode: String,
    pub(crate) declarations: Vec<ReplacePathDeclaration>,
    pub(crate) pruned_subtrees: usize,
    pub(crate) diagnostic_files_total: usize,
    pub(crate) diagnostic_bytes_total: u64,
    pub(crate) diagnostic_files: Vec<ReplacedFileDiagnostic>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReplacePathDeclaration {
    pub(crate) root: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ReplacedFileDiagnostic {
    pub(crate) hidden_root: String,
    pub(crate) replaced_by: String,
    pub(crate) replace_path: String,
    pub(crate) relative_path: String,
    pub(crate) bytes: u64,
    pub(crate) modified_ns: u128,
    pub(crate) content_hash: String,
}

impl LayeredSourcePlan {
    pub(crate) fn from_roots(roots: &[PathBuf]) -> Result<Self, String> {
        let mut layers = Vec::new();
        for root in dedupe_layer_roots(roots) {
            layers.push(LayeredSourceLayer {
                replace_paths: replace_paths_for_root(&root)?,
                root,
                masked_by_higher: BTreeMap::new(),
            });
        }

        let mut active_masks = BTreeMap::<String, PathBuf>::new();
        for layer in layers.iter_mut().rev() {
            layer.masked_by_higher = active_masks.clone();
            for path in &layer.replace_paths {
                // Keep the highest-priority declarer when more than one layer
                // replaces the same subtree.
                active_masks
                    .entry(path.clone())
                    .or_insert_with(|| layer.root.clone());
            }
        }
        Ok(Self { layers })
    }

    pub(crate) fn layers(&self) -> &[LayeredSourceLayer] {
        &self.layers
    }

    pub(crate) fn roots(&self) -> Vec<PathBuf> {
        self.layers.iter().map(|layer| layer.root.clone()).collect()
    }

    pub(crate) fn layer_index(&self, root: &Path) -> Option<usize> {
        self.layers
            .iter()
            .position(|layer| layered_paths_equal(&layer.root, root))
    }

    pub(crate) fn layer_declares_replace_paths(&self, layer_index: usize) -> bool {
        self.layers
            .get(layer_index)
            .is_some_and(|layer| !layer.replace_paths.is_empty())
    }

    pub(crate) fn is_visible(&self, layer_index: usize, relative: &str) -> bool {
        let Some(layer) = self.layers.get(layer_index) else {
            return false;
        };
        let relative = normalize_replace_path(relative);
        !layer
            .masked_by_higher
            .keys()
            .any(|mask| path_is_within(&relative, mask))
    }

    pub(crate) fn collect_files(
        &self,
        layer_index: usize,
        relative_directory: &str,
    ) -> Result<Vec<PathBuf>, String> {
        self.collect_files_from(layer_index, relative_directory, relative_directory)
    }

    /// Collect files from a physical directory mounted at a different virtual
    /// path. HOI4 DLC directories are the main example: `dlc/<id>/interface`
    /// participates in the virtual `interface` tree and must obey a Mod's
    /// `replace_path = "interface"` declaration.
    pub(crate) fn collect_files_from(
        &self,
        layer_index: usize,
        physical_relative_directory: &str,
        virtual_relative_directory: &str,
    ) -> Result<Vec<PathBuf>, String> {
        let layer = self
            .layers
            .get(layer_index)
            .ok_or_else(|| format!("invalid layered source index {layer_index}"))?;
        let physical_relative_directory = normalize_replace_path(physical_relative_directory);
        let virtual_relative_directory = normalize_replace_path(virtual_relative_directory);
        if !self.is_visible(layer_index, &virtual_relative_directory) {
            return Ok(Vec::new());
        }
        let directory = layer
            .root
            .join(physical_relative_directory.replace('/', "\\"));
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        collect_visible_files_inner(
            self,
            layer_index,
            &directory,
            &virtual_relative_directory,
            &directory,
            &mut files,
        )?;
        files.sort();
        Ok(files)
    }

    pub(crate) fn visible_file(&self, layer_index: usize, relative: &str) -> Option<PathBuf> {
        if !self.is_visible(layer_index, relative) {
            return None;
        }
        let layer = self.layers.get(layer_index)?;
        let path = layer.root.join(relative.replace('/', "\\"));
        path.is_file().then_some(path)
    }

    pub(crate) fn visibility_fingerprint(&self, layer_index: usize) -> String {
        let Some(layer) = self.layers.get(layer_index) else {
            return String::new();
        };
        let mut value = slash_path(&layer.root);
        for (path, owner) in &layer.masked_by_higher {
            value.push('|');
            value.push_str(path);
            value.push('@');
            value.push_str(&slash_path(owner));
        }
        format!("{:016x}", stable_text_hash(value.as_bytes()))
    }

    pub(crate) fn report(&self, options: LayeredScanOptions) -> Result<LayeredScanReport, String> {
        let mut report = LayeredScanReport {
            mode: if options.replace_path_diagnostics {
                "replace_path_diagnostics".to_string()
            } else {
                "effective".to_string()
            },
            ..Default::default()
        };
        for layer in &self.layers {
            for path in &layer.replace_paths {
                report.declarations.push(ReplacePathDeclaration {
                    root: layer.root.display().to_string(),
                    path: path.clone(),
                });
            }
        }

        let mut seen = BTreeSet::new();
        for layer in &self.layers {
            for (replace_path, replaced_by) in minimal_masks(&layer.masked_by_higher) {
                let candidate = layer.root.join(replace_path.replace('/', "\\"));
                if !candidate.exists() {
                    continue;
                }
                report.pruned_subtrees += 1;
                if !options.replace_path_diagnostics {
                    continue;
                }
                let files = if candidate.is_file() {
                    vec![candidate]
                } else {
                    collect_files(&candidate)?
                };
                for file in files {
                    let key = slash_path(&file).to_ascii_lowercase();
                    if !seen.insert(key) {
                        continue;
                    }
                    let metadata = fs::metadata(&file)
                        .map_err(|error| format!("metadata {}: {error}", file.display()))?;
                    let bytes = fs::read(&file).map_err(|error| {
                        format!("read diagnostic file {}: {error}", file.display())
                    })?;
                    report.diagnostic_files_total += 1;
                    report.diagnostic_bytes_total =
                        report.diagnostic_bytes_total.saturating_add(metadata.len());
                    if report.diagnostic_files.len() < options.max_replaced_files {
                        let modified_ns = metadata
                            .modified()
                            .ok()
                            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                            .map(|duration| duration.as_nanos())
                            .unwrap_or_default();
                        report.diagnostic_files.push(ReplacedFileDiagnostic {
                            hidden_root: layer.root.display().to_string(),
                            replaced_by: replaced_by.display().to_string(),
                            replace_path: replace_path.clone(),
                            relative_path: relative_slash_path(&layer.root, &file),
                            bytes: metadata.len(),
                            modified_ns,
                            content_hash: format!("{:016x}", stable_text_hash(&bytes)),
                        });
                    }
                }
            }
        }
        Ok(report)
    }
}

fn collect_visible_files_inner(
    plan: &LayeredSourcePlan,
    layer_index: usize,
    physical_base: &Path,
    virtual_base: &str,
    directory: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("read dir {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let suffix = relative_slash_path(physical_base, &path);
        let virtual_relative = if suffix.is_empty() {
            virtual_base.to_string()
        } else {
            format!("{virtual_base}/{suffix}")
        };
        if !plan.is_visible(layer_index, &virtual_relative) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read type {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_visible_files_inner(
                plan,
                layer_index,
                physical_base,
                virtual_base,
                &path,
                out,
            )?;
        } else if file_type.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

fn replace_paths_for_root(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut paths = BTreeSet::new();
    let descriptor = root.join("descriptor.mod");
    if descriptor.is_file() {
        collect_replace_paths(&read_utf8_lossy(&descriptor)?, &mut paths);
    }
    let Some(parent) = root.parent() else {
        return Ok(paths);
    };
    let root_key = canonical_layer_key(root);
    for entry in fs::read_dir(parent)
        .map_err(|error| format!("read launcher directory {}: {error}", parent.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let launcher = entry.path();
        if !launcher.is_file()
            || !launcher
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mod"))
        {
            continue;
        }
        // Launcher directories can contain stale or temporarily locked .mod
        // files unrelated to this root. They must not make every index build
        // fail; only a readable descriptor whose `path` matches is relevant.
        let Ok(text) = read_utf8_lossy(&launcher) else {
            continue;
        };
        let Some(raw_path) = descriptor_scalar_value(&text, "path") else {
            continue;
        };
        let path = PathBuf::from(raw_path.replace('/', "\\"));
        let resolved = if path.is_absolute() {
            path
        } else {
            launcher.parent().unwrap_or(parent).join(path)
        };
        if canonical_layer_key(&resolved) == root_key {
            collect_replace_paths(&text, &mut paths);
        }
    }
    Ok(paths)
}

fn collect_replace_paths(text: &str, out: &mut BTreeSet<String>) {
    for line in text.lines() {
        let line = line
            .trim_start_matches('\u{feff}')
            .split('#')
            .next()
            .unwrap_or("")
            .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(value) = find_assignment_in_text(line, "replace_path") {
            let normalized = normalize_replace_path(value);
            if !normalized.is_empty() && !normalized.split('/').any(|part| part == "..") {
                out.insert(normalized);
            }
        }
    }
}

pub(crate) fn normalize_replace_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
        .to_ascii_lowercase()
}

fn path_is_within(relative: &str, mask: &str) -> bool {
    relative == mask
        || relative
            .strip_prefix(mask)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn minimal_masks(masks: &BTreeMap<String, PathBuf>) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    for (path, owner) in masks {
        if masks
            .keys()
            .any(|other| other != path && path_is_within(path, other))
        {
            continue;
        }
        out.push((path.clone(), owner.clone()));
    }
    out
}

fn dedupe_layer_roots(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !out
            .iter()
            .any(|existing| layered_paths_equal(existing, root))
        {
            out.push(root.clone());
        }
    }
    out
}

fn layered_paths_equal(left: &Path, right: &Path) -> bool {
    canonical_layer_key(left) == canonical_layer_key(right)
}

fn canonical_layer_key(path: &Path) -> String {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    slash_path(&path).trim_end_matches('/').to_ascii_lowercase()
}

fn stable_text_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn layered_scan_report_json(report: &LayeredScanReport) -> String {
    let declarations = report
        .declarations
        .iter()
        .map(|entry| {
            format!(
                "{{\"root\": {}, \"path\": {}}}",
                json_str(&entry.root),
                json_str(&entry.path)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let files = report
        .diagnostic_files
        .iter()
        .map(|file| {
            format!(
                "{{\"hidden_root\": {}, \"replaced_by\": {}, \"replace_path\": {}, \"relative_path\": {}, \"bytes\": {}, \"modified_ns\": {}, \"content_hash\": {}}}",
                json_str(&file.hidden_root),
                json_str(&file.replaced_by),
                json_str(&file.replace_path),
                json_str(&file.relative_path),
                file.bytes,
                file.modified_ns,
                json_str(&file.content_hash)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n      ");
    format!(
        "{{\"mode\": {}, \"effective_index_includes_replaced_files\": false, \"declarations\": [{}], \"pruned_subtrees\": {}, \"diagnostic_files_total\": {}, \"diagnostic_bytes_total\": {}, \"diagnostic_files\": [\n      {}\n    ]}}",
        json_str(&report.mode),
        declarations,
        report.pruned_subtrees,
        report.diagnostic_files_total,
        report.diagnostic_bytes_total,
        files
    )
}
