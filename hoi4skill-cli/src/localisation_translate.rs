//! Localisation translation planning and scaffolding.
//!
//! The CLI does not pretend to be a machine-translation engine. It extracts
//! source localisation, filters keys that already exist in the target language,
//! and renders either an AI translation prompt or a target-language YAML
//! scaffold.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Debug)]
pub(crate) struct LocalisationTranslationEntry {
    pub(crate) key: String,
    pub(crate) value: String,
    pub(crate) source_file: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalisationSourceFile {
    pub(crate) path: PathBuf,
    pub(crate) relative: String,
    pub(crate) entries: Vec<LocalisationTranslationEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocalisationTranslationFormat {
    Prompt,
    Yml,
    Json,
}

pub(crate) fn cmd_translate_localisation(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let from = normalise_localisation_language(value(&map, "from").unwrap_or("english"))?;
    let to = normalise_localisation_language(value(&map, "to").unwrap_or("simp_chinese"))?;
    let format = parse_translation_format(value(&map, "format").unwrap_or("prompt"))?;
    let max_items = parse_usize_option(&map, "max-items", usize::MAX)?;
    let include_existing = map.flags.contains("include-existing");
    let overwrite = map.flags.contains("overwrite");
    let translated_inputs = repeated_values(&map, "translated-input")
        .into_iter()
        .chain(repeated_values(&map, "translation"))
        .chain(repeated_values(&map, "translated"))
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    let apply = map.flags.contains("apply") || !translated_inputs.is_empty();
    let key_prefixes = repeated_values(&map, "key-prefix")
        .into_iter()
        .chain(repeated_values(&map, "prefix"))
        .map(str::to_string)
        .collect::<Vec<_>>();

    let mod_root = value(&map, "mod-root")
        .or_else(|| map.positionals.first().map(String::as_str))
        .map(normalize_path)
        .transpose()?;
    let source_roots = source_localisation_roots(&map, mod_root.as_deref(), &from)?;
    if apply {
        let Some(mod_root) = mod_root.as_deref() else {
            return Err(
                "--apply requires --mod-root so translated keys can be written back".to_string(),
            );
        };
        if translated_inputs.is_empty() {
            return Err(
                "--apply requires at least one --translated-input <file-or-dir>".to_string(),
            );
        }
        let translations = collect_translated_localisation_map(&translated_inputs)?;
        let source_files = collect_localisation_source_files(
            Some(mod_root),
            &source_roots,
            &key_prefixes,
            &BTreeSet::new(),
            max_items,
        )?;
        let report = apply_localisation_translations(
            mod_root,
            &source_files,
            &from,
            &to,
            &translations,
            overwrite,
        )?;
        return write_or_print(
            &report,
            value(&map, "report").or_else(|| value(&map, "output")),
        );
    }

    let target_existing = if include_existing {
        BTreeSet::new()
    } else {
        target_existing_keys(mod_root.as_deref(), &to)?
    };
    let source_files = collect_localisation_source_files(
        mod_root.as_deref(),
        &source_roots,
        &key_prefixes,
        &target_existing,
        max_items,
    )?;
    let entries_total = source_files
        .iter()
        .map(|file| file.entries.len())
        .sum::<usize>();

    match format {
        LocalisationTranslationFormat::Prompt => {
            let prompt = render_localisation_translation_prompt(&from, &to, &source_files);
            write_or_print(&prompt, value(&map, "output"))
        }
        LocalisationTranslationFormat::Json => {
            let json = localisation_translation_json(&from, &to, &source_files, entries_total);
            write_or_print(&json, value(&map, "output"))
        }
        LocalisationTranslationFormat::Yml => {
            if let Some(output_dir) = value(&map, "output-dir") {
                let output_dir = normalize_path(output_dir)?;
                let report =
                    write_translation_yml_files(&source_files, &from, &to, &output_dir, overwrite)?;
                write_or_print(&report, value(&map, "report"))
            } else {
                let yml = render_localisation_translation_yml(
                    &to,
                    &all_translation_entries(&source_files),
                );
                write_or_print(&yml, value(&map, "output"))
            }
        }
    }
}

pub(crate) fn collect_translated_localisation_map(
    inputs: &[PathBuf],
) -> Result<BTreeMap<String, String>, String> {
    let mut translations = BTreeMap::new();
    for input in inputs {
        for file in collect_localisation_files(input)? {
            let text = read_utf8_lossy(&file)?;
            for line in text.lines() {
                if let Some((key, value)) = parse_localisation_line(line) {
                    translations.insert(key, value);
                }
            }
        }
    }
    Ok(translations)
}

pub(crate) fn apply_localisation_translations(
    mod_root: &Path,
    source_files: &[LocalisationSourceFile],
    from: &str,
    to: &str,
    translations: &BTreeMap<String, String>,
    overwrite: bool,
) -> Result<String, String> {
    let target_dir = mod_root.join("localisation").join(to);
    fs::create_dir_all(&target_dir).map_err(|e| format!("create {}: {e}", target_dir.display()))?;

    let source_entries = all_translation_entries(source_files);
    let source_keys = source_entries
        .iter()
        .map(|entry| entry.key.clone())
        .collect::<BTreeSet<_>>();
    let existing_before = target_existing_keys(Some(mod_root), to)?;
    let mut grouped: BTreeMap<PathBuf, BTreeMap<String, String>> = BTreeMap::new();
    let mut written_keys = Vec::new();
    let mut existing_keys = Vec::new();
    let mut missing_keys = Vec::new();
    let mut suspicious_same_as_source = Vec::new();

    for entry in &source_entries {
        if existing_before.contains(&entry.key) && !overwrite {
            existing_keys.push(entry.key.clone());
            continue;
        }
        let Some(translated_value) = translations.get(&entry.key) else {
            if !existing_before.contains(&entry.key) {
                missing_keys.push(entry.key.clone());
            }
            continue;
        };
        if translated_value.trim() == entry.value.trim() && from != to {
            suspicious_same_as_source.push(entry.key.clone());
        }
        let target = target_dir.join(target_file_name_for_source(entry, from, to));
        grouped
            .entry(target)
            .or_default()
            .insert(entry.key.clone(), translated_value.clone());
        written_keys.push(entry.key.clone());
    }

    let mut written_files = Vec::new();
    let mut updated_keys = Vec::new();
    let mut appended_keys = Vec::new();
    for (target, entries) in grouped {
        let result = upsert_localisation_entries(&target, to, &entries, overwrite)?;
        if result.changed {
            written_files.push(slash_path(&target));
        }
        updated_keys.extend(result.updated_keys);
        appended_keys.extend(result.appended_keys);
    }

    let after = target_existing_keys(Some(mod_root), to)?;
    let missing_after_apply = source_keys
        .iter()
        .filter(|key| !after.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    let translated_unused_keys = translations
        .keys()
        .filter(|key| !source_keys.contains(*key))
        .cloned()
        .collect::<Vec<_>>();

    Ok(format!(
        concat!(
            "{{\"schema\": \"hoi4skill.localisation_translate.apply.v1\", ",
            "\"from\": {}, \"to\": {}, \"source_keys_total\": {}, \"translated_keys_total\": {}, ",
            "\"written_files\": {}, \"written_keys\": {}, \"appended_keys\": {}, \"updated_keys\": {}, ",
            "\"existing_keys\": {}, \"missing_keys\": {}, \"missing_after_apply\": {}, ",
            "\"translated_unused_keys\": {}, \"suspicious_same_as_source\": {}}}\n"
        ),
        json_str(from),
        json_str(to),
        source_keys.len(),
        translations.len(),
        json_array(&written_files),
        json_array(&written_keys),
        json_array(&appended_keys),
        json_array(&updated_keys),
        json_array(&existing_keys),
        json_array(&missing_keys),
        json_array(&missing_after_apply),
        json_array(&translated_unused_keys),
        json_array(&suspicious_same_as_source)
    ))
}

pub(crate) fn target_file_name_for_source(
    entry: &LocalisationTranslationEntry,
    from: &str,
    to: &str,
) -> String {
    Path::new(&entry.source_file)
        .file_name()
        .and_then(OsStr::to_str)
        .map(|name| target_localisation_output_name(name, from, to))
        .unwrap_or_else(|| format!("translated_l_{to}.yml"))
}

pub(crate) struct LocalisationUpsertResult {
    pub(crate) changed: bool,
    pub(crate) appended_keys: Vec<String>,
    pub(crate) updated_keys: Vec<String>,
}

pub(crate) fn upsert_localisation_entries(
    path: &Path,
    language: &str,
    entries: &BTreeMap<String, String>,
    overwrite: bool,
) -> Result<LocalisationUpsertResult, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let header = format!("l_{language}:");
    let mut text = if path.exists() {
        read_utf8_lossy(path)?
    } else {
        format!("{header}\n")
    };
    if !text
        .lines()
        .any(|line| line.trim_start_matches('\u{feff}').trim() == header)
    {
        text = format!("{header}\n{text}");
    }

    let mut remaining = entries.clone();
    let mut updated_keys = Vec::new();
    if overwrite {
        let mut changed_text = String::new();
        for line in text.lines() {
            if let Some((key, _old_value)) = parse_localisation_line(line) {
                if let Some(new_value) = remaining.remove(&key) {
                    let indent = line
                        .chars()
                        .take_while(|ch| ch.is_whitespace())
                        .collect::<String>();
                    changed_text.push_str(&format!(
                        "{indent}{key}:0 \"{}\"\n",
                        localisation_value(&new_value)
                    ));
                    updated_keys.push(key);
                    continue;
                }
            }
            changed_text.push_str(line);
            changed_text.push('\n');
        }
        text = changed_text;
    } else {
        let mut existing = BTreeSet::new();
        collect_localisation_keys(&text, &mut existing);
        remaining.retain(|key, _| !existing.contains(key));
    }

    let mut appended_keys = Vec::new();
    for (key, value) in &remaining {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("  {key}:0 \"{}\"\n", localisation_value(value)));
        appended_keys.push(key.clone());
    }
    let changed = !updated_keys.is_empty() || !appended_keys.is_empty() || !path.exists();
    if changed {
        fs::write(path, format!("\u{feff}{text}"))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    }

    Ok(LocalisationUpsertResult {
        changed,
        appended_keys,
        updated_keys,
    })
}

pub(crate) fn parse_translation_format(raw: &str) -> Result<LocalisationTranslationFormat, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "prompt" | "md" | "markdown" => Ok(LocalisationTranslationFormat::Prompt),
        "yml" | "yaml" => Ok(LocalisationTranslationFormat::Yml),
        "json" | "report" => Ok(LocalisationTranslationFormat::Json),
        other => Err(format!(
            "unknown --format `{other}`; use prompt, yml, or json"
        )),
    }
}

pub(crate) fn normalise_localisation_language(raw: &str) -> Result<String, String> {
    let mut value = raw.trim().trim_start_matches("l_").replace('-', "_");
    value.make_ascii_lowercase();
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(format!(
            "invalid localisation language `{raw}`; use a HOI4 language folder name like english, french, german, russian, japanese, or simp_chinese"
        ));
    }
    Ok(value)
}

pub(crate) fn source_localisation_roots(
    map: &ArgMap,
    mod_root: Option<&Path>,
    from: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut roots = repeated_values(map, "input")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    if roots.is_empty() {
        let Some(root) = mod_root else {
            return Err("missing --mod-root or --input".to_string());
        };
        roots.push(root.join("localisation").join(from));
    }
    Ok(roots)
}

pub(crate) fn target_existing_keys(
    mod_root: Option<&Path>,
    to: &str,
) -> Result<BTreeSet<String>, String> {
    let Some(root) = mod_root else {
        return Ok(BTreeSet::new());
    };
    let target = root.join("localisation").join(to);
    if !target.exists() {
        return Ok(BTreeSet::new());
    }
    let mut keys = BTreeSet::new();
    for file in collect_localisation_files(&target)? {
        let text = read_utf8_lossy(&file)?;
        collect_localisation_keys(&text, &mut keys);
    }
    Ok(keys)
}

pub(crate) fn collect_localisation_source_files(
    mod_root: Option<&Path>,
    source_roots: &[PathBuf],
    key_prefixes: &[String],
    target_existing: &BTreeSet<String>,
    max_items: usize,
) -> Result<Vec<LocalisationSourceFile>, String> {
    let mut out = Vec::new();
    let mut remaining = max_items;
    for root in source_roots {
        if remaining == 0 {
            break;
        }
        let files = collect_localisation_files(root)?;
        for path in files {
            if remaining == 0 {
                break;
            }
            let text = read_utf8_lossy(&path)?;
            let relative = mod_root
                .map(|root| relative_slash_path(root, &path))
                .unwrap_or_else(|| slash_path(&path));
            let mut entries = Vec::new();
            for line in text.lines() {
                if remaining == 0 {
                    break;
                }
                let Some((key, value)) = parse_localisation_line(line) else {
                    continue;
                };
                if !key_prefixes.is_empty()
                    && !key_prefixes.iter().any(|prefix| key.starts_with(prefix))
                {
                    continue;
                }
                if target_existing.contains(&key) {
                    continue;
                }
                entries.push(LocalisationTranslationEntry {
                    key,
                    value,
                    source_file: relative.clone(),
                });
                remaining = remaining.saturating_sub(1);
            }
            if !entries.is_empty() {
                out.push(LocalisationSourceFile {
                    path,
                    relative,
                    entries,
                });
            }
        }
    }
    Ok(out)
}

pub(crate) fn collect_localisation_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if root.is_file() {
        return Ok(if is_localisation_yml(root) {
            vec![root.to_path_buf()]
        } else {
            Vec::new()
        });
    }
    let mut files = collect_files(root)?
        .into_iter()
        .filter(|path| is_localisation_yml(path))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub(crate) fn is_localisation_yml(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "yml" | "yaml"))
        .unwrap_or(false)
}

pub(crate) fn all_translation_entries(
    source_files: &[LocalisationSourceFile],
) -> Vec<LocalisationTranslationEntry> {
    source_files
        .iter()
        .flat_map(|file| file.entries.iter().cloned())
        .collect()
}

pub(crate) fn render_localisation_translation_prompt(
    from: &str,
    to: &str,
    source_files: &[LocalisationSourceFile],
) -> String {
    let entries = all_translation_entries(source_files);
    let mut out = String::new();
    out.push_str("# HOI4 Localisation Translation Prompt\n\n");
    out.push_str(&format!("- Source language: `{from}`\n"));
    out.push_str(&format!("- Target language: `{to}`\n"));
    out.push_str(&format!("- Entries: {}\n\n", entries.len()));
    out.push_str("## Rules\n\n");
    out.push_str("- Preserve localisation keys exactly; translate only quoted values.\n");
    out.push_str(&format!(
        "- Output a valid HOI4 localisation block using exactly `l_{to}:`; do not hard-code `l_simp_chinese:` unless the target language is `simp_chinese`.\n"
    ));
    out.push_str("- Preserve HOI4 placeholders, scripted localisation, variables, colour codes, icon codes, and formatting tokens exactly.\n");
    out.push_str("- Do not translate tokens such as `$VAR$`, `$STATE|Y$`, `[ROOT.GetName]`, `[From.GetAdjective]`, `§Y...§!`, `£pol_power`, `%`, `\\n`, or `^` control fragments.\n");
    out.push_str("- Keep escaped quotes valid for `.yml` output.\n");
    out.push_str("- Do not add `_mod_name` localisation keys; mod names belong in `descriptor.mod` and the launcher `.mod` file.\n");
    if to == "simp_chinese" {
        out.push_str("- Translate into natural Simplified Chinese in HOI4 mod style; avoid machine-translation stiffness.\n");
    } else {
        out.push_str("- Translate naturally into the requested target language, keeping HOI4 terms readable for players.\n");
    }
    out.push_str("\n## Output Shape\n\n");
    out.push_str("```yaml\n");
    out.push_str(&format!("l_{to}:\n"));
    out.push_str("  KEY:0 \"translated value\"\n");
    out.push_str("```\n\n");
    out.push_str("## Source Entries\n\n");
    for file in source_files {
        out.push_str(&format!("### `{}`\n\n", file.relative));
        out.push_str("```yaml\n");
        for entry in &file.entries {
            out.push_str(&format!(
                "{}:0 \"{}\"\n",
                entry.key,
                localisation_value(&entry.value)
            ));
        }
        out.push_str("```\n\n");
    }
    out
}

pub(crate) fn render_localisation_translation_yml(
    to: &str,
    entries: &[LocalisationTranslationEntry],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("\u{feff}l_{to}:\n"));
    for entry in entries {
        out.push_str(&format!(
            "  # source: {} | translate value before release\n",
            entry.source_file
        ));
        out.push_str(&format!(
            "  {}:0 \"{}\"\n",
            entry.key,
            localisation_value(&entry.value)
        ));
    }
    out
}

pub(crate) fn write_translation_yml_files(
    source_files: &[LocalisationSourceFile],
    from: &str,
    to: &str,
    output_dir: &Path,
    overwrite: bool,
) -> Result<String, String> {
    fs::create_dir_all(output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let mut written = Vec::new();
    let mut skipped = Vec::new();
    for source in source_files {
        let file_name = source
            .path
            .file_name()
            .and_then(OsStr::to_str)
            .map(|name| target_localisation_output_name(name, from, to))
            .unwrap_or_else(|| format!("translated_l_{to}.yml"));
        let target = output_dir.join(file_name);
        if target.exists() && !overwrite {
            skipped.push(slash_path(&target));
            continue;
        }
        fs::write(
            &target,
            render_localisation_translation_yml(to, &source.entries),
        )
        .map_err(|e| format!("write {}: {e}", target.display()))?;
        written.push(slash_path(&target));
    }

    Ok(format!(
        "{{\"schema\": \"hoi4skill.localisation_translate.write.v1\", \"written_files\": {}, \"skipped_existing_files\": {}}}\n",
        json_array(&written),
        json_array(&skipped)
    ))
}

pub(crate) fn target_localisation_output_name(name: &str, from: &str, to: &str) -> String {
    let from_suffix = format!("_l_{from}");
    let to_suffix = format!("_l_{to}");
    if let Some((stem, ext)) = name.rsplit_once('.') {
        if let Some(prefix) = stem.strip_suffix(&from_suffix) {
            return format!("{prefix}{to_suffix}.{ext}");
        }
    }
    name.replace(&from_suffix, &to_suffix)
}

pub(crate) fn localisation_translation_json(
    from: &str,
    to: &str,
    source_files: &[LocalisationSourceFile],
    entries_total: usize,
) -> String {
    let files = source_files
        .iter()
        .map(|file| {
            format!(
                "{{\"path\": {}, \"entries\": {}}}",
                json_str(&file.relative),
                file.entries.len()
            )
        })
        .collect::<Vec<_>>();
    let entries = all_translation_entries(source_files)
        .into_iter()
        .map(|entry| {
            format!(
                "{{\"key\": {}, \"source_file\": {}, \"source_value\": {}}}",
                json_str(&entry.key),
                json_str(&entry.source_file),
                json_str(&entry.value)
            )
        })
        .collect::<Vec<_>>();
    format!(
        "{{\"schema\": \"hoi4skill.localisation_translate.v1\", \"from\": {}, \"to\": {}, \"source_files\": [{}], \"entries_total\": {}, \"entries\": [{}]}}\n",
        json_str(from),
        json_str(to),
        files.join(", "),
        entries_total,
        entries.join(", ")
    )
}
