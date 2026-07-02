//! P2 safety gates: registered symbols, legal conditions, scope-compatible
//! modifiers, and iterator plans.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_symbol_registration_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let kind = require_value(&map, "kind")?;
    let symbols = requested_symbols(&map)?;
    let index = safety_game_index(&map)?;
    let catalog = symbol_catalog(&index, &kind)?;
    let rows = symbols
        .iter()
        .map(|symbol| symbol_audit_row(symbol, &kind, catalog))
        .collect::<Vec<_>>();
    let missing_count = rows.iter().filter(|row| !row.ok).count();
    let json = render_symbol_registration_audit_json(&kind, &rows, missing_count);
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && missing_count > 0 {
        return Err(format!(
            "{missing_count} unregistered {kind} symbol(s); fix typos or register the symbols before writing HOI4 code"
        ));
    }
    Ok(())
}

pub(crate) fn cmd_condition_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let conditions = repeated_values(&map, "condition")
        .into_iter()
        .map(str::to_string)
        .chain(value(&map, "trigger").map(str::to_string))
        .collect::<Vec<_>>();
    if conditions.is_empty() {
        return Err("missing --condition or --trigger".to_string());
    }
    let index = safety_game_index(&map)?;
    let rows = conditions
        .iter()
        .map(|condition| symbol_audit_row(condition, "trigger", &index.triggers))
        .collect::<Vec<_>>();
    let missing_count = rows.iter().filter(|row| !row.ok).count();
    let json = render_condition_plan_json(&rows, missing_count, value(&map, "context"));
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && missing_count > 0 {
        return Err("one or more preconditions are not registered HOI4 triggers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_modifier_scope_catalog(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = safety_game_index(&map)?;
    let max_items = parse_usize_option(&map, "max-items", 400)?;
    let rows = index
        .modifiers
        .iter()
        .take(max_items)
        .map(|modifier| {
            let scope = classify_modifier_scope(modifier);
            format!(
                "{{\"modifier\": {}, \"scope_class\": {}, \"shared\": {}, \"rule\": {}}}",
                json_str(modifier),
                json_str(scope),
                json_bool(scope == "shared"),
                json_str(scope_rule(scope))
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"modifier_count\": {},\n  \"reported_count\": {},\n  \"rules\": {},\n  \"modifiers\": [{}]\n}}\n",
        json_str("hoi4skill.modifier_scope_catalog.v1"),
        json_str("scope_catalog_ready"),
        index.modifiers.len(),
        rows.len(),
        json_array(&scope_catalog_rules()),
        rows.join(", ")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_scope_compat_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let modifier = require_value(&map, "modifier")?;
    let container = require_value(&map, "container")?;
    let index = safety_game_index(&map)?;
    let mut blockers = Vec::new();
    if !index.modifiers.contains(&modifier) {
        blockers.push(format!(
            "modifier `{modifier}` is not registered in the code index"
        ));
    }
    let scope_class = classify_modifier_scope(&modifier);
    if !modifier_scope_compatible(
        scope_class,
        &container,
        map.flags.contains("allow-ambiguous"),
    ) {
        blockers.push(format!(
            "modifier `{modifier}` is classified as `{scope_class}` and is not safe in `{container}`"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"modifier\": {},\n  \"container\": {},\n  \"scope_class\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.scope_compat_audit.v1"),
        json_bool(ok),
        json_str(if ok { "scope_compatible" } else { "blocked" }),
        json_str(&modifier),
        json_str(&container),
        json_str(scope_class),
        json_array(&blockers),
        json_str(scope_rule(scope_class))
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_iterator_effect_plan(args: &[String]) -> Result<(), String> {
    render_iterator_gate(args, "hoi4skill.iterator_effect_plan.v1")
}

pub(crate) fn cmd_iterator_scope_audit(args: &[String]) -> Result<(), String> {
    render_iterator_gate(args, "hoi4skill.iterator_scope_audit.v1")
}

pub(crate) fn cmd_weak_ai_regression_suite(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let mut blockers = Vec::new();
    if game_root.as_ref().is_none_or(|root| !root.exists()) {
        blockers.push("weak-ai-regression-suite requires existing --game-root".to_string());
    }
    if mod_root.as_ref().is_none_or(|root| !root.exists()) {
        blockers.push("weak-ai-regression-suite requires existing --mod-root".to_string());
    }
    let cases = weak_ai_regression_cases();
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"case_count\": {},\n  \"cases\": [\n{}\n  ],\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.weak_ai_regression_suite.v1"),
        json_bool(ok),
        json_str(if ok { "weak_ai_regression_suite_ready" } else { "blocked" }),
        json_optional_str(game_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_optional_str(mod_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        cases.len(),
        cases
            .iter()
            .map(weak_ai_regression_case_json)
            .collect::<Vec<_>>()
            .join(",\n"),
        json_array(&blockers),
        json_array(&[
            "hoi4skill large-mod-ai-output-insurance --mod-root <mod> --game-root <hoi4> --text <bad_ai_output> --output .hoi4skill/ai_output_insurance.json".to_string(),
            "hoi4skill validate-repair-context <mod> --game-root <hoi4> --output .hoi4skill/ai_repair_context.json".to_string(),
            "hoi4skill runtime-evidence-gate --mod-root <mod> --game-root <hoi4> --transaction .hoi4skill/mod_transaction.json --require-passed".to_string(),
            "hoi4skill large-mod-release-gate --mod-root <mod> --require-passed --output .hoi4skill/release_gate.json".to_string(),
        ]),
        json_str("Every case is expected to fail before writing; semantic repair must search local indexed code and never invent Clausewitz syntax.")
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_semantic_repair_search(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let query = value(&map, "query")
        .or_else(|| value(&map, "text"))
        .or_else(|| value(&map, "symbol"))
        .map(str::to_string)
        .or_else(|| map.positionals.first().cloned())
        .ok_or_else(|| {
            "semantic-repair-search requires --query, --text, --symbol, or a positional query"
                .to_string()
        })?;
    let index = safety_game_index(&map)?;
    let explicit_kind = value(&map, "kind").map(str::to_string);
    let error_type = semantic_repair_error_type(&query, explicit_kind.as_deref());
    let kind =
        explicit_kind.or_else(|| semantic_repair_kind_for_error(&error_type).map(str::to_string));
    let symbol = value(&map, "symbol")
        .map(str::to_string)
        .or_else(|| semantic_repair_symbol(&query, kind.as_deref()));
    let max_candidates = parse_usize_option(&map, "max-candidates", 8)?;
    let candidates = if let (Some(kind), Some(symbol)) = (kind.as_deref(), symbol.as_deref()) {
        semantic_repair_candidates(&index, kind, symbol, max_candidates)?
    } else {
        Vec::new()
    };
    let blockers = semantic_repair_blockers(kind.as_deref(), symbol.as_deref(), &candidates);
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"query\": {},\n  \"error_type\": {},\n  \"kind\": {},\n  \"symbol\": {},\n  \"local_index_only\": true,\n  \"stores_source_code\": false,\n  \"candidate_count\": {},\n  \"candidates\": [{}],\n  \"blockers\": {},\n  \"repair_rules\": {},\n  \"next_commands\": {}\n}}\n",
        json_str("hoi4skill.semantic_repair_search.v1"),
        json_bool(ok),
        json_str(if ok {
            "semantic_repair_candidates_ready"
        } else {
            "semantic_repair_blocked"
        }),
        json_str(&query),
        json_str(&error_type),
        kind.as_ref()
            .map(|value| json_str(value))
            .unwrap_or_else(|| "null".to_string()),
        symbol
            .as_ref()
            .map(|value| json_str(value))
            .unwrap_or_else(|| "null".to_string()),
        candidates.len(),
        candidates.join(", "),
        json_array(&blockers),
        json_array(&[
            "repair candidates come only from the local game/dependency/target code index".to_string(),
            "if candidates are empty or ambiguous, ask the user or extend the index before writing".to_string(),
            "AI must choose from candidates or structured CLI plans; it must not invent Clausewitz syntax".to_string(),
        ]),
        json_array(&semantic_repair_next_commands(kind.as_deref(), symbol.as_deref()))
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn render_iterator_gate(args: &[String], schema: &str) -> Result<(), String> {
    let map = parse_args(args);
    let iterator = require_value(&map, "iterator")?;
    let condition = require_value(&map, "condition")?;
    let effect = require_value(&map, "effect")?;
    let index = safety_game_index(&map)?;
    let mut blockers = Vec::new();
    if !known_iterator(&iterator) {
        blockers.push(format!(
            "iterator `{iterator}` is not in the P2 iterator whitelist"
        ));
    }
    if !index.triggers.contains(&condition) {
        blockers.push(format!(
            "condition `{condition}` is not a registered trigger"
        ));
    }
    if !index.effects.contains(&effect) {
        blockers.push(format!("effect `{effect}` is not a registered effect"));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"iterator\": {},\n  \"condition\": {},\n  \"effect\": {},\n  \"scope_stack\": {},\n  \"clausewitz_shape\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str(schema),
        json_bool(ok),
        json_str(if ok { "iterator_plan_ready" } else { "blocked" }),
        json_str(&iterator),
        json_str(&condition),
        json_str(&effect),
        json_array(&["ROOT keeps caller scope".to_string(), "PREV points at the iterated object inside nested ROOT blocks".to_string()]),
        json_str(&format!("{iterator} = {{ limit = {{ {condition} = yes }} ROOT = {{ {effect} = {{ target = PREV }} }} }}")),
        json_array(&blockers),
        json_array(&[
            "use iterator + limit for conditional batch effects; do not replace with a fixed TAG unless the user explicitly requested that TAG".to_string(),
            "all conditions must be registered triggers and all actions must be registered effects".to_string(),
        ])
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn semantic_repair_error_type(query: &str, kind: Option<&str>) -> String {
    let lower = query.to_ascii_lowercase();
    if lower.contains("stale") || lower.contains("旧模板") || lower.contains("hash changed") {
        "stale_template".to_string()
    } else if lower.contains("container") || lower.contains("容器") {
        "wrong_container".to_string()
    } else if lower.contains("scope") || lower.contains("作用域") {
        "wrong_scope".to_string()
    } else if lower.contains("runtime") || lower.contains("error.log") {
        "missing_runtime_evidence".to_string()
    } else if lower.contains("sprite") || lower.contains("gfx") || lower.contains("picture") {
        "unknown_sprite".to_string()
    } else if matches!(kind, Some("modifier" | "modifiers")) || lower.contains("modifier") {
        "unknown_modifier".to_string()
    } else if matches!(kind, Some("effect" | "effects")) || lower.contains("effect") {
        "unknown_effect".to_string()
    } else if matches!(kind, Some("trigger" | "triggers")) || lower.contains("trigger") {
        "unknown_trigger".to_string()
    } else if matches!(kind, Some("tag" | "country_tag" | "country_tags")) || lower.contains("tag")
    {
        "unknown_tag".to_string()
    } else if matches!(kind, Some("state" | "province"))
        || lower.contains("state")
        || lower.contains("province")
    {
        "unknown_map_symbol".to_string()
    } else {
        "unknown_symbol".to_string()
    }
}

fn semantic_repair_kind_for_error(error_type: &str) -> Option<&'static str> {
    match error_type {
        "unknown_modifier" => Some("modifier"),
        "unknown_effect" => Some("effect"),
        "unknown_trigger" => Some("trigger"),
        "unknown_sprite" => Some("sprite"),
        "unknown_tag" => Some("tag"),
        "unknown_map_symbol" => Some("state"),
        _ => None,
    }
}

fn semantic_repair_symbol(query: &str, kind: Option<&str>) -> Option<String> {
    if let Some(value) = semantic_backticked_value(query) {
        return Some(value);
    }
    let lower = query.to_ascii_lowercase();
    for marker in [
        "unknown modifier ",
        "unknown effect ",
        "unknown trigger ",
        "unknown sprite ",
        "unknown tag ",
    ] {
        if let Some(start) = lower.find(marker) {
            return semantic_word_at(query, start + marker.len());
        }
    }
    if kind.is_some() {
        query
            .split_whitespace()
            .find(|token| {
                token
                    .chars()
                    .any(|ch| ch == '_' || ch.is_ascii_alphanumeric())
            })
            .map(|token| token.trim_matches(['`', '"', '\'', ',', ';']).to_string())
            .filter(|token| !token.is_empty())
    } else {
        None
    }
}

fn semantic_backticked_value(query: &str) -> Option<String> {
    let start = query.find('`')?;
    let rest = &query[start + 1..];
    let end = rest.find('`')?;
    Some(rest[..end].trim().to_string()).filter(|value| !value.is_empty())
}

fn semantic_word_at(query: &str, start: usize) -> Option<String> {
    query[start..]
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | ':' | '：'))
        .next()
        .map(|value| value.trim_matches(['`', '"', '\'']).to_string())
        .filter(|value| !value.is_empty())
}

fn semantic_repair_candidates(
    index: &GameIndex,
    kind: &str,
    symbol: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    let catalog = symbol_catalog(index, kind)?;
    let mut symbols = Vec::new();
    if catalog.contains(symbol) {
        symbols.push(symbol.to_string());
    }
    for candidate in nearest_symbols(symbol, catalog, limit) {
        if !symbols.contains(&candidate) {
            symbols.push(candidate);
        }
    }
    Ok(symbols
        .into_iter()
        .take(limit)
        .map(|candidate| {
            format!(
                "{{\"kind\": {}, \"symbol\": {}, \"source\": \"local_code_index\"}}",
                json_str(kind),
                json_str(&candidate)
            )
        })
        .collect())
}

fn semantic_repair_blockers(
    kind: Option<&str>,
    symbol: Option<&str>,
    candidates: &[String],
) -> Vec<String> {
    let mut blockers = Vec::new();
    if kind.is_none() {
        blockers.push(
            "could not infer symbol kind; pass --kind or run validate-repair-context".to_string(),
        );
    }
    if symbol.is_none() {
        blockers
            .push("could not infer symbol; pass --symbol or quote the unknown token".to_string());
    }
    if kind.is_some() && symbol.is_some() && candidates.is_empty() {
        blockers.push(
            "no local indexed candidates found; ask user or refresh knowledge/index before writing"
                .to_string(),
        );
    }
    blockers
}

fn semantic_repair_next_commands(kind: Option<&str>, symbol: Option<&str>) -> Vec<String> {
    let mut commands = Vec::new();
    if let (Some(kind), Some(symbol)) = (kind, symbol) {
        commands.push(format!(
            "hoi4skill symbol-registration-audit --kind {kind} --symbol {symbol} --game-root <HOI4 root> --require-passed"
        ));
    }
    commands.push(
        "hoi4skill validate-repair-context <mod> --game-root <HOI4 root> --strict-code-index --output .hoi4skill/ai_repair_context.json"
            .to_string(),
    );
    commands
}

struct WeakAiRegressionCase {
    id: &'static str,
    lane: &'static str,
    bad_input: &'static str,
    expected_blocker: &'static str,
    repair_gate: &'static str,
}

fn weak_ai_regression_cases() -> Vec<WeakAiRegressionCase> {
    vec![
        WeakAiRegressionCase {
            id: "unknown_modifier_typo",
            lane: "symbol",
            bad_input: "political_p_gain = 0.05",
            expected_blocker: "modifier is not registered; suggest indexed political_power_gain only if present",
            repair_gate: "symbol-registration-audit + validate-repair-context",
        },
        WeakAiRegressionCase {
            id: "wrong_container_mio_modifier",
            lane: "scope_container",
            bad_input: "MIO or state scoped modifier placed in national spirit",
            expected_blocker: "scope/container mismatch",
            repair_gate: "scope-container-contract + scope-compat-audit",
        },
        WeakAiRegressionCase {
            id: "fixed_tag_instead_of_iterator",
            lane: "iterator",
            bad_input: "CHI = { create_wargoal = { target = PRC type = annex_everything } }",
            expected_blocker: "conditional batch request must use iterator + limit when user asked all matching countries",
            repair_gate: "iterator-effect-plan",
        },
        WeakAiRegressionCase {
            id: "guessed_state_or_province",
            lane: "map_history",
            bad_input: "Jiangxi = owner PRC without indexed state/province evidence",
            expected_blocker: "place name must resolve to indexed state/province candidates or ask the user",
            repair_gate: "map-intent-plan + province-query",
        },
        WeakAiRegressionCase {
            id: "broken_localisation_token",
            lane: "localisation",
            bad_input: "translated [ROOT.GetNameDef] or color token changed",
            expected_blocker: "token preservation diff",
            repair_gate: "localisation-token-check",
        },
        WeakAiRegressionCase {
            id: "missing_flag_triplet",
            lane: "asset",
            bad_input: "country/cosmetic flag referenced without normal medium small tga assets",
            expected_blocker: "flag triplet missing",
            repair_gate: "flag-image-import + gfx-audit",
        },
        WeakAiRegressionCase {
            id: "dead_event_route",
            lane: "route",
            bad_input: "event id exists but no focus/decision/on_action/event source reaches it",
            expected_blocker: "dead event or missing incoming route",
            repair_gate: "event-chain-graph + route-blocker-audit",
        },
        WeakAiRegressionCase {
            id: "gui_missing_mount",
            lane: "gui",
            bad_input: "interface window generated without scripted_gui/decision/open route evidence",
            expected_blocker: "GUI mount/open evidence missing",
            repair_gate: "gui-runtime-evidence-contract",
        },
        WeakAiRegressionCase {
            id: "map_topology_without_evidence",
            lane: "map",
            bad_input: "new province or adjacency without definition/bmp/adjacency evidence",
            expected_blocker: "topology manual review blocker",
            repair_gate: "map-topology-gate + map-release-gate",
        },
        WeakAiRegressionCase {
            id: "stale_parent_template",
            lane: "knowledge",
            bad_input: "old parent-mod template reused after dependency hash changed",
            expected_blocker: "stale knowledge/template hash",
            repair_gate: "knowledge-delta-refresh + stale-plan-gate",
        },
    ]
}

fn weak_ai_regression_case_json(case: &WeakAiRegressionCase) -> String {
    format!(
        "    {{\"id\": {}, \"lane\": {}, \"bad_input\": {}, \"expected_blocker\": {}, \"repair_gate\": {}}}",
        json_str(case.id),
        json_str(case.lane),
        json_str(case.bad_input),
        json_str(case.expected_blocker),
        json_str(case.repair_gate)
    )
}

struct SymbolAuditRow {
    symbol: String,
    kind: String,
    ok: bool,
    suggestions: Vec<String>,
}

fn requested_symbols(map: &ArgMap) -> Result<Vec<String>, String> {
    let mut symbols = repeated_values(map, "symbol")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if symbols.is_empty() {
        if let Some(symbol) = map.positionals.first() {
            symbols.push(symbol.clone());
        }
    }
    if symbols.is_empty() {
        return Err("missing --symbol".to_string());
    }
    Ok(symbols)
}

fn safety_game_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn symbol_catalog<'a>(index: &'a GameIndex, kind: &str) -> Result<&'a BTreeSet<String>, String> {
    match kind {
        "effect" | "effects" => Ok(&index.effects),
        "trigger" | "triggers" | "condition" | "conditions" => Ok(&index.triggers),
        "modifier" | "modifiers" => Ok(&index.modifiers),
        "sprite" | "sprites" => Ok(&index.sprites),
        "tag" | "country_tag" | "country_tags" => Ok(&index.country_tags),
        "technology" | "technologies" => Ok(&index.technologies),
        "idea" | "ideas" | "national_spirit" => Ok(&index.ideas),
        "dynamic_modifier" | "dynamic_modifiers" => Ok(&index.dynamic_modifiers),
        _ => Err(format!(
            "unknown --kind {kind}; expected effect, trigger, modifier, sprite, tag, technology, idea, or dynamic_modifier"
        )),
    }
}

fn symbol_audit_row(symbol: &str, kind: &str, catalog: &BTreeSet<String>) -> SymbolAuditRow {
    SymbolAuditRow {
        symbol: symbol.to_string(),
        kind: kind.to_string(),
        ok: catalog.contains(symbol),
        suggestions: nearest_symbols(symbol, catalog, 5),
    }
}

fn nearest_symbols(symbol: &str, catalog: &BTreeSet<String>, limit: usize) -> Vec<String> {
    let mut scored = catalog
        .iter()
        .map(|candidate| (levenshtein(symbol, candidate), candidate.clone()))
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(limit)
        .filter(|(distance, _)| *distance <= symbol.len().max(6))
        .map(|(_, candidate)| candidate)
        .collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut costs = (0..=b.len()).collect::<Vec<_>>();
    for (i, ca) in a.chars().enumerate() {
        let mut last = i;
        costs[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let old = costs[j + 1];
            costs[j + 1] = if ca == cb {
                last
            } else {
                1 + last.min(old).min(costs[j])
            };
            last = old;
        }
    }
    costs[b.len()]
}

fn render_symbol_registration_audit_json(
    kind: &str,
    rows: &[SymbolAuditRow],
    missing_count: usize,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"kind\": {},\n  \"symbol_count\": {},\n  \"missing_count\": {},\n  \"symbols\": [{}]\n}}\n",
        json_str("hoi4skill.symbol_registration_audit.v1"),
        json_bool(missing_count == 0),
        json_str(if missing_count == 0 { "registered" } else { "missing_symbols" }),
        json_str(kind),
        rows.len(),
        missing_count,
        render_symbol_rows(rows)
    )
}

fn render_condition_plan_json(
    rows: &[SymbolAuditRow],
    missing_count: usize,
    context: Option<&str>,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"context\": {},\n  \"legal_contexts\": {},\n  \"missing_count\": {},\n  \"conditions\": [{}]\n}}\n",
        json_str("hoi4skill.condition_plan.v1"),
        json_bool(missing_count == 0),
        json_str(if missing_count == 0 { "conditions_registered" } else { "missing_conditions" }),
        json_optional_str(context),
        json_array(&["trigger".to_string(), "available".to_string(), "visible".to_string(), "limit".to_string()]),
        missing_count,
        render_symbol_rows(rows)
    )
}

fn render_symbol_rows(rows: &[SymbolAuditRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{{\"symbol\": {}, \"kind\": {}, \"registered\": {}, \"suggestions\": {}}}",
                json_str(&row.symbol),
                json_str(&row.kind),
                json_bool(row.ok),
                json_array(&row.suggestions)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn classify_modifier_scope(modifier: &str) -> &'static str {
    let lower = modifier.to_ascii_lowercase();
    if lower.contains("mio") {
        "mio"
    } else if lower.contains("state")
        || lower.contains("local_")
        || lower.contains("resources")
        || lower.contains("resource")
        || lower.contains("compliance")
        || lower.contains("resistance")
        || lower.contains("building")
    {
        "state"
    } else if lower.contains("stability")
        || lower.contains("political_power")
        || lower.contains("war_support")
        || lower.contains("justify_war_goal")
        || lower.contains("research")
        || lower.contains("production")
        || lower.contains("consumer_goods")
    {
        "country_tag"
    } else {
        "shared"
    }
}

fn modifier_scope_compatible(scope_class: &str, container: &str, allow_ambiguous: bool) -> bool {
    let container = container.to_ascii_lowercase();
    match scope_class {
        "country_tag" => matches!(
            container.as_str(),
            "national_spirit" | "idea" | "dynamic_modifier" | "country" | "country_effect" | "tag"
        ),
        "state" => matches!(
            container.as_str(),
            "state" | "state_modifier" | "state_effect"
        ),
        "mio" => matches!(
            container.as_str(),
            "mio" | "military_industrial_organization"
        ),
        "shared" => allow_ambiguous || !container.is_empty(),
        _ => false,
    }
}

fn scope_rule(scope_class: &str) -> &'static str {
    match scope_class {
        "country_tag" => "country/tag-wide modifier; allowed in national spirits, dynamic modifiers, and country-scope effects",
        "state" => "state/province/local modifier; keep in state-scoped containers and effects",
        "mio" => "MIO modifier; keep in military industrial organization containers",
        "shared" => "ambiguous/shared modifier; require evidence or allow-ambiguous before final write",
        _ => "unknown scope class",
    }
}

fn scope_catalog_rules() -> Vec<String> {
    vec![
        "MIO modifiers stay in MIO containers".to_string(),
        "state/province/local modifiers stay in state-scoped containers".to_string(),
        "country/tag-wide modifiers may be used by national spirits and dynamic modifiers"
            .to_string(),
        "ambiguous shared modifiers require evidence before final write".to_string(),
    ]
}

fn known_iterator(iterator: &str) -> bool {
    matches!(
        iterator,
        "every_country"
            | "every_other_country"
            | "every_enemy_country"
            | "every_ally_country"
            | "every_state"
            | "every_owned_state"
            | "every_controlled_state"
            | "every_core_state"
            | "every_neighbor_state"
    )
}
