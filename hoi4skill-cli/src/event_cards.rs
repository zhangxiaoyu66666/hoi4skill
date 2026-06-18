//! Event-card parsing, namespace numbering, file writes, and localisation insertion.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_parse_event_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
        .transpose()?;
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during event-card parsing".to_string());
    }
    let text = read_utf8_lossy(&input)?;
    let cards = parse_cards(&text, &["事件"]);
    enforce_strict_event_card_gate(&map, &cards, game_index.as_ref())?;
    let json = parse_event_cards_json(&text, tag, prefix);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_apply_event_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
        .transpose()?;
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during event-card generation".to_string());
    }
    let text = read_utf8_lossy(&input)?;
    let cards = parse_cards(&text, &["事件"]);
    enforce_strict_event_card_gate(&map, &cards, game_index.as_ref())?;
    let changed =
        apply_event_cards_to_mod_with_index(&mod_root, &cards, tag, prefix, game_index.as_ref())?;

    println!("Applied event cards: {}", cards.len());
    if changed.is_empty() {
        println!("No file changes were needed.");
    } else {
        println!("Changed:");
        for path in changed {
            println!("  {}", path.display());
        }
    }
    run_post_apply_checks(&mod_root, &map, game_index.as_ref(), Some(&input))?;
    Ok(())
}

pub(crate) fn enforce_strict_event_card_gate(
    map: &ArgMap,
    cards: &[Card],
    game_index: Option<&GameIndex>,
) -> Result<(), String> {
    let options = validation_options_from_args(map);
    enforce_strict_event_card_gate_with_options(options, cards, game_index)
}

pub(crate) fn enforce_strict_event_card_gate_with_options(
    options: ValidationOptions,
    cards: &[Card],
    game_index: Option<&GameIndex>,
) -> Result<(), String> {
    if !options.strict_code_index {
        return Ok(());
    }
    if game_index.is_none() {
        return Err(
            "strict event-card generation requires --game-root before writing files".to_string(),
        );
    }
    let mut errors = Vec::new();
    for card in cards {
        if let Some(trigger) = card.fields.get("触发").or_else(|| card.fields.get("条件")) {
            let suggestions = suggest_trigger(trigger);
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` trigger", card.title),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` trigger", card.title),
                    &suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` trigger", card.title),
                    &suggestions,
                    index,
                ));
            }
        }
        for option in event_options(card) {
            let suggestions = suggest_common("event", &option.effects, None, None, None, None);
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` option {}", card.title, option.key),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` option {}", card.title, option.key),
                    &suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` option {}", card.title, option.key),
                    &suggestions,
                    index,
                ));
            }
            let hidden_suggestions =
                suggest_common("event", &option.hidden_effects, None, None, None, None);
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` hidden option {}", card.title, option.key),
                &hidden_suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` hidden option {}", card.title, option.key),
                    &hidden_suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` hidden option {}", card.title, option.key),
                    &hidden_suggestions,
                    index,
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "strict event-card generation blocked unresolved AI mappings:\n{}",
            errors.join("\n")
        ))
    }
}

#[cfg(test)]
pub(crate) fn apply_event_cards_to_mod(
    mod_root: &Path,
    cards: &[Card],
    tag: &str,
    prefix: &str,
) -> Result<Vec<PathBuf>, String> {
    apply_event_cards_to_mod_with_index(mod_root, cards, tag, prefix, None)
}

pub(crate) fn apply_event_cards_to_mod_with_index(
    mod_root: &Path,
    cards: &[Card],
    tag: &str,
    prefix: &str,
    game_index: Option<&GameIndex>,
) -> Result<Vec<PathBuf>, String> {
    let namespace_targets = scan_event_namespace_targets(mod_root)?;
    let existing_fingerprints = scan_existing_event_card_fingerprints(mod_root)?;
    let picture_catalog = collect_event_picture_catalog(mod_root, game_index)?;
    let mut counters = namespace_targets
        .iter()
        .map(|(namespace, target)| (namespace.clone(), target.max_id))
        .collect::<BTreeMap<_, _>>();
    let mut event_files: BTreeMap<PathBuf, EventFileAppend> = BTreeMap::new();
    let mut loc_entries = BTreeMap::new();

    for card in cards {
        let namespace = event_card_namespace(card, prefix);
        let fingerprint = event_card_fingerprint(card, &namespace);
        if existing_fingerprints.contains(&fingerprint) {
            continue;
        }
        let counter = counters.entry(namespace.clone()).or_insert(0);
        *counter += 1;
        let event_id_max = active_event_id_max();
        if *counter > event_id_max {
            return Err(format!(
                "namespace {namespace} has reached event id limit {event_id_max}"
            ));
        }
        let event_id = format!("{}.{}", namespace, counter);
        let event_path = namespace_targets
            .get(&namespace)
            .map(|target| target.path.clone())
            .unwrap_or_else(|| mod_root.join("events").join(format!("{prefix}_events.txt")));
        let entry = event_files.entry(event_path).or_default();
        entry.namespaces.insert(namespace.clone());
        let picture = resolve_event_picture(card, &picture_catalog);
        entry.blocks.push((
            event_id.to_string(),
            render_event_block_with_picture(
                card,
                &event_id,
                tag,
                prefix,
                Some(&fingerprint),
                &picture,
            ),
        ));
        insert_event_localisation(card, &event_id, &mut loc_entries);
    }

    let mut changed = Vec::new();
    for (path, append) in event_files {
        if !append.blocks.is_empty()
            && append_event_blocks(&path, &append.namespaces, &append.blocks)?
        {
            changed.push(path);
        }
    }
    if !loc_entries.is_empty() {
        let path = target_localisation_path(mod_root, tag);
        if append_localisation_entries(&path, &loc_entries)? {
            changed.push(path);
        }
    }

    Ok(changed)
}

pub(crate) fn collect_event_picture_catalog(
    mod_root: &Path,
    game_index: Option<&GameIndex>,
) -> Result<BTreeSet<String>, String> {
    let mut pictures = BTreeSet::new();
    let interface_root = mod_root.join("interface");
    if interface_root.exists() {
        for file in collect_files(&interface_root)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") != "gfx" {
                continue;
            }
            collect_event_pictures(&read_utf8_lossy(&file)?, &mut pictures);
        }
    }
    if let Some(index) = game_index {
        pictures.extend(index.event_pictures.iter().cloned());
    }
    Ok(pictures)
}

pub(crate) fn resolve_event_picture(card: &Card, catalog: &BTreeSet<String>) -> String {
    if let Some(explicit) = card.fields.get("图片").map(|value| value.trim_matches('"')) {
        if is_reference_identifier(explicit) {
            return explicit.to_string();
        }
        let semantic_title = format!("{} {explicit}", card.title);
        return choose_semantic_reference_from_catalog(&semantic_title, catalog)
            .unwrap_or_else(|| "GFX_report_event_generic".to_string());
    }
    choose_semantic_reference_from_catalog(&card.title, catalog)
        .unwrap_or_else(|| "GFX_report_event_generic".to_string())
}

pub(crate) fn event_card_namespace(card: &Card, prefix: &str) -> String {
    card.fields
        .get("命名空间")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(prefix)
        .to_string()
}

#[derive(Clone)]
pub(crate) struct EventNamespaceTarget {
    pub(crate) path: PathBuf,
    pub(crate) max_id: i64,
}

#[derive(Default)]
pub(crate) struct EventFileAppend {
    pub(crate) namespaces: BTreeSet<String>,
    pub(crate) blocks: Vec<(String, String)>,
}

pub(crate) fn scan_event_namespace_targets(
    mod_root: &Path,
) -> Result<BTreeMap<String, EventNamespaceTarget>, String> {
    let mut targets = BTreeMap::new();
    let events_root = mod_root.join("events");
    if !events_root.exists() {
        return Ok(targets);
    }
    for file in collect_files(&events_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for line in text.lines() {
            if let Some(namespace) = assignment_value(line.trim(), "add_namespace") {
                targets
                    .entry(namespace.to_string())
                    .or_insert_with(|| EventNamespaceTarget {
                        path: file.clone(),
                        max_id: 0,
                    });
            }
        }
        for kind in ["country_event", "news_event", "state_event"] {
            for block in blocks_named(&text, kind) {
                let Some(id) = block_assignment(&block, "id") else {
                    continue;
                };
                let Some((namespace, number)) = event_id_namespace_number(&id) else {
                    continue;
                };
                targets
                    .entry(namespace)
                    .and_modify(|target| {
                        if number > target.max_id {
                            target.max_id = number;
                            target.path = file.clone();
                        }
                    })
                    .or_insert_with(|| EventNamespaceTarget {
                        path: file.clone(),
                        max_id: number,
                    });
            }
        }
    }
    Ok(targets)
}

pub(crate) fn scan_existing_event_card_fingerprints(
    mod_root: &Path,
) -> Result<BTreeSet<String>, String> {
    let mut fingerprints = BTreeSet::new();
    let events_root = mod_root.join("events");
    if !events_root.exists() {
        return Ok(fingerprints);
    }
    for file in collect_files(&events_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for line in text.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix("# hoi4skill_card = ") else {
                continue;
            };
            let value = rest.trim();
            if !value.is_empty() {
                fingerprints.insert(value.to_string());
            }
        }
    }
    Ok(fingerprints)
}

pub(crate) fn event_card_fingerprint(card: &Card, namespace: &str) -> String {
    let mut text = String::new();
    text.push_str(namespace);
    text.push('\n');
    text.push_str(&card.kind);
    text.push('\n');
    text.push_str(&card.title);
    text.push('\n');
    for (key, value) in &card.fields {
        text.push_str(key);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }
    format!("ev_{:016x}", stable_hash64(&text))
}

pub(crate) fn stable_hash64(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn event_card_numbered_ids(cards: &[Card], prefix: &str) -> Vec<(String, String)> {
    let mut counters: BTreeMap<String, usize> = BTreeMap::new();
    cards
        .iter()
        .map(|card| {
            let namespace = event_card_namespace(card, prefix);
            let counter = counters.entry(namespace.clone()).or_insert(0);
            *counter += 1;
            let event_id = format!("{}.{}", namespace, counter);
            (namespace, event_id)
        })
        .collect()
}

pub(crate) fn render_event_block_with_picture(
    card: &Card,
    event_id: &str,
    tag: &str,
    prefix: &str,
    fingerprint: Option<&str>,
    picture: &str,
) -> String {
    let event_type = normalize_event_type(card.fields.get("类型").map(String::as_str));
    let trigger = card.fields.get("触发").or_else(|| card.fields.get("条件"));
    let trigger_lines = trigger
        .map(|text| {
            split_cn_list(text)
                .into_iter()
                .flat_map(suggest_trigger)
                .filter_map(|suggestion| {
                    if suggestion.kind == "trigger" {
                        concrete_suggestion_code(&suggestion)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let options = event_options(card);

    let mut out = String::new();
    if let Some(fingerprint) = fingerprint {
        out.push_str(&format!("# hoi4skill_card = {fingerprint}\n"));
    }
    out.push_str(&format!("{event_type} = {{\n"));
    out.push_str(&format!("\tid = {event_id}\n"));
    out.push_str(&format!("\ttitle = {event_id}.t\n"));
    out.push_str(&format!("\tdesc = {event_id}.d\n"));
    out.push_str(&format!("\tpicture = {picture}\n"));
    out.push_str("\tis_triggered_only = yes\n");
    if !trigger_lines.is_empty() {
        out.push_str("\ttrigger = {\n");
        for line in trigger_lines {
            out.push_str(&format!("\t\t{line}\n"));
        }
        out.push_str("\t}\n");
    } else if trigger.is_some() {
        out.push_str("\t# TODO: map event trigger candidates before enabling a trigger block\n");
    }
    for option in options {
        out.push_str(&render_event_option(&option, event_id));
    }
    if !card.fields.contains_key("命名空间") && prefix != tag {
        out.push_str(&format!("\t# namespace inferred from prefix: {prefix}\n"));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_event_option(option: &EventOption, event_id: &str) -> String {
    let mut out = String::new();
    out.push_str("\n\toption = {\n");
    out.push_str(&format!("\t\tname = {event_id}.{}\n", option.key));
    if !option.ai_chance.trim().is_empty() {
        if let Some(factor) = parse_int(&option.ai_chance) {
            out.push_str("\t\tai_chance = {\n");
            out.push_str(&format!("\t\t\tfactor = {factor}\n"));
            out.push_str("\t\t}\n");
        }
    }
    let suggestions = suggest_common("event", &option.effects, None, None, None, None);
    let (effect_lines, effect_comments) = decision_effect_lines(&suggestions);
    if option.effects.trim().is_empty() && option.hidden_effects.trim().is_empty() {
        out.push_str("\t\t# TODO: add option effects\n");
    }
    for comment in effect_comments {
        out.push_str(&format!("\t\t# {comment}\n"));
    }
    for line in effect_lines {
        out.push_str(&indent_lines(&line, "\t\t"));
    }
    if !option.hidden_effects.trim().is_empty() {
        out.push_str("\t\thidden_effect = {\n");
        let hidden_suggestions =
            suggest_common("event", &option.hidden_effects, None, None, None, None);
        let (hidden_lines, hidden_comments) = decision_effect_lines(&hidden_suggestions);
        if hidden_lines.is_empty() && hidden_comments.is_empty() {
            out.push_str(&format!("\t\t\t# {}\n", option.hidden_effects.trim()));
        }
        for comment in hidden_comments {
            out.push_str(&format!("\t\t\t# {comment}\n"));
        }
        for line in hidden_lines {
            out.push_str(&indent_lines(&line, "\t\t\t"));
        }
        out.push_str("\t\t}\n");
    }
    out.push_str("\t}\n");
    out
}

#[derive(Clone)]
pub(crate) struct EventOption {
    pub(crate) key: String,
    pub(crate) text: String,
    pub(crate) effects: String,
    pub(crate) hidden_effects: String,
    pub(crate) ai_chance: String,
}

pub(crate) fn event_options(card: &Card) -> Vec<EventOption> {
    let mut options = BTreeMap::new();
    for (key, value) in &card.fields {
        if let Some(suffix) = key.strip_prefix("选项") {
            let option_key = option_key(suffix);
            let effects = card
                .fields
                .get(&format!("效果{suffix}"))
                .cloned()
                .unwrap_or_default();
            let hidden_effects = card
                .fields
                .get(&format!("隐藏效果{suffix}"))
                .cloned()
                .unwrap_or_default();
            let ai_chance = card
                .fields
                .get(&format!("AI权重{suffix}"))
                .or_else(|| card.fields.get(&format!("ai权重{suffix}")))
                .or_else(|| card.fields.get(&format!("ai{suffix}")))
                .cloned()
                .unwrap_or_default();
            options.insert(
                option_key.clone(),
                EventOption {
                    key: option_key,
                    text: value.clone(),
                    effects,
                    hidden_effects,
                    ai_chance,
                },
            );
        }
    }
    if options.is_empty() {
        options.insert(
            "a".to_string(),
            EventOption {
                key: "a".to_string(),
                text: "好的。".to_string(),
                effects: String::new(),
                hidden_effects: String::new(),
                ai_chance: String::new(),
            },
        );
    }
    options.into_values().collect()
}

pub(crate) fn insert_event_localisation(
    card: &Card,
    event_id: &str,
    loc_entries: &mut BTreeMap<String, String>,
) {
    loc_entries.insert(
        format!("{event_id}.t"),
        card.fields
            .get("标题")
            .cloned()
            .unwrap_or_else(|| card.title.clone()),
    );
    loc_entries.insert(
        format!("{event_id}.d"),
        card.fields
            .get("描述")
            .cloned()
            .unwrap_or_else(|| format!("{}。", card.title)),
    );
    for option in event_options(card) {
        loc_entries.insert(format!("{event_id}.{}", option.key), option.text);
    }
}

pub(crate) fn append_event_blocks(
    path: &Path,
    namespaces: &BTreeSet<String>,
    blocks: &[(String, String)],
) -> Result<bool, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = if path.exists() {
        read_utf8_lossy(path)?
    } else {
        "# Generated events by hoi4skill\n".to_string()
    };
    let mut changed = false;
    for namespace in namespaces {
        let marker = format!("add_namespace = {namespace}");
        if !text.contains(&marker) {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&format!("{marker}\n"));
            changed = true;
        }
    }
    for (key, block) in blocks {
        if text.contains(key) {
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(block);
        changed = true;
    }
    if changed || !path.exists() {
        fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn parse_event_cards_json(text: &str, tag: &str, prefix: &str) -> String {
    let cards = parse_cards(text, &["事件"]);
    let event_ids = event_card_numbered_ids(&cards, prefix);
    let safety_suggestions = event_cards_safety_suggestions(&cards);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!(
        "  \"safety\": {},\n",
        suggestions_safety_json(&safety_suggestions)
    ));
    out.push_str("  \"features\": [\n");
    for (idx, (card, (ns, event_id))) in cards.iter().zip(event_ids.iter()).enumerate() {
        comma(&mut out, idx, "    ");
        let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
        let event_type = normalize_event_type(card.fields.get("类型").map(String::as_str));
        let mut suggestions = Vec::new();
        suggestions.push(Suggestion::new(
            "event_namespace",
            &format!("add_namespace = {ns}"),
            ns,
            "",
        ));
        suggestions.push(Suggestion::new(
            "event_header",
            &format!(
                "{event_type} = {{ id = {event_id} title = {event_id}.t desc = {event_id}.d is_triggered_only = yes }}"
            ),
            &card.title,
            "",
        ));
        suggestions.push(Suggestion::new(
            "event_script",
            &render_event_script_skeleton(ns, event_type, event_id),
            &card.title,
            "Write the namespace first, then the event body; the event id must use that namespace.",
        ));
        if let Some(pic) = card.fields.get("图片") {
            suggestions.push(Suggestion::new(
                "event_picture",
                &format!("picture = {pic}"),
                pic,
                "",
            ));
        }
        if let Some(trigger) = card.fields.get("触发").or_else(|| card.fields.get("条件")) {
            suggestions.extend(suggest_trigger(trigger));
        }

        let mut options = BTreeMap::new();
        for option in event_options(card) {
            let mut option_suggestions =
                suggest_common("event", &option.effects, None, None, None, None);
            option_suggestions.extend(suggest_common(
                "event",
                &option.hidden_effects,
                None,
                None,
                None,
                None,
            ));
            options.insert(
                option.key.clone(),
                format!(
                    "{{\"key\": {}, \"text\": {}, \"effects\": {}, \"hidden_effects\": {}, \"ai_chance\": {}, \"suggestions\": {}, \"safety\": {}}}",
                    json_str(&option.key),
                    json_str(&option.text),
                    json_str(&option.effects),
                    json_str(&option.hidden_effects),
                    json_str(&option.ai_chance),
                    suggestions_json(&option_suggestions),
                    suggestions_safety_json(&option_suggestions)
                ),
            );
        }

        let loc_keys: Vec<String> = std::iter::once(format!("{event_id}.t"))
            .chain(std::iter::once(format!("{event_id}.d")))
            .chain(options.keys().map(|k| format!("{event_id}.{k}")))
            .collect();
        let files = vec![
            format!("events/{prefix}_events.txt"),
            target_localisation_relative_path(target),
        ];
        out.push_str(&format!(
            "{{\"type\": \"event\", \"title\": {}, \"target\": {}, \"namespace\": {}, \"event_type\": {}, \"event_id\": {}, \"fields\": {}, \"options\": {}, \"files\": {}, \"localisation_keys\": {}, \"suggestions\": {}, \"safety\": {}}}",
            json_str(&card.title),
            json_str(target),
            json_str(ns),
            json_str(event_type),
            json_str(event_id),
            json_object(&card.fields),
            json_raw_object(&options),
            json_array(&files),
            json_array(&loc_keys),
            suggestions_json(&suggestions),
            suggestions_safety_json(&suggestions)
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn event_cards_safety_suggestions(cards: &[Card]) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for card in cards {
        if let Some(trigger) = card.fields.get("触发").or_else(|| card.fields.get("条件")) {
            out.extend(suggest_trigger(trigger));
        }
        for option in event_options(card) {
            out.extend(suggest_common(
                "event",
                &option.effects,
                None,
                None,
                None,
                None,
            ));
            out.extend(suggest_common(
                "event",
                &option.hidden_effects,
                None,
                None,
                None,
                None,
            ));
        }
    }
    out
}

pub(crate) fn render_event_script_skeleton(
    namespace: &str,
    event_type: &str,
    event_id: &str,
) -> String {
    format!(
        "add_namespace = {namespace}\n\n{event_type} = {{\n\tid = {event_id}\n\ttitle = {event_id}.t\n\tdesc = {event_id}.d\n\tis_triggered_only = yes\n}}"
    )
}
