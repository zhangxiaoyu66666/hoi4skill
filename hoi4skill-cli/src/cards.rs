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
    let context = if requested_context == "auto" {
        infer_intent_context(&intent)
    } else {
        requested_context
    };
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
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

    let suggestions = compile_intent_suggestions(&intent, context)?;
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
    let json = compile_intent_json(
        &input_text,
        &intent,
        context,
        options.strict_code_index,
        game_index.as_ref(),
        &suggestions,
        &errors,
    );
    write_or_print(&json, value(&map, "output"))?;
    if !errors.is_empty() {
        Err("intent compilation blocked unresolved or unindexed HOI4 code".to_string())
    } else {
        Ok(())
    }
}

pub(crate) fn intent_text_from_args(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "intent")) {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_utf8_lossy(&normalize_path(input)?);
    }
    Err("compile-intent requires --text or --input".to_string())
}

pub(crate) fn normalize_llm_intent_text(text: &str) -> String {
    let trimmed = text.trim();
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
) -> Result<Vec<Suggestion>, String> {
    let mut suggestions = Vec::new();
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
) -> String {
    let indexed = index.is_some();
    let allowed_kinds = context_allowed_suggestion_kinds(context)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.intent_compile.v1\",\n  \"input\": {},\n  \"intent\": {},\n  \"context\": {},\n  \"allowed_kinds\": {},\n  \"strict_code_index\": {},\n  \"code_index_checked\": {},\n  \"ok\": {},\n  \"safety\": {},\n  \"suggestions\": {},\n  \"index_checks\": {},\n  \"errors\": {},\n  \"anti_hallucination_rule\": {}\n}}\n",
        json_str(input),
        json_str(intent),
        json_str(context),
        json_array(&allowed_kinds),
        json_bool(strict_code_index),
        json_bool(indexed),
        json_bool(errors.is_empty()),
        suggestions_safety_json_with_extra_blockers(suggestions, errors),
        suggestions_json(suggestions),
        compile_intent_index_checks_json(suggestions, index),
        json_array(errors),
        json_str("AI may provide intent only; final Clausewitz code must come from mapped suggestions and must fail if safety or code-index checks report blockers.")
    )
}

pub(crate) fn context_allowed_suggestion_kinds(context: &str) -> Vec<&'static str> {
    match context {
        "idea" => vec!["idea_modifier", "idea_modifier_candidate", "idea_field"],
        "effect" => vec!["country_effect", "country_effect_candidate"],
        "trigger" => vec!["trigger", "trigger_candidate"],
        "state_effect" => vec!["state_effect_candidate"],
        "auto" => vec![
            "idea_modifier",
            "idea_modifier_candidate",
            "idea_field",
            "country_effect",
            "country_effect_candidate",
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
    let mut needs_effects = false;
    let mut needs_triggers = false;
    let mut needs_modifiers = false;
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
        "country_effect" | "country_effect_candidate" | "state_effect_candidate" => {
            Some((key.to_string(), "effect"))
        }
        "trigger" | "trigger_candidate" => Some((key.to_string(), "trigger")),
        "idea_modifier" | "idea_modifier_candidate" => Some((key.to_string(), "modifier")),
        _ => None,
    }
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
            errors.push(format!(
                "{context}: `{}` still contains placeholder code `{}`; resolve IDs/numbers from the mod index before final generation",
                suggestion.source, suggestion.code
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
    for suggestion in suggestions {
        if suggestion.code.contains('<') || suggestion.code.contains('>') {
            continue;
        }
        match suggestion.kind.as_str() {
            "country_effect" | "country_effect_candidate" | "state_effect_candidate"
                if !index.effects.is_empty() =>
            {
                if let Some(key) = assignment_key(&suggestion.code) {
                    if !index.effects.contains(key) {
                        let related = related_code_symbols_text(index, key, Some("effect"));
                        errors.push(format!(
                            "{context}: `{}` maps to unindexed effect `{key}`; verify it with `check-code-symbol --kind effect` before final generation{}",
                            suggestion.source, related
                        ));
                    }
                    if key == "add_resource" && !index.resources.is_empty() {
                        if let Some(resource) = code_assignment_value(&suggestion.code, "type") {
                            if !index.resources.contains(resource) {
                                let related =
                                    related_code_symbols_text(index, resource, Some("resource"));
                                errors.push(format!(
                                    "{context}: `{}` maps to unindexed resource `{resource}`; verify it with `check-code-symbol --kind resource` before final generation{}",
                                    suggestion.source, related
                                ));
                            }
                        }
                    }
                    if key == "add_building_construction" && !index.buildings.is_empty() {
                        if let Some(building) = code_assignment_value(&suggestion.code, "type") {
                            if !index.buildings.contains(building) {
                                let related =
                                    related_code_symbols_text(index, building, Some("building"));
                                errors.push(format!(
                                    "{context}: `{}` maps to unindexed building `{building}`; verify it with `check-code-symbol --kind building` before final generation{}",
                                    suggestion.source, related
                                ));
                            }
                        }
                    }
                }
            }
            "trigger" | "trigger_candidate" if !index.triggers.is_empty() => {
                if let Some(key) = assignment_key(&suggestion.code) {
                    if !index.triggers.contains(key) {
                        let related = related_code_symbols_text(index, key, Some("trigger"));
                        errors.push(format!(
                            "{context}: `{}` maps to unindexed trigger `{key}`; verify it with `check-code-symbol --kind trigger` before final generation{}",
                            suggestion.source, related
                        ));
                    }
                }
            }
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
    } else if text.contains("和平") || text.contains("无战争") {
        out.push(Suggestion::new("trigger", "has_war = no", text, ""));
    } else if text.contains("战争中") || text.contains("正在战争") {
        out.push(Suggestion::new("trigger", "has_war = yes", text, ""));
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
        v if v.contains("省份") || v.contains("州") || v.eq_ignore_ascii_case("state_event") => {
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
