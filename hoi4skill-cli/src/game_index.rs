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
    pub(crate) idea_pictures: BTreeSet<String>,
    pub(crate) event_pictures: BTreeSet<String>,
    pub(crate) decision_icons: BTreeSet<String>,
    pub(crate) decision_category_pictures: BTreeSet<String>,
    pub(crate) leader_portraits: BTreeSet<String>,
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
    pub(crate) effects: BTreeSet<String>,
    pub(crate) triggers: BTreeSet<String>,
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

pub(crate) fn cmd_code_catalog(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_paths = dependency_mod_roots(&map)?;
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let index = build_game_index_with_mod_paths(&game_root, &mod_paths)?;
    let json = code_catalog_json(&index, max_items);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_check_code_symbol(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_paths = dependency_mod_roots(&map)?;
    let symbol = require_value(&map, "symbol")?;
    let requested_kind = value(&map, "kind");
    let index = build_game_index_with_mod_paths(&game_root, &mod_paths)?;
    let (json, ok) = check_code_symbol_json(&index, &symbol, requested_kind);
    write_or_print(&json, value(&map, "output"))?;
    if ok {
        Ok(())
    } else {
        Err(format!(
            "`{symbol}` was not found in the indexed HOI4 code catalog{}",
            requested_kind
                .map(|kind| format!(" for kind `{kind}`"))
                .unwrap_or_default()
        ))
    }
}

pub(crate) fn build_game_index(game_root: &Path) -> Result<GameIndex, String> {
    build_game_index_with_mod_paths(game_root, &[])
}

pub(crate) fn build_country_tag_index_with_mod_paths(
    game_root: &Path,
    mod_paths: &[PathBuf],
) -> Result<GameIndex, String> {
    let indexed_roots = std::iter::once(game_root.to_path_buf())
        .chain(mod_paths.iter().cloned())
        .collect::<Vec<_>>();
    let mut index = GameIndex {
        game_root: game_root.to_path_buf(),
        indexed_roots: indexed_roots.clone(),
        ..Default::default()
    };
    for root in indexed_roots {
        if !root.is_dir() {
            return Err(format!(
                "{}: indexed root is not a directory",
                root.display()
            ));
        }
        let tag_root = root.join("common").join("country_tags");
        if !tag_root.exists() {
            continue;
        }
        for file in collect_files(&tag_root)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
                continue;
            }
            collect_country_tags(&read_utf8_lossy(&file)?, &mut index.country_tags);
        }
    }
    Ok(index)
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
        } else if ext == "md" && norm.ends_with("/documentation/effects_documentation.md") {
            let text = read_utf8_lossy(&file)?;
            collect_markdown_heading_identifiers(&text, &mut index.effects);
        } else if ext == "md" && norm.ends_with("/documentation/triggers_documentation.md") {
            let text = read_utf8_lossy(&file)?;
            collect_markdown_heading_identifiers(&text, &mut index.triggers);
        } else if ext == "gfx" && norm.contains("/interface/") {
            let text = read_utf8_lossy(&file)?;
            collect_sprite_names(&text, &mut index.sprites);
            collect_focus_goal_icons_from_gfx_file(&file, &text, &mut index.focus_goal_sprites);
            collect_idea_pictures(&text, &mut index.idea_pictures);
            collect_event_pictures(&text, &mut index.event_pictures);
            collect_decision_icons(&text, &mut index.decision_icons);
            collect_decision_category_pictures(&text, &mut index.decision_category_pictures);
            collect_leader_portraits(&text, &mut index.leader_portraits);
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
        if is_identifier_like(key) && !is_documentation_section_heading(key) {
            entries.insert(key.to_string());
        }
    }
}

pub(crate) fn is_documentation_section_heading(key: &str) -> bool {
    matches!(key, "Effects" | "Modifiers" | "Triggers" | "Table")
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

pub(crate) fn collect_idea_pictures(text: &str, pictures: &mut BTreeSet<String>) {
    let mut sprites = BTreeSet::new();
    collect_sprite_names(text, &mut sprites);
    pictures.extend(
        sprites
            .into_iter()
            .filter_map(|sprite| sprite.strip_prefix("GFX_idea_").map(str::to_string)),
    );
}

pub(crate) fn collect_event_pictures(text: &str, pictures: &mut BTreeSet<String>) {
    let mut sprites = BTreeSet::new();
    collect_sprite_names(text, &mut sprites);
    pictures.extend(
        sprites
            .into_iter()
            .filter(|sprite| sprite.starts_with("GFX_report_event_")),
    );
}

pub(crate) fn collect_decision_icons(text: &str, icons: &mut BTreeSet<String>) {
    let mut sprites = BTreeSet::new();
    collect_sprite_names(text, &mut sprites);
    icons.extend(sprites.into_iter().filter_map(|sprite| {
        sprite
            .strip_prefix("GFX_decision_")
            .filter(|name| !name.starts_with("category_"))
            .map(str::to_string)
    }));
}

pub(crate) fn collect_decision_category_pictures(text: &str, pictures: &mut BTreeSet<String>) {
    let mut sprites = BTreeSet::new();
    collect_sprite_names(text, &mut sprites);
    pictures.extend(sprites.into_iter().filter(|sprite| {
        sprite.starts_with("GFX_decision_category_")
            || sprite.starts_with("GFX_decision_cat_picture_")
    }));
}

pub(crate) fn collect_leader_portraits(text: &str, portraits: &mut BTreeSet<String>) {
    let mut sprites = BTreeSet::new();
    collect_sprite_names(text, &mut sprites);
    portraits.extend(
        sprites
            .into_iter()
            .filter(|sprite| sprite.starts_with("GFX_portrait_")),
    );
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
    let idea_pictures = index.idea_pictures.iter().cloned().collect::<Vec<_>>();
    let event_pictures = index.event_pictures.iter().cloned().collect::<Vec<_>>();
    let decision_icons = index.decision_icons.iter().cloned().collect::<Vec<_>>();
    let decision_category_pictures = index
        .decision_category_pictures
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let leader_portraits = index.leader_portraits.iter().cloned().collect::<Vec<_>>();
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
    let effects = index.effects.iter().cloned().collect::<Vec<_>>();
    let triggers = index.triggers.iter().cloned().collect::<Vec<_>>();
    let modifiers = index.modifiers.iter().cloned().collect::<Vec<_>>();
    let state_ids = index.state_ids.iter().copied().collect::<Vec<_>>();
    let province_ids = index.province_ids.iter().copied().collect::<Vec<_>>();
    format!(
        "{{\n  \"game\": {{\"id\": {}, \"display_name\": {}}},\n  \"game_root\": {},\n  \"indexed_roots\": {},\n  \"country_tags\": {},\n  \"state_ids\": {},\n  \"state_names\": {},\n  \"province_ids\": {},\n  \"sprites\": {},\n  \"focus_goal_sprites\": {},\n  \"idea_pictures\": {},\n  \"event_pictures\": {},\n  \"decision_icons\": {},\n  \"decision_category_pictures\": {},\n  \"leader_portraits\": {},\n  \"buildings\": {},\n  \"building_max_levels\": {},\n  \"resources\": {},\n  \"ideologies\": {},\n  \"traits\": {},\n  \"equipment_types\": {},\n  \"technologies\": {},\n  \"technology_categories\": {},\n  \"sub_units\": {},\n  \"wargoal_types\": {},\n  \"effects\": {},\n  \"triggers\": {},\n  \"modifiers\": {},\n  \"counts\": {{\"indexed_roots\": {}, \"country_tags\": {}, \"state_ids\": {}, \"state_names\": {}, \"province_ids\": {}, \"sprites\": {}, \"focus_goal_sprites\": {}, \"idea_pictures\": {}, \"event_pictures\": {}, \"decision_icons\": {}, \"decision_category_pictures\": {}, \"leader_portraits\": {}, \"buildings\": {}, \"building_max_levels\": {}, \"resources\": {}, \"ideologies\": {}, \"traits\": {}, \"equipment_types\": {}, \"technologies\": {}, \"technology_categories\": {}, \"sub_units\": {}, \"wargoal_types\": {}, \"effects\": {}, \"triggers\": {}, \"modifiers\": {}}}\n}}\n",
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
        json_array(&idea_pictures),
        json_array(&event_pictures),
        json_array(&decision_icons),
        json_array(&decision_category_pictures),
        json_array(&leader_portraits),
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
        json_array(&effects),
        json_array(&triggers),
        json_array(&modifiers),
        index.indexed_roots.len(),
        index.country_tags.len(),
        index.state_ids.len(),
        index.state_names.len(),
        index.province_ids.len(),
        index.sprites.len(),
        index.focus_goal_sprites.len(),
        index.idea_pictures.len(),
        index.event_pictures.len(),
        index.decision_icons.len(),
        index.decision_category_pictures.len(),
        index.leader_portraits.len(),
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
        index.effects.len(),
        index.triggers.len(),
        index.modifiers.len()
    )
}

pub(crate) fn code_catalog_json(index: &GameIndex, max_items: usize) -> String {
    let indexed_roots = index
        .indexed_roots
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let categories = [
        code_catalog_category(
            "effects",
            "script_command",
            "Commands allowed in effect contexts such as completion_reward, option, immediate, and complete_effect.",
            &index.effects,
            max_items,
        ),
        code_catalog_category(
            "triggers",
            "script_condition",
            "Conditions allowed in trigger contexts such as available, visible, trigger, and limit.",
            &index.triggers,
            max_items,
        ),
        code_catalog_category(
            "modifiers",
            "modifier",
            "Modifier keys for national spirits, static modifiers, dynamic modifiers, and other verified modifier blocks.",
            &index.modifiers,
            max_items,
        ),
        code_catalog_category(
            "buildings",
            "game_data_id",
            "Building IDs for state-scoped construction effects.",
            &index.buildings,
            max_items,
        ),
        code_catalog_category(
            "resources",
            "game_data_id",
            "Resource IDs for state resource effects.",
            &index.resources,
            max_items,
        ),
        code_catalog_category(
            "equipment_types",
            "game_data_id",
            "Equipment IDs for stockpile, production, and technology effects.",
            &index.equipment_types,
            max_items,
        ),
        code_catalog_category(
            "technologies",
            "game_data_id",
            "Technology IDs for set_technology and prerequisite checks.",
            &index.technologies,
            max_items,
        ),
        code_catalog_category(
            "technology_categories",
            "game_data_id",
            "Technology category IDs for technology definitions and bonuses.",
            &index.technology_categories,
            max_items,
        ),
        code_catalog_category(
            "sub_units",
            "game_data_id",
            "Sub-unit IDs for division templates and unit effects.",
            &index.sub_units,
            max_items,
        ),
        code_catalog_category(
            "wargoal_types",
            "game_data_id",
            "Wargoal type IDs for create_wargoal.",
            &index.wargoal_types,
            max_items,
        ),
        code_catalog_category(
            "country_tags",
            "resource_id",
            "Country tags found in indexed country tag files.",
            &index.country_tags,
            max_items,
        ),
        code_catalog_category(
            "focus_ids",
            "resource_id",
            "National focus IDs already defined in indexed roots.",
            &index.focus_ids,
            max_items,
        ),
        code_catalog_category(
            "focus_goal_sprites",
            "resource_id",
            "Registered focus goal sprite IDs.",
            &index.focus_goal_sprites,
            max_items,
        ),
        code_catalog_category(
            "idea_pictures",
            "resource_id",
            "Bare idea picture names derived from registered GFX_idea_* sprites.",
            &index.idea_pictures,
            max_items,
        ),
        code_catalog_category(
            "event_pictures",
            "resource_id",
            "Registered event picture sprite IDs.",
            &index.event_pictures,
            max_items,
        ),
        code_catalog_category(
            "decision_icons",
            "resource_id",
            "Bare decision icon names derived from registered GFX_decision_* sprites.",
            &index.decision_icons,
            max_items,
        ),
        code_catalog_category(
            "decision_category_pictures",
            "resource_id",
            "Registered decision category picture sprite IDs.",
            &index.decision_category_pictures,
            max_items,
        ),
        code_catalog_category(
            "leader_portraits",
            "resource_id",
            "Registered leader portrait sprite IDs.",
            &index.leader_portraits,
            max_items,
        ),
        code_catalog_i64_category(
            "state_ids",
            "map_id",
            "Indexed state IDs; use plan-history-edit before writing history/state files.",
            &index.state_ids,
            max_items,
        ),
        code_catalog_i64_category(
            "province_ids",
            "map_id",
            "Indexed province IDs; use plan-history-edit before writing capitals, VP, or unit locations.",
            &index.province_ids,
            max_items,
        ),
    ];
    let rules = vec![
        "Treat missing category entries as unknown; do not invent Clausewitz keys or resource IDs.",
        "LLM output should state intent or structured cards; Rust writers assemble final Clausewitz code.",
        "Use effects only in effect contexts, triggers only in trigger contexts, and modifiers inside verified modifier blocks.",
        "Unresolved mapping comments, TODO generated code markers, and placeholder IDs are strict validation errors.",
        "Run validate --game-root <HOI4 root> --strict-code-index or --final-check before accepting generated code.",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.code_catalog.v1\",\n  \"game\": {{\"id\": {}, \"display_name\": {}}},\n  \"game_root\": {},\n  \"indexed_roots\": {},\n  \"max_items_per_category\": {},\n  \"anti_hallucination_rules\": {},\n  \"categories\": [\n    {}\n  ],\n  \"counts\": {{\"categories\": {}, \"effects\": {}, \"triggers\": {}, \"modifiers\": {}, \"resources\": {}, \"resource_ids\": {}, \"map_ids\": {}}}\n}}\n",
        json_str(HOI4_PROFILE.id),
        json_str(HOI4_PROFILE.display_name),
        json_str(&index.game_root.display().to_string()),
        json_array(&indexed_roots),
        max_items,
        json_array(&rules),
        categories.join(",\n    "),
        categories.len(),
        index.effects.len(),
        index.triggers.len(),
        index.modifiers.len(),
        index.buildings.len()
            + index.resources.len()
            + index.equipment_types.len()
            + index.technologies.len()
            + index.technology_categories.len()
            + index.sub_units.len()
            + index.wargoal_types.len(),
        index.country_tags.len()
            + index.focus_ids.len()
            + index.focus_goal_sprites.len()
            + index.idea_pictures.len()
            + index.event_pictures.len()
            + index.decision_icons.len()
            + index.decision_category_pictures.len()
            + index.leader_portraits.len(),
        index.state_ids.len() + index.province_ids.len()
    )
}

fn code_catalog_category(
    id: &str,
    kind: &str,
    description: &str,
    values: &BTreeSet<String>,
    max_items: usize,
) -> String {
    let items = values.iter().take(max_items).cloned().collect::<Vec<_>>();
    format!(
        "{{\"id\": {}, \"kind\": {}, \"description\": {}, \"count\": {}, \"truncated\": {}, \"items\": {}}}",
        json_str(id),
        json_str(kind),
        json_str(description),
        values.len(),
        json_bool(values.len() > items.len()),
        json_array(&items)
    )
}

fn code_catalog_i64_category(
    id: &str,
    kind: &str,
    description: &str,
    values: &BTreeSet<i64>,
    max_items: usize,
) -> String {
    let items = values
        .iter()
        .take(max_items)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    format!(
        "{{\"id\": {}, \"kind\": {}, \"description\": {}, \"count\": {}, \"truncated\": {}, \"items\": {}}}",
        json_str(id),
        json_str(kind),
        json_str(description),
        values.len(),
        json_bool(values.len() > items.len()),
        json_array(&items)
    )
}

#[derive(Clone)]
pub(crate) struct CodeSymbolMatch {
    pub(crate) category: &'static str,
    pub(crate) kind: &'static str,
    pub(crate) symbol: String,
}

pub(crate) fn check_code_symbol_json(
    index: &GameIndex,
    symbol: &str,
    requested_kind: Option<&str>,
) -> (String, bool) {
    let matches = code_symbol_matches(index, symbol, requested_kind);
    let ok = !matches.is_empty();
    let semantic_candidates = if ok {
        Vec::new()
    } else {
        related_code_symbol_matches(index, symbol, requested_kind, 8)
    };
    let candidates = code_symbol_candidates(symbol)
        .into_iter()
        .collect::<Vec<_>>();
    let match_json = matches
        .iter()
        .map(code_symbol_match_json)
        .collect::<Vec<_>>();
    let semantic_candidate_json = semantic_candidates
        .iter()
        .map(code_symbol_match_json)
        .collect::<Vec<_>>();
    let message = if ok {
        format!(
            "`{}` is indexed as {}.",
            symbol,
            matches
                .iter()
                .map(|item| format!("{}/{}", item.category, item.kind))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!(
            "`{}` is not indexed{}; do not emit it as Clausewitz code until code-catalog evidence exists.",
            symbol,
            requested_kind
                .map(|kind| format!(" as `{kind}`"))
                .unwrap_or_default()
        )
    };
    (
        format!(
            "{{\n  \"schema\": \"hoi4skill.code_symbol_check.v1\",\n  \"ok\": {},\n  \"symbol\": {},\n  \"requested_kind\": {},\n  \"normalized_candidates\": {},\n  \"matches\": {},\n  \"semantic_candidates\": {},\n  \"message\": {},\n  \"anti_hallucination_rule\": {}\n}}\n",
            json_bool(ok),
            json_str(symbol),
            requested_kind.map(json_str).unwrap_or_else(|| "null".to_string()),
            json_array(&candidates),
            format_args!("[{}]", match_json.join(", ")),
            format_args!("[{}]", semantic_candidate_json.join(", ")),
            json_str(&message),
            json_str("If ok is false, fail generation instead of inventing syntax or resources.")
        ),
        ok,
    )
}

pub(crate) fn related_code_symbol_matches(
    index: &GameIndex,
    query: &str,
    requested_kind: Option<&str>,
    limit: usize,
) -> Vec<CodeSymbolMatch> {
    let mut scored = Vec::new();
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "effects",
        "effect",
        &index.effects,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "triggers",
        "trigger",
        &index.triggers,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "modifiers",
        "modifier",
        &index.modifiers,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "buildings",
        "resource_id",
        &index.buildings,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "resources",
        "resource_id",
        &index.resources,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "equipment_types",
        "resource_id",
        &index.equipment_types,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "technologies",
        "resource_id",
        &index.technologies,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "technology_categories",
        "resource_id",
        &index.technology_categories,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "sub_units",
        "resource_id",
        &index.sub_units,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "wargoal_types",
        "resource_id",
        &index.wargoal_types,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "focus_goal_sprites",
        "resource_id",
        &index.focus_goal_sprites,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "idea_pictures",
        "resource_id",
        &index.idea_pictures,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "event_pictures",
        "resource_id",
        &index.event_pictures,
    );
    collect_related_code_symbol_matches(
        &mut scored,
        index,
        query,
        requested_kind,
        "decision_icons",
        "resource_id",
        &index.decision_icons,
    );
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.symbol.cmp(&b.1.symbol)));
    scored.dedup_by(|a, b| a.1.category == b.1.category && a.1.symbol == b.1.symbol);
    scored
        .into_iter()
        .filter(|(score, _)| *score > 0)
        .take(limit)
        .map(|(_, item)| item)
        .collect()
}

fn collect_related_code_symbol_matches(
    out: &mut Vec<(i32, CodeSymbolMatch)>,
    _index: &GameIndex,
    query: &str,
    requested_kind: Option<&str>,
    category: &'static str,
    kind: &'static str,
    values: &BTreeSet<String>,
) {
    if !code_symbol_kind_allowed(requested_kind, category, kind) {
        return;
    }
    for symbol in values {
        let score = related_code_symbol_score(query, symbol);
        if score > 0 {
            out.push((
                score,
                CodeSymbolMatch {
                    category,
                    kind,
                    symbol: symbol.clone(),
                },
            ));
        }
    }
}

pub(crate) fn related_code_symbol_score(query: &str, symbol: &str) -> i32 {
    let query_norm = normalize_code_search_text(query);
    let symbol_norm = normalize_code_search_text(symbol);
    if query_norm.is_empty() || symbol_norm.is_empty() {
        return 0;
    }
    let mut score = 0;
    if query_norm == symbol_norm {
        score += 200;
    } else if symbol_norm.contains(&query_norm) || query_norm.contains(&symbol_norm) {
        score += 90;
    }
    let query_tokens = code_search_tokens(&query_norm);
    let symbol_tokens = code_search_tokens(&symbol_norm);
    for token in &query_tokens {
        if is_generic_code_search_token(token) {
            continue;
        }
        if token.len() >= 3 && symbol_tokens.iter().any(|candidate| candidate == token) {
            score += 20;
        } else if token.len() >= 4
            && symbol_tokens
                .iter()
                .any(|candidate| candidate.contains(token) || token.contains(candidate))
        {
            score += 8;
        }
    }
    for term in semantic_code_search_terms(query) {
        let term_norm = normalize_code_search_text(term);
        if symbol_norm == term_norm {
            score += 120;
        } else if symbol_norm.contains(&term_norm) {
            score += 70;
        }
    }
    let distance = bounded_levenshtein(&query_norm, &symbol_norm, 4);
    if distance <= 4 && query_norm.len().abs_diff(symbol_norm.len()) <= 4 {
        score += 30 - distance as i32 * 5;
    }
    score
}

pub(crate) fn semantic_code_search_terms(query: &str) -> Vec<&'static str> {
    let mut terms = Vec::new();
    if contains_any(
        query,
        &["政治点", "政治力量", "political power", "political_power"],
    ) {
        terms.extend(["add_political_power", "political_power_factor"]);
    }
    if contains_any(query, &["稳定", "stability"]) {
        terms.extend(["add_stability", "stability_factor"]);
    }
    if contains_any(
        query,
        &["战争支持", "战争支援", "war support", "war_support"],
    ) {
        terms.extend(["add_war_support", "war_support_factor"]);
    }
    if contains_any(
        query,
        &["战争正当化", "正当化战争", "war goal", "wargoal", "justify"],
    ) {
        terms.extend(["justify_war_goal_time", "create_wargoal"]);
    }
    if contains_any(query, &["民族精神", "idea"]) {
        terms.extend(["add_ideas", "remove_ideas", "has_idea"]);
    }
    if contains_any(query, &["事件", "event"]) {
        terms.extend(["country_event", "news_event"]);
    }
    if contains_any(query, &["战争中", "无战争", "和平", "has war"]) {
        terms.push("has_war");
    }
    if contains_any(query, &["工厂", "军工", "民工", "building", "factory"]) {
        terms.extend([
            "add_building_construction",
            "arms_factory",
            "industrial_complex",
        ]);
    }
    terms
}

fn normalize_code_search_text(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '_' || ch == '-' || ch.is_whitespace() {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn code_search_tokens(value: &str) -> Vec<&str> {
    value.split('_').filter(|token| !token.is_empty()).collect()
}

fn is_generic_code_search_token(token: &str) -> bool {
    matches!(
        token,
        "add" | "set" | "remove" | "has" | "is" | "get" | "create" | "delete"
    )
}

fn bounded_levenshtein(a: &str, b: &str, max_distance: usize) -> usize {
    if a.len().abs_diff(b.len()) > max_distance {
        return max_distance + 1;
    }
    let mut prev = (0..=b.len()).collect::<Vec<_>>();
    let mut curr = vec![0; b.len() + 1];
    for (i, ca) in a.bytes().enumerate() {
        curr[0] = i + 1;
        let mut row_min = curr[0];
        for (j, cb) in b.bytes().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            row_min = row_min.min(curr[j + 1]);
        }
        if row_min > max_distance {
            return max_distance + 1;
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

pub(crate) fn code_symbol_matches(
    index: &GameIndex,
    symbol: &str,
    requested_kind: Option<&str>,
) -> Vec<CodeSymbolMatch> {
    let candidates = code_symbol_candidates(symbol);
    let mut matches = Vec::new();
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "effects",
        "effect",
        &index.effects,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "triggers",
        "trigger",
        &index.triggers,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "modifiers",
        "modifier",
        &index.modifiers,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "buildings",
        "resource_id",
        &index.buildings,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "resources",
        "resource_id",
        &index.resources,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "equipment_types",
        "resource_id",
        &index.equipment_types,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "technologies",
        "resource_id",
        &index.technologies,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "technology_categories",
        "resource_id",
        &index.technology_categories,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "sub_units",
        "resource_id",
        &index.sub_units,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "wargoal_types",
        "resource_id",
        &index.wargoal_types,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "country_tags",
        "resource_id",
        &index.country_tags,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "focus_ids",
        "resource_id",
        &index.focus_ids,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "sprites",
        "resource_id",
        &index.sprites,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "focus_goal_sprites",
        "resource_id",
        &index.focus_goal_sprites,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "idea_pictures",
        "resource_id",
        &index.idea_pictures,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "event_pictures",
        "resource_id",
        &index.event_pictures,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "decision_icons",
        "resource_id",
        &index.decision_icons,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "decision_category_pictures",
        "resource_id",
        &index.decision_category_pictures,
    );
    push_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "leader_portraits",
        "resource_id",
        &index.leader_portraits,
    );
    push_i64_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "state_ids",
        "map_id",
        &index.state_ids,
    );
    push_i64_code_symbol_match(
        &mut matches,
        requested_kind,
        &candidates,
        "province_ids",
        "map_id",
        &index.province_ids,
    );
    matches
}

fn code_symbol_candidates(symbol: &str) -> BTreeSet<String> {
    let mut candidates = BTreeSet::new();
    let trimmed = symbol.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return candidates;
    }
    candidates.insert(trimmed.to_string());
    if let Some(rest) = trimmed.strip_prefix("GFX_idea_") {
        candidates.insert(rest.to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("GFX_decision_") {
        candidates.insert(rest.to_string());
    }
    candidates
}

fn push_code_symbol_match(
    matches: &mut Vec<CodeSymbolMatch>,
    requested_kind: Option<&str>,
    candidates: &BTreeSet<String>,
    category: &'static str,
    kind: &'static str,
    values: &BTreeSet<String>,
) {
    if !code_symbol_kind_allowed(requested_kind, category, kind) {
        return;
    }
    for candidate in candidates {
        if values.contains(candidate) {
            matches.push(CodeSymbolMatch {
                category,
                kind,
                symbol: candidate.clone(),
            });
        }
    }
}

fn push_i64_code_symbol_match(
    matches: &mut Vec<CodeSymbolMatch>,
    requested_kind: Option<&str>,
    candidates: &BTreeSet<String>,
    category: &'static str,
    kind: &'static str,
    values: &BTreeSet<i64>,
) {
    if !code_symbol_kind_allowed(requested_kind, category, kind) {
        return;
    }
    for candidate in candidates {
        if let Ok(value) = candidate.parse::<i64>() {
            if values.contains(&value) {
                matches.push(CodeSymbolMatch {
                    category,
                    kind,
                    symbol: value.to_string(),
                });
            }
        }
    }
}

fn code_symbol_kind_allowed(requested_kind: Option<&str>, category: &str, kind: &str) -> bool {
    let Some(requested_kind) = requested_kind else {
        return true;
    };
    let requested = requested_kind.trim().to_ascii_lowercase();
    if requested == kind || requested == category {
        return true;
    }
    matches!(
        (requested.as_str(), category, kind),
        ("script_command", "effects", "effect")
            | ("effect", "effects", "effect")
            | ("script_condition", "triggers", "trigger")
            | ("condition", "triggers", "trigger")
            | ("trigger", "triggers", "trigger")
            | ("modifier", "modifiers", "modifier")
            | ("building", "buildings", "resource_id")
            | ("buildings", "buildings", "resource_id")
            | ("resource", "resources", "resource_id")
            | ("resources", "resources", "resource_id")
            | ("resource_id", _, "resource_id")
            | ("map", _, "map_id")
            | ("map_id", _, "map_id")
    )
}

fn code_symbol_match_json(item: &CodeSymbolMatch) -> String {
    format!(
        "{{\"category\": {}, \"kind\": {}, \"symbol\": {}}}",
        json_str(item.category),
        json_str(item.kind),
        json_str(&item.symbol)
    )
}
