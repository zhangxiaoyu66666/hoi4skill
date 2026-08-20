//! Persistent, project-wide terminology constraints for localisation translation.
//!
//! A glossary is stored once per Mod and shared by every translation batch. The
//! prompt renderer uses only terms relevant to the current batch, while the
//! apply path validates every effective target value before writing any file.

#[allow(unused_imports)]
use crate::*;

pub(crate) const LOCALISATION_GLOSSARY_SCHEMA: &str = "hoi4skill.localisation_glossary.v1";
pub(crate) const DEFAULT_LOCALISATION_GLOSSARY: &str = ".hoi4skill/localisation_glossary.json";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct LocalisationGlossaryEntry {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) source: String,
    pub(crate) target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) note: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct LocalisationGlossary {
    #[serde(default = "localisation_glossary_schema")]
    pub(crate) schema: String,
    #[serde(default)]
    pub(crate) entries: Vec<LocalisationGlossaryEntry>,
}

impl Default for LocalisationGlossary {
    fn default() -> Self {
        Self {
            schema: localisation_glossary_schema(),
            entries: Vec::new(),
        }
    }
}

fn localisation_glossary_schema() -> String {
    LOCALISATION_GLOSSARY_SCHEMA.to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LocalisationGlossaryViolation {
    pub(crate) key: String,
    pub(crate) source_file: String,
    pub(crate) source_term: String,
    pub(crate) required_target: String,
    pub(crate) actual_target: String,
}

pub(crate) fn localisation_glossary_path(
    map: &ArgMap,
    mod_root: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    if let Some(raw) = value(map, "glossary") {
        return normalize_path(raw).map(Some);
    }
    Ok(mod_root.map(|root| root.join(DEFAULT_LOCALISATION_GLOSSARY)))
}

pub(crate) fn load_localisation_glossary(
    path: Option<&Path>,
) -> Result<LocalisationGlossary, String> {
    let Some(path) = path else {
        return Ok(LocalisationGlossary::default());
    };
    if !path.exists() {
        return Ok(LocalisationGlossary::default());
    }
    let text = read_utf8_lossy(path)?;
    let mut glossary: LocalisationGlossary = serde_json::from_str(&text)
        .map_err(|error| format!("parse localisation glossary {}: {error}", path.display()))?;
    if glossary.schema != LOCALISATION_GLOSSARY_SCHEMA {
        return Err(format!(
            "unsupported localisation glossary schema `{}` in {}; expected `{LOCALISATION_GLOSSARY_SCHEMA}`",
            glossary.schema,
            path.display()
        ));
    }
    canonicalise_localisation_glossary(&mut glossary)?;
    Ok(glossary)
}

pub(crate) fn write_localisation_glossary(
    path: &Path,
    glossary: &LocalisationGlossary,
) -> Result<(), String> {
    let mut glossary = glossary.clone();
    canonicalise_localisation_glossary(&mut glossary)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let mut text = serde_json::to_string_pretty(&glossary)
        .map_err(|error| format!("serialize localisation glossary: {error}"))?;
    text.push('\n');
    fs::write(path, text).map_err(|error| format!("write {}: {error}", path.display()))
}

fn canonicalise_localisation_glossary(glossary: &mut LocalisationGlossary) -> Result<(), String> {
    let mut canonical = BTreeMap::<(String, String, String), LocalisationGlossaryEntry>::new();
    for mut entry in std::mem::take(&mut glossary.entries) {
        entry.from = normalise_localisation_language(&entry.from)?;
        entry.to = normalise_localisation_language(&entry.to)?;
        entry.source = entry.source.trim().to_string();
        entry.target = entry.target.trim().to_string();
        entry.note = entry.note.trim().to_string();
        if entry.source.is_empty() || entry.target.is_empty() {
            return Err(
                "localisation glossary source and target terms cannot be empty".to_string(),
            );
        }
        let key = (entry.from.clone(), entry.to.clone(), entry.source.clone());
        if let Some(previous) = canonical.get(&key) {
            if previous.target != entry.target {
                return Err(format!(
                    "conflicting glossary translations for `{}` ({} -> {}): `{}` and `{}`",
                    entry.source, entry.from, entry.to, previous.target, entry.target
                ));
            }
        } else {
            canonical.insert(key, entry);
        }
    }
    glossary.schema = localisation_glossary_schema();
    glossary.entries = canonical.into_values().collect();
    Ok(())
}

pub(crate) fn localisation_glossary_entries_for_pair(
    glossary: &LocalisationGlossary,
    from: &str,
    to: &str,
) -> Vec<LocalisationGlossaryEntry> {
    glossary
        .entries
        .iter()
        .filter(|entry| entry.from == from && entry.to == to)
        .cloned()
        .collect()
}

pub(crate) fn applicable_localisation_glossary_entries(
    glossary: &LocalisationGlossary,
    from: &str,
    to: &str,
    source_entries: &[LocalisationTranslationEntry],
) -> Vec<LocalisationGlossaryEntry> {
    let pair_terms = localisation_glossary_entries_for_pair(glossary, from, to);
    pair_terms
        .iter()
        .filter(|term| {
            source_entries
                .iter()
                .any(|entry| localisation_glossary_term_applies(&entry.value, term, &pair_terms))
        })
        .cloned()
        .collect()
}

pub(crate) fn check_localisation_glossary_value(
    entry: &LocalisationTranslationEntry,
    translated_value: &str,
    terms: &[LocalisationGlossaryEntry],
) -> Vec<LocalisationGlossaryViolation> {
    terms
        .iter()
        .filter(|term| {
            localisation_glossary_term_applies(&entry.value, term, terms)
                && !translated_value.contains(&term.target)
        })
        .map(|term| LocalisationGlossaryViolation {
            key: entry.key.clone(),
            source_file: entry.source_file.clone(),
            source_term: term.source.clone(),
            required_target: term.target.clone(),
            actual_target: translated_value.to_string(),
        })
        .collect()
}

fn localisation_glossary_term_applies(
    source_value: &str,
    term: &LocalisationGlossaryEntry,
    terms: &[LocalisationGlossaryEntry],
) -> bool {
    source_value.match_indices(&term.source).any(|(start, _)| {
        let end = start + term.source.len();
        !terms.iter().any(|more_specific| {
            more_specific.source.len() > term.source.len()
                && source_value
                    .match_indices(&more_specific.source)
                    .any(|(specific_start, _)| {
                        let specific_end = specific_start + more_specific.source.len();
                        specific_start <= start && specific_end >= end
                    })
        })
    })
}

pub(crate) fn localisation_glossary_violation_json(
    violation: &LocalisationGlossaryViolation,
) -> String {
    format!(
        "{{\"key\": {}, \"source_file\": {}, \"source_term\": {}, \"required_target\": {}, \"actual_target\": {}}}",
        json_str(&violation.key),
        json_str(&violation.source_file),
        json_str(&violation.source_term),
        json_str(&violation.required_target),
        json_str(&violation.actual_target)
    )
}

pub(crate) fn localisation_glossary_entry_json(entry: &LocalisationGlossaryEntry) -> String {
    format!(
        "{{\"from\": {}, \"to\": {}, \"source\": {}, \"target\": {}, \"note\": {}}}",
        json_str(&entry.from),
        json_str(&entry.to),
        json_str(&entry.source),
        json_str(&entry.target),
        json_str(&entry.note)
    )
}

pub(crate) fn cmd_localisation_glossary(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = value(&map, "mod-root")
        .or_else(|| map.positionals.first().map(String::as_str))
        .map(normalize_path)
        .transpose()?;
    let path = localisation_glossary_path(&map, mod_root.as_deref())?.ok_or_else(|| {
        "localisation-glossary requires --mod-root <mod> or --glossary <file>".to_string()
    })?;
    let from = normalise_localisation_language(value(&map, "from").unwrap_or("simp_chinese"))?;
    let to = normalise_localisation_language(value(&map, "to").unwrap_or("english"))?;
    let mut glossary = load_localisation_glossary(Some(&path))?;
    let mut changed = false;

    let mut set_specs = repeated_values(&map, "set")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    match (value(&map, "source"), value(&map, "target")) {
        (Some(source), Some(target)) => set_specs.push(format!("{source}={target}")),
        (None, None) => {}
        _ => return Err("use --source and --target together".to_string()),
    }
    let note = value(&map, "note").unwrap_or("").trim().to_string();
    let mut updates = BTreeMap::<String, String>::new();
    for spec in set_specs {
        let (source, target) = spec.split_once('=').ok_or_else(|| {
            format!("invalid --set `{spec}`; expected <source>=<required-target>")
        })?;
        let source = source.trim().to_string();
        let target = target.trim().to_string();
        if source.is_empty() || target.is_empty() {
            return Err(format!(
                "invalid --set `{spec}`; source and target terms cannot be empty"
            ));
        }
        if let Some(previous) = updates.insert(source.clone(), target.clone()) {
            if previous != target {
                return Err(format!(
                    "conflicting --set values for `{source}`: `{previous}` and `{target}`"
                ));
            }
        }
    }
    for (source, target) in updates {
        glossary
            .entries
            .retain(|entry| !(entry.from == from && entry.to == to && entry.source == source));
        glossary.entries.push(LocalisationGlossaryEntry {
            from: from.clone(),
            to: to.clone(),
            source,
            target,
            note: note.clone(),
        });
        changed = true;
    }
    for source in repeated_values(&map, "remove") {
        let before = glossary.entries.len();
        glossary.entries.retain(|entry| {
            !(entry.from == from && entry.to == to && entry.source == source.trim())
        });
        changed |= glossary.entries.len() != before;
    }
    canonicalise_localisation_glossary(&mut glossary)?;
    if changed {
        write_localisation_glossary(&path, &glossary)?;
    }

    let pair_entries = localisation_glossary_entries_for_pair(&glossary, &from, &to);
    let mut violations = Vec::new();
    let mut checked_values = 0usize;
    if map.flags.contains("check") {
        let mod_root = mod_root.as_deref().ok_or_else(|| {
            "localisation-glossary --check requires --mod-root to read source and target localisation"
                .to_string()
        })?;
        let source_roots = vec![mod_root.join("localisation").join(&from)];
        let source_files = collect_localisation_source_files(
            Some(mod_root),
            &source_roots,
            &[],
            &BTreeSet::new(),
            usize::MAX,
        )?;
        let target_values = collect_localisation_values(&mod_root.join("localisation").join(&to))?;
        for entry in all_translation_entries(&source_files) {
            if let Some(target_value) = target_values.get(&entry.key) {
                checked_values += 1;
                violations.extend(check_localisation_glossary_value(
                    &entry,
                    target_value,
                    &pair_entries,
                ));
            }
        }
    }

    let report = format!(
        concat!(
            "{{\"schema\": \"hoi4skill.localisation_glossary.report.v1\", ",
            "\"path\": {}, \"from\": {}, \"to\": {}, \"changed\": {}, ",
            "\"entries_total\": {}, \"pair_entries_total\": {}, \"checked_values\": {}, ",
            "\"violation_count\": {}, \"entries\": [{}], \"violations\": [{}]}}\n"
        ),
        json_str(&slash_path(&path)),
        json_str(&from),
        json_str(&to),
        changed,
        glossary.entries.len(),
        pair_entries.len(),
        checked_values,
        violations.len(),
        pair_entries
            .iter()
            .map(localisation_glossary_entry_json)
            .collect::<Vec<_>>()
            .join(", "),
        violations
            .iter()
            .map(localisation_glossary_violation_json)
            .collect::<Vec<_>>()
            .join(", ")
    );
    write_or_print(
        &report,
        value(&map, "output").or_else(|| value(&map, "report")),
    )?;
    if !violations.is_empty() {
        return Err(format!(
            "localisation glossary check failed: {} inconsistent translation(s); update the translations or explicitly revise the glossary",
            violations.len()
        ));
    }
    Ok(())
}

pub(crate) fn collect_localisation_values(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for file in collect_localisation_files(root)? {
        let text = read_utf8_lossy(&file)?;
        for line in text.lines() {
            if let Some((key, value)) = parse_localisation_line(line) {
                values.insert(key, value);
            }
        }
    }
    Ok(values)
}
