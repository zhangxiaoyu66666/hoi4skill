//! Best-effort import of existing HOI4 mod content into a report IR.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_import_mod_ir(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let input = normalize_path(&input)?;
    let max_items = parse_usize_option(&map, "max-items", 1000)?;
    let resolved = resolve_mod_root(&input)?;
    let json = if map.flags.contains("skip-localisation") {
        import_mod_ir_json_with_options(&resolved, max_items, false)?
    } else {
        import_mod_ir_json(&resolved, max_items)?
    };
    write_or_print(&json, value(&map, "output"))
}

#[derive(Clone)]
pub(crate) struct ImportedFocus {
    pub(crate) file: String,
    pub(crate) tree_id: String,
    pub(crate) country_tag: String,
    pub(crate) id: String,
    pub(crate) icon: Option<String>,
    pub(crate) x: Option<i64>,
    pub(crate) y: Option<i64>,
    pub(crate) cost: Option<i64>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) prerequisites: Vec<String>,
    pub(crate) mutually_exclusive: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct ImportedEvent {
    pub(crate) file: String,
    pub(crate) event_type: String,
    pub(crate) id: String,
    pub(crate) namespace: Option<String>,
    pub(crate) number: Option<i64>,
    pub(crate) title_key: Option<String>,
    pub(crate) desc_key: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) picture: Option<String>,
    pub(crate) options: Vec<ImportedEventOption>,
}

#[derive(Clone)]
pub(crate) struct ImportedEventOption {
    pub(crate) name_key: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ImportedIdea {
    pub(crate) file: String,
    pub(crate) category: String,
    pub(crate) id: String,
    pub(crate) picture: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ImportedDecisionCategory {
    pub(crate) file: String,
    pub(crate) id: String,
    pub(crate) icon: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ImportedDecision {
    pub(crate) file: String,
    pub(crate) category: String,
    pub(crate) id: String,
    pub(crate) icon: Option<String>,
    pub(crate) cost: Option<i64>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
}

pub(crate) fn import_mod_ir_json(
    resolved: &ModRootResolution,
    max_items: usize,
) -> Result<String, String> {
    import_mod_ir_json_with_options(resolved, max_items, true)
}

fn import_mod_ir_json_with_options(
    resolved: &ModRootResolution,
    max_items: usize,
    include_localisation: bool,
) -> Result<String, String> {
    let root = &resolved.root;
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let localisation = if include_localisation {
        collect_focus_localisation_map(root)?
    } else {
        BTreeMap::new()
    };
    let mut focuses = import_focuses(root, &localisation)?;
    let mut events = import_events(root, &localisation)?;
    let mut ideas = import_ideas(root, &localisation)?;
    let mut decision_categories = import_decision_categories(root, &localisation)?;
    let mut decisions = import_decisions(root, &localisation)?;
    let localisation_total = localisation.len();
    let localisation_entries = localisation
        .iter()
        .take(max_items)
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();

    let focuses_total = focuses.len();
    let events_total = events.len();
    let ideas_total = ideas.len();
    let decision_categories_total = decision_categories.len();
    let decisions_total = decisions.len();
    focuses.truncate(max_items);
    events.truncate(max_items);
    ideas.truncate(max_items);
    decision_categories.truncate(max_items);
    decisions.truncate(max_items);

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
    out.push_str("  \"schema\": \"hoi4skill.imported_mod_ir.v1\",\n");
    out.push_str(&format!("  \"max_items_per_section\": {},\n", max_items));
    out.push_str(&format!(
        "  \"focuses\": {},\n",
        imported_focuses_json(&focuses)
    ));
    out.push_str(&format!(
        "  \"events\": {},\n",
        imported_events_json(&events)
    ));
    out.push_str(&format!("  \"ideas\": {},\n", imported_ideas_json(&ideas)));
    out.push_str(&format!(
        "  \"decision_categories\": {},\n",
        imported_decision_categories_json(&decision_categories)
    ));
    out.push_str(&format!(
        "  \"decisions\": {},\n",
        imported_decisions_json(&decisions)
    ));
    out.push_str(&format!(
        "  \"localisation\": {{\"included\": {}, \"language\": \"simp_chinese\", \"keys_total\": {}, \"keys_returned\": {}, \"entries\": {}}},\n",
        json_bool(include_localisation),
        localisation_total,
        localisation_entries.len(),
        json_object(&localisation_entries)
    ));
    out.push_str(&format!(
        "  \"counts\": {{\"focuses_total\": {}, \"focuses_returned\": {}, \"events_total\": {}, \"events_returned\": {}, \"ideas_total\": {}, \"ideas_returned\": {}, \"decision_categories_total\": {}, \"decision_categories_returned\": {}, \"decisions_total\": {}, \"decisions_returned\": {}, \"localisation_keys_total\": {}, \"localisation_keys_returned\": {}}}\n",
        focuses_total,
        focuses.len(),
        events_total,
        events.len(),
        ideas_total,
        ideas.len(),
        decision_categories_total,
        decision_categories.len(),
        decisions_total,
        decisions.len(),
        localisation_total,
        localisation_entries.len()
    ));
    out.push_str("}\n");
    Ok(out)
}

pub(crate) fn import_focuses(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<Vec<ImportedFocus>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "common/national_focus")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        let trees = direct_blocks_named(&text, "focus_tree");
        if trees.is_empty() {
            import_focus_blocks(
                &mut out,
                &rel,
                "",
                "",
                &direct_blocks_named(&text, "focus"),
                localisation,
            );
            continue;
        }
        for tree in trees {
            let tree_id = block_assignment(&tree, "id").unwrap_or_default();
            let country_tag = direct_blocks_named(&tree, "country")
                .first()
                .and_then(|block| block_assignment(block, "tag"))
                .unwrap_or_default();
            let focus_blocks = direct_blocks_named(&tree, "focus");
            import_focus_blocks(
                &mut out,
                &rel,
                &tree_id,
                &country_tag,
                &focus_blocks,
                localisation,
            );
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    Ok(out)
}

pub(crate) fn import_focus_blocks(
    out: &mut Vec<ImportedFocus>,
    file: &str,
    tree_id: &str,
    country_tag: &str,
    blocks: &[String],
    localisation: &BTreeMap<String, String>,
) {
    for block in blocks {
        let Some(id) = block_assignment(block, "id") else {
            continue;
        };
        out.push(ImportedFocus {
            file: file.to_string(),
            tree_id: tree_id.to_string(),
            country_tag: country_tag.to_string(),
            icon: block_assignment(block, "icon"),
            x: direct_i64_assignment(block, "x"),
            y: direct_i64_assignment(block, "y"),
            cost: direct_i64_assignment(block, "cost"),
            title: localisation.get(&id).cloned(),
            desc: localisation.get(&format!("{id}_desc")).cloned(),
            prerequisites: wrapped_assignment_values(block, "prerequisite", "focus"),
            mutually_exclusive: wrapped_assignment_values(block, "mutually_exclusive", "focus"),
            id,
        });
    }
}

pub(crate) fn import_events(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<Vec<ImportedEvent>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "events")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for kind in ["country_event", "news_event", "state_event"] {
            for block in direct_blocks_named(&text, kind) {
                let Some(id) = block_assignment(&block, "id") else {
                    continue;
                };
                let (namespace, number) = event_id_namespace_number(&id)
                    .map(|(namespace, number)| (Some(namespace), Some(number)))
                    .unwrap_or((None, None));
                let title_key = block_assignment(&block, "title");
                let desc_key = block_assignment(&block, "desc");
                let options = direct_blocks_named(&block, "option")
                    .into_iter()
                    .map(|option| {
                        let name_key = block_assignment(&option, "name");
                        let name = name_key
                            .as_deref()
                            .and_then(|key| localisation.get(key))
                            .cloned();
                        ImportedEventOption { name_key, name }
                    })
                    .collect::<Vec<_>>();
                out.push(ImportedEvent {
                    file: rel.clone(),
                    event_type: kind.to_string(),
                    namespace,
                    number,
                    title: title_key
                        .as_deref()
                        .and_then(|key| localisation.get(key))
                        .cloned(),
                    desc: desc_key
                        .as_deref()
                        .and_then(|key| localisation.get(key))
                        .cloned(),
                    picture: block_assignment(&block, "picture"),
                    title_key,
                    desc_key,
                    options,
                    id,
                });
            }
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    Ok(out)
}

pub(crate) fn import_ideas(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<Vec<ImportedIdea>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "common/ideas")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for ideas_block in direct_blocks_named(&text, "ideas") {
            for (category, category_block) in direct_child_blocks(&ideas_block) {
                if !is_identifier_like(&category) || is_import_definition_field(&category) {
                    continue;
                }
                for (id, idea_block) in direct_child_blocks(&category_block) {
                    if !is_identifier_like(&id) || is_import_definition_field(&id) {
                        continue;
                    }
                    out.push(ImportedIdea {
                        file: rel.clone(),
                        category: category.clone(),
                        title: localisation.get(&id).cloned(),
                        desc: localisation.get(&format!("{id}_desc")).cloned(),
                        picture: block_assignment(&idea_block, "picture"),
                        id,
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.category.cmp(&b.category))
            .then(a.id.cmp(&b.id))
    });
    Ok(out)
}

pub(crate) fn import_decision_categories(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<Vec<ImportedDecisionCategory>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "common/decisions/categories")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for (id, block) in direct_child_blocks(&text) {
            if !is_identifier_like(&id) || is_import_definition_field(&id) {
                continue;
            }
            out.push(ImportedDecisionCategory {
                file: rel.clone(),
                title: localisation.get(&id).cloned(),
                desc: localisation.get(&format!("{id}_desc")).cloned(),
                icon: block_assignment(&block, "icon"),
                id,
            });
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    Ok(out)
}

pub(crate) fn import_decisions(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<Vec<ImportedDecision>, String> {
    let mut out = Vec::new();
    for file in txt_files(root, "common/decisions")? {
        let rel = rel_slash(root, &file);
        if rel.contains("common/decisions/categories/") {
            continue;
        }
        let text = strip_comments(&read_utf8_lossy(&file)?);
        for (category, category_block) in direct_child_blocks(&text) {
            if !is_identifier_like(&category) || is_import_definition_field(&category) {
                continue;
            }
            for (id, block) in direct_child_blocks(&category_block) {
                if !is_identifier_like(&id) || is_import_definition_field(&id) {
                    continue;
                }
                out.push(ImportedDecision {
                    file: rel.clone(),
                    category: category.clone(),
                    title: localisation.get(&id).cloned(),
                    desc: localisation.get(&format!("{id}_desc")).cloned(),
                    icon: block_assignment(&block, "icon"),
                    cost: direct_i64_assignment(&block, "cost"),
                    id,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.category.cmp(&b.category))
            .then(a.id.cmp(&b.id))
    });
    Ok(out)
}

pub(crate) fn txt_files(root: &Path, rel_dir: &str) -> Result<Vec<PathBuf>, String> {
    let dir = root.join(rel_dir.replace('/', "\\"));
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = collect_files(&dir)?
        .into_iter()
        .filter(|file| file.extension().and_then(OsStr::to_str).unwrap_or("") == "txt")
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub(crate) fn direct_i64_assignment(block: &str, key: &str) -> Option<i64> {
    block_assignment(block, key).and_then(|value| value.parse::<i64>().ok())
}

pub(crate) fn wrapped_assignment_values(block: &str, wrapper: &str, key: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    for wrapped in direct_blocks_named(block, wrapper) {
        for value in assignment_values_in_text(&wrapped, key) {
            values.insert(value);
        }
    }
    values.into_iter().collect()
}

pub(crate) fn is_import_definition_field(key: &str) -> bool {
    is_common_definition_field(key)
        || matches!(
            key,
            "allowed"
                | "available"
                | "visible"
                | "complete_effect"
                | "remove_effect"
                | "cancel_effect"
                | "target_trigger"
                | "targets"
                | "days_remove"
                | "days_re_enable"
                | "fire_only_once"
                | "fixed_random_seed"
                | "ai_will_do"
                | "modifier"
                | "allowed_civil_war"
                | "picture"
                | "rule"
                | "show_ideas"
                | "default"
                | "cancel_if_not_visible"
                | "state_target"
                | "highlight_states"
        )
}

pub(crate) fn imported_focuses_json(values: &[ImportedFocus]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|focus| {
                format!(
                    "{{\"type\": \"focus\", \"file\": {}, \"tree_id\": {}, \"country_tag\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"cost\": {}, \"title\": {}, \"desc\": {}, \"prerequisites\": {}, \"mutually_exclusive\": {}}}",
                    json_str(&focus.file),
                    json_str(&focus.tree_id),
                    json_str(&focus.country_tag),
                    json_str(&focus.id),
                    json_optional_str(focus.icon.as_deref()),
                    json_optional_i64(focus.x),
                    json_optional_i64(focus.y),
                    json_optional_i64(focus.cost),
                    json_optional_str(focus.title.as_deref()),
                    json_optional_str(focus.desc.as_deref()),
                    json_array(&focus.prerequisites),
                    json_array(&focus.mutually_exclusive)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn imported_events_json(values: &[ImportedEvent]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|event| {
                format!(
                    "{{\"type\": \"event\", \"file\": {}, \"event_type\": {}, \"id\": {}, \"namespace\": {}, \"number\": {}, \"title_key\": {}, \"desc_key\": {}, \"title\": {}, \"desc\": {}, \"picture\": {}, \"options\": {}}}",
                    json_str(&event.file),
                    json_str(&event.event_type),
                    json_str(&event.id),
                    json_optional_str(event.namespace.as_deref()),
                    json_optional_i64(event.number),
                    json_optional_str(event.title_key.as_deref()),
                    json_optional_str(event.desc_key.as_deref()),
                    json_optional_str(event.title.as_deref()),
                    json_optional_str(event.desc.as_deref()),
                    json_optional_str(event.picture.as_deref()),
                    imported_event_options_json(&event.options)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn imported_event_options_json(values: &[ImportedEventOption]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|option| {
                format!(
                    "{{\"name_key\": {}, \"name\": {}}}",
                    json_optional_str(option.name_key.as_deref()),
                    json_optional_str(option.name.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn imported_ideas_json(values: &[ImportedIdea]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|idea| {
                format!(
                    "{{\"type\": \"idea\", \"file\": {}, \"category\": {}, \"id\": {}, \"picture\": {}, \"title\": {}, \"desc\": {}}}",
                    json_str(&idea.file),
                    json_str(&idea.category),
                    json_str(&idea.id),
                    json_optional_str(idea.picture.as_deref()),
                    json_optional_str(idea.title.as_deref()),
                    json_optional_str(idea.desc.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn imported_decision_categories_json(values: &[ImportedDecisionCategory]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|category| {
                format!(
                    "{{\"type\": \"decision_category\", \"file\": {}, \"id\": {}, \"icon\": {}, \"title\": {}, \"desc\": {}}}",
                    json_str(&category.file),
                    json_str(&category.id),
                    json_optional_str(category.icon.as_deref()),
                    json_optional_str(category.title.as_deref()),
                    json_optional_str(category.desc.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn imported_decisions_json(values: &[ImportedDecision]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|decision| {
                format!(
                    "{{\"type\": \"decision\", \"file\": {}, \"category\": {}, \"id\": {}, \"icon\": {}, \"cost\": {}, \"title\": {}, \"desc\": {}}}",
                    json_str(&decision.file),
                    json_str(&decision.category),
                    json_str(&decision.id),
                    json_optional_str(decision.icon.as_deref()),
                    json_optional_i64(decision.cost),
                    json_optional_str(decision.title.as_deref()),
                    json_optional_str(decision.desc.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod import_ir_option_tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn skip_localisation_avoids_loading_and_returning_localisation() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("hoi4skill-import-skip-loc-{stamp}"));
        let output = root.join("report.json");
        fs::create_dir_all(root.join("common/national_focus")).unwrap();
        fs::create_dir_all(root.join("localisation/simp_chinese")).unwrap();
        fs::write(
            root.join("common/national_focus/test.txt"),
            "focus_tree = { id = test focus = { id = TEST_focus } }",
        )
        .unwrap();
        fs::write(
            root.join("localisation/simp_chinese/test_l_simp_chinese.yml"),
            "l_simp_chinese:\n TEST_focus:0 \"测试国策\"\n",
        )
        .unwrap();

        cmd_import_mod_ir(&[
            root.to_string_lossy().to_string(),
            "--skip-localisation".to_string(),
            "--output".to_string(),
            output.to_string_lossy().to_string(),
        ])
        .unwrap();
        let report = read_utf8_lossy(&output).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert!(report.contains("\"included\": false"));
        assert!(report.contains("\"localisation_keys_total\": 0"));
        assert!(report.contains("\"title\": null"));
    }
}
