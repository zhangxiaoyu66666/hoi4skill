//! HOI4 game-data indexing used by validation and generation.

#[allow(unused_imports)]
use crate::*;

#[derive(Default, Clone)]
pub(crate) struct GameIndex {
    pub(crate) game_root: PathBuf,
    pub(crate) indexed_roots: Vec<PathBuf>,
    pub(crate) country_tags: BTreeSet<String>,
    pub(crate) focus_ids: BTreeSet<String>,
    pub(crate) state_ids: BTreeSet<i64>,
    pub(crate) state_names: BTreeMap<String, i64>,
    pub(crate) province_ids: BTreeSet<i64>,
    pub(crate) sprites: BTreeSet<String>,
    pub(crate) focus_goal_sprites: BTreeSet<String>,
    pub(crate) buildings: BTreeSet<String>,
    pub(crate) building_max_levels: BTreeMap<String, i64>,
    pub(crate) resources: BTreeSet<String>,
    pub(crate) ideologies: BTreeSet<String>,
    pub(crate) traits: BTreeSet<String>,
    pub(crate) equipment_types: BTreeSet<String>,
    pub(crate) technologies: BTreeSet<String>,
    pub(crate) technology_categories: BTreeSet<String>,
    pub(crate) sub_units: BTreeSet<String>,
    pub(crate) wargoal_types: BTreeSet<String>,
    pub(crate) modifiers: BTreeSet<String>,
}

pub(crate) fn cmd_build_game_index(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_paths = dependency_mod_roots(&map)?;
    let index = if mod_paths.is_empty() {
        build_game_index(&game_root)?
    } else {
        build_game_index_with_mod_paths(&game_root, &mod_paths)?
    };
    let json = game_index_json(&index);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn build_game_index(game_root: &Path) -> Result<GameIndex, String> {
    build_game_index_with_mod_paths(game_root, &[])
}

pub(crate) fn build_game_index_with_mod_paths(
    game_root: &Path,
    mod_paths: &[PathBuf],
) -> Result<GameIndex, String> {
    if !game_root.exists() {
        return Err(format!("{}: game root does not exist", game_root.display()));
    }
    if !game_root.is_dir() {
        return Err(format!(
            "{}: game root is not a directory",
            game_root.display()
        ));
    }

    let mut index = GameIndex {
        game_root: game_root.to_path_buf(),
        indexed_roots: std::iter::once(game_root.to_path_buf())
            .chain(mod_paths.iter().cloned())
            .collect(),
        ..Default::default()
    };
    for root in index.indexed_roots.clone() {
        if !root.exists() {
            return Err(format!("{}: indexed root does not exist", root.display()));
        }
        if !root.is_dir() {
            return Err(format!(
                "{}: indexed root is not a directory",
                root.display()
            ));
        }
        collect_game_index_root(&mut index, &root)?;
    }
    Ok(index)
}

pub(crate) fn collect_game_index_root(index: &mut GameIndex, root: &Path) -> Result<(), String> {
    for file in collect_files(root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let norm = slash_path(&file);
        if ext == "txt" && norm.contains("/common/country_tags/") {
            let text = read_utf8_lossy(&file)?;
            collect_country_tags(&text, &mut index.country_tags);
        } else if ext == "txt" && norm.contains("/common/national_focus/") {
            let text = read_utf8_lossy(&file)?;
            index.focus_ids.extend(focus_tree_existing_ids(&text));
        } else if ext == "txt" && norm.contains("/history/states/") {
            let text = read_utf8_lossy(&file)?;
            collect_state_data(
                &text,
                &mut index.state_ids,
                &mut index.state_names,
                &mut index.province_ids,
            );
        } else if ext == "csv" && norm.ends_with("/map/definition.csv") {
            let text = read_utf8_lossy(&file)?;
            collect_province_ids_from_definition(&text, &mut index.province_ids);
        } else if ext == "txt" && norm.contains("/common/buildings/") {
            let text = read_utf8_lossy(&file)?;
            collect_buildings(&text, &mut index.buildings, &mut index.building_max_levels);
        } else if ext == "txt" && norm.contains("/common/resources/") {
            let text = read_utf8_lossy(&file)?;
            collect_named_entries(&text, &mut index.resources, &["resources"]);
        } else if ext == "txt" && norm.contains("/common/ideologies/") {
            let text = read_utf8_lossy(&file)?;
            collect_named_entries(&text, &mut index.ideologies, &["ideologies", "types"]);
        } else if ext == "txt" && is_trait_definition_path(&norm) {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(
                &text,
                &mut index.traits,
                &[
                    "leader_traits",
                    "country_leader_traits",
                    "unit_leader_traits",
                    "traits",
                ],
            );
        } else if ext == "txt" && norm.contains("/common/units/equipment/") {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(
                &text,
                &mut index.equipment_types,
                &["equipments", "equipment"],
            );
        } else if ext == "txt" && norm.contains("/common/technologies/") {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(&text, &mut index.technologies, &["technologies"]);
            collect_technology_categories(&text, &mut index.technology_categories);
        } else if ext == "txt"
            && norm.contains("/common/units/")
            && !norm.contains("/common/units/equipment/")
        {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(&text, &mut index.sub_units, &["sub_units"]);
        } else if ext == "txt" && norm.contains("/common/wargoals/") {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(&text, &mut index.wargoal_types, &["wargoal_types"]);
        } else if ext == "txt" && is_modifier_definition_path(&norm) {
            let text = read_utf8_lossy(&file)?;
            collect_direct_entries_in_wrappers(
                &text,
                &mut index.modifiers,
                &["modifiers", "static_modifiers", "dynamic_modifiers"],
            );
        } else if ext == "md" && norm.ends_with("/documentation/modifiers_documentation.md") {
            let text = read_utf8_lossy(&file)?;
            collect_markdown_heading_identifiers(&text, &mut index.modifiers);
        } else if ext == "gfx" && norm.contains("/interface/") {
            let text = read_utf8_lossy(&file)?;
            collect_sprite_names(&text, &mut index.sprites);
            collect_focus_goal_icons_from_gfx_file(&file, &text, &mut index.focus_goal_sprites);
        }
    }
    Ok(())
}

pub(crate) fn dependency_mod_roots(map: &ArgMap) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for key in ["mod-path", "dependency-mod", "dependency-mod-path"] {
        for raw in repeated_values(map, key) {
            for path in split_path_option(raw) {
                let path = normalize_path(path)?;
                let resolved = resolve_mod_root(&path)?;
                roots.push(resolved.root);
            }
        }
    }
    Ok(dedupe_paths(roots))
}

pub(crate) fn split_path_option(raw: &str) -> Vec<&str> {
    raw.split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

pub(crate) fn collect_country_tags(text: &str, tags: &mut BTreeSet<String>) {
    for line in strip_comments(text).lines() {
        if let Some(key) = assignment_key(line) {
            if looks_like_tag(key) {
                tags.insert(key.to_string());
            }
        }
    }
}

pub(crate) fn collect_state_data(
    text: &str,
    state_ids: &mut BTreeSet<i64>,
    state_names: &mut BTreeMap<String, i64>,
    province_ids: &mut BTreeSet<i64>,
) {
    for block in blocks_named(text, "state") {
        if let Some(id) = block_assignment(&block, "id").and_then(|s| s.parse::<i64>().ok()) {
            state_ids.insert(id);
            if let Some(name) = block_assignment(&block, "name") {
                state_names.insert(name, id);
            }
            for provinces in blocks_named(&block, "provinces") {
                collect_i64_tokens(&provinces, province_ids);
            }
        }
    }
}

pub(crate) fn collect_province_ids_from_definition(text: &str, province_ids: &mut BTreeSet<i64>) {
    for line in text.lines() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let first = trimmed.split(';').next().unwrap_or("").trim();
        if let Ok(id) = first.parse::<i64>() {
            if id > 0 {
                province_ids.insert(id);
            }
        }
    }
}

pub(crate) fn collect_i64_tokens(text: &str, out: &mut BTreeSet<i64>) {
    for token in token_candidates(text) {
        if let Ok(value) = token.parse::<i64>() {
            out.insert(value);
        }
    }
}

pub(crate) fn collect_buildings(
    text: &str,
    buildings: &mut BTreeSet<String>,
    building_max_levels: &mut BTreeMap<String, i64>,
) {
    let mut found = BTreeSet::new();
    collect_named_entries(text, &mut found, &["buildings"]);
    for building in found {
        buildings.insert(building.clone());
        for block in blocks_named(text, &building) {
            if let Some(max_level) =
                block_assignment(&block, "max_level").and_then(|s| s.parse::<i64>().ok())
            {
                building_max_levels.insert(building.clone(), max_level);
            }
        }
    }
}

pub(crate) fn is_trait_definition_path(norm: &str) -> bool {
    norm.contains("/common/country_leader/")
        || norm.contains("/common/unit_leader/")
        || norm.contains("/common/traits/")
}

pub(crate) fn is_modifier_definition_path(norm: &str) -> bool {
    norm.contains("/common/modifiers/") || norm.contains("/common/dynamic_modifiers/")
}

pub(crate) fn collect_direct_entries_in_wrappers(
    text: &str,
    entries: &mut BTreeSet<String>,
    wrappers: &[&str],
) {
    let cleaned = strip_comments(text);
    for wrapper in wrappers {
        for block in blocks_named(&cleaned, wrapper) {
            for key in direct_block_keys(&block) {
                if !is_common_definition_field(&key) && is_identifier_like(&key) {
                    entries.insert(key);
                }
            }
        }
    }
}

pub(crate) fn collect_markdown_heading_identifiers(text: &str, entries: &mut BTreeSet<String>) {
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("## ") else {
            continue;
        };
        let key = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches('`');
        if is_identifier_like(key) {
            entries.insert(key.to_string());
        }
    }
}

pub(crate) fn collect_technology_categories(text: &str, entries: &mut BTreeSet<String>) {
    for block in blocks_named(&strip_comments(text), "categories") {
        for token in token_candidates(&block) {
            if is_reference_identifier(token) {
                entries.insert(token.to_string());
            }
        }
    }
}

pub(crate) fn collect_named_entries(text: &str, entries: &mut BTreeSet<String>, wrappers: &[&str]) {
    for line in strip_comments(text).lines() {
        if let Some(key) = assignment_key(line) {
            if wrappers.contains(&key) || is_common_definition_field(key) {
                continue;
            }
            if key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            {
                entries.insert(key.to_string());
            }
        }
    }
}

pub(crate) fn is_common_definition_field(key: &str) -> bool {
    matches!(
        key,
        "cost"
            | "value"
            | "icon"
            | "show_on_map"
            | "always_shown"
            | "infrastructure_construction_effect"
            | "base_cost_conversion"
            | "damage"
            | "max_level"
            | "type"
            | "types"
            | "rules"
            | "modifiers"
            | "dynamic_faction_names"
    )
}

pub(crate) fn game_index_json(index: &GameIndex) -> String {
    let indexed_roots = index
        .indexed_roots
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let tags = index.country_tags.iter().cloned().collect::<Vec<_>>();
    let sprites = index.sprites.iter().cloned().collect::<Vec<_>>();
    let focus_goal_sprites = index.focus_goal_sprites.iter().cloned().collect::<Vec<_>>();
    let buildings = index.buildings.iter().cloned().collect::<Vec<_>>();
    let resources = index.resources.iter().cloned().collect::<Vec<_>>();
    let ideologies = index.ideologies.iter().cloned().collect::<Vec<_>>();
    let traits = index.traits.iter().cloned().collect::<Vec<_>>();
    let equipment_types = index.equipment_types.iter().cloned().collect::<Vec<_>>();
    let technologies = index.technologies.iter().cloned().collect::<Vec<_>>();
    let technology_categories = index
        .technology_categories
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let sub_units = index.sub_units.iter().cloned().collect::<Vec<_>>();
    let wargoal_types = index.wargoal_types.iter().cloned().collect::<Vec<_>>();
    let modifiers = index.modifiers.iter().cloned().collect::<Vec<_>>();
    let state_ids = index.state_ids.iter().copied().collect::<Vec<_>>();
    let province_ids = index.province_ids.iter().copied().collect::<Vec<_>>();
    format!(
        "{{\n  \"game\": {{\"id\": {}, \"display_name\": {}}},\n  \"game_root\": {},\n  \"indexed_roots\": {},\n  \"country_tags\": {},\n  \"state_ids\": {},\n  \"state_names\": {},\n  \"province_ids\": {},\n  \"sprites\": {},\n  \"focus_goal_sprites\": {},\n  \"buildings\": {},\n  \"building_max_levels\": {},\n  \"resources\": {},\n  \"ideologies\": {},\n  \"traits\": {},\n  \"equipment_types\": {},\n  \"technologies\": {},\n  \"technology_categories\": {},\n  \"sub_units\": {},\n  \"wargoal_types\": {},\n  \"modifiers\": {},\n  \"counts\": {{\"indexed_roots\": {}, \"country_tags\": {}, \"state_ids\": {}, \"state_names\": {}, \"province_ids\": {}, \"sprites\": {}, \"focus_goal_sprites\": {}, \"buildings\": {}, \"building_max_levels\": {}, \"resources\": {}, \"ideologies\": {}, \"traits\": {}, \"equipment_types\": {}, \"technologies\": {}, \"technology_categories\": {}, \"sub_units\": {}, \"wargoal_types\": {}, \"modifiers\": {}}}\n}}\n",
        json_str(HOI4_PROFILE.id),
        json_str(HOI4_PROFILE.display_name),
        json_str(&index.game_root.display().to_string()),
        json_array(&indexed_roots),
        json_array(&tags),
        json_i64_array(&state_ids),
        json_i64_object(&index.state_names),
        json_i64_array(&province_ids),
        json_array(&sprites),
        json_array(&focus_goal_sprites),
        json_array(&buildings),
        json_i64_object(&index.building_max_levels),
        json_array(&resources),
        json_array(&ideologies),
        json_array(&traits),
        json_array(&equipment_types),
        json_array(&technologies),
        json_array(&technology_categories),
        json_array(&sub_units),
        json_array(&wargoal_types),
        json_array(&modifiers),
        index.indexed_roots.len(),
        index.country_tags.len(),
        index.state_ids.len(),
        index.state_names.len(),
        index.province_ids.len(),
        index.sprites.len(),
        index.focus_goal_sprites.len(),
        index.buildings.len(),
        index.building_max_levels.len(),
        index.resources.len(),
        index.ideologies.len(),
        index.traits.len(),
        index.equipment_types.len(),
        index.technologies.len(),
        index.technology_categories.len(),
        index.sub_units.len(),
        index.wargoal_types.len(),
        index.modifiers.len()
    )
}
