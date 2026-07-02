//! Generic Chinese card parsing and suggestion inference used by multiple generators.

#[allow(unused_imports)]
use crate::*;

pub(crate) struct Card {
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) fields: BTreeMap<String, String>,
}

pub(crate) fn parse_cards(text: &str, allowed: &[&str]) -> Vec<Card> {
    let mut cards = Vec::new();
    let mut current: Option<Card> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if is_focus_layout_noise_line(trimmed) {
            continue;
        }
        if trimmed.chars().all(|c| c == '-') {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            continue;
        }
        if let Some((key, val)) = split_field(trimmed) {
            if allowed.contains(&key) {
                if let Some(card) = current.take() {
                    cards.push(card);
                }
                current = Some(Card {
                    kind: key.to_string(),
                    title: val.to_string(),
                    fields: BTreeMap::new(),
                });
            } else if is_focus_layout_explanatory_field(key, val) {
                continue;
            } else if let Some(card) = current.as_mut() {
                card.fields.insert(key.to_string(), val.to_string());
            }
        } else if let Some(card) = current.as_mut() {
            card.fields
                .entry("描述".to_string())
                .and_modify(|s| {
                    s.push('\n');
                    s.push_str(trimmed);
                })
                .or_insert_with(|| trimmed.to_string());
        }
    }
    if let Some(card) = current.take() {
        cards.push(card);
    }
    cards
}

pub(crate) fn split_field(line: &str) -> Option<(&str, &str)> {
    let (idx, sep) = line.char_indices().find(|(_, c)| *c == ':' || *c == '：')?;
    let value_start = idx + sep.len_utf8();
    Some((line[..idx].trim(), line[value_start..].trim()))
}

pub(crate) fn join_existing_fields(
    fields: &BTreeMap<String, String>,
    keys: &[&str],
) -> Option<String> {
    let values = keys
        .iter()
        .filter_map(|key| fields.get(*key))
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join("；"))
    }
}

pub(crate) fn cmd_compile_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input_text = intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&input_text);
    let requested_context =
        normalize_intent_context(value(&map, "kind").or_else(|| value(&map, "context")))?;
    let mut context = if requested_context == "auto" {
        infer_intent_context(&intent)
    } else {
        requested_context
    };
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
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during intent compilation".to_string());
    }
    let options = validation_options_from_args(&map);
    if options.strict_code_index && game_index.is_none() {
        return Err(
            "strict intent compilation requires --game-root before accepting compiled code"
                .to_string(),
        );
    }
    if game_index
        .as_ref()
        .and_then(|index| dynamic_modifier_id_from_intent(&intent, index))
        .is_some()
        && matches!(context, "auto" | "idea")
    {
        context = "effect";
    }
    if game_index
        .as_ref()
        .is_some_and(|index| idea_replacement_intent(intent.as_str(), index).is_some())
        && matches!(context, "auto" | "idea")
    {
        context = "effect";
    }
    if model_normalized_intent(&intent).is_some() && matches!(context, "auto" | "idea") {
        context = "effect";
    }
    if add_idea_intent(&intent).is_some() && matches!(context, "auto" | "idea") {
        context = "effect";
    }

    let generation_prefix = value(&map, "prefix").unwrap_or("mod");
    let generation_tag = value(&map, "tag");
    let suggestions =
        compile_intent_suggestions(&intent, context, game_index.as_ref(), generation_prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("intent", &suggestions, game_index.as_ref());
    errors.extend(context_suggestion_errors("intent", context, &suggestions));
    if let Some(index) = game_index.as_ref() {
        if options.strict_code_index {
            errors.extend(missing_code_index_category_errors(
                "intent",
                &suggestions,
                index,
            ));
        }
        errors.extend(unindexed_suggestion_errors("intent", &suggestions, index));
    }
    let final_blockers =
        compile_intent_final_blockers(options.strict_code_index, game_index.as_ref(), &errors);
    let json = compile_intent_json(
        &input_text,
        &intent,
        context,
        options.strict_code_index,
        game_index.as_ref(),
        &suggestions,
        &errors,
        generation_prefix,
        generation_tag,
    );
    write_or_print(&json, value(&map, "output"))?;
    if !errors.is_empty() {
        Err("intent compilation blocked unresolved or unindexed HOI4 code".to_string())
    } else if compile_intent_requires_final_code(&map) && !final_blockers.is_empty() {
        Err("intent compilation produced draft-only code; rerun with --game-root and --strict-code-index before final use".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn cmd_author_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input_text = intent_text_from_args(&map)?;
    let systems_for_text = author_intent_systems_from_text(&input_text);
    let intent =
        normalize_llm_intent_text(&author_intent_effect_text(&input_text, &systems_for_text));
    let requested_context = normalize_intent_context(
        value(&map, "kind")
            .or_else(|| value(&map, "context"))
            .or(Some("auto")),
    )?;
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
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during author intent planning".to_string());
    }
    let options = validation_options_from_args(&map);
    let context = effective_compile_intent_context(&intent, requested_context, game_index.as_ref());
    let generation_prefix = value(&map, "prefix").unwrap_or("mod");
    let generation_tag = value(&map, "tag");
    let suggestions =
        compile_intent_suggestions(&intent, context, game_index.as_ref(), generation_prefix)?;
    let mut errors = unresolved_suggestion_errors_with_index(
        "author intent plan",
        &suggestions,
        game_index.as_ref(),
    );
    errors.extend(context_suggestion_errors(
        "author intent plan",
        context,
        &suggestions,
    ));
    if let Some(index) = game_index.as_ref() {
        if options.strict_code_index {
            errors.extend(missing_code_index_category_errors(
                "author intent plan",
                &suggestions,
                index,
            ));
        }
        errors.extend(unindexed_suggestion_errors(
            "author intent plan",
            &suggestions,
            index,
        ));
    }
    let mut blockers =
        compile_intent_final_blockers(options.strict_code_index, game_index.as_ref(), &errors);
    let systems = author_intent_systems(&input_text, &intent, &suggestions, game_index.as_ref());
    blockers.extend(author_intent_missing_context_blockers(&map, &systems));
    blockers.sort();
    blockers.dedup();
    let plan = author_intent_plan_json(
        &input_text,
        &intent,
        context,
        generation_prefix,
        generation_tag,
        &systems,
        &suggestions,
        &errors,
        &blockers,
        &map,
        options.strict_code_index,
        game_index.as_ref(),
    );
    write_or_print(&plan, value(&map, "output"))?;
    if blockers.is_empty() {
        Ok(())
    } else if map.flags.contains("fail-on-blocker") || map.flags.contains("require-ready") {
        Err("author intent plan blocked missing context or unsafe code".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn cmd_author_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("execute") {
        return cmd_author_intent_plan(args);
    }
    if !map.flags.contains("final-check") {
        return Err(
            "author-intent --execute writes mod files and requires --final-check".to_string(),
        );
    }
    let input_text = intent_text_from_args(&map)?;
    let systems_for_text = author_intent_systems_from_text(&input_text);
    let intent =
        normalize_llm_intent_text(&author_intent_effect_text(&input_text, &systems_for_text));
    let requested_context = normalize_intent_context(
        value(&map, "kind")
            .or_else(|| value(&map, "context"))
            .or(Some("auto")),
    )?;
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "author-intent --execute requires --game-root".to_string())?,
    )?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_mods =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    let context = effective_compile_intent_context(&intent, requested_context, Some(&game_index));
    let generation_prefix = value(&map, "prefix").unwrap_or("mod");
    let generation_tag = value(&map, "tag");
    let suggestions =
        compile_intent_suggestions(&intent, context, Some(&game_index), generation_prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("author intent", &suggestions, Some(&game_index));
    errors.extend(context_suggestion_errors(
        "author intent",
        context,
        &suggestions,
    ));
    errors.extend(missing_code_index_category_errors(
        "author intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(unindexed_suggestion_errors(
        "author intent",
        &suggestions,
        &game_index,
    ));
    errors.sort();
    errors.dedup();
    let systems = author_intent_systems(&input_text, &intent, &suggestions, Some(&game_index));
    let writer = author_intent_primary_writer(&systems);
    let mut blockers = errors.clone();
    blockers.extend(author_intent_execute_blockers(&map, &systems, writer));
    blockers.sort();
    blockers.dedup();
    if !blockers.is_empty() {
        let plan = author_intent_plan_json(
            &input_text,
            &intent,
            context,
            generation_prefix,
            generation_tag,
            &systems,
            &suggestions,
            &errors,
            &blockers,
            &map,
            true,
            Some(&game_index),
        );
        write_or_print(&plan, value(&map, "output"))?;
        return Err("author-intent execution blocked missing context or unsafe code".to_string());
    }
    let prepared = prepare_author_intent_execution_files(&map, &input_text, &intent, &systems)?;
    let mut delegated_args = author_intent_delegated_args_without_text(args);
    delegated_args.push("--intent-input".to_string());
    delegated_args.push(prepared.intent_path.display().to_string());
    match writer {
        "plan-dynamic-modifier-change" => {
            delegated_args.push("--input".to_string());
            delegated_args.push(prepared.intent_path.display().to_string());
            cmd_plan_dynamic_modifier_change(&delegated_args)
        }
        "apply-focus-intent" => {
            let parent = prepared.parent_path.as_ref().ok_or_else(|| {
                "author-intent execution could not prepare focus input".to_string()
            })?;
            author_intent_push_default_effect_kind(&map, &mut delegated_args);
            delegated_args.push("--input".to_string());
            delegated_args.push(parent.display().to_string());
            if value(&map, "focus-title").is_none() {
                if let Some(title) = author_intent_title(&input_text, "focus") {
                    delegated_args.push("--focus-title".to_string());
                    delegated_args.push(title);
                }
            }
            cmd_apply_focus_intent(&delegated_args)
        }
        "apply-event-intent" => {
            let parent = prepared.parent_path.as_ref().ok_or_else(|| {
                "author-intent execution could not prepare event input".to_string()
            })?;
            author_intent_push_default_effect_kind(&map, &mut delegated_args);
            delegated_args.push("--input".to_string());
            delegated_args.push(parent.display().to_string());
            if value(&map, "event-title").is_none() {
                if let Some(title) = author_intent_title(&input_text, "event") {
                    delegated_args.push("--event-title".to_string());
                    delegated_args.push(title);
                }
            }
            if value(&map, "option").is_none() {
                delegated_args.push("--option".to_string());
                delegated_args.push("A".to_string());
            }
            cmd_apply_event_intent(&delegated_args)
        }
        "apply-decision-intent" => {
            let parent = prepared.parent_path.as_ref().ok_or_else(|| {
                "author-intent execution could not prepare decision input".to_string()
            })?;
            author_intent_push_default_effect_kind(&map, &mut delegated_args);
            delegated_args.push("--input".to_string());
            delegated_args.push(parent.display().to_string());
            if value(&map, "decision-title").is_none() {
                if let Some(title) = author_intent_title(&input_text, "decision") {
                    delegated_args.push("--decision-title".to_string());
                    delegated_args.push(title);
                }
            }
            cmd_apply_decision_intent(&delegated_args)
        }
        _ => {
            author_intent_push_default_effect_kind(&map, &mut delegated_args);
            cmd_apply_intent_patch_plan(&delegated_args)
        }
    }
}

fn compile_intent_requires_final_code(map: &ArgMap) -> bool {
    map.flags.contains("require-final-code")
        || map.flags.contains("fail-on-draft")
        || map.flags.contains("no-draft")
}

pub(crate) fn cmd_plan_dynamic_modifier_change(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input_text = intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&input_text);
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "plan-dynamic-modifier-change requires --game-root".to_string())?,
    )?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_mods =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let dynamic_modifier = dynamic_modifier_id_from_intent(&intent, &game_index)
        .or_else(|| dynamic_modifier_id_from_explicit_protocol(&intent))
        .or_else(|| dynamic_modifier_id_from_author_label(&intent, prefix));
    let generated_spec = if dynamic_modifier
        .as_ref()
        .is_some_and(|id| !game_index.dynamic_modifiers.contains(id))
    {
        dynamic_modifier
            .as_deref()
            .map(|id| generated_dynamic_modifier_spec_from_intent(&intent, id, &game_index))
            .transpose()?
    } else {
        None
    };
    let suggestions = if let Some(spec) = generated_spec.as_ref() {
        generated_dynamic_modifier_suggestions(spec)
    } else {
        dynamic_modifier_intent_suggestions(&intent, &game_index)
    };
    let mut errors = Vec::new();
    if dynamic_modifier.is_none() {
        errors.push(
            "dynamic modifier plan requires an indexed dynamic modifier name, explicit dynamic modifier ID, or `动态修正：<name>` in --text"
                .to_string(),
        );
    }
    if suggestions.is_empty() {
        errors.push(
            "dynamic modifier plan found no effect segments; provide a concrete numeric change"
                .to_string(),
        );
    }
    errors.extend(unresolved_suggestion_errors_with_index(
        "dynamic_modifier_change",
        &suggestions,
        Some(&game_index),
    ));
    errors.extend(context_suggestion_errors(
        "dynamic_modifier_change",
        "effect",
        &suggestions,
    ));
    errors.extend(missing_code_index_category_errors(
        "dynamic_modifier_change",
        &suggestions,
        &game_index,
    ));
    errors.extend(dynamic_modifier_generated_spec_errors(
        generated_spec.as_ref(),
        &game_index,
    ));
    let allowed_generated_effects = generated_spec
        .as_ref()
        .map(|spec| spec.allowed_scripted_effects())
        .unwrap_or_default();
    errors.extend(filter_allowed_generated_scripted_effect_errors(
        unindexed_suggestion_errors("dynamic_modifier_change", &suggestions, &game_index),
        &allowed_generated_effects,
    ));
    if suggestions
        .iter()
        .any(|suggestion| suggestion.kind != "country_effect")
    {
        errors.push(
            "dynamic modifier change must resolve to verified country_effect snippets; do not create a national spirit fallback"
                .to_string(),
        );
    }
    if map.flags.contains("execute") && !map.flags.contains("final-check") {
        errors.push(
            "plan-dynamic-modifier-change --execute writes mod files and requires --final-check"
                .to_string(),
        );
    }
    if map.flags.contains("execute") && value(&map, "mod-root").is_none() {
        errors.push("plan-dynamic-modifier-change --execute requires --mod-root".to_string());
    }
    if map.flags.contains("execute") && value(&map, "tag").is_none() {
        errors.push("plan-dynamic-modifier-change --execute requires --tag".to_string());
    }
    let mut apply_result = None;
    if errors.is_empty() && map.flags.contains("execute") {
        let mod_root = normalize_path(value(&map, "mod-root").ok_or_else(|| {
            "plan-dynamic-modifier-change --execute requires --mod-root".to_string()
        })?)?;
        let tag = value(&map, "tag").unwrap_or("TAG");
        let result = if let Some(spec) = generated_spec.as_ref() {
            apply_generated_dynamic_modifier_change(&mod_root, tag, prefix, spec)?
        } else {
            AppliedDynamicModifierChange {
                changed_files: Vec::new(),
                generated_symbols: Vec::new(),
            }
        };
        let mut check_mod_paths = dependency_mods.clone();
        check_mod_paths.push(mod_root.clone());
        let post_index = build_game_index_with_mod_paths(&game_root, &check_mod_paths)?;
        run_post_apply_checks(&mod_root, &map, Some(&post_index), None)?;
        apply_result = Some(result);
    }
    errors.sort();
    errors.dedup();
    let json = dynamic_modifier_change_plan_json(
        &input_text,
        &intent,
        dynamic_modifier.as_deref(),
        &suggestions,
        &errors,
        apply_result.as_ref(),
    );
    write_or_print(&json, value(&map, "output"))?;
    if errors.is_empty() {
        Ok(())
    } else {
        Err("dynamic modifier change plan blocked unresolved or unindexed code".to_string())
    }
}

pub(crate) fn dynamic_modifier_id_from_explicit_protocol(intent: &str) -> Option<String> {
    for token in intent.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_')) {
        if let Some(id) = token.strip_prefix("temp_") {
            if is_reference_identifier(id) {
                return Some(id.to_string());
            }
        }
        if let Some(id) = token.strip_prefix("change_") {
            if is_reference_identifier(id) {
                return Some(id.to_string());
            }
        }
    }
    intent
        .split_whitespace()
        .find(|token| {
            is_reference_identifier(token)
                && (token.contains("_dynamic")
                    || token.contains("_made_in")
                    || token.contains("_modifier"))
        })
        .map(str::to_string)
}

pub(crate) fn dynamic_modifier_id_from_author_label(intent: &str, prefix: &str) -> Option<String> {
    let label = dynamic_modifier_display_name_from_intent(intent)?;
    let slug = sanitize_identifier_part(&label, "dynamic_modifier");
    Some(format!(
        "{}_{}",
        sanitize_identifier_part(prefix, "mod"),
        slug
    ))
}

pub(crate) fn dynamic_modifier_display_name_from_intent(intent: &str) -> Option<String> {
    for line in intent.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains("调用协议") {
            continue;
        }
        if let Some((key, value)) = split_field(trimmed) {
            if key.contains("动态修正")
                || key.contains("dynamic modifier")
                || key.contains("dynamic_modifier")
            {
                let name = value
                    .split([' ', '，', ',', '；', ';', '\n'])
                    .next()
                    .unwrap_or(value)
                    .trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
        if let Some((name, _)) = trimmed.split_once("的效果") {
            let name = name.trim();
            if !name.is_empty() && !is_reference_identifier(name) {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[derive(Clone)]
pub(crate) struct GeneratedDynamicModifierEntry {
    pub(crate) source: String,
    pub(crate) modifier: String,
    pub(crate) value: f64,
    pub(crate) variable: String,
    pub(crate) temp_variable: String,
    pub(crate) scripted_effect: String,
    pub(crate) tooltip_key: String,
    pub(crate) tooltip_text: String,
}

#[derive(Clone)]
pub(crate) struct GeneratedDynamicModifierSpec {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) icon: String,
    pub(crate) entries: Vec<GeneratedDynamicModifierEntry>,
}

impl GeneratedDynamicModifierSpec {
    pub(crate) fn allowed_scripted_effects(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .map(|entry| entry.scripted_effect.clone())
            .collect()
    }
}

pub(crate) fn generated_dynamic_modifier_spec_from_intent(
    intent: &str,
    dynamic_modifier: &str,
    index: &GameIndex,
) -> Result<GeneratedDynamicModifierSpec, String> {
    let display_name = dynamic_modifier_display_name_from_intent(intent)
        .unwrap_or_else(|| dynamic_modifier.to_string());
    let mut entries = Vec::new();
    for raw in split_cn_list(intent) {
        let segment = raw.trim();
        if segment.is_empty()
            || segment.starts_with('#')
            || segment.contains("custom_effect_tooltip")
            || segment.contains("set_temp_variable")
            || segment.contains("change_")
            || segment.contains("调用协议")
        {
            continue;
        }
        let cleaned = dynamic_modifier_effect_segment(segment, dynamic_modifier, index);
        let cleaned = cleaned
            .trim_start_matches(&display_name)
            .trim_start_matches("的效果")
            .trim_start_matches("效果")
            .trim_start_matches(':')
            .trim_start_matches('：')
            .trim();
        if cleaned.is_empty() {
            continue;
        }
        let Some(value) = dynamic_modifier_effect_value(cleaned) else {
            continue;
        };
        let Some(modifier) = dynamic_modifier_modifier_key_for_segment(cleaned) else {
            return Err(format!(
                "dynamic modifier `{dynamic_modifier}` effect segment `{cleaned}` is not mapped to an indexed modifier; ask the user or model to normalize it before writing"
            ));
        };
        let slot = dynamic_modifier_slot_for_modifier(modifier);
        let variable = format!("{dynamic_modifier}_{slot}");
        let temp_variable = format!("temp_{dynamic_modifier}");
        let scripted_effect = format!("change_{dynamic_modifier}_{slot}");
        let tooltip_key = format!("{scripted_effect}_tt");
        let tooltip_text = dynamic_modifier_tooltip_text(cleaned, &temp_variable, value);
        entries.push(GeneratedDynamicModifierEntry {
            source: cleaned.to_string(),
            modifier: modifier.to_string(),
            value,
            variable,
            temp_variable,
            scripted_effect,
            tooltip_key,
            tooltip_text,
        });
    }
    if entries.is_empty() {
        return Err(format!(
            "dynamic modifier `{dynamic_modifier}` has no parseable numeric modifier effects"
        ));
    }
    Ok(GeneratedDynamicModifierSpec {
        id: dynamic_modifier.to_string(),
        display_name,
        icon: String::new(),
        entries,
    })
}

pub(crate) fn dynamic_modifier_modifier_key_for_segment(segment: &str) -> Option<&'static str> {
    if segment.contains("生产效率上限") || segment.contains("效率上限") {
        Some("production_factory_max_efficiency_factor")
    } else if segment.contains("建造速度") || segment.contains("建设速度") {
        Some("production_speed_buildings_factor")
    } else if segment.contains("稳定度") || segment.contains("稳定") {
        Some("stability_factor")
    } else if segment.contains("政治点") || segment.contains("政治力量") {
        Some("political_power_gain")
    } else if segment.contains("战争支持") || segment.contains("战争支援") {
        Some("war_support_factor")
    } else if segment.contains("战争正当化") || segment.contains("正当化战争") {
        Some("justify_war_goal_time")
    } else {
        semantic_modifier_key(segment)
    }
}

pub(crate) fn dynamic_modifier_slot_for_modifier(modifier: &str) -> String {
    match modifier {
        "production_speed_buildings_factor" => "build_speed".to_string(),
        "production_factory_max_efficiency_factor" => "max_efficiency".to_string(),
        "stability_factor" => "stability".to_string(),
        "political_power_gain" => "political_power".to_string(),
        "war_support_factor" => "war_support".to_string(),
        "justify_war_goal_time" => "justify_war".to_string(),
        other => sanitize_identifier_part(other, "modifier"),
    }
}

pub(crate) fn dynamic_modifier_tooltip_text(
    segment: &str,
    temp_variable: &str,
    value: f64,
) -> String {
    let label = segment
        .split(['+', '-', '=', '＝'])
        .next()
        .unwrap_or(segment)
        .trim()
        .trim_end_matches(':')
        .trim_end_matches('：')
        .trim();
    if label.is_empty() {
        format!("动态修正变化：{}", fmt_float(value))
    } else {
        format!("{label}：[?{temp_variable}|+%]")
    }
}

pub(crate) fn generated_dynamic_modifier_suggestions(
    spec: &GeneratedDynamicModifierSpec,
) -> Vec<Suggestion> {
    let mut lines = vec![format!("custom_effect_tooltip = {}_tt", spec.id)];
    for entry in &spec.entries {
        lines.push(format!(
            "set_temp_variable = {{ {} = {} }}",
            entry.temp_variable,
            fmt_float(entry.value)
        ));
        lines.push(format!("{} = yes", entry.scripted_effect));
    }
    vec![Suggestion::new(
        "country_effect",
        &lines.join("\n"),
        &format!(
            "{}: {}",
            spec.id,
            spec.entries
                .iter()
                .map(|entry| entry.source.as_str())
                .collect::<Vec<_>>()
                .join("；")
        ),
        "Generated variable-driven dynamic modifier protocol; Rust writer must emit common/dynamic_modifiers and common/scripted_effects before final validation.",
    )]
}

pub(crate) fn dynamic_modifier_generated_spec_errors(
    spec: Option<&GeneratedDynamicModifierSpec>,
    index: &GameIndex,
) -> Vec<String> {
    let Some(spec) = spec else {
        return Vec::new();
    };
    let mut errors = Vec::new();
    for effect in [
        "custom_effect_tooltip",
        "set_temp_variable",
        "add_to_variable",
        "add_dynamic_modifier",
    ] {
        if !index.effects.is_empty() && !index.effects.contains(effect) {
            errors.push(format!(
                "dynamic_modifier_change: generated `{}` requires indexed effect `{effect}` before final generation",
                spec.id
            ));
        }
    }
    for entry in &spec.entries {
        if !index.modifiers.is_empty() && !index.modifiers.contains(&entry.modifier) {
            let related = related_code_symbols_text(index, &entry.modifier, Some("modifier"));
            errors.push(format!(
                "dynamic_modifier_change: `{}` maps `{}` to unindexed modifier `{}`{}",
                spec.id, entry.source, entry.modifier, related
            ));
        }
    }
    errors
}

pub(crate) fn filter_allowed_generated_scripted_effect_errors(
    errors: Vec<String>,
    allowed_effects: &BTreeSet<String>,
) -> Vec<String> {
    if allowed_effects.is_empty() {
        return errors;
    }
    errors
        .into_iter()
        .filter(|error| {
            !allowed_effects
                .iter()
                .any(|effect| error.contains(&format!("unindexed effect `{effect}`")))
        })
        .collect()
}

#[derive(Clone)]
pub(crate) struct AppliedDynamicModifierChange {
    pub(crate) changed_files: Vec<PathBuf>,
    pub(crate) generated_symbols: Vec<String>,
}

pub(crate) fn apply_generated_dynamic_modifier_change(
    mod_root: &Path,
    tag: &str,
    prefix: &str,
    spec: &GeneratedDynamicModifierSpec,
) -> Result<AppliedDynamicModifierChange, String> {
    let prefix = sanitize_identifier_part(prefix, "mod");
    let dynamic_path = mod_root
        .join("common")
        .join("dynamic_modifiers")
        .join(format!("{prefix}_dynamic_modifiers.txt"));
    let scripted_path = mod_root
        .join("common")
        .join("scripted_effects")
        .join(format!("{prefix}_dynamic_modifier_effects.txt"));
    let loc_path = target_localisation_path(mod_root, tag);
    let dynamic_block = render_generated_dynamic_modifier_block(spec);
    let scripted_blocks = spec
        .entries
        .iter()
        .map(|entry| {
            (
                entry.scripted_effect.clone(),
                render_generated_dynamic_modifier_scripted_effect(spec, entry),
            )
        })
        .collect::<Vec<_>>();
    let mut loc_entries = BTreeMap::new();
    loc_entries.insert(spec.id.clone(), spec.display_name.clone());
    loc_entries.insert(
        format!("{}_tt", spec.id),
        format!("§H{}§!追加动态修正：\\n", spec.display_name),
    );
    for entry in &spec.entries {
        loc_entries.insert(entry.tooltip_key.clone(), entry.tooltip_text.clone());
    }

    let mut changed_files = Vec::new();
    if append_unique_blocks(
        &dynamic_path,
        "# Generated dynamic modifiers by hoi4skill\n",
        &[(spec.id.clone(), dynamic_block)],
    )? {
        changed_files.push(dynamic_path);
    }
    if append_unique_blocks(
        &scripted_path,
        "# Generated dynamic modifier scripted effects by hoi4skill\n",
        &scripted_blocks,
    )? {
        changed_files.push(scripted_path);
    }
    if append_localisation_entries(&loc_path, &loc_entries)? {
        changed_files.push(loc_path);
    }
    let mut generated_symbols = vec![spec.id.clone()];
    generated_symbols.extend(
        spec.entries
            .iter()
            .map(|entry| entry.scripted_effect.clone()),
    );
    Ok(AppliedDynamicModifierChange {
        changed_files,
        generated_symbols,
    })
}

pub(crate) fn render_generated_dynamic_modifier_block(
    spec: &GeneratedDynamicModifierSpec,
) -> String {
    let mut out = format!("{} = {{\n", spec.id);
    if !spec.icon.trim().is_empty() {
        out.push_str(&format!("    icon = {}\n", spec.icon));
    }
    for entry in &spec.entries {
        out.push_str(&format!("    {} = {}\n", entry.modifier, entry.variable));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_generated_dynamic_modifier_scripted_effect(
    spec: &GeneratedDynamicModifierSpec,
    entry: &GeneratedDynamicModifierEntry,
) -> String {
    format!(
        "{} = {{\n    custom_effect_tooltip = {}\n    set_temp_variable = {{ {} = {} }}\n    add_to_variable = {{ {} = {} }}\n    add_dynamic_modifier = {{ modifier = {} }}\n}}\n",
        entry.scripted_effect,
        entry.tooltip_key,
        entry.temp_variable,
        fmt_float(entry.value),
        entry.variable,
        entry.temp_variable,
        spec.id
    )
}

pub(crate) fn cmd_apply_intent_patch_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("final-check") {
        return Err(
            "apply-intent-patch-plan writes mod files and requires --final-check".to_string(),
        );
    }
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let input_text = intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&input_text);
    let requested_context =
        normalize_intent_context(value(&map, "kind").or_else(|| value(&map, "context")))?;
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "apply-intent-patch-plan requires --game-root".to_string())?,
    )?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &mod_root, true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    let context = effective_compile_intent_context(&intent, requested_context, Some(&game_index));
    let generation_prefix = value(&map, "prefix").unwrap_or("mod");
    let generation_tag = value(&map, "tag").ok_or_else(|| {
        "apply-intent-patch-plan requires --tag for localisation routing".to_string()
    })?;
    let suggestions =
        compile_intent_suggestions(&intent, context, Some(&game_index), generation_prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("intent", &suggestions, Some(&game_index));
    errors.extend(context_suggestion_errors("intent", context, &suggestions));
    errors.extend(missing_code_index_category_errors(
        "intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(unindexed_suggestion_errors(
        "intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(apply_intent_patch_plan_errors(&suggestions));
    errors.sort();
    errors.dedup();
    if !errors.is_empty() {
        let json = compile_intent_json(
            &input_text,
            &intent,
            context,
            true,
            Some(&game_index),
            &suggestions,
            &errors,
            generation_prefix,
            Some(generation_tag),
        );
        write_or_print(&json, value(&map, "output"))?;
        return Err("intent patch apply blocked unresolved or unindexed HOI4 code".to_string());
    }

    let result = apply_intent_suggestions_to_mod(
        &mod_root,
        &suggestions,
        generation_prefix,
        generation_tag,
    )?;
    run_post_apply_checks(&mod_root, &map, Some(&game_index), None)?;
    let json = apply_intent_patch_report_json(
        &input_text,
        &intent,
        context,
        generation_prefix,
        generation_tag,
        &suggestions,
        &result,
    );
    write_or_print(&json, value(&map, "output"))?;
    Ok(())
}

pub(crate) fn cmd_apply_focus_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("final-check") {
        return Err("apply-focus-intent writes mod files and requires --final-check".to_string());
    }
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("focus");
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "apply-focus-intent requires --game-root".to_string())?,
    )?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &mod_root, true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    enforce_tag_request_contract(&map, tag, Some(&game_index))?;

    let layout_text = read_utf8_lossy(&input)?;
    let mut layout = parse_focus_layout_with_rewards(&layout_text, tag, prefix);
    if let Some(tree_id) = value(&map, "tree-id") {
        layout.tree_id = tree_id.to_string();
    }
    let intent_text = focus_intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&intent_text);
    let requested_context =
        normalize_intent_context(value(&map, "kind").or_else(|| value(&map, "context")))?;
    let context = effective_compile_intent_context(&intent, requested_context, Some(&game_index));
    let suggestions = compile_intent_suggestions(&intent, context, Some(&game_index), prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("focus intent", &suggestions, Some(&game_index));
    errors.extend(context_suggestion_errors(
        "focus intent",
        context,
        &suggestions,
    ));
    errors.extend(missing_code_index_category_errors(
        "focus intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(unindexed_suggestion_errors(
        "focus intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(apply_intent_patch_plan_errors(&suggestions));
    errors.sort();
    errors.dedup();
    if !errors.is_empty() {
        let json = compile_intent_json(
            &intent_text,
            &intent,
            context,
            true,
            Some(&game_index),
            &suggestions,
            &errors,
            prefix,
            Some(tag),
        );
        write_or_print(&json, value(&map, "output"))?;
        return Err("focus intent apply blocked unresolved or unindexed HOI4 code".to_string());
    }

    let target_focus_id = inject_intent_effects_into_focus_layout(
        &mut layout,
        &suggestions,
        value(&map, "focus-id"),
        value(&map, "focus-title"),
    )?;
    enforce_strict_focus_layout_gate(&map, &mod_root, &layout, tag, Some(&game_index))?;
    let mut changed_files =
        apply_intent_suggestions_to_mod(&mod_root, &suggestions, prefix, tag)?.changed_files;
    changed_files.extend(apply_focus_layout_to_mod_with_index(
        &mod_root,
        &layout,
        tag,
        prefix,
        Some(&game_index),
    )?);
    changed_files.sort();
    changed_files.dedup();
    run_post_apply_checks(&mod_root, &map, Some(&game_index), Some(&input))?;
    let report = apply_focus_intent_report_json(
        &layout_text,
        &intent_text,
        context,
        tag,
        prefix,
        &target_focus_id,
        &suggestions,
        &changed_files,
    );
    write_or_print(&report, value(&map, "output"))?;
    Ok(())
}

pub(crate) fn cmd_apply_decision_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("final-check") {
        return Err(
            "apply-decision-intent writes mod files and requires --final-check".to_string(),
        );
    }
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("feature");
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "apply-decision-intent requires --game-root".to_string())?,
    )?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &mod_root, true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    enforce_tag_request_contract(&map, tag, Some(&game_index))?;

    let card_text = read_utf8_lossy(&input)?;
    let mut cards = parse_cards(&card_text, FEATURE_CARD_HEADERS);
    let intent_text = focus_intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&intent_text);
    let requested_context =
        normalize_intent_context(value(&map, "kind").or_else(|| value(&map, "context")))?;
    let context = effective_compile_intent_context(&intent, requested_context, Some(&game_index));
    let suggestions = compile_intent_suggestions(&intent, context, Some(&game_index), prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("decision intent", &suggestions, Some(&game_index));
    errors.extend(context_suggestion_errors(
        "decision intent",
        context,
        &suggestions,
    ));
    errors.extend(missing_code_index_category_errors(
        "decision intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(unindexed_suggestion_errors(
        "decision intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(apply_intent_patch_plan_errors(&suggestions));
    errors.sort();
    errors.dedup();
    if !errors.is_empty() {
        let json = compile_intent_json(
            &intent_text,
            &intent,
            context,
            true,
            Some(&game_index),
            &suggestions,
            &errors,
            prefix,
            Some(tag),
        );
        write_or_print(&json, value(&map, "output"))?;
        return Err("decision intent apply blocked unresolved or unindexed HOI4 code".to_string());
    }

    let target_decision_title = inject_intent_effects_into_decision_cards(
        &mut cards,
        &suggestions,
        value(&map, "decision-title"),
    )?;
    let mut gate_index = game_index.clone();
    for idea in generated_idea_ids_from_suggestions(&suggestions) {
        gate_index.ideas.insert(idea);
    }
    enforce_strict_feature_card_gate(&map, &cards, tag, prefix, Some(&gate_index))?;
    let mut changed_files =
        apply_intent_suggestions_to_mod(&mod_root, &suggestions, prefix, tag)?.changed_files;
    changed_files.extend(apply_feature_cards_to_mod_with_index(
        &mod_root,
        &cards,
        tag,
        prefix,
        Some(&game_index),
    )?);
    changed_files.sort();
    changed_files.dedup();
    run_post_apply_checks(&mod_root, &map, Some(&game_index), Some(&input))?;
    let report = apply_decision_intent_report_json(
        &card_text,
        &intent_text,
        context,
        tag,
        prefix,
        &target_decision_title,
        &suggestions,
        &changed_files,
    );
    write_or_print(&report, value(&map, "output"))?;
    Ok(())
}

pub(crate) fn cmd_apply_event_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if !map.flags.contains("final-check") {
        return Err("apply-event-intent writes mod files and requires --final-check".to_string());
    }
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let game_root = normalize_path(
        value(&map, "game-root")
            .ok_or_else(|| "apply-event-intent requires --game-root".to_string())?,
    )?;
    let dependency_mods = dependency_mod_roots_for_edited_mod(&map, &mod_root, true)?;
    let game_index = build_game_index_with_mod_paths(&game_root, &dependency_mods)?;
    enforce_tag_request_contract(&map, tag, Some(&game_index))?;

    let card_text = read_text_document(&input)?;
    let mut cards = parse_cards(&card_text, &["事件"]);
    let parse_errors = event_card_command_parse_errors(&card_text, &cards);
    if !parse_errors.is_empty() {
        return Err(format!(
            "event intent apply blocked malformed event cards:\n{}",
            parse_errors.join("\n")
        ));
    }
    let intent_text = focus_intent_text_from_args(&map)?;
    let intent = normalize_llm_intent_text(&intent_text);
    let requested_context =
        normalize_intent_context(value(&map, "kind").or_else(|| value(&map, "context")))?;
    let context = effective_compile_intent_context(&intent, requested_context, Some(&game_index));
    let suggestions = compile_intent_suggestions(&intent, context, Some(&game_index), prefix)?;
    let mut errors =
        unresolved_suggestion_errors_with_index("event intent", &suggestions, Some(&game_index));
    errors.extend(context_suggestion_errors(
        "event intent",
        context,
        &suggestions,
    ));
    errors.extend(missing_code_index_category_errors(
        "event intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(unindexed_suggestion_errors(
        "event intent",
        &suggestions,
        &game_index,
    ));
    errors.extend(apply_intent_patch_plan_errors(&suggestions));
    errors.sort();
    errors.dedup();
    if !errors.is_empty() {
        let json = compile_intent_json(
            &intent_text,
            &intent,
            context,
            true,
            Some(&game_index),
            &suggestions,
            &errors,
            prefix,
            Some(tag),
        );
        write_or_print(&json, value(&map, "output"))?;
        return Err("event intent apply blocked unresolved or unindexed HOI4 code".to_string());
    }

    let event_target = inject_intent_effects_into_event_cards(
        &mut cards,
        &suggestions,
        value(&map, "event-title"),
        value(&map, "option").unwrap_or("A"),
        map.flags.contains("hidden-effect") || map.flags.contains("hidden"),
    )?;
    let (chain_index, planned_ids) =
        build_event_chain_index_for_mod_with_ids(&mod_root, &cards, prefix)?;
    let mut gate_index = game_index.clone();
    for idea in generated_idea_ids_from_suggestions(&suggestions) {
        gate_index.ideas.insert(idea);
    }
    enforce_strict_event_card_gate_with_chain(
        &map,
        &cards,
        Some(&gate_index),
        Some(&chain_index),
        Some(&planned_ids),
    )?;
    let mut changed_files =
        apply_intent_suggestions_to_mod(&mod_root, &suggestions, prefix, tag)?.changed_files;
    changed_files.extend(apply_event_cards_to_mod_with_index(
        &mod_root,
        &cards,
        tag,
        prefix,
        Some(&game_index),
    )?);
    changed_files.sort();
    changed_files.dedup();
    run_post_apply_checks(&mod_root, &map, Some(&game_index), Some(&input))?;
    let report = apply_event_intent_report_json(
        &card_text,
        &intent_text,
        context,
        tag,
        prefix,
        &event_target,
        &suggestions,
        &changed_files,
    );
    write_or_print(&report, value(&map, "output"))?;
    Ok(())
}

pub(crate) fn intent_text_from_args(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "intent")) {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_text_document(&normalize_path(input)?);
    }
    Err("compile-intent requires --text or --input".to_string())
}

pub(crate) fn focus_intent_text_from_args(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "intent")) {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "intent-input") {
        return read_utf8_lossy(&normalize_path(input)?);
    }
    Err("apply-focus-intent requires --text, --intent, or --intent-input".to_string())
}

pub(crate) fn effective_compile_intent_context(
    intent: &str,
    requested_context: &'static str,
    game_index: Option<&GameIndex>,
) -> &'static str {
    let mut context = if requested_context == "auto" {
        infer_intent_context(intent)
    } else {
        requested_context
    };
    if game_index
        .and_then(|index| dynamic_modifier_id_from_intent(intent, index))
        .is_some()
        && matches!(context, "auto" | "idea")
    {
        context = "effect";
    }
    if game_index.is_some_and(|index| idea_replacement_intent(intent, index).is_some())
        && matches!(context, "auto" | "idea")
    {
        context = "effect";
    }
    if model_normalized_intent(intent).is_some() && matches!(context, "auto" | "idea") {
        context = "effect";
    }
    if add_idea_intent(intent).is_some() && matches!(context, "auto" | "idea") {
        context = "effect";
    }
    context
}

pub(crate) fn normalize_llm_intent_text(text: &str) -> String {
    let without_comments = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = without_comments.trim();
    if let Some((key, value)) = split_field(trimmed) {
        let normalized_key = key.trim().to_ascii_lowercase();
        if matches!(
            normalized_key.as_str(),
            "llm" | "ai" | "intent" | "intention"
        ) || matches!(key.trim(), "意图" | "效果意图" | "代码意图")
        {
            return value.trim().to_string();
        }
    }
    trimmed.to_string()
}

pub(crate) fn normalize_intent_context(value: Option<&str>) -> Result<&'static str, String> {
    match value.unwrap_or("idea") {
        "idea" | "national-spirit" | "national_spirit" | "modifier" | "idea-modifier"
        | "idea_modifier" => Ok("idea"),
        "effect" | "country-effect" | "country_effect" | "decision" | "country" => Ok("effect"),
        "trigger" | "condition" | "条件" => Ok("trigger"),
        "state-effect" | "state_effect" | "state" => Ok("state_effect"),
        "auto" => Ok("auto"),
        other => Err(format!(
            "unknown intent context `{other}`; use idea|effect|trigger|state-effect|auto"
        )),
    }
}

pub(crate) fn infer_intent_context(intent: &str) -> &'static str {
    if semantic_idea_modifier(intent).is_some() {
        return "idea";
    }
    let trimmed = intent.trim();
    if trimmed.starts_with("完成国策")
        || trimmed.starts_with("拥有民族精神")
        || trimmed.contains("和平")
        || trimmed.contains("无战争")
        || trimmed.contains("不在战争")
        || trimmed.contains("非战争")
        || trimmed.contains("战争中")
        || trimmed.contains("正在战争")
    {
        return "trigger";
    }
    if state_resource_effect(trimmed).is_some()
        || contains_any(
            trimmed,
            &[
                "基础设施",
                "基建",
                "防空",
                "船坞",
                "炼油",
                "合成油",
                "民用工厂",
                "民工",
                "军用工厂",
                "军工",
                "添加核心",
                "获得核心",
                "移除核心",
            ],
        )
    {
        return "state_effect";
    }
    if contains_any(
        trimmed,
        &[
            "政治点",
            "政治力量",
            "海军经验",
            "陆军经验",
            "空军经验",
            "触发事件",
            "触发国家事件",
            "触发新闻",
            "设置旗标",
            "设置国家旗标",
            "添加民族精神",
            "获得民族精神",
            "移除民族精神",
        ],
    ) {
        return "effect";
    }
    "idea"
}

pub(crate) fn compile_intent_suggestions(
    intent: &str,
    context: &str,
    index: Option<&GameIndex>,
    generation_prefix: &str,
) -> Result<Vec<Suggestion>, String> {
    let mut suggestions = Vec::new();
    if let Some(index) = index {
        suggestions.extend(model_normalized_intent_suggestions(
            intent,
            index,
            generation_prefix,
        ));
        if !suggestions.is_empty() {
            dedupe_suggestions(&mut suggestions);
            return Ok(suggestions);
        }
        suggestions.extend(dynamic_modifier_intent_suggestions(intent, index));
        if !suggestions.is_empty() {
            dedupe_suggestions(&mut suggestions);
            return Ok(suggestions);
        }
        suggestions.extend(idea_replacement_intent_suggestions(
            intent,
            index,
            generation_prefix,
        ));
        if !suggestions.is_empty() {
            dedupe_suggestions(&mut suggestions);
            return Ok(suggestions);
        }
        suggestions.extend(add_idea_intent_suggestions(
            intent,
            index,
            generation_prefix,
        ));
        if !suggestions.is_empty() {
            dedupe_suggestions(&mut suggestions);
            return Ok(suggestions);
        }
    }
    match context {
        "idea" => suggestions.extend(suggest_common("idea", intent, None, None, None, None)),
        "effect" => suggestions.extend(suggest_common("effect", intent, None, None, None, None)),
        "state_effect" => {
            suggestions.extend(
                suggest_common("effect", intent, None, None, None, None)
                    .into_iter()
                    .map(|mut suggestion| {
                        if suggestion.kind == "country_effect" {
                            suggestion.kind = "state_effect_candidate".to_string();
                            if suggestion.note.is_empty() {
                                suggestion.note =
                                    "Requested state-effect context; verify scope before writing."
                                        .to_string();
                            }
                        }
                        suggestion
                    }),
            );
        }
        "trigger" => {
            for raw in split_cn_list(intent) {
                suggestions.extend(suggest_trigger(raw));
            }
        }
        "auto" => {
            suggestions.extend(suggest_common("idea", intent, None, None, None, None));
            for raw in split_cn_list(intent) {
                suggestions.extend(suggest_trigger(raw));
            }
            dedupe_suggestions(&mut suggestions);
        }
        _ => return Err(format!("unknown intent context `{context}`")),
    }
    Ok(suggestions)
}

pub(crate) fn dynamic_modifier_intent_suggestions(
    intent: &str,
    index: &GameIndex,
) -> Vec<Suggestion> {
    let Some(dynamic_modifier) = dynamic_modifier_id_from_intent(intent, index) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut pushed_tooltip = false;
    let mut sources = Vec::new();
    let mut unresolved = Vec::new();
    for raw in split_cn_list(intent) {
        let segment = raw.trim();
        if segment.is_empty() || segment == dynamic_modifier {
            continue;
        }
        let cleaned = dynamic_modifier_effect_segment(segment, &dynamic_modifier, index);
        if cleaned.trim().is_empty() {
            continue;
        }
        let Some(value) = dynamic_modifier_effect_value(cleaned) else {
            unresolved.push(cleaned.to_string());
            continue;
        };
        let Some(effect) =
            dynamic_modifier_change_effect_for_segment(&dynamic_modifier, cleaned, index)
        else {
            unresolved.push(cleaned.to_string());
            continue;
        };
        if !pushed_tooltip {
            lines.push(format!("custom_effect_tooltip = {dynamic_modifier}_tt"));
            pushed_tooltip = true;
        }
        lines.push(format!(
            "set_temp_variable = {{ temp_{dynamic_modifier} = {} }}",
            fmt_float(value)
        ));
        lines.push(format!("{effect} = yes"));
        sources.push(cleaned.to_string());
    }
    if !lines.is_empty() {
        return vec![Suggestion::new(
            "country_effect",
            &lines.join("\n"),
            &format!("{dynamic_modifier}: {}", sources.join("；")),
            "Variable-driven dynamic modifier update; do not convert this into a national spirit or raw modifier block.",
        )];
    }
    let unresolved_text = if unresolved.is_empty() {
        intent.trim().to_string()
    } else {
        unresolved.join("；")
    };
    vec![Suggestion::new(
        "raw_effect",
        &unresolved_text,
        &unresolved_text,
        &format!(
            "Recognized dynamic modifier `{dynamic_modifier}`, but no verified change_* scripted effect slot matched the requested effect; do not create a national spirit fallback."
        ),
    )]
}

pub(crate) fn dynamic_modifier_id_from_intent(intent: &str, index: &GameIndex) -> Option<String> {
    let mut matches = Vec::new();
    for (name, ids) in &index.dynamic_modifier_names {
        if !name.is_empty() && intent.contains(name) {
            for id in ids {
                matches.push((name.len(), id.clone()));
            }
        }
    }
    for id in &index.dynamic_modifiers {
        if intent.contains(id) {
            matches.push((id.len(), id.clone()));
        }
    }
    matches
        .into_iter()
        .max_by_key(|(len, id)| (*len, id.len()))
        .map(|(_, id)| id)
}

pub(crate) fn dynamic_modifier_effect_segment<'a>(
    segment: &'a str,
    dynamic_modifier: &str,
    index: &GameIndex,
) -> &'a str {
    let mut out = segment.trim();
    for name in index
        .dynamic_modifier_names
        .iter()
        .filter_map(|(name, ids)| ids.contains(dynamic_modifier).then_some(name.as_str()))
        .chain(std::iter::once(dynamic_modifier))
    {
        out = out.trim_start_matches(name).trim();
    }
    out.trim_start_matches("的效果")
        .trim_start_matches("效果")
        .trim_start_matches(':')
        .trim_start_matches('：')
        .trim()
}

pub(crate) fn dynamic_modifier_effect_value(segment: &str) -> Option<f64> {
    parse_percent(segment).or_else(|| {
        parse_last_number_with_sign(segment).map(|(mut value, explicit_sign)| {
            if !explicit_sign && value > 0.0 && has_negative_direction(segment) {
                value = -value;
            }
            value
        })
    })
}

pub(crate) fn dynamic_modifier_change_effect_for_segment(
    dynamic_modifier: &str,
    segment: &str,
    index: &GameIndex,
) -> Option<String> {
    let scoped_prefix = format!("{dynamic_modifier}|");
    let mut matches = Vec::new();
    for (label, effects) in &index.dynamic_modifier_effect_tooltips {
        let Some(clean_label) = label.strip_prefix(&scoped_prefix) else {
            continue;
        };
        if clean_label.is_empty() || !segment.contains(clean_label) {
            continue;
        }
        for effect in effects {
            if effect
                .strip_prefix("change_")
                .is_some_and(|rest| rest.starts_with(dynamic_modifier))
            {
                matches.push((clean_label.len(), effect.clone()));
            }
        }
    }
    matches
        .into_iter()
        .max_by_key(|(len, effect)| (*len, effect.len()))
        .map(|(_, effect)| effect)
}

#[derive(Clone)]
pub(crate) struct IdeaReplacementIntent {
    pub(crate) old_name: String,
    pub(crate) new_name: String,
    pub(crate) effects: String,
}

#[derive(Clone)]
pub(crate) struct ModelNormalizedIntent {
    pub(crate) intent_type: String,
    pub(crate) idea_name: Option<String>,
    pub(crate) old_idea: Option<String>,
    pub(crate) new_idea: Option<String>,
    pub(crate) dynamic_modifier: Option<String>,
    pub(crate) effects: Option<String>,
}

pub(crate) fn model_normalized_intent(text: &str) -> Option<ModelNormalizedIntent> {
    let fields = model_normalized_fields(text);
    let raw_type = fields
        .get("type")
        .or_else(|| fields.get("intent_type"))
        .or_else(|| fields.get("action"))?
        .trim();
    let intent_type = canonical_model_intent_type(raw_type)?;
    if !matches!(
        intent_type.as_str(),
        "add_national_spirit"
            | "replace_national_spirit"
            | "add_idea"
            | "replace_idea"
            | "swap_ideas"
            | "dynamic_modifier_change"
    ) {
        return None;
    }
    Some(ModelNormalizedIntent {
        intent_type,
        idea_name: fields
            .get("idea_name")
            .or_else(|| fields.get("idea"))
            .or_else(|| fields.get("national_spirit"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        old_idea: fields
            .get("old_idea")
            .or_else(|| fields.get("remove_idea"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        new_idea: fields
            .get("new_idea")
            .or_else(|| fields.get("add_idea"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        dynamic_modifier: fields
            .get("dynamic_modifier")
            .or_else(|| fields.get("dynamic_modifier_name"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        effects: fields
            .get("effects")
            .or_else(|| fields.get("modifiers"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    })
}

pub(crate) fn model_normalized_fields(text: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim().trim_matches(',');
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || matches!(
                trimmed,
                "hoi4skill_intent:" | "hoi4skill-intent:" | "intent:"
            )
        {
            continue;
        }
        let Some((key, value)) = split_model_field(trimmed) else {
            continue;
        };
        let key = canonical_model_field_key(&clean_model_field_token(key));
        let value = clean_model_field_token(value);
        if !key.is_empty() && !value.is_empty() {
            fields.insert(key, value);
        }
    }
    fields
}

pub(crate) fn split_model_field(line: &str) -> Option<(&str, &str)> {
    if let Some((key, value)) = line.split_once(':') {
        return Some((key, value));
    }
    if let Some((key, value)) = line.split_once('：') {
        return Some((key, value));
    }
    line.split_once('=')
}

pub(crate) fn canonical_model_field_key(key: &str) -> String {
    let normalized = clean_model_field_token(key)
        .to_ascii_lowercase()
        .replace(['-', ' '], "_");
    match normalized.as_str() {
        "type" | "intent_type" | "action" | "类型" | "意图" | "动作" | "操作" => {
            "type".to_string()
        }
        "idea_name" | "idea" | "national_spirit" | "spirit" | "民族精神" | "精神" | "精神名称"
        | "民族精神名称" => "idea_name".to_string(),
        "old_idea"
        | "remove_idea"
        | "old_national_spirit"
        | "旧民族精神"
        | "原民族精神"
        | "移除民族精神"
        | "替换前" => "old_idea".to_string(),
        "new_idea"
        | "add_idea"
        | "new_national_spirit"
        | "新民族精神"
        | "目标民族精神"
        | "添加民族精神"
        | "替换后"
        | "替换为" => "new_idea".to_string(),
        "dynamic_modifier"
        | "dynamic_modifier_name"
        | "动态修正"
        | "动态修正名称"
        | "动态modifier"
        | "动态_modifier" => "dynamic_modifier".to_string(),
        "effects" | "effect" | "modifiers" | "modifier" | "效果" | "修正" | "修正效果" => {
            "effects".to_string()
        }
        _ => normalized,
    }
}

pub(crate) fn canonical_model_intent_type(value: &str) -> Option<String> {
    let normalized = clean_model_field_token(value)
        .to_ascii_lowercase()
        .replace(['-', ' '], "_");
    let canonical = match normalized.as_str() {
        "add_national_spirit"
        | "add_idea"
        | "新增民族精神"
        | "增加民族精神"
        | "添加民族精神"
        | "获得民族精神" => "add_national_spirit",
        "replace_national_spirit"
        | "replace_idea"
        | "swap_ideas"
        | "替换民族精神"
        | "更换民族精神"
        | "交换民族精神" => "replace_national_spirit",
        "dynamic_modifier_change"
        | "change_dynamic_modifier"
        | "update_dynamic_modifier"
        | "动态修正变更"
        | "变更动态修正"
        | "修改动态修正"
        | "调整动态修正" => "dynamic_modifier_change",
        _ => return None,
    };
    Some(canonical.to_string())
}

pub(crate) fn clean_model_field_token(value: &str) -> String {
    value
        .trim()
        .trim_matches(',')
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim()
        .trim_matches(['"', '\''])
        .trim()
        .to_string()
}

pub(crate) fn model_normalized_intent_suggestions(
    intent: &str,
    index: &GameIndex,
    generation_prefix: &str,
) -> Vec<Suggestion> {
    let Some(normalized) = model_normalized_intent(intent) else {
        return Vec::new();
    };
    match normalized.intent_type.as_str() {
        "add_national_spirit" | "add_idea" => normalized
            .idea_name
            .as_deref()
            .map(|idea_name| {
                add_idea_suggestions_from_parts(
                    idea_name,
                    normalized.effects.as_deref().unwrap_or(""),
                    index,
                    generation_prefix,
                    "model_normalized",
                )
            })
            .unwrap_or_default(),
        "replace_national_spirit" | "replace_idea" | "swap_ideas" => {
            let Some(old_name) = normalized.old_idea.as_deref() else {
                return Vec::new();
            };
            let Some(new_name) = normalized.new_idea.as_deref() else {
                return Vec::new();
            };
            idea_replacement_suggestions_from_parts(
                old_name,
                new_name,
                normalized.effects.as_deref().unwrap_or(""),
                index,
                generation_prefix,
                "model_normalized",
            )
        }
        "dynamic_modifier_change" => {
            let Some(dynamic_modifier) = normalized.dynamic_modifier.as_deref() else {
                return Vec::new();
            };
            let intent = format!(
                "动态修正：{}\n效果：{}",
                dynamic_modifier,
                normalized.effects.as_deref().unwrap_or("")
            );
            dynamic_modifier_intent_suggestions(&intent, index)
                .into_iter()
                .map(|mut suggestion| {
                    suggestion.source = format!("model_normalized: {}", suggestion.source);
                    suggestion
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn idea_replacement_intent(
    intent: &str,
    _index: &GameIndex,
) -> Option<IdeaReplacementIntent> {
    if !intent.contains("民族精神") || !intent.contains("替换为") {
        return None;
    }
    let after_label = intent
        .split_once("民族精神")
        .map(|(_, rest)| rest)
        .unwrap_or(intent)
        .trim_start_matches([':', '：'])
        .trim();
    let (old_name, after_swap) = after_label.split_once("替换为")?;
    let (new_name, effects) = split_replacement_target_and_effects(after_swap);
    let old_name = clean_intent_idea_name(old_name);
    let new_name = clean_intent_idea_name(new_name);
    if old_name.is_empty() || new_name.is_empty() {
        return None;
    }
    Some(IdeaReplacementIntent {
        old_name,
        new_name,
        effects: effects.trim().to_string(),
    })
}

pub(crate) fn idea_replacement_intent_suggestions(
    intent: &str,
    index: &GameIndex,
    generation_prefix: &str,
) -> Vec<Suggestion> {
    let Some(parsed) = idea_replacement_intent(intent, index) else {
        return Vec::new();
    };
    idea_replacement_suggestions_from_parts(
        &parsed.old_name,
        &parsed.new_name,
        &parsed.effects,
        index,
        generation_prefix,
        "natural_language",
    )
}

pub(crate) fn idea_replacement_suggestions_from_parts(
    old_name: &str,
    new_name: &str,
    effects: &str,
    index: &GameIndex,
    generation_prefix: &str,
    source_kind: &str,
) -> Vec<Suggestion> {
    let old_id = resolve_idea_name_or_placeholder(old_name, index);
    let modifier_lines = idea_modifier_lines_from_effects(effects);
    let new_matches = idea_ids_for_name(new_name, index);
    let (new_id, generated_definition) = if new_matches.len() == 1 {
        (new_matches[0].clone(), None)
    } else if new_matches.is_empty() && !modifier_lines.is_empty() {
        let generated_id = generated_idea_id(generation_prefix, new_name, index);
        let definition = render_generated_idea_definition(&generated_id, &modifier_lines);
        (generated_id, Some(definition))
    } else {
        (format!("<idea id for {new_name}>"), None)
    };
    let mut note = "National spirit replacement uses swap_ideas; new-spirit modifiers belong in common/ideas, not inside completion_reward.".to_string();
    if !modifier_lines.is_empty() {
        note.push_str(" New idea modifier draft: ");
        note.push_str(&modifier_lines.join("; "));
    }
    if generated_definition.is_some() {
        note.push_str(" The target national spirit is not indexed, so this intent includes a generated common/ideas definition because the user supplied explicit effects.");
    }
    let code = format!("swap_ideas = {{\n\tremove_idea = {old_id}\n\tadd_idea = {new_id}\n}}");
    let mut suggestions = vec![Suggestion::new(
        "country_effect",
        &code,
        &format!("{source_kind}: replace national spirit {old_name} -> {new_name}"),
        &note,
    )];
    if let Some(definition) = generated_definition {
        suggestions.push(Suggestion::new(
            "idea_definition",
            &definition,
            &format!("{source_kind}: create national spirit {new_name}"),
            "Generated only because the user explicitly supplied the new national spirit's effects; write this to common/ideas and localisation before relying on swap_ideas.",
        ));
        suggestions.push(Suggestion::new(
            "localisation_entry",
            &render_generated_idea_localisation(&new_id, new_name),
            &format!("{source_kind}: localise national spirit {new_name}"),
            "Generated localisation for the new national spirit; replace the empty _desc with final copywriting before release.",
        ));
    }
    suggestions
}

pub(crate) fn split_replacement_target_and_effects(text: &str) -> (&str, &str) {
    for marker in ["效果为", "效果：", "效果:", "效果"] {
        if let Some((name, effects)) = text.split_once(marker) {
            return (name.trim(), effects.trim());
        }
    }
    (text.trim(), "")
}

#[derive(Clone)]
pub(crate) struct AddIdeaIntent {
    pub(crate) idea_name: String,
    pub(crate) effects: String,
}

pub(crate) fn add_idea_intent(intent: &str) -> Option<AddIdeaIntent> {
    let marker = ["增加民族精神", "添加民族精神", "获得民族精神"]
        .iter()
        .find(|marker| intent.contains(**marker))?;
    let (_, after_marker) = intent.split_once(marker)?;
    let after_marker = after_marker.trim_start_matches([':', '：']).trim();
    let (idea_name, effects) = split_added_idea_name_and_effects(after_marker)?;
    let idea_name = clean_intent_idea_name(&idea_name);
    if idea_name.is_empty() {
        return None;
    }
    Some(AddIdeaIntent { idea_name, effects })
}

pub(crate) fn split_added_idea_name_and_effects(text: &str) -> Option<(String, String)> {
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let first = *lines.first()?;
    if let Some((name, rest)) = first.split_once([':', '：']) {
        let mut effect_parts = Vec::new();
        if !rest.trim().is_empty() {
            effect_parts.push(rest.trim().to_string());
        }
        effect_parts.extend(lines.iter().skip(1).map(|line| (*line).to_string()));
        return Some((name.trim().to_string(), effect_parts.join("；")));
    }
    Some((
        first.trim().to_string(),
        lines
            .iter()
            .skip(1)
            .map(|line| (*line).to_string())
            .collect::<Vec<_>>()
            .join("；"),
    ))
}

pub(crate) fn add_idea_intent_suggestions(
    intent: &str,
    index: &GameIndex,
    generation_prefix: &str,
) -> Vec<Suggestion> {
    let Some(parsed) = add_idea_intent(intent) else {
        return Vec::new();
    };
    add_idea_suggestions_from_parts(
        &parsed.idea_name,
        &parsed.effects,
        index,
        generation_prefix,
        "natural_language",
    )
}

pub(crate) fn add_idea_suggestions_from_parts(
    idea_name: &str,
    effects: &str,
    index: &GameIndex,
    generation_prefix: &str,
    source_kind: &str,
) -> Vec<Suggestion> {
    let matches = idea_ids_for_name(idea_name, index);
    let modifier_lines = idea_modifier_lines_from_effects(effects);
    let (idea_id, generated_definition) = if matches.len() == 1 {
        (matches[0].clone(), None)
    } else if matches.is_empty() && !modifier_lines.is_empty() {
        let generated_id = generated_idea_id(generation_prefix, idea_name, index);
        let definition = render_generated_idea_definition(&generated_id, &modifier_lines);
        (generated_id, Some(definition))
    } else {
        (format!("<idea id for {idea_name}>"), None)
    };
    let mut suggestions = vec![Suggestion::new(
        "country_effect",
        &format!("add_ideas = {idea_id}"),
        &format!("{source_kind}: add national spirit {idea_name}"),
        if generated_definition.is_some() {
            "Add the generated national spirit in common/ideas before using add_ideas."
        } else {
            ""
        },
    )];
    if let Some(definition) = generated_definition {
        suggestions.push(Suggestion::new(
            "idea_definition",
            &definition,
            &format!("{source_kind}: create national spirit {idea_name}"),
            "Generated only because the user explicitly supplied the new national spirit's effects; write this to common/ideas and localisation before relying on add_ideas.",
        ));
        suggestions.push(Suggestion::new(
            "localisation_entry",
            &render_generated_idea_localisation(&idea_id, idea_name),
            &format!("{source_kind}: localise national spirit {idea_name}"),
            "Generated localisation for the new national spirit; replace the empty _desc with final copywriting before release.",
        ));
    }
    suggestions
}

pub(crate) fn clean_intent_idea_name(text: &str) -> String {
    text.trim()
        .trim_start_matches([':', '：'])
        .trim_matches(['。', '，', ',', '；', ';'])
        .trim()
        .to_string()
}

pub(crate) fn resolve_idea_name_or_placeholder(name: &str, index: &GameIndex) -> String {
    let ids = idea_ids_for_name(name, index);
    if ids.len() == 1 {
        return ids[0].clone();
    }
    format!("<idea id for {name}>")
}

pub(crate) fn idea_ids_for_name(name: &str, index: &GameIndex) -> Vec<String> {
    if index.ideas.contains(name) {
        return vec![name.to_string()];
    }
    index
        .idea_names
        .get(name)
        .map(|ids| ids.iter().cloned().collect())
        .unwrap_or_default()
}

pub(crate) fn generated_idea_id(prefix: &str, name: &str, index: &GameIndex) -> String {
    let prefix = sanitize_identifier_part(prefix, "mod");
    let mut slug = slugify(name, "");
    if slug.is_empty() {
        slug = format!("spirit_{}", stable_identifier_hash(name));
    }
    let base = ensure_idea_id_suffix(&format!("{prefix}_{slug}"));
    if !index.ideas.contains(&base) {
        return base;
    }
    for idx in 2..1000 {
        let candidate = ensure_idea_id_suffix(&format!("{prefix}_{slug}_{idx}"));
        if !index.ideas.contains(&candidate) {
            return candidate;
        }
    }
    ensure_idea_id_suffix(&format!("{prefix}_spirit_{}", stable_identifier_hash(name)))
}

pub(crate) fn stable_identifier_hash(value: &str) -> String {
    let mut hash: u32 = 2166136261;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

pub(crate) fn render_generated_idea_definition(id: &str, modifier_lines: &[String]) -> String {
    let mut out = String::new();
    out.push_str("ideas = {\n\tcountry = {\n");
    out.push_str(&format!("\t\t{id} = {{\n"));
    out.push_str("\t\t\tpicture = generic_production_bonus\n");
    out.push_str("\t\t\tmodifier = {\n");
    for line in modifier_lines {
        out.push_str(&format!("\t\t\t\t{line}\n"));
    }
    out.push_str("\t\t\t}\n");
    out.push_str("\t\t}\n");
    out.push_str("\t}\n}\n");
    out
}

pub(crate) fn render_generated_idea_localisation(id: &str, name: &str) -> String {
    format!(
        "l_simp_chinese:\n {id}:0 \"{}\"\n {id}_desc:0 \"\"\n",
        name.replace('"', "\\\"")
    )
}

pub(crate) fn idea_modifier_lines_from_effects(effects: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for raw in split_cn_list(effects) {
        let count_before = lines.len();
        if raw.contains("稳定") || raw.to_ascii_lowercase().contains("stability") {
            if let Some(v) = parse_percent(raw) {
                lines.push(format!("stability_factor = {}", fmt_float(v)));
            }
        }
        if raw.contains("战争支持")
            || raw.contains("战争支援")
            || raw.to_ascii_lowercase().contains("war support")
            || raw.to_ascii_lowercase().contains("war_support")
        {
            if let Some(v) = parse_percent(raw) {
                lines.push(format!("war_support_factor = {}", fmt_float(v)));
            }
        }
        if raw.contains("政治点")
            || raw.contains("政治力量")
            || raw.contains("政治点数")
            || raw.to_ascii_lowercase().contains("political power")
            || raw.to_ascii_lowercase().contains("political_power")
        {
            if let Some((mut value, explicit_sign)) = parse_last_number_with_sign(raw) {
                if !explicit_sign && value > 0.0 && has_negative_direction(raw) {
                    value = -value;
                }
                lines.push(format!("political_power_gain = {}", fmt_float(value)));
            }
        }
        if lines.len() == count_before {
            if let Some(suggestion) = semantic_idea_modifier(raw) {
                lines.push(suggestion.code);
            }
        }
    }
    lines
}

pub(crate) fn dedupe_suggestions(suggestions: &mut Vec<Suggestion>) {
    let mut seen = BTreeSet::new();
    suggestions.retain(|suggestion| {
        seen.insert(format!(
            "{}\n{}\n{}",
            suggestion.kind, suggestion.code, suggestion.source
        ))
    });
}

pub(crate) fn compile_intent_json(
    input: &str,
    intent: &str,
    context: &str,
    strict_code_index: bool,
    index: Option<&GameIndex>,
    suggestions: &[Suggestion],
    errors: &[String],
    generation_prefix: &str,
    generation_tag: Option<&str>,
) -> String {
    let final_blockers = compile_intent_final_blockers(strict_code_index, index, errors);
    let indexed = index.is_some();
    let allowed_kinds = context_allowed_suggestion_kinds(context)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.intent_compile.v1\",\n  \"input\": {},\n  \"intent\": {},\n  \"context\": {},\n  \"allowed_kinds\": {},\n  \"strict_code_index\": {},\n  \"code_index_checked\": {},\n  \"ok\": {},\n  \"safety\": {},\n  \"effect_strategies\": {},\n  \"suggestions\": {},\n  \"patch_plan\": {},\n  \"index_checks\": {},\n  \"errors\": {},\n  \"anti_hallucination_rule\": {}\n}}\n",
        json_str(input),
        json_str(intent),
        json_str(context),
        json_array(&allowed_kinds),
        json_bool(strict_code_index),
        json_bool(indexed),
        json_bool(errors.is_empty()),
        suggestions_safety_json_with_extra_blockers(suggestions, &final_blockers),
        json_array(&suggestion_effect_strategies(suggestions)),
        suggestions_json(suggestions),
        compile_intent_patch_plan_json(
            suggestions,
            generation_prefix,
            generation_tag,
            &final_blockers
        ),
        compile_intent_index_checks_json(suggestions, index),
        json_array(errors),
        json_str("AI may provide intent only; final Clausewitz code must come from mapped suggestions and must fail if safety or code-index checks report blockers.")
    )
}

pub(crate) fn compile_intent_final_blockers(
    strict_code_index: bool,
    index: Option<&GameIndex>,
    errors: &[String],
) -> Vec<String> {
    let mut final_blockers = errors.to_vec();
    if index.is_none() {
        final_blockers.push(
            "code index was not checked; this is a draft mapping only and cannot be final Clausewitz code"
                .to_string(),
        );
    } else if !strict_code_index {
        final_blockers.push(
            "strict code index was not requested; rerun with --strict-code-index before final code"
                .to_string(),
        );
    }
    final_blockers
}

pub(crate) fn compile_intent_patch_plan_json(
    suggestions: &[Suggestion],
    generation_prefix: &str,
    generation_tag: Option<&str>,
    errors: &[String],
) -> String {
    let prefix = sanitize_identifier_part(generation_prefix, "mod");
    let idea_path = format!("common/ideas/{prefix}_ideas.txt");
    let localisation_file = generation_tag
        .map(target_localisation_file_name)
        .unwrap_or_else(|| format!("{prefix}_l_simp_chinese.yml"));
    let localisation_path = format!("localisation/simp_chinese/{localisation_file}");
    let mut planned_files = BTreeSet::new();
    let mut generated_symbols = BTreeSet::new();
    let mut items = Vec::new();

    for suggestion in suggestions {
        let (destination, target, requires_parent_context, symbols) = match suggestion.kind.as_str()
        {
            "country_effect" | "country_effect_candidate" => (
                "effect_context".to_string(),
                "caller effect block, such as focus completion_reward, decision complete_effect, or event option effect".to_string(),
                true,
                idea_symbols_from_effect_code(&suggestion.code),
            ),
            "idea_definition" => {
                planned_files.insert(idea_path.clone());
                let ids = idea_ids_from_idea_definition_code(&suggestion.code);
                generated_symbols.extend(ids.iter().cloned());
                (
                    "file_append".to_string(),
                    idea_path.clone(),
                    false,
                    ids,
                )
            }
            "localisation_entry" => {
                planned_files.insert(localisation_path.clone());
                (
                    "file_append".to_string(),
                    localisation_path.clone(),
                    false,
                    localisation_keys_from_entry(&suggestion.code),
                )
            }
            "idea_modifier" | "idea_modifier_candidate" => (
                "idea_modifier_block".to_string(),
                "modifier block inside an existing or generated idea".to_string(),
                true,
                Vec::new(),
            ),
            "trigger" | "trigger_candidate" => (
                "trigger_context".to_string(),
                "caller trigger block, such as available, visible, or option trigger".to_string(),
                true,
                Vec::new(),
            ),
            "state_effect_candidate" => (
                "state_effect_context".to_string(),
                "state-scoped effect block; caller must provide a state scope".to_string(),
                true,
                Vec::new(),
            ),
            _ => (
                "unresolved_mapping".to_string(),
                "blocked until the raw intent is mapped to a verified HOI4 code kind".to_string(),
                true,
                Vec::new(),
            ),
        };
        items.push(format!(
            "{{\"kind\": {}, \"source\": {}, \"destination\": {}, \"target\": {}, \"requires_parent_context\": {}, \"effect_strategy\": {}, \"symbols\": {}, \"code\": {}, \"note\": {}}}",
            json_str(&suggestion.kind),
            json_str(&suggestion.source),
            json_str(&destination),
            json_str(&target),
            json_bool(requires_parent_context),
            json_str(suggestion_effect_strategy(suggestion)),
            json_array(&symbols),
            json_str(&suggestion.code),
            json_str(&suggestion.note)
        ));
    }

    let generated_symbols_json = generated_symbols
        .iter()
        .map(|symbol| {
            format!(
                "{{\"kind\": {}, \"symbol\": {}, \"source\": {}}}",
                json_str("idea"),
                json_str(symbol),
                json_str("idea_definition")
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let planned_file_values = planned_files.into_iter().collect::<Vec<_>>();
    format!(
        "{{\"schema\": \"hoi4skill.intent_patch_plan.v1\", \"mode\": \"dry_run\", \"prefix\": {}, \"tag\": {}, \"can_apply\": {}, \"effect_strategies\": {}, \"planned_files\": {}, \"generated_symbols\": [{}], \"items\": [{}], \"blockers\": {}, \"rule\": {}}}",
        json_str(&prefix),
        generation_tag.map(json_str).unwrap_or_else(|| "null".to_string()),
        json_bool(errors.is_empty()),
        json_array(&suggestion_effect_strategies(suggestions)),
        json_array(&planned_file_values),
        generated_symbols_json,
        items.join(", "),
        json_array(errors),
        json_str("Patch plans are declarative: write operations must follow these destinations and must not invent files, symbols, or parent contexts.")
    )
}

pub(crate) fn author_intent_systems(
    input: &str,
    intent: &str,
    suggestions: &[Suggestion],
    index: Option<&GameIndex>,
) -> Vec<String> {
    let mut systems = BTreeSet::new();
    systems.extend(author_intent_systems_from_text(input));
    let intent_lower = intent.to_ascii_lowercase();
    if contains_any(intent, &["动态修正", "动态modifier", "动态 modifier"])
        || intent_lower.contains("dynamic modifier")
        || model_normalized_intent(intent)
            .as_ref()
            .is_some_and(|normalized| normalized.intent_type == "dynamic_modifier_change")
        || index
            .and_then(|index| dynamic_modifier_id_from_intent(intent, index))
            .is_some()
    {
        systems.insert("dynamic_modifier".to_string());
    }
    if add_idea_intent(intent).is_some()
        || model_normalized_intent(intent).is_some_and(|normalized| {
            matches!(
                normalized.intent_type.as_str(),
                "add_national_spirit"
                    | "replace_national_spirit"
                    | "add_idea"
                    | "replace_idea"
                    | "swap_ideas"
            )
        })
        || suggestions.iter().any(|suggestion| {
            matches!(
                suggestion_effect_strategy(suggestion),
                "add_existing_or_generated_national_spirit"
                    | "create_national_spirit_definition"
                    | "replace_national_spirit_with_swap_ideas"
            )
        })
    {
        systems.insert("national_spirit".to_string());
    }
    if suggestions
        .iter()
        .any(|suggestion| suggestion.kind == "localisation_entry")
    {
        systems.insert("localisation".to_string());
    }
    if systems.is_empty() {
        systems.insert("effect_intent".to_string());
    }
    systems.into_iter().collect()
}

pub(crate) fn author_intent_systems_from_text(input: &str) -> Vec<String> {
    let lower = input.to_ascii_lowercase();
    let mut systems = BTreeSet::new();
    if contains_any(input, &["国策", "焦点"]) || lower.contains("focus") {
        systems.insert("focus".to_string());
    }
    if contains_any(input, &["事件", "新闻"]) || lower.contains("event") {
        systems.insert("event".to_string());
    }
    if input.contains("决议") || lower.contains("decision") {
        systems.insert("decision".to_string());
    }
    if contains_any(input, &["动态修正", "动态modifier", "动态 modifier"])
        || lower.contains("dynamic modifier")
    {
        systems.insert("dynamic_modifier".to_string());
    }
    if contains_any(input, &["民族精神", "national spirit"]) {
        systems.insert("national_spirit".to_string());
    }
    if input.contains('【')
        || input.contains('】')
        || contains_any(input, &["本地化", "颜色", "图标", "国旗"])
        || lower.contains("localisation")
        || lower.contains("localization")
    {
        systems.insert("localisation".to_string());
    }
    systems.into_iter().collect()
}

pub(crate) fn author_intent_effect_text(input: &str, systems: &[String]) -> String {
    if systems.iter().any(|system| system == "dynamic_modifier") {
        return input.trim().to_string();
    }
    let mut lines = Vec::new();
    let mut in_effect = false;
    for raw in input.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = split_field(trimmed) {
            let normalized_key = key.trim();
            if author_intent_is_parent_metadata_key(normalized_key) {
                in_effect = false;
                continue;
            }
            if normalized_key.starts_with("效果") || normalized_key.eq_ignore_ascii_case("effect")
            {
                in_effect = true;
                if !value.trim().is_empty() {
                    lines.push(value.trim().to_string());
                }
                continue;
            }
            if in_effect {
                lines.push(trimmed.to_string());
            }
        } else if in_effect {
            lines.push(trimmed.to_string());
        }
    }
    if lines.is_empty() {
        input.trim().to_string()
    } else {
        lines.join("\n")
    }
}

pub(crate) fn author_intent_is_parent_metadata_key(key: &str) -> bool {
    matches!(
        key,
        "国策"
            | "焦点"
            | "事件"
            | "决议"
            | "标题"
            | "描述"
            | "目标"
            | "命名空间"
            | "分类"
            | "图片"
            | "选项A"
            | "选项B"
            | "选项C"
            | "选项D"
    ) || key.to_ascii_lowercase().contains("title")
        || key.to_ascii_lowercase().contains("desc")
}

pub(crate) fn author_intent_primary_writer(systems: &[String]) -> &'static str {
    if systems.iter().any(|system| system == "focus") {
        return "apply-focus-intent";
    }
    if systems.iter().any(|system| system == "event") {
        return "apply-event-intent";
    }
    if systems.iter().any(|system| system == "decision") {
        return "apply-decision-intent";
    }
    if systems.iter().any(|system| system == "dynamic_modifier") {
        return "plan-dynamic-modifier-change";
    }
    if systems.iter().any(|system| system == "localisation") {
        return "author-placeholder-plan";
    }
    "apply-intent-patch-plan"
}

pub(crate) fn author_intent_missing_context_blockers(
    map: &ArgMap,
    systems: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    let needs_writer = systems.iter().any(|system| {
        matches!(
            system.as_str(),
            "focus" | "event" | "decision" | "national_spirit" | "effect_intent"
        )
    });
    let parent_systems = systems
        .iter()
        .filter(|system| matches!(system.as_str(), "focus" | "event" | "decision"))
        .count();
    if value(map, "game-root").is_none() {
        blockers.push("game-root missing; run detect-hoi4-path and rerun with --game-root before accepting final code".to_string());
    }
    if !validation_options_from_args(map).strict_code_index {
        blockers.push(
            "strict code index not requested; rerun with --strict-code-index or --final-check"
                .to_string(),
        );
    }
    if needs_writer && value(map, "mod-root").is_none() {
        blockers.push("mod-root missing; writer commands need --mod-root".to_string());
    }
    if needs_writer && value(map, "tag").is_none() {
        blockers.push(
            "tag missing; writer commands need an existing or explicitly authorized country tag"
                .to_string(),
        );
    }
    if parent_systems > 1 {
        blockers.push(
            "request spans multiple parent systems; split into a work package or provide separate focus/event/decision card inputs"
                .to_string(),
        );
    }
    if systems.iter().any(|system| system == "focus")
        && value(map, "focus-input")
            .or_else(|| value(map, "parent-input"))
            .is_none()
    {
        blockers.push(
            "focus parent input missing; save the focus parent template and pass it as --input to apply-focus-intent"
                .to_string(),
        );
    }
    if systems.iter().any(|system| system == "event")
        && value(map, "event-input")
            .or_else(|| value(map, "parent-input"))
            .is_none()
    {
        blockers.push(
            "event parent input missing; save the event parent template and pass it as --input to apply-event-intent"
                .to_string(),
        );
    }
    if systems.iter().any(|system| system == "decision")
        && value(map, "decision-input")
            .or_else(|| value(map, "parent-input"))
            .is_none()
    {
        blockers.push(
            "decision parent input missing; save the decision parent template and pass it as --input to apply-decision-intent"
                .to_string(),
        );
    }
    blockers
}

pub(crate) fn author_intent_execute_blockers(
    map: &ArgMap,
    systems: &[String],
    writer: &str,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if value(map, "game-root").is_none() {
        blockers
            .push("game-root missing; author-intent --execute requires --game-root".to_string());
    }
    if !validation_options_from_args(map).strict_code_index {
        blockers.push(
            "final-check missing; author-intent --execute requires --final-check".to_string(),
        );
    }
    if !matches!(writer, "plan-dynamic-modifier-change") && value(map, "mod-root").is_none() {
        blockers.push("mod-root missing; author-intent --execute needs --mod-root".to_string());
    }
    if matches!(
        writer,
        "apply-focus-intent"
            | "apply-event-intent"
            | "apply-decision-intent"
            | "apply-intent-patch-plan"
    ) && value(map, "tag").is_none()
    {
        blockers.push(
            "tag missing; author-intent --execute needs an existing or explicitly authorized country tag"
                .to_string(),
        );
    }
    let parent_systems = systems
        .iter()
        .filter(|system| matches!(system.as_str(), "focus" | "event" | "decision"))
        .count();
    if parent_systems > 1 {
        blockers.push(
            "request spans multiple parent systems; split it before author-intent --execute"
                .to_string(),
        );
    }
    blockers
}

pub(crate) struct PreparedAuthorIntentFiles {
    pub(crate) intent_path: PathBuf,
    pub(crate) parent_path: Option<PathBuf>,
}

pub(crate) fn prepare_author_intent_execution_files(
    map: &ArgMap,
    input_text: &str,
    intent_text: &str,
    systems: &[String],
) -> Result<PreparedAuthorIntentFiles, String> {
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let output_dir = value(map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .or_else(|| {
            mod_root
                .as_ref()
                .map(|root| root.join(".hoi4skill").join("author_intent"))
        })
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("author_intent"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let intent_path = output_dir.join("intent.txt");
    fs::write(&intent_path, intent_text)
        .map_err(|e| format!("write {}: {e}", intent_path.display()))?;
    let parent_path = if let Some(parent) = author_intent_explicit_parent_input(map, systems) {
        Some(normalize_path(parent)?)
    } else if let Some(system) = author_intent_single_parent_system(systems) {
        let path = output_dir.join(format!("{system}_parent.txt"));
        let content = author_intent_parent_template_content(
            input_text,
            system,
            value(map, "prefix").unwrap_or("mod"),
            value(map, "tag"),
        );
        fs::write(&path, content).map_err(|e| format!("write {}: {e}", path.display()))?;
        Some(path)
    } else {
        None
    };
    Ok(PreparedAuthorIntentFiles {
        intent_path,
        parent_path,
    })
}

pub(crate) fn author_intent_delegated_args_without_text(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--execute" {
            i += 1;
            continue;
        }
        if matches!(
            arg.as_str(),
            "--text" | "--intent" | "--input" | "--intent-input"
        ) {
            i += if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                2
            } else {
                1
            };
            continue;
        }
        out.push(arg.clone());
        i += 1;
    }
    out
}

pub(crate) fn author_intent_push_default_effect_kind(map: &ArgMap, args: &mut Vec<String>) {
    if value(map, "kind").is_none() && value(map, "context").is_none() {
        args.push("--kind".to_string());
        args.push("effect".to_string());
    }
}

pub(crate) fn author_intent_explicit_parent_input<'a>(
    map: &'a ArgMap,
    systems: &[String],
) -> Option<&'a str> {
    value(map, "parent-input").or_else(|| {
        if systems.iter().any(|system| system == "focus") {
            value(map, "focus-input")
        } else if systems.iter().any(|system| system == "event") {
            value(map, "event-input")
        } else if systems.iter().any(|system| system == "decision") {
            value(map, "decision-input")
        } else {
            None
        }
    })
}

pub(crate) fn author_intent_single_parent_system(systems: &[String]) -> Option<&'static str> {
    let parents = systems
        .iter()
        .filter_map(|system| match system.as_str() {
            "focus" => Some("focus"),
            "event" => Some("event"),
            "decision" => Some("decision"),
            _ => None,
        })
        .collect::<Vec<_>>();
    (parents.len() == 1).then_some(parents[0])
}

pub(crate) fn author_intent_parent_template_content(
    input: &str,
    system: &str,
    generation_prefix: &str,
    generation_tag: Option<&str>,
) -> String {
    match system {
        "focus" => {
            let title =
                author_intent_title(input, "focus").unwrap_or_else(|| "<focus-title>".to_string());
            format!("{title}\n")
        }
        "decision" => {
            let title = author_intent_title(input, "decision")
                .unwrap_or_else(|| "<decision-title>".to_string());
            let tag = generation_tag.unwrap_or("<tag>");
            format!("决议：{title}\n目标：{tag}\n分类：国家决议\n效果：\n描述：<description>\n")
        }
        "event" => {
            let title =
                author_intent_title(input, "event").unwrap_or_else(|| "<event-title>".to_string());
            let tag = generation_tag.unwrap_or("<tag>");
            let namespace = sanitize_identifier_part(generation_prefix, "mod");
            format!(
                "事件：{title}\n目标：{tag}\n命名空间：{namespace}\n标题：{title}\n描述：<description>\n选项A：好的。\n效果A：\n"
            )
        }
        _ => input.to_string(),
    }
}

pub(crate) fn author_intent_plan_json(
    input: &str,
    intent: &str,
    context: &str,
    generation_prefix: &str,
    generation_tag: Option<&str>,
    systems: &[String],
    suggestions: &[Suggestion],
    errors: &[String],
    blockers: &[String],
    map: &ArgMap,
    strict_code_index: bool,
    index: Option<&GameIndex>,
) -> String {
    let writer = author_intent_primary_writer(systems);
    let ready = blockers.is_empty() && errors.is_empty();
    let commands = author_intent_next_commands(
        writer,
        systems,
        generation_prefix,
        generation_tag,
        map,
        errors,
    );
    let questions = author_intent_questions(systems, map);
    let parent_templates =
        author_intent_parent_templates_json(input, generation_prefix, generation_tag);
    format!(
        "{{\n  \"schema\": \"hoi4skill.author_intent_plan.v1\",\n  \"input\": {},\n  \"intent\": {},\n  \"systems\": {},\n  \"primary_writer\": {},\n  \"context\": {},\n  \"ok\": {},\n  \"ready_to_execute\": {},\n  \"strict_code_index\": {},\n  \"code_index_checked\": {},\n  \"effect_strategies\": {},\n  \"intent_compile\": {},\n  \"writer_commands\": {},\n  \"parent_templates\": {},\n  \"questions_for_user\": {},\n  \"blockers\": {},\n  \"anti_hallucination_rules\": {}\n}}\n",
        json_str(input),
        json_str(intent),
        json_array(systems),
        json_str(writer),
        json_str(context),
        json_bool(errors.is_empty()),
        json_bool(ready),
        json_bool(strict_code_index),
        json_bool(index.is_some()),
        json_array(&suggestion_effect_strategies(suggestions)),
        compile_intent_json(
            input,
            intent,
            context,
            strict_code_index,
            index,
            suggestions,
            errors,
            generation_prefix,
            generation_tag,
        ),
        json_array(&commands),
        parent_templates,
        json_array(&questions),
        json_array(blockers),
        json_array(&author_intent_anti_hallucination_rules())
    )
}

pub(crate) fn author_intent_next_commands(
    writer: &str,
    systems: &[String],
    generation_prefix: &str,
    generation_tag: Option<&str>,
    map: &ArgMap,
    errors: &[String],
) -> Vec<String> {
    let mod_root = value(map, "mod-root").unwrap_or("<mod-root>");
    let game_root = value(map, "game-root").unwrap_or("<game-root>");
    let tag = generation_tag.unwrap_or("<tag>");
    let prefix = sanitize_identifier_part(generation_prefix, "mod");
    let focus_title = author_intent_title(value(map, "text").unwrap_or(""), "focus")
        .unwrap_or_else(|| "<focus-title>".to_string());
    let decision_title = author_intent_title(value(map, "text").unwrap_or(""), "decision")
        .unwrap_or_else(|| "<decision-title>".to_string());
    let event_title = author_intent_title(value(map, "text").unwrap_or(""), "event")
        .unwrap_or_else(|| "<event-title>".to_string());
    let mut commands = Vec::new();
    commands.push(format!(
        "hoi4skill compile-intent --input <intent.txt> --kind auto --prefix {prefix} --tag {tag} --game-root {game_root} --strict-code-index --output .hoi4skill/intent_compile.json"
    ));
    if systems.iter().any(|system| system == "localisation") {
        commands.push(format!(
            "hoi4skill author-placeholder-plan --input <intent.txt> --game-root {game_root} --output .hoi4skill/placeholder_plan.json"
        ));
    }
    match writer {
        "plan-dynamic-modifier-change" => commands.push(format!(
            "hoi4skill plan-dynamic-modifier-change --input <intent.txt> --game-root {game_root} --output .hoi4skill/dynamic_modifier_plan.json"
        )),
        "apply-focus-intent" => commands.push(format!(
            "hoi4skill apply-focus-intent --input <focus_layout.txt> --intent-input <intent.txt> --mod-root {mod_root} --tag {tag} --prefix {prefix} --game-root {game_root} --final-check --focus-title {focus_title} --output .hoi4skill/applied_focus_intent.json"
        )),
        "apply-event-intent" => commands.push(format!(
            "hoi4skill apply-event-intent --input <event_cards.txt> --intent-input <intent.txt> --mod-root {mod_root} --tag {tag} --prefix {prefix} --game-root {game_root} --final-check --event-title {event_title} --option A --output .hoi4skill/applied_event_intent.json"
        )),
        "apply-decision-intent" => commands.push(format!(
            "hoi4skill apply-decision-intent --input <decision_cards.txt> --intent-input <intent.txt> --mod-root {mod_root} --tag {tag} --prefix {prefix} --game-root {game_root} --final-check --decision-title {decision_title} --output .hoi4skill/applied_decision_intent.json"
        )),
        "author-placeholder-plan" => {}
        _ => commands.push(format!(
            "hoi4skill apply-intent-patch-plan --input <intent.txt> --mod-root {mod_root} --tag {tag} --prefix {prefix} --game-root {game_root} --final-check --output .hoi4skill/applied_intent_patch.json"
        )),
    }
    if !errors.is_empty() {
        commands.push(format!(
            "hoi4skill query-clausewitz-library --query <failed intent or symbol> --library <clausewitz-library> --max-results 6"
        ));
        commands.push(format!(
            "hoi4skill ai-repair-bundle {mod_root} --game-root {game_root} --request <literal user request> --output-dir .hoi4skill/repair_bundle"
        ));
    }
    commands
}

pub(crate) fn author_intent_questions(systems: &[String], map: &ArgMap) -> Vec<String> {
    let mut questions = Vec::new();
    if value(map, "tag").is_none() {
        questions.push(
            "Which existing country tag or verified cosmetic tag should own this content?"
                .to_string(),
        );
    }
    if systems.iter().any(|system| system == "localisation") {
        questions.push("For every unresolved 【placeholder】, should hoi4skill use an existing localisation/sprite tag, create a new asset placeholder, or leave the copy blocked?".to_string());
    }
    if systems.iter().any(|system| system == "dynamic_modifier") {
        questions.push("If no indexed change_* helper exists, provide the scripted_effect mapping instead of allowing an idea fallback.".to_string());
    }
    questions
}

pub(crate) fn author_intent_anti_hallucination_rules() -> Vec<String> {
    vec![
        "AI may describe intent, but Clausewitz code must come from compile-intent suggestions or a parent writer command.".to_string(),
        "Unknown effects, triggers, modifiers, sprites, ideas, event pictures, decision icons, and tags are blockers, not guesses.".to_string(),
        "Dynamic modifiers must use indexed custom_effect_tooltip, set_temp_variable, and change_* scripted effects; do not model them as national spirits.".to_string(),
        "National spirit replacement must use swap_ideas; new spirit modifiers belong in common/ideas plus localisation.".to_string(),
        "Localisation placeholders such as 【国旗】 or colour wrappers must resolve to indexed loc/sprite/cosmetic-tag evidence or ask the user.".to_string(),
    ]
}

pub(crate) fn author_intent_parent_templates_json(
    input: &str,
    generation_prefix: &str,
    generation_tag: Option<&str>,
) -> String {
    let mut templates = Vec::new();
    if contains_any(input, &["国策", "焦点"]) || input.to_ascii_lowercase().contains("focus") {
        let title =
            author_intent_title(input, "focus").unwrap_or_else(|| "<focus-title>".to_string());
        templates.push(format!(
            "{{\"system\": {}, \"path\": {}, \"content\": {}}}",
            json_str("focus"),
            json_str("<focus_layout.txt>"),
            json_str(&format!("{title}\n"))
        ));
    }
    if input.contains("决议") || input.to_ascii_lowercase().contains("decision") {
        let title = author_intent_title(input, "decision")
            .unwrap_or_else(|| "<decision-title>".to_string());
        let tag = generation_tag.unwrap_or("<tag>");
        templates.push(format!(
            "{{\"system\": {}, \"path\": {}, \"content\": {}}}",
            json_str("decision"),
            json_str("<decision_cards.txt>"),
            json_str(&format!(
                "决议：{title}\n目标：{tag}\n分类：<decision-category>\n效果：\n描述：<description>\n"
            ))
        ));
    }
    if contains_any(input, &["事件", "新闻"]) || input.to_ascii_lowercase().contains("event") {
        let title =
            author_intent_title(input, "event").unwrap_or_else(|| "<event-title>".to_string());
        let tag = generation_tag.unwrap_or("<tag>");
        let namespace = sanitize_identifier_part(generation_prefix, "mod");
        templates.push(format!(
            "{{\"system\": {}, \"path\": {}, \"content\": {}}}",
            json_str("event"),
            json_str("<event_cards.txt>"),
            json_str(&format!(
                "事件：{title}\n目标：{tag}\n命名空间：{namespace}\n标题：{title}\n描述：<description>\n图片：GFX_report_event_generic\n选项A：好的。\n效果A：\n"
            ))
        ));
    }
    format!("[{}]", templates.join(", "))
}

pub(crate) fn author_intent_title(input: &str, system: &str) -> Option<String> {
    for line in input.lines() {
        let trimmed = line.trim();
        let Some((key, value)) = split_field(trimmed) else {
            continue;
        };
        let lower_key = key.to_ascii_lowercase();
        let matched = match system {
            "focus" => key.contains("国策") || key.contains("焦点") || lower_key.contains("focus"),
            "decision" => key.contains("决议") || lower_key.contains("decision"),
            "event" => key.contains("事件") || key.contains("新闻") || lower_key.contains("event"),
            _ => false,
        };
        if matched && !value.trim().is_empty() {
            return Some(value.trim().to_string());
        }
    }
    None
}

pub(crate) fn dynamic_modifier_change_plan_json(
    input: &str,
    intent: &str,
    dynamic_modifier: Option<&str>,
    suggestions: &[Suggestion],
    errors: &[String],
    apply_result: Option<&AppliedDynamicModifierChange>,
) -> String {
    let effect_snippets = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.clone())
        .collect::<Vec<_>>();
    let scripted_effects = effect_snippets
        .iter()
        .flat_map(|code| dynamic_modifier_plan_scripted_effects(code))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let temp_variables = dynamic_modifier
        .map(|id| vec![format!("temp_{id}")])
        .unwrap_or_default();
    let rules = vec![
        "Use this plan only in an effect context such as focus completion_reward, decision complete_effect, event option effect, or scripted effect.".to_string(),
        "Do not convert a variable-driven dynamic modifier into add_ideas or a generated national spirit.".to_string(),
        "Do not invent change_* scripted effects or temp_* variable names; every helper must come from the indexed game or dependency mod.".to_string(),
        "If can_apply is false, ask for the intended dynamic modifier/helper mapping or add the scripted_effect implementation first.".to_string(),
    ];
    let changed_files = apply_result
        .map(|result| {
            result
                .changed_files
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let generated_symbols = apply_result
        .map(|result| result.generated_symbols.clone())
        .unwrap_or_default();
    format!(
        "{{\n  \"schema\": \"hoi4skill.dynamic_modifier_change_plan.v1\",\n  \"input\": {},\n  \"intent\": {},\n  \"dynamic_modifier\": {},\n  \"can_apply\": {},\n  \"executed\": {},\n  \"changed_files\": {},\n  \"generated_symbols\": {},\n  \"effect_strategies\": {},\n  \"effect_snippets\": {},\n  \"verified_scripted_effects\": {},\n  \"temp_variables\": {},\n  \"suggestions\": {},\n  \"blockers\": {},\n  \"anti_hallucination_rules\": {}\n}}\n",
        json_str(input),
        json_str(intent),
        dynamic_modifier
            .map(json_str)
            .unwrap_or_else(|| "null".to_string()),
        json_bool(errors.is_empty()),
        json_bool(apply_result.is_some()),
        json_array(&changed_files),
        json_array(&generated_symbols),
        json_array(&suggestion_effect_strategies(suggestions)),
        json_array(&effect_snippets),
        json_array(&scripted_effects),
        json_array(&temp_variables),
        suggestions_json(suggestions),
        json_array(errors),
        json_array(&rules)
    )
}

pub(crate) fn dynamic_modifier_plan_scripted_effects(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|line| {
            let key = assignment_key(line.trim())?;
            key.starts_with("change_").then_some(key.to_string())
        })
        .collect()
}

pub(crate) struct AppliedIntentPatch {
    pub(crate) changed_files: Vec<PathBuf>,
    pub(crate) effect_snippets: Vec<String>,
    pub(crate) generated_symbols: Vec<String>,
}

pub(crate) fn apply_intent_patch_plan_errors(suggestions: &[Suggestion]) -> Vec<String> {
    suggestions
        .iter()
        .filter(|suggestion| {
            !matches!(
                suggestion.kind.as_str(),
                "country_effect" | "idea_definition" | "localisation_entry"
            )
        })
        .map(|suggestion| {
            format!(
                "intent patch apply does not know how to write `{}` from `{}`; use a parent writer such as apply-feature-cards, apply-event-cards, or apply-focus-layout",
                suggestion.kind, suggestion.source
            )
        })
        .collect()
}

pub(crate) fn apply_intent_suggestions_to_mod(
    mod_root: &Path,
    suggestions: &[Suggestion],
    generation_prefix: &str,
    generation_tag: &str,
) -> Result<AppliedIntentPatch, String> {
    let prefix = sanitize_identifier_part(generation_prefix, "mod");
    let idea_path = mod_root
        .join("common")
        .join("ideas")
        .join(format!("{prefix}_ideas.txt"));
    let loc_path = target_localisation_path(mod_root, generation_tag);
    let mut idea_blocks = Vec::new();
    let mut loc_entries = BTreeMap::new();
    let mut effect_snippets = Vec::new();
    let mut generated_symbols = BTreeSet::new();
    let mut changed_files = Vec::new();

    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "country_effect" => effect_snippets.push(suggestion.code.clone()),
            "idea_definition" => {
                let ids = idea_ids_from_idea_definition_code(&suggestion.code);
                if ids.is_empty() {
                    return Err(format!(
                        "idea_definition from `{}` did not contain a direct ideas/country block",
                        suggestion.source
                    ));
                }
                for id in ids {
                    generated_symbols.insert(id.clone());
                    idea_blocks.push((id, suggestion.code.clone()));
                }
            }
            "localisation_entry" => {
                let entries = localisation_entries_from_code(&suggestion.code);
                if entries.is_empty() {
                    return Err(format!(
                        "localisation_entry from `{}` did not contain parseable localisation lines",
                        suggestion.source
                    ));
                }
                loc_entries.extend(entries);
            }
            _ => {}
        }
    }

    if !idea_blocks.is_empty()
        && append_unique_blocks(
            &idea_path,
            "# Generated intent national spirits by hoi4skill\n",
            &idea_blocks,
        )?
    {
        changed_files.push(idea_path);
    }
    if !loc_entries.is_empty() && append_localisation_entries(&loc_path, &loc_entries)? {
        changed_files.push(loc_path);
    }

    Ok(AppliedIntentPatch {
        changed_files,
        effect_snippets,
        generated_symbols: generated_symbols.into_iter().collect(),
    })
}

pub(crate) fn localisation_entries_from_code(code: &str) -> BTreeMap<String, String> {
    let mut entries = BTreeMap::new();
    for line in code.lines() {
        if let Some((key, value)) = parse_localisation_line(line) {
            entries.insert(key, value);
        }
    }
    entries
}

pub(crate) fn apply_intent_patch_report_json(
    input: &str,
    intent: &str,
    context: &str,
    generation_prefix: &str,
    generation_tag: &str,
    suggestions: &[Suggestion],
    result: &AppliedIntentPatch,
) -> String {
    let changed = result
        .changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.intent_patch_apply.v1\",\n  \"input\": {},\n  \"intent\": {},\n  \"context\": {},\n  \"ok\": true,\n  \"prefix\": {},\n  \"tag\": {},\n  \"changed_files\": {},\n  \"effect_snippets\": {},\n  \"generated_symbols\": {},\n  \"patch_plan\": {},\n  \"rule\": {}\n}}\n",
        json_str(input),
        json_str(intent),
        json_str(context),
        json_str(&sanitize_identifier_part(generation_prefix, "mod")),
        json_str(generation_tag),
        json_array(&changed),
        json_array(&result.effect_snippets),
        json_array(&result.generated_symbols),
        compile_intent_patch_plan_json(suggestions, generation_prefix, Some(generation_tag), &[]),
        json_str("Only file_append items were written. effect_snippets still require an explicit parent focus, decision, or event context.")
    )
}

pub(crate) fn inject_intent_effects_into_focus_layout(
    layout: &mut FocusLayout,
    suggestions: &[Suggestion],
    focus_id: Option<&str>,
    focus_title: Option<&str>,
) -> Result<String, String> {
    let effect_lines = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.trim().to_string())
        .filter(|code| !code.is_empty())
        .collect::<Vec<_>>();
    if effect_lines.is_empty() {
        return Err("focus intent produced no country_effect snippet to attach".to_string());
    }
    let idx = if let Some(focus_id) = focus_id {
        layout
            .focuses
            .iter()
            .position(|focus| focus.id == focus_id)
            .ok_or_else(|| format!("focus id `{focus_id}` was not found in the supplied layout"))?
    } else if let Some(focus_title) = focus_title {
        let matches = layout
            .focuses
            .iter()
            .enumerate()
            .filter(|(_, focus)| focus.title == focus_title)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            matches[0]
        } else if matches.is_empty() {
            return Err(format!(
                "focus title `{focus_title}` was not found in the supplied layout"
            ));
        } else {
            return Err(format!(
                "focus title `{focus_title}` matched multiple focuses; use --focus-id"
            ));
        }
    } else {
        0
    };
    let focus = layout
        .focuses
        .get_mut(idx)
        .ok_or_else(|| "focus layout is empty; cannot attach intent effect".to_string())?;
    for line in effect_lines {
        if !focus
            .completion_reward
            .iter()
            .any(|existing| existing == &line)
        {
            focus.completion_reward.push(line);
        }
    }
    Ok(focus.id.clone())
}

pub(crate) fn apply_focus_intent_report_json(
    layout_text: &str,
    intent_text: &str,
    context: &str,
    tag: &str,
    prefix: &str,
    focus_id: &str,
    suggestions: &[Suggestion],
    changed_files: &[PathBuf],
) -> String {
    let changed = changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let effect_snippets = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.clone())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.focus_intent_apply.v1\",\n  \"layout_input\": {},\n  \"intent_input\": {},\n  \"context\": {},\n  \"ok\": true,\n  \"tag\": {},\n  \"prefix\": {},\n  \"focus_id\": {},\n  \"changed_files\": {},\n  \"effect_snippets\": {},\n  \"patch_plan\": {},\n  \"rule\": {}\n}}\n",
        json_str(layout_text),
        json_str(intent_text),
        json_str(context),
        json_str(tag),
        json_str(prefix),
        json_str(focus_id),
        json_array(&changed),
        json_array(&effect_snippets),
        compile_intent_patch_plan_json(suggestions, prefix, Some(tag), &[]),
        json_str("The compiled country_effect snippets were attached to the selected focus completion_reward after code-index checks and before final validation.")
    )
}

pub(crate) fn inject_intent_effects_into_decision_cards(
    cards: &mut [Card],
    suggestions: &[Suggestion],
    decision_title: Option<&str>,
) -> Result<String, String> {
    let effect_lines = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.trim().to_string())
        .filter(|code| !code.is_empty())
        .collect::<Vec<_>>();
    if effect_lines.is_empty() {
        return Err("decision intent produced no country_effect snippet to attach".to_string());
    }
    let decision_indexes = cards
        .iter()
        .enumerate()
        .filter(|(_, card)| card.kind == "决议")
        .filter(|(_, card)| {
            decision_title
                .map(|title| card.title == title)
                .unwrap_or(true)
        })
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let idx = if decision_indexes.len() == 1 {
        decision_indexes[0]
    } else if decision_indexes.is_empty() {
        return Err(decision_title.map_or_else(
            || "no decision card was found in the supplied input".to_string(),
            |title| format!("decision title `{title}` was not found in the supplied input"),
        ));
    } else {
        return Err("multiple decision cards were found; use --decision-title".to_string());
    };
    let card = cards
        .get_mut(idx)
        .ok_or_else(|| "decision card index was invalid".to_string())?;
    let mut existing = card.fields.get("效果").cloned().unwrap_or_default();
    for line in effect_lines {
        let already_present = split_cn_list(&existing)
            .into_iter()
            .any(|part| part.trim() == line);
        if already_present {
            continue;
        }
        if !existing.trim().is_empty() {
            existing.push('；');
        }
        existing.push_str(&line);
    }
    card.fields.insert("效果".to_string(), existing);
    Ok(card.title.clone())
}

pub(crate) fn apply_decision_intent_report_json(
    card_text: &str,
    intent_text: &str,
    context: &str,
    tag: &str,
    prefix: &str,
    decision_title: &str,
    suggestions: &[Suggestion],
    changed_files: &[PathBuf],
) -> String {
    let changed = changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let effect_snippets = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.clone())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.decision_intent_apply.v1\",\n  \"card_input\": {},\n  \"intent_input\": {},\n  \"context\": {},\n  \"ok\": true,\n  \"tag\": {},\n  \"prefix\": {},\n  \"decision_title\": {},\n  \"changed_files\": {},\n  \"effect_snippets\": {},\n  \"patch_plan\": {},\n  \"rule\": {}\n}}\n",
        json_str(card_text),
        json_str(intent_text),
        json_str(context),
        json_str(tag),
        json_str(prefix),
        json_str(decision_title),
        json_array(&changed),
        json_array(&effect_snippets),
        compile_intent_patch_plan_json(suggestions, prefix, Some(tag), &[]),
        json_str("The compiled country_effect snippets were attached to the selected decision complete_effect after code-index checks and before final validation.")
    )
}

pub(crate) struct EventIntentTarget {
    pub(crate) event_title: String,
    pub(crate) option_suffix: String,
    pub(crate) field: String,
}

pub(crate) fn inject_intent_effects_into_event_cards(
    cards: &mut [Card],
    suggestions: &[Suggestion],
    event_title: Option<&str>,
    option: &str,
    hidden_effect: bool,
) -> Result<EventIntentTarget, String> {
    let effect_lines = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.trim().to_string())
        .filter(|code| !code.is_empty())
        .collect::<Vec<_>>();
    if effect_lines.is_empty() {
        return Err("event intent produced no country_effect snippet to attach".to_string());
    }
    let event_indexes = cards
        .iter()
        .enumerate()
        .filter(|(_, card)| card.kind == "事件")
        .filter(|(_, card)| event_title.map(|title| card.title == title).unwrap_or(true))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let idx = if event_indexes.len() == 1 {
        event_indexes[0]
    } else if event_indexes.is_empty() {
        return Err(event_title.map_or_else(
            || "no event card was found in the supplied input".to_string(),
            |title| format!("event title `{title}` was not found in the supplied input"),
        ));
    } else {
        return Err("multiple event cards were found; use --event-title".to_string());
    };
    let suffix = normalize_event_option_suffix(option)?;
    let field = if hidden_effect {
        format!("隐藏效果{suffix}")
    } else {
        format!("效果{suffix}")
    };
    let card = cards
        .get_mut(idx)
        .ok_or_else(|| "event card index was invalid".to_string())?;
    ensure_event_option_exists(card, &suffix);
    let mut existing = card.fields.get(&field).cloned().unwrap_or_default();
    for line in effect_lines {
        let already_present = split_cn_list(&existing)
            .into_iter()
            .any(|part| part.trim() == line);
        if already_present {
            continue;
        }
        if !existing.trim().is_empty() {
            existing.push('；');
        }
        existing.push_str(&line);
    }
    card.fields.insert(field.clone(), existing);
    Ok(EventIntentTarget {
        event_title: card.title.clone(),
        option_suffix: suffix,
        field,
    })
}

pub(crate) fn normalize_event_option_suffix(option: &str) -> Result<String, String> {
    let trimmed = option.trim();
    if trimmed.is_empty() {
        return Ok("A".to_string());
    }
    let without_prefix = trimmed
        .strip_prefix("选项")
        .or_else(|| trimmed.strip_prefix("option"))
        .or_else(|| trimmed.strip_prefix("Option"))
        .unwrap_or(trimmed)
        .trim();
    if without_prefix.is_empty() || !without_prefix.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(format!(
            "event option `{option}` must be an ASCII option suffix like A, B, C, A2"
        ));
    }
    let mut chars = without_prefix.chars();
    let Some(first) = chars.next() else {
        return Ok("A".to_string());
    };
    let mut suffix = String::new();
    suffix.push(first.to_ascii_uppercase());
    suffix.extend(chars);
    Ok(suffix)
}

pub(crate) fn ensure_event_option_exists(card: &mut Card, suffix: &str) {
    let key = format!("选项{suffix}");
    card.fields
        .entry(key)
        .or_insert_with(|| "好的。".to_string());
}

pub(crate) fn apply_event_intent_report_json(
    card_text: &str,
    intent_text: &str,
    context: &str,
    tag: &str,
    prefix: &str,
    target: &EventIntentTarget,
    suggestions: &[Suggestion],
    changed_files: &[PathBuf],
) -> String {
    let changed = changed_files
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let effect_snippets = suggestions
        .iter()
        .filter(|suggestion| suggestion.kind == "country_effect")
        .map(|suggestion| suggestion.code.clone())
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.event_intent_apply.v1\",\n  \"card_input\": {},\n  \"intent_input\": {},\n  \"context\": {},\n  \"ok\": true,\n  \"tag\": {},\n  \"prefix\": {},\n  \"event_title\": {},\n  \"option\": {},\n  \"field\": {},\n  \"changed_files\": {},\n  \"effect_snippets\": {},\n  \"patch_plan\": {},\n  \"rule\": {}\n}}\n",
        json_str(card_text),
        json_str(intent_text),
        json_str(context),
        json_str(tag),
        json_str(prefix),
        json_str(&target.event_title),
        json_str(&target.option_suffix),
        json_str(&target.field),
        json_array(&changed),
        json_array(&effect_snippets),
        compile_intent_patch_plan_json(suggestions, prefix, Some(tag), &[]),
        json_str("The compiled country_effect snippets were attached to the selected event option field after code-index checks and before final validation.")
    )
}

pub(crate) fn context_allowed_suggestion_kinds(context: &str) -> Vec<&'static str> {
    match context {
        "idea" => vec!["idea_modifier", "idea_modifier_candidate", "idea_field"],
        "effect" => vec![
            "country_effect",
            "country_effect_candidate",
            "idea_definition",
            "localisation_entry",
        ],
        "trigger" => vec!["trigger", "trigger_candidate"],
        "state_effect" => vec!["state_effect_candidate"],
        "auto" => vec![
            "idea_modifier",
            "idea_modifier_candidate",
            "idea_field",
            "country_effect",
            "country_effect_candidate",
            "idea_definition",
            "localisation_entry",
            "trigger",
            "trigger_candidate",
            "state_effect_candidate",
        ],
        _ => Vec::new(),
    }
}

pub(crate) fn context_suggestion_errors(
    context: &str,
    intent_context: &str,
    suggestions: &[Suggestion],
) -> Vec<String> {
    if intent_context == "auto" {
        return Vec::new();
    }
    let allowed = context_allowed_suggestion_kinds(intent_context);
    let mut errors = Vec::new();
    for suggestion in suggestions {
        if suggestion.kind == "raw_effect" || suggestion.kind == "raw_trigger" {
            continue;
        }
        if !allowed.iter().any(|kind| *kind == suggestion.kind) {
            errors.push(format!(
                "{context}: `{}` compiled to `{}` but --kind `{intent_context}` only accepts {}; rerun compile-intent with the correct --kind or add an explicit mapping",
                suggestion.source,
                suggestion.kind,
                allowed.join(", ")
            ));
        }
    }
    errors
}

pub(crate) fn missing_code_index_category_errors(
    context: &str,
    suggestions: &[Suggestion],
    index: &GameIndex,
) -> Vec<String> {
    let mut errors = Vec::new();
    let generated_ideas = generated_idea_ids_from_suggestions(suggestions);
    let mut needs_effects = false;
    let mut needs_triggers = false;
    let mut needs_modifiers = false;
    let mut needs_ideas = false;
    let mut needs_resources = false;
    let mut needs_buildings = false;

    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "country_effect" | "country_effect_candidate" | "state_effect_candidate" => {
                if assignment_key(&suggestion.code).is_some() {
                    needs_effects = true;
                }
                if assignment_key(&suggestion.code) == Some("add_resource") {
                    needs_resources = true;
                }
                if assignment_key(&suggestion.code) == Some("add_building_construction") {
                    needs_buildings = true;
                }
                if effect_suggestion_needs_existing_idea_index(&suggestion.code, &generated_ideas) {
                    needs_ideas = true;
                }
            }
            "trigger" | "trigger_candidate" => {
                if assignment_key(&suggestion.code).is_some() {
                    needs_triggers = true;
                }
            }
            "idea_modifier" | "idea_modifier_candidate" => {
                if assignment_key(&suggestion.code).is_some() {
                    needs_modifiers = true;
                }
            }
            _ => {}
        }
    }

    if needs_effects && index.effects.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed effects; rebuild the index from documentation/effects_documentation.md or load the required game/dependency code before accepting generated effects"
        ));
    }
    if needs_triggers && index.triggers.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed triggers; rebuild the index from documentation/triggers_documentation.md or load the required game/dependency code before accepting generated triggers"
        ));
    }
    if needs_modifiers && index.modifiers.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed modifiers; rebuild the index from documentation/modifiers_documentation.md or load the required game/dependency code before accepting generated modifiers"
        ));
    }
    if needs_ideas && index.ideas.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed ideas; load common/ideas from the required game/dependency code before accepting generated idea references"
        ));
    }
    if needs_resources && index.resources.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed resources; load the required game/dependency code before accepting add_resource output"
        ));
    }
    if needs_buildings && index.buildings.is_empty() {
        errors.push(format!(
            "{context}: strict code index has no indexed buildings; load the required game/dependency code before accepting add_building_construction output"
        ));
    }
    errors
}

pub(crate) fn effect_suggestion_needs_existing_idea_index(
    code: &str,
    generated_ideas: &BTreeSet<String>,
) -> bool {
    if let Some(key) = assignment_key(code) {
        if matches!(key, "remove_ideas" | "has_idea") {
            return true;
        }
        if key == "add_ideas" {
            return code_assignment_value(code, key)
                .is_none_or(|idea| !generated_ideas.contains(idea));
        }
    }
    for (scope, block) in direct_child_blocks(code) {
        if scope == "swap_ideas" {
            if direct_assignment_value(&block, "remove_idea").is_some() {
                return true;
            }
            if direct_assignment_value(&block, "add_idea")
                .is_some_and(|idea| !generated_ideas.contains(idea))
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn compile_intent_index_checks_json(
    suggestions: &[Suggestion],
    index: Option<&GameIndex>,
) -> String {
    let Some(index) = index else {
        return "[]".to_string();
    };
    format!(
        "[{}]",
        suggestions
            .iter()
            .flat_map(|suggestion| compile_intent_index_checks_for_suggestion(suggestion, index))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn compile_intent_index_checks_for_suggestion(
    suggestion: &Suggestion,
    index: &GameIndex,
) -> Vec<String> {
    let mut checks = Vec::new();
    if let Some((symbol, kind)) = suggestion_primary_symbol(suggestion) {
        let ok = !code_symbol_matches(index, &symbol, Some(kind)).is_empty();
        checks.push(format!(
            "{{\"source\": {}, \"code\": {}, \"symbol\": {}, \"kind\": {}, \"ok\": {}}}",
            json_str(&suggestion.source),
            json_str(&suggestion.code),
            json_str(&symbol),
            json_str(kind),
            json_bool(ok)
        ));
    }
    if let Some(resource) = suggestion_resource_symbol(suggestion) {
        let ok = !code_symbol_matches(index, resource, Some("resource_id")).is_empty();
        checks.push(format!(
            "{{\"source\": {}, \"code\": {}, \"symbol\": {}, \"kind\": {}, \"ok\": {}}}",
            json_str(&suggestion.source),
            json_str(&suggestion.code),
            json_str(resource),
            json_str("resource_id"),
            json_bool(ok)
        ));
    }
    checks
}

pub(crate) fn suggestion_primary_symbol(suggestion: &Suggestion) -> Option<(String, &'static str)> {
    let key = assignment_key(&suggestion.code)?;
    match suggestion.kind.as_str() {
        "country_effect" | "country_effect_candidate" | "state_effect_candidate" => Some((
            effect_suggestion_primary_key(&suggestion.code).unwrap_or_else(|| key.to_string()),
            "effect",
        )),
        "trigger" | "trigger_candidate" => Some((key.to_string(), "trigger")),
        "idea_modifier" | "idea_modifier_candidate" => Some((key.to_string(), "modifier")),
        _ => None,
    }
}

pub(crate) fn effect_suggestion_primary_key(code: &str) -> Option<String> {
    for key in direct_assignment_keys(code) {
        if key == "limit" {
            continue;
        }
        if is_static_effect_scope_key(&key) || is_effect_control_block(&key) {
            for (scope, scoped_block) in direct_child_blocks(code) {
                if scope == key
                    || is_static_effect_scope_key(&scope)
                    || is_effect_control_block(&scope)
                {
                    if let Some(inner) = effect_suggestion_primary_key(&scoped_block) {
                        return Some(inner);
                    }
                }
            }
            continue;
        }
        return Some(key);
    }
    let key = assignment_key(code)?;
    (key != "limit").then(|| key.to_string())
}

pub(crate) fn is_static_effect_scope_key(key: &str) -> bool {
    matches!(
        key,
        "ROOT" | "FROM" | "PREV" | "THIS" | "OVERLORD" | "overlord" | "owner" | "controller"
    ) || looks_like_tag(key)
        || parse_plain_i64(key).is_some()
}

pub(crate) fn suggestion_resource_symbol(suggestion: &Suggestion) -> Option<&str> {
    if assignment_key(&suggestion.code) == Some("add_resource") {
        code_assignment_value(&suggestion.code, "type")
    } else {
        None
    }
}

#[derive(Clone)]
pub(crate) struct Suggestion {
    pub(crate) kind: String,
    pub(crate) code: String,
    pub(crate) source: String,
    pub(crate) note: String,
}

pub(crate) fn suggestion_effect_strategy(suggestion: &Suggestion) -> &'static str {
    if suggestion.kind == "country_effect" && suggestion.code.contains("swap_ideas") {
        return "replace_national_spirit_with_swap_ideas";
    }
    if suggestion.kind == "country_effect"
        && suggestion.code.contains("custom_effect_tooltip")
        && suggestion.code.contains("set_temp_variable")
        && suggestion
            .code
            .lines()
            .any(|line| assignment_key(line.trim()).is_some_and(|key| key.starts_with("change_")))
    {
        return "dynamic_modifier_scripted_effect_protocol";
    }
    if suggestion.kind == "country_effect" && assignment_key(&suggestion.code) == Some("add_ideas")
    {
        return "add_existing_or_generated_national_spirit";
    }
    if suggestion.kind == "idea_definition" {
        return "create_national_spirit_definition";
    }
    if suggestion.kind == "localisation_entry" {
        return "localise_generated_symbol";
    }
    if suggestion.kind == "idea_modifier" || suggestion.kind == "idea_modifier_candidate" {
        return "idea_modifier_only";
    }
    if suggestion.kind == "state_effect_candidate" {
        return "requires_state_scope";
    }
    if suggestion.kind == "trigger" || suggestion.kind == "trigger_candidate" {
        return "trigger_condition";
    }
    "unclassified"
}

pub(crate) fn suggestion_effect_strategies(suggestions: &[Suggestion]) -> Vec<String> {
    suggestions
        .iter()
        .map(suggestion_effect_strategy)
        .filter(|strategy| *strategy != "unclassified")
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

impl Suggestion {
    pub(crate) fn new(kind: &str, code: &str, source: &str, note: &str) -> Self {
        Self {
            kind: kind.to_string(),
            code: code.to_string(),
            source: source.to_string(),
            note: note.to_string(),
        }
    }
}

pub(crate) fn unresolved_suggestion_errors_with_index(
    context: &str,
    suggestions: &[Suggestion],
    index: Option<&GameIndex>,
) -> Vec<String> {
    let mut errors = Vec::new();
    for suggestion in suggestions {
        if suggestion.kind == "raw_effect" || suggestion.kind == "raw_trigger" {
            if index.is_some_and(|index| verified_raw_suggestion_against_index(suggestion, index)) {
                continue;
            }
            let related = index
                .map(|index| {
                    related_code_symbols_text(
                        index,
                        &suggestion.source,
                        suggestion_kind_hint(&suggestion.kind),
                    )
                })
                .unwrap_or_default();
            errors.push(format!(
                "{context}: `{}` is unresolved {}; map the user intent to a verified CLI shorthand or code-catalog entry before final generation{}",
                suggestion.source, suggestion.kind, related
            ));
            continue;
        }
        if suggestion.code.contains('<') || suggestion.code.contains('>') {
            let note = if suggestion.note.trim().is_empty() {
                String::new()
            } else {
                format!("; {}", suggestion.note.trim())
            };
            errors.push(format!(
                "{context}: `{}` still contains placeholder code `{}`; resolve IDs/numbers from the mod index before final generation{}",
                suggestion.source, suggestion.code, note
            ));
            continue;
        }
        if suggestion
            .note
            .contains("Needs Codex mapping before final code")
        {
            errors.push(format!(
                "{context}: `{}` still needs Codex mapping before final code",
                suggestion.source
            ));
        }
    }
    errors
}

pub(crate) fn verified_raw_suggestion_against_index(
    suggestion: &Suggestion,
    index: &GameIndex,
) -> bool {
    if suggestion.code.contains('<') || suggestion.code.contains('>') {
        return false;
    }
    let Some(key) = assignment_key(&suggestion.code) else {
        return false;
    };
    match suggestion.kind.as_str() {
        "raw_effect" => index.effects.contains(key),
        "raw_trigger" => {
            if !index.triggers.contains(key) {
                return false;
            }
            if matches!(key, "tag" | "original_tag") {
                if let Some(value) = code_assignment_value(&suggestion.code, key) {
                    return !looks_like_tag(value) || index.country_tags.contains(value);
                }
            }
            true
        }
        _ => false,
    }
}

pub(crate) fn suggestion_kind_hint(kind: &str) -> Option<&'static str> {
    match kind {
        "raw_effect" => Some("effect"),
        "raw_trigger" => Some("trigger"),
        _ => None,
    }
}

pub(crate) fn unindexed_suggestion_errors(
    context: &str,
    suggestions: &[Suggestion],
    index: &GameIndex,
) -> Vec<String> {
    let mut errors = Vec::new();
    let generated_ideas = generated_idea_ids_from_suggestions(suggestions);
    for suggestion in suggestions {
        if suggestion.code.contains('<') || suggestion.code.contains('>') {
            continue;
        }
        match suggestion.kind.as_str() {
            "country_effect" | "country_effect_candidate" | "state_effect_candidate"
                if !index.effects.is_empty() =>
            {
                collect_unindexed_effect_code_errors(
                    &mut errors,
                    context,
                    &suggestion.source,
                    &suggestion.code,
                    index,
                    &generated_ideas,
                );
            }
            "trigger" | "trigger_candidate" if !index.triggers.is_empty() => errors.extend(
                unindexed_trigger_suggestion_code_errors(context, suggestion, index),
            ),
            "idea_modifier" | "idea_modifier_candidate" if !index.modifiers.is_empty() => {
                if let Some(key) = assignment_key(&suggestion.code) {
                    if !index.modifiers.contains(key) {
                        let related = related_code_symbols_text(index, key, Some("modifier"));
                        errors.push(format!(
                            "{context}: `{}` maps to unindexed modifier `{key}`; verify it with `check-code-symbol --kind modifier` before final generation{}",
                            suggestion.source, related
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    errors
}

pub(crate) fn generated_idea_ids_from_suggestions(suggestions: &[Suggestion]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for suggestion in suggestions {
        if suggestion.kind != "idea_definition" {
            continue;
        }
        ids.extend(idea_ids_from_idea_definition_code(&suggestion.code));
    }
    ids
}

pub(crate) fn idea_ids_from_idea_definition_code(code: &str) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for (wrapper, wrapper_block) in direct_child_blocks(code) {
        if wrapper == "ideas" {
            for (_category, category_block) in direct_child_blocks(&wrapper_block) {
                for (idea, _idea_block) in direct_child_blocks(&category_block) {
                    ids.insert(idea);
                }
            }
        }
    }
    ids.into_iter().collect()
}

pub(crate) fn idea_symbols_from_effect_code(code: &str) -> Vec<String> {
    let mut symbols = BTreeSet::new();
    for key in ["add_ideas", "remove_ideas", "has_idea"] {
        if let Some(idea) = code_assignment_value(code, key) {
            symbols.insert(idea.to_string());
        }
    }
    for (scope, scoped_block) in direct_child_blocks(code) {
        if scope == "swap_ideas" {
            for key in ["remove_idea", "add_idea"] {
                if let Some(idea) = direct_assignment_value(&scoped_block, key) {
                    symbols.insert(idea.to_string());
                }
            }
        }
    }
    symbols.into_iter().collect()
}

pub(crate) fn localisation_keys_from_entry(code: &str) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "l_simp_chinese:" {
            continue;
        }
        let Some((key, _rest)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if !key.is_empty() {
            keys.insert(key.to_string());
        }
    }
    keys.into_iter().collect()
}

pub(crate) fn collect_unindexed_effect_code_errors(
    errors: &mut Vec<String>,
    context: &str,
    source: &str,
    code: &str,
    index: &GameIndex,
    generated_ideas: &BTreeSet<String>,
) {
    for key in direct_assignment_keys(code) {
        if key == "limit" {
            continue;
        }
        if index.effects.contains(&key)
            || is_effect_scope_key(&key, index)
            || is_effect_control_block(&key)
        {
            if key == "add_resource" && !index.resources.is_empty() {
                if let Some(resource) = code_assignment_value(code, "type") {
                    if !index.resources.contains(resource) {
                        let related = related_code_symbols_text(index, resource, Some("resource"));
                        errors.push(format!(
                            "{context}: `{source}` maps to unindexed resource `{resource}`; verify it with `check-code-symbol --kind resource` before final generation{related}"
                        ));
                    }
                }
            }
            if key == "add_building_construction" && !index.buildings.is_empty() {
                if let Some(building) = code_assignment_value(code, "type") {
                    if !index.buildings.contains(building) {
                        let related = related_code_symbols_text(index, building, Some("building"));
                        errors.push(format!(
                            "{context}: `{source}` maps to unindexed building `{building}`; verify it with `check-code-symbol --kind building` before final generation{related}"
                        ));
                    }
                }
            }
            if matches!(key.as_str(), "add_ideas" | "remove_ideas") && !index.ideas.is_empty() {
                if let Some(idea) = code_assignment_value(code, &key) {
                    if !index.ideas.contains(idea) && !generated_ideas.contains(idea) {
                        let related = related_code_symbols_text(index, idea, Some("idea"));
                        errors.push(format!(
                            "{context}: `{source}` maps `{key}` to unindexed idea `{idea}`; verify it with `check-code-symbol --kind idea` before final generation{related}"
                        ));
                    }
                }
            }
            continue;
        }
        let related = related_code_symbols_text(index, &key, Some("effect"));
        errors.push(format!(
            "{context}: `{source}` maps to unindexed effect `{key}`; verify it with `check-code-symbol --kind effect` before final generation{related}"
        ));
    }
    for (scope, scoped_block) in direct_child_blocks(code) {
        if scope == "limit" {
            continue;
        }
        if scope == "swap_ideas" {
            collect_unindexed_swap_ideas_errors(
                errors,
                context,
                source,
                &scoped_block,
                index,
                generated_ideas,
            );
            continue;
        }
        if is_effect_scope_key(&scope, index) || is_effect_control_block(&scope) {
            collect_unindexed_effect_code_errors(
                errors,
                context,
                source,
                &scoped_block,
                index,
                generated_ideas,
            );
        }
    }
}

pub(crate) fn collect_unindexed_swap_ideas_errors(
    errors: &mut Vec<String>,
    context: &str,
    source: &str,
    code: &str,
    index: &GameIndex,
    generated_ideas: &BTreeSet<String>,
) {
    if index.ideas.is_empty() {
        errors.push(format!(
            "{context}: `{source}` uses swap_ideas but strict code index has no indexed common/ideas IDs; load game/dependency idea files before accepting generated idea replacement"
        ));
        return;
    }
    for key in ["remove_idea", "add_idea"] {
        let Some(idea) = direct_assignment_value(code, key) else {
            errors.push(format!(
                "{context}: `{source}` uses swap_ideas without `{key}`; use swap_ideas = {{ remove_idea = <old idea> add_idea = <new idea> }}"
            ));
            continue;
        };
        if key == "add_idea" && generated_ideas.contains(idea) {
            continue;
        }
        if !index.ideas.contains(idea) {
            let related = related_code_symbols_text(index, idea, Some("idea"));
            errors.push(format!(
                "{context}: `{source}` maps `{key}` to unindexed idea `{idea}`; resolve the national spirit ID from common/ideas before final generation{related}"
            ));
        }
    }
}

pub(crate) fn unindexed_trigger_suggestion_code_errors(
    context: &str,
    suggestion: &Suggestion,
    index: &GameIndex,
) -> Vec<String> {
    let mut errors = Vec::new();
    collect_unindexed_trigger_code_errors(
        &mut errors,
        context,
        &suggestion.source,
        &suggestion.code,
        index,
    );
    errors
}

pub(crate) fn collect_unindexed_trigger_code_errors(
    errors: &mut Vec<String>,
    context: &str,
    source: &str,
    code: &str,
    index: &GameIndex,
) {
    for key in direct_assignment_keys(code) {
        if index.triggers.contains(&key)
            || is_trigger_child_context(&key, index)
            || is_trigger_control_block(&key)
        {
            if key == "has_idea" && !index.ideas.is_empty() {
                if let Some(idea) = code_assignment_value(code, "has_idea") {
                    if !index.ideas.contains(idea) {
                        let related = related_code_symbols_text(index, idea, Some("idea"));
                        errors.push(format!(
                            "{context}: `{source}` maps `has_idea` to unindexed idea `{idea}`; verify it with `check-code-symbol --kind idea` before final generation{related}"
                        ));
                    }
                }
            }
            continue;
        }
        let related = related_code_symbols_text(index, &key, Some("trigger"));
        errors.push(format!(
            "{context}: `{source}` maps to unindexed trigger `{key}`; verify it with `check-code-symbol --kind trigger` before final generation{related}"
        ));
    }
    for (scope, scoped_block) in direct_child_blocks(code) {
        if is_trigger_child_context(&scope, index) || is_trigger_control_block(&scope) {
            collect_unindexed_trigger_code_errors(errors, context, source, &scoped_block, index);
        }
    }
}

pub(crate) fn related_code_symbols_text(
    index: &GameIndex,
    query: &str,
    requested_kind: Option<&str>,
) -> String {
    let matches = related_code_symbol_matches(index, query, requested_kind, 5);
    if matches.is_empty() {
        return String::new();
    }
    format!(
        "; related indexed code: {}",
        matches
            .iter()
            .map(|item| format!("{}/{} `{}`", item.category, item.kind, item.symbol))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn code_assignment_value<'a>(code: &'a str, wanted_key: &str) -> Option<&'a str> {
    let tokens = code
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '{' | '}'))
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    for window in tokens.windows(3) {
        if window[0] == wanted_key && window[1] == "=" {
            return Some(window[2]);
        }
    }
    None
}

pub(crate) fn suggest_common(
    ty: &str,
    effects: &str,
    cost: Option<&str>,
    duration: Option<&str>,
    condition: Option<&str>,
    removal: Option<&str>,
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if let Some(cost) = cost.and_then(parse_int) {
        out.push(Suggestion::new(
            "decision_cost",
            &format!("cost = {cost}"),
            "",
            "Decision political power cost.",
        ));
    }
    if let Some(days) = duration.and_then(parse_int) {
        out.push(Suggestion::new(
            "days_remove",
            &format!("days_remove = {days}"),
            "",
            "",
        ));
    }
    if let Some(cond) = condition {
        for raw in split_cn_list(cond) {
            out.extend(suggest_trigger(raw));
        }
    }
    for raw in split_cn_list(effects) {
        let suggestion_count_before = out.len();
        if ambiguous_multi_effect_segment(raw) {
            out.push(Suggestion::new(
                "raw_effect",
                raw,
                raw,
                "Multiple effect intents appear in one segment; split them with `，` or `；` before final code.",
            ));
            continue;
        }
        if let Some(suggestion) = semantic_idea_modifier(raw) {
            out.push(suggestion);
            continue;
        }
        let percent = parse_percent(raw);
        let number = parse_int(raw);
        if raw.contains("政治点") || raw.contains("政治力量") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("add_political_power = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("稳定") {
            if let Some(v) = percent {
                let code = if ty == "idea" {
                    format!("stability_factor = {}", fmt_float(v))
                } else {
                    format!("add_stability = {}", fmt_float(v))
                };
                out.push(Suggestion::new(
                    if ty == "idea" {
                        "idea_modifier"
                    } else {
                        "country_effect"
                    },
                    &code,
                    raw,
                    "",
                ));
            }
        } else if raw.contains("战争支持") || raw.contains("战争支援") {
            if let Some(v) = percent {
                let code = if ty == "idea" {
                    format!("war_support_factor = {}", fmt_float(v))
                } else {
                    format!("add_war_support = {}", fmt_float(v))
                };
                out.push(Suggestion::new(
                    if ty == "idea" {
                        "idea_modifier"
                    } else {
                        "country_effect"
                    },
                    &code,
                    raw,
                    "",
                ));
            }
        } else if raw.contains("海军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("navy_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("陆军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("army_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("空军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("air_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("消费品") {
            if let Some(v) = percent {
                out.push(Suggestion::new(
                    "idea_modifier_candidate",
                    &format!("consumer_goods_factor = {}", fmt_float(v)),
                    raw,
                    "Verify modifier name against local game documentation or nearby mod code.",
                ));
            }
        } else if raw.contains("建造速度") || raw.contains("建设速度") {
            if let Some(v) = percent {
                out.push(Suggestion::new(
                    "idea_modifier_candidate",
                    &format!("production_speed_buildings_factor = {}", fmt_float(v)),
                    raw,
                    "Verify modifier name against local game documentation or nearby mod code.",
                ));
            }
        } else if raw.contains("基础设施") || raw.contains("基建") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = infrastructure level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("防空") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = anti_air_building level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("船坞") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = dockyard level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("炼油") || raw.contains("合成油") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = synthetic_refinery level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("民用工厂") || raw.contains("民工") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = industrial_complex level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("军用工厂") || raw.contains("军工") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = arms_factory level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if let Some((resource, amount)) = state_resource_effect(raw) {
            out.push(Suggestion::new(
                "state_effect_candidate",
                &format!("add_resource = {{ type = {resource} amount = {amount} }}"),
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("移除核心") {
            if let Some(tag) = ascii_tag_from_text(raw) {
                out.push(Suggestion::new(
                    "state_effect_candidate",
                    &format!("remove_core_of = {tag}"),
                    raw,
                    "Must run inside a state scope.",
                ));
            } else {
                out.push(Suggestion::new(
                    "raw_effect",
                    raw,
                    raw,
                    "Resolve the country tag before removing a core.",
                ));
            }
        } else if raw.contains("添加核心") || raw.contains("获得核心") {
            if let Some(tag) = ascii_tag_from_text(raw) {
                out.push(Suggestion::new(
                    "state_effect_candidate",
                    &format!("add_core_of = {tag}"),
                    raw,
                    "Must run inside a state scope.",
                ));
            } else {
                out.push(Suggestion::new(
                    "raw_effect",
                    raw,
                    raw,
                    "Resolve the country tag before adding a core.",
                ));
            }
        } else if raw.contains("添加民族精神") || raw.contains("获得民族精神") {
            let idea_name = raw.replace("添加民族精神", "").replace("获得民族精神", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!("add_ideas = <idea id for {}>", idea_name.trim()),
                raw,
                "",
            ));
        } else if raw.contains("移除民族精神") {
            let idea_name = raw.replace("移除民族精神", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!("remove_ideas = <idea id for {}>", idea_name.trim()),
                raw,
                "",
            ));
        } else if raw.contains("触发新闻") {
            let event_name = raw.replace("触发新闻", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!(
                    "news_event = {{ id = <event id for {}> }}",
                    event_name.trim()
                ),
                raw,
                "",
            ));
        } else if raw.contains("触发事件") || raw.contains("触发国家事件") {
            let event_name = raw.replace("触发国家事件", "").replace("触发事件", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!(
                    "country_event = {{ id = <event id for {}> }}",
                    event_name.trim()
                ),
                raw,
                "",
            ));
        } else if raw.contains("设置旗标") || raw.contains("设置国家旗标") {
            let flag = slugify(
                raw.replace("设置国家旗标", "")
                    .replace("设置旗标", "")
                    .trim(),
                "my_flag",
            );
            out.push(Suggestion::new(
                "country_effect",
                &format!("set_country_flag = {flag}"),
                raw,
                "",
            ));
        } else if assignment_key(raw).is_some() {
            out.push(Suggestion::new(
                if ty == "idea" {
                    "idea_modifier_candidate"
                } else {
                    "country_effect"
                },
                raw,
                raw,
                "Raw assignment; strict code index must verify the key before final generation.",
            ));
        } else if !raw.trim().is_empty() {
            out.push(Suggestion::new(
                "raw_effect",
                raw,
                raw,
                "Needs Codex mapping before final code.",
            ));
        }
        if out.len() == suggestion_count_before && !raw.trim().is_empty() {
            out.push(Suggestion::new(
                "raw_effect",
                raw,
                raw,
                "Could not parse a required numeric value; map the intent explicitly before final code.",
            ));
        }
    }
    if let Some(removal) = removal {
        if removal.contains("不可") || removal.contains("不能") || removal.contains("永久") {
            out.push(Suggestion::new(
                "idea_field",
                "removal_cost = -1",
                removal,
                "",
            ));
        }
    }
    out
}

pub(crate) fn ambiguous_multi_effect_segment(raw: &str) -> bool {
    let mut families = 0;
    for needles in [
        &["政治点", "政治力量"][..],
        &["稳定度", "稳定"][..],
        &["战争支持", "战争支援"][..],
        &["海军经验"][..],
        &["陆军经验"][..],
        &["空军经验"][..],
        &["消费品"][..],
        &["建造速度", "建设速度"][..],
        &["军用工厂", "军工"][..],
        &["民用工厂", "民工"][..],
    ] {
        if needles.iter().any(|needle| raw.contains(needle)) {
            families += 1;
        }
    }
    families > 1
}

pub(crate) fn semantic_idea_modifier(raw: &str) -> Option<Suggestion> {
    let percent = parse_percent(raw)?;
    let key = semantic_modifier_key(raw)?;
    Some(Suggestion::new(
        "idea_modifier_candidate",
        &format!("{key} = {}", fmt_float(percent)),
        raw,
        "Resolved from semantic modifier shorthand; verify against local game code index before release.",
    ))
}

pub(crate) fn semantic_modifier_key(raw: &str) -> Option<&'static str> {
    let label = raw
        .split(['=', '＝', ':', '：'])
        .next()
        .unwrap_or(raw)
        .trim();
    if semantic_label_contains_any(
        label,
        &["战争正当化", "正当化战争", "战争目标正当化", "战争借口"],
    ) {
        Some("justify_war_goal_time")
    } else if semantic_label_contains_any(
        label,
        &["政治点获取", "政治力量获取", "政治点增益", "政治力量增益"],
    ) {
        Some("political_power_factor")
    } else {
        None
    }
}

fn semantic_label_contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(crate) fn state_resource_effect(raw: &str) -> Option<(&'static str, i64)> {
    let resource = if raw.contains("steel") || raw.contains("钢") {
        "steel"
    } else if raw.contains("aluminium") || raw.contains("aluminum") || raw.contains("铝") {
        "aluminium"
    } else if raw.contains("oil") || raw.contains("石油") {
        "oil"
    } else if raw.contains("rubber") || raw.contains("橡胶") {
        "rubber"
    } else if raw.contains("tungsten") || raw.contains("钨") {
        "tungsten"
    } else if raw.contains("chromium") || raw.contains("铬") {
        "chromium"
    } else {
        return None;
    };
    let amount = parse_int(raw).unwrap_or(1);
    Some((resource, amount))
}

pub(crate) fn suggest_trigger(text: &str) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if let Some(rest) = text.strip_prefix("完成国策") {
        out.push(Suggestion::new(
            "trigger_candidate",
            &format!("has_completed_focus = <focus id for {}>", rest.trim()),
            text,
            "Resolve the Chinese focus title to a real focus ID before code generation.",
        ));
    } else if let Some(rest) = text.strip_prefix("拥有民族精神") {
        out.push(Suggestion::new(
            "trigger_candidate",
            &format!("has_idea = <idea id for {}>", rest.trim()),
            text,
            "",
        ));
    } else if text.contains("和平")
        || text.contains("无战争")
        || text.contains("不在战争")
        || text.contains("非战争")
    {
        out.push(Suggestion::new("trigger", "has_war = no", text, ""));
    } else if text.contains("战争中") || text.contains("正在战争") {
        out.push(Suggestion::new("trigger", "has_war = yes", text, ""));
    } else if assignment_key(text).is_some() {
        out.push(Suggestion::new(
            "trigger",
            text,
            text,
            "Raw trigger assignment; strict code index must verify the key before final generation.",
        ));
    } else {
        out.push(Suggestion::new(
            "raw_trigger",
            text,
            text,
            "Needs Codex mapping before final code.",
        ));
    }
    out
}

pub(crate) fn split_cn_list(text: &str) -> Vec<&str> {
    text.split(['，', ',', '；', ';', '、', '\n', '\r'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

pub(crate) fn parse_percent(text: &str) -> Option<f64> {
    let idx = text.find(['%', '％'])?;
    let prefix = &text[..idx];
    parse_last_number_with_sign(prefix).map(|(mut v, explicit_sign)| {
        if !explicit_sign && v > 0.0 && has_negative_direction(text) {
            v = -v;
        }
        v / 100.0
    })
}

pub(crate) fn parse_int(text: &str) -> Option<i64> {
    parse_last_number_with_sign(text).map(|(mut v, explicit_sign)| {
        if !explicit_sign && v > 0.0 && has_negative_direction(text) {
            v = -v;
        }
        v as i64
    })
}

pub(crate) fn parse_last_number_with_sign(text: &str) -> Option<(f64, bool)> {
    let mut current = String::new();
    let mut last = None;
    for ch in text.chars() {
        let ch = normalize_number_char(ch);
        if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(v) = current.parse::<f64>() {
                last = Some((v, current.starts_with(['-', '+'])));
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        if let Ok(v) = current.parse::<f64>() {
            last = Some((v, current.starts_with(['-', '+'])));
        }
    }
    last
}

pub(crate) fn normalize_number_char(ch: char) -> char {
    match ch {
        '０' => '0',
        '１' => '1',
        '２' => '2',
        '３' => '3',
        '４' => '4',
        '５' => '5',
        '６' => '6',
        '７' => '7',
        '８' => '8',
        '９' => '9',
        '．' => '.',
        '＋' => '+',
        '－' | '−' | '–' | '—' => '-',
        _ => ch,
    }
}

pub(crate) fn has_negative_direction(text: &str) -> bool {
    contains_any(
        text,
        &[
            "降低",
            "减少",
            "削减",
            "下降",
            "下调",
            "缩短",
            "减轻",
            "压低",
            "降低了",
        ],
    )
}

pub(crate) fn fmt_float(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(crate) fn normalize_event_type(value: Option<&str>) -> &str {
    match value.unwrap_or("") {
        v if v.contains("新闻") || v.eq_ignore_ascii_case("news_event") => "news_event",
        v if v.contains("省份")
            || v.contains("地区")
            || v.contains("州")
            || v.eq_ignore_ascii_case("state_event") =>
        {
            "state_event"
        }
        _ => "country_event",
    }
}

pub(crate) fn option_key(s: &str) -> String {
    match s.trim() {
        "" | "A" | "a" | "一" => "a".to_string(),
        "B" | "b" | "二" => "b".to_string(),
        "C" | "c" | "三" => "c".to_string(),
        "D" | "d" | "四" => "d".to_string(),
        other => slugify(other, "a"),
    }
}
