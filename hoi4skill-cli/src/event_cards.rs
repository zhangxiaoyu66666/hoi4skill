//! Event-card parsing, namespace numbering, file writes, and localisation insertion.

#[allow(unused_imports)]
use crate::*;
use std::collections::VecDeque;

pub(crate) fn cmd_parse_event_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_mods = dependency_mod_roots_for_optional_edited_mod(
        &map,
        mod_root.as_deref(),
        game_root.is_some(),
    )?;
    let game_index = game_root
        .as_ref()
        .map(|path| build_game_index_with_mod_paths(path, &dependency_mods))
        .transpose()?;
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during event-card parsing".to_string());
    }
    let text = read_text_document(&input)?;
    let cards = parse_cards(&text, &["事件"]);
    let parse_errors = event_card_command_parse_errors(&text, &cards);
    if validation_options_from_args(&map).strict_code_index && !parse_errors.is_empty() {
        return Err(format!(
            "strict event-card generation blocked malformed event cards:\n{}",
            parse_errors.join("\n")
        ));
    }
    let chain_plan = mod_root
        .as_ref()
        .map(|mod_root| build_event_chain_index_for_mod_with_ids(mod_root, &cards, prefix))
        .transpose()?;
    enforce_strict_event_card_gate_with_chain(
        &map,
        &cards,
        game_index.as_ref(),
        chain_plan.as_ref().map(|(chain_index, _)| chain_index),
        chain_plan
            .as_ref()
            .map(|(_, planned_ids)| planned_ids.as_slice()),
    )?;
    let json = if let Some((chain_index, planned_ids)) = chain_plan.as_ref() {
        parse_event_cards_json_with_chain(&text, tag, prefix, Some(chain_index), Some(planned_ids))
    } else {
        parse_event_cards_json(&text, tag, prefix)
    };
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_apply_event_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_mods =
        dependency_mod_roots_for_edited_mod(&map, &mod_root, game_root.is_some())?;
    let game_index = game_root
        .as_ref()
        .map(|path| build_game_index_with_mod_paths(path, &dependency_mods))
        .transpose()?;
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during event-card generation".to_string());
    }
    let text = read_text_document(&input)?;
    let cards = parse_cards(&text, &["事件"]);
    let parse_errors = event_card_command_parse_errors(&text, &cards);
    if !parse_errors.is_empty() {
        return Err(format!(
            "event-card generation blocked malformed event cards:\n{}",
            parse_errors.join("\n")
        ));
    }
    let (chain_index, planned_ids) =
        build_event_chain_index_for_mod_with_ids(&mod_root, &cards, prefix)?;
    enforce_strict_event_card_gate_with_chain(
        &map,
        &cards,
        game_index.as_ref(),
        Some(&chain_index),
        Some(&planned_ids),
    )?;
    let changed =
        apply_event_cards_to_mod_with_index(&mod_root, &cards, tag, prefix, game_index.as_ref())?;

    println!("Applied event cards: {}", cards.len());
    if changed.is_empty() {
        println!("No file changes were needed.");
    } else {
        println!("Changed:");
        for path in &changed {
            println!("  {}", path.display());
        }
    }
    run_post_apply_checks(&mod_root, &map, game_index.as_ref(), Some(&input))?;
    if let Some(output) = value(&map, "output") {
        let report = apply_writer_report_json(
            "hoi4skill.event_cards_apply.v1",
            &input,
            &mod_root,
            tag,
            prefix,
            "event_count",
            cards.len(),
            &changed,
        );
        write_or_print(&report, Some(output))?;
    }
    Ok(())
}

pub(crate) fn cmd_event_trigger_report(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let text = read_text_document(&input)?;
    let cards = parse_cards(&text, &["事件"]);
    let parse_errors = event_card_command_parse_errors(&text, &cards);
    if !parse_errors.is_empty() {
        return Err(format!(
            "event trigger report blocked malformed event cards:\n{}",
            parse_errors.join("\n")
        ));
    }
    let planned_ids = if let Some(mod_root) = mod_root.as_ref() {
        build_event_chain_index_for_mod_with_ids(mod_root, &cards, prefix)?.1
    } else {
        plan_event_card_ids(
            &cards,
            prefix,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )?
    };
    write_or_print(
        &event_trigger_report_json(&cards, &planned_ids, tag, prefix),
        value(&map, "output"),
    )
}

pub(crate) fn enforce_strict_event_card_gate_with_chain(
    map: &ArgMap,
    cards: &[Card],
    game_index: Option<&GameIndex>,
    chain_index: Option<&EventChainIndex>,
    planned_ids: Option<&[PlannedEventId]>,
) -> Result<(), String> {
    let options = validation_options_from_args(map);
    enforce_strict_event_card_gate_with_options_and_chain(
        options,
        cards,
        game_index,
        chain_index,
        planned_ids,
    )
}

pub(crate) fn enforce_strict_event_card_gate_with_options(
    options: ValidationOptions,
    cards: &[Card],
    game_index: Option<&GameIndex>,
) -> Result<(), String> {
    enforce_strict_event_card_gate_with_options_and_chain(options, cards, game_index, None, None)
}

pub(crate) fn enforce_strict_event_card_gate_with_options_and_chain(
    options: ValidationOptions,
    cards: &[Card],
    game_index: Option<&GameIndex>,
    chain_index: Option<&EventChainIndex>,
    planned_ids: Option<&[PlannedEventId]>,
) -> Result<(), String> {
    if !options.strict_code_index {
        return Ok(());
    }
    if game_index.is_none() {
        return Err(
            "strict event-card generation requires --game-root before writing files".to_string(),
        );
    }
    let fallback_planned_ids;
    let fallback_chain_index;
    let (chain_index, planned_ids) = match (chain_index, planned_ids) {
        (Some(chain_index), Some(planned_ids)) => (chain_index, planned_ids),
        (Some(chain_index), None) => {
            fallback_planned_ids = plan_event_card_ids(
                cards,
                "strict_event",
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
            )?;
            (chain_index, fallback_planned_ids.as_slice())
        }
        (None, _) => {
            fallback_planned_ids = plan_event_card_ids(
                cards,
                "strict_event",
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
            )?;
            fallback_chain_index = build_event_chain_index(cards, &fallback_planned_ids);
            (&fallback_chain_index, fallback_planned_ids.as_slice())
        }
    };
    let mut errors = Vec::new();
    for cycle in event_chain_unsafe_cycles(cards, planned_ids, chain_index) {
        errors.push(format!(
            "event chain has unsafe immediate unconditional cycle `{}`; add `延迟A`/`延迟小时A`/`随机延迟天数A`/`后续条件A` or remove one next-event edge before final generation",
            cycle.join(" -> ")
        ));
    }
    for card in cards {
        if let Some(index) = game_index {
            errors.extend(unindexed_explicit_event_picture_errors(
                &format!("事件 `{}` picture", card.title),
                card,
                index,
            ));
        }
        if let Some(trigger) = event_trigger_text(card) {
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
        if let Some(immediate) = event_immediate_effect_text(card) {
            let suggestions = event_option_effect_suggestions(immediate, Some(chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` immediate", card.title),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` immediate", card.title),
                    immediate,
                    index,
                ));
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` immediate", card.title),
                    &suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` immediate", card.title),
                    &suggestions,
                    index,
                ));
            }
        }
        if let Some(after) = event_after_effect_text(card) {
            let suggestions = event_option_effect_suggestions(after, Some(chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` after", card.title),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` after", card.title),
                    after,
                    index,
                ));
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` after", card.title),
                    &suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` after", card.title),
                    &suggestions,
                    index,
                ));
            }
        }
        if let Some(after_hidden) = event_after_hidden_effect_text(card) {
            let suggestions = event_option_effect_suggestions(after_hidden, Some(chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` after hidden_effect", card.title),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` after hidden_effect", card.title),
                    after_hidden,
                    index,
                ));
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` after hidden_effect", card.title),
                    &suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` after hidden_effect", card.title),
                    &suggestions,
                    index,
                ));
            }
        }
        for option in event_options(card) {
            if !option.trigger.trim().is_empty() {
                let suggestions = split_cn_list(&option.trigger)
                    .into_iter()
                    .flat_map(suggest_trigger)
                    .collect::<Vec<_>>();
                errors.extend(unresolved_suggestion_errors_with_index(
                    &format!("事件 `{}` option {} trigger", card.title, option.key),
                    &suggestions,
                    game_index,
                ));
                if let Some(index) = game_index {
                    errors.extend(missing_code_index_category_errors(
                        &format!("事件 `{}` option {} trigger", card.title, option.key),
                        &suggestions,
                        index,
                    ));
                    errors.extend(unindexed_suggestion_errors(
                        &format!("事件 `{}` option {} trigger", card.title, option.key),
                        &suggestions,
                        index,
                    ));
                }
            }
            let suggestions = event_option_effect_suggestions(&option.effects, Some(&chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` option {}", card.title, option.key),
                &suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` option {}", card.title, option.key),
                    &option.effects,
                    index,
                ));
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
            let tooltip_suggestions =
                event_option_effect_suggestions(&option.effect_tooltip_effects, Some(chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` option {} effect_tooltip", card.title, option.key),
                &tooltip_suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` option {} effect_tooltip", card.title, option.key),
                    &option.effect_tooltip_effects,
                    index,
                ));
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` option {} effect_tooltip", card.title, option.key),
                    &tooltip_suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` option {} effect_tooltip", card.title, option.key),
                    &tooltip_suggestions,
                    index,
                ));
            }
            let hidden_suggestions =
                event_option_effect_suggestions(&option.hidden_effects, Some(&chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` hidden option {}", card.title, option.key),
                &hidden_suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                errors.extend(event_chain_scope_errors_for_text(
                    &format!("事件 `{}` hidden option {}", card.title, option.key),
                    &option.hidden_effects,
                    index,
                ));
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
            let next_event_suggestions =
                event_option_next_event_suggestions(&option, Some(chain_index));
            errors.extend(unresolved_suggestion_errors_with_index(
                &format!("事件 `{}` next option {}", card.title, option.key),
                &next_event_suggestions,
                game_index,
            ));
            if let Some(index) = game_index {
                for entry in option
                    .next_events
                    .iter()
                    .chain(option.random_next_events.iter())
                {
                    errors.extend(event_option_next_event_condition_errors(
                        &format!(
                            "事件 `{}` next option {} {} condition",
                            card.title, option.key, entry.field
                        ),
                        entry,
                        game_index,
                    ));
                }
                for (_, next_event) in event_option_next_event_reference_texts(&option) {
                    errors.extend(event_chain_scope_errors_for_text(
                        &format!("事件 `{}` next option {}", card.title, option.key),
                        &next_event,
                        index,
                    ));
                }
                for (_, next_event) in event_option_random_next_event_reference_texts(&option) {
                    errors.extend(event_chain_scope_errors_for_text(
                        &format!("事件 `{}` random next option {}", card.title, option.key),
                        &next_event,
                        index,
                    ));
                }
                errors.extend(event_option_random_next_event_weight_errors(
                    &format!("事件 `{}` random next option {}", card.title, option.key),
                    &option,
                ));
                errors.extend(missing_code_index_category_errors(
                    &format!("事件 `{}` next option {}", card.title, option.key),
                    &next_event_suggestions,
                    index,
                ));
                errors.extend(unindexed_suggestion_errors(
                    &format!("事件 `{}` next option {}", card.title, option.key),
                    &next_event_suggestions,
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
    let source_label_errors = duplicate_event_source_label_errors(cards);
    if !source_label_errors.is_empty() {
        return Err(format!(
            "event-card generation blocked malformed event cards:\n{}",
            source_label_errors.join("\n")
        ));
    }
    let namespace_targets = scan_event_namespace_targets(mod_root)?;
    let existing_fingerprint_ids = scan_existing_event_card_ids(mod_root)?;
    let existing_source_ids = scan_existing_event_card_source_ids(mod_root)?;
    let existing_event_id_types = scan_existing_event_id_types(mod_root)?;
    let existing_fingerprints = existing_fingerprint_ids
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let picture_catalog = collect_event_picture_catalog(mod_root, game_index)?;
    let planned_ids = plan_event_card_ids(
        cards,
        prefix,
        &namespace_targets,
        &existing_fingerprint_ids,
        &existing_source_ids,
    )?;
    let mut chain_index = build_event_chain_index(cards, &planned_ids);
    chain_index
        .known_ids
        .extend(existing_event_id_types.keys().cloned());
    chain_index.id_to_event_type.extend(existing_event_id_types);
    let mut event_files: BTreeMap<PathBuf, EventFileAppend> = BTreeMap::new();
    let mut loc_entries = BTreeMap::new();

    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        let namespace = planned.namespace.clone();
        let fingerprint = event_card_fingerprint(card, &namespace);
        let event_id = planned.event_id.clone();
        if existing_fingerprints.contains(&fingerprint) {
            insert_event_localisation_with_index(card, &event_id, &mut loc_entries, game_index)?;
            continue;
        }
        let source_key = event_card_source_key(card, &namespace);
        let event_path = namespace_targets
            .get(&namespace)
            .map(|target| target.path.clone())
            .unwrap_or_else(|| mod_root.join("events").join(format!("{prefix}_events.txt")));
        let entry = event_files.entry(event_path).or_default();
        entry.namespaces.insert(namespace.clone());
        let picture = resolve_event_picture(card, &picture_catalog);
        entry.blocks.push(EventBlockWrite {
            event_id: event_id.to_string(),
            source_key: source_key.clone(),
            block: render_event_block_with_picture(
                card,
                &event_id,
                tag,
                prefix,
                Some(&fingerprint),
                Some(&source_key),
                &picture,
                Some(&chain_index),
            ),
        });
        insert_event_localisation_with_index(card, &event_id, &mut loc_entries, game_index)?;
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
        if upsert_event_localisation_entries(&path, &loc_entries)? {
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

pub(crate) fn explicit_event_picture_id(card: &Card) -> Option<String> {
    let value = card.fields.get("图片")?.trim().trim_matches('"');
    if is_reference_identifier(value) {
        Some(value.to_string())
    } else {
        None
    }
}

pub(crate) fn unindexed_explicit_event_picture_errors(
    context: &str,
    card: &Card,
    index: &GameIndex,
) -> Vec<String> {
    let Some(picture) = explicit_event_picture_id(card) else {
        return Vec::new();
    };
    if index.event_pictures.is_empty() {
        return vec![format!(
            "{context}: strict code index has no indexed event pictures; load the required game/dependency code before reusing event picture `{picture}`"
        )];
    }
    if index.event_pictures.contains(&picture) {
        return Vec::new();
    }
    let related = related_code_symbols_text(index, &picture, Some("event_picture"));
    vec![format!(
        "{context}: `图片：{picture}` references an unindexed event picture; verify it with `check-code-symbol --kind event_picture` before final generation{related}"
    )]
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
    pub(crate) blocks: Vec<EventBlockWrite>,
}

pub(crate) struct EventBlockWrite {
    pub(crate) event_id: String,
    pub(crate) source_key: String,
    pub(crate) block: String,
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

pub(crate) fn scan_existing_event_card_ids(
    mod_root: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut ids = BTreeMap::new();
    let events_root = mod_root.join("events");
    if !events_root.exists() {
        return Ok(ids);
    }
    for file in collect_files(&events_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut pending_fingerprint: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("# hoi4skill_card = ") {
                let value = rest.trim();
                pending_fingerprint = (!value.is_empty()).then(|| value.to_string());
                continue;
            }
            let Some(fingerprint) = pending_fingerprint.as_ref() else {
                continue;
            };
            if let Some(id) = find_assignment_in_text(trimmed, "id") {
                ids.insert(fingerprint.clone(), id.to_string());
                pending_fingerprint = None;
            }
        }
    }
    Ok(ids)
}

pub(crate) fn scan_existing_event_card_source_ids(
    mod_root: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut ids = BTreeMap::new();
    let events_root = mod_root.join("events");
    if !events_root.exists() {
        return Ok(ids);
    }
    for file in collect_files(&events_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut pending_source: Option<String> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("# hoi4skill_source = ") {
                let value = rest.trim();
                pending_source = (!value.is_empty()).then(|| value.to_string());
                continue;
            }
            let Some(source) = pending_source.as_ref() else {
                continue;
            };
            if let Some(id) = find_assignment_in_text(trimmed, "id") {
                ids.insert(source.clone(), id.to_string());
                pending_source = None;
            }
        }
    }
    Ok(ids)
}

pub(crate) fn scan_existing_event_id_types(
    mod_root: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut ids = BTreeMap::new();
    let events_root = mod_root.join("events");
    if !events_root.exists() {
        return Ok(ids);
    }
    for file in collect_files(&events_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for kind in ["country_event", "news_event", "state_event"] {
            for block in blocks_named(&text, kind) {
                if let Some(id) = block_assignment(&block, "id") {
                    ids.insert(id, kind.to_string());
                }
            }
        }
    }
    Ok(ids)
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

pub(crate) fn event_card_source_key(card: &Card, namespace: &str) -> String {
    let mut text = String::new();
    text.push_str(namespace);
    text.push('\n');
    text.push_str(&card.kind);
    text.push('\n');
    if let Some(source) = event_card_explicit_source_label(card) {
        text.push_str("explicit:");
        text.push_str(&source);
    } else {
        text.push_str("title:");
        text.push_str(&card.title);
    }
    text.push('\n');
    format!("evsrc_{:016x}", stable_hash64(&text))
}

pub(crate) fn event_card_explicit_source_label(card: &Card) -> Option<String> {
    [
        "事件键",
        "事件源",
        "事件稳定键",
        "稳定键",
        "稳定ID",
        "编辑键",
        "编辑ID",
        "剧情键",
        "链ID",
        "source_key",
        "event_key",
        "event_source",
        "source",
    ]
    .iter()
    .filter_map(|key| card.fields.get(*key))
    .map(|value| value.trim())
    .find(|value| !value.is_empty())
    .map(str::to_string)
}

pub(crate) fn stable_hash64(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Clone)]
pub(crate) struct PlannedEventId {
    pub(crate) namespace: String,
    pub(crate) event_id: String,
}

pub(crate) fn event_card_numbered_ids(cards: &[Card], prefix: &str) -> Vec<(String, String)> {
    plan_event_card_ids(
        cards,
        prefix,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap_or_default()
    .into_iter()
    .map(|planned| (planned.namespace, planned.event_id))
    .collect()
}

pub(crate) fn plan_event_card_ids(
    cards: &[Card],
    prefix: &str,
    namespace_targets: &BTreeMap<String, EventNamespaceTarget>,
    existing_fingerprint_ids: &BTreeMap<String, String>,
    existing_source_ids: &BTreeMap<String, String>,
) -> Result<Vec<PlannedEventId>, String> {
    let mut counters = namespace_targets
        .iter()
        .map(|(namespace, target)| (namespace.clone(), target.max_id))
        .collect::<BTreeMap<_, _>>();
    let event_id_max = active_event_id_max();
    cards
        .iter()
        .map(|card| {
            let namespace = event_card_namespace(card, prefix);
            let fingerprint = event_card_fingerprint(card, &namespace);
            if let Some(event_id) = existing_fingerprint_ids.get(&fingerprint) {
                return Ok(PlannedEventId {
                    namespace,
                    event_id: event_id.clone(),
                });
            }
            let source_key = event_card_source_key(card, &namespace);
            if let Some(event_id) = existing_source_ids.get(&source_key) {
                return Ok(PlannedEventId {
                    namespace,
                    event_id: event_id.clone(),
                });
            }
            let counter = counters.entry(namespace.clone()).or_insert(0);
            *counter += 1;
            if *counter > event_id_max {
                return Err(format!(
                    "namespace {namespace} has reached event id limit {event_id_max}"
                ));
            }
            Ok(PlannedEventId {
                event_id: format!("{}.{}", namespace, counter),
                namespace,
            })
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn build_event_chain_index_for_mod(
    mod_root: &Path,
    cards: &[Card],
    prefix: &str,
) -> Result<EventChainIndex, String> {
    Ok(build_event_chain_index_for_mod_with_ids(mod_root, cards, prefix)?.0)
}

pub(crate) fn build_event_chain_index_for_mod_with_ids(
    mod_root: &Path,
    cards: &[Card],
    prefix: &str,
) -> Result<(EventChainIndex, Vec<PlannedEventId>), String> {
    let namespace_targets = scan_event_namespace_targets(mod_root)?;
    let existing_fingerprint_ids = scan_existing_event_card_ids(mod_root)?;
    let existing_event_id_types = scan_existing_event_id_types(mod_root)?;
    let existing_source_ids = scan_existing_event_card_source_ids(mod_root)?;
    let planned_ids = plan_event_card_ids(
        cards,
        prefix,
        &namespace_targets,
        &existing_fingerprint_ids,
        &existing_source_ids,
    )?;
    let mut chain_index = build_event_chain_index(cards, &planned_ids);
    chain_index
        .known_ids
        .extend(existing_event_id_types.keys().cloned());
    chain_index.id_to_event_type.extend(existing_event_id_types);
    Ok((chain_index, planned_ids))
}

pub(crate) fn build_event_chain_index(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
) -> EventChainIndex {
    let mut title_to_id = BTreeMap::new();
    let mut title_to_event_type = BTreeMap::new();
    let mut id_to_event_type = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    let mut known_ids = BTreeSet::new();
    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        let event_type = normalize_event_type(card.fields.get("类型").map(String::as_str));
        known_ids.insert(planned.event_id.clone());
        id_to_event_type.insert(planned.event_id.clone(), event_type.to_string());
        for title in event_chain_titles(card) {
            if title.is_empty() {
                continue;
            }
            if title_to_id
                .insert(title.clone(), planned.event_id.clone())
                .is_some()
            {
                duplicates.insert(title.clone());
            }
            title_to_event_type.insert(title, event_type.to_string());
        }
    }
    for title in &duplicates {
        title_to_id.remove(title);
        title_to_event_type.remove(title);
    }
    EventChainIndex {
        title_to_id,
        title_to_event_type,
        id_to_event_type,
        duplicate_titles: duplicates,
        known_ids,
    }
}

pub(crate) fn event_chain_titles(card: &Card) -> Vec<String> {
    let mut titles = vec![card.title.trim().to_string()];
    if let Some(title) = card.fields.get("标题") {
        titles.push(title.trim().to_string());
    }
    if let Some(source) = event_card_explicit_source_label(card) {
        titles.push(source);
    }
    titles.sort();
    titles.dedup();
    titles
}

#[derive(Default)]
pub(crate) struct EventChainIndex {
    pub(crate) title_to_id: BTreeMap<String, String>,
    pub(crate) title_to_event_type: BTreeMap<String, String>,
    pub(crate) id_to_event_type: BTreeMap<String, String>,
    pub(crate) duplicate_titles: BTreeSet<String>,
    pub(crate) known_ids: BTreeSet<String>,
}

pub(crate) struct EventChainReference {
    pub(crate) event_type: &'static str,
    pub(crate) event_type_explicit: bool,
    pub(crate) title: String,
    pub(crate) explicit_event_id: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) days: Option<i64>,
    pub(crate) hours: Option<i64>,
    pub(crate) random_days: Option<i64>,
    pub(crate) random_hours: Option<i64>,
    pub(crate) trigger_for: Option<String>,
}

pub(crate) fn event_option_effect_suggestions(
    effects: &str,
    chain_index: Option<&EventChainIndex>,
) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();
    for raw in split_cn_list(effects) {
        if let Some(reference) = parse_event_chain_reference(raw) {
            suggestions.push(resolve_event_chain_reference(raw, &reference, chain_index));
        } else {
            suggestions.extend(suggest_common("event", raw, None, None, None, None));
        }
    }
    suggestions
}

pub(crate) fn event_option_next_event_suggestions(
    option: &EventOption,
    chain_index: Option<&EventChainIndex>,
) -> Vec<Suggestion> {
    let mut suggestions = option
        .next_events
        .iter()
        .filter_map(|entry| {
            event_option_next_event_entry_reference_text(entry)
                .map(|text| event_option_next_event_entry_suggestion(entry, &text, chain_index))
        })
        .collect::<Vec<_>>();
    suggestions.extend(event_option_random_next_event_suggestions(
        option,
        chain_index,
    ));
    suggestions
}

pub(crate) fn event_option_next_event_entry_suggestion(
    entry: &EventOptionNextEvent,
    text: &str,
    chain_index: Option<&EventChainIndex>,
) -> Suggestion {
    let suggestion = event_option_effect_suggestions(text, chain_index)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            Suggestion::new(
                "country_effect_candidate",
                &format!("<event id for {}>", entry.target.trim()),
                text,
                "Event-chain entry could not be parsed.",
            )
        });
    event_option_wrap_suggestion_with_condition(entry, suggestion)
}

pub(crate) fn event_option_wrap_suggestion_with_condition(
    entry: &EventOptionNextEvent,
    suggestion: Suggestion,
) -> Suggestion {
    let condition = entry.condition.trim();
    if condition.is_empty() {
        return suggestion;
    }
    let trigger_lines = event_option_next_event_condition_trigger_lines(condition);
    let mut code = String::from("if = {\n\tlimit = {\n");
    if trigger_lines.is_empty() {
        code.push_str(&format!("\t\t# {}\n", condition));
    }
    for line in trigger_lines {
        code.push_str(&indent_lines(&line, "\t\t"));
    }
    code.push_str("\t}\n");
    code.push_str(&indent_lines(&suggestion.code, "\t"));
    code.push('}');
    Suggestion::new(
        &suggestion.kind,
        &code,
        &format!("{} if {}", suggestion.source, condition),
        &suggestion.note,
    )
}

pub(crate) fn event_option_next_event_reference_texts(
    option: &EventOption,
) -> Vec<(String, String)> {
    option
        .next_events
        .iter()
        .filter_map(|entry| {
            event_option_next_event_entry_reference_text(entry)
                .map(|text| (entry.field.clone(), text))
        })
        .collect()
}

pub(crate) fn event_option_random_next_event_reference_texts(
    option: &EventOption,
) -> Vec<(String, String)> {
    option
        .random_next_events
        .iter()
        .filter_map(|entry| {
            event_option_next_event_entry_reference_text(entry)
                .map(|text| (entry.field.clone(), text))
        })
        .collect()
}

pub(crate) fn event_option_random_next_event_suggestions(
    option: &EventOption,
    chain_index: Option<&EventChainIndex>,
) -> Vec<Suggestion> {
    let entries = option
        .random_next_events
        .iter()
        .filter_map(|entry| {
            event_option_next_event_entry_reference_text(entry).map(|text| (entry, text))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        return Vec::new();
    }
    let mut code = String::from("random_list = {\n");
    let mut source = Vec::new();
    for (entry, text) in entries {
        let weight = event_option_random_next_event_weight(entry).unwrap_or(100);
        let suggestion = event_option_next_event_entry_suggestion(entry, &text, chain_index);
        code.push_str(&format!("\t{weight} = {{\n"));
        code.push_str(&indent_lines(&suggestion.code, "\t\t"));
        code.push_str("\t}\n");
        source.push(text);
    }
    code.push('}');
    vec![Suggestion::new(
        "country_effect",
        &code,
        &source.join(" | "),
        "Resolved random event-chain branch from structured event-card fields.",
    )]
}

pub(crate) fn event_option_next_event_condition_trigger_lines(condition: &str) -> Vec<String> {
    split_cn_list(condition)
        .into_iter()
        .flat_map(suggest_trigger)
        .filter_map(|suggestion| {
            if suggestion.kind == "trigger" {
                concrete_suggestion_code(&suggestion)
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn event_option_next_event_condition_suggestions(
    entry: &EventOptionNextEvent,
) -> Vec<Suggestion> {
    split_cn_list(&entry.condition)
        .into_iter()
        .flat_map(suggest_trigger)
        .collect()
}

pub(crate) fn event_option_next_event_condition_errors(
    context: &str,
    entry: &EventOptionNextEvent,
    game_index: Option<&GameIndex>,
) -> Vec<String> {
    if entry.condition.trim().is_empty() {
        return Vec::new();
    }
    let suggestions = event_option_next_event_condition_suggestions(entry);
    let mut errors = unresolved_suggestion_errors_with_index(context, &suggestions, game_index);
    if let Some(index) = game_index {
        errors.extend(missing_code_index_category_errors(
            context,
            &suggestions,
            index,
        ));
        errors.extend(unindexed_suggestion_errors(context, &suggestions, index));
    }
    errors
}

pub(crate) fn event_option_random_next_event_weight(entry: &EventOptionNextEvent) -> Option<i64> {
    let value = entry.weight.trim();
    if value.is_empty() {
        Some(100)
    } else {
        parse_int(value).filter(|weight| *weight > 0)
    }
}

pub(crate) fn event_option_random_next_event_weight_errors(
    context: &str,
    option: &EventOption,
) -> Vec<String> {
    option
        .random_next_events
        .iter()
        .filter_map(|entry| {
            let weight = entry.weight.trim();
            if weight.is_empty() || event_option_random_next_event_weight(entry).is_some() {
                None
            } else {
                Some(format!(
                    "{context}: `{}` uses invalid random next-event weight `{weight}`; use a positive integer before final generation",
                    entry.target.trim()
                ))
            }
        })
        .collect()
}

pub(crate) fn event_option_next_event_entry_reference_text(
    entry: &EventOptionNextEvent,
) -> Option<String> {
    let target = entry.target.trim();
    if target.is_empty() {
        return None;
    }
    let mut text = if target.contains("触发事件")
        || target.contains("触发国家事件")
        || target.contains("触发州事件")
        || target.contains("触发省份事件")
        || target.contains("触发地区事件")
        || target.contains("触发新闻")
    {
        target.to_string()
    } else {
        let next_type = normalize_event_type(Some(&entry.event_type));
        let trigger = match next_type {
            "news_event" => "触发新闻",
            "state_event" => "触发州事件",
            _ => "触发事件",
        };
        let mut timing = Vec::new();
        if let Some(days) = parse_int(&entry.days).filter(|days| *days > 0) {
            timing.push(format!("days = {days}"));
        }
        if let Some(hours) = parse_int(&entry.hours).filter(|hours| *hours > 0) {
            timing.push(format!("hours = {hours}"));
        }
        if let Some(random_days) =
            parse_int(&entry.random_days).filter(|random_days| *random_days > 0)
        {
            timing.push(format!("random_days = {random_days}"));
        }
        if let Some(random_hours) =
            parse_int(&entry.random_hours).filter(|random_hours| *random_hours > 0)
        {
            timing.push(format!("random_hours = {random_hours}"));
        }
        let timing = if timing.is_empty() {
            String::new()
        } else {
            format!(" {}", timing.join(" "))
        };
        let trigger_for = event_option_next_event_entry_trigger_for(entry, next_type)
            .map(|value| format!(" trigger_for = {value}"))
            .unwrap_or_default();
        format!("{trigger} {target}{timing}{trigger_for}")
    };
    if let Some(scope) = event_option_next_event_entry_scope(entry) {
        if event_chain_scope(&text).is_none() {
            text.push_str(&format!(
                " scope = {}",
                event_chain_scope_assignment_value(&scope)
            ));
        }
    }
    Some(text)
}

pub(crate) fn parse_event_chain_reference(raw: &str) -> Option<EventChainReference> {
    let event_type_explicit = raw.contains("触发新闻")
        || raw.contains("触发国家事件")
        || raw.contains("触发州事件")
        || raw.contains("触发省份事件")
        || raw.contains("触发地区事件")
        || raw.contains("触发state_event");
    let event_type = if raw.contains("触发新闻") {
        "news_event"
    } else if raw.contains("触发州事件")
        || raw.contains("触发省份事件")
        || raw.contains("触发地区事件")
        || raw.contains("触发state_event")
    {
        "state_event"
    } else if raw.contains("触发国家事件") || raw.contains("触发事件") {
        "country_event"
    } else {
        return None;
    };
    let trigger_for = event_chain_trigger_for(raw);
    let scope = event_chain_scope(raw);
    let mut target = raw
        .replace("触发事件ID", "")
        .replace("触发事件id", "")
        .replace("触发国家事件", "")
        .replace("触发州事件", "")
        .replace("触发省份事件", "")
        .replace("触发地区事件", "")
        .replace("触发state_event", "")
        .replace("触发事件", "")
        .replace("触发新闻事件", "")
        .replace("触发新闻", "")
        .replace("事件ID", "")
        .replace("事件id", "")
        .replace("天后", "天")
        .replace("日后", "日")
        .replace("延迟", "")
        .replace("之后", "")
        .replace("以后", "")
        .replace("立即", "")
        .replace([':', '：'], " ");
    target = strip_event_chain_trigger_for(&target);
    target = strip_event_chain_scope(&target);
    let days = parse_event_chain_delay_days(&target);
    let hours = parse_event_chain_delay_hours(&target);
    let random_days = event_chain_assignment_i64(&target, "random_days");
    let random_hours = event_chain_assignment_i64(&target, "random_hours");
    target = target.trim().trim_matches('"').to_string();
    let explicit_candidate = strip_event_chain_time_tokens(&target);
    if event_id_namespace_number(&explicit_candidate).is_some() {
        return Some(EventChainReference {
            event_type,
            event_type_explicit,
            title: explicit_candidate.clone(),
            explicit_event_id: Some(explicit_candidate),
            scope,
            days,
            hours,
            random_days,
            random_hours,
            trigger_for,
        });
    }
    let title =
        if days.is_some() || hours.is_some() || random_days.is_some() || random_hours.is_some() {
            strip_event_chain_time_tokens(&target)
        } else {
            target
        };
    (!title.is_empty()).then_some(EventChainReference {
        event_type,
        event_type_explicit,
        title,
        explicit_event_id: None,
        scope,
        days,
        hours,
        random_days,
        random_hours,
        trigger_for,
    })
}

pub(crate) fn event_option_next_event_entry_trigger_for(
    entry: &EventOptionNextEvent,
    next_type: &str,
) -> Option<String> {
    let value = entry.trigger_for.trim();
    if !value.is_empty() {
        return Some(value.trim_matches('"').to_string());
    }
    (next_type == "state_event").then(|| "controller".to_string())
}

pub(crate) fn event_option_next_event_entry_scope(entry: &EventOptionNextEvent) -> Option<String> {
    let value = entry.scope.trim();
    (!value.is_empty()).then(|| value.trim_matches('"').to_string())
}

pub(crate) fn event_chain_trigger_for(raw: &str) -> Option<String> {
    find_assignment_in_text(raw, "trigger_for")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').to_string())
}

pub(crate) fn event_chain_scope(raw: &str) -> Option<String> {
    find_assignment_in_text(raw, "scope")
        .or_else(|| find_assignment_in_text(raw, "target_scope"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_matches('"').to_string())
}

pub(crate) fn strip_event_chain_trigger_for(text: &str) -> String {
    strip_event_chain_assignment_tokens(text, &["trigger_for"])
}

pub(crate) fn strip_event_chain_scope(text: &str) -> String {
    strip_event_chain_assignment_tokens(text, &["scope", "target_scope"])
}

pub(crate) fn strip_event_chain_assignment_tokens(text: &str, keys: &[&str]) -> String {
    let mut out = Vec::new();
    let mut iter = text.split_whitespace().peekable();
    while let Some(token) = iter.next() {
        if keys.contains(&token) {
            if iter.peek().is_some_and(|next| *next == "=") {
                iter.next();
            }
            if iter.peek().is_some() {
                iter.next();
            }
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            if keys.contains(&key) {
                if value.is_empty() && iter.peek().is_some() {
                    iter.next();
                }
                continue;
            }
        }
        out.push(token);
    }
    out.join(" ")
}

pub(crate) fn strip_event_chain_time_tokens(text: &str) -> String {
    let mut out = Vec::new();
    let mut iter = text.split_whitespace().peekable();
    while let Some(token) = iter.next() {
        if is_event_chain_timing_key(token) {
            if iter.peek().is_some_and(|next| *next == "=") {
                iter.next();
            }
            if iter.peek().is_some() {
                iter.next();
            }
            continue;
        }
        if let Some(value) = token.split_once('=') {
            if is_event_chain_timing_key(value.0) {
                if value.1.is_empty() && iter.peek().is_some() {
                    iter.next();
                }
                continue;
            }
        }
        if is_event_chain_delay_token(token) {
            continue;
        }
        out.push(token);
    }
    let stripped = out.join(" ");
    let stripped = stripped.trim().trim_matches('"');
    if stripped == text.trim().trim_matches('"') {
        strip_leading_event_chain_delay(stripped)
    } else {
        stripped.to_string()
    }
}

pub(crate) fn is_event_chain_timing_key(token: &str) -> bool {
    matches!(
        token,
        "days" | "hours" | "random_days" | "random_hours" | "months" | "random_months"
    )
}

pub(crate) fn is_event_chain_delay_token(token: &str) -> bool {
    (token.contains('天') || token.contains('日') || token.contains("小时"))
        && parse_int(token).is_some()
        && token.chars().all(|ch| {
            ch.is_ascii_digit() || is_fullwidth_digit(ch) || matches!(ch, '天' | '日' | '小' | '时')
        })
}

pub(crate) fn parse_event_chain_delay_days(text: &str) -> Option<i64> {
    if let Some(days) = event_chain_assignment_i64(text, "days") {
        return Some(days);
    }
    let marker = text.find(['天', '日'])?;
    parse_int(&text[..marker]).filter(|days| *days > 0)
}

pub(crate) fn parse_event_chain_delay_hours(text: &str) -> Option<i64> {
    if let Some(hours) = event_chain_assignment_i64(text, "hours") {
        return Some(hours);
    }
    let marker = text.find("小时")?;
    parse_int(&text[..marker]).filter(|hours| *hours > 0)
}

pub(crate) fn event_chain_assignment_i64(text: &str, key: &str) -> Option<i64> {
    find_assignment_in_text(text, key)
        .and_then(parse_int)
        .filter(|value| *value > 0)
}

pub(crate) fn strip_leading_event_chain_delay(text: &str) -> String {
    let mut seen_number = false;
    for (idx, ch) in text.char_indices() {
        if ch.is_ascii_digit() || is_fullwidth_digit(ch) {
            seen_number = true;
            continue;
        }
        if seen_number && matches!(ch, '天' | '日') {
            let end_idx = idx + ch.len_utf8();
            return text[end_idx..].trim().trim_matches('"').to_string();
        }
        if seen_number && ch == '小' && text[idx..].starts_with("小时") {
            let end_idx = idx + "小时".len();
            return text[end_idx..].trim().trim_matches('"').to_string();
        }
        break;
    }
    text.trim().trim_matches('"').to_string()
}

pub(crate) fn is_fullwidth_digit(ch: char) -> bool {
    matches!(
        ch,
        '０' | '１' | '２' | '３' | '４' | '５' | '６' | '７' | '８' | '９'
    )
}

pub(crate) fn resolve_event_chain_reference(
    source: &str,
    reference: &EventChainReference,
    chain_index: Option<&EventChainIndex>,
) -> Suggestion {
    let Some(chain_index) = chain_index else {
        return unresolved_event_chain_reference(source, reference, "");
    };
    if let Some(event_id) = reference.explicit_event_id.as_ref() {
        if chain_index.known_ids.contains(event_id) {
            let event_type =
                event_chain_reference_effect_type(reference, chain_index, Some(event_id));
            return resolved_event_chain_suggestion(
                source,
                &event_type,
                event_id,
                reference.days,
                reference.hours,
                reference.random_days,
                reference.random_hours,
                reference.trigger_for.as_deref(),
                reference.scope.as_deref(),
            );
        }
        return unresolved_event_chain_reference(
            source,
            reference,
            "Explicit event id is not known in the current batch or scanned mod events; verify it before final code.",
        );
    }
    if chain_index.duplicate_titles.contains(&reference.title) {
        return unresolved_event_chain_reference(
            source,
            reference,
            "Event title is duplicated in this batch; add unique `事件键` values to the target events and reference the intended event key before final code.",
        );
    }
    let Some(event_id) = chain_index.title_to_id.get(&reference.title) else {
        return unresolved_event_chain_reference(
            source,
            reference,
            "Event title was not found in this batch; add the target event card or use an explicit verified event id.",
        );
    };
    let event_type = event_chain_reference_effect_type(reference, chain_index, Some(event_id));
    resolved_event_chain_suggestion(
        source,
        &event_type,
        event_id,
        reference.days,
        reference.hours,
        reference.random_days,
        reference.random_hours,
        reference.trigger_for.as_deref(),
        reference.scope.as_deref(),
    )
}

pub(crate) fn event_chain_reference_effect_type(
    reference: &EventChainReference,
    chain_index: &EventChainIndex,
    event_id: Option<&str>,
) -> String {
    if reference.event_type_explicit {
        return reference.event_type.to_string();
    }
    if let Some(event_id) = event_id {
        if let Some(event_type) = chain_index.id_to_event_type.get(event_id) {
            return event_type.clone();
        }
    }
    if let Some(event_type) = chain_index.title_to_event_type.get(&reference.title) {
        return event_type.clone();
    }
    reference.event_type.to_string()
}

pub(crate) fn resolved_event_chain_suggestion(
    source: &str,
    event_type: &str,
    event_id: &str,
    days: Option<i64>,
    hours: Option<i64>,
    random_days: Option<i64>,
    random_hours: Option<i64>,
    trigger_for: Option<&str>,
    scope: Option<&str>,
) -> Suggestion {
    let mut code = format!("{event_type} = {{ id = {event_id}");
    if let Some(days) = days {
        code.push_str(&format!(" days = {days}"));
    }
    if let Some(hours) = hours {
        code.push_str(&format!(" hours = {hours}"));
    }
    if let Some(random_days) = random_days {
        code.push_str(&format!(" random_days = {random_days}"));
    }
    if let Some(random_hours) = random_hours {
        code.push_str(&format!(" random_hours = {random_hours}"));
    }
    if event_type == "state_event" {
        if let Some(trigger_for) = trigger_for {
            code.push_str(&format!(" trigger_for = {trigger_for}"));
        }
    }
    code.push_str(" }");
    code = wrap_event_chain_code_with_scope(code, scope);
    Suggestion::new(
        "country_effect",
        &code,
        source,
        "Resolved event-chain target from the current event-card batch.",
    )
}

pub(crate) fn unresolved_event_chain_reference(
    source: &str,
    reference: &EventChainReference,
    note: &str,
) -> Suggestion {
    let mut code = format!(
        "{} = {{ id = <event id for {}>",
        reference.event_type, reference.title
    );
    if let Some(days) = reference.days {
        code.push_str(&format!(" days = {days}"));
    }
    if let Some(hours) = reference.hours {
        code.push_str(&format!(" hours = {hours}"));
    }
    if let Some(random_days) = reference.random_days {
        code.push_str(&format!(" random_days = {random_days}"));
    }
    if let Some(random_hours) = reference.random_hours {
        code.push_str(&format!(" random_hours = {random_hours}"));
    }
    if reference.event_type == "state_event" {
        if let Some(trigger_for) = reference.trigger_for.as_deref() {
            code.push_str(&format!(" trigger_for = {trigger_for}"));
        }
    }
    code.push_str(" }");
    code = wrap_event_chain_code_with_scope(code, reference.scope.as_deref());
    Suggestion::new("country_effect_candidate", &code, source, note)
}

pub(crate) fn normalize_event_chain_scope(scope: Option<&str>) -> Option<String> {
    scope
        .map(str::trim)
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.trim_matches('"').to_string())
}

pub(crate) fn event_chain_scope_assignment_value(scope: &str) -> String {
    let trimmed = scope.trim().trim_matches('"');
    if trimmed.chars().any(char::is_whitespace) {
        format!("\"{}\"", trimmed.replace('"', ""))
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn event_chain_scope_segments(scope: &str) -> Option<Vec<String>> {
    let normalized = normalize_event_chain_scope(Some(scope))?;
    let segments = normalized
        .split(|ch: char| matches!(ch, '.' | '>' | '/' | '\\'))
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !is_event_chain_scope_segment(segment))
    {
        None
    } else {
        Some(segments)
    }
}

pub(crate) fn is_event_chain_scope_segment(segment: &str) -> bool {
    matches!(
        segment,
        "ROOT" | "FROM" | "PREV" | "THIS" | "OVERLORD" | "overlord" | "owner" | "controller"
    ) || looks_like_tag(segment)
        || parse_plain_i64(segment).is_some()
}

pub(crate) fn wrap_event_chain_code_with_scope(mut code: String, scope: Option<&str>) -> String {
    let Some(scope) = normalize_event_chain_scope(scope) else {
        return code;
    };
    let Some(segments) = event_chain_scope_segments(&scope) else {
        return format!("<invalid event scope> = {{ {code} }}");
    };
    for segment in segments.iter().rev() {
        code = format!("{segment} = {{ {code} }}");
    }
    code
}

pub(crate) fn event_chain_scope_errors_for_text(
    context: &str,
    text: &str,
    index: &GameIndex,
) -> Vec<String> {
    let mut errors = Vec::new();
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        if let Some(scope) = reference.scope.as_deref() {
            errors.extend(event_chain_scope_errors(context, &raw, scope, index));
        }
    }
    errors
}

pub(crate) fn event_chain_scope_errors(
    context: &str,
    source: &str,
    scope: &str,
    index: &GameIndex,
) -> Vec<String> {
    let Some(segments) = event_chain_scope_segments(scope) else {
        return vec![format!(
            "{context}: `{source}` uses invalid event-chain scope `{}`; use ROOT/FROM/PREV/THIS/owner/controller or a verified country tag before final generation",
            scope.trim()
        )];
    };
    let mut errors = Vec::new();
    for segment in segments {
        if looks_like_tag(&segment)
            && !index.country_tags.contains(&segment)
            && !is_dynamic_tag_ref(&segment)
        {
            let related = related_code_symbols_text(index, &segment, Some("country_tag"));
            errors.push(format!(
                "{context}: `{source}` uses unindexed event-chain country tag scope `{segment}`; verify it with `resolve-country-tag` or the mod index before final generation{related}"
            ));
        }
    }
    errors
}

pub(crate) fn render_event_block_with_picture(
    card: &Card,
    event_id: &str,
    tag: &str,
    prefix: &str,
    fingerprint: Option<&str>,
    source_key: Option<&str>,
    picture: &str,
    chain_index: Option<&EventChainIndex>,
) -> String {
    let event_type = normalize_event_type(card.fields.get("类型").map(String::as_str));
    let trigger = event_trigger_text(card);
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
    if let Some(source_key) = source_key {
        out.push_str(&format!("# hoi4skill_source = {source_key}\n"));
    }
    out.push_str(&format!("{event_type} = {{\n"));
    out.push_str(&format!("\tid = {event_id}\n"));
    out.push_str(&format!("\ttitle = {event_id}.t\n"));
    out.push_str(&format!("\tdesc = {event_id}.d\n"));
    out.push_str(&format!("\tpicture = {picture}\n"));
    if let Some(days) = event_timeout_days(card) {
        out.push_str(&format!("\ttimeout_days = {days}\n"));
    }
    if let Some(days) = event_mean_time_to_happen_days(card) {
        out.push_str("\tmean_time_to_happen = {\n");
        out.push_str(&format!("\t\tdays = {days}\n"));
        out.push_str("\t}\n");
    } else {
        out.push_str("\tis_triggered_only = yes\n");
    }
    for (field, aliases) in [
        (
            "fire_only_once",
            &["只触发一次", "仅触发一次", "一次性", "fire_only_once"][..],
        ),
        ("major", &["主要事件", "重大事件", "major"][..]),
        (
            "show_major",
            &["显示主要", "显示为主要事件", "show_major"][..],
        ),
        ("hidden", &["隐藏", "隐藏事件", "hidden"][..]),
    ] {
        if let Some(value) = event_bool_field_value(card, aliases) {
            out.push_str(&format!("\t{field} = {}\n", event_bool_code(value)));
        }
    }
    if !trigger_lines.is_empty() {
        out.push_str("\ttrigger = {\n");
        for line in trigger_lines {
            out.push_str(&format!("\t\t{line}\n"));
        }
        out.push_str("\t}\n");
    } else if trigger.is_some() {
        out.push_str("\t# TODO: map event trigger candidates before enabling a trigger block\n");
    }
    if let Some(immediate) = event_immediate_effect_text(card) {
        let suggestions = event_option_effect_suggestions(immediate, chain_index);
        let (immediate_lines, immediate_comments) = decision_effect_lines(&suggestions);
        out.push_str("\timmediate = {\n");
        if immediate_lines.is_empty() && immediate_comments.is_empty() {
            out.push_str(&format!("\t\t# {}\n", immediate.trim()));
        }
        for comment in immediate_comments {
            out.push_str(&format!("\t\t# {comment}\n"));
        }
        for line in immediate_lines {
            out.push_str(&indent_lines(&line, "\t\t"));
        }
        out.push_str("\t}\n");
    }
    let after = event_after_effect_text(card);
    let after_hidden = event_after_hidden_effect_text(card);
    if after.is_some() || after_hidden.is_some() {
        out.push_str("\tafter = {\n");
        if let Some(after) = after {
            let suggestions = event_option_effect_suggestions(after, chain_index);
            let (after_lines, after_comments) = decision_effect_lines(&suggestions);
            if after_lines.is_empty() && after_comments.is_empty() {
                out.push_str(&format!("\t\t# {}\n", after.trim()));
            }
            for comment in after_comments {
                out.push_str(&format!("\t\t# {comment}\n"));
            }
            for line in after_lines {
                out.push_str(&indent_lines(&line, "\t\t"));
            }
        }
        if let Some(after_hidden) = after_hidden {
            let suggestions = event_option_effect_suggestions(after_hidden, chain_index);
            let (hidden_lines, hidden_comments) = decision_effect_lines(&suggestions);
            out.push_str("\t\thidden_effect = {\n");
            if hidden_lines.is_empty() && hidden_comments.is_empty() {
                out.push_str(&format!("\t\t\t# {}\n", after_hidden.trim()));
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
    }
    for option in options {
        out.push_str(&render_event_option(&option, event_id, chain_index));
    }
    if !card.fields.contains_key("命名空间") && prefix != tag {
        out.push_str(&format!("\t# namespace inferred from prefix: {prefix}\n"));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_event_option(
    option: &EventOption,
    event_id: &str,
    chain_index: Option<&EventChainIndex>,
) -> String {
    let mut out = String::new();
    out.push_str("\n\toption = {\n");
    out.push_str(&format!("\t\tname = {event_id}.{}\n", option.key));
    if !option.tooltip.trim().is_empty() {
        out.push_str(&format!(
            "\t\tcustom_effect_tooltip = {event_id}.{}.tt\n",
            option.key
        ));
    }
    if !option.trigger.trim().is_empty() {
        let trigger_lines = split_cn_list(&option.trigger)
            .into_iter()
            .flat_map(suggest_trigger)
            .filter_map(|suggestion| {
                if suggestion.kind == "trigger" {
                    concrete_suggestion_code(&suggestion)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        out.push_str("\t\ttrigger = {\n");
        if trigger_lines.is_empty() {
            out.push_str(&format!("\t\t\t# {}\n", option.trigger.trim()));
        }
        for line in trigger_lines {
            out.push_str(&format!("\t\t\t{line}\n"));
        }
        out.push_str("\t\t}\n");
    }
    if !option.ai_chance.trim().is_empty() {
        if let Some(factor) = parse_int(&option.ai_chance) {
            out.push_str("\t\tai_chance = {\n");
            out.push_str(&format!("\t\t\tfactor = {factor}\n"));
            out.push_str("\t\t}\n");
        }
    }
    let suggestions = event_option_effect_suggestions(&option.effects, chain_index);
    let (effect_lines, effect_comments) = decision_effect_lines(&suggestions);
    for comment in effect_comments {
        out.push_str(&format!("\t\t# {comment}\n"));
    }
    for line in effect_lines {
        out.push_str(&indent_lines(&line, "\t\t"));
    }
    if !option.effect_tooltip_effects.trim().is_empty() {
        out.push_str("\t\teffect_tooltip = {\n");
        let tooltip_suggestions =
            event_option_effect_suggestions(&option.effect_tooltip_effects, chain_index);
        let (tooltip_lines, tooltip_comments) = decision_effect_lines(&tooltip_suggestions);
        if tooltip_lines.is_empty() && tooltip_comments.is_empty() {
            out.push_str(&format!(
                "\t\t\t# {}\n",
                option.effect_tooltip_effects.trim()
            ));
        }
        for comment in tooltip_comments {
            out.push_str(&format!("\t\t\t# {comment}\n"));
        }
        for line in tooltip_lines {
            out.push_str(&indent_lines(&line, "\t\t\t"));
        }
        out.push_str("\t\t}\n");
    }
    if !option.hidden_effects.trim().is_empty()
        || !option.next_events.is_empty()
        || !option.random_next_events.is_empty()
    {
        out.push_str("\t\thidden_effect = {\n");
        let mut hidden_suggestions =
            event_option_effect_suggestions(&option.hidden_effects, chain_index);
        hidden_suggestions.extend(event_option_next_event_suggestions(option, chain_index));
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
    pub(crate) tooltip: String,
    pub(crate) effects: String,
    pub(crate) effect_tooltip_effects: String,
    pub(crate) hidden_effects: String,
    pub(crate) trigger: String,
    pub(crate) next_event: String,
    pub(crate) next_event_type: String,
    pub(crate) next_event_days: String,
    pub(crate) next_event_hours: String,
    pub(crate) next_event_random_days: String,
    pub(crate) next_event_random_hours: String,
    pub(crate) next_event_trigger_for: String,
    pub(crate) next_event_scope: String,
    pub(crate) next_events: Vec<EventOptionNextEvent>,
    pub(crate) random_next_events: Vec<EventOptionNextEvent>,
    pub(crate) ai_chance: String,
}

#[derive(Clone)]
pub(crate) struct EventOptionNextEvent {
    pub(crate) field: String,
    pub(crate) target: String,
    pub(crate) event_type: String,
    pub(crate) days: String,
    pub(crate) hours: String,
    pub(crate) random_days: String,
    pub(crate) random_hours: String,
    pub(crate) trigger_for: String,
    pub(crate) scope: String,
    pub(crate) weight: String,
    pub(crate) condition: String,
}

pub(crate) fn event_options(card: &Card) -> Vec<EventOption> {
    let mut options = BTreeMap::new();
    let mut suffixes = BTreeSet::new();
    for key in card.fields.keys() {
        if let Some((base_suffix, _)) = numbered_next_event_suffix_for_key(key) {
            suffixes.insert(base_suffix);
            continue;
        }
        if let Some((base_suffix, _)) = random_next_event_suffix_for_key(key) {
            suffixes.insert(base_suffix);
            continue;
        }
        if let Some(suffix) = event_option_field_suffix(key) {
            suffixes.insert(suffix.to_string());
        }
    }
    for suffix in suffixes {
        let option_key = option_key(&suffix);
        let option_text = card
            .fields
            .get(&format!("选项{suffix}"))
            .cloned()
            .unwrap_or_else(|| "好的。".to_string());
        let tooltip = event_option_tooltip_text(card, &suffix)
            .map(str::to_string)
            .unwrap_or_default();
        let effects = card
            .fields
            .get(&format!("效果{suffix}"))
            .cloned()
            .unwrap_or_default();
        let effect_tooltip_effects = event_option_effect_tooltip_text(card, &suffix)
            .map(str::to_string)
            .unwrap_or_default();
        let hidden_effects = card
            .fields
            .get(&format!("隐藏效果{suffix}"))
            .cloned()
            .unwrap_or_default();
        let trigger = event_option_trigger_text(card, &suffix)
            .map(str::to_string)
            .unwrap_or_default();
        let next_event = card
            .fields
            .get(&format!("后续事件{suffix}"))
            .or_else(|| card.fields.get(&format!("下一事件{suffix}")))
            .or_else(|| card.fields.get(&format!("触发事件{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_type = card
            .fields
            .get(&format!("后续类型{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件类型{suffix}")))
            .or_else(|| card.fields.get(&format!("下一事件类型{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_days = card
            .fields
            .get(&format!("延迟{suffix}"))
            .or_else(|| card.fields.get(&format!("延迟天数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续天数{suffix}")))
            .or_else(|| card.fields.get(&format!("days{suffix}")))
            .or_else(|| card.fields.get(&format!("后续延迟{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_hours = card
            .fields
            .get(&format!("延迟小时{suffix}"))
            .or_else(|| card.fields.get(&format!("延迟小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续小时{suffix}")))
            .or_else(|| card.fields.get(&format!("后续小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("hours{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_random_days = card
            .fields
            .get(&format!("随机延迟{suffix}"))
            .or_else(|| card.fields.get(&format!("随机延迟天数{suffix}")))
            .or_else(|| card.fields.get(&format!("随机天数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续随机天数{suffix}")))
            .or_else(|| card.fields.get(&format!("random_days{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_random_hours = card
            .fields
            .get(&format!("随机延迟小时{suffix}"))
            .or_else(|| card.fields.get(&format!("随机延迟小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("随机小时{suffix}")))
            .or_else(|| card.fields.get(&format!("后续随机小时{suffix}")))
            .or_else(|| card.fields.get(&format!("random_hours{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_trigger_for = card
            .fields
            .get(&format!("后续触发对象{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件触发对象{suffix}")))
            .or_else(|| card.fields.get(&format!("触发对象{suffix}")))
            .or_else(|| card.fields.get(&format!("trigger_for{suffix}")))
            .or_else(|| card.fields.get(&format!("后续trigger_for{suffix}")))
            .cloned()
            .unwrap_or_default();
        let next_event_scope = card
            .fields
            .get(&format!("后续作用域{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件作用域{suffix}")))
            .or_else(|| card.fields.get(&format!("后续目标{suffix}")))
            .or_else(|| card.fields.get(&format!("后续范围{suffix}")))
            .or_else(|| card.fields.get(&format!("作用域{suffix}")))
            .or_else(|| card.fields.get(&format!("scope{suffix}")))
            .or_else(|| card.fields.get(&format!("target_scope{suffix}")))
            .or_else(|| card.fields.get(&format!("后续scope{suffix}")))
            .cloned()
            .unwrap_or_default();
        let mut next_events = Vec::new();
        if !next_event.trim().is_empty() {
            next_events.push(EventOptionNextEvent {
                field: "next_event".to_string(),
                target: next_event.clone(),
                event_type: next_event_type.clone(),
                days: next_event_days.clone(),
                hours: next_event_hours.clone(),
                random_days: next_event_random_days.clone(),
                random_hours: next_event_random_hours.clone(),
                trigger_for: next_event_trigger_for.clone(),
                scope: next_event_scope.clone(),
                weight: String::new(),
                condition: event_option_next_event_condition(card, &suffix),
            });
        }
        for numbered_suffix in numbered_next_event_suffixes(card, &suffix) {
            let entry = event_option_next_event_entry(card, &numbered_suffix);
            if !entry.target.trim().is_empty() {
                next_events.push(entry);
            }
        }
        let mut random_next_events = Vec::new();
        for random_suffix in random_next_event_suffixes(card, &suffix) {
            let entry = event_option_random_next_event_entry(card, &random_suffix);
            if !entry.target.trim().is_empty() {
                random_next_events.push(entry);
            }
        }
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
                text: option_text,
                tooltip,
                effects,
                effect_tooltip_effects,
                hidden_effects,
                trigger,
                next_event,
                next_event_type,
                next_event_days,
                next_event_hours,
                next_event_random_days,
                next_event_random_hours,
                next_event_trigger_for,
                next_event_scope,
                next_events,
                random_next_events,
                ai_chance,
            },
        );
    }
    if options.is_empty() {
        options.insert(
            "a".to_string(),
            EventOption {
                key: "a".to_string(),
                text: "好的。".to_string(),
                tooltip: String::new(),
                effects: String::new(),
                effect_tooltip_effects: String::new(),
                hidden_effects: String::new(),
                trigger: String::new(),
                next_event: String::new(),
                next_event_type: String::new(),
                next_event_days: String::new(),
                next_event_hours: String::new(),
                next_event_random_days: String::new(),
                next_event_random_hours: String::new(),
                next_event_trigger_for: String::new(),
                next_event_scope: String::new(),
                next_events: Vec::new(),
                random_next_events: Vec::new(),
                ai_chance: String::new(),
            },
        );
    }
    options.into_values().collect()
}

pub(crate) fn event_option_next_event_entry(card: &Card, suffix: &str) -> EventOptionNextEvent {
    let sequence = trailing_ascii_digits(suffix).unwrap_or("");
    EventOptionNextEvent {
        field: if sequence.is_empty() {
            "next_event".to_string()
        } else {
            format!("next_event{sequence}")
        },
        target: card
            .fields
            .get(&format!("后续事件{suffix}"))
            .or_else(|| card.fields.get(&format!("下一事件{suffix}")))
            .or_else(|| card.fields.get(&format!("触发事件{suffix}")))
            .cloned()
            .unwrap_or_default(),
        event_type: card
            .fields
            .get(&format!("后续类型{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件类型{suffix}")))
            .or_else(|| card.fields.get(&format!("下一事件类型{suffix}")))
            .cloned()
            .unwrap_or_default(),
        days: card
            .fields
            .get(&format!("延迟{suffix}"))
            .or_else(|| card.fields.get(&format!("延迟天数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续天数{suffix}")))
            .or_else(|| card.fields.get(&format!("days{suffix}")))
            .or_else(|| card.fields.get(&format!("后续延迟{suffix}")))
            .cloned()
            .unwrap_or_default(),
        hours: card
            .fields
            .get(&format!("延迟小时{suffix}"))
            .or_else(|| card.fields.get(&format!("延迟小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续小时{suffix}")))
            .or_else(|| card.fields.get(&format!("后续小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("hours{suffix}")))
            .cloned()
            .unwrap_or_default(),
        random_days: card
            .fields
            .get(&format!("随机延迟{suffix}"))
            .or_else(|| card.fields.get(&format!("随机延迟天数{suffix}")))
            .or_else(|| card.fields.get(&format!("随机天数{suffix}")))
            .or_else(|| card.fields.get(&format!("后续随机天数{suffix}")))
            .or_else(|| card.fields.get(&format!("random_days{suffix}")))
            .cloned()
            .unwrap_or_default(),
        random_hours: card
            .fields
            .get(&format!("随机延迟小时{suffix}"))
            .or_else(|| card.fields.get(&format!("随机延迟小时数{suffix}")))
            .or_else(|| card.fields.get(&format!("随机小时{suffix}")))
            .or_else(|| card.fields.get(&format!("后续随机小时{suffix}")))
            .or_else(|| card.fields.get(&format!("random_hours{suffix}")))
            .cloned()
            .unwrap_or_default(),
        trigger_for: card
            .fields
            .get(&format!("后续触发对象{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件触发对象{suffix}")))
            .or_else(|| card.fields.get(&format!("触发对象{suffix}")))
            .or_else(|| card.fields.get(&format!("trigger_for{suffix}")))
            .or_else(|| card.fields.get(&format!("后续trigger_for{suffix}")))
            .cloned()
            .unwrap_or_default(),
        scope: card
            .fields
            .get(&format!("后续作用域{suffix}"))
            .or_else(|| card.fields.get(&format!("后续事件作用域{suffix}")))
            .or_else(|| card.fields.get(&format!("后续目标{suffix}")))
            .or_else(|| card.fields.get(&format!("后续范围{suffix}")))
            .or_else(|| card.fields.get(&format!("作用域{suffix}")))
            .or_else(|| card.fields.get(&format!("scope{suffix}")))
            .or_else(|| card.fields.get(&format!("target_scope{suffix}")))
            .or_else(|| card.fields.get(&format!("后续scope{suffix}")))
            .cloned()
            .unwrap_or_default(),
        weight: String::new(),
        condition: event_option_next_event_condition(card, suffix),
    }
}

pub(crate) fn event_option_random_next_event_entry(
    card: &Card,
    suffix: &str,
) -> EventOptionNextEvent {
    let sequence = trailing_ascii_digits(suffix).unwrap_or("");
    EventOptionNextEvent {
        field: if sequence.is_empty() {
            "random_next_event".to_string()
        } else {
            format!("random_next_event{sequence}")
        },
        target: card
            .fields
            .get(&format!("随机后续事件{suffix}"))
            .or_else(|| card.fields.get(&format!("随机下一事件{suffix}")))
            .or_else(|| card.fields.get(&format!("随机触发事件{suffix}")))
            .cloned()
            .unwrap_or_default(),
        event_type: card
            .fields
            .get(&format!("随机后续类型{suffix}"))
            .or_else(|| card.fields.get(&format!("随机后续事件类型{suffix}")))
            .cloned()
            .unwrap_or_default(),
        days: card
            .fields
            .get(&format!("随机后续延迟{suffix}"))
            .or_else(|| card.fields.get(&format!("随机后续天数{suffix}")))
            .cloned()
            .unwrap_or_default(),
        hours: card
            .fields
            .get(&format!("随机后续延迟小时{suffix}"))
            .or_else(|| card.fields.get(&format!("随机后续小时{suffix}")))
            .cloned()
            .unwrap_or_default(),
        random_days: card
            .fields
            .get(&format!("随机后续随机天数{suffix}"))
            .cloned()
            .unwrap_or_default(),
        random_hours: card
            .fields
            .get(&format!("随机后续随机小时{suffix}"))
            .cloned()
            .unwrap_or_default(),
        trigger_for: card
            .fields
            .get(&format!("随机后续触发对象{suffix}"))
            .or_else(|| card.fields.get(&format!("随机后续事件触发对象{suffix}")))
            .cloned()
            .unwrap_or_default(),
        scope: card
            .fields
            .get(&format!("随机后续作用域{suffix}"))
            .or_else(|| card.fields.get(&format!("随机后续事件作用域{suffix}")))
            .or_else(|| card.fields.get(&format!("随机后续目标{suffix}")))
            .cloned()
            .unwrap_or_default(),
        weight: card
            .fields
            .get(&format!("随机后续权重{suffix}"))
            .or_else(|| card.fields.get(&format!("随机权重{suffix}")))
            .or_else(|| card.fields.get(&format!("weight{suffix}")))
            .cloned()
            .unwrap_or_default(),
        condition: event_option_random_next_event_condition(card, suffix),
    }
}

pub(crate) fn event_option_next_event_condition(card: &Card, suffix: &str) -> String {
    card.fields
        .get(&format!("后续条件{suffix}"))
        .or_else(|| card.fields.get(&format!("后续事件条件{suffix}")))
        .or_else(|| card.fields.get(&format!("后续触发条件{suffix}")))
        .or_else(|| card.fields.get(&format!("后续limit{suffix}")))
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn event_option_random_next_event_condition(card: &Card, suffix: &str) -> String {
    card.fields
        .get(&format!("随机后续条件{suffix}"))
        .or_else(|| card.fields.get(&format!("随机后续事件条件{suffix}")))
        .or_else(|| card.fields.get(&format!("随机后续触发条件{suffix}")))
        .or_else(|| card.fields.get(&format!("随机后续limit{suffix}")))
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn event_option_next_events_json(entries: &[EventOptionNextEvent]) -> String {
    format!(
        "[{}]",
        entries
            .iter()
            .map(|entry| {
                format!(
                    "{{\"field\": {}, \"target\": {}, \"event_type\": {}, \"days\": {}, \"hours\": {}, \"random_days\": {}, \"random_hours\": {}, \"trigger_for\": {}, \"scope\": {}, \"weight\": {}, \"condition\": {}}}",
                    json_str(&entry.field),
                    json_str(&entry.target),
                    json_str(&entry.event_type),
                    json_str(&entry.days),
                    json_str(&entry.hours),
                    json_str(&entry.random_days),
                    json_str(&entry.random_hours),
                    json_str(&entry.trigger_for),
                    json_str(&entry.scope),
                    json_str(&entry.weight),
                    json_str(&entry.condition)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn numbered_next_event_suffixes(card: &Card, option_suffix: &str) -> Vec<String> {
    let mut suffixes = BTreeSet::new();
    for key in card.fields.keys() {
        if let Some((base, suffix)) = numbered_next_event_suffix_for_key(key) {
            if base == option_suffix {
                suffixes.insert(suffix);
            }
        }
    }
    suffixes.into_iter().collect()
}

pub(crate) fn random_next_event_suffixes(card: &Card, option_suffix: &str) -> Vec<String> {
    let mut suffixes = BTreeSet::new();
    for key in card.fields.keys() {
        if let Some((base, suffix)) = random_next_event_suffix_for_key(key) {
            if base == option_suffix {
                suffixes.insert(suffix);
            }
        }
    }
    suffixes.into_iter().collect()
}

pub(crate) fn numbered_next_event_suffix_for_key(key: &str) -> Option<(String, String)> {
    for prefix in numbered_next_event_field_prefixes() {
        if let Some(suffix) = key.strip_prefix(prefix) {
            let digits = trailing_ascii_digits(suffix)?;
            if digits.is_empty() || digits == "1" {
                continue;
            }
            let base = suffix.trim_end_matches(|ch: char| ch.is_ascii_digit());
            if !base.trim().is_empty() {
                return Some((base.to_string(), suffix.to_string()));
            }
        }
    }
    None
}

pub(crate) fn random_next_event_suffix_for_key(key: &str) -> Option<(String, String)> {
    for prefix in random_next_event_field_prefixes() {
        if let Some(suffix) = key.strip_prefix(prefix) {
            if suffix.trim().is_empty() {
                continue;
            }
            let digits = trailing_ascii_digits(suffix).unwrap_or("");
            let base = if digits.is_empty() {
                suffix
            } else {
                suffix.trim_end_matches(|ch: char| ch.is_ascii_digit())
            };
            if !base.trim().is_empty() {
                return Some((base.to_string(), suffix.to_string()));
            }
        }
    }
    None
}

pub(crate) fn trailing_ascii_digits(value: &str) -> Option<&str> {
    let first_digit = value
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(idx, _)| idx)?;
    Some(&value[first_digit..])
}

pub(crate) fn random_next_event_field_prefixes() -> &'static [&'static str] {
    &[
        "随机后续事件类型",
        "随机后续事件作用域",
        "随机后续事件触发对象",
        "随机后续事件条件",
        "随机后续随机小时",
        "随机后续随机天数",
        "随机后续延迟小时",
        "随机后续事件",
        "随机下一事件",
        "随机触发事件",
        "随机后续类型",
        "随机后续作用域",
        "随机后续触发对象",
        "随机后续触发条件",
        "随机后续条件",
        "随机后续目标",
        "随机后续权重",
        "随机后续limit",
        "随机后续延迟",
        "随机后续天数",
        "随机后续小时",
        "随机权重",
        "weight",
    ]
}

pub(crate) fn numbered_next_event_field_prefixes() -> &'static [&'static str] {
    &[
        "后续事件类型",
        "后续事件作用域",
        "后续事件触发对象",
        "后续事件条件",
        "后续触发条件",
        "下一事件类型",
        "随机延迟小时数",
        "随机延迟天数",
        "后续trigger_for",
        "后续随机天数",
        "后续随机小时",
        "延迟小时数",
        "后续小时数",
        "target_scope",
        "random_days",
        "random_hours",
        "后续作用域",
        "后续触发对象",
        "后续条件",
        "后续limit",
        "随机延迟小时",
        "随机延迟",
        "后续类型",
        "后续事件",
        "下一事件",
        "触发事件",
        "后续目标",
        "后续范围",
        "作用域",
        "scope",
        "后续scope",
        "触发对象",
        "trigger_for",
        "延迟小时",
        "延迟天数",
        "延迟",
        "后续延迟",
        "后续天数",
        "后续小时",
        "随机天数",
        "随机小时",
        "days",
        "hours",
    ]
}

pub(crate) fn event_option_field_suffix(key: &str) -> Option<&str> {
    for tail in [
        "效果提示代码",
        "效果预览",
        "预览效果",
        "effect_tooltip",
        "效果提示",
        "自定义提示",
        "工具提示",
        "提示",
        "tooltip",
    ] {
        if let Some(suffix) = key
            .strip_prefix("选项")
            .and_then(|rest| rest.strip_suffix(tail))
            .filter(|suffix| !suffix.trim().is_empty())
        {
            return Some(suffix);
        }
    }
    for tail in ["触发条件", "可见条件", "显示条件", "可选条件", "条件"] {
        if let Some(suffix) = key
            .strip_prefix("选项")
            .and_then(|rest| rest.strip_suffix(tail))
            .filter(|suffix| !suffix.trim().is_empty())
        {
            return Some(suffix);
        }
    }
    [
        "后续事件类型",
        "下一事件类型",
        "后续事件作用域",
        "后续作用域",
        "后续事件触发对象",
        "后续触发对象",
        "随机后续事件类型",
        "随机后续事件作用域",
        "随机后续事件触发对象",
        "随机后续事件条件",
        "随机后续随机小时",
        "随机后续随机天数",
        "随机后续延迟小时",
        "随机后续事件",
        "随机下一事件",
        "随机触发事件",
        "随机后续类型",
        "随机后续作用域",
        "随机后续触发对象",
        "随机后续触发条件",
        "随机后续条件",
        "随机后续目标",
        "随机后续权重",
        "随机后续limit",
        "随机后续延迟",
        "随机后续天数",
        "随机后续小时",
        "随机权重",
        "后续目标",
        "后续范围",
        "后续事件",
        "下一事件",
        "触发事件",
        "选项效果提示代码",
        "选项效果预览",
        "选项预览效果",
        "选项触发条件",
        "选项条件",
        "custom_effect_tooltip",
        "effect_tooltip",
        "效果提示代码",
        "效果提示",
        "效果预览",
        "预览效果",
        "自定义提示",
        "工具提示",
        "选项提示",
        "提示",
        "tooltip",
        "隐藏效果",
        "延迟天数",
        "延迟小时数",
        "延迟小时",
        "随机延迟小时数",
        "随机延迟小时",
        "随机延迟天数",
        "随机延迟",
        "随机天数",
        "随机小时",
        "后续延迟",
        "后续天数",
        "后续小时数",
        "后续小时",
        "后续随机天数",
        "后续随机小时",
        "后续类型",
        "触发对象",
        "后续scope",
        "target_scope",
        "scope",
        "作用域",
        "后续trigger_for",
        "trigger_for",
        "random_days",
        "random_hours",
        "days",
        "hours",
        "可见条件",
        "显示条件",
        "可选条件",
        "触发条件",
        "条件",
        "AI权重",
        "ai权重",
        "选项",
        "效果",
        "延迟",
        "ai",
    ]
    .iter()
    .find_map(|prefix| {
        key.strip_prefix(prefix)
            .filter(|suffix| !suffix.trim().is_empty())
    })
}

pub(crate) fn event_option_trigger_text<'a>(card: &'a Card, suffix: &str) -> Option<&'a str> {
    [
        format!("选项{suffix}触发条件"),
        format!("选项{suffix}可见条件"),
        format!("选项{suffix}显示条件"),
        format!("选项{suffix}可选条件"),
        format!("选项{suffix}条件"),
        format!("选项触发条件{suffix}"),
        format!("选项条件{suffix}"),
        format!("触发条件{suffix}"),
        format!("可见条件{suffix}"),
        format!("显示条件{suffix}"),
        format!("可选条件{suffix}"),
        format!("条件{suffix}"),
    ]
    .iter()
    .find_map(|key| card.fields.get(key).map(String::as_str))
}

pub(crate) fn event_option_effect_tooltip_text<'a>(
    card: &'a Card,
    suffix: &str,
) -> Option<&'a str> {
    [
        format!("选项{suffix}效果提示代码"),
        format!("选项{suffix}效果预览"),
        format!("选项{suffix}预览效果"),
        format!("效果提示代码{suffix}"),
        format!("效果预览{suffix}"),
        format!("预览效果{suffix}"),
        format!("effect_tooltip{suffix}"),
    ]
    .iter()
    .find_map(|key| card.fields.get(key).map(String::as_str))
}

pub(crate) fn event_option_tooltip_text<'a>(card: &'a Card, suffix: &str) -> Option<&'a str> {
    [
        format!("选项{suffix}效果提示"),
        format!("选项{suffix}自定义提示"),
        format!("选项{suffix}工具提示"),
        format!("选项{suffix}提示"),
        format!("效果提示{suffix}"),
        format!("自定义提示{suffix}"),
        format!("工具提示{suffix}"),
        format!("选项提示{suffix}"),
        format!("提示{suffix}"),
        format!("tooltip{suffix}"),
        format!("custom_effect_tooltip{suffix}"),
    ]
    .iter()
    .find_map(|key| card.fields.get(key).map(String::as_str))
}

pub(crate) fn event_trigger_text(card: &Card) -> Option<&str> {
    [
        "触发",
        "触发条件",
        "触发器",
        "条件",
        "可触发条件",
        "发生条件",
    ]
    .iter()
    .find_map(|key| card.fields.get(*key).map(String::as_str))
}

pub(crate) fn event_immediate_effect_text(card: &Card) -> Option<&str> {
    [
        "立即效果",
        "即时效果",
        "事件立即效果",
        "立即执行",
        "immediate",
        "立即",
    ]
    .iter()
    .find_map(|key| card.fields.get(*key).map(String::as_str))
}

pub(crate) fn event_after_effect_text(card: &Card) -> Option<&str> {
    [
        "收尾效果",
        "事件后效果",
        "结束后效果",
        "事件结束后效果",
        "after",
        "after_effect",
        "after_effects",
    ]
    .iter()
    .find_map(|key| card.fields.get(*key).map(String::as_str))
}

pub(crate) fn event_after_hidden_effect_text(card: &Card) -> Option<&str> {
    [
        "收尾隐藏效果",
        "事件后隐藏效果",
        "结束后隐藏效果",
        "事件结束后隐藏效果",
        "after_hidden_effect",
        "after hidden_effect",
    ]
    .iter()
    .find_map(|key| card.fields.get(*key).map(String::as_str))
}

pub(crate) fn event_bool_field_value<'a>(card: &'a Card, aliases: &[&str]) -> Option<&'a str> {
    aliases
        .iter()
        .find_map(|key| card.fields.get(*key).map(String::as_str))
}

pub(crate) fn event_bool_code(value: &str) -> &'static str {
    let normalized = value.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "no" | "false" | "0" | "否" | "不" | "关闭" | "关" | "禁用"
    ) {
        "no"
    } else {
        "yes"
    }
}

pub(crate) fn event_mean_time_to_happen_days(card: &Card) -> Option<i64> {
    [
        "平均发生时间",
        "平均触发时间",
        "发生时间",
        "MTTH",
        "mtth",
        "mean_time_to_happen",
    ]
    .iter()
    .filter_map(|key| card.fields.get(*key))
    .find_map(|value| parse_int(value).filter(|days| *days > 0))
}

pub(crate) fn event_timeout_days(card: &Card) -> Option<i64> {
    [
        "超时",
        "超时天数",
        "限时",
        "限时天数",
        "倒计时",
        "倒计时天数",
        "timeout",
        "timeout_days",
    ]
    .iter()
    .filter_map(|key| card.fields.get(*key))
    .find_map(|value| parse_int(value).filter(|days| *days > 0))
}

pub(crate) fn insert_event_localisation(
    card: &Card,
    event_id: &str,
    loc_entries: &mut BTreeMap<String, String>,
) {
    insert_event_localisation_with_index(card, event_id, loc_entries, None)
        .expect("event localisation without game index does not fail");
}

pub(crate) fn insert_event_localisation_with_index(
    card: &Card,
    event_id: &str,
    loc_entries: &mut BTreeMap<String, String>,
    game_index: Option<&GameIndex>,
) -> Result<(), String> {
    loc_entries.insert(
        format!("{event_id}.t"),
        compile_event_localisation_text(
            card.fields
                .get("标题")
                .cloned()
                .unwrap_or_else(|| card.title.clone()),
            game_index,
        )?,
    );
    loc_entries.insert(
        format!("{event_id}.d"),
        compile_event_localisation_text(
            event_description_text(card).unwrap_or_else(|| format!("{}。", card.title)),
            game_index,
        )?,
    );
    for option in event_options(card) {
        loc_entries.insert(
            format!("{event_id}.{}", option.key),
            compile_event_localisation_text(option.text, game_index)?,
        );
        if !option.tooltip.trim().is_empty() {
            loc_entries.insert(
                format!("{event_id}.{}.tt", option.key),
                compile_event_localisation_text(option.tooltip, game_index)?,
            );
        }
    }
    Ok(())
}

pub(crate) fn compile_event_localisation_text(
    value: String,
    game_index: Option<&GameIndex>,
) -> Result<String, String> {
    if let Some(game_index) = game_index {
        compile_author_localisation_placeholders_with_index(&value, game_index)
    } else {
        Ok(compile_author_localisation_placeholders(&value))
    }
}

pub(crate) fn event_description_text(card: &Card) -> Option<String> {
    ["描述", "事件描述", "事件文案", "文案", "正文"]
        .iter()
        .find_map(|key| card.fields.get(*key).cloned())
}

pub(crate) fn append_event_blocks(
    path: &Path,
    namespaces: &BTreeSet<String>,
    blocks: &[EventBlockWrite],
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
    for block in blocks {
        if replace_generated_event_block(
            &mut text,
            &block.source_key,
            &block.event_id,
            &block.block,
        ) {
            changed = true;
            continue;
        }
        if event_id_exists_in_text(&text, &block.event_id) {
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(&block.block);
        changed = true;
    }
    if changed || !path.exists() {
        fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn replace_generated_event_block(
    text: &mut String,
    source_key: &str,
    event_id: &str,
    block: &str,
) -> bool {
    let Some((replace_start, block_start, block_end)) =
        generated_event_block_span(text, source_key)
    else {
        return false;
    };
    let old_block = &text[block_start..block_end];
    if find_assignment_in_text(old_block, "id") != Some(event_id) {
        return false;
    }
    let replacement = if block.ends_with('\n') {
        block.to_string()
    } else {
        format!("{block}\n")
    };
    if text[replace_start..block_end] == replacement {
        return false;
    }
    text.replace_range(replace_start..block_end, &replacement);
    true
}

pub(crate) fn generated_event_block_span(
    text: &str,
    source_key: &str,
) -> Option<(usize, usize, usize)> {
    let marker = format!("# hoi4skill_source = {source_key}");
    let lines = text_line_ranges(text);
    let source_idx = lines
        .iter()
        .position(|(_, _, line)| line.trim() == marker)?;
    let replace_start = if source_idx > 0
        && lines[source_idx - 1]
            .2
            .trim()
            .starts_with("# hoi4skill_card = ")
    {
        lines[source_idx - 1].0
    } else {
        lines[source_idx].0
    };
    let block_start = lines
        .iter()
        .skip(source_idx + 1)
        .find_map(|(start, _, line)| {
            let trimmed = line.trim_start();
            if ["country_event", "news_event", "state_event"]
                .iter()
                .any(|kind| trimmed.starts_with(kind))
                && trimmed.contains('=')
            {
                Some(*start)
            } else {
                None
            }
        })?;
    let block_end = clausewitz_block_end(text, block_start)?;
    Some((replace_start, block_start, block_end))
}

pub(crate) fn text_line_ranges(text: &str) -> Vec<(usize, usize, String)> {
    let mut ranges = Vec::new();
    let mut start = 0usize;
    for line in text.split_inclusive('\n') {
        let end = start + line.len();
        ranges.push((start, end, line.to_string()));
        start = end;
    }
    if start < text.len() {
        ranges.push((start, text.len(), text[start..].to_string()));
    }
    ranges
}

pub(crate) fn clausewitz_block_end(text: &str, start: usize) -> Option<usize> {
    let open = text[start..].find('{')? + start;
    let mut depth = 1i32;
    let mut in_quote = false;
    let mut escape = false;
    for (offset, ch) in text[open + 1..].char_indices() {
        if ch == '"' && !escape {
            in_quote = !in_quote;
        }
        if in_quote {
            escape = ch == '\\' && !escape;
            if ch != '\\' {
                escape = false;
            }
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                let mut end = open + 1 + offset + ch.len_utf8();
                if text[end..].starts_with("\r\n") {
                    end += 2;
                } else if text[end..].starts_with('\n') {
                    end += 1;
                }
                return Some(end);
            }
        }
        escape = false;
    }
    None
}

pub(crate) fn event_id_exists_in_text(text: &str, event_id: &str) -> bool {
    event_blocks(text)
        .into_iter()
        .any(|block| block_assignment(&block, "id").as_deref() == Some(event_id))
}

pub(crate) fn upsert_event_localisation_entries(
    path: &Path,
    entries: &BTreeMap<String, String>,
) -> Result<bool, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = if path.exists() {
        let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from("l_simp_chinese:\n")
    };
    if !text
        .lines()
        .any(|line| line.trim_start_matches('\u{feff}').trim() == "l_simp_chinese:")
    {
        text = format!("l_simp_chinese:\n{text}");
    }

    let mut changed = false;
    for (key, value) in entries {
        let line = format!("  {key}:0 \"{}\"\n", localisation_value(value));
        if let Some(replaced) = replace_event_localisation_line(&mut text, key, &line) {
            if replaced {
                changed = true;
            }
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&line);
        changed = true;
    }
    if changed || !path.exists() {
        fs::write(path, format!("\u{feff}{text}"))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn replace_event_localisation_line(
    text: &mut String,
    key: &str,
    line: &str,
) -> Option<bool> {
    let prefix = format!("{key}:");
    for (start, end, existing) in text_line_ranges(text) {
        let trimmed = existing.trim_start_matches('\u{feff}').trim_start();
        if !trimmed.starts_with(&prefix) {
            continue;
        }
        if existing == line {
            return Some(false);
        }
        text.replace_range(start..end, line);
        return Some(true);
    }
    None
}

pub(crate) fn parse_event_cards_json(text: &str, tag: &str, prefix: &str) -> String {
    parse_event_cards_json_with_chain(text, tag, prefix, None, None)
}

pub(crate) fn event_trigger_report_json(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    tag: &str,
    prefix: &str,
) -> String {
    let events = cards
        .iter()
        .zip(planned_ids.iter())
        .map(|(card, planned)| {
            let namespace = event_card_namespace(card, prefix);
            format!(
                "{{\"title\": {}, \"event_id\": {}, \"namespace\": {}, \"source_key\": {}, \"event_type\": {}, \"target\": {}, \"trigger\": {}, \"options\": {}}}",
                json_str(&card.title),
                json_str(&planned.event_id),
                json_str(&planned.namespace),
                json_str(&event_card_source_key(card, &namespace)),
                json_str(normalize_event_type(card.fields.get("类型").map(String::as_str))),
                json_str(card.fields.get("目标").map(String::as_str).unwrap_or(tag)),
                json_optional_str(event_trigger_text(card)),
                event_trigger_report_options_json(&event_options(card))
            )
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.event_trigger_report.v1\",\n  \"tag\": {},\n  \"prefix\": {},\n  \"event_count\": {},\n  \"events\": [\n    {}\n  ]\n}}\n",
        json_str(tag),
        json_str(prefix),
        cards.len(),
        events
    )
}

pub(crate) fn event_trigger_report_options_json(options: &[EventOption]) -> String {
    format!(
        "[{}]",
        options
            .iter()
            .map(|option| {
                format!(
                    "{{\"key\": {}, \"text\": {}, \"trigger\": {}, \"next_events\": {}, \"random_next_events\": {}}}",
                    json_str(&option.key),
                    json_str(&option.text),
                    json_str(&option.trigger),
                    event_option_next_events_json(&option.next_events),
                    event_option_next_events_json(&option.random_next_events)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn parse_event_cards_json_with_chain(
    text: &str,
    tag: &str,
    prefix: &str,
    external_chain_index: Option<&EventChainIndex>,
    external_planned_ids: Option<&[PlannedEventId]>,
) -> String {
    let cards = parse_cards(text, &["事件"]);
    let parse_blockers = event_card_parse_blockers(text, &cards);
    let fallback_planned_ids;
    let planned_ids = if let Some(planned_ids) = external_planned_ids {
        planned_ids
    } else {
        fallback_planned_ids = plan_event_card_ids(
            &cards,
            prefix,
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
        )
        .unwrap_or_default();
        &fallback_planned_ids
    };
    let fallback_chain_index;
    let chain_index = if let Some(chain_index) = external_chain_index {
        chain_index
    } else {
        fallback_chain_index = build_event_chain_index(&cards, &planned_ids);
        &fallback_chain_index
    };
    let chain_cycles = event_chain_cycles(&cards, planned_ids, &chain_index);
    let unsafe_chain_cycles = event_chain_unsafe_cycles(&cards, planned_ids, &chain_index);
    let chain_summary = event_chain_graph_summary(&cards, planned_ids, &chain_index);
    let chain_review_items = event_chain_review_items(
        &cards,
        planned_ids,
        &chain_index,
        &chain_summary,
        &unsafe_chain_cycles,
    );
    let chain_blocking_review_count = chain_review_items
        .iter()
        .filter(|item| item.severity == "error")
        .count();
    let safety_suggestions = event_cards_safety_suggestions_with_chain(&cards, Some(&chain_index));
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.event_cards.v1\",\n");
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!(
        "  \"safety\": {},\n",
        suggestions_safety_json_with_extra_blockers(&safety_suggestions, &parse_blockers)
    ));
    out.push_str(&format!(
        "  \"event_chain\": {},\n",
        event_chain_edges_json(&cards, &planned_ids, &chain_index)
    ));
    out.push_str(&format!(
        "  \"event_chain_edge_count\": {},\n",
        chain_summary.edge_count
    ));
    out.push_str(&format!(
        "  \"event_chain_resolved_edge_count\": {},\n",
        chain_summary.resolved_edge_count
    ));
    out.push_str(&format!(
        "  \"event_chain_unresolved_edge_count\": {},\n",
        chain_summary.unresolved_edge_count
    ));
    out.push_str(&format!(
        "  \"event_chain_ambiguous_edge_count\": {},\n",
        chain_summary.ambiguous_edge_count
    ));
    out.push_str(&format!(
        "  \"event_chain_entry_count\": {},\n",
        chain_summary.entry_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_entry_events\": {},\n",
        json_array(&chain_summary.entry_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_terminal_count\": {},\n",
        chain_summary.terminal_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_terminal_events\": {},\n",
        json_array(&chain_summary.terminal_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_isolated_count\": {},\n",
        chain_summary.isolated_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_isolated_events\": {},\n",
        json_array(&chain_summary.isolated_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_layer_count\": {},\n",
        chain_summary.layers.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_layers\": {},\n",
        event_chain_layers_json(&chain_summary.layers)
    ));
    out.push_str(&format!(
        "  \"event_chain_unlayered_count\": {},\n",
        chain_summary.unlayered_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_unlayered_events\": {},\n",
        json_array(&chain_summary.unlayered_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_branching_count\": {},\n",
        chain_summary.branching_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_branching_events\": {},\n",
        json_array(&chain_summary.branching_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_merging_count\": {},\n",
        chain_summary.merging_events.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_merging_events\": {},\n",
        json_array(&chain_summary.merging_events)
    ));
    out.push_str(&format!(
        "  \"event_chain_review_count\": {},\n",
        chain_review_items.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_blocking_review_count\": {},\n",
        chain_blocking_review_count
    ));
    out.push_str(&format!(
        "  \"event_chain_review_items\": {},\n",
        event_chain_review_items_json(&chain_review_items)
    ));
    out.push_str(&format!(
        "  \"event_chain_cycle_count\": {},\n",
        chain_cycles.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_cycles\": {},\n",
        event_chain_cycles_json(&chain_cycles)
    ));
    out.push_str(&format!(
        "  \"event_chain_unsafe_cycle_count\": {},\n",
        unsafe_chain_cycles.len()
    ));
    out.push_str(&format!(
        "  \"event_chain_unsafe_cycles\": {},\n",
        event_chain_cycles_json(&unsafe_chain_cycles)
    ));
    out.push_str(&format!(
        "  \"duplicate_chain_titles\": {},\n",
        json_array(
            &chain_index
                .duplicate_titles
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        )
    ));
    out.push_str("  \"features\": [\n");
    for (idx, (card, planned)) in cards.iter().zip(planned_ids.iter()).enumerate() {
        comma(&mut out, idx, "    ");
        let ns = &planned.namespace;
        let event_id = &planned.event_id;
        let source_label = event_card_explicit_source_label(card).unwrap_or_default();
        let source_key = event_card_source_key(card, ns);
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
        if let Some(days) = event_timeout_days(card) {
            suggestions.push(Suggestion::new(
                "event_timeout",
                &format!("timeout_days = {days}"),
                &days.to_string(),
                "Verified HOI4 event field from local game events.",
            ));
        }
        if let Some(trigger) = event_trigger_text(card) {
            suggestions.extend(suggest_trigger(trigger));
        }
        if let Some(immediate) = event_immediate_effect_text(card) {
            suggestions.extend(event_option_effect_suggestions(
                immediate,
                Some(&chain_index),
            ));
        }
        if let Some(after) = event_after_effect_text(card) {
            suggestions.extend(event_option_effect_suggestions(after, Some(&chain_index)));
        }
        if let Some(after_hidden) = event_after_hidden_effect_text(card) {
            suggestions.extend(event_option_effect_suggestions(
                after_hidden,
                Some(&chain_index),
            ));
        }

        let mut options = BTreeMap::new();
        for option in event_options(card) {
            let mut option_suggestions = if option.trigger.trim().is_empty() {
                Vec::new()
            } else {
                split_cn_list(&option.trigger)
                    .into_iter()
                    .flat_map(suggest_trigger)
                    .collect::<Vec<_>>()
            };
            option_suggestions.extend(event_option_effect_suggestions(
                &option.effects,
                Some(&chain_index),
            ));
            option_suggestions.extend(event_option_effect_suggestions(
                &option.effect_tooltip_effects,
                Some(&chain_index),
            ));
            option_suggestions.extend(event_option_effect_suggestions(
                &option.hidden_effects,
                Some(&chain_index),
            ));
            option_suggestions.extend(event_option_next_event_suggestions(
                &option,
                Some(&chain_index),
            ));
            options.insert(
                option.key.clone(),
                format!(
                    "{{\"key\": {}, \"text\": {}, \"tooltip\": {}, \"trigger\": {}, \"effects\": {}, \"effect_tooltip_effects\": {}, \"hidden_effects\": {}, \"next_event\": {}, \"next_event_type\": {}, \"next_event_days\": {}, \"next_event_hours\": {}, \"next_event_random_days\": {}, \"next_event_random_hours\": {}, \"next_event_trigger_for\": {}, \"next_event_scope\": {}, \"next_events\": {}, \"random_next_events\": {}, \"ai_chance\": {}, \"suggestions\": {}, \"safety\": {}}}",
                    json_str(&option.key),
                    json_str(&option.text),
                    json_str(&option.tooltip),
                    json_str(&option.trigger),
                    json_str(&option.effects),
                    json_str(&option.effect_tooltip_effects),
                    json_str(&option.hidden_effects),
                    json_str(&option.next_event),
                    json_str(&option.next_event_type),
                    json_str(&option.next_event_days),
                    json_str(&option.next_event_hours),
                    json_str(&option.next_event_random_days),
                    json_str(&option.next_event_random_hours),
                    json_str(&option.next_event_trigger_for),
                    json_str(&option.next_event_scope),
                    event_option_next_events_json(&option.next_events),
                    event_option_next_events_json(&option.random_next_events),
                    json_str(&option.ai_chance),
                    suggestions_json(&option_suggestions),
                    suggestions_safety_json(&option_suggestions)
                ),
            );
        }

        let loc_keys: Vec<String> = std::iter::once(format!("{event_id}.t"))
            .chain(std::iter::once(format!("{event_id}.d")))
            .chain(event_options(card).into_iter().flat_map(|option| {
                let mut keys = vec![format!("{event_id}.{}", option.key)];
                if !option.tooltip.trim().is_empty() {
                    keys.push(format!("{event_id}.{}.tt", option.key));
                }
                keys
            }))
            .collect();
        let files = vec![
            format!("events/{prefix}_events.txt"),
            target_localisation_relative_path(target),
        ];
        out.push_str(&format!(
            "{{\"type\": \"event\", \"title\": {}, \"source_label\": {}, \"source_key\": {}, \"target\": {}, \"namespace\": {}, \"event_type\": {}, \"event_id\": {}, \"fields\": {}, \"options\": {}, \"files\": {}, \"localisation_keys\": {}, \"suggestions\": {}, \"safety\": {}}}",
            json_str(&card.title),
            json_str(&source_label),
            json_str(&source_key),
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

pub(crate) fn event_card_parse_blockers(text: &str, cards: &[Card]) -> Vec<String> {
    let mut blockers = duplicate_event_source_label_errors(cards);
    if !cards.is_empty() || text.trim().is_empty() {
        return blockers;
    }
    let has_event_like_fields = text.lines().any(|line| {
        let Some((key, _)) = split_field(line.trim()) else {
            return false;
        };
        is_event_like_card_field_key(key)
    });
    if has_event_like_fields {
        blockers.push(
            "no event cards were parsed; each event card must start with `事件：<标题>` before fields like `标题：`, `选项A：`, or `后续事件A：`".to_string(),
        );
    } else {
        return blockers;
    }
    blockers
}

pub(crate) fn duplicate_event_source_label_errors(cards: &[Card]) -> Vec<String> {
    let mut first_seen: BTreeMap<String, String> = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    let mut errors = Vec::new();
    for card in cards {
        let Some(label) = event_card_explicit_source_label(card) else {
            continue;
        };
        let prior = first_seen.insert(label.clone(), card.title.clone());
        if let Some(first_title) = prior {
            if duplicates.insert(label.clone()) {
                errors.push(format!(
                    "duplicate event key `{label}` used by events `{first_title}` and `{}`; each `事件键` must be unique in one event-card batch",
                    card.title
                ));
            }
        }
    }
    errors
}

pub(crate) fn is_event_like_card_field_key(key: &str) -> bool {
    matches!(
        key,
        "标题"
            | "题目"
            | "描述"
            | "事件描述"
            | "事件文案"
            | "文案"
            | "正文"
            | "目标"
            | "命名空间"
            | "类型"
            | "触发"
            | "触发条件"
            | "触发器"
            | "条件"
            | "可触发条件"
            | "发生条件"
            | "图片"
            | "立即效果"
            | "即时效果"
            | "事件立即效果"
            | "立即执行"
            | "immediate"
            | "立即"
            | "收尾效果"
            | "事件后效果"
            | "结束后效果"
            | "事件结束后效果"
            | "after"
            | "after_effect"
            | "after_effects"
            | "收尾隐藏效果"
            | "事件后隐藏效果"
            | "结束后隐藏效果"
            | "事件结束后隐藏效果"
            | "after_hidden_effect"
            | "after hidden_effect"
            | "平均发生时间"
            | "平均触发时间"
            | "发生时间"
            | "MTTH"
            | "mtth"
            | "mean_time_to_happen"
            | "超时"
            | "超时天数"
            | "限时"
            | "限时天数"
            | "倒计时"
            | "倒计时天数"
            | "timeout"
            | "timeout_days"
            | "只触发一次"
            | "仅触发一次"
            | "一次性"
            | "fire_only_once"
            | "主要事件"
            | "重大事件"
            | "major"
            | "显示主要"
            | "显示为主要事件"
            | "show_major"
            | "隐藏"
            | "隐藏事件"
            | "hidden"
    ) || [
        "选项",
        "选项提示",
        "效果提示",
        "效果提示代码",
        "效果预览",
        "预览效果",
        "自定义提示",
        "工具提示",
        "提示",
        "tooltip",
        "custom_effect_tooltip",
        "effect_tooltip",
        "效果",
        "隐藏效果",
        "后续事件",
        "下一事件",
        "触发事件",
        "后续类型",
        "后续事件类型",
        "下一事件类型",
        "随机后续事件",
        "随机下一事件",
        "随机触发事件",
        "随机后续类型",
        "随机后续事件类型",
        "随机后续作用域",
        "随机后续事件作用域",
        "随机后续目标",
        "随机后续权重",
        "随机权重",
        "随机后续触发对象",
        "随机后续事件触发对象",
        "随机后续延迟",
        "随机后续延迟小时",
        "随机后续天数",
        "随机后续小时",
        "随机后续随机天数",
        "随机后续随机小时",
        "weight",
        "后续作用域",
        "后续事件作用域",
        "后续事件条件",
        "后续目标",
        "后续触发条件",
        "后续条件",
        "后续limit",
        "后续范围",
        "作用域",
        "scope",
        "target_scope",
        "后续scope",
        "后续触发对象",
        "后续事件触发对象",
        "触发对象",
        "trigger_for",
        "后续trigger_for",
        "后续天数",
        "后续小时",
        "后续小时数",
        "后续随机天数",
        "后续随机小时",
        "随机延迟",
        "随机延迟天数",
        "随机延迟小时",
        "随机延迟小时数",
        "随机天数",
        "随机小时",
        "days",
        "hours",
        "random_days",
        "random_hours",
        "延迟",
        "延迟天数",
        "延迟小时",
        "延迟小时数",
        "后续延迟",
        "AI权重",
        "ai权重",
        "ai",
    ]
    .iter()
    .any(|prefix| {
        key.strip_prefix(prefix)
            .is_some_and(|suffix| !suffix.trim().is_empty())
    })
}

pub(crate) fn event_card_command_parse_errors(text: &str, cards: &[Card]) -> Vec<String> {
    let mut errors = event_card_parse_blockers(text, cards);
    if cards.is_empty() && errors.is_empty() {
        if text.trim().is_empty() {
            errors.push("no event cards were parsed; input is empty".to_string());
        } else {
            errors.push(
                "no event cards were parsed; input must contain at least one `事件：<标题>` card"
                    .to_string(),
            );
        }
    }
    errors
}

pub(crate) fn event_chain_cycles(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> Vec<Vec<String>> {
    let adjacency = event_chain_resolved_adjacency(cards, planned_ids, chain_index);
    find_event_chain_cycles(&adjacency)
}

pub(crate) fn event_chain_unsafe_cycles(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> Vec<Vec<String>> {
    let edges = event_chain_resolved_edges(cards, planned_ids, chain_index);
    find_event_chain_unsafe_cycles(&edges)
}

#[derive(Default)]
pub(crate) struct EventChainGraphSummary {
    pub(crate) edge_count: usize,
    pub(crate) resolved_edge_count: usize,
    pub(crate) unresolved_edge_count: usize,
    pub(crate) ambiguous_edge_count: usize,
    pub(crate) entry_events: Vec<String>,
    pub(crate) terminal_events: Vec<String>,
    pub(crate) isolated_events: Vec<String>,
    pub(crate) layers: Vec<Vec<String>>,
    pub(crate) unlayered_events: Vec<String>,
    pub(crate) branching_events: Vec<String>,
    pub(crate) merging_events: Vec<String>,
}

pub(crate) fn event_chain_graph_summary(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> EventChainGraphSummary {
    let adjacency = event_chain_resolved_adjacency(cards, planned_ids, chain_index);
    let mut summary = event_chain_graph_summary_from_adjacency(planned_ids, &adjacency);
    let edge_counts = event_chain_edge_status_counts(cards, chain_index);
    summary.edge_count = edge_counts.edge_count;
    summary.resolved_edge_count = edge_counts.resolved_edge_count;
    summary.unresolved_edge_count = edge_counts.unresolved_edge_count;
    summary.ambiguous_edge_count = edge_counts.ambiguous_edge_count;
    summary
}

pub(crate) fn event_chain_graph_summary_from_adjacency(
    planned_ids: &[PlannedEventId],
    adjacency: &BTreeMap<String, Vec<String>>,
) -> EventChainGraphSummary {
    let planned = planned_ids
        .iter()
        .map(|planned| planned.event_id.clone())
        .collect::<BTreeSet<_>>();
    let mut incoming = BTreeSet::new();
    let mut incoming_counts: BTreeMap<String, usize> = BTreeMap::new();
    for targets in adjacency.values() {
        for target in targets {
            if planned.contains(target) {
                incoming.insert(target.clone());
                *incoming_counts.entry(target.clone()).or_default() += 1;
            }
        }
    }
    let mut summary = EventChainGraphSummary::default();
    for planned_id in planned_ids {
        let event_id = &planned_id.event_id;
        let has_incoming = incoming.contains(event_id);
        let has_outgoing = adjacency
            .get(event_id)
            .is_some_and(|targets| !targets.is_empty());
        if !has_incoming {
            summary.entry_events.push(event_id.clone());
        }
        if !has_outgoing {
            summary.terminal_events.push(event_id.clone());
        }
        if !has_incoming && !has_outgoing {
            summary.isolated_events.push(event_id.clone());
        }
        if adjacency
            .get(event_id)
            .is_some_and(|targets| targets.len() > 1)
        {
            summary.branching_events.push(event_id.clone());
        }
        if incoming_counts.get(event_id).copied().unwrap_or(0) > 1 {
            summary.merging_events.push(event_id.clone());
        }
    }
    let cycle_nodes = event_chain_cycle_nodes(adjacency);
    let (layers, unlayered_events) = event_chain_layers_from_adjacency(
        planned_ids,
        adjacency,
        &summary.entry_events,
        &cycle_nodes,
    );
    summary.layers = layers;
    summary.unlayered_events = unlayered_events;
    summary
}

pub(crate) fn event_chain_cycle_nodes(
    adjacency: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    find_event_chain_cycles(adjacency)
        .into_iter()
        .flatten()
        .collect()
}

pub(crate) fn event_chain_layers_from_adjacency(
    planned_ids: &[PlannedEventId],
    adjacency: &BTreeMap<String, Vec<String>>,
    entry_events: &[String],
    excluded_events: &BTreeSet<String>,
) -> (Vec<Vec<String>>, Vec<String>) {
    let planned = planned_ids
        .iter()
        .map(|planned| planned.event_id.clone())
        .collect::<BTreeSet<_>>();
    let mut depths: BTreeMap<String, usize> = BTreeMap::new();
    let mut queue = VecDeque::new();
    for event_id in entry_events {
        if planned.contains(event_id)
            && !excluded_events.contains(event_id)
            && depths.insert(event_id.clone(), 0).is_none()
        {
            queue.push_back(event_id.clone());
        }
    }
    let max_steps = planned.len().saturating_mul(planned.len().max(1));
    let mut steps = 0usize;
    while let Some(source) = queue.pop_front() {
        steps += 1;
        if steps > max_steps {
            break;
        }
        let Some(source_depth) = depths.get(&source).copied() else {
            continue;
        };
        for target in adjacency.get(&source).into_iter().flatten() {
            if !planned.contains(target) || excluded_events.contains(target) {
                continue;
            }
            let target_depth = source_depth + 1;
            let should_update = depths
                .get(target)
                .is_none_or(|existing_depth| target_depth > *existing_depth);
            if should_update {
                depths.insert(target.clone(), target_depth);
                queue.push_back(target.clone());
            }
        }
    }
    let max_depth = depths.values().copied().max().unwrap_or(0);
    let mut layers = vec![Vec::new(); max_depth + 1];
    for planned_id in planned_ids {
        if let Some(depth) = depths.get(&planned_id.event_id).copied() {
            layers[depth].push(planned_id.event_id.clone());
        }
    }
    layers.retain(|layer| !layer.is_empty());
    let unlayered_events = planned_ids
        .iter()
        .filter(|planned| {
            !depths.contains_key(&planned.event_id) || excluded_events.contains(&planned.event_id)
        })
        .map(|planned| planned.event_id.clone())
        .collect::<Vec<_>>();
    (layers, unlayered_events)
}

#[derive(Default)]
pub(crate) struct EventChainEdgeStatusCounts {
    pub(crate) edge_count: usize,
    pub(crate) resolved_edge_count: usize,
    pub(crate) unresolved_edge_count: usize,
    pub(crate) ambiguous_edge_count: usize,
}

pub(crate) fn event_chain_edge_status_counts(
    cards: &[Card],
    chain_index: &EventChainIndex,
) -> EventChainEdgeStatusCounts {
    let mut counts = EventChainEdgeStatusCounts::default();
    for card in cards {
        for option in event_options(card) {
            collect_event_chain_edge_status_counts_from_text(
                &mut counts,
                &option.effects,
                chain_index,
            );
            collect_event_chain_edge_status_counts_from_text(
                &mut counts,
                &option.hidden_effects,
                chain_index,
            );
            for entry in option
                .next_events
                .iter()
                .chain(option.random_next_events.iter())
            {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_event_chain_edge_status_counts_from_text(
                    &mut counts,
                    &next_event,
                    chain_index,
                );
            }
        }
    }
    counts
}

pub(crate) fn collect_event_chain_edge_status_counts_from_text(
    counts: &mut EventChainEdgeStatusCounts,
    text: &str,
    chain_index: &EventChainIndex,
) {
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        let (status, _, _) = event_chain_edge_status(&reference, chain_index);
        counts.edge_count += 1;
        match status {
            "resolved" => counts.resolved_edge_count += 1,
            "ambiguous" => counts.ambiguous_edge_count += 1,
            "unresolved" => counts.unresolved_edge_count += 1,
            _ => {}
        }
    }
}

pub(crate) struct EventChainReviewItem {
    pub(crate) kind: &'static str,
    pub(crate) severity: &'static str,
    pub(crate) source_event_id: Option<String>,
    pub(crate) source_event: Option<String>,
    pub(crate) option: Option<String>,
    pub(crate) field: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) message: String,
    pub(crate) fix: String,
}

pub(crate) fn event_chain_review_items(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
    summary: &EventChainGraphSummary,
    unsafe_cycles: &[Vec<String>],
) -> Vec<EventChainReviewItem> {
    let mut items = Vec::new();
    collect_event_chain_edge_review_items(&mut items, cards, planned_ids, chain_index);
    for event_id in &summary.isolated_events {
        items.push(EventChainReviewItem {
            kind: "isolated_event",
            severity: "info",
            source_event_id: Some(event_id.clone()),
            source_event: None,
            option: None,
            field: None,
            target: None,
            message: format!("event `{event_id}` has no incoming or outgoing resolved event-chain edges"),
            fix: "Connect it with `后续事件A`/`隐藏效果A：触发事件 ...`, or keep it only if this event is intentionally standalone.".to_string(),
        });
    }
    for cycle in unsafe_cycles {
        items.push(EventChainReviewItem {
            kind: "unsafe_immediate_cycle",
            severity: "error",
            source_event_id: cycle.first().cloned(),
            source_event: None,
            option: None,
            field: None,
            target: None,
            message: format!(
                "event chain has unsafe immediate unconditional cycle `{}`",
                cycle.join(" -> ")
            ),
            fix: "Add `延迟A`/`延迟小时A`/`随机延迟天数A`/`后续条件A`, or remove one next-event edge.".to_string(),
        });
    }
    items
}

pub(crate) fn collect_event_chain_edge_review_items(
    items: &mut Vec<EventChainReviewItem>,
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) {
    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        for option in event_options(card) {
            collect_event_chain_edge_review_items_from_text(
                items,
                card,
                planned,
                &option,
                "effects",
                &option.effects,
                chain_index,
            );
            collect_event_chain_edge_review_items_from_text(
                items,
                card,
                planned,
                &option,
                "hidden_effects",
                &option.hidden_effects,
                chain_index,
            );
            for entry in option
                .next_events
                .iter()
                .chain(option.random_next_events.iter())
            {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_event_chain_edge_review_items_from_text(
                    items,
                    card,
                    planned,
                    &option,
                    &entry.field,
                    &next_event,
                    chain_index,
                );
            }
        }
    }
}

pub(crate) fn collect_event_chain_edge_review_items_from_text(
    items: &mut Vec<EventChainReviewItem>,
    card: &Card,
    planned: &PlannedEventId,
    option: &EventOption,
    field: &str,
    text: &str,
    chain_index: &EventChainIndex,
) {
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        let (status, _, blocker) = event_chain_edge_status(&reference, chain_index);
        let (kind, severity, fix) = match status {
            "unresolved" => (
                "unresolved_chain_target",
                "error",
                "Use an event title or `事件键` from this batch, add the missing event card, or reference an already indexed event id.",
            ),
            "ambiguous" => (
                "ambiguous_chain_target",
                "error",
                "Add unique `事件键` values to duplicated target events, then reference the intended event key.",
            ),
            _ => continue,
        };
        let message = if let Some(blocker) = blocker {
            format!(
                "event `{}` option `{}` field `{field}` target `{}` is {status}: {blocker}",
                card.title, option.key, reference.title
            )
        } else {
            format!(
                "event `{}` option `{}` field `{field}` target `{}` is {status}",
                card.title, option.key, reference.title
            )
        };
        items.push(EventChainReviewItem {
            kind,
            severity,
            source_event_id: Some(planned.event_id.clone()),
            source_event: Some(card.title.clone()),
            option: Some(option.key.clone()),
            field: Some(field.to_string()),
            target: Some(reference.title),
            message,
            fix: fix.to_string(),
        });
    }
}

pub(crate) fn event_chain_review_items_json(items: &[EventChainReviewItem]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(event_chain_review_item_json)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn event_chain_review_item_json(item: &EventChainReviewItem) -> String {
    format!(
        "{{\"kind\": {}, \"severity\": {}, \"source_event_id\": {}, \"source_event\": {}, \"option\": {}, \"field\": {}, \"target\": {}, \"message\": {}, \"fix\": {}}}",
        json_str(item.kind),
        json_str(item.severity),
        optional_json_str(item.source_event_id.as_deref()),
        optional_json_str(item.source_event.as_deref()),
        optional_json_str(item.option.as_deref()),
        optional_json_str(item.field.as_deref()),
        optional_json_str(item.target.as_deref()),
        json_str(&item.message),
        json_str(&item.fix)
    )
}

pub(crate) fn event_chain_resolved_adjacency(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> BTreeMap<String, Vec<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        for option in event_options(card) {
            collect_resolved_event_chain_targets_from_text(
                &mut adjacency,
                &planned.event_id,
                &option.effects,
                chain_index,
            );
            collect_resolved_event_chain_targets_from_text(
                &mut adjacency,
                &planned.event_id,
                &option.hidden_effects,
                chain_index,
            );
            for entry in option
                .next_events
                .iter()
                .chain(option.random_next_events.iter())
            {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_resolved_event_chain_targets_from_text(
                    &mut adjacency,
                    &planned.event_id,
                    &next_event,
                    chain_index,
                );
            }
        }
    }
    adjacency
        .into_iter()
        .map(|(source, targets)| (source, targets.into_iter().collect()))
        .collect()
}

#[derive(Clone)]
pub(crate) struct EventChainResolvedEdge {
    pub(crate) target_id: String,
    pub(crate) breaks_immediate_cycle: bool,
}

pub(crate) fn event_chain_resolved_edges(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> BTreeMap<String, Vec<EventChainResolvedEdge>> {
    let mut edges: BTreeMap<String, BTreeMap<String, bool>> = BTreeMap::new();
    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        for option in event_options(card) {
            collect_resolved_event_chain_edges_from_text(
                &mut edges,
                &planned.event_id,
                &option.effects,
                None,
                chain_index,
            );
            collect_resolved_event_chain_edges_from_text(
                &mut edges,
                &planned.event_id,
                &option.hidden_effects,
                None,
                chain_index,
            );
            for entry in option
                .next_events
                .iter()
                .chain(option.random_next_events.iter())
            {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_resolved_event_chain_edges_from_text(
                    &mut edges,
                    &planned.event_id,
                    &next_event,
                    Some(&entry.condition),
                    chain_index,
                );
            }
        }
    }
    edges
        .into_iter()
        .map(|(source, targets)| {
            (
                source,
                targets
                    .into_iter()
                    .map(
                        |(target_id, breaks_immediate_cycle)| EventChainResolvedEdge {
                            target_id,
                            breaks_immediate_cycle,
                        },
                    )
                    .collect(),
            )
        })
        .collect()
}

pub(crate) fn collect_resolved_event_chain_targets_from_text(
    adjacency: &mut BTreeMap<String, BTreeSet<String>>,
    source_id: &str,
    text: &str,
    chain_index: &EventChainIndex,
) {
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        let (status, target_id, _) = event_chain_edge_status(&reference, chain_index);
        if status == "resolved" {
            if let Some(target_id) = target_id {
                adjacency
                    .entry(source_id.to_string())
                    .or_default()
                    .insert(target_id);
            }
        }
    }
}

pub(crate) fn collect_resolved_event_chain_edges_from_text(
    edges: &mut BTreeMap<String, BTreeMap<String, bool>>,
    source_id: &str,
    text: &str,
    condition: Option<&str>,
    chain_index: &EventChainIndex,
) {
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        let (status, target_id, _) = event_chain_edge_status(&reference, chain_index);
        if status != "resolved" {
            continue;
        }
        let Some(target_id) = target_id else {
            continue;
        };
        let breaks_immediate_cycle =
            event_chain_reference_breaks_immediate_cycle(&reference, condition);
        let prior = edges
            .entry(source_id.to_string())
            .or_default()
            .entry(target_id)
            .or_insert(true);
        *prior = *prior && breaks_immediate_cycle;
    }
}

pub(crate) fn event_chain_reference_breaks_immediate_cycle(
    reference: &EventChainReference,
    condition: Option<&str>,
) -> bool {
    reference.days.is_some()
        || reference.hours.is_some()
        || reference.random_days.is_some()
        || reference.random_hours.is_some()
        || condition.is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn find_event_chain_cycles(
    adjacency: &BTreeMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut seen = BTreeSet::new();
    for start in adjacency.keys() {
        let mut stack = Vec::new();
        find_event_chain_cycles_from(start, start, adjacency, &mut stack, &mut cycles, &mut seen);
    }
    cycles
}

pub(crate) fn find_event_chain_cycles_from(
    start: &str,
    current: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    stack: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
    seen: &mut BTreeSet<String>,
) {
    stack.push(current.to_string());
    if let Some(targets) = adjacency.get(current) {
        for target in targets {
            if target == start {
                let mut cycle = stack.clone();
                cycle.push(start.to_string());
                let canonical = canonical_event_chain_cycle_key(&cycle);
                if seen.insert(canonical) {
                    cycles.push(cycle);
                }
                continue;
            }
            if stack.iter().any(|item| item == target) {
                continue;
            }
            find_event_chain_cycles_from(start, target, adjacency, stack, cycles, seen);
        }
    }
    stack.pop();
}

pub(crate) fn find_event_chain_unsafe_cycles(
    edges: &BTreeMap<String, Vec<EventChainResolvedEdge>>,
) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut seen = BTreeSet::new();
    for start in edges.keys() {
        let mut stack = Vec::new();
        find_event_chain_unsafe_cycles_from(
            start,
            start,
            edges,
            &mut stack,
            &mut cycles,
            &mut seen,
        );
    }
    cycles
}

pub(crate) fn find_event_chain_unsafe_cycles_from(
    start: &str,
    current: &str,
    edges: &BTreeMap<String, Vec<EventChainResolvedEdge>>,
    stack: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
    seen: &mut BTreeSet<String>,
) {
    stack.push(current.to_string());
    if let Some(targets) = edges.get(current) {
        for edge in targets {
            if edge.breaks_immediate_cycle {
                continue;
            }
            if edge.target_id == start {
                let mut cycle = stack.clone();
                cycle.push(start.to_string());
                let canonical = canonical_event_chain_cycle_key(&cycle);
                if seen.insert(canonical) {
                    cycles.push(cycle);
                }
                continue;
            }
            if stack.iter().any(|item| item == &edge.target_id) {
                continue;
            }
            find_event_chain_unsafe_cycles_from(start, &edge.target_id, edges, stack, cycles, seen);
        }
    }
    stack.pop();
}

pub(crate) fn canonical_event_chain_cycle_key(cycle: &[String]) -> String {
    let nodes = if cycle.len() > 1 && cycle.first() == cycle.last() {
        &cycle[..cycle.len() - 1]
    } else {
        cycle
    };
    if nodes.is_empty() {
        return String::new();
    }
    let rotations = (0..nodes.len())
        .map(|idx| {
            nodes[idx..]
                .iter()
                .chain(nodes[..idx].iter())
                .cloned()
                .collect::<Vec<_>>()
                .join(" -> ")
        })
        .collect::<Vec<_>>();
    rotations.into_iter().min().unwrap_or_default()
}

pub(crate) fn event_chain_cycles_json(cycles: &[Vec<String>]) -> String {
    format!(
        "[{}]",
        cycles
            .iter()
            .map(|cycle| json_str(&cycle.join(" -> ")))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn event_chain_layers_json(layers: &[Vec<String>]) -> String {
    format!(
        "[{}]",
        layers
            .iter()
            .map(|layer| json_array(layer))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn event_chain_edges_json(
    cards: &[Card],
    planned_ids: &[PlannedEventId],
    chain_index: &EventChainIndex,
) -> String {
    let mut edges = Vec::new();
    for (card, planned) in cards.iter().zip(planned_ids.iter()) {
        for option in event_options(card) {
            collect_event_chain_edges_from_text(
                &mut edges,
                card,
                planned,
                &option,
                "effects",
                &option.effects,
                None,
                chain_index,
            );
            collect_event_chain_edges_from_text(
                &mut edges,
                card,
                planned,
                &option,
                "hidden_effects",
                &option.hidden_effects,
                None,
                chain_index,
            );
            for entry in &option.next_events {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_event_chain_edges_from_text(
                    &mut edges,
                    card,
                    planned,
                    &option,
                    &entry.field,
                    &next_event,
                    Some(&entry.condition),
                    chain_index,
                );
            }
            for entry in &option.random_next_events {
                let Some(next_event) = event_option_next_event_entry_reference_text(entry) else {
                    continue;
                };
                collect_event_chain_edges_from_text(
                    &mut edges,
                    card,
                    planned,
                    &option,
                    &entry.field,
                    &next_event,
                    Some(&entry.condition),
                    chain_index,
                );
            }
        }
    }
    format!("[{}]", edges.join(", "))
}

pub(crate) fn collect_event_chain_edges_from_text(
    edges: &mut Vec<String>,
    card: &Card,
    planned: &PlannedEventId,
    option: &EventOption,
    field: &str,
    text: &str,
    condition: Option<&str>,
    chain_index: &EventChainIndex,
) {
    for raw in split_cn_list(text) {
        let Some(reference) = parse_event_chain_reference(raw) else {
            continue;
        };
        edges.push(event_chain_edge_json(
            card,
            planned,
            option,
            field,
            raw,
            &reference,
            condition,
            chain_index,
        ));
    }
}

pub(crate) fn event_chain_edge_json(
    card: &Card,
    planned: &PlannedEventId,
    option: &EventOption,
    field: &str,
    raw: &str,
    reference: &EventChainReference,
    condition: Option<&str>,
    chain_index: &EventChainIndex,
) -> String {
    let suggestion = resolve_event_chain_reference(raw, reference, Some(chain_index));
    let (status, target_id, blocker) = event_chain_edge_status(reference, chain_index);
    let event_type =
        event_chain_reference_effect_type(reference, chain_index, target_id.as_deref());
    format!(
        "{{\"source_event\": {}, \"source_event_id\": {}, \"option\": {}, \"field\": {}, \"target\": {}, \"target_event_id\": {}, \"event_type\": {}, \"scope\": {}, \"condition\": {}, \"days\": {}, \"hours\": {}, \"random_days\": {}, \"random_hours\": {}, \"status\": {}, \"code\": {}, \"blocker\": {}}}",
        json_str(&card.title),
        json_str(&planned.event_id),
        json_str(&option.key),
        json_str(field),
        json_str(&reference.title),
        optional_json_str(target_id.as_deref()),
        json_str(&event_type),
        optional_json_str(reference.scope.as_deref()),
        optional_json_str(condition.filter(|value| !value.trim().is_empty())),
        optional_i64_json(reference.days),
        optional_i64_json(reference.hours),
        optional_i64_json(reference.random_days),
        optional_i64_json(reference.random_hours),
        json_str(status),
        json_str(&suggestion.code),
        optional_json_str(blocker)
    )
}

pub(crate) fn event_chain_edge_status<'a>(
    reference: &'a EventChainReference,
    chain_index: &'a EventChainIndex,
) -> (&'static str, Option<String>, Option<&'static str>) {
    if let Some(event_id) = reference.explicit_event_id.as_ref() {
        if chain_index.known_ids.contains(event_id) {
            return ("resolved", Some(event_id.clone()), None);
        }
        return (
            "unresolved",
            None,
            Some("explicit event id is not known in the current batch or scanned mod events"),
        );
    }
    if chain_index.duplicate_titles.contains(&reference.title) {
        return (
            "ambiguous",
            None,
            Some("event title is duplicated in this batch; add unique event keys and reference the intended event key"),
        );
    }
    if let Some(event_id) = chain_index.title_to_id.get(&reference.title) {
        return ("resolved", Some(event_id.clone()), None);
    }
    (
        "unresolved",
        None,
        Some("event title was not found in this batch"),
    )
}

pub(crate) fn optional_json_str(value: Option<&str>) -> String {
    value.map(json_str).unwrap_or_else(|| "null".to_string())
}

pub(crate) fn optional_i64_json(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

pub(crate) fn event_cards_safety_suggestions(cards: &[Card]) -> Vec<Suggestion> {
    event_cards_safety_suggestions_with_chain(cards, None)
}

pub(crate) fn event_cards_safety_suggestions_with_chain(
    cards: &[Card],
    chain_index: Option<&EventChainIndex>,
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    for card in cards {
        if let Some(trigger) = event_trigger_text(card) {
            out.extend(suggest_trigger(trigger));
        }
        if let Some(immediate) = event_immediate_effect_text(card) {
            out.extend(event_option_effect_suggestions(immediate, chain_index));
        }
        if let Some(after) = event_after_effect_text(card) {
            out.extend(event_option_effect_suggestions(after, chain_index));
        }
        if let Some(after_hidden) = event_after_hidden_effect_text(card) {
            out.extend(event_option_effect_suggestions(after_hidden, chain_index));
        }
        for option in event_options(card) {
            if !option.trigger.trim().is_empty() {
                out.extend(
                    split_cn_list(&option.trigger)
                        .into_iter()
                        .flat_map(suggest_trigger),
                );
            }
            out.extend(event_option_effect_suggestions(
                &option.effects,
                chain_index,
            ));
            out.extend(event_option_effect_suggestions(
                &option.effect_tooltip_effects,
                chain_index,
            ));
            out.extend(event_option_effect_suggestions(
                &option.hidden_effects,
                chain_index,
            ));
            for entry in option
                .next_events
                .iter()
                .chain(option.random_next_events.iter())
            {
                out.extend(event_option_next_event_condition_suggestions(entry));
            }
            out.extend(event_option_next_event_suggestions(&option, chain_index));
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
