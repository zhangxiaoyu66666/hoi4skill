//! Persistent per-file GFX audit fragments for real changed-only analysis.

use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const GFX_MANIFEST_SCHEMA: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct GfxFileStamp {
    len: u64,
    modified_ns: u128,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct GfxFileFragment {
    pub(crate) sprites: Vec<(String, String)>,
    pub(crate) raw_names: BTreeSet<String>,
    pub(crate) refs: BTreeSet<String>,
}

#[derive(Default)]
pub(crate) struct GfxFragmentSet {
    pub(crate) files: BTreeMap<String, GfxFileFragment>,
    pub(crate) image_files_total: usize,
    pub(crate) changed_images: BTreeSet<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedGfxFileFragment {
    stamp: GfxFileStamp,
    fragment: GfxFileFragment,
}

#[derive(Default, Serialize, Deserialize)]
struct GfxManifest {
    schema: u32,
    root: String,
    files: BTreeMap<String, CachedGfxFileFragment>,
    image_files_total: usize,
    image_path_hashes: BTreeSet<u64>,
}

pub(crate) fn load_gfx_fragments(root: &Path, files: &[PathBuf]) -> Result<GfxFragmentSet, String> {
    let path = gfx_manifest_path(root);
    let root_key = slash_path(root);
    let previous_manifest = read_manifest(&path)
        .filter(|manifest| manifest.schema == GFX_MANIFEST_SCHEMA && manifest.root == root_key);
    let mut changed = previous_manifest.is_none();
    let mut previous = previous_manifest
        .map(|manifest| manifest.files)
        .unwrap_or_default();
    let mut next = BTreeMap::new();
    let mut image_files_total = 0usize;
    let mut image_path_hashes = BTreeSet::new();
    for file in files {
        let relative = rel_slash(root, file);
        if is_gfx_image_path(&relative) {
            image_files_total += 1;
            image_path_hashes.insert(stable_gfx_path_hash(&relative));
            continue;
        }
        if !is_gfx_fragment_source(file, &relative) {
            continue;
        }
        let stamp = gfx_file_stamp(file)?;
        let entry = if let Some(entry) = previous
            .remove(&relative)
            .filter(|entry| entry.stamp == stamp)
        {
            entry
        } else {
            changed = true;
            CachedGfxFileFragment {
                stamp,
                fragment: parse_gfx_file_fragment(root, file)?,
            }
        };
        if !entry.fragment.sprites.is_empty() || !entry.fragment.refs.is_empty() {
            next.insert(relative, entry);
        }
    }
    changed |= !previous.is_empty();
    if changed {
        let manifest = GfxManifest {
            schema: GFX_MANIFEST_SCHEMA,
            root: root_key,
            files: next,
            image_files_total,
            image_path_hashes,
        };
        let _ = write_manifest_atomic(&path, &manifest);
        return Ok(fragment_set_from_manifest(manifest));
    }
    Ok(GfxFragmentSet {
        files: next
            .into_iter()
            .map(|(relative, entry)| (relative, entry.fragment))
            .collect(),
        image_files_total,
        changed_images: BTreeSet::new(),
    })
}

pub(crate) fn load_changed_gfx_fragments(
    root: &Path,
    changed_files: &[String],
) -> Result<GfxFragmentSet, String> {
    let path = gfx_manifest_path(root);
    let root_key = slash_path(root);
    let mut manifest = read_manifest(&path)
        .filter(|manifest| manifest.schema == GFX_MANIFEST_SCHEMA && manifest.root == root_key)
        .unwrap_or_default();
    if manifest.files.is_empty() {
        let files = collect_files(root)?;
        return load_gfx_fragments(root, &files);
    }
    let mut changed = false;
    let mut changed_images = BTreeSet::new();
    for relative in changed_files {
        let normalized = slash_path(Path::new(relative));
        let file = root.join(normalized.replace('/', "\\"));
        if is_gfx_image_path(&normalized) {
            let image_hash = stable_gfx_path_hash(&normalized);
            let existed = manifest.image_path_hashes.contains(&image_hash);
            let exists = file.is_file();
            if exists && !existed {
                manifest.image_path_hashes.insert(image_hash);
                manifest.image_files_total += 1;
                changed = true;
            } else if !exists && existed {
                manifest.image_path_hashes.remove(&image_hash);
                manifest.image_files_total = manifest.image_files_total.saturating_sub(1);
                changed = true;
            }
            changed_images.insert(normalized);
            continue;
        }
        if !is_gfx_fragment_source(&file, &normalized) {
            continue;
        }
        if file.is_file() {
            let stamp = gfx_file_stamp(&file)?;
            let needs_refresh = manifest
                .files
                .get(&normalized)
                .is_none_or(|entry| entry.stamp != stamp);
            if needs_refresh {
                manifest.files.insert(
                    normalized,
                    CachedGfxFileFragment {
                        stamp,
                        fragment: parse_gfx_file_fragment(root, &file)?,
                    },
                );
                changed = true;
            }
        } else {
            changed |= manifest.files.remove(&normalized).is_some();
        }
    }
    if changed {
        let _ = write_manifest_atomic(&path, &manifest);
    }
    let mut set = fragment_set_from_manifest(manifest);
    set.changed_images = changed_images;
    Ok(set)
}

fn fragment_set_from_manifest(manifest: GfxManifest) -> GfxFragmentSet {
    GfxFragmentSet {
        files: manifest
            .files
            .into_iter()
            .map(|(relative, entry)| (relative, entry.fragment))
            .collect(),
        image_files_total: manifest.image_files_total,
        changed_images: BTreeSet::new(),
    }
}

fn is_gfx_fragment_source(file: &Path, relative: &str) -> bool {
    let ext = file
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    (relative.starts_with("interface/") && ext == "gfx")
        || matches!(ext.as_str(), "txt" | "gui" | "asset")
}

fn is_gfx_image_path(relative: &str) -> bool {
    let ext = Path::new(relative)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    relative.starts_with("gfx/") && matches!(ext.as_str(), "dds" | "png" | "tga")
}

fn parse_gfx_file_fragment(root: &Path, file: &Path) -> Result<GfxFileFragment, String> {
    let relative = rel_slash(root, file);
    let ext = file
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut fragment = GfxFileFragment::default();
    if relative.starts_with("interface/") && ext == "gfx" {
        let text = read_utf8_lossy(file)?;
        fragment.raw_names = raw_gfx_name_assignments(&text);
        for block in named_gfx_type_blocks(&text) {
            let name = block_assignment(&block, "name").unwrap_or_default();
            let texture = gfx_texturefile_assignment(&block).unwrap_or_default();
            if !name.is_empty() || !texture.is_empty() {
                fragment.sprites.push((name, texture));
            }
        }
    } else if matches!(ext.as_str(), "txt" | "gui" | "asset") {
        let text = read_utf8_lossy(file)?;
        if text.contains("GFX_") {
            let cleaned = strip_comments(&text);
            fragment.refs.extend(
                token_candidates(&cleaned)
                    .into_iter()
                    .filter(|token| token.starts_with("GFX_"))
                    .map(str::to_string),
            );
        }
    }
    Ok(fragment)
}

fn gfx_file_stamp(path: &Path) -> Result<GfxFileStamp, String> {
    let metadata =
        fs::metadata(path).map_err(|error| format!("metadata {}: {error}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(GfxFileStamp {
        len: metadata.len(),
        modified_ns,
    })
}

fn gfx_manifest_path(root: &Path) -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hoi4skill")
        .join("cache")
        .join("gfx-manifest-v4");
    base.join(format!(
        "{:016x}.bin",
        stable_gfx_path_hash(&slash_path(root))
    ))
}

fn stable_gfx_path_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.to_ascii_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn read_manifest(path: &Path) -> Option<GfxManifest> {
    let mut file = fs::File::open(path).ok()?;
    bincode::serde::decode_from_std_read(&mut file, bincode::config::standard()).ok()
}

fn write_manifest_atomic(path: &Path, manifest: &GfxManifest) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "GFX manifest path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("create {}: {error}", temporary.display()))?;
    bincode::serde::encode_into_std_write(manifest, &mut file, bincode::config::standard())
        .map_err(|error| format!("serialize {}: {error}", temporary.display()))?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("replace {}: {error}", path.display()))?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "rename {} to {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}
