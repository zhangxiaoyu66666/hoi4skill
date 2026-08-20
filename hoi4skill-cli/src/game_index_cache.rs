//! Versioned persistent cache for parsed HOI4 game-index snapshots.

use crate::*;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const GAME_INDEX_CACHE_SCHEMA: u32 = 21;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct GameIndexFileStamp {
    len: u64,
    modified_ns: u128,
}

#[derive(Serialize, Deserialize)]
struct GameIndexRootCache {
    schema: u32,
    profile: GameIndexProfile,
    root: String,
    visibility_fingerprint: String,
    files: BTreeMap<String, GameIndexFileStamp>,
    index: GameIndex,
}

pub(crate) fn cached_game_index_layer_snapshot(
    plan: &LayeredSourcePlan,
    layer_index: usize,
    profile: GameIndexProfile,
    scan_options: LayeredScanOptions,
) -> Result<GameIndex, String> {
    let root = &plan.layers()[layer_index].root;
    if !scan_options.use_index_cache
        || profile == GameIndexProfile::Full
        || scan_options.replace_path_diagnostics
    {
        let files = collect_game_index_files_for_layer(plan, layer_index, profile)?;
        return collect_game_index_files_parallel(root, &files, profile);
    }
    let files = collect_game_index_files_for_layer(plan, layer_index, profile)?;
    let stamps = game_index_file_stamps(root, &files)?;
    let cache_path = game_index_cache_path(root, profile)?;
    let root_key = slash_path(root);
    let visibility_fingerprint = plan.visibility_fingerprint(layer_index);
    if let Some(cache) = read_cache(&cache_path).filter(|cache| {
        cache.schema == GAME_INDEX_CACHE_SCHEMA
            && cache.profile == profile
            && cache.root == root_key
            && cache.visibility_fingerprint == visibility_fingerprint
            && cache.files == stamps
    }) {
        return Ok(cache.index);
    }

    let snapshot = collect_game_index_files_parallel(root, &files, profile)?;
    let cache = GameIndexRootCache {
        schema: GAME_INDEX_CACHE_SCHEMA,
        profile,
        root: root_key,
        visibility_fingerprint,
        files: stamps,
        index: snapshot,
    };
    // The cache is advisory: a locked or read-only cache directory must never
    // make indexing or strict validation fail after fresh parsing succeeded.
    let _ = write_cache_atomic(&cache_path, &cache);
    Ok(cache.index)
}

fn collect_game_index_files_parallel(
    root: &Path,
    files: &[PathBuf],
    profile: GameIndexProfile,
) -> Result<GameIndex, String> {
    let files_per_chunk = files_per_cpu_chunk(files.len());
    let partials = files
        .par_chunks(files_per_chunk)
        .map(|chunk| {
            let mut partial = GameIndex::default();
            for file in chunk {
                collect_game_index_file(&mut partial, root, file, profile)?;
            }
            Ok(partial)
        })
        .collect::<Vec<Result<GameIndex, String>>>();
    let mut snapshot = GameIndex::default();
    for partial in partials {
        merge_game_index(&mut snapshot, &partial?);
    }
    Ok(snapshot)
}

fn game_index_file_stamps(
    root: &Path,
    files: &[PathBuf],
) -> Result<BTreeMap<String, GameIndexFileStamp>, String> {
    files
        .par_iter()
        .map(|file| {
            let metadata = fs::metadata(file)
                .map_err(|error| format!("metadata {}: {error}", file.display()))?;
            let modified_ns = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            Ok((
                relative_slash_path(root, file),
                GameIndexFileStamp {
                    len: metadata.len(),
                    modified_ns,
                },
            ))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()
}

pub(crate) fn game_index_cache_path(
    root: &Path,
    profile: GameIndexProfile,
) -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hoi4skill")
        .join("cache")
        .join("game-index-v21");
    let key = stable_path_hash(&slash_path(root));
    let profile = match profile {
        GameIndexProfile::Full => "full",
        GameIndexProfile::CodeCatalog => "code-catalog",
        GameIndexProfile::ClausewitzReference => "reference",
        GameIndexProfile::Validation => "validation",
    };
    Ok(base.join(format!("{key:016x}-{profile}.bin")))
}

fn stable_path_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.to_ascii_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn read_cache(path: &Path) -> Option<GameIndexRootCache> {
    let mut file = fs::File::open(path).ok()?;
    bincode::serde::decode_from_std_read(&mut file, bincode::config::standard()).ok()
}

fn write_cache_atomic(path: &Path, cache: &GameIndexRootCache) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("cache path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| format!("create {}: {error}", temporary.display()))?;
        bincode::serde::encode_into_std_write(cache, &mut file, bincode::config::standard())
            .map_err(|error| format!("serialize {}: {error}", temporary.display()))?;
    }
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

pub(crate) fn merge_game_index(target: &mut GameIndex, source: &GameIndex) {
    target
        .country_tags
        .extend(source.country_tags.iter().cloned());
    merge_set_map(&mut target.country_name_tags, &source.country_name_tags);
    merge_set_map(
        &mut target.localisation_icon_names,
        &source.localisation_icon_names,
    );
    target.focus_ids.extend(source.focus_ids.iter().cloned());
    target.state_ids.extend(source.state_ids.iter().copied());
    target.state_names.extend(source.state_names.clone());
    target
        .province_ids
        .extend(source.province_ids.iter().copied());
    target.sprites.extend(source.sprites.iter().cloned());
    target
        .raw_gfx_names
        .extend(source.raw_gfx_names.iter().cloned());
    target
        .focus_goal_sprites
        .extend(source.focus_goal_sprites.iter().cloned());
    target
        .idea_pictures
        .extend(source.idea_pictures.iter().cloned());
    target
        .event_pictures
        .extend(source.event_pictures.iter().cloned());
    target
        .decision_icons
        .extend(source.decision_icons.iter().cloned());
    target
        .decision_categories
        .extend(source.decision_categories.iter().cloned());
    target
        .decision_category_pictures
        .extend(source.decision_category_pictures.iter().cloned());
    target
        .leader_portraits
        .extend(source.leader_portraits.iter().cloned());
    target.buildings.extend(source.buildings.iter().cloned());
    target
        .building_max_levels
        .extend(source.building_max_levels.clone());
    target.resources.extend(source.resources.iter().cloned());
    target.ideologies.extend(source.ideologies.iter().cloned());
    target
        .ideology_types
        .extend(source.ideology_types.iter().cloned());
    target.characters.extend(source.characters.iter().cloned());
    target.traits.extend(source.traits.iter().cloned());
    target
        .equipment_types
        .extend(source.equipment_types.iter().cloned());
    target
        .equipment_duplicate_archetypes
        .extend(source.equipment_duplicate_archetypes.clone());
    target
        .technologies
        .extend(source.technologies.iter().cloned());
    target
        .technology_categories
        .extend(source.technology_categories.iter().cloned());
    target.sub_units.extend(source.sub_units.iter().cloned());
    target
        .wargoal_types
        .extend(source.wargoal_types.iter().cloned());
    target.effects.extend(source.effects.iter().cloned());
    target
        .documented_effect_parameters
        .extend(source.documented_effect_parameters.iter().cloned());
    target.triggers.extend(source.triggers.iter().cloned());
    target
        .documented_dynamic_variables
        .extend(source.documented_dynamic_variables.iter().cloned());
    target.modifiers.extend(source.modifiers.iter().cloned());
    target
        .observed_modifiers
        .extend(source.observed_modifiers.iter().cloned());
    target.ideas.extend(source.ideas.iter().cloned());
    merge_set_map(&mut target.idea_names, &source.idea_names);
    target
        .dynamic_modifiers
        .extend(source.dynamic_modifiers.iter().cloned());
    target
        .dynamic_modifier_variables
        .extend(source.dynamic_modifier_variables.iter().cloned());
    merge_set_map(
        &mut target.dynamic_modifier_names,
        &source.dynamic_modifier_names,
    );
    merge_set_map(
        &mut target.dynamic_modifier_effect_tooltips,
        &source.dynamic_modifier_effect_tooltips,
    );
    target
        .localisation_entries
        .extend(source.localisation_entries.clone());
    merge_set_map(
        &mut target.localisation_entry_aliases,
        &source.localisation_entry_aliases,
    );
}

pub(crate) fn merge_game_index_layer(
    target: &mut GameIndex,
    source: &GameIndex,
    layer_index: usize,
) {
    merge_game_index(target, source);
    if layer_index == 0 {
        target
            .modifiers
            .extend(source.observed_modifiers.iter().cloned());
    }
}

fn merge_set_map(
    target: &mut BTreeMap<String, BTreeSet<String>>,
    source: &BTreeMap<String, BTreeSet<String>>,
) {
    for (key, values) in source {
        target
            .entry(key.clone())
            .or_default()
            .extend(values.iter().cloned());
    }
}
