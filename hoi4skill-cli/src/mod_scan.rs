//! Mod style and knowledge-base summarization.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_scan_mod_style(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let input = normalize_path(&input)?;
    let options = ModStyleScanOptions {
        max_sprites: parse_usize_option(&map, "max-sprites", 400)?,
        max_non_ascii_paths: parse_usize_option(&map, "max-non-ascii-paths", 80)?,
    };
    let resolved = resolve_mod_root(&input)?;
    let json = scan_mod_style_json(&resolved, &options)?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) struct ModRootResolution {
    pub(crate) input: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) input_kind: String,
}

pub(crate) struct ModStyleScanOptions {
    pub(crate) max_sprites: usize,
    pub(crate) max_non_ascii_paths: usize,
}

#[derive(Default)]
pub(crate) struct EventNamespaceStats {
    pub(crate) files: BTreeSet<String>,
    pub(crate) max_id: Option<i64>,
    pub(crate) country_events: i64,
    pub(crate) news_events: i64,
    pub(crate) state_events: i64,
}

pub(crate) struct FocusTreeStyle {
    pub(crate) file: String,
    pub(crate) tree_id: String,
    pub(crate) country_tag: String,
    pub(crate) focus_count: usize,
}

pub(crate) struct LocalisationFileStyle {
    pub(crate) file: String,
    pub(crate) header: String,
    pub(crate) bom: bool,
    pub(crate) key_count: usize,
    pub(crate) colon_zero_count: usize,
    pub(crate) loose_count: usize,
}

pub(crate) fn resolve_mod_root(input: &Path) -> Result<ModRootResolution, String> {
    if input.is_file() {
        let file_name = input
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if file_name == "descriptor.mod" {
            return Ok(ModRootResolution {
                input: input.to_path_buf(),
                root: input
                    .parent()
                    .ok_or_else(|| format!("{} has no parent directory", input.display()))?
                    .to_path_buf(),
                input_kind: "descriptor".to_string(),
            });
        }
        if input.extension().and_then(OsStr::to_str).unwrap_or("") == "mod" {
            let text = read_utf8_lossy(input)?;
            if let Some(path) = descriptor_scalar_value(&text, "path") {
                let normalized = path.replace('/', "\\");
                let path = PathBuf::from(normalized);
                let root = if path.is_absolute() {
                    path
                } else {
                    input.parent().unwrap_or_else(|| Path::new(".")).join(path)
                };
                return Ok(ModRootResolution {
                    input: input.to_path_buf(),
                    root,
                    input_kind: "launcher".to_string(),
                });
            }
        }
    }
    if input.is_dir() {
        return Ok(ModRootResolution {
            input: input.to_path_buf(),
            root: input.to_path_buf(),
            input_kind: "directory".to_string(),
        });
    }
    Err(format!(
        "{}: expected a mod directory, descriptor.mod, or launcher .mod file",
        input.display()
    ))
}

pub(crate) fn scan_mod_style_json(
    resolved: &ModRootResolution,
    options: &ModStyleScanOptions,
) -> Result<String, String> {
    let root = &resolved.root;
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let files = collect_files(root)?;
    let descriptor_path = root.join("descriptor.mod");
    let descriptor_text = if descriptor_path.exists() {
        Some(read_utf8_lossy(&descriptor_path)?)
    } else {
        None
    };
    let metadata = descriptor_text
        .as_deref()
        .map(scan_descriptor_metadata)
        .unwrap_or_default();
    let dependencies = descriptor_text
        .as_deref()
        .map(|text| descriptor_list_values(text, "dependencies"))
        .unwrap_or_default();
    let tags = descriptor_text
        .as_deref()
        .map(|text| descriptor_list_values(text, "tags"))
        .unwrap_or_default();
    let launcher_files = find_launcher_mod_files(root)?;
    let top_level_entries = scan_top_level_entries(root)?;
    let common_modules = scan_common_modules(root)?;
    let extension_counts = scan_extension_counts(root, &files);
    let focus_trees = scan_focus_tree_styles(root)?;
    let focus_prefixes = scan_focus_id_prefixes(root)?;
    let focus_icons = scan_focus_icon_counts(root)?;
    let event_namespaces = scan_event_namespace_styles(root)?;
    let localisation_files = scan_localisation_file_styles(root)?;
    let localisation_languages = localisation_language_counts(&localisation_files);
    let sprites = scan_sprite_index(root, options.max_sprites)?;
    let sprite_total = scan_sprites(root)?.len();
    let idea_pictures = scan_assignment_counts_in_dir(root, "common/ideas", "picture")?;
    let decision_categories = scan_decision_categories(root)?;
    let country_tags = scan_country_tag_styles(root)?;
    let history_countries = scan_history_country_styles(root)?;
    let non_ascii_paths = scan_non_ascii_paths(root, &files, options.max_non_ascii_paths);

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&root.display().to_string())
    ));
    out.push_str(&format!(
        "  \"input\": {},\n",
        json_str(&resolved.input.display().to_string())
    ));
    out.push_str(&format!(
        "  \"input_kind\": {},\n",
        json_str(&resolved.input_kind)
    ));
    out.push_str(&format!(
        "  \"descriptor\": {},\n",
        json_str(&descriptor_path.display().to_string())
    ));
    out.push_str(&format!(
        "  \"launcher_mod_files\": {},\n",
        json_array(&launcher_files)
    ));
    out.push_str(&format!("  \"metadata\": {},\n", json_object(&metadata)));
    out.push_str(&format!(
        "  \"dependencies\": {},\n",
        json_array(&dependencies)
    ));
    out.push_str(&format!("  \"tags\": {},\n", json_array(&tags)));
    out.push_str(&format!(
        "  \"top_level_entries\": {},\n",
        json_object(&top_level_entries)
    ));
    out.push_str(&format!(
        "  \"file_extensions\": {},\n",
        json_i64_object(&extension_counts)
    ));
    out.push_str(&format!(
        "  \"common_modules\": {},\n",
        json_i64_object(&common_modules)
    ));
    out.push_str(&format!(
        "  \"focus_trees\": {},\n",
        focus_trees_json(&focus_trees)
    ));
    out.push_str(&format!(
        "  \"focus_id_prefixes\": {},\n",
        json_i64_object(&focus_prefixes)
    ));
    out.push_str(&format!(
        "  \"focus_icons\": {},\n",
        json_i64_object(&focus_icons)
    ));
    out.push_str(&format!(
        "  \"event_namespaces\": {},\n",
        event_namespace_stats_json(&event_namespaces)
    ));
    out.push_str(&format!(
        "  \"localisation_languages\": {},\n",
        json_i64_object(&localisation_languages)
    ));
    out.push_str(&format!(
        "  \"localisation_files\": {},\n",
        localisation_files_json(&localisation_files)
    ));
    out.push_str(&format!("  \"gfx_sprite_count\": {},\n", sprite_total));
    out.push_str(&format!(
        "  \"gfx_sprites_truncated\": {},\n",
        json_bool(sprite_total > options.max_sprites)
    ));
    out.push_str(&format!("  \"gfx_sprites\": {},\n", json_object(&sprites)));
    out.push_str(&format!(
        "  \"idea_pictures\": {},\n",
        json_i64_object(&idea_pictures)
    ));
    out.push_str(&format!(
        "  \"decision_categories\": {},\n",
        json_array(&decision_categories)
    ));
    out.push_str(&format!(
        "  \"country_tags\": {},\n",
        json_array(&country_tags)
    ));
    out.push_str(&format!(
        "  \"history_countries\": {},\n",
        json_array(&history_countries)
    ));
    out.push_str(&format!(
        "  \"non_ascii_paths\": {}\n",
        json_array(&non_ascii_paths)
    ));
    out.push_str("}\n");
    Ok(out)
}

pub(crate) fn cmd_mod_knowledge(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let input = normalize_path(&input)?;
    let max_items = parse_usize_option(&map, "max-items", 80)?;
    let max_sprites = parse_usize_option(&map, "max-sprites", max_items.max(80))?;
    let dependency_roots = dependency_mod_roots(&map)?;
    let resolved = resolve_mod_root(&input)?;
    let json = mod_knowledge_json(&resolved, max_items, max_sprites, &dependency_roots)?;
    write_or_print(&json, value(&map, "output"))
}

pub(crate) struct LauncherModSummary {
    pub(crate) file: String,
    pub(crate) metadata: BTreeMap<String, String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) tags: Vec<String>,
}

pub(crate) struct ModKindSummary {
    pub(crate) kind: String,
    pub(crate) confidence: String,
    pub(crate) reasons: Vec<String>,
}

pub(crate) struct CountryTagMapping {
    pub(crate) file: String,
    pub(crate) tag: String,
    pub(crate) country_file: String,
}

pub(crate) struct CharacterStyle {
    pub(crate) file: String,
    pub(crate) id: String,
    pub(crate) roles: Vec<String>,
    pub(crate) traits: Vec<String>,
}

pub(crate) struct HistoryCharacterUse {
    pub(crate) file: String,
    pub(crate) tag: String,
    pub(crate) recruited_characters: Vec<String>,
    pub(crate) legacy_country_leaders: usize,
}

pub(crate) struct LegacyCountryLeaderStyle {
    pub(crate) file: String,
    pub(crate) name: Option<String>,
    pub(crate) ideology: Option<String>,
    pub(crate) picture: Option<String>,
    pub(crate) traits: Vec<String>,
}

pub(crate) struct CountryCreationSyntaxSummary {
    pub(crate) root: String,
    pub(crate) leader_style: String,
    pub(crate) country_tag_mappings: usize,
    pub(crate) country_definition_files: usize,
    pub(crate) country_leader_traits: usize,
    pub(crate) characters: usize,
    pub(crate) history_character_files: usize,
    pub(crate) legacy_country_leaders: usize,
}

#[derive(Clone)]
pub(crate) struct HistoryStateStyle {
    pub(crate) file: String,
    pub(crate) id: Option<i64>,
    pub(crate) name: Option<String>,
    pub(crate) manpower: Option<i64>,
    pub(crate) state_category: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) controller: Option<String>,
    pub(crate) cores: Vec<String>,
    pub(crate) province_count: usize,
    pub(crate) province_sample: Vec<i64>,
    pub(crate) victory_point_provinces: Vec<i64>,
    pub(crate) buildings: Vec<String>,
    pub(crate) resources: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct ProvinceDefinitionSummary {
    pub(crate) file: String,
    pub(crate) province_count: usize,
    pub(crate) land_count: usize,
    pub(crate) sea_count: usize,
    pub(crate) lake_count: usize,
    pub(crate) unknown_type_count: usize,
    pub(crate) sample_ids: Vec<i64>,
}

pub(crate) fn mod_knowledge_json(
    resolved: &ModRootResolution,
    max_items: usize,
    max_sprites: usize,
    dependency_roots: &[PathBuf],
) -> Result<String, String> {
    let root = &resolved.root;
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let files = collect_files(root)?;
    let descriptor_path = root.join("descriptor.mod");
    let descriptor_text = if descriptor_path.exists() {
        Some(read_utf8_lossy(&descriptor_path)?)
    } else {
        None
    };
    let descriptor_exists = descriptor_text.is_some();
    let metadata = descriptor_text
        .as_deref()
        .map(scan_descriptor_metadata)
        .unwrap_or_default();
    let descriptor_dependencies = descriptor_text
        .as_deref()
        .map(|text| descriptor_list_values(text, "dependencies"))
        .unwrap_or_default();
    let descriptor_tags = descriptor_text
        .as_deref()
        .map(|text| descriptor_list_values(text, "tags"))
        .unwrap_or_default();
    let launcher_mod_files = launcher_mod_summaries(root)?;
    let mut dependencies = descriptor_dependencies.clone();
    for launcher in &launcher_mod_files {
        dependencies.extend(launcher.dependencies.iter().cloned());
    }
    dependencies.sort();
    dependencies.dedup();

    let mod_kind = classify_mod_kind(
        descriptor_exists,
        &dependencies,
        !launcher_mod_files.is_empty(),
    );
    let top_level_entries = scan_top_level_entries(root)?;
    let common_modules = scan_common_modules(root)?;
    let extension_counts = scan_extension_counts(root, &files);
    let focus_trees = scan_focus_tree_styles(root)?;
    let focus_prefixes = scan_focus_id_prefixes(root)?;
    let focus_icons = scan_focus_icon_counts(root)?;
    let event_namespaces = scan_event_namespace_styles(root)?;
    let localisation_files = scan_localisation_file_styles(root)?;
    let localisation_languages = localisation_language_counts(&localisation_files);
    let sprite_total = scan_sprites(root)?.len();
    let sprites = scan_sprite_index(root, max_sprites)?;
    let idea_pictures = scan_assignment_counts_in_dir(root, "common/ideas", "picture")?;
    let decision_categories = scan_decision_categories(root)?;
    let country_tags = scan_country_tag_styles(root)?;
    let history_countries = scan_history_country_styles(root)?;
    let history_state_files = scan_history_state_files(root)?;
    let mut history_states = scan_history_state_styles(root)?;
    let history_states_total = history_states.len();
    let history_state_province_refs_total = history_states
        .iter()
        .map(|state| state.province_count)
        .sum::<usize>();
    history_states.truncate(max_items);
    let province_definitions = scan_province_definitions(root)?;
    let province_definition_ids_total = province_definitions
        .iter()
        .map(|definition| definition.province_count)
        .sum::<usize>();
    let country_tag_mappings = scan_country_tag_mappings(root)?;
    let country_definition_files = scan_country_definition_files(root)?;
    let country_leader_traits = scan_country_leader_traits(root)?;
    let characters = scan_character_styles(root, max_items)?;
    let history_character_uses = scan_history_character_uses(root, max_items)?;
    let legacy_country_leaders = scan_legacy_country_leaders(root, max_items)?;
    let content_files_sample = sample_content_files(root, &files, max_items);
    let dependency_root_strings = dependency_roots
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let country_creation_syntax = make_country_creation_syntax_summary(
        root,
        country_tag_mappings.len(),
        country_definition_files.len(),
        country_leader_traits.len(),
        characters.len(),
        history_character_uses.len(),
        legacy_country_leaders.len(),
    );
    let dependency_country_creation_styles =
        scan_dependency_country_creation_styles(dependency_roots)?;

    let localisation = collect_focus_localisation_map(root)?;
    let mut focuses = import_focuses(root, &localisation)?;
    let mut events = import_events(root, &localisation)?;
    let mut ideas = import_ideas(root, &localisation)?;
    let mut decision_categories_imported = import_decision_categories(root, &localisation)?;
    let mut decisions = import_decisions(root, &localisation)?;

    let focuses_total = focuses.len();
    let events_total = events.len();
    let ideas_total = ideas.len();
    let decision_categories_total = decision_categories_imported.len();
    let decisions_total = decisions.len();
    let localisation_total = localisation.len();

    focuses.truncate(max_items);
    events.truncate(max_items);
    ideas.truncate(max_items);
    decision_categories_imported.truncate(max_items);
    decisions.truncate(max_items);

    let counts_json = format!(
        "{{\"files_total\": {}, \"focus_trees\": {}, \"focuses_total\": {}, \"events_total\": {}, \"ideas_total\": {}, \"decision_categories_total\": {}, \"decisions_total\": {}, \"localisation_keys_total\": {}, \"gfx_sprite_count\": {}, \"dependency_mod_roots\": {}, \"country_tag_mappings\": {}, \"country_definition_files\": {}, \"country_leader_traits\": {}, \"characters_returned\": {}, \"history_character_files_returned\": {}, \"legacy_country_leaders_returned\": {}, \"history_state_files\": {}, \"history_states_total\": {}, \"history_states_returned\": {}, \"history_state_province_refs_total\": {}, \"province_definition_files\": {}, \"province_ids_from_definition_total\": {}}}",
        files.len(),
        focus_trees.len(),
        focuses_total,
        events_total,
        ideas_total,
        decision_categories_total,
        decisions_total,
        localisation_total,
        sprite_total,
        dependency_roots.len(),
        country_tag_mappings.len(),
        country_definition_files.len(),
        country_leader_traits.len(),
        characters.len(),
        history_character_uses.len(),
        legacy_country_leaders.len(),
        history_state_files.len(),
        history_states_total,
        history_states.len(),
        history_state_province_refs_total,
        province_definitions.len(),
        province_definition_ids_total
    );
    let markdown_summary = render_mod_knowledge_markdown(ModKnowledgeMarkdownInput {
        root,
        metadata: &metadata,
        mod_kind: &mod_kind,
        dependencies: &dependencies,
        dependency_roots: &dependency_root_strings,
        country_creation_syntax: &country_creation_syntax,
        dependency_country_creation_styles: &dependency_country_creation_styles,
        focus_trees: &focus_trees,
        focus_prefixes: &focus_prefixes,
        event_namespaces: &event_namespaces,
        country_tags: &country_tags,
        history_countries: &history_countries,
        history_state_files: &history_state_files,
        history_states: &history_states,
        province_definitions: &province_definitions,
        country_tag_mappings: &country_tag_mappings,
        country_definition_files: &country_definition_files,
        country_leader_traits: &country_leader_traits,
        characters: &characters,
        history_character_uses: &history_character_uses,
        legacy_country_leaders: &legacy_country_leaders,
        decision_categories: &decision_categories,
        localisation_languages: &localisation_languages,
        common_modules: &common_modules,
        sprites: &sprites,
        idea_pictures: &idea_pictures,
        content_files_sample: &content_files_sample,
    });

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.mod_knowledge.v1\",\n");
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&root.display().to_string())
    ));
    out.push_str(&format!(
        "  \"input\": {},\n",
        json_str(&resolved.input.display().to_string())
    ));
    out.push_str(&format!(
        "  \"input_kind\": {},\n",
        json_str(&resolved.input_kind)
    ));
    out.push_str(&format!(
        "  \"descriptor\": {{\"path\": {}, \"exists\": {}, \"metadata\": {}, \"dependencies\": {}, \"tags\": {}}},\n",
        json_str(&descriptor_path.display().to_string()),
        json_bool(descriptor_exists),
        json_object(&metadata),
        json_array(&descriptor_dependencies),
        json_array(&descriptor_tags)
    ));
    out.push_str(&format!(
        "  \"launcher_mod_files\": {},\n",
        launcher_mod_summaries_json(&launcher_mod_files)
    ));
    out.push_str(&format!(
        "  \"mod_kind\": {{\"kind\": {}, \"confidence\": {}, \"reasons\": {}}},\n",
        json_str(&mod_kind.kind),
        json_str(&mod_kind.confidence),
        json_array(&mod_kind.reasons)
    ));
    out.push_str(&format!(
        "  \"dependency_names\": {},\n",
        json_array(&dependencies)
    ));
    out.push_str(&format!(
        "  \"dependency_mod_roots\": {},\n",
        json_array(&dependency_root_strings)
    ));
    out.push_str(&format!("  \"counts\": {},\n", counts_json));
    out.push_str(&format!(
        "  \"top_level_entries\": {},\n",
        json_object(&top_level_entries)
    ));
    out.push_str(&format!(
        "  \"file_extensions\": {},\n",
        json_i64_object(&extension_counts)
    ));
    out.push_str(&format!(
        "  \"common_modules\": {},\n",
        json_i64_object(&common_modules)
    ));
    out.push_str("  \"knowledge_base\": {");
    out.push_str(&format!(
        "\"content_files_sample\": {}, ",
        json_array(&content_files_sample)
    ));
    out.push_str(&format!(
        "\"country_tags\": {}, ",
        json_array(&country_tags)
    ));
    out.push_str(&format!(
        "\"history_countries\": {}, ",
        json_array(&history_countries)
    ));
    out.push_str(&format!(
        "\"history_state_files\": {}, ",
        json_array(&history_state_files)
    ));
    out.push_str(&format!(
        "\"history_states\": {}, ",
        history_states_json(&history_states)
    ));
    out.push_str(&format!(
        "\"province_definitions\": {}, ",
        province_definitions_json(&province_definitions)
    ));
    out.push_str(&format!(
        "\"country_creation_syntax\": {}, ",
        country_creation_syntax_json(&country_creation_syntax)
    ));
    out.push_str(&format!(
        "\"dependency_country_creation_styles\": {}, ",
        country_creation_syntax_array_json(&dependency_country_creation_styles)
    ));
    out.push_str(&format!(
        "\"country_tag_mappings\": {}, ",
        country_tag_mappings_json(&country_tag_mappings)
    ));
    out.push_str(&format!(
        "\"country_definition_files\": {}, ",
        json_array(&country_definition_files)
    ));
    out.push_str(&format!(
        "\"country_leader_traits\": {}, ",
        json_array(&country_leader_traits)
    ));
    out.push_str(&format!(
        "\"characters\": {}, ",
        character_styles_json(&characters)
    ));
    out.push_str(&format!(
        "\"history_character_uses\": {}, ",
        history_character_uses_json(&history_character_uses)
    ));
    out.push_str(&format!(
        "\"legacy_country_leaders\": {}, ",
        legacy_country_leaders_json(&legacy_country_leaders)
    ));
    out.push_str(&format!(
        "\"focus_trees\": {}, ",
        focus_trees_json(&focus_trees)
    ));
    out.push_str(&format!(
        "\"focus_id_prefixes\": {}, ",
        json_i64_object(&focus_prefixes)
    ));
    out.push_str(&format!(
        "\"focus_icons\": {}, ",
        json_i64_object(&focus_icons)
    ));
    out.push_str(&format!(
        "\"event_namespaces\": {}, ",
        event_namespace_stats_json(&event_namespaces)
    ));
    out.push_str(&format!(
        "\"decision_categories\": {}, ",
        json_array(&decision_categories)
    ));
    out.push_str(&format!(
        "\"idea_pictures\": {}, ",
        json_i64_object(&idea_pictures)
    ));
    out.push_str(&format!(
        "\"localisation_languages\": {}, ",
        json_i64_object(&localisation_languages)
    ));
    out.push_str(&format!(
        "\"gfx_sprites_truncated\": {}, ",
        json_bool(sprite_total > max_sprites)
    ));
    out.push_str(&format!("\"gfx_sprites\": {}, ", json_object(&sprites)));
    out.push_str(&format!(
        "\"content_samples\": {{\"focuses\": {}, \"events\": {}, \"ideas\": {}, \"decision_categories\": {}, \"decisions\": {}}}",
        imported_focuses_json(&focuses),
        imported_events_json(&events),
        imported_ideas_json(&ideas),
        imported_decision_categories_json(&decision_categories_imported),
        imported_decisions_json(&decisions)
    ));
    out.push_str("},\n");
    out.push_str(&format!(
        "  \"anti_hallucination_rules\": {},\n",
        json_array(&mod_knowledge_rules(&mod_kind, &dependencies))
    ));
    out.push_str(&format!(
        "  \"markdown_summary\": {}\n",
        json_str(&markdown_summary)
    ));
    out.push_str("}\n");
    Ok(out)
}

pub(crate) fn launcher_mod_summaries(root: &Path) -> Result<Vec<LauncherModSummary>, String> {
    let launcher_files = find_launcher_mod_files(root)?;
    let mut out = Vec::new();
    for file in launcher_files {
        let path = PathBuf::from(&file);
        let Ok(text) = read_utf8_lossy(&path) else {
            continue;
        };
        out.push(LauncherModSummary {
            file,
            metadata: scan_descriptor_metadata(&text),
            dependencies: descriptor_list_values(&text, "dependencies"),
            tags: descriptor_list_values(&text, "tags"),
        });
    }
    Ok(out)
}

pub(crate) fn launcher_mod_summaries_json(values: &[LauncherModSummary]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|launcher| {
                format!(
                    "{{\"file\": {}, \"metadata\": {}, \"dependencies\": {}, \"tags\": {}}}",
                    json_str(&launcher.file),
                    json_object(&launcher.metadata),
                    json_array(&launcher.dependencies),
                    json_array(&launcher.tags)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn classify_mod_kind(
    descriptor_exists: bool,
    dependencies: &[String],
    has_launcher_mod: bool,
) -> ModKindSummary {
    if !descriptor_exists {
        return ModKindSummary {
            kind: "unknown_no_descriptor".to_string(),
            confidence: "low".to_string(),
            reasons: vec![
                "descriptor.mod was not found in the resolved mod root".to_string(),
                "treat this as unsafe to edit until the real mod root is confirmed".to_string(),
            ],
        };
    }
    if !dependencies.is_empty() {
        return ModKindSummary {
            kind: "submod".to_string(),
            confidence: "high".to_string(),
            reasons: vec![
                "descriptor.mod or launcher .mod declares dependencies".to_string(),
                "generation must distinguish local overrides from dependency-provided content"
                    .to_string(),
            ],
        };
    }
    let mut reasons = vec![
        "descriptor.mod exists and no dependencies were declared".to_string(),
        "treat existing local files as the source of truth before generating new IDs".to_string(),
    ];
    if !has_launcher_mod {
        reasons.push("no launcher-side .mod file was found next to the mod root".to_string());
    }
    ModKindSummary {
        kind: "standalone_mod".to_string(),
        confidence: if has_launcher_mod { "medium" } else { "low" }.to_string(),
        reasons,
    }
}

pub(crate) fn sample_content_files(root: &Path, files: &[PathBuf], limit: usize) -> Vec<String> {
    let mut out = files
        .iter()
        .map(|path| rel_slash(root, path))
        .filter(|path| {
            path == "descriptor.mod"
                || path.starts_with("common/")
                || path.starts_with("events/")
                || path.starts_with("history/")
                || path.starts_with("interface/")
                || path.starts_with("localisation/")
        })
        .collect::<Vec<_>>();
    out.sort();
    out.truncate(limit);
    out
}

pub(crate) fn mod_knowledge_rules(
    mod_kind: &ModKindSummary,
    dependencies: &[String],
) -> Vec<String> {
    let mut rules = vec![
        "Before editing, read this knowledge_base and only use observed tags, namespaces, IDs, sprites, folders, and file conventions unless the user explicitly asks to create new ones.".to_string(),
        "If a fact is absent from the knowledge_base, report it as unknown instead of inventing it.".to_string(),
        "Mod display names belong in descriptor.mod and the launcher-side .mod file; never create *_mod_name localisation keys.".to_string(),
        "Generated content must preserve nearby file style and must not overwrite unrelated existing files.".to_string(),
        "When creating a standalone mod country or country leader, prefer modern common/characters plus history recruit_character unless the user explicitly requests legacy create_country_leader.".to_string(),
        "State and province edits require observed history_states/province_definitions, a build-game-index result, or explicit user-provided IDs; capital uses a province ID, not a state ID.".to_string(),
    ];
    if mod_kind.kind == "submod" {
        rules.push(format!(
            "This is a submod; dependency names are {}. Validate inherited tags, sprites, technologies, and scripted values with --mod-path dependency roots before claiming they exist.",
            if dependencies.is_empty() {
                "unknown".to_string()
            } else {
                dependencies.join(", ")
            }
        ));
        rules.push("When creating or editing country leaders in a submod, follow the indexed dependency mod's observed syntax, including legacy create_country_leader if the dependency uses it; if dependency roots were not indexed, report leader syntax as unknown instead of guessing.".to_string());
        rules.push("If a submod has no local history/states or map/definition.csv facts, index the dependency/game root before using state or province IDs.".to_string());
    }
    rules
}

pub(crate) struct ModKnowledgeMarkdownInput<'a> {
    pub(crate) root: &'a Path,
    pub(crate) metadata: &'a BTreeMap<String, String>,
    pub(crate) mod_kind: &'a ModKindSummary,
    pub(crate) dependencies: &'a [String],
    pub(crate) dependency_roots: &'a [String],
    pub(crate) country_creation_syntax: &'a CountryCreationSyntaxSummary,
    pub(crate) dependency_country_creation_styles: &'a [CountryCreationSyntaxSummary],
    pub(crate) focus_trees: &'a [FocusTreeStyle],
    pub(crate) focus_prefixes: &'a BTreeMap<String, i64>,
    pub(crate) event_namespaces: &'a BTreeMap<String, EventNamespaceStats>,
    pub(crate) country_tags: &'a [String],
    pub(crate) history_countries: &'a [String],
    pub(crate) history_state_files: &'a [String],
    pub(crate) history_states: &'a [HistoryStateStyle],
    pub(crate) province_definitions: &'a [ProvinceDefinitionSummary],
    pub(crate) country_tag_mappings: &'a [CountryTagMapping],
    pub(crate) country_definition_files: &'a [String],
    pub(crate) country_leader_traits: &'a [String],
    pub(crate) characters: &'a [CharacterStyle],
    pub(crate) history_character_uses: &'a [HistoryCharacterUse],
    pub(crate) legacy_country_leaders: &'a [LegacyCountryLeaderStyle],
    pub(crate) decision_categories: &'a [String],
    pub(crate) localisation_languages: &'a BTreeMap<String, i64>,
    pub(crate) common_modules: &'a BTreeMap<String, i64>,
    pub(crate) sprites: &'a BTreeMap<String, String>,
    pub(crate) idea_pictures: &'a BTreeMap<String, i64>,
    pub(crate) content_files_sample: &'a [String],
}

pub(crate) fn render_mod_knowledge_markdown(input: ModKnowledgeMarkdownInput<'_>) -> String {
    let ModKnowledgeMarkdownInput {
        root,
        metadata,
        mod_kind,
        dependencies,
        dependency_roots,
        country_creation_syntax,
        dependency_country_creation_styles,
        focus_trees,
        focus_prefixes,
        event_namespaces,
        country_tags,
        history_countries,
        history_state_files,
        history_states,
        province_definitions,
        country_tag_mappings,
        country_definition_files,
        country_leader_traits,
        characters,
        history_character_uses,
        legacy_country_leaders,
        decision_categories,
        localisation_languages,
        common_modules,
        sprites,
        idea_pictures,
        content_files_sample,
    } = input;
    let name = metadata.get("name").cloned().unwrap_or_else(|| {
        root.file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("mod")
            .to_string()
    });
    let namespace_names = event_namespaces.keys().cloned().collect::<Vec<_>>();
    let focus_tree_lines = focus_trees
        .iter()
        .take(20)
        .map(|tree| {
            format!(
                "{} -> {} / {} / {} focuses",
                tree.file, tree.tree_id, tree.country_tag, tree.focus_count
            )
        })
        .collect::<Vec<_>>();
    let sprite_names = sprites.keys().take(30).cloned().collect::<Vec<_>>();
    let country_mapping_lines = country_tag_mappings
        .iter()
        .take(30)
        .map(|mapping| {
            format!(
                "{} -> {} ({})",
                mapping.tag, mapping.country_file, mapping.file
            )
        })
        .collect::<Vec<_>>();
    let character_lines = characters
        .iter()
        .take(30)
        .map(|character| {
            format!(
                "{}:{} roles=[{}] traits=[{}]",
                character.file,
                character.id,
                character.roles.join(","),
                character.traits.join(",")
            )
        })
        .collect::<Vec<_>>();
    let history_character_lines = history_character_uses
        .iter()
        .take(30)
        .map(|history| {
            format!(
                "{} tag={} recruit=[{}] create_country_leader={}",
                history.file,
                history.tag,
                history.recruited_characters.join(","),
                history.legacy_country_leaders
            )
        })
        .collect::<Vec<_>>();
    let history_state_lines = history_states
        .iter()
        .take(30)
        .map(|state| {
            format!(
                "{} id={} name={} owner={} controller={} cores=[{}] provinces={} sample=[{}] vp=[{}] buildings=[{}] resources=[{}]",
                state.file,
                state.id.map(|id| id.to_string()).unwrap_or_else(|| "unknown".to_string()),
                state.name.as_deref().unwrap_or("unknown"),
                state.owner.as_deref().unwrap_or("unknown"),
                state.controller.as_deref().unwrap_or("unknown"),
                state.cores.join(","),
                state.province_count,
                state
                    .province_sample
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                state
                    .victory_point_provinces
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                state.buildings.join(","),
                state.resources.join(",")
            )
        })
        .collect::<Vec<_>>();
    let province_definition_lines = province_definitions
        .iter()
        .take(10)
        .map(|definition| {
            format!(
                "{} provinces={} land={} sea={} lake={} unknown_type={} sample=[{}]",
                definition.file,
                definition.province_count,
                definition.land_count,
                definition.sea_count,
                definition.lake_count,
                definition.unknown_type_count,
                definition
                    .sample_ids
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>();
    let legacy_leader_lines = legacy_country_leaders
        .iter()
        .take(30)
        .map(|leader| {
            format!(
                "{} name={} ideology={} picture={} traits=[{}]",
                leader.file,
                leader.name.as_deref().unwrap_or("unknown"),
                leader.ideology.as_deref().unwrap_or("unknown"),
                leader.picture.as_deref().unwrap_or("unknown"),
                leader.traits.join(",")
            )
        })
        .collect::<Vec<_>>();
    let dependency_country_style_lines = dependency_country_creation_styles
        .iter()
        .take(20)
        .map(|summary| {
            format!(
                "{} leader_style={} characters={} legacy_country_leaders={}",
                summary.root,
                summary.leader_style,
                summary.characters,
                summary.legacy_country_leaders
            )
        })
        .collect::<Vec<_>>();
    format!(
        "# HOI4 Mod Knowledge Base\n\n- mod_root: {}\n- name: {}\n- mod_kind: {} ({})\n- dependencies: {}\n- dependency_mod_roots: {}\n- country_creation_syntax: {} / tag_mappings={} / common_countries={} / leader_traits={} / characters={} / legacy_country_leaders={}\n- dependency_country_creation_styles: {}\n- country_tags: {}\n- country_tag_mappings: {}\n- country_definition_files: {}\n- history_countries: {}\n- history_state_files: {}\n- history_states: {}\n- province_definitions: {}\n- country_leader_traits: {}\n- characters: {}\n- history_character_uses: {}\n- legacy_country_leaders: {}\n- focus_trees: {}\n- focus_id_prefixes: {}\n- event_namespaces: {}\n- decision_categories: {}\n- idea_pictures: {}\n- localisation_languages: {}\n- common_modules: {}\n- gfx_sprites_sample: {}\n- content_files_sample: {}\n\nHard rules for AI edits:\n- Determine mod_kind before editing. If mod_kind is submod, dependency content must be indexed with dependency paths before claiming a tag, sprite, technology, scripted value, country, country leader syntax, state id, or province id exists.\n- For standalone_mod country creation, use modern common/country_tags + common/countries + history/countries + localisation, and use common/characters plus recruit_character for leaders unless the user explicitly specifies legacy create_country_leader.\n- For submod country creation, follow dependency_country_creation_styles and the dependency's observed syntax; if no dependency root was indexed, report country/leader syntax as unknown instead of guessing.\n- For history/state edits, verify state id, STATE_* name, owner, cores, province list, buildings, and resources through history_states, province_definitions, build-game-index, or explicitly supplied user IDs before writing. capital uses province ID, not state ID.\n- If this mod has no local history/states or map/definition.csv facts, report state/province facts as unknown locally and request a game/dependency index or explicit IDs instead of guessing from Chinese place names.\n- Prefer state-scoped scripted effects for uncertain state edits; do not copy or rewrite vanilla history/states files unless the target state file and province IDs are confirmed.\n- Use only facts listed above or content read from the target files. Unknown facts must be reported as unknown, not invented.\n- Preserve descriptor.mod and launcher .mod as the only place for the mod display name; never generate *_mod_name localisation.\n- Add new IDs in the observed prefix/namespace style and check for collisions against this mod and dependency indexes.\n",
        root.display(),
        name,
        mod_kind.kind,
        mod_kind.confidence,
        list_or_none(dependencies, 20),
        list_or_none(dependency_roots, 20),
        country_creation_syntax.leader_style,
        country_creation_syntax.country_tag_mappings,
        country_creation_syntax.country_definition_files,
        country_creation_syntax.country_leader_traits,
        country_creation_syntax.characters,
        country_creation_syntax.legacy_country_leaders,
        list_or_none(&dependency_country_style_lines, 20),
        list_or_none(country_tags, 30),
        list_or_none(&country_mapping_lines, 30),
        list_or_none(country_definition_files, 30),
        list_or_none(history_countries, 30),
        list_or_none(history_state_files, 30),
        list_or_none(&history_state_lines, 30),
        list_or_none(&province_definition_lines, 10),
        list_or_none(country_leader_traits, 30),
        list_or_none(&character_lines, 30),
        list_or_none(&history_character_lines, 30),
        list_or_none(&legacy_leader_lines, 30),
        list_or_none(&focus_tree_lines, 20),
        list_or_none(&top_i64_entries(focus_prefixes, 20), 20),
        list_or_none(&namespace_names, 30),
        list_or_none(decision_categories, 30),
        list_or_none(&top_i64_entries(idea_pictures, 20), 20),
        list_or_none(&top_i64_entries(localisation_languages, 20), 20),
        list_or_none(&top_i64_entries(common_modules, 30), 30),
        list_or_none(&sprite_names, 30),
        list_or_none(content_files_sample, 40)
    )
}

pub(crate) fn top_i64_entries(values: &BTreeMap<String, i64>, limit: usize) -> Vec<String> {
    let mut pairs = values.iter().collect::<Vec<_>>();
    pairs.sort_by(|(ka, va), (kb, vb)| vb.cmp(va).then(ka.cmp(kb)));
    pairs
        .into_iter()
        .take(limit)
        .map(|(key, value)| format!("{key} ({value})"))
        .collect()
}

pub(crate) fn list_or_none(values: &[String], limit: usize) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values
            .iter()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }
}
