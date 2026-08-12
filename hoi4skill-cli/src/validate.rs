//! Static validation and semantic checks for generated HOI4 mod files.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_validate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = map
        .positionals
        .first()
        .cloned()
        .or_else(|| map.values.get("mod-root").cloned())
        .ok_or_else(|| "missing mod root".to_string())?;
    let root = normalize_path(&root)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &root, game_root.is_some())?;
    let game_index = game_root
        .as_ref()
        .map(|path| {
            build_game_index_with_profile(path, &dependency_mods, GameIndexProfile::Validation)
        })
        .transpose()?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during validation".to_string());
    }
    let options = validation_options_from_args(&map);
    let mut reporter = validate_mod_with_options(&root, game_index.as_ref(), options)?;
    if let Some(request) = value(&map, "request") {
        check_request_scope_for_new_mod(&root, request, &mut reporter);
    }
    check_text_alignment_from_validate_args(&root, &map, &mut reporter)?;
    let report = validation_report_from_args(
        &root,
        &map,
        &reporter,
        game_root.as_deref(),
        &dependency_mods,
    )?;
    report.effective_reporter.print();
    if let Some(output) = value(&map, "output") {
        write_or_print(&validation_report_json(&report), Some(output))?;
    }
    if report.effective_reporter.errors.is_empty() {
        Ok(())
    } else {
        if let (Some(game_root), Some(game_index)) = (game_root.as_deref(), game_index.as_ref()) {
            write_auto_validation_repair_context_artifacts(
                &root,
                game_root,
                &dependency_mods,
                &report,
                game_index,
                &map,
            )?;
        }
        Err("validation failed".to_string())
    }
}

pub(crate) fn cmd_validation_baseline(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = map
        .positionals
        .first()
        .cloned()
        .or_else(|| map.values.get("mod-root").cloned())
        .ok_or_else(|| "missing mod root".to_string())?;
    let root = normalize_path(&root)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &root, game_root.is_some())?;
    let game_index = game_root
        .as_ref()
        .map(|path| {
            build_game_index_with_profile(path, &dependency_mods, GameIndexProfile::Validation)
        })
        .transpose()?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during validation".to_string());
    }
    let options = validation_options_from_args(&map);
    let mut reporter = validate_mod_with_options(&root, game_index.as_ref(), options)?;
    if let Some(request) = value(&map, "request") {
        check_request_scope_for_new_mod(&root, request, &mut reporter);
    }
    check_text_alignment_from_validate_args(&root, &map, &mut reporter)?;
    let report = validation_report_from_args(
        &root,
        &map,
        &reporter,
        game_root.as_deref(),
        &dependency_mods,
    )?;
    let output = value(&map, "output")
        .or_else(|| value(&map, "baseline-output"))
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".hoi4skill").join("validation_baseline.json"));
    let output_string = output.display().to_string();
    write_or_print(&validation_report_json(&report), Some(&output_string))?;
    println!(
        "validation baseline written: {} (errors={}, warnings={})",
        output.display(),
        report.total_errors,
        report.total_warnings
    );
    Ok(())
}

pub(crate) fn cmd_validate_repair_context(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = map
        .positionals
        .first()
        .cloned()
        .or_else(|| map.values.get("mod-root").cloned())
        .ok_or_else(|| "missing mod root".to_string())?;
    let root = normalize_path(&root)?;
    let game_root = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .ok_or_else(|| "validate-repair-context requires --game-root".to_string())?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &root, true)?;
    let game_index =
        build_game_index_with_profile(&game_root, &dependency_mods, GameIndexProfile::Validation)?;
    let mut options = validation_options_from_args(&map);
    options.strict_code_index = true;
    let mut reporter = validate_mod_with_options(&root, Some(&game_index), options)?;
    if let Some(request) = value(&map, "request") {
        check_request_scope_for_new_mod(&root, request, &mut reporter);
    }
    check_text_alignment_from_validate_args(&root, &map, &mut reporter)?;
    let report =
        validation_report_from_args(&root, &map, &reporter, Some(&game_root), &dependency_mods)?;
    let max_items = value(&map, "max-items")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20)
        .max(1);
    let max_examples = value(&map, "max-examples")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let library_path = value(&map, "code-library")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            root.join(".hoi4skill")
                .join("validation_clausewitz_library")
        });
    let (libraries, retrieval_error) = if map.flags.contains("no-code-examples") {
        (Vec::new(), None)
    } else {
        match ensure_clausewitz_libraries(&game_root, &dependency_mods, Some(&library_path)) {
            Ok(libraries) => (libraries, None),
            Err(err) => (Vec::new(), Some(err)),
        }
    };
    let json = validation_repair_context_json(
        &root,
        &game_root,
        &dependency_mods,
        &report,
        &game_index,
        &libraries,
        retrieval_error.as_deref(),
        max_items,
        max_examples,
    );
    if let Some(markdown_output) =
        value(&map, "markdown-output").or_else(|| value(&map, "md-output"))
    {
        let markdown = validation_repair_context_markdown(
            &root,
            &game_root,
            &dependency_mods,
            &report,
            &game_index,
            &libraries,
            retrieval_error.as_deref(),
            max_items,
            max_examples,
        );
        write_or_print(&markdown, Some(markdown_output))?;
    }
    write_or_print(&json, value(&map, "output"))?;
    Ok(())
}

fn write_auto_validation_repair_context_artifacts(
    root: &Path,
    game_root: &Path,
    dependency_mods: &[PathBuf],
    report: &ValidationReport,
    index: &GameIndex,
    map: &ArgMap,
) -> Result<(), String> {
    if map.flags.contains("no-repair-context") || map.flags.contains("skip-repair-context") {
        return Ok(());
    }
    let max_items = value(map, "repair-context-max-items")
        .or_else(|| value(map, "max-items"))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(20)
        .max(1);
    let max_examples = value(map, "repair-context-max-examples")
        .or_else(|| value(map, "max-examples"))
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3);
    let library_path = value(map, "code-library")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            root.join(".hoi4skill")
                .join("validation_clausewitz_library")
        });
    let (libraries, retrieval_error) = if map.flags.contains("no-code-examples") {
        (Vec::new(), None)
    } else {
        match ensure_clausewitz_libraries(game_root, dependency_mods, Some(&library_path)) {
            Ok(libraries) => (libraries, None),
            Err(err) => (Vec::new(), Some(err)),
        }
    };
    let output_dir = root.join(".hoi4skill");
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let json_path = value(map, "repair-context-output")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| output_dir.join("ai_repair_context.json"));
    let markdown_path = value(map, "repair-context-markdown-output")
        .or_else(|| value(map, "repair-context-md-output"))
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| output_dir.join("ai_repair_context.md"));
    let json = validation_repair_context_json(
        root,
        game_root,
        dependency_mods,
        report,
        index,
        &libraries,
        retrieval_error.as_deref(),
        max_items,
        max_examples,
    );
    let markdown = validation_repair_context_markdown(
        root,
        game_root,
        dependency_mods,
        report,
        index,
        &libraries,
        retrieval_error.as_deref(),
        max_items,
        max_examples,
    );
    if let Some(parent) = json_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    if let Some(parent) = markdown_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(&json_path, json).map_err(|e| format!("write {}: {e}", json_path.display()))?;
    fs::write(&markdown_path, markdown)
        .map_err(|e| format!("write {}: {e}", markdown_path.display()))?;
    Ok(())
}

pub(crate) fn validation_options_from_args(map: &ArgMap) -> ValidationOptions {
    ValidationOptions {
        strict_code_index: map.flags.contains("strict-code-index")
            || map.flags.contains("final-check")
            || map.flags.contains("require-code-index"),
    }
}

pub(crate) fn should_run_final_checks(map: &ArgMap) -> bool {
    validation_options_from_args(map).strict_code_index
        || has_text_alignment_args(map)
        || map.flags.contains("check-output")
        || map.flags.contains("check-output-text")
}

pub(crate) fn run_post_apply_checks(
    mod_root: &Path,
    map: &ArgMap,
    game_index: Option<&GameIndex>,
    default_text_source: Option<&Path>,
) -> Result<(), String> {
    if !should_run_final_checks(map) {
        return Ok(());
    }
    let mut reporter =
        validate_mod_with_options(mod_root, game_index, validation_options_from_args(map))?;
    if has_text_alignment_args(map) {
        check_text_alignment_from_validate_args(mod_root, map, &mut reporter)?;
    } else if let Some(input) = default_text_source {
        let tag = value(map, "tag").unwrap_or("TAG");
        let prefix = value(map, "prefix").unwrap_or("mod");
        let expected = expected_texts_from_path(input, value(map, "sheet"), tag, prefix)?;
        for item in text_alignment_report(mod_root, expected)?.missing() {
            reporter.error(format!(
                "text alignment missing user-provided text `{}` from {}",
                item.expected.text, item.expected.source
            ));
        }
    }
    reporter.print();
    if reporter.errors.is_empty() {
        Ok(())
    } else {
        Err(
            "post-apply final checks failed; fix the generated output before claiming completion"
                .to_string(),
        )
    }
}

pub(crate) fn check_request_scope_for_new_mod(root: &Path, request: &str, reporter: &mut Reporter) {
    let scope = requirement_scope_contract(request, false, "TAG", "feature");
    if !scope
        .authorized_systems
        .iter()
        .any(|system| system == "new_mod_descriptor")
    {
        return;
    }

    for (system, paths) in [
        (
            "country_definition",
            &["common/country_tags", "common/countries"][..],
        ),
        ("country_history", &["history/countries"][..]),
        ("state_history", &["history/states"][..]),
        ("initial_units", &["history/units"][..]),
        ("characters", &["common/characters"][..]),
        ("english_localisation", &["localisation/english"][..]),
        ("decisions", &["common/decisions"][..]),
        ("technologies", &["common/technologies"][..]),
        ("custom_gui", &["common/scripted_guis"][..]),
    ] {
        if scope
            .authorized_systems
            .iter()
            .any(|authorized| authorized == system)
        {
            continue;
        }
        for relative in paths {
            let path = root.join(relative);
            if path.exists() {
                reporter.error(format!(
                    "{}: request-scope violation; `{relative}` exists but the literal new-mod request did not authorize `{system}`",
                    path.display()
                ));
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn validate_mod(
    root: &Path,
    game_index: Option<&GameIndex>,
) -> Result<Reporter, String> {
    validate_mod_with_options(root, game_index, ValidationOptions::default())
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ValidationOptions {
    pub(crate) strict_code_index: bool,
}

pub(crate) fn validate_mod_with_options(
    root: &Path,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
) -> Result<Reporter, String> {
    let mut reporter = Reporter::default();
    let base_game_index = game_index;
    let mut effective_game_index = None;
    if let Some(index) = base_game_index {
        let mut index = clone_game_index_for_validation(index);
        if root.is_dir() {
            let root_key = slash_path(root).to_ascii_lowercase();
            if !index
                .indexed_roots
                .iter()
                .any(|indexed| slash_path(indexed).to_ascii_lowercase() == root_key)
            {
                index.indexed_roots.push(root.to_path_buf());
            }
            collect_game_index_root_with_profile(&mut index, root, GameIndexProfile::Validation)?;
        }
        effective_game_index = Some(index);
    }
    let game_index = effective_game_index.as_ref().or(base_game_index);
    let mut ids: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut namespaces: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut localisation_keys: BTreeSet<String> = BTreeSet::new();
    let mut localisation_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut sprite_names: BTreeSet<String> = BTreeSet::new();
    let mut raw_gfx_names: BTreeSet<String> = BTreeSet::new();
    let mut gfx_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut idea_picture_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut event_picture_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut tag_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut focus_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut local_focus_ids: BTreeSet<String> = BTreeSet::new();
    let mut game_data_refs = GameDataRefs::default();
    let mut indexed_history_files = Vec::new();

    if !root.exists() {
        reporter.error(format!("{}: path does not exist", root.display()));
    } else if !root.is_dir() {
        reporter.error(format!("{}: path is not a directory", root.display()));
    } else {
        check_descriptor(root, &mut reporter);
        for file in collect_files(root)? {
            let ext = file
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let norm = slash_path(&file);
            if norm.contains("/history/countries/") || norm.contains("/history/states/") {
                indexed_history_files.push(file.clone());
            }
            if matches!(ext.as_str(), "txt" | "mod" | "gfx" | "gui" | "asset") {
                let text = read_utf8_lossy(&file)?;
                let cleaned = strip_comments(&text);
                check_braces_cleaned(&file, &cleaned, &mut reporter);
                collect_ids_and_namespaces(&file, &text, &mut ids, &mut namespaces, &mut reporter);
                collect_localisation_refs(&file, &text, &mut localisation_refs, &mut reporter);
                if ext == "gfx" && norm.contains("/interface/") {
                    collect_sprite_names(&text, &mut sprite_names);
                    raw_gfx_names.extend(raw_gfx_name_assignments(&text));
                    check_sprite_textures(root, &file, &text, game_index, &mut reporter);
                } else {
                    collect_gfx_refs_cleaned(&file, &cleaned, &mut gfx_refs);
                }
                collect_idea_picture_refs_cleaned(
                    &file,
                    &cleaned,
                    &mut idea_picture_refs,
                    &mut reporter,
                );
                collect_event_picture_refs_cleaned(&file, &cleaned, &mut event_picture_refs);
                collect_country_tag_refs_cleaned(&file, &cleaned, &mut tag_refs);
                collect_focus_refs_cleaned(&file, &cleaned, &mut focus_refs, &mut local_focus_ids);
                collect_game_data_refs_cleaned(&file, &cleaned, &mut game_data_refs);
                check_script_semantics_cleaned(&file, &cleaned, game_index, options, &mut reporter);
                check_unresolved_generation_markers(&file, &text, options, &mut reporter);
            } else if matches!(ext.as_str(), "yml" | "yaml") {
                let text = read_utf8_lossy(&file)?;
                if norm.contains("/localisation/") {
                    check_localisation(&file, &mut reporter);
                    check_indexed_localisation_tokens(
                        &file,
                        &text,
                        game_index,
                        options,
                        &mut reporter,
                    );
                    collect_localisation_keys(&text, &mut localisation_keys);
                } else {
                    check_yaml_duplicate_keys(&file, &text, &mut reporter);
                }
            } else if ext == "jsonl" {
                let text = read_utf8_lossy(&file)?;
                check_jsonl_duplicate_keys(&file, &text, &mut reporter);
            }
        }
    }

    if game_index.is_none() && !indexed_history_files.is_empty() {
        reporter.error(format!(
            "history files require indexed validation with --game-root before completion can be claimed: {}",
            indexed_history_files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if options.strict_code_index && game_index.is_none() {
        reporter.error(
            "strict code index validation requires --game-root, and --mod-path for every dependency mod; final output cannot be accepted without checking generated code against the local HOI4/dependency codebase"
                .to_string(),
        );
    }

    for (id, paths) in ids {
        if paths.len() > 1 {
            reporter.warn(format!(
                "duplicate high-risk id {id}: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    for (ns, paths) in namespaces {
        if paths.len() > 1 {
            reporter.warn(format!(
                "namespace {ns} appears in multiple files: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    for (key, paths) in localisation_refs {
        if !is_known_localisation_key(&key, &localisation_keys, game_index)
            && !is_known_localisation_key(&key, &localisation_keys, base_game_index)
        {
            report_paths(
                &mut reporter,
                game_index.is_some() || options.strict_code_index,
                format!(
                    "localisation key {key} is referenced but not defined in this mod or indexed roots"
                ),
                &paths,
            );
        }
    }
    for (sprite, paths) in gfx_refs {
        if !is_known_sprite_with_options(&sprite, &sprite_names, game_index, options) {
            let classification = if raw_gfx_names.contains(&sprite)
                || game_index.is_some_and(|index| index.raw_gfx_names.contains(&sprite))
            {
                "parser_gap"
            } else {
                "confirmed_missing"
            };
            report_paths(
                &mut reporter,
                game_index.is_some() || options.strict_code_index,
                format!(
                    "[{classification}] GFX key {sprite} is referenced but not defined in this mod or indexed roots"
                ),
                &paths,
            );
        }
    }
    let local_idea_pictures = sprite_names
        .iter()
        .filter_map(|sprite| sprite.strip_prefix("GFX_idea_").map(str::to_string))
        .collect::<BTreeSet<_>>();
    for (picture, paths) in idea_picture_refs {
        let known = local_idea_pictures.contains(&picture)
            || game_index.is_some_and(|index| index.idea_pictures.contains(&picture))
            || (!options.strict_code_index && picture == "generic_production_bonus");
        if !known {
            report_paths(
                &mut reporter,
                game_index.is_some() || options.strict_code_index,
                format!(
                    "idea picture {picture} requires a registered GFX_idea_{picture} sprite in this mod or indexed roots"
                ),
                &paths,
            );
        }
    }
    let local_event_pictures = sprite_names
        .iter()
        .filter(|sprite| sprite.starts_with("GFX_report_event_"))
        .cloned()
        .collect::<BTreeSet<_>>();
    for (picture, paths) in event_picture_refs {
        let known = local_event_pictures.contains(&picture)
            || game_index.is_some_and(|index| index.event_pictures.contains(&picture))
            || (!options.strict_code_index
                && game_index.is_none()
                && picture.starts_with("GFX_report_event_"));
        if !known {
            let related = game_index
                .map(|index| related_code_symbols_text(index, &picture, Some("event_picture")))
                .unwrap_or_default();
            report_paths(
                &mut reporter,
                game_index.is_some() || options.strict_code_index,
                format!(
                    "event picture {picture} requires a registered event picture sprite in this mod or indexed roots{related}"
                ),
                &paths,
            );
        }
    }
    if let Some(index) = game_index {
        for (tag, paths) in tag_refs {
            if !index.country_tags.contains(&tag) && !is_dynamic_tag_ref(&tag) {
                report_paths(
                    &mut reporter,
                    true,
                    format!("country tag {tag} is referenced but not present in game index"),
                    &paths,
                );
            }
        }
        report_unknown_index_refs(
            "building type",
            &game_data_refs.buildings,
            &index.buildings,
            &mut reporter,
            true,
            Some((index, "building")),
        );
        warn_building_levels(&game_data_refs.building_levels, index, &mut reporter);
        report_unknown_index_refs(
            "resource",
            &game_data_refs.resources,
            &index.resources,
            &mut reporter,
            true,
            Some((index, "resource")),
        );
        report_unknown_index_refs(
            "ideology",
            &game_data_refs.ideologies,
            &index.ideologies,
            &mut reporter,
            true,
            None,
        );
        report_unknown_index_refs_if_indexed(
            "trait",
            &game_data_refs.traits,
            &index.traits,
            &mut reporter,
            true,
            options.strict_code_index,
            None,
        );
        report_unknown_index_refs_if_indexed(
            "equipment type",
            &game_data_refs.equipment,
            &index.equipment_types,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "resource_id")),
        );
        report_unknown_index_refs_if_indexed(
            "technology",
            &game_data_refs.technologies,
            &index.technologies,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "resource_id")),
        );
        report_unknown_index_refs_if_indexed(
            "technology category",
            &game_data_refs.technology_categories,
            &index.technology_categories,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "resource_id")),
        );
        report_unknown_index_refs_if_indexed(
            "sub unit",
            &game_data_refs.sub_units,
            &index.sub_units,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "resource_id")),
        );
        report_unknown_index_refs_if_indexed(
            "wargoal type",
            &game_data_refs.wargoal_types,
            &index.wargoal_types,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "resource_id")),
        );
        report_unknown_index_refs_if_indexed(
            "modifier",
            &game_data_refs.modifiers,
            &index.modifiers,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "modifier")),
        );
        report_unknown_index_refs_if_indexed(
            "idea",
            &game_data_refs.ideas,
            &index.ideas,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "idea")),
        );
        report_dynamic_modifiers_used_as_ideas(
            &game_data_refs.ideas,
            index,
            &mut reporter,
            options.strict_code_index,
        );
        report_unknown_index_refs_if_indexed(
            "dynamic modifier",
            &game_data_refs.dynamic_modifiers,
            &index.dynamic_modifiers,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "dynamic_modifier")),
        );
        report_unknown_index_refs_if_indexed(
            "dynamic modifier variable",
            &game_data_refs.dynamic_modifier_variables,
            &index.dynamic_modifier_variables,
            &mut reporter,
            true,
            options.strict_code_index,
            Some((index, "dynamic_modifier_variable")),
        );
    }
    let mut known_focus_ids = local_focus_ids;
    if let Some(index) = game_index {
        known_focus_ids.extend(index.focus_ids.iter().cloned());
    }
    for (focus_id, paths) in focus_refs {
        if !known_focus_ids.contains(&focus_id) {
            report_paths(
                &mut reporter,
                game_index.is_some(),
                format!(
                    "focus id {focus_id} is referenced but not present in the indexed focus trees"
                ),
                &paths,
            );
        }
    }

    Ok(reporter)
}

pub(crate) fn clone_game_index_for_validation(index: &GameIndex) -> GameIndex {
    GameIndex {
        game_root: index.game_root.clone(),
        indexed_roots: index.indexed_roots.clone(),
        country_tags: index.country_tags.clone(),
        focus_ids: index.focus_ids.clone(),
        state_ids: index.state_ids.clone(),
        province_ids: index.province_ids.clone(),
        sprites: index.sprites.clone(),
        raw_gfx_names: index.raw_gfx_names.clone(),
        idea_pictures: index.idea_pictures.clone(),
        event_pictures: index.event_pictures.clone(),
        buildings: index.buildings.clone(),
        building_max_levels: index.building_max_levels.clone(),
        resources: index.resources.clone(),
        ideologies: index.ideologies.clone(),
        traits: index.traits.clone(),
        equipment_types: index.equipment_types.clone(),
        technologies: index.technologies.clone(),
        technology_categories: index.technology_categories.clone(),
        sub_units: index.sub_units.clone(),
        wargoal_types: index.wargoal_types.clone(),
        effects: index.effects.clone(),
        triggers: index.triggers.clone(),
        modifiers: index.modifiers.clone(),
        ideas: index.ideas.clone(),
        dynamic_modifiers: index.dynamic_modifiers.clone(),
        dynamic_modifier_variables: index.dynamic_modifier_variables.clone(),
        ..Default::default()
    }
}

#[derive(Default)]
pub(crate) struct Reporter {
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

struct ValidationReport {
    mod_root: PathBuf,
    game_root: Option<PathBuf>,
    dependency_mods: Vec<PathBuf>,
    strict_code_index: bool,
    total_errors: usize,
    total_warnings: usize,
    baseline_errors: usize,
    baseline_warnings: usize,
    changed_files: Vec<String>,
    effective_reporter: Reporter,
}

fn validation_report_from_args(
    root: &Path,
    map: &ArgMap,
    reporter: &Reporter,
    game_root: Option<&Path>,
    dependency_mods: &[PathBuf],
) -> Result<ValidationReport, String> {
    let baseline = value(map, "baseline")
        .map(normalize_path)
        .transpose()?
        .map(|path| read_validation_baseline(&path))
        .transpose()?;
    let changed_files = validation_changed_files(root, map)?;
    let mut errors = reporter.errors.clone();
    let mut warnings = reporter.warnings.clone();

    let mut baseline_errors = 0;
    let mut baseline_warnings = 0;
    if let Some((baseline_error_set, baseline_warning_set)) = baseline {
        let before = errors.len();
        errors.retain(|error| !baseline_error_set.contains(error));
        baseline_errors = before - errors.len();
        let before = warnings.len();
        warnings.retain(|warning| !baseline_warning_set.contains(warning));
        baseline_warnings = before - warnings.len();
    }

    if map.flags.contains("changed-only") {
        if changed_files.is_empty() {
            return Err("--changed-only requires at least one --changed <path>".to_string());
        }
        errors.retain(|error| validation_message_mentions_changed(root, error, &changed_files));
        warnings
            .retain(|warning| validation_message_mentions_changed(root, warning, &changed_files));
    }

    Ok(ValidationReport {
        mod_root: root.to_path_buf(),
        game_root: game_root.map(Path::to_path_buf),
        dependency_mods: dependency_mods.to_vec(),
        strict_code_index: validation_options_from_args(map).strict_code_index,
        total_errors: reporter.errors.len(),
        total_warnings: reporter.warnings.len(),
        baseline_errors,
        baseline_warnings,
        changed_files,
        effective_reporter: Reporter { errors, warnings },
    })
}

fn validation_report_json(report: &ValidationReport) -> String {
    let dependency_mods = report
        .dependency_mods
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.validation_report.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"dependency_mods\": {},\n  \"strict_code_index\": {},\n  \"total_errors\": {},\n  \"total_warnings\": {},\n  \"effective_errors\": {},\n  \"effective_warnings\": {},\n  \"baseline_errors_filtered\": {},\n  \"baseline_warnings_filtered\": {},\n  \"changed_files\": {},\n  \"error_groups\": {},\n  \"warning_groups\": {},\n  \"errors\": {},\n  \"warnings\": {}\n}}\n",
        json_bool(report.effective_reporter.errors.is_empty()),
        if report.effective_reporter.errors.is_empty() {
            if report.effective_reporter.warnings.is_empty() {
                json_str("ok")
            } else {
                json_str("warnings")
            }
        } else {
            json_str("errors")
        },
        json_str(&report.mod_root.display().to_string()),
        report
            .game_root
            .as_ref()
            .map(|path| json_str(&path.display().to_string()))
            .unwrap_or_else(|| "null".to_string()),
        json_array(&dependency_mods),
        json_bool(report.strict_code_index),
        report.total_errors,
        report.total_warnings,
        report.effective_reporter.errors.len(),
        report.effective_reporter.warnings.len(),
        report.baseline_errors,
        report.baseline_warnings,
        json_array(&report.changed_files),
        validation_issue_groups_json(&report.effective_reporter.errors, 40),
        validation_issue_groups_json(&report.effective_reporter.warnings, 40),
        json_array(&report.effective_reporter.errors),
        json_array(&report.effective_reporter.warnings)
    )
}

fn validation_repair_context_json(
    root: &Path,
    game_root: &Path,
    dependency_mods: &[PathBuf],
    report: &ValidationReport,
    index: &GameIndex,
    libraries: &[PathBuf],
    retrieval_error: Option<&str>,
    max_items: usize,
    max_examples: usize,
) -> String {
    let items = report
        .effective_reporter
        .errors
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(idx, message)| {
            validation_repair_item_json_with_context(
                idx + 1,
                message,
                index,
                Some(root),
                Some(game_root),
                dependency_mods,
                libraries,
                max_examples,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let dependency_strings = dependency_mods
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let dependency_args = validation_repair_dependency_args(dependency_mods);
    let changed_args = validation_repair_changed_args(&report.changed_files);
    let next_commands = vec![
        format!(
            "hoi4skill validate {} --game-root {}{} --strict-code-index{} --output .hoi4skill/validation.json",
            validation_command_path_arg(root),
            validation_command_path_arg(game_root),
            dependency_args,
            changed_args
        ),
        format!(
            "hoi4skill validate-repair-context {} --game-root {}{}{} --output .hoi4skill/ai_repair_context.json",
            validation_command_path_arg(root),
            validation_command_path_arg(game_root),
            dependency_args,
            changed_args
        ),
    ];
    let ai_rules = vec![
        "Treat every repair item as blocking until strict validation passes.".to_string(),
        "Use only indexed candidates, explicit user-provided symbols, or CLI-generated patch plans.".to_string(),
        "If candidates are empty or ambiguous, ask the user or run check-code-symbol/query-clausewitz-library before writing code.".to_string(),
        "Do not delete player-visible user text to hide text-alignment failures.".to_string(),
        "Do not replace dynamic modifiers with national spirits unless the user explicitly changes the design.".to_string(),
    ];
    let retrieval_status = if retrieval_error.is_some() {
        "unavailable"
    } else if libraries.is_empty() {
        "disabled"
    } else {
        "ok"
    };
    format!(
        "{{\n  \"schema\": \"hoi4skill.validation_repair_context.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"dependency_mods\": {},\n  \"changed_files\": {},\n  \"strict_code_index\": true,\n  \"code_example_retrieval\": {{\"status\": {}, \"library_count\": {}, \"error\": {}}},\n  \"effective_errors\": {},\n  \"effective_warnings\": {},\n  \"included_repair_items\": {},\n  \"repair_items\": [\n{}\n  ],\n  \"ai_rules\": {},\n  \"next_commands\": {},\n  \"anti_hallucination_rule\": {}\n}}\n",
        json_bool(report.effective_reporter.errors.is_empty()),
        if report.effective_reporter.errors.is_empty() {
            json_str("ok")
        } else {
            json_str("needs_repair")
        },
        json_str(&root.display().to_string()),
        json_str(&game_root.display().to_string()),
        json_array(&dependency_strings),
        json_array(&report.changed_files),
        json_str(retrieval_status),
        libraries.len(),
        json_optional_str(retrieval_error),
        report.effective_reporter.errors.len(),
        report.effective_reporter.warnings.len(),
        report.effective_reporter.errors.len().min(max_items),
        items,
        json_array(&ai_rules),
        json_array(&next_commands),
        json_str("If a symbol, syntax, scope, picture, idea, or modifier is absent from this context and strict code index, fail or ask instead of inventing Clausewitz code.")
    )
}

fn validation_repair_context_markdown(
    root: &Path,
    game_root: &Path,
    dependency_mods: &[PathBuf],
    report: &ValidationReport,
    index: &GameIndex,
    libraries: &[PathBuf],
    retrieval_error: Option<&str>,
    max_items: usize,
    max_examples: usize,
) -> String {
    let mut out = String::new();
    out.push_str("# Validation Repair Context\n\n");
    out.push_str("- schema: `hoi4skill.validation_repair_context.v1`\n");
    out.push_str(&format!(
        "- status: `{}`\n",
        if report.effective_reporter.errors.is_empty() {
            "ok"
        } else {
            "needs_repair"
        }
    ));
    out.push_str(&format!("- mod_root: `{}`\n", root.display()));
    out.push_str(&format!("- game_root: `{}`\n", game_root.display()));
    if !dependency_mods.is_empty() {
        out.push_str("- dependency_mods:\n");
        for dependency in dependency_mods {
            out.push_str(&format!("  - `{}`\n", dependency.display()));
        }
    }
    if !report.changed_files.is_empty() {
        out.push_str("- changed_files:\n");
        for changed in &report.changed_files {
            out.push_str(&format!("  - `{}`\n", changed));
        }
    }
    out.push_str(&format!(
        "- effective_errors: `{}`\n",
        report.effective_reporter.errors.len()
    ));
    out.push_str(&format!(
        "- effective_warnings: `{}`\n",
        report.effective_reporter.warnings.len()
    ));
    let retrieval_status = if retrieval_error.is_some() {
        "unavailable"
    } else if libraries.is_empty() {
        "disabled"
    } else {
        "ok"
    };
    out.push_str(&format!(
        "- code_example_retrieval: `{}` (libraries: `{}`)\n",
        retrieval_status,
        libraries.len()
    ));
    if let Some(error) = retrieval_error {
        out.push_str(&format!(
            "- retrieval_error: `{}`\n",
            error.replace('`', "'")
        ));
    }
    out.push_str("\n## AI Rules\n\n");
    out.push_str("- Treat every repair item as blocking until strict validation passes.\n");
    out.push_str("- Use only indexed candidates, explicitly authorized dependency/user-mod examples, explicit user-provided symbols, or CLI-generated patch plans.\n");
    out.push_str("- If candidates are empty or ambiguous, ask the user or run `check-code-symbol` / `code-catalog` before writing code.\n");
    out.push_str("- Do not delete player-visible user text to hide text-alignment failures.\n");
    out.push_str("- Do not replace dynamic modifiers with national spirits unless the user explicitly changes the design.\n");
    out.push_str("\n## Repair Items\n\n");
    if report.effective_reporter.errors.is_empty() {
        out.push_str("- No blocking repair items.\n");
    } else {
        for (idx, message) in report
            .effective_reporter
            .errors
            .iter()
            .take(max_items)
            .enumerate()
        {
            out.push_str(&validation_repair_item_markdown(
                idx + 1,
                message,
                index,
                Some(root),
                Some(game_root),
                dependency_mods,
                libraries,
                max_examples,
            ));
            out.push('\n');
        }
    }
    out.push_str("\n## Required Next Commands\n\n");
    let dependency_args = validation_repair_dependency_args(dependency_mods);
    out.push_str(&format!(
        "- `hoi4skill validate {} --game-root {}{} --strict-code-index{} --output .hoi4skill/validation.json`\n",
        validation_command_path_arg(root),
        validation_command_path_arg(game_root),
        dependency_args,
        validation_repair_changed_args(&report.changed_files)
    ));
    out.push_str(&format!(
        "- `hoi4skill validate-repair-context {} --game-root {}{}{} --output .hoi4skill/ai_repair_context.json --markdown-output .hoi4skill/ai_repair_context.md`\n",
        validation_command_path_arg(root),
        validation_command_path_arg(game_root),
        dependency_args,
        validation_repair_changed_args(&report.changed_files)
    ));
    out.push_str("\n## Anti Hallucination Rule\n\n");
    out.push_str("If a symbol, syntax, scope, picture, idea, or modifier is absent from this context and strict code index, fail or ask instead of inventing Clausewitz code.\n");
    out
}

fn validation_repair_item_markdown(
    priority: usize,
    message: &str,
    index: &GameIndex,
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_mods: &[PathBuf],
    libraries: &[PathBuf],
    max_examples: usize,
) -> String {
    let file = validation_repair_file(message);
    let (kind, symbol) = validation_repair_kind_symbol(message);
    let query = symbol.as_deref().unwrap_or(message);
    let candidates = related_code_symbol_matches(index, query, kind.as_deref(), 8);
    let repair_queries = validation_repair_queries(message, symbol.as_deref(), &candidates);
    let examples = validation_repair_code_examples(libraries, &repair_queries, max_examples);
    let commands = validation_repair_commands(
        kind.as_deref(),
        symbol.as_deref(),
        mod_root,
        game_root,
        dependency_mods,
    );
    let do_not_fix_by = validation_repair_do_not_fix_by(message, kind.as_deref());
    let questions = validation_repair_questions(message, symbol.as_deref());
    let mut out = String::new();
    out.push_str(&format!(
        "### {}. {}\n\n",
        priority,
        validation_repair_category(message, kind.as_deref())
    ));
    out.push_str("- blocking: `true`\n");
    if let Some(file) = file {
        out.push_str(&format!("- file: `{}`\n", file.replace('`', "'")));
    }
    if let Some(kind) = kind.as_deref() {
        out.push_str(&format!("- kind: `{kind}`\n"));
    }
    if let Some(symbol) = symbol.as_deref() {
        out.push_str(&format!("- symbol: `{}`\n", symbol.replace('`', "'")));
    }
    out.push_str(&format!("- message: `{}`\n", message.replace('`', "'")));
    out.push_str(&format!(
        "- required_action: `{}`\n",
        validation_repair_required_action(message).replace('`', "'")
    ));
    out.push_str("\nRelated indexed code:\n");
    if candidates.is_empty() {
        out.push_str("- none; ask the user or run `check-code-symbol` instead of guessing.\n");
    } else {
        for candidate in candidates.iter().take(8) {
            out.push_str(&format!(
                "- `{}` / `{}`: `{}`\n",
                candidate.category, candidate.kind, candidate.symbol
            ));
        }
    }
    out.push_str("\nRepair queries:\n");
    for query in repair_queries.iter().take(8) {
        out.push_str(&format!("- `{}`\n", query.replace('`', "'")));
    }
    if !examples.is_empty() {
        out.push_str("\nRetrieved local examples:\n");
        for example in examples.iter().take(max_examples) {
            out.push_str(&format!(
                "- `{}` `{}` from `{}`\n",
                example.system,
                example.symbol,
                example.source.replace('`', "'")
            ));
            out.push_str(&markdown_fence(
                "hoi4",
                &truncate_chars(&example.code, 4_000),
            ));
        }
    }
    if !questions.is_empty() {
        out.push_str("\nQuestions if evidence is missing:\n");
        for question in questions {
            out.push_str(&format!("- {question}\n"));
        }
    }
    out.push_str("\nDo not fix by:\n");
    for rule in do_not_fix_by {
        out.push_str(&format!("- {rule}\n"));
    }
    out.push_str("\nSuggested commands:\n");
    for command in commands {
        out.push_str(&format!("- `{command}`\n"));
    }
    out
}

fn validation_repair_dependency_args(dependency_mods: &[PathBuf]) -> String {
    let mut out = String::new();
    for dependency in dependency_mods {
        out.push_str(" --mod-path ");
        out.push_str(&validation_command_path_arg(dependency));
    }
    out
}

fn validation_repair_changed_args(changed_files: &[String]) -> String {
    if changed_files.is_empty() {
        return String::new();
    }
    let mut out = String::from(" --changed-only");
    for changed in changed_files {
        out.push_str(" --changed ");
        out.push_str(&validation_command_path_arg(Path::new(changed)));
    }
    out
}

fn validation_command_path_arg(path: &Path) -> String {
    format!("\"{}\"", path.display().to_string().replace('"', "\\\""))
}

pub(crate) fn validation_repair_item_json(
    priority: usize,
    message: &str,
    index: &GameIndex,
) -> String {
    validation_repair_item_json_with_context(priority, message, index, None, None, &[], &[], 0)
}

fn validation_repair_item_json_with_context(
    priority: usize,
    message: &str,
    index: &GameIndex,
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_mods: &[PathBuf],
    libraries: &[PathBuf],
    max_examples: usize,
) -> String {
    let file = validation_repair_file(message);
    let (kind, symbol) = validation_repair_kind_symbol(message);
    let query = symbol.as_deref().unwrap_or(message);
    let candidates = related_code_symbol_matches(index, query, kind.as_deref(), 8);
    let repair_queries = validation_repair_queries(message, symbol.as_deref(), &candidates);
    let examples = validation_repair_code_examples(libraries, &repair_queries, max_examples);
    let commands = validation_repair_commands(
        kind.as_deref(),
        symbol.as_deref(),
        mod_root,
        game_root,
        dependency_mods,
    );
    let do_not_fix_by = validation_repair_do_not_fix_by(message, kind.as_deref());
    let questions = validation_repair_questions(message, symbol.as_deref());
    format!(
        "    {{\n      \"priority\": {},\n      \"blocking\": true,\n      \"category\": {},\n      \"file\": {},\n      \"kind\": {},\n      \"symbol\": {},\n      \"message\": {},\n      \"related_indexed_code\": {},\n      \"repair_queries\": {},\n      \"retrieved_code_examples\": {},\n      \"required_action\": {},\n      \"questions\": {},\n      \"do_not_fix_by\": {},\n      \"suggested_commands\": {}\n    }}",
        priority,
        json_str(&validation_repair_category(message, kind.as_deref())),
        file.map(|value| json_str(&value)).unwrap_or_else(|| "null".to_string()),
        kind.map(|value| json_str(&value)).unwrap_or_else(|| "null".to_string()),
        symbol.map(|value| json_str(&value)).unwrap_or_else(|| "null".to_string()),
        json_str(message),
        validation_code_matches_json(&candidates),
        json_array(&repair_queries),
        validation_clausewitz_examples_json(&examples),
        json_str(&validation_repair_required_action(message)),
        json_array(&questions),
        json_array(&do_not_fix_by),
        json_array(&commands)
    )
}

fn validation_repair_file(message: &str) -> Option<String> {
    let (head, _) = message.split_once(": ")?;
    if looks_like_windows_path_prefix(message) || message.starts_with('/') {
        Some(head.to_string())
    } else {
        None
    }
}

fn validation_repair_kind_symbol(message: &str) -> (Option<String>, Option<String>) {
    let kind = validation_repair_kind(message);
    let symbol = validation_repair_symbol(message, kind.as_deref());
    (kind, symbol)
}

fn validation_repair_kind(message: &str) -> Option<String> {
    if let Some(kind) = extract_check_code_symbol_kind(message) {
        return Some(kind);
    }
    if message.contains("dynamic modifier ") && message.contains("national spirit/idea reference") {
        return Some("dynamic_modifier".to_string());
    }
    for (needle, kind) in [
        ("unknown effect `", "effect"),
        ("effect-like key `", "effect"),
        ("unknown trigger `", "trigger"),
        ("trigger-like key `", "trigger"),
        ("unknown modifier `", "modifier"),
        ("modifier-like key `", "modifier"),
        ("unindexed event picture", "event_picture"),
        ("event picture ", "event_picture"),
        ("idea picture ", "resource_id"),
        ("unindexed idea", "idea"),
        ("unindexed resource", "resource"),
        ("unindexed building", "building"),
        ("uses unindexed icon", "resource_id"),
        ("uses unindexed country scope", "resource_id"),
        ("uses non-ASCII scripted localisation scope", "resource_id"),
        ("has invalid HOI4 token", "localisation_token"),
        ("unindexed event-chain country tag", "country_tag"),
    ] {
        if message.contains(needle) {
            return Some(kind.to_string());
        }
    }
    None
}

fn extract_check_code_symbol_kind(message: &str) -> Option<String> {
    let marker = "check-code-symbol --kind ";
    let start = message.find(marker)? + marker.len();
    let mut out = String::new();
    for ch in message[start..].chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            break;
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn validation_repair_symbol(message: &str, kind: Option<&str>) -> Option<String> {
    if message.contains("dynamic modifier ") && message.contains("national spirit/idea reference") {
        return extract_word_after(message, "dynamic modifier ");
    }
    for marker in [
        "unknown effect `",
        "effect-like key `",
        "unknown trigger `",
        "trigger-like key `",
        "unknown modifier `",
        "modifier-like key `",
        "unindexed effect `",
        "unindexed trigger `",
        "unindexed modifier `",
        "unindexed idea `",
        "unindexed resource `",
        "unindexed building `",
        "uses unindexed icon `",
        "uses unindexed country scope `",
        "uses non-ASCII scripted localisation scope `",
    ] {
        if let Some(value) = extract_backticked_after(message, marker) {
            return Some(strip_localisation_symbol(&strip_author_label(&value)));
        }
    }
    if kind.is_some() {
        if let Some(value) = last_backticked_before(message, "verify it with") {
            return Some(strip_author_label(&value));
        }
        if let Some(value) = last_backticked_value(message) {
            return Some(strip_author_label(&value));
        }
    }
    if message.contains("event picture ") {
        return extract_word_after(message, "event picture ");
    }
    if message.contains("idea picture ") {
        return extract_word_after(message, "idea picture ");
    }
    None
}

fn extract_backticked_after(message: &str, marker: &str) -> Option<String> {
    let start = message.find(marker)? + marker.len();
    let rest = &message[start..];
    let end = rest.find('`')?;
    Some(rest[..end].to_string())
}

fn last_backticked_before(message: &str, before: &str) -> Option<String> {
    let end = message.find(before).unwrap_or(message.len());
    last_backticked_value(&message[..end])
}

fn last_backticked_value(message: &str) -> Option<String> {
    let mut values = Vec::new();
    let mut rest = message;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else {
            break;
        };
        values.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    values.pop()
}

fn strip_author_label(value: &str) -> String {
    value
        .rsplit(['：', ':'])
        .next()
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn strip_localisation_symbol(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('£')
        .trim_matches(['[', ']'])
        .split('.')
        .next()
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn extract_word_after(message: &str, marker: &str) -> Option<String> {
    let start = message.find(marker)? + marker.len();
    let value = message[start..]
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | ',' | '`'))
        .next()
        .unwrap_or("")
        .trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn validation_code_matches_json(matches: &[CodeSymbolMatch]) -> String {
    format!(
        "[{}]",
        matches
            .iter()
            .map(|item| {
                format!(
                    "{{\"category\": {}, \"kind\": {}, \"symbol\": {}}}",
                    json_str(item.category),
                    json_str(item.kind),
                    json_str(&item.symbol)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validation_repair_queries(
    message: &str,
    symbol: Option<&str>,
    candidates: &[CodeSymbolMatch],
) -> Vec<String> {
    let mut queries = Vec::new();
    if let Some(symbol) = symbol {
        push_validation_repair_query(&mut queries, symbol);
    }
    for candidate in candidates.iter().take(5) {
        push_validation_repair_query(&mut queries, &candidate.symbol);
    }
    push_validation_repair_query(&mut queries, message);
    queries
}

fn push_validation_repair_query(out: &mut Vec<String>, raw: &str) {
    let query = raw
        .trim()
        .trim_matches('`')
        .replace(['\r', '\n', '\t'], " ");
    let query = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if query.len() < 3 {
        return;
    }
    let query = truncate_chars(&query, 220);
    if !out.iter().any(|existing| existing == &query) {
        out.push(query);
    }
}

fn validation_repair_code_examples(
    libraries: &[PathBuf],
    queries: &[String],
    max_examples: usize,
) -> Vec<ClausewitzExample> {
    if libraries.is_empty() || max_examples == 0 {
        return Vec::new();
    }
    let mut examples = Vec::new();
    for query in queries {
        if let Ok(found) = query_clausewitz_libraries(libraries, query, None, max_examples) {
            for example in found {
                if !examples.iter().any(|existing: &ClausewitzExample| {
                    existing.system == example.system
                        && existing.symbol == example.symbol
                        && existing.source == example.source
                }) {
                    examples.push(example);
                }
                if examples.len() >= max_examples {
                    return examples;
                }
            }
        }
    }
    examples
}

fn validation_clausewitz_examples_json(examples: &[ClausewitzExample]) -> String {
    format!(
        "[{}]",
        examples
            .iter()
            .map(|example| {
                format!(
                    "{{\"system\": {}, \"symbol\": {}, \"source\": {}, \"code\": {}}}",
                    json_str(&example.system),
                    json_str(&example.symbol),
                    json_str(&example.source),
                    json_str(&truncate_chars(&example.code, 4_000))
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validation_repair_category(message: &str, kind: Option<&str>) -> String {
    if message.contains("unresolved generated code marker") || message.contains("TODO") {
        "unresolved_ai_placeholder".to_string()
    } else if message.contains("dynamic modifier ")
        && message.contains("national spirit/idea reference")
    {
        "dynamic_modifier_misuse".to_string()
    } else if message.contains("localisation key")
        && (message.contains("invalid HOI4 token")
            || message.contains("uses unindexed icon")
            || message.contains("uses unindexed country scope")
            || message.contains("uses non-ASCII scripted localisation scope"))
    {
        "localisation_token_mapping".to_string()
    } else if message.contains("text alignment") {
        "text_alignment".to_string()
    } else if message.contains("request-scope violation") {
        "request_scope_violation".to_string()
    } else if message.contains("dynamic_modifier") {
        "dynamic_modifier_code".to_string()
    } else if kind.is_some() {
        "unindexed_code_symbol".to_string()
    } else {
        "validation_error".to_string()
    }
}

fn validation_repair_required_action(message: &str) -> String {
    if message.contains("unresolved generated code marker") || message.contains("TODO") {
        "Replace the placeholder with a CLI-compiled intent, an indexed symbol, or an explicit user-approved mapping; rerun strict validation.".to_string()
    } else if message.contains("dynamic modifier ")
        && message.contains("national spirit/idea reference")
    {
        "Replace the national-spirit reference with dynamic modifier code: use add_dynamic_modifier/remove_dynamic_modifier/has_dynamic_modifier when appropriate, or regenerate the effect through compile-intent/plan-dynamic-modifier-change so it uses the verified dynamic_modifier_scripted_effect_protocol.".to_string()
    } else if message.contains("uses unindexed icon") {
        "Resolve the localisation icon through author-placeholder-plan/register-gfx-icons, or ask the user for the existing GFX sprite before writing final localisation.".to_string()
    } else if message.contains("uses unindexed country scope")
        || message.contains("uses non-ASCII scripted localisation scope")
    {
        "Resolve the country/cosmetic alias to an indexed tag with author-placeholder-plan or ask the user for the exact scripted localisation token.".to_string()
    } else if message.contains("has invalid HOI4 token") {
        "Fix or compile the localisation control token, preserving player-visible text, then rerun strict validation.".to_string()
    } else if message.contains("text alignment") {
        "Restore or localise the exact user-provided player-visible text, then rerun validate with the same text source.".to_string()
    } else if message.contains("request-scope violation") {
        "Remove the unauthorized generated system or ask the user to explicitly authorize that system before generating it.".to_string()
    } else if message.contains("unknown") || message.contains("unindexed") {
        "Replace the symbol with an indexed candidate, generate the missing declared asset/system through a dedicated CLI command, or ask the user for an explicit mapping.".to_string()
    } else {
        "Fix the reported file without inventing syntax, then rerun strict validation and this repair-context command.".to_string()
    }
}

fn validation_repair_do_not_fix_by(message: &str, kind: Option<&str>) -> Vec<String> {
    let mut rules = vec![
        "do not invent a Clausewitz key that is absent from the code index".to_string(),
        "do not silence the error by deleting user-requested player-visible content".to_string(),
        "do not skip final validation or downgrade strict-code-index".to_string(),
    ];
    if message.contains("dynamic_modifier") || kind == Some("dynamic_modifier") {
        rules.push("do not convert a dynamic modifier into a national spirit unless the user explicitly requests that design change".to_string());
    }
    if kind == Some("event_picture") || kind == Some("resource_id") {
        rules.push("do not substitute a random GFX sprite; use an indexed picture or ask for the intended asset".to_string());
    }
    if message.contains("localisation key") {
        rules.push("do not leave raw author placeholders or non-ASCII scripted localisation scopes in final .yml files".to_string());
        rules.push(
            "do not replace missing icons or country scopes by deleting player-visible prose"
                .to_string(),
        );
    }
    rules
}

fn validation_repair_commands(
    kind: Option<&str>,
    symbol: Option<&str>,
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_mods: &[PathBuf],
) -> Vec<String> {
    let mut commands = Vec::new();
    let mod_arg = mod_root
        .map(validation_command_path_arg)
        .unwrap_or_else(|| "<mod-root>".to_string());
    let game_arg = game_root
        .map(validation_command_path_arg)
        .unwrap_or_else(|| "<HOI4 root>".to_string());
    let dependency_args = validation_repair_dependency_args(dependency_mods);
    if let (Some(kind), Some(symbol)) = (kind, symbol) {
        if kind != "localisation_token" {
            commands.push(format!(
                "hoi4skill check-code-symbol --game-root {game_arg}{dependency_args} --kind {kind} --symbol {symbol}"
            ));
            commands.push(format!(
                "hoi4skill query-clausewitz-library --query {symbol} --system {kind}"
            ));
        }
    }
    commands.push(format!(
        "hoi4skill validate {mod_arg} --game-root {game_arg}{dependency_args} --strict-code-index"
    ));
    commands
}

fn validation_repair_questions(message: &str, symbol: Option<&str>) -> Vec<String> {
    if message.contains("dynamic modifier ") && message.contains("national spirit/idea reference") {
        let dynamic_modifier = symbol.unwrap_or("<dynamic_modifier>");
        return vec![format!(
            "Should `{dynamic_modifier}` be applied as a dynamic modifier, changed through its scripted-effect helper protocol, or replaced by a deliberately new national spirit approved by the user?"
        )];
    }
    if message.contains("uses unindexed icon") {
        let icon = symbol.unwrap_or("<icon>");
        return vec![format!(
            "Does localisation icon `{icon}` already have an existing GFX sprite? If yes, provide the exact sprite name; if not, approve adding/registering the asset before final localisation."
        )];
    }
    if message.contains("uses unindexed country scope") {
        let tag = symbol.unwrap_or("<TAG>");
        return vec![format!(
            "Which indexed country or cosmetic tag should `{tag}` refer to? Provide `[TAG.GetName]`, `[TAG.GetLeader]`, or `[TAG.GetFlag]` explicitly if needed."
        )];
    }
    if message.contains("uses non-ASCII scripted localisation scope") {
        let scope = symbol.unwrap_or("<scope>");
        return vec![format!(
            "The scope `{scope}` is not valid final HOI4 scripted localisation. Which indexed tag/cosmetic alias should this author placeholder compile to?"
        )];
    }
    if message.contains("has invalid HOI4 token") {
        return vec![
            "Should this text be compiled from author placeholders with author-placeholder-plan, or should the HOI4 control token be edited directly?".to_string(),
        ];
    }
    Vec::new()
}

fn validation_issue_groups_json(messages: &[String], max_groups: usize) -> String {
    let mut groups: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for message in messages {
        let key = validation_issue_group_key(message);
        let entry = groups.entry(key).or_insert_with(|| (0, Vec::new()));
        entry.0 += 1;
        if entry.1.len() < 5 {
            entry.1.push(message.clone());
        }
    }
    let mut groups = groups.into_iter().collect::<Vec<_>>();
    groups.sort_by(|(_, left), (_, right)| right.0.cmp(&left.0));
    groups.truncate(max_groups);
    format!(
        "[{}]",
        groups
            .into_iter()
            .map(|(message, (count, examples))| {
                format!(
                    "{{\"count\": {}, \"message\": {}, \"examples\": {}}}",
                    count,
                    json_str(&message),
                    json_array(&examples)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn validation_issue_group_key(message: &str) -> String {
    let Some((_, rest)) = message.split_once(": ") else {
        return message.to_string();
    };
    if looks_like_windows_path_prefix(message) || message.starts_with('/') {
        rest.to_string()
    } else {
        message.to_string()
    }
}

pub(crate) fn looks_like_windows_path_prefix(message: &str) -> bool {
    let bytes = message.as_bytes();
    bytes.len() > 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}

fn read_validation_baseline(path: &Path) -> Result<(BTreeSet<String>, BTreeSet<String>), String> {
    let text = read_utf8_lossy(path)?;
    Ok((
        parse_json_string_array_field(&text, "errors"),
        parse_json_string_array_field(&text, "warnings"),
    ))
}

pub(crate) fn parse_json_string_array_field(text: &str, key: &str) -> BTreeSet<String> {
    let Some(key_pos) = text.find(&format!("\"{key}\":")) else {
        return BTreeSet::new();
    };
    let Some(array_start_rel) = text[key_pos..].find('[') else {
        return BTreeSet::new();
    };
    let array_start = key_pos + array_start_rel;
    let Some(array_end_rel) = text[array_start..].find(']') else {
        return BTreeSet::new();
    };
    parse_json_string_array(&text[array_start + 1..array_start + array_end_rel])
        .into_iter()
        .collect()
}

fn parse_json_string_array(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in text.chars() {
        if !in_string {
            if ch == '"' {
                in_string = true;
                current.clear();
            }
            continue;
        }
        if escape {
            match ch {
                '"' => current.push('"'),
                '\\' => current.push('\\'),
                'n' => current.push('\n'),
                'r' => current.push('\r'),
                't' => current.push('\t'),
                other => current.push(other),
            }
            escape = false;
        } else if ch == '\\' {
            escape = true;
        } else if ch == '"' {
            values.push(current.clone());
            in_string = false;
        } else {
            current.push(ch);
        }
    }
    values
}

fn validation_changed_files(root: &Path, map: &ArgMap) -> Result<Vec<String>, String> {
    let mut files = BTreeSet::new();
    for key in ["changed", "changed-file"] {
        for raw in repeated_values(map, key) {
            files.insert(validation_normalize_changed_path(root, raw));
        }
    }
    for raw in repeated_values(map, "changed-list")
        .into_iter()
        .chain(repeated_values(map, "changed-files").into_iter())
    {
        let path = validation_changed_list_path(root, raw)?;
        for item in read_changed_paths_file(&path)? {
            files.insert(validation_normalize_changed_path(root, &item));
        }
    }
    for raw in repeated_values(map, "author-report")
        .into_iter()
        .chain(repeated_values(map, "changed-report").into_iter())
        .chain(repeated_values(map, "author-output").into_iter())
    {
        let path = validation_changed_list_path(root, raw)?;
        for item in production_changed_paths_from_author_report(root, &path)? {
            files.insert(item);
        }
    }
    if !map.flags.contains("ignore-default-author-report")
        && !map.flags.contains("no-default-author-report")
    {
        let default = root
            .join(".hoi4skill")
            .join("author_intent_workflow_author.json");
        if default.exists() {
            for item in production_changed_paths_from_author_report(root, &default)? {
                files.insert(item);
            }
        }
    }
    if map.flags.contains("from-git")
        || map.flags.contains("changed-from-git")
        || map.flags.contains("git")
    {
        let git_root = value(map, "git-root")
            .map(normalize_path)
            .transpose()?
            .unwrap_or_else(|| root.to_path_buf());
        for item in collect_git_changed_paths(&git_root)? {
            files.insert(validation_normalize_changed_path(root, &item));
        }
    }
    Ok(files.into_iter().collect())
}

fn validation_normalize_changed_path(root: &Path, raw: &str) -> String {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        relative_slash_path(root, &path)
    } else {
        slash_path(&path)
    }
}

fn validation_changed_list_path(root: &Path, raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        normalize_path(raw)
    } else {
        Ok(root.join(path))
    }
}

fn validation_message_mentions_changed(root: &Path, msg: &str, changed_files: &[String]) -> bool {
    let msg_slash = msg.replace('\\', "/");
    changed_files.iter().any(|changed| {
        let changed_slash = changed.replace('\\', "/");
        let absolute = slash_path(&root.join(changed));
        msg.contains(changed) || msg_slash.contains(&changed_slash) || msg_slash.contains(&absolute)
    })
}

impl Reporter {
    pub(crate) fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }
    pub(crate) fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }
    pub(crate) fn print(&self) {
        if !self.errors.is_empty() {
            println!("ERRORS:");
            for msg in &self.errors {
                println!("  - {msg}");
            }
        }
        if !self.warnings.is_empty() {
            println!("WARNINGS:");
            for msg in &self.warnings {
                println!("  - {msg}");
            }
        }
        if self.errors.is_empty() && self.warnings.is_empty() {
            println!("OK: no static issues found.");
        } else if self.errors.is_empty() {
            println!("WARN: warnings only; review the list above.");
        } else {
            println!("ERROR: validation failed.");
        }
    }
}

pub(crate) fn check_descriptor(root: &Path, reporter: &mut Reporter) {
    let path = root.join("descriptor.mod");
    if !path.exists() {
        reporter.error(format!("{}: missing descriptor.mod", path.display()));
        return;
    }
    match read_utf8_lossy(&path) {
        Ok(text) => {
            for key in ["name", "supported_version"] {
                if !text
                    .lines()
                    .any(|line| line.trim_start().starts_with(&format!("{key}=")))
                {
                    reporter.warn(format!("{}: missing {key}= metadata", path.display()));
                }
            }
        }
        Err(err) => reporter.error(err),
    }
}

pub(crate) fn check_localisation(path: &Path, reporter: &mut Reporter) {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            reporter.error(format!("read {}: {err}", path.display()));
            return;
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    let first = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("");
    let first = first.trim_start_matches('\u{feff}').trim();
    if first.is_empty() {
        reporter.warn(format!("{}: empty localisation file", path.display()));
    } else if !(first.starts_with("l_") && first.ends_with(':')) {
        reporter.error(format!(
            "{}: first non-empty line should be a language header like l_simp_chinese:",
            path.display()
        ));
    }
    if slash_path(path).contains("/localisation/") && !bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        reporter.error(format!(
            "{}: localisation file has no UTF-8 BOM; HOI4 may fail to load it",
            path.display()
        ));
    }
    check_mod_name_localisation_keys(path, &text, reporter);
    check_localisation_tokens(path, &text, reporter);
    check_yaml_duplicate_keys(path, &text, reporter);
}

pub(crate) fn check_localisation_tokens(path: &Path, text: &str, reporter: &mut Reporter) {
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let Some((key, value)) = parse_localisation_line(line) else {
            continue;
        };
        let (_, issues) = extract_localisation_tokens(&value);
        for issue in issues {
            reporter.error(format!(
                "{}:{line_no}: localisation key `{key}` has invalid HOI4 token: {}",
                path.display(),
                issue.message
            ));
        }
    }
}

pub(crate) fn check_indexed_localisation_tokens(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    if !options.strict_code_index {
        return;
    }
    let Some(index) = game_index else {
        return;
    };
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let Some((key, value)) = parse_localisation_line(line) else {
            continue;
        };
        let (tokens, _) = extract_localisation_tokens(&value);
        for token in tokens {
            if token.kind == "icon" {
                let icon = token.text.trim_start_matches('£');
                if !indexed_localisation_icon_exists(icon, index) {
                    reporter.error(format!(
                        "{}:{line_no}: localisation key `{key}` uses unindexed icon `{}`; register the sprite or ask the user for the correct icon before final localisation",
                        path.display(),
                        token.text
                    ));
                }
            } else if token.kind == "scripted_loc" {
                if let Some(scope) = scripted_localisation_scope(&token.text) {
                    if scope.chars().any(|ch| !ch.is_ascii()) {
                        reporter.error(format!(
                            "{}:{line_no}: localisation key `{key}` uses non-ASCII scripted localisation scope `{scope}` in `{}`; compile author placeholders to `[TAG.GetName]`/`[TAG.GetLeader]`/`[TAG.GetFlag]` before final localisation",
                            path.display(),
                            token.text
                        ));
                    } else if looks_like_tag(scope)
                        && !index.country_tags.contains(scope)
                        && !is_dynamic_tag_ref(scope)
                    {
                        reporter.error(format!(
                            "{}:{line_no}: localisation key `{key}` uses unindexed country scope `{scope}` in `{}`; verify the tag/cosmetic alias before final localisation",
                            path.display(),
                            token.text
                        ));
                    }
                }
            }
        }
    }
}

fn scripted_localisation_scope(token: &str) -> Option<&str> {
    let inner = token.strip_prefix('[')?.strip_suffix(']')?;
    if inner.starts_with('?') {
        return None;
    }
    let (scope, method) = inner.split_once('.')?;
    if !scripted_localisation_country_method(method) {
        return None;
    }
    let scope = scope.trim();
    if is_builtin_scripted_localisation_scope(scope) {
        None
    } else {
        Some(scope)
    }
}

fn scripted_localisation_country_method(method: &str) -> bool {
    let method = method.split('|').next().unwrap_or(method).trim();
    matches!(
        method,
        "GetName"
            | "GetNameDef"
            | "GetNameWithFlag"
            | "GetAdjective"
            | "GetLeader"
            | "GetFlag"
            | "GetRulingIdeology"
            | "GetRulingParty"
            | "GetCommunistParty"
            | "GetDemocraticParty"
            | "GetFascistParty"
            | "GetNeutralParty"
    )
}

fn is_builtin_scripted_localisation_scope(scope: &str) -> bool {
    matches!(
        scope.to_ascii_uppercase().as_str(),
        "ROOT" | "FROM" | "PREV" | "THIS"
    ) || matches!(
        scope,
        "owner" | "controller" | "capital_scope" | "overlord" | "faction_leader"
    )
}

pub(crate) fn check_mod_name_localisation_keys(path: &Path, text: &str, reporter: &mut Reporter) {
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = line.trim_start_matches('\u{feff}');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut content = line.trim_start();
        if let Some(rest) = content.strip_prefix("- ") {
            content = rest.trim_start();
        }
        let Some(colon) = content.find(':') else {
            continue;
        };
        let key = content[..colon].trim().trim_matches('"').trim_matches('\'');
        if key.ends_with("_mod_name") {
            reporter.warn(format!(
                "{}: localisation key `{key}` at line {line_no} looks like a mod display name; write mod names in descriptor.mod and the launcher .mod file instead of l_simp_chinese",
                path.display()
            ));
        }
    }
}

pub(crate) fn check_yaml_duplicate_keys(path: &Path, text: &str, reporter: &mut Reporter) {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut sequence_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut block_scalar_indent: Option<usize> = None;
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = line.trim_start_matches('\u{feff}');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        if let Some(block_indent) = block_scalar_indent {
            if indent > block_indent {
                continue;
            }
            block_scalar_indent = None;
        }
        let mut content = line.trim_start();
        let is_sequence_item = content.starts_with("- ");
        if let Some(rest) = content.strip_prefix("- ") {
            content = rest.trim_start();
        }
        let Some(colon) = content.find(':') else {
            continue;
        };
        let key = content[..colon].trim().trim_matches('"').trim_matches('\'');
        let value_after_colon = content[colon + 1..].trim();
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        while stack.last().is_some_and(|(level, _)| *level >= indent) {
            stack.pop();
        }
        if is_sequence_item {
            let parent_key = stack
                .iter()
                .map(|(_, key)| key.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let sequence_key = format!("{parent_key}@{indent}");
            let index = sequence_counts.entry(sequence_key).or_insert(0);
            stack.push((indent, format!("[{}]", *index)));
            *index += 1;
        }
        let mut full_key = stack
            .iter()
            .map(|(_, key)| key.as_str())
            .collect::<Vec<_>>()
            .join(".");
        if full_key.is_empty() {
            full_key.push_str(key);
        } else {
            full_key.push('.');
            full_key.push_str(key);
        }
        if let Some(first_line) = seen.insert(full_key.clone(), line_no) {
            reporter.warn(format!(
                "{}: duplicate YAML key `{full_key}` at line {line_no} (first seen at line {first_line})",
                path.display()
            ));
        }
        if value_after_colon.starts_with('|') || value_after_colon.starts_with('>') {
            block_scalar_indent = Some(indent);
        } else if value_after_colon.is_empty() {
            stack.push((indent, key.to_string()));
        }
    }
}

pub(crate) fn check_jsonl_duplicate_keys(path: &Path, text: &str, reporter: &mut Reporter) {
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for key in jsonl_top_level_keys(trimmed) {
            if let Some(first_index) = seen.insert(key.clone(), line_no) {
                reporter.warn(format!(
                    "{}: duplicate JSONL key `{key}` at line {line_no} (first seen at line {first_index})",
                    path.display()
                ));
            }
        }
    }
}

pub(crate) fn jsonl_top_level_keys(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut escape = false;
    let mut expect_key = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' && !escape {
            if !in_quote && depth == 1 && expect_key {
                let start = i + 1;
                i += 1;
                let mut string_escape = false;
                while i < bytes.len() {
                    let inner = bytes[i] as char;
                    if inner == '"' && !string_escape {
                        break;
                    }
                    string_escape = inner == '\\' && !string_escape;
                    if inner != '\\' {
                        string_escape = false;
                    }
                    i += 1;
                }
                if i >= bytes.len() {
                    break;
                }
                let key = &line[start..i];
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b':' {
                    keys.push(key.to_string());
                    expect_key = false;
                }
            } else {
                in_quote = !in_quote;
            }
            i += 1;
            continue;
        }
        if in_quote {
            escape = ch == '\\' && !escape;
            if ch != '\\' {
                escape = false;
            }
            i += 1;
            continue;
        }
        escape = false;
        match ch {
            '{' => {
                depth += 1;
                if depth == 1 {
                    expect_key = true;
                }
            }
            '}' => {
                depth = (depth - 1).max(0);
                expect_key = false;
            }
            ',' if depth == 1 => expect_key = true,
            _ => {}
        }
        i += 1;
    }
    keys
}

pub(crate) fn check_braces(path: &Path, text: &str, reporter: &mut Reporter) {
    let cleaned = strip_comments(text);
    check_braces_cleaned(path, &cleaned, reporter);
}

fn check_braces_cleaned(path: &Path, cleaned: &str, reporter: &mut Reporter) {
    let mut depth: i32 = 0;
    let mut in_quote = false;
    let mut escape = false;
    for (idx, ch) in cleaned.chars().enumerate() {
        if ch == '"' && !escape {
            in_quote = !in_quote;
        } else if !in_quote {
            if ch == '{' {
                depth += 1;
            } else if ch == '}' {
                depth -= 1;
                if depth < 0 {
                    reporter.error(format!(
                        "{}: extra closing brace near character {idx}",
                        path.display()
                    ));
                    return;
                }
            }
        }
        escape = ch == '\\' && !escape;
        if ch != '\\' {
            escape = false;
        }
    }
    if depth != 0 {
        reporter.error(format!(
            "{}: unbalanced braces, final depth {depth}",
            path.display()
        ));
    }
}

pub(crate) fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let mut in_quote = false;
        let mut escape = false;
        for ch in line.chars() {
            if ch == '"' && !escape {
                in_quote = !in_quote;
            }
            if ch == '#' && !in_quote {
                break;
            }
            out.push(ch);
            escape = ch == '\\' && !escape;
            if ch != '\\' {
                escape = false;
            }
        }
        out.push('\n');
    }
    out
}

pub(crate) fn collect_ids_and_namespaces(
    path: &Path,
    text: &str,
    ids: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    namespaces: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    reporter: &mut Reporter,
) {
    let norm = slash_path(path);
    if norm.contains("/events/") {
        let mut local_namespaces = BTreeSet::new();
        let mut seen_event_body = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if is_event_definition_line(trimmed) {
                seen_event_body = true;
            }
            if trimmed.starts_with("namespace") && trimmed.contains('=') {
                reporter.error(format!(
                    "{}: event namespace should use add_namespace = ..., not namespace = ...",
                    path.display()
                ));
            }
            if let Some(value) = assignment_value(trimmed, "add_namespace") {
                if seen_event_body {
                    reporter.warn(format!(
                        "{}: add_namespace = {value} should be declared at the top level before event bodies",
                        path.display()
                    ));
                }
                local_namespaces.insert(value.to_string());
                namespaces
                    .entry(value.to_string())
                    .or_default()
                    .insert(path.to_path_buf());
            }
        }
        for block in event_blocks(text) {
            if let Some(value) = block_assignment(&block, "id") {
                ids.entry(value.to_string())
                    .or_default()
                    .insert(path.to_path_buf());
                warn_event_id_outside_declared_namespace(path, &value, &local_namespaces, reporter);
            }
        }
    }
    if norm.contains("/common/national_focus/") {
        for block in blocks_named(text, "focus") {
            if let Some(value) = block_assignment(&block, "id") {
                ids.entry(value.to_string())
                    .or_default()
                    .insert(path.to_path_buf());
            }
        }
    }
}

pub(crate) fn is_event_definition_line(line: &str) -> bool {
    ["country_event", "news_event", "state_event"]
        .iter()
        .any(|key| line.starts_with(key) && line.contains('='))
}

pub(crate) fn warn_event_id_outside_declared_namespace(
    path: &Path,
    event_id: &str,
    local_namespaces: &BTreeSet<String>,
    reporter: &mut Reporter,
) {
    let Some((namespace, number)) = event_id_namespace_number(event_id) else {
        reporter.warn(format!(
            "{}: event id {event_id} should be written as <namespace>.<number>",
            path.display()
        ));
        return;
    };
    let event_id_max = active_event_id_max();
    if !(1..=event_id_max).contains(&number) {
        reporter.warn(format!(
            "{}: event id {event_id} uses number {number}; HOI4 event IDs should use 1..={event_id_max} inside the namespace",
            path.display()
        ));
    }
    if local_namespaces.contains(&namespace) {
        return;
    }
    if local_namespaces.is_empty() {
        reporter.warn(format!(
            "{}: event id {event_id} must be preceded by add_namespace = {namespace} in the same event file",
            path.display()
        ));
    } else {
        reporter.warn(format!(
            "{}: event id {event_id} uses namespace {namespace}, but this file declares {}",
            path.display(),
            local_namespaces
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}

pub(crate) fn collect_localisation_keys(text: &str, keys: &mut BTreeSet<String>) {
    for line in text.lines() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("l_") && trimmed.ends_with(':') {
            continue;
        }
        if let Some(idx) = trimmed.find(':') {
            let key = trimmed[..idx].trim();
            if !key.is_empty() {
                keys.insert(key.to_string());
            }
        }
    }
}

pub(crate) fn collect_localisation_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    reporter: &mut Reporter,
) {
    let norm = slash_path(path);
    if norm.contains("/common/national_focus/") {
        for block in blocks_named(text, "focus") {
            if let Some(id) = block_assignment(&block, "id") {
                add_localisation_ref(refs, &id, path);
                add_localisation_ref(refs, &format!("{id}_desc"), path);
            }
        }
    }

    if norm.contains("/events/") {
        for block in event_blocks(text) {
            if block_assignment(&block, "is_triggered_only").is_none()
                && block_assignment(&block, "mean_time_to_happen").is_none()
            {
                reporter.warn(format!(
                    "{}: event block should include is_triggered_only = yes or mean_time_to_happen",
                    path.display()
                ));
            }
            for key in ["title", "desc"] {
                if let Some(value) = block_assignment(&block, key) {
                    add_localisation_ref_if_key(refs, &value, path);
                }
            }
            for option in blocks_named(&block, "option") {
                if let Some(name) = block_assignment(&option, "name") {
                    add_localisation_ref_if_key(refs, &name, path);
                } else {
                    reporter.warn(format!(
                        "{}: event option block should include name = <localisation key>",
                        path.display()
                    ));
                }
            }
        }
    }
}

pub(crate) fn add_localisation_ref(
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    key: &str,
    path: &Path,
) {
    refs.entry(key.to_string())
        .or_default()
        .insert(path.to_path_buf());
}

pub(crate) fn add_localisation_ref_if_key(
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    value: &str,
    path: &Path,
) {
    if is_localisation_key_like(value) {
        add_localisation_ref(refs, value, path);
    }
}

pub(crate) fn is_localisation_key_like(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-'))
        && value.chars().any(|c| matches!(c, '_' | '.' | ':' | '-'))
}

pub(crate) fn event_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    for key in ["country_event", "news_event", "state_event"] {
        blocks.extend(
            blocks_named(text, key)
                .into_iter()
                .filter(|block| is_event_definition_block(block)),
        );
    }
    blocks
}

pub(crate) fn is_event_definition_block(block: &str) -> bool {
    let event_definition_field_signals = [
        "title",
        "desc",
        "picture",
        "is_triggered_only",
        "fire_only_once",
        "mean_time_to_happen",
        "immediate",
        "option",
        "trigger",
    ];
    event_definition_field_signals
        .iter()
        .any(|key| block_assignment(block, key).is_some() || !blocks_named(block, key).is_empty())
        || direct_assignment_keys(block).into_iter().any(|key| {
            !matches!(key.as_str(), "id" | "days")
                && closest_critical_field(&key, &event_definition_field_signals).is_some()
        })
}

pub(crate) fn check_sprite_textures(
    root: &Path,
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    reporter: &mut Reporter,
) {
    for block in textured_gfx_type_blocks(text) {
        if let Some(texturefile) = gfx_texturefile_assignment(&block) {
            if resolve_texture_in_indexed_roots(root, &texturefile, game_index).is_none() {
                reporter.warn(format!(
                    "{}: sprite texturefile not found in this mod or indexed roots: {}",
                    path.display(),
                    texturefile
                ));
            }
        }
    }
}

pub(crate) fn resolve_texture_in_indexed_roots(
    root: &Path,
    texturefile: &str,
    game_index: Option<&GameIndex>,
) -> Option<PathBuf> {
    resolve_texture(root, texturefile).or_else(|| {
        game_index.and_then(|index| {
            index
                .indexed_roots
                .iter()
                .filter(|indexed_root| *indexed_root != root)
                .find_map(|indexed_root| resolve_texture(indexed_root, texturefile))
        })
    })
}

pub(crate) fn collect_sprite_names(text: &str, sprite_names: &mut BTreeSet<String>) {
    for block in named_gfx_type_blocks(text) {
        if let Some(name) = block_assignment(&block, "name") {
            sprite_names.insert(name);
        }
    }
}

pub(crate) fn collect_gfx_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    if !text.contains("GFX_") {
        return;
    }
    let cleaned = strip_comments(text);
    collect_gfx_refs_cleaned(path, &cleaned, refs);
}

fn collect_gfx_refs_cleaned(
    path: &Path,
    cleaned: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    if !cleaned.contains("GFX_") {
        return;
    }
    for token in token_candidates(cleaned) {
        if token.starts_with("GFX_") {
            refs.entry(token.to_string())
                .or_default()
                .insert(path.to_path_buf());
        }
    }
}

#[allow(dead_code)]
pub(crate) fn collect_idea_picture_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    reporter: &mut Reporter,
) {
    let cleaned = strip_comments(text);
    collect_idea_picture_refs_cleaned(path, &cleaned, refs, reporter);
}

fn collect_idea_picture_refs_cleaned(
    path: &Path,
    cleaned: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    reporter: &mut Reporter,
) {
    if !slash_path(path).contains("/common/ideas/") {
        return;
    }
    for picture in assignment_values_in_text(cleaned, "picture") {
        let picture = picture.trim_matches('"');
        if picture.starts_with("GFX_idea_") {
            reporter.error(format!(
                "{}: idea picture must omit the GFX_idea_ prefix; use `picture = {}`",
                path.display(),
                picture.trim_start_matches("GFX_idea_")
            ));
            continue;
        }
        if is_reference_identifier(picture) {
            refs.entry(picture.to_string())
                .or_default()
                .insert(path.to_path_buf());
        }
    }
}

#[allow(dead_code)]
pub(crate) fn collect_event_picture_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    let cleaned = strip_comments(text);
    collect_event_picture_refs_cleaned(path, &cleaned, refs);
}

fn collect_event_picture_refs_cleaned(
    path: &Path,
    cleaned: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    if !slash_path(path).contains("/events/") {
        return;
    }
    for block in event_blocks(cleaned) {
        let Some(picture) = block_assignment(&block, "picture") else {
            continue;
        };
        let picture = picture.trim_matches('"');
        if is_reference_identifier(picture) {
            refs.entry(picture.to_string())
                .or_default()
                .insert(path.to_path_buf());
        }
    }
}

#[allow(dead_code)]
pub(crate) fn collect_country_tag_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    let cleaned = strip_comments(text);
    collect_country_tag_refs_cleaned(path, &cleaned, refs);
}

fn collect_country_tag_refs_cleaned(
    path: &Path,
    cleaned: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
) {
    let norm = slash_path(path);
    if !(norm.contains("/common/")
        || norm.contains("/events/")
        || norm.contains("/history/")
        || norm.ends_with(".txt")
        || norm.ends_with(".mod"))
    {
        return;
    }
    for key in [
        "tag",
        "original_tag",
        "owner",
        "controller",
        "add_core_of",
        "set_cosmetic_tag",
    ] {
        for value in assignment_values_in_text(cleaned, key) {
            if looks_like_tag(&value) {
                refs.entry(value).or_default().insert(path.to_path_buf());
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn collect_focus_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    local_ids: &mut BTreeSet<String>,
) {
    let cleaned = strip_comments(text);
    collect_focus_refs_cleaned(path, &cleaned, refs, local_ids);
}

fn collect_focus_refs_cleaned(
    path: &Path,
    cleaned: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    local_ids: &mut BTreeSet<String>,
) {
    let norm = slash_path(path);
    if !norm.contains("/common/national_focus/") {
        return;
    }
    for block in blocks_named(cleaned, "focus") {
        if let Some(id) = block_assignment(&block, "id") {
            local_ids.insert(id.clone());
        }
        for prerequisite in blocks_named(&block, "prerequisite") {
            for value in assignment_values_in_text(&prerequisite, "focus") {
                if is_reference_identifier(&value) {
                    add_ref(refs, &value, path);
                }
            }
        }
        for mutual in blocks_named(&block, "mutually_exclusive") {
            for value in assignment_values_in_text(&mutual, "focus") {
                if is_reference_identifier(&value) {
                    add_ref(refs, &value, path);
                }
            }
        }
        if let Some(relative_id) = direct_assignment_value(&block, "relative_position_id") {
            if is_reference_identifier(relative_id) {
                add_ref(refs, relative_id, path);
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct GameDataRefs {
    pub(crate) buildings: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) building_levels: Vec<BuildingLevelRef>,
    pub(crate) resources: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) ideologies: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) traits: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) equipment: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) technologies: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) technology_categories: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) sub_units: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) wargoal_types: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) modifiers: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) ideas: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) dynamic_modifiers: BTreeMap<String, BTreeSet<PathBuf>>,
    pub(crate) dynamic_modifier_variables: BTreeMap<String, BTreeSet<PathBuf>>,
}

#[allow(dead_code)]
pub(crate) fn collect_game_data_refs(path: &Path, text: &str, refs: &mut GameDataRefs) {
    let cleaned = strip_comments(text);
    collect_game_data_refs_cleaned(path, &cleaned, refs);
}

fn collect_game_data_refs_cleaned(path: &Path, cleaned: &str, refs: &mut GameDataRefs) {
    let norm = slash_path(path);
    if !(norm.contains("/common/")
        || norm.contains("/events/")
        || norm.contains("/history/")
        || norm.ends_with(".txt"))
    {
        return;
    }
    for block in blocks_named(cleaned, "add_building_construction") {
        if let Some(building) = block_assignment(&block, "type") {
            add_ref(&mut refs.buildings, &building, path);
            if let Some(level) = block_assignment(&block, "level").and_then(|s| s.parse().ok()) {
                refs.building_levels.push(BuildingLevelRef {
                    building,
                    level,
                    path: path.to_path_buf(),
                });
            }
        }
    }
    for block in blocks_named(cleaned, "add_resource") {
        if let Some(resource) = block_assignment(&block, "type") {
            add_ref(&mut refs.resources, &resource, path);
        }
    }
    for key in ["ruling_party", "ideology"] {
        for ideology in assignment_values_in_text(cleaned, key) {
            if is_identifier_like(&ideology) {
                add_ref(&mut refs.ideologies, &ideology, path);
            }
        }
    }
    for block in blocks_named(cleaned, "traits") {
        for token in token_candidates(&block) {
            if is_reference_identifier(token) {
                add_ref(&mut refs.traits, token, path);
            }
        }
    }
    for key in [
        "add_trait",
        "remove_trait",
        "add_unit_leader_trait",
        "remove_unit_leader_trait",
        "add_country_leader_trait",
        "remove_country_leader_trait",
    ] {
        for value in assignment_values_in_text(&cleaned, key) {
            if is_reference_identifier(&value) {
                add_ref(&mut refs.traits, &value, path);
            }
        }
    }
    for name in [
        "add_equipment_to_stockpile",
        "create_equipment_variant",
        "add_equipment_production",
    ] {
        for block in blocks_named(&cleaned, name) {
            if let Some(equipment) = block_assignment(&block, "type") {
                if is_reference_identifier(&equipment) {
                    add_ref(&mut refs.equipment, &equipment, path);
                }
            }
        }
    }
    if !norm.contains("/common/units/equipment/") {
        for block in blocks_named(&cleaned, "equipment") {
            for key in direct_block_keys(&block) {
                if is_reference_identifier(&key) {
                    add_ref(&mut refs.equipment, &key, path);
                }
            }
        }
    }
    for block in blocks_named(&cleaned, "set_technology") {
        for key in direct_block_keys(&block) {
            if !is_set_technology_metadata_key(&key) && is_reference_identifier(&key) {
                add_ref(&mut refs.technologies, &key, path);
            }
        }
    }
    if norm.contains("/common/technologies/") {
        for block in blocks_named(&cleaned, "categories") {
            for token in token_candidates(&block) {
                if is_reference_identifier(token) {
                    add_ref(&mut refs.technology_categories, token, path);
                }
            }
        }
    }
    for name in ["regiments", "support"] {
        for block in blocks_named(&cleaned, name) {
            for key in direct_block_keys(&block) {
                if is_reference_identifier(&key) {
                    add_ref(&mut refs.sub_units, &key, path);
                }
            }
        }
    }
    for block in blocks_named(&cleaned, "create_wargoal") {
        if let Some(wargoal) = block_assignment(&block, "type") {
            if is_reference_identifier(&wargoal) {
                add_ref(&mut refs.wargoal_types, &wargoal, path);
            }
        }
    }
    for block in blocks_named(&cleaned, "modifier") {
        for key in direct_block_keys(&block) {
            if is_modifier_ref_candidate(&key) {
                add_ref(&mut refs.modifiers, &key, path);
            }
        }
    }
    for key in ["add_ideas", "remove_ideas", "has_idea"] {
        for idea in assignment_values_in_text(&cleaned, key) {
            if idea != "{"
                && !matches!(idea.as_str(), "yes" | "no")
                && is_reference_identifier(&idea)
            {
                add_ref(&mut refs.ideas, &idea, path);
            }
        }
    }
    for block in blocks_named(&cleaned, "swap_ideas") {
        for key in ["remove_idea", "add_idea"] {
            if let Some(idea) = block_assignment(&block, key) {
                if is_reference_identifier(&idea) {
                    add_ref(&mut refs.ideas, &idea, path);
                }
            }
        }
    }
    for key in [
        "add_dynamic_modifier",
        "remove_dynamic_modifier",
        "has_dynamic_modifier",
    ] {
        for block in blocks_named(&cleaned, key) {
            if let Some(dynamic_modifier) = block_assignment(&block, "modifier") {
                if is_reference_identifier(&dynamic_modifier) {
                    add_ref(&mut refs.dynamic_modifiers, &dynamic_modifier, path);
                }
            }
        }
        for dynamic_modifier in assignment_values_in_text(&cleaned, key) {
            if dynamic_modifier != "{"
                && !matches!(dynamic_modifier.as_str(), "yes" | "no")
                && is_reference_identifier(&dynamic_modifier)
            {
                add_ref(&mut refs.dynamic_modifiers, &dynamic_modifier, path);
            }
        }
    }
    if norm.contains("/common/dynamic_modifiers/") {
        collect_dynamic_modifier_definition_refs(path, &cleaned, refs);
    }
    if norm.contains("/common/scripted_effects/") {
        collect_dynamic_modifier_change_effect_refs(path, &cleaned, refs);
    }
}

pub(crate) fn is_set_technology_metadata_key(key: &str) -> bool {
    matches!(key, "popup")
}

pub(crate) fn collect_dynamic_modifier_definition_refs(
    path: &Path,
    text: &str,
    refs: &mut GameDataRefs,
) {
    for (name, block) in direct_child_blocks(text) {
        if name == "dynamic_modifiers" {
            for (_, nested) in direct_child_blocks(&block) {
                collect_dynamic_modifier_block_modifier_refs(path, &nested, refs);
            }
        } else {
            collect_dynamic_modifier_block_modifier_refs(path, &block, refs);
        }
    }
}

pub(crate) fn collect_dynamic_modifier_block_modifier_refs(
    path: &Path,
    block: &str,
    refs: &mut GameDataRefs,
) {
    for key in direct_assignment_keys(block) {
        if is_dynamic_modifier_definition_meta_key(&key) {
            continue;
        }
        if is_modifier_ref_candidate(&key) {
            add_ref(&mut refs.modifiers, &key, path);
        }
    }
}

pub(crate) fn is_dynamic_modifier_definition_meta_key(key: &str) -> bool {
    matches!(
        key,
        "icon" | "enable" | "remove_trigger" | "attacker_modifier"
    )
}

pub(crate) fn collect_dynamic_modifier_change_effect_refs(
    path: &Path,
    text: &str,
    refs: &mut GameDataRefs,
) {
    for (name, block) in direct_child_blocks(text) {
        if !name.starts_with("change_") {
            continue;
        }
        for variable_block in blocks_named(&block, "add_to_variable") {
            for key in direct_assignment_keys(&variable_block) {
                if is_reference_identifier(&key) {
                    add_ref(&mut refs.dynamic_modifier_variables, &key, path);
                }
            }
        }
        for variable_block in blocks_named(&block, "set_variable") {
            for key in direct_assignment_keys(&variable_block) {
                if is_reference_identifier(&key) {
                    add_ref(&mut refs.dynamic_modifier_variables, &key, path);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BuildingLevelRef {
    pub(crate) building: String,
    pub(crate) level: i64,
    pub(crate) path: PathBuf,
}

pub(crate) fn add_ref(refs: &mut BTreeMap<String, BTreeSet<PathBuf>>, key: &str, path: &Path) {
    refs.entry(key.to_string())
        .or_default()
        .insert(path.to_path_buf());
}

pub(crate) fn report_unknown_index_refs(
    label: &str,
    refs: &BTreeMap<String, BTreeSet<PathBuf>>,
    known: &BTreeSet<String>,
    reporter: &mut Reporter,
    as_error: bool,
    related_index: Option<(&GameIndex, &str)>,
) {
    for (key, paths) in refs {
        if !known.contains(key) {
            let related = related_index
                .map(|(index, kind)| related_code_symbols_text(index, key, Some(kind)))
                .unwrap_or_default();
            report_paths(
                reporter,
                as_error,
                format!("{label} {key} is referenced but not present in game index{related}"),
                paths,
            );
        }
    }
}

pub(crate) fn report_unknown_index_refs_if_indexed(
    label: &str,
    refs: &BTreeMap<String, BTreeSet<PathBuf>>,
    known: &BTreeSet<String>,
    reporter: &mut Reporter,
    as_error: bool,
    strict_code_index: bool,
    related_index: Option<(&GameIndex, &str)>,
) {
    if known.is_empty() {
        if strict_code_index && !refs.is_empty() {
            for (key, paths) in refs {
                report_paths(
                    reporter,
                    true,
                    format!(
                        "{label} {key} cannot be verified because the strict code index has no `{label}` entries; rebuild the index from the HOI4 game root and required dependency mods"
                    ),
                    paths,
                );
            }
        }
        return;
    }
    report_unknown_index_refs(label, refs, known, reporter, as_error, related_index);
}

pub(crate) fn report_dynamic_modifiers_used_as_ideas(
    refs: &BTreeMap<String, BTreeSet<PathBuf>>,
    index: &GameIndex,
    reporter: &mut Reporter,
    strict_code_index: bool,
) {
    if !strict_code_index {
        return;
    }
    for (idea, paths) in refs {
        if index.dynamic_modifiers.contains(idea) {
            report_paths(
                reporter,
                true,
                format!(
                    "dynamic modifier {idea} is used as a national spirit/idea reference; dynamic modifiers must use add_dynamic_modifier/remove_dynamic_modifier/has_dynamic_modifier or the dynamic_modifier_scripted_effect_protocol, not add_ideas/remove_ideas/has_idea/swap_ideas"
                ),
                paths,
            );
        }
    }
}

pub(crate) fn report_paths(
    reporter: &mut Reporter,
    as_error: bool,
    message: String,
    paths: &BTreeSet<PathBuf>,
) {
    let rendered = format!(
        "{message}: {}",
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if as_error {
        reporter.error(rendered);
    } else {
        reporter.warn(rendered);
    }
}

pub(crate) fn warn_building_levels(
    refs: &[BuildingLevelRef],
    index: &GameIndex,
    reporter: &mut Reporter,
) {
    for reference in refs {
        if let Some(max_level) = index.building_max_levels.get(&reference.building) {
            if reference.level > *max_level {
                reporter.warn(format!(
                    "{}: building type {} level {} exceeds game max_level {}",
                    reference.path.display(),
                    reference.building,
                    reference.level,
                    max_level
                ));
            }
        }
    }
}

pub(crate) fn is_reference_identifier(value: &str) -> bool {
    is_identifier_like(value) && !is_reserved_reference_value(value)
}

pub(crate) fn is_reserved_reference_value(value: &str) -> bool {
    matches!(
        value,
        "yes"
            | "no"
            | "none"
            | "all"
            | "ROOT"
            | "FROM"
            | "PREV"
            | "THIS"
            | "TAG"
            | "random"
            | "owner"
            | "controller"
    ) || value.parse::<i64>().is_ok()
}

pub(crate) fn is_modifier_ref_candidate(key: &str) -> bool {
    is_reference_identifier(key)
        && !matches!(
            key,
            "add"
                | "factor"
                | "base"
                | "tag"
                | "limit"
                | "modifier"
                | "days"
                | "value"
                | "icon"
                | "picture"
                | "allowed"
                | "available"
                | "visible"
                | "ai_will_do"
                | "target_trigger"
                | "state_trigger"
                | "target_root_trigger"
                | "always"
                | "has_war"
                | "has_completed_focus"
                | "has_idea"
                | "has_government"
                | "original_tag"
                | "is_ai"
                | "NOT"
                | "OR"
                | "AND"
        )
}

pub(crate) fn token_candidates(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | ':' | '-')))
        .filter(|token| !token.is_empty())
        .collect()
}

pub(crate) fn is_known_vanilla_gfx(sprite: &str) -> bool {
    sprite.starts_with("GFX_goal_generic_")
        || sprite.starts_with("GFX_report_event_")
        || sprite.starts_with("GFX_decision_")
        || sprite.starts_with("GFX_idea_")
        || matches!(
            sprite,
            "GFX_goal_unknown"
                | "GFX_report_event_generic"
                | "GFX_decision_category_generic_political_reform"
        )
}

#[allow(dead_code)]
pub(crate) fn is_known_sprite(
    sprite: &str,
    local_sprites: &BTreeSet<String>,
    game_index: Option<&GameIndex>,
) -> bool {
    is_known_sprite_with_options(
        sprite,
        local_sprites,
        game_index,
        ValidationOptions::default(),
    )
}

pub(crate) fn is_known_sprite_with_options(
    sprite: &str,
    local_sprites: &BTreeSet<String>,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
) -> bool {
    if options.strict_code_index {
        return local_sprites.contains(sprite)
            || game_index.is_some_and(|index| index.sprites.contains(sprite));
    }
    local_sprites.contains(sprite)
        || game_index.is_some_and(|index| index.sprites.contains(sprite))
        || is_known_vanilla_gfx(sprite)
}

pub(crate) fn is_known_localisation_key(
    key: &str,
    local_keys: &BTreeSet<String>,
    game_index: Option<&GameIndex>,
) -> bool {
    local_keys.contains(key)
        || game_index.is_some_and(|index| index.localisation_entries.contains_key(key))
}

pub(crate) fn is_dynamic_tag_ref(tag: &str) -> bool {
    matches!(tag, "TAG" | "ROOT" | "FROM" | "PREV")
}

#[allow(dead_code)]
pub(crate) fn check_script_semantics(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    reporter: &mut Reporter,
) {
    check_script_semantics_with_options(
        path,
        text,
        game_index,
        ValidationOptions::default(),
        reporter,
    )
}

pub(crate) fn check_script_semantics_with_options(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let cleaned = strip_comments(text);
    check_script_semantics_cleaned(path, &cleaned, game_index, options, reporter);
}

fn check_script_semantics_cleaned(
    path: &Path,
    cleaned: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let norm = slash_path(path);
    if !(norm.contains("/common/")
        || norm.contains("/events/")
        || norm.contains("/history/")
        || norm.ends_with(".txt"))
    {
        return;
    }
    if norm.contains("/common/national_focus/") {
        check_national_focus_fields(path, cleaned, reporter);
    }
    if norm.contains("/events/") {
        check_event_fields(path, cleaned, reporter);
    }
    if !norm.contains("/common/game_rules/") {
        check_effect_contexts(path, cleaned, game_index, options, reporter);
        check_trigger_contexts(path, cleaned, game_index, options, reporter);
    }
    check_scripted_helper_contexts(path, &norm, cleaned, game_index, options, reporter);
    check_dynamic_modifier_definition_contexts(path, &norm, cleaned, game_index, options, reporter);
    check_suspicious_assignments(path, cleaned, game_index, reporter);
}

pub(crate) fn check_scripted_helper_contexts(
    path: &Path,
    norm_path: &str,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    if norm_path.contains("/common/scripted_effects/") {
        for (name, block) in direct_child_blocks(text) {
            check_unknown_effect_keys(
                path,
                &format!("scripted_effect `{name}`"),
                &block,
                game_index,
                options,
                reporter,
            );
        }
    }
    if norm_path.contains("/common/scripted_triggers/") {
        for (name, block) in direct_child_blocks(text) {
            check_unknown_trigger_keys(
                path,
                &format!("scripted_trigger `{name}`"),
                &block,
                game_index,
                options,
                reporter,
            );
        }
    }
}

pub(crate) fn check_dynamic_modifier_definition_contexts(
    path: &Path,
    norm_path: &str,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    if !norm_path.contains("/common/dynamic_modifiers/") {
        return;
    }
    let Some(index) = game_index else {
        return;
    };
    for (name, block) in direct_child_blocks(text) {
        if name == "dynamic_modifiers" {
            for (nested_name, nested_block) in direct_child_blocks(&block) {
                check_dynamic_modifier_definition_block(
                    path,
                    &nested_name,
                    &nested_block,
                    index,
                    options,
                    reporter,
                );
            }
        } else {
            check_dynamic_modifier_definition_block(path, &name, &block, index, options, reporter);
        }
    }
}

pub(crate) fn check_dynamic_modifier_definition_block(
    path: &Path,
    name: &str,
    block: &str,
    index: &GameIndex,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    for key in direct_assignment_keys(block) {
        if is_dynamic_modifier_definition_meta_key(&key) {
            continue;
        }
        if index.modifiers.is_empty() {
            if options.strict_code_index && is_modifier_ref_candidate(&key) {
                reporter.error(format!(
                    "{}: dynamic_modifier `{name}` uses modifier-like key `{key}`, but strict code index has no indexed modifiers; rebuild the index from `documentation/modifiers_documentation.md` or load the required game/dependency code before final output",
                    path.display()
                ));
            }
        } else if is_modifier_ref_candidate(&key) && !index.modifiers.contains(&key) {
            let related = related_code_symbols_text(index, &key, Some("modifier"));
            reporter.error(format!(
                "{}: dynamic_modifier `{name}` uses unknown modifier `{key}`; use a real modifier from `documentation/modifiers_documentation.md` or verified local code{}",
                path.display(),
                related
            ));
        }
    }
    for trigger_name in ["enable", "remove_trigger"] {
        for trigger_block in blocks_named(block, trigger_name) {
            check_unknown_trigger_keys_in_block(
                path,
                &format!("dynamic_modifier `{name}` {trigger_name}"),
                &trigger_block,
                index,
                options,
                reporter,
            );
        }
    }
}

pub(crate) fn check_unresolved_generation_markers(
    path: &Path,
    text: &str,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    if !options.strict_code_index {
        return;
    }
    for (idx, line) in text.lines().enumerate() {
        let marker = unresolved_generation_marker(line);
        if let Some(marker) = marker {
            reporter.error(format!(
                "{}:{}: unresolved generated code marker `{marker}`; AI intent must be mapped to verified CLI output before final acceptance",
                path.display(),
                idx + 1
            ));
        }
    }
}

pub(crate) fn unresolved_generation_marker(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if line.contains("Needs Codex mapping before final code") {
        Some("Needs Codex mapping before final code")
    } else if line.contains("TODO raw HOI4 block") {
        Some("TODO raw HOI4 block")
    } else if line.contains("TODO unmapped line") {
        Some("TODO unmapped line")
    } else if line.contains("TODO: add effects from card text") {
        Some("TODO: add effects from card text")
    } else if line.contains("TODO: add idea modifiers from card effects") {
        Some("TODO: add idea modifiers from card effects")
    } else if line.contains("TODO: add scripted effect body") {
        Some("TODO: add scripted effect body")
    } else if line.contains("TODO: add scripted trigger body") {
        Some("TODO: add scripted trigger body")
    } else if line.contains("TODO: add state effects") {
        Some("TODO: add state effects")
    } else if line.contains("TODO: add option effects") {
        Some("TODO: add option effects")
    } else if trimmed.starts_with("# TODO:") || trimmed.starts_with("# TODO ") {
        Some("TODO generated code marker")
    } else if line.contains("<idea id for") {
        Some("<idea id for ...>")
    } else if line.contains("<event id for") {
        Some("<event id for ...>")
    } else if line.contains("<focus id for") {
        Some("<focus id for ...>")
    } else if line.contains("<number>") {
        Some("<number>")
    } else {
        None
    }
}

pub(crate) fn check_national_focus_fields(path: &Path, text: &str, reporter: &mut Reporter) {
    const CRITICAL_FOCUS_FIELDS: &[&str] = &[
        "id",
        "icon",
        "x",
        "y",
        "cost",
        "prerequisite",
        "mutually_exclusive",
        "relative_position_id",
        "available",
        "bypass",
        "cancel_if_invalid",
        "continue_if_invalid",
        "available_if_capitulated",
        "completion_reward",
        "select_effect",
        "ai_will_do",
        "search_filters",
        "allow_branch",
        "will_lead_to_war_with",
        "historical_ai",
        "offset",
        "initial_show_position",
        "dynamic",
    ];

    check_focus_tree_country_selectors(path, text, reporter);

    for block in blocks_named(text, "focus") {
        let focus_id = block_assignment(&block, "id").unwrap_or_else(|| "<unknown>".to_string());
        if is_position_fallback_focus_id(&focus_id) {
            reporter.error(format!(
                "{}: focus {focus_id} uses a generated position fallback id; replace it with a semantic focus id",
                path.display()
            ));
        }
        check_required_focus_template_fields(path, &focus_id, &block, reporter);
        for key in direct_assignment_keys(&block) {
            if CRITICAL_FOCUS_FIELDS.contains(&key.as_str()) {
                continue;
            }
            let expected = focus_field_alias(&key)
                .or_else(|| closest_critical_field(&key, CRITICAL_FOCUS_FIELDS));
            if let Some(expected) = expected {
                reporter.error(format!(
                    "{}: focus {focus_id} uses unknown near-match field `{key}`; use the exact HOI4 field `{expected}`",
                    path.display()
                ));
            }
        }
    }
}

pub(crate) fn check_focus_tree_country_selectors(path: &Path, text: &str, reporter: &mut Reporter) {
    for tree in blocks_named(text, "focus_tree") {
        let tree_id = block_assignment(&tree, "id").unwrap_or_else(|| "<unknown>".to_string());
        if block_assignment(&tree, "default_focus").is_some() {
            reporter.error(format!(
                "{}: focus_tree {tree_id} uses unsupported `default_focus`; remove it",
                path.display()
            ));
        }

        let scalar_country = direct_assignment_value(&tree, "country")
            .filter(|value| *value != "{")
            .map(str::to_string);
        if let Some(value) = scalar_country {
            reporter.error(format!(
                "{}: focus_tree {tree_id} must use `country = {{ factor = 0 modifier = {{ add = 10 tag = <TAG> }} }}`; scalar `country = {value}` is not loadable",
                path.display()
            ));
            continue;
        }

        let countries = blocks_named(&tree, "country");
        if countries.len() != 1 {
            reporter.error(format!(
                "{}: focus_tree {tree_id} must use exactly one `country = {{ factor = 0 modifier = {{ add = 10 tag = <TAG> }} }}` selector",
                path.display()
            ));
            continue;
        }

        let country = &countries[0];
        let modifiers = blocks_named(country, "modifier");
        let factor = block_assignment(country, "factor");
        let add = modifiers
            .first()
            .and_then(|modifier| block_assignment(modifier, "add"));
        let tag = modifiers
            .first()
            .and_then(|modifier| block_assignment(modifier, "tag"));
        if factor.as_deref() != Some("0")
            || modifiers.len() != 1
            || add.as_deref() != Some("10")
            || !tag.as_deref().is_some_and(looks_like_tag)
        {
            reporter.error(format!(
                "{}: focus_tree {tree_id} has an invalid country selector; use exactly `country = {{ factor = 0 modifier = {{ add = 10 tag = <TAG> }} }}`",
                path.display()
            ));
        }
        check_focus_tree_position_anchor(path, &tree_id, &tree, reporter);
    }
}

pub(crate) fn check_focus_tree_position_anchor(
    path: &Path,
    tree_id: &str,
    tree: &str,
    reporter: &mut Reporter,
) {
    let focuses = blocks_named(tree, "focus");
    let roots = focuses
        .iter()
        .filter(|focus| blocks_named(focus, "prerequisite").is_empty())
        .filter_map(|focus| block_assignment(focus, "id"))
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return;
    }
    let opening_id = &roots[0];
    for focus in focuses {
        let Some(focus_id) = block_assignment(&focus, "id") else {
            continue;
        };
        let relative_id = direct_assignment_value(&focus, "relative_position_id");
        if focus_id == *opening_id {
            if let Some(relative_id) = relative_id {
                reporter.error(format!(
                    "{}: focus_tree {tree_id} opening focus {opening_id} must use its own absolute x/y and must not set relative_position_id = {relative_id}",
                    path.display()
                ));
            }
            continue;
        }
        match relative_id {
            Some(relative_id) if relative_id == opening_id => {}
            Some(relative_id) => reporter.error(format!(
                "{}: focus_tree {tree_id} focus {focus_id} uses relative_position_id = {relative_id}; keep prerequisite links for progression, but anchor every later focus to opening focus {opening_id} and calculate x/y relative to that opening focus",
                path.display()
            )),
            None => reporter.error(format!(
                "{}: focus_tree {tree_id} focus {focus_id} is missing relative_position_id; anchor every later focus to opening focus {opening_id}",
                path.display()
            )),
        }
    }
}

pub(crate) fn check_required_focus_template_fields(
    path: &Path,
    focus_id: &str,
    block: &str,
    reporter: &mut Reporter,
) {
    for key in ["icon", "x", "y", "cost", "completion_reward"] {
        if direct_assignment_value(block, key).is_none() {
            reporter.error(format!(
                "{}: focus {focus_id} is missing required template field `{key}`",
                path.display()
            ));
        }
    }
    for key in [
        "ai_will_do",
        "available",
        "bypass",
        "cancel_if_invalid",
        "continue_if_invalid",
        "available_if_capitulated",
    ] {
        if direct_assignment_value(block, key).is_none() {
            reporter.warn(format!(
                "{}: focus {focus_id} is missing recommended generated-template field `{key}`",
                path.display()
            ));
        }
    }
}

pub(crate) fn focus_field_alias(actual: &str) -> Option<&'static str> {
    match actual {
        "mutual_exclusion" | "mutual_exclusive" | "mutually_exclusion" | "mutually_exclusives" => {
            Some("mutually_exclusive")
        }
        "relative_position" | "relative_positioning_id" => Some("relative_position_id"),
        _ => None,
    }
}

pub(crate) fn check_event_fields(path: &Path, text: &str, reporter: &mut Reporter) {
    const CRITICAL_EVENT_FIELDS: &[&str] = &[
        "id",
        "title",
        "desc",
        "picture",
        "is_triggered_only",
        "fire_only_once",
        "major",
        "show_major",
        "hidden",
        "trigger",
        "mean_time_to_happen",
        "immediate",
        "option",
        "after",
        "days",
        "timeout_days",
    ];

    for block in event_blocks(text) {
        let event_id = block_assignment(&block, "id").unwrap_or_else(|| "<unknown>".to_string());
        for key in direct_assignment_keys(&block) {
            if CRITICAL_EVENT_FIELDS.contains(&key.as_str()) {
                continue;
            }
            if let Some(expected) = closest_critical_field(&key, CRITICAL_EVENT_FIELDS) {
                reporter.error(format!(
                    "{}: event {event_id} uses unknown near-match field `{key}`; use the exact HOI4 field `{expected}`",
                    path.display()
                ));
            }
        }
    }
}

pub(crate) fn closest_critical_field<'a>(
    actual: &str,
    expected_fields: &'a [&str],
) -> Option<&'a str> {
    expected_fields
        .iter()
        .copied()
        .filter_map(|expected| {
            let distance = edit_distance(actual, expected);
            let threshold = if expected.len() <= 5 {
                1
            } else if expected.len() >= 16 {
                3
            } else {
                2
            };
            (distance <= threshold).then_some((distance, expected))
        })
        .min_by_key(|(distance, expected)| (*distance, expected.len()))
        .map(|(_, expected)| expected)
}

pub(crate) fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = Vec::with_capacity(right_chars.len() + 1);
        current.push(left_index + 1);
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            let insertion = current[right_index] + 1;
            let deletion = previous[right_index + 1] + 1;
            current.push(substitution.min(insertion).min(deletion));
        }
        previous = current;
    }
    previous[right_chars.len()]
}

pub(crate) fn direct_assignment_keys(block: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = block.as_bytes();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut escape = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_quote {
            if ch == '"' && !escape {
                in_quote = false;
            }
            if escape {
                escape = false;
            } else {
                escape = ch == '\\';
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_quote = true;
            escape = false;
            i += 1;
            continue;
        }
        if ch == '{' {
            depth += 1;
            i += 1;
            continue;
        }
        if ch == '}' {
            depth = (depth - 1).max(0);
            i += 1;
            continue;
        }
        if depth == 0 && is_identifier_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_identifier_byte(bytes[i]) {
                i += 1;
            }
            let key = &block[start..i];
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                keys.push(key.to_string());
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'{' {
                    if let Some((_, end)) = braced_content_at(block, j) {
                        i = end + 1;
                        continue;
                    }
                }
                while j < bytes.len()
                    && !(bytes[j] as char).is_whitespace()
                    && bytes[j] != b'{'
                    && bytes[j] != b'}'
                {
                    j += 1;
                }
                i = j;
                continue;
            }
            continue;
        }
        i += 1;
    }
    keys
}

pub(crate) fn check_effect_contexts(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let norm = slash_path(path);
    for name in [
        "complete_effect",
        "completion_reward",
        "hidden_effect",
        "effect_tooltip",
        "effect",
        "effects",
        "option",
        "select_effect",
    ] {
        for block in blocks_named(text, name) {
            if name == "effects" && norm.contains("/common/scripted_guis/") {
                let callback_names = direct_child_blocks(&block)
                    .into_iter()
                    .map(|(callback, _)| callback)
                    .collect::<BTreeSet<_>>();
                if let Some(index) = game_index {
                    for key in direct_assignment_keys(&block) {
                        if !callback_names.contains(&key) {
                            report_unknown_effect_key(
                                path,
                                "scripted_gui effects",
                                &key,
                                index,
                                reporter,
                            );
                        }
                    }
                }
                for (callback, callback_block) in direct_child_blocks(&block) {
                    check_unknown_effect_keys(
                        path,
                        &format!("scripted_gui effect `{callback}`"),
                        &callback_block,
                        game_index,
                        options,
                        reporter,
                    );
                }
                continue;
            }
            for event_block in blocks_named(&block, "news_event") {
                if block_assignment(&event_block, "title").is_some()
                    || block_assignment(&event_block, "desc").is_some()
                    || block_assignment(&event_block, "picture").is_some()
                    || block_assignment(&event_block, "is_triggered_only").is_some()
                {
                    reporter.warn(format!(
                        "{}: news_event definition appears inside an effect context; trigger an existing event by id instead",
                        path.display()
                    ));
                }
            }
            if !blocks_named(&block, "modifier").is_empty() {
                reporter.warn(format!(
                    "{}: modifier = {{ ... }} appears inside an effect context; modifiers belong in national spirits (`common/ideas`) or valid modifier fields. A focus completion_reward cannot apply long-term modifiers directly; create an idea and use add_ideas, then remove_ideas at the ending boundary if it is temporary",
                    path.display()
                ));
            }
            for trigger in ["has_war", "has_completed_focus", "has_idea"] {
                if direct_assignment_value(&block, trigger).is_some() {
                    reporter.warn(format!(
                        "{}: trigger-like condition `{trigger}` appears directly inside an effect context",
                        path.display()
                    ));
                }
            }
            for state_effect in [
                "add_building_construction",
                "add_extra_state_shared_building_slots",
            ] {
                if direct_assignment_value(&block, state_effect).is_some() {
                    reporter.warn(format!(
                        "{}: state effect `{state_effect}` appears directly inside an effect context; enter a state scope first",
                        path.display()
                    ));
                }
            }
            check_unknown_effect_keys(path, name, &block, game_index, options, reporter);
        }
    }
}

pub(crate) fn check_unknown_effect_keys(
    path: &Path,
    context: &str,
    block: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let Some(index) = game_index else {
        return;
    };
    if index.effects.is_empty() {
        if options.strict_code_index {
            for key in direct_assignment_keys(block) {
                report_unverifiable_effect_key(path, context, &key, reporter);
            }
            for (scope, scoped_block) in direct_child_blocks(block) {
                if is_effect_scope_key_without_effect_docs(&scope, index) {
                    for key in direct_assignment_keys(&scoped_block) {
                        report_unverifiable_effect_key(path, context, &key, reporter);
                    }
                }
            }
        }
        return;
    }
    for key in direct_assignment_keys(block) {
        report_unknown_effect_key(path, context, &key, index, reporter);
    }
    for (scope, scoped_block) in direct_child_blocks(block) {
        if is_effect_scope_key(&scope, index) {
            check_unknown_effect_keys(path, context, &scoped_block, game_index, options, reporter);
        } else if is_effect_control_block(&scope) {
            check_unknown_effect_keys(path, context, &scoped_block, game_index, options, reporter);
        }
    }
}

pub(crate) fn report_unverifiable_effect_key(
    path: &Path,
    context: &str,
    key: &str,
    reporter: &mut Reporter,
) {
    if !is_effect_key_candidate(key) {
        return;
    }
    reporter.error(format!(
        "{}: effect context `{context}` uses effect-like key `{key}`, but strict code index has no indexed effects; rebuild the index from `documentation/effects_documentation.md` or load the required game/dependency code before final output",
        path.display()
    ));
}

pub(crate) fn is_effect_scope_key_without_effect_docs(key: &str, index: &GameIndex) -> bool {
    matches!(
        key,
        "ROOT"
            | "FROM"
            | "PREV"
            | "THIS"
            | "OVERLORD"
            | "overlord"
            | "owner"
            | "controller"
            | "capital_scope"
    ) || index.country_tags.contains(key)
        || looks_like_tag(key)
}

pub(crate) fn report_unknown_effect_key(
    path: &Path,
    context: &str,
    key: &str,
    index: &GameIndex,
    reporter: &mut Reporter,
) {
    if !is_effect_key_candidate(key) {
        return;
    }
    if is_effect_scope_key(key, index) {
        return;
    }
    if index.effects.contains(key) {
        return;
    }
    let related = related_code_symbols_text(index, key, Some("effect"));
    reporter.error(format!(
        "{}: effect context `{context}` uses unknown effect `{key}`; use a real effect from `documentation/effects_documentation.md` or a verified scripted effect{}",
        path.display(),
        related
    ));
}

pub(crate) fn is_effect_scope_key(key: &str, index: &GameIndex) -> bool {
    matches!(
        key,
        "ROOT"
            | "FROM"
            | "PREV"
            | "THIS"
            | "OVERLORD"
            | "overlord"
            | "owner"
            | "controller"
            | "capital_scope"
    ) || index.country_tags.contains(key)
        || looks_like_tag(key)
        || parse_plain_i64(key).is_some()
}

pub(crate) fn is_effect_control_block(key: &str) -> bool {
    matches!(
        key,
        "if" | "else"
            | "else_if"
            | "random"
            | "random_list"
            | "ordered_country"
            | "every_country"
            | "every_other_country"
            | "every_state"
            | "every_owned_state"
            | "every_controlled_state"
            | "random_state"
            | "random_owned_state"
            | "random_controlled_state"
            | "random_owned_controlled_state"
    )
}

pub(crate) fn is_effect_key_candidate(key: &str) -> bool {
    if !is_identifier_like(key) {
        return false;
    }
    !matches!(
        key,
        "name"
            | "title"
            | "desc"
            | "picture"
            | "id"
            | "trigger"
            | "ai_chance"
            | "factor"
            | "base"
            | "modifier"
            | "limit"
            | "prioritize"
            | "tooltip"
            | "count"
            | "scope"
            | "array"
            | "var"
            | "global"
            | "days"
            | "random"
            | "is_triggered_only"
            | "fire_only_once"
            | "mean_time_to_happen"
            | "immediate"
            | "hidden_effect"
            | "option"
            | "if"
            | "else"
            | "else_if"
            | "random_list"
    )
}

pub(crate) fn check_trigger_contexts(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let norm = slash_path(path);
    for name in [
        "trigger",
        "triggers",
        "available",
        "visible",
        "allowed",
        "limit",
        "target_root_trigger",
        "target_trigger",
        "state_trigger",
    ] {
        for block in blocks_named(text, name) {
            if name == "triggers" && norm.contains("/common/scripted_guis/") {
                let Some(index) = game_index else {
                    continue;
                };
                let callback_names = direct_child_blocks(&block)
                    .into_iter()
                    .map(|(callback, _)| callback)
                    .collect::<BTreeSet<_>>();
                for key in direct_assignment_keys(&block) {
                    if !callback_names.contains(&key) {
                        report_unknown_trigger_key(
                            path,
                            "scripted_gui triggers",
                            &key,
                            index,
                            reporter,
                        );
                    }
                }
                for (callback, callback_block) in direct_child_blocks(&block) {
                    check_unknown_trigger_keys_in_block(
                        path,
                        &format!("scripted_gui trigger `{callback}`"),
                        &callback_block,
                        index,
                        options,
                        reporter,
                    );
                }
                continue;
            }
            for effect in [
                "add_political_power",
                "add_stability",
                "add_war_support",
                "set_country_flag",
                "add_building_construction",
                "add_ideas",
                "remove_ideas",
            ] {
                if direct_assignment_value(&block, effect).is_some() {
                    reporter.warn(format!(
                        "{}: effect-like command `{effect}` appears directly inside a trigger context",
                        path.display()
                    ));
                }
            }
            check_unknown_trigger_keys(path, name, &block, game_index, options, reporter);
        }
    }
}

pub(crate) fn check_unknown_trigger_keys(
    path: &Path,
    context: &str,
    block: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    let Some(index) = game_index else {
        return;
    };
    check_unknown_trigger_keys_in_block(path, context, block, index, options, reporter);
}

pub(crate) fn check_unknown_trigger_keys_in_block(
    path: &Path,
    context: &str,
    block: &str,
    index: &GameIndex,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
    for key in direct_assignment_keys(block) {
        if index.triggers.is_empty() {
            if options.strict_code_index {
                report_unverifiable_trigger_key(path, context, &key, reporter);
            }
        } else {
            report_unknown_trigger_key(path, context, &key, index, reporter);
        }
    }
    for (scope, scoped_block) in direct_child_blocks(block) {
        if is_trigger_child_context(&scope, index) || is_trigger_control_block(&scope) {
            check_unknown_trigger_keys_in_block(
                path,
                context,
                &scoped_block,
                index,
                options,
                reporter,
            );
        }
    }
}

pub(crate) fn report_unverifiable_trigger_key(
    path: &Path,
    context: &str,
    key: &str,
    reporter: &mut Reporter,
) {
    if !is_trigger_key_candidate(key) {
        return;
    }
    reporter.error(format!(
        "{}: trigger context `{context}` uses trigger-like key `{key}`, but strict code index has no indexed triggers; rebuild the index from `documentation/triggers_documentation.md` or load the required game/dependency code before final output",
        path.display()
    ));
}

pub(crate) fn report_unknown_trigger_key(
    path: &Path,
    context: &str,
    key: &str,
    index: &GameIndex,
    reporter: &mut Reporter,
) {
    if !is_trigger_key_candidate(key) {
        return;
    }
    if is_trigger_child_context(key, index) || is_trigger_control_block(key) {
        return;
    }
    if index.triggers.contains(key) {
        return;
    }
    let related = related_code_symbols_text(index, key, Some("trigger"));
    reporter.error(format!(
        "{}: trigger context `{context}` uses unknown trigger `{key}`; use a real trigger from `documentation/triggers_documentation.md` or a verified scripted trigger{}",
        path.display(),
        related
    ));
}

pub(crate) fn is_trigger_child_context(key: &str, index: &GameIndex) -> bool {
    matches!(
        key,
        "NOT"
            | "OR"
            | "AND"
            | "NOR"
            | "ROOT"
            | "FROM"
            | "PREV"
            | "THIS"
            | "OVERLORD"
            | "owner"
            | "controller"
            | "capital_scope"
    ) || index.country_tags.contains(key)
        || looks_like_tag(key)
        || parse_plain_i64(key).is_some()
}

pub(crate) fn is_trigger_control_block(key: &str) -> bool {
    matches!(key, "if" | "else" | "else_if")
}

pub(crate) fn is_trigger_key_candidate(key: &str) -> bool {
    if !is_identifier_like(key) {
        return false;
    }
    if parse_plain_i64(key).is_some() {
        return false;
    }
    !matches!(
        key,
        "name"
            | "title"
            | "desc"
            | "picture"
            | "id"
            | "trigger"
            | "available"
            | "visible"
            | "allowed"
            | "limit"
            | "target_root_trigger"
            | "target_trigger"
            | "state_trigger"
            | "custom_trigger_tooltip"
            | "tooltip"
            | "prioritize"
            | "count"
            | "scope"
            | "array"
            | "var"
            | "global"
            | "factor"
            | "base"
            | "modifier"
            | "add"
            | "tag"
            | "value"
            | "days"
            | "always"
            | "original_tag"
            | "is_ai"
            | "NOT"
            | "OR"
            | "AND"
            | "NOR"
            | "ROOT"
            | "FROM"
            | "PREV"
            | "THIS"
            | "OVERLORD"
            | "owner"
            | "controller"
            | "if"
            | "else"
            | "else_if"
    )
}

pub(crate) fn check_suspicious_assignments(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    reporter: &mut Reporter,
) {
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = assignment_value(trimmed, "add_core_of") {
            if value.parse::<i64>().is_ok() {
                reporter.warn(format!(
                    "{}: add_core_of usually expects a country tag, got numeric value `{value}`",
                    path.display()
                ));
            }
        }
        if let Some(value) = assignment_value(trimmed, "add_core") {
            if looks_like_tag(value) {
                reporter.warn(format!(
                    "{}: add_core usually expects a state/province target, got tag-like value `{value}`; check add_core/add_core_of direction",
                    path.display()
                ));
            }
        }
        if let Some(value) = assignment_value(trimmed, "capital") {
            if let Ok(capital) = value.parse::<i64>() {
                if let Some(index) = game_index {
                    if index.state_ids.contains(&capital) {
                        reporter.warn(format!(
                            "{}: capital = {value} matches a known state id; verify HOI4 expects province id here",
                            path.display()
                        ));
                    } else if !index.province_ids.is_empty()
                        && !index.province_ids.contains(&capital)
                    {
                        reporter.warn(format!(
                            "{}: capital = {value} is not present in the province index; verify the capital province id",
                            path.display()
                        ));
                    }
                } else {
                    reporter.warn(format!(
                        "{}: capital = {value} cannot be verified without game data; confirm it is the intended capital id",
                        path.display()
                    ));
                }
            }
        }
    }
}

pub(crate) fn direct_assignment_value<'a>(block: &'a str, key: &str) -> Option<&'a str> {
    let mut depth: i32 = 0;
    for line in block.lines() {
        let trimmed = line.trim();
        if depth == 0 {
            if let Some(value) = assignment_value(trimmed, key) {
                return Some(value);
            }
        }
        depth += trimmed.chars().filter(|c| *c == '{').count() as i32;
        depth -= trimmed.chars().filter(|c| *c == '}').count() as i32;
        depth = depth.max(0);
    }
    None
}

pub(crate) fn looks_like_tag(value: &str) -> bool {
    value.len() == 3 && value.chars().all(|c| c.is_ascii_uppercase())
}

pub(crate) fn assignment_key(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let (key, _) = trimmed.split_once('=')?;
    let key = key.trim();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '.'))
    {
        None
    } else {
        Some(key)
    }
}

pub(crate) fn is_identifier_like(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

pub(crate) fn assignment_values_in_text(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find(key) {
        let before_ok = idx == 0
            || rest[..idx]
                .chars()
                .last()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after_key = &rest[idx + key.len()..];
        let after_ok = after_key
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '=');
        if before_ok && after_ok {
            if let Some(value) = after_key.trim_start().strip_prefix('=') {
                let parsed = read_assignment_value(value.trim_start());
                if !parsed.is_empty() {
                    values.push(parsed.to_string());
                }
            }
        }
        rest = after_key;
    }
    values
}

pub(crate) fn assignment_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    Some(
        rest.trim_matches('"')
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_matches('"'),
    )
    .filter(|s| !s.is_empty())
}
