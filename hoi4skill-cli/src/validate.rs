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
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
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
    let report = validation_report_from_args(&root, &map, &reporter)?;
    report.effective_reporter.print();
    if let Some(output) = value(&map, "output") {
        write_or_print(&validation_report_json(&report), Some(output))?;
    }
    if report.effective_reporter.errors.is_empty() {
        Ok(())
    } else {
        Err("validation failed".to_string())
    }
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
    let mut ids: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut namespaces: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut localisation_keys: BTreeSet<String> = BTreeSet::new();
    let mut localisation_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut sprite_names: BTreeSet<String> = BTreeSet::new();
    let mut gfx_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
    let mut idea_picture_refs: BTreeMap<String, BTreeSet<PathBuf>> = BTreeMap::new();
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
                check_braces(&file, &text, &mut reporter);
                collect_ids_and_namespaces(&file, &text, &mut ids, &mut namespaces, &mut reporter);
                collect_localisation_refs(&file, &text, &mut localisation_refs, &mut reporter);
                if ext == "gfx" && norm.contains("/interface/") {
                    collect_sprite_names(&text, &mut sprite_names);
                    check_sprite_textures(root, &file, &text, &mut reporter);
                } else {
                    collect_gfx_refs(&file, &text, &mut gfx_refs);
                }
                collect_idea_picture_refs(&file, &text, &mut idea_picture_refs, &mut reporter);
                collect_country_tag_refs(&file, &text, &mut tag_refs);
                collect_focus_refs(&file, &text, &mut focus_refs, &mut local_focus_ids);
                collect_game_data_refs(&file, &text, &mut game_data_refs);
                check_script_semantics_with_options(
                    &file,
                    &text,
                    game_index,
                    options,
                    &mut reporter,
                );
                check_unresolved_generation_markers(&file, &text, options, &mut reporter);
            } else if matches!(ext.as_str(), "yml" | "yaml") {
                let text = read_utf8_lossy(&file)?;
                if norm.contains("/localisation/") {
                    check_localisation(&file, &mut reporter);
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
        if !localisation_keys.contains(&key) {
            reporter.warn(format!(
                "localisation key {key} is referenced but not defined in this mod: {}",
                paths
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    for (sprite, paths) in gfx_refs {
        if !is_known_sprite_with_options(&sprite, &sprite_names, game_index, options) {
            report_paths(
                &mut reporter,
                game_index.is_some() || options.strict_code_index,
                format!(
                    "GFX key {sprite} is referenced but not defined in this mod or indexed roots"
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

#[derive(Default)]
pub(crate) struct Reporter {
    pub(crate) errors: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

struct ValidationReport {
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
        total_errors: reporter.errors.len(),
        total_warnings: reporter.warnings.len(),
        baseline_errors,
        baseline_warnings,
        changed_files,
        effective_reporter: Reporter { errors, warnings },
    })
}

fn validation_report_json(report: &ValidationReport) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.validation_report.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"total_errors\": {},\n  \"total_warnings\": {},\n  \"effective_errors\": {},\n  \"effective_warnings\": {},\n  \"baseline_errors_filtered\": {},\n  \"baseline_warnings_filtered\": {},\n  \"changed_files\": {},\n  \"errors\": {},\n  \"warnings\": {}\n}}\n",
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
        report.total_errors,
        report.total_warnings,
        report.effective_reporter.errors.len(),
        report.effective_reporter.warnings.len(),
        report.baseline_errors,
        report.baseline_warnings,
        json_array(&report.changed_files),
        json_array(&report.effective_reporter.errors),
        json_array(&report.effective_reporter.warnings)
    )
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
    let mut files = Vec::new();
    for raw in repeated_values(map, "changed") {
        let path = PathBuf::from(raw);
        let rel = if path.is_absolute() {
            relative_slash_path(root, &path)
        } else {
            slash_path(&path)
        };
        files.push(rel);
    }
    Ok(files)
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
    check_yaml_duplicate_keys(path, &text, reporter);
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
    for (line_idx, line) in text.lines().enumerate() {
        let line_no = line_idx + 1;
        let line = line.trim_start_matches('\u{feff}');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let mut content = line.trim_start();
        if let Some(rest) = content.strip_prefix("- ") {
            content = rest.trim_start();
        }
        let Some(colon) = content.find(':') else {
            continue;
        };
        let key = content[..colon].trim().trim_matches('"').trim_matches('\'');
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        while stack.last().is_some_and(|(level, _)| *level >= indent) {
            stack.pop();
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
        stack.push((indent, key.to_string()));
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
        blocks.extend(blocks_named(text, key));
    }
    blocks
}

pub(crate) fn check_sprite_textures(root: &Path, path: &Path, text: &str, reporter: &mut Reporter) {
    for block in sprite_type_blocks(text) {
        if let Some(texturefile) = block_assignment(&block, "texturefile") {
            if resolve_texture(root, &texturefile).is_none() {
                reporter.warn(format!(
                    "{}: sprite texturefile not found in this mod: {}",
                    path.display(),
                    texturefile
                ));
            }
        }
    }
}

pub(crate) fn collect_sprite_names(text: &str, sprite_names: &mut BTreeSet<String>) {
    for block in sprite_type_blocks(text) {
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
    for token in token_candidates(&strip_comments(text)) {
        if token.starts_with("GFX_") {
            refs.entry(token.to_string())
                .or_default()
                .insert(path.to_path_buf());
        }
    }
}

pub(crate) fn collect_idea_picture_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    reporter: &mut Reporter,
) {
    if !slash_path(path).contains("/common/ideas/") {
        return;
    }
    for picture in assignment_values_in_text(&strip_comments(text), "picture") {
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

pub(crate) fn collect_country_tag_refs(
    path: &Path,
    text: &str,
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
    let cleaned = strip_comments(text);
    for key in [
        "tag",
        "original_tag",
        "owner",
        "controller",
        "add_core_of",
        "set_cosmetic_tag",
    ] {
        for value in assignment_values_in_text(&cleaned, key) {
            if looks_like_tag(&value) {
                refs.entry(value).or_default().insert(path.to_path_buf());
            }
        }
    }
}

pub(crate) fn collect_focus_refs(
    path: &Path,
    text: &str,
    refs: &mut BTreeMap<String, BTreeSet<PathBuf>>,
    local_ids: &mut BTreeSet<String>,
) {
    let norm = slash_path(path);
    if !norm.contains("/common/national_focus/") {
        return;
    }
    let cleaned = strip_comments(text);
    for block in blocks_named(&cleaned, "focus") {
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
}

pub(crate) fn collect_game_data_refs(path: &Path, text: &str, refs: &mut GameDataRefs) {
    let norm = slash_path(path);
    if !(norm.contains("/common/")
        || norm.contains("/events/")
        || norm.contains("/history/")
        || norm.ends_with(".txt"))
    {
        return;
    }
    let cleaned = strip_comments(text);
    for block in blocks_named(&cleaned, "add_building_construction") {
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
    for block in blocks_named(&cleaned, "add_resource") {
        if let Some(resource) = block_assignment(&block, "type") {
            add_ref(&mut refs.resources, &resource, path);
        }
    }
    for key in ["ruling_party", "ideology"] {
        for ideology in assignment_values_in_text(&cleaned, key) {
            if is_identifier_like(&ideology) {
                add_ref(&mut refs.ideologies, &ideology, path);
            }
        }
    }
    for block in blocks_named(&cleaned, "traits") {
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
            if is_reference_identifier(&key) {
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
    let norm = slash_path(path);
    if !(norm.contains("/common/")
        || norm.contains("/events/")
        || norm.contains("/history/")
        || norm.ends_with(".txt"))
    {
        return;
    }
    let cleaned = strip_comments(text);
    if norm.contains("/common/national_focus/") {
        check_national_focus_fields(path, &cleaned, reporter);
    }
    if norm.contains("/events/") {
        check_event_fields(path, &cleaned, reporter);
    }
    check_effect_contexts(path, &cleaned, game_index, options, reporter);
    check_trigger_contexts(path, &cleaned, game_index, options, reporter);
    check_scripted_helper_contexts(path, &norm, &cleaned, game_index, options, reporter);
    check_suspicious_assignments(path, &cleaned, game_index, reporter);
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
    for key in [
        "icon",
        "x",
        "y",
        "cost",
        "ai_will_do",
        "available",
        "bypass",
        "cancel_if_invalid",
        "continue_if_invalid",
        "available_if_capitulated",
        "completion_reward",
    ] {
        if direct_assignment_value(block, key).is_none() {
            reporter.error(format!(
                "{}: focus {focus_id} is missing required template field `{key}`",
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
    for name in [
        "complete_effect",
        "completion_reward",
        "hidden_effect",
        "effect",
        "effects",
        "option",
        "select_effect",
    ] {
        for block in blocks_named(text, name) {
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
            for key in direct_assignment_keys(&scoped_block) {
                report_unknown_effect_key(path, context, &key, index, reporter);
            }
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
    matches!(key, "ROOT" | "FROM" | "PREV" | "THIS")
        || index.country_tags.contains(key)
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
    matches!(key, "ROOT" | "FROM" | "PREV" | "THIS")
        || index.country_tags.contains(key)
        || looks_like_tag(key)
        || parse_plain_i64(key).is_some()
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
            | "days"
            | "random"
            | "is_triggered_only"
            | "fire_only_once"
            | "mean_time_to_happen"
            | "immediate"
            | "option"
    )
}

pub(crate) fn check_trigger_contexts(
    path: &Path,
    text: &str,
    game_index: Option<&GameIndex>,
    options: ValidationOptions,
    reporter: &mut Reporter,
) {
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
        if is_trigger_child_context(&scope, index) {
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
        "NOT" | "OR" | "AND" | "NOR" | "ROOT" | "FROM" | "PREV" | "THIS"
    ) || index.country_tags.contains(key)
        || looks_like_tag(key)
}

pub(crate) fn is_trigger_key_candidate(key: &str) -> bool {
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
            | "available"
            | "visible"
            | "allowed"
            | "limit"
            | "target_root_trigger"
            | "target_trigger"
            | "state_trigger"
            | "custom_trigger_tooltip"
            | "tooltip"
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
