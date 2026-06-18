//! Project-level localisation audit for large mods.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_loc_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let changed_files = loc_audit_changed_files(&resolved.root, &map)?;
    if map.flags.contains("changed-only") && changed_files.is_empty() {
        return Err("--changed-only requires at least one --changed <path>".to_string());
    }
    let mut report = audit_localisation(&resolved.root)?;
    if map.flags.contains("changed-only") {
        report.filter_changed(&changed_files);
    }
    let json = loc_audit_json(&resolved, &report, &changed_files, max_items);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_loc_sync_report(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let from = normalise_localisation_language(value(&map, "from").unwrap_or("english"))?;
    let to = normalise_localisation_language(value(&map, "to").unwrap_or("simp_chinese"))?;
    if from == to {
        return Err("--from and --to must be different languages".to_string());
    }
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let report = build_loc_sync_report(&resolved.root, &from, &to)?;
    let json = loc_sync_report_json(&resolved, &report, max_items);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Default)]
struct LocAuditReport {
    languages: BTreeMap<String, i64>,
    files_total: usize,
    keys_total: usize,
    refs_total: usize,
    missing: Vec<LocIssue>,
    orphan: Vec<LocIssue>,
    duplicate: Vec<LocIssue>,
}

#[derive(Clone)]
struct LocIssue {
    key: String,
    files: Vec<String>,
}

struct LocSyncReport {
    from: String,
    to: String,
    from_files_total: usize,
    to_files_total: usize,
    from_keys_total: usize,
    to_keys_total: usize,
    common_count: usize,
    missing_in_to: Vec<LocIssue>,
    extra_in_to: Vec<LocIssue>,
    duplicate_from: Vec<LocIssue>,
    duplicate_to: Vec<LocIssue>,
    warnings: Vec<String>,
    suggested_commands: Vec<String>,
}

impl LocAuditReport {
    fn filter_changed(&mut self, changed_files: &[String]) {
        self.missing
            .retain(|issue| loc_issue_touches_changed(issue, changed_files));
        self.orphan
            .retain(|issue| loc_issue_touches_changed(issue, changed_files));
        self.duplicate
            .retain(|issue| loc_issue_touches_changed(issue, changed_files));
    }
}

fn audit_localisation(root: &Path) -> Result<LocAuditReport, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let mut defined: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut all_locations: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut languages = BTreeMap::<String, i64>::new();
    let files_total =
        collect_localisation_definitions(root, &mut defined, &mut all_locations, &mut languages)?;
    let refs = collect_project_localisation_refs(root)?;

    let missing = refs
        .iter()
        .filter(|(key, _)| !defined.contains_key(*key))
        .map(|(key, files)| LocIssue {
            key: key.clone(),
            files: files.iter().cloned().collect(),
        })
        .collect::<Vec<_>>();
    let orphan = defined
        .iter()
        .filter(|(key, _)| !refs.contains_key(*key))
        .map(|(key, files)| LocIssue {
            key: key.clone(),
            files: files.iter().cloned().collect(),
        })
        .collect::<Vec<_>>();
    let duplicate = all_locations
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(key, files)| LocIssue {
            key: key.clone(),
            files: files.clone(),
        })
        .collect::<Vec<_>>();

    Ok(LocAuditReport {
        languages,
        files_total,
        keys_total: defined.len(),
        refs_total: refs.len(),
        missing,
        orphan,
        duplicate,
    })
}

fn build_loc_sync_report(root: &Path, from: &str, to: &str) -> Result<LocSyncReport, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let from_defs = collect_language_localisation_definitions(root, from)?;
    let to_defs = collect_language_localisation_definitions(root, to)?;
    let from_keys = from_defs.keys.keys().cloned().collect::<BTreeSet<_>>();
    let to_keys = to_defs.keys.keys().cloned().collect::<BTreeSet<_>>();
    let missing_in_to = from_keys
        .difference(&to_keys)
        .map(|key| LocIssue {
            key: key.clone(),
            files: from_defs.keys.get(key).cloned().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let extra_in_to = to_keys
        .difference(&from_keys)
        .map(|key| LocIssue {
            key: key.clone(),
            files: to_defs.keys.get(key).cloned().unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let duplicate_from = duplicate_loc_issues(&from_defs.locations);
    let duplicate_to = duplicate_loc_issues(&to_defs.locations);

    let mut warnings = Vec::new();
    if from_defs.files_total == 0 {
        warnings.push(format!(
            "source language `{from}` has no localisation files"
        ));
    }
    if to_defs.files_total == 0 {
        warnings.push(format!("target language `{to}` has no localisation files"));
    }
    if !missing_in_to.is_empty() {
        warnings.push(format!(
            "{} key(s) exist in `{from}` but are missing in `{to}`",
            missing_in_to.len()
        ));
    }
    if !duplicate_from.is_empty() || !duplicate_to.is_empty() {
        warnings.push(
            "duplicate localisation keys should be resolved before translation sync".to_string(),
        );
    }

    Ok(LocSyncReport {
        from: from.to_string(),
        to: to.to_string(),
        from_files_total: from_defs.files_total,
        to_files_total: to_defs.files_total,
        from_keys_total: from_keys.len(),
        to_keys_total: to_keys.len(),
        common_count: from_keys.intersection(&to_keys).count(),
        missing_in_to,
        extra_in_to,
        duplicate_from,
        duplicate_to,
        warnings,
        suggested_commands: vec![
            format!("hoi4skill translate-localisation --mod-root <mod-root> --from {from} --to {to} --format prompt --output loc_sync_{from}_to_{to}.md"),
            format!("hoi4skill loc-audit <mod-root> --output loc_audit_{from}_{to}.json"),
            "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
        ],
    })
}

struct LanguageLocalisationDefinitions {
    files_total: usize,
    keys: BTreeMap<String, Vec<String>>,
    locations: BTreeMap<String, Vec<String>>,
}

fn collect_language_localisation_definitions(
    root: &Path,
    language: &str,
) -> Result<LanguageLocalisationDefinitions, String> {
    let language_root = root.join("localisation").join(language);
    let mut files_total = 0;
    let mut keys = BTreeMap::<String, Vec<String>>::new();
    let mut locations = BTreeMap::<String, Vec<String>>::new();
    if !language_root.exists() {
        return Ok(LanguageLocalisationDefinitions {
            files_total,
            keys,
            locations,
        });
    }
    for file in collect_files(&language_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        files_total += 1;
        let rel = rel_slash(root, &file);
        for (line_no, key) in localisation_keys_with_lines(&read_utf8_lossy(&file)?) {
            keys.entry(key.clone()).or_default().push(rel.clone());
            locations
                .entry(key)
                .or_default()
                .push(format!("{rel}:{line_no}"));
        }
    }
    Ok(LanguageLocalisationDefinitions {
        files_total,
        keys,
        locations,
    })
}

fn duplicate_loc_issues(locations: &BTreeMap<String, Vec<String>>) -> Vec<LocIssue> {
    locations
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(key, files)| LocIssue {
            key: key.clone(),
            files: files.clone(),
        })
        .collect()
}

fn collect_localisation_definitions(
    root: &Path,
    defined: &mut BTreeMap<String, BTreeSet<String>>,
    all_locations: &mut BTreeMap<String, Vec<String>>,
    languages: &mut BTreeMap<String, i64>,
) -> Result<usize, String> {
    let loc_root = root.join("localisation");
    if !loc_root.exists() {
        return Ok(0);
    }
    let mut files_total = 0;
    for file in collect_files(&loc_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        files_total += 1;
        let rel = rel_slash(root, &file);
        if let Some(language) = loc_language_from_rel(&rel) {
            *languages.entry(language).or_default() += 1;
        }
        for (line_no, key) in localisation_keys_with_lines(&read_utf8_lossy(&file)?) {
            defined.entry(key.clone()).or_default().insert(rel.clone());
            all_locations
                .entry(key)
                .or_default()
                .push(format!("{rel}:{line_no}"));
        }
    }
    Ok(files_total)
}

fn collect_project_localisation_refs(
    root: &Path,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut refs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut reporter = Reporter::default();
    for file in collect_files(root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "txt" | "gui" | "gfx" | "asset") {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut path_refs = BTreeMap::<String, BTreeSet<PathBuf>>::new();
        collect_localisation_refs(&file, &text, &mut path_refs, &mut reporter);
        for (key, paths) in path_refs {
            for path in paths {
                refs.entry(key.clone())
                    .or_default()
                    .insert(rel_slash(root, &path));
            }
        }
    }

    let localisation = BTreeMap::new();
    for idea in import_ideas(root, &localisation)? {
        add_loc_audit_ref(&mut refs, &idea.id, &idea.file);
        add_loc_audit_ref(&mut refs, &format!("{}_desc", idea.id), &idea.file);
    }
    for category in import_decision_categories(root, &localisation)? {
        add_loc_audit_ref(&mut refs, &category.id, &category.file);
        add_loc_audit_ref(&mut refs, &format!("{}_desc", category.id), &category.file);
    }
    for decision in import_decisions(root, &localisation)? {
        add_loc_audit_ref(&mut refs, &decision.id, &decision.file);
        add_loc_audit_ref(&mut refs, &format!("{}_desc", decision.id), &decision.file);
    }
    Ok(refs)
}

fn add_loc_audit_ref(refs: &mut BTreeMap<String, BTreeSet<String>>, key: &str, file: &str) {
    refs.entry(key.to_string())
        .or_default()
        .insert(file.to_string());
}

fn localisation_keys_with_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with("l_") && trimmed.ends_with(':') {
            continue;
        }
        if let Some(colon) = trimmed.find(':') {
            let key = trimmed[..colon].trim();
            if !key.is_empty() {
                out.push((idx + 1, key.to_string()));
            }
        }
    }
    out
}

fn loc_audit_json(
    resolved: &ModRootResolution,
    report: &LocAuditReport,
    changed_files: &[String],
    max_items: usize,
) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.loc_audit.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"files_total\": {},\n  \"keys_total\": {},\n  \"refs_total\": {},\n  \"missing_count\": {},\n  \"orphan_count\": {},\n  \"duplicate_count\": {},\n  \"changed_files\": {},\n  \"languages\": {},\n  \"missing\": {},\n  \"orphan\": {},\n  \"duplicate\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        report.files_total,
        report.keys_total,
        report.refs_total,
        report.missing.len(),
        report.orphan.len(),
        report.duplicate.len(),
        json_array(changed_files),
        json_i64_object(&report.languages),
        loc_issues_json(&report.missing, max_items),
        loc_issues_json(&report.orphan, max_items),
        loc_issues_json(&report.duplicate, max_items)
    )
}

fn loc_sync_report_json(
    resolved: &ModRootResolution,
    report: &LocSyncReport,
    max_items: usize,
) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.loc_sync_report.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"from\": {},\n  \"to\": {},\n  \"from_files_total\": {},\n  \"to_files_total\": {},\n  \"from_keys_total\": {},\n  \"to_keys_total\": {},\n  \"common_count\": {},\n  \"missing_in_to_count\": {},\n  \"extra_in_to_count\": {},\n  \"duplicate_from_count\": {},\n  \"duplicate_to_count\": {},\n  \"missing_in_to\": {},\n  \"extra_in_to\": {},\n  \"duplicate_from\": {},\n  \"duplicate_to\": {},\n  \"warnings\": {},\n  \"suggested_commands\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_str(&report.from),
        json_str(&report.to),
        report.from_files_total,
        report.to_files_total,
        report.from_keys_total,
        report.to_keys_total,
        report.common_count,
        report.missing_in_to.len(),
        report.extra_in_to.len(),
        report.duplicate_from.len(),
        report.duplicate_to.len(),
        loc_issues_json(&report.missing_in_to, max_items),
        loc_issues_json(&report.extra_in_to, max_items),
        loc_issues_json(&report.duplicate_from, max_items),
        loc_issues_json(&report.duplicate_to, max_items),
        json_array(&report.warnings),
        json_array(&report.suggested_commands)
    )
}

fn loc_issues_json(issues: &[LocIssue], max_items: usize) -> String {
    format!(
        "[{}]",
        issues
            .iter()
            .take(max_items)
            .map(|issue| {
                format!(
                    "{{\"key\": {}, \"files\": {}}}",
                    json_str(&issue.key),
                    json_array(&issue.files)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn loc_audit_changed_files(root: &Path, map: &ArgMap) -> Result<Vec<String>, String> {
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

fn loc_issue_touches_changed(issue: &LocIssue, changed_files: &[String]) -> bool {
    issue.files.iter().any(|file| {
        changed_files
            .iter()
            .any(|changed| file == changed || file.starts_with(&format!("{changed}:")))
    })
}

fn loc_language_from_rel(rel: &str) -> Option<String> {
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.len() >= 2 && parts[0] == "localisation" {
        Some(parts[1].to_string())
    } else {
        None
    }
}
