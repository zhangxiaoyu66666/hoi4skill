//! Diplomacy, wargoal, and AI strategy planning gates.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_diplomatic_effect_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = diplomacy_game_index(&map)?;
    let iterator = value(&map, "iterator").unwrap_or("every_other_country");
    let condition = value(&map, "condition")
        .map(str::to_string)
        .unwrap_or_else(|| infer_diplomacy_condition(&text).to_string());
    let wargoal_type = value(&map, "wargoal-type")
        .or_else(|| value(&map, "type"))
        .unwrap_or("annex_everything");
    let target = value(&map, "target").unwrap_or("PREV");
    let scope = value(&map, "scope").unwrap_or("ROOT");
    let strategy_type = value(&map, "strategy-type").unwrap_or("declare_war");
    let mut blockers = Vec::new();
    validate_iterator(iterator, &mut blockers);
    validate_trigger(&index, &condition, &mut blockers);
    validate_trigger(&index, "can_declare_war_on", &mut blockers);
    validate_effect(&index, "create_wargoal", &mut blockers);
    validate_effect(&index, "add_ai_strategy", &mut blockers);
    validate_wargoal_type(&index, wargoal_type, &mut blockers);
    validate_ai_strategy_type(strategy_type, &mut blockers);
    validate_scope_token(scope, &mut blockers);
    validate_target_token(&index, target, &mut blockers);
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"iterator\": {},\n  \"condition\": {},\n  \"scope\": {},\n  \"target\": {},\n  \"wargoal_type\": {},\n  \"ai_strategy_type\": {},\n  \"clausewitz_shape\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.diplomatic_effect_plan.v1"),
        json_bool(ok),
        json_str(if ok {
            "diplomatic_plan_ready"
        } else {
            "blocked"
        }),
        json_str(&text),
        json_str(iterator),
        json_str(&condition),
        json_str(scope),
        json_str(target),
        json_str(wargoal_type),
        json_str(strategy_type),
        json_str(&diplomacy_clausewitz_shape(
            iterator,
            &condition,
            scope,
            target,
            wargoal_type,
            strategy_type,
        )),
        json_array(&blockers),
        json_str("conditional diplomacy must use iterator + limit + verified effects/triggers/wargoal types; do not collapse it into a fixed TAG effect")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_iterator_diplomacy_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = diplomacy_game_index(&map)?;
    let iterator = require_value(&map, "iterator")?;
    let condition = require_value(&map, "condition")?;
    let effect = require_value(&map, "effect")?;
    let target = value(&map, "target").unwrap_or("PREV");
    let scope = value(&map, "scope").unwrap_or("ROOT");
    let wargoal_type = value(&map, "wargoal-type").unwrap_or("annex_everything");
    let mut blockers = Vec::new();
    validate_iterator(&iterator, &mut blockers);
    validate_trigger(&index, &condition, &mut blockers);
    validate_effect(&index, &effect, &mut blockers);
    if effect == "create_wargoal" {
        validate_wargoal_type(&index, wargoal_type, &mut blockers);
    }
    validate_scope_token(scope, &mut blockers);
    validate_target_token(&index, target, &mut blockers);
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"iterator\": {},\n  \"condition\": {},\n  \"effect\": {},\n  \"scope\": {},\n  \"target\": {},\n  \"wargoal_type\": {},\n  \"scope_stack\": {},\n  \"clausewitz_shape\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.iterator_diplomacy_plan.v1"),
        json_bool(ok),
        json_str(if ok {
            "iterator_diplomacy_ready"
        } else {
            "blocked"
        }),
        json_str(&iterator),
        json_str(&condition),
        json_str(&effect),
        json_str(scope),
        json_str(target),
        json_str(wargoal_type),
        json_array(&[
            "ROOT remains the caller country".to_string(),
            "PREV is the iterated country inside the nested ROOT block".to_string(),
        ]),
        json_str(&iterator_clausewitz_shape(
            &iterator,
            &condition,
            &effect,
            scope,
            target,
            wargoal_type,
        )),
        json_array(&blockers),
        json_array(&[
            "batch diplomacy must use iterator + limit".to_string(),
            "condition must be a registered trigger".to_string(),
            "effect must be a registered effect".to_string(),
        ])
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_ai_strategy_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = diplomacy_game_index(&map)?;
    let strategy_type = require_value(&map, "type")?;
    let target = require_value(&map, "target")?;
    let scope = value(&map, "scope").unwrap_or("ROOT");
    let value_amount = value(&map, "value").unwrap_or("500");
    let mut blockers = Vec::new();
    validate_effect(&index, "add_ai_strategy", &mut blockers);
    validate_ai_strategy_type(&strategy_type, &mut blockers);
    validate_scope_token(scope, &mut blockers);
    validate_target_token(&index, &target, &mut blockers);
    if value_amount.parse::<i64>().is_err() {
        blockers.push(format!(
            "ai strategy value `{value_amount}` is not an integer"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"type\": {},\n  \"scope\": {},\n  \"target\": {},\n  \"value\": {},\n  \"clausewitz_shape\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.ai_strategy_audit.v1"),
        json_bool(ok),
        json_str(if ok { "ai_strategy_ok" } else { "blocked" }),
        json_str(&strategy_type),
        json_str(scope),
        json_str(&target),
        json_str(value_amount),
        json_str(&format!(
            "{scope} = {{ add_ai_strategy = {{ type = {strategy_type} id = {target} value = {value_amount} }} }}"
        )),
        json_array(&blockers),
        json_str("AI strategy plans must use add_ai_strategy in a country scope with a verified target token")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_war_goal_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = diplomacy_game_index(&map)?;
    let target_filter = require_value(&map, "target-filter")?;
    let wargoal_type = require_value(&map, "type")?;
    let target = value(&map, "target").unwrap_or("PREV");
    let mut blockers = Vec::new();
    validate_effect(&index, "create_wargoal", &mut blockers);
    validate_trigger(&index, &target_filter, &mut blockers);
    validate_wargoal_type(&index, &wargoal_type, &mut blockers);
    validate_target_token(&index, target, &mut blockers);
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"target_filter\": {},\n  \"type\": {},\n  \"target\": {},\n  \"clausewitz_shape\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.war_goal_plan.v1"),
        json_bool(ok),
        json_str(if ok { "wargoal_plan_ready" } else { "blocked" }),
        json_str(&target_filter),
        json_str(&wargoal_type),
        json_str(target),
        json_str(&format!(
            "every_other_country = {{ limit = {{ {target_filter} = yes }} ROOT = {{ create_wargoal = {{ target = {target} type = {wargoal_type} }} }} }}"
        )),
        json_array(&blockers),
        json_str("wargoal plans validate target filters and wargoal type IDs before create_wargoal can be assembled")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn diplomacy_game_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn infer_diplomacy_condition(text: &str) -> &'static str {
    if text.contains("中国") {
        "is_chinese_tag"
    } else if text.contains("宣战") {
        "can_declare_war_on"
    } else {
        "always"
    }
}

fn validate_iterator(iterator: &str, blockers: &mut Vec<String>) {
    if !matches!(
        iterator,
        "every_country" | "every_other_country" | "every_enemy_country" | "every_ally_country"
    ) {
        blockers.push(format!(
            "iterator `{iterator}` is not in the diplomacy whitelist"
        ));
    }
}

fn validate_trigger(index: &GameIndex, trigger: &str, blockers: &mut Vec<String>) {
    if !index.triggers.contains(trigger) {
        blockers.push(format!("trigger `{trigger}` is not indexed"));
    }
}

fn validate_effect(index: &GameIndex, effect: &str, blockers: &mut Vec<String>) {
    if !index.effects.contains(effect) {
        blockers.push(format!("effect `{effect}` is not indexed"));
    }
}

fn validate_wargoal_type(index: &GameIndex, wargoal_type: &str, blockers: &mut Vec<String>) {
    if !index.wargoal_types.contains(wargoal_type) {
        blockers.push(format!("wargoal type `{wargoal_type}` is not indexed"));
    }
}

fn validate_ai_strategy_type(strategy_type: &str, blockers: &mut Vec<String>) {
    if !matches!(
        strategy_type,
        "declare_war" | "befriend" | "protect" | "conquer" | "antagonize" | "ignore"
    ) {
        blockers.push(format!(
            "AI strategy type `{strategy_type}` is not whitelisted"
        ));
    }
}

fn validate_scope_token(scope: &str, blockers: &mut Vec<String>) {
    if !matches!(scope, "ROOT" | "THIS" | "FROM" | "PREV") && !looks_like_tag(scope) {
        blockers.push(format!(
            "scope `{scope}` is not ROOT/THIS/FROM/PREV or a country tag"
        ));
    }
}

fn validate_target_token(index: &GameIndex, target: &str, blockers: &mut Vec<String>) {
    if matches!(target, "ROOT" | "THIS" | "FROM" | "PREV") {
        return;
    }
    if looks_like_tag(target) {
        if !index.country_tags.contains(target) {
            blockers.push(format!("target country tag `{target}` is not indexed"));
        }
        return;
    }
    blockers.push(format!(
        "target `{target}` is not ROOT/THIS/FROM/PREV or an indexed country tag"
    ));
}

fn diplomacy_clausewitz_shape(
    iterator: &str,
    condition: &str,
    scope: &str,
    target: &str,
    wargoal_type: &str,
    strategy_type: &str,
) -> String {
    format!(
        "{iterator} = {{ limit = {{ {condition} = yes can_declare_war_on = ROOT }} {scope} = {{ add_ai_strategy = {{ type = {strategy_type} id = {target} value = 500 }} create_wargoal = {{ target = {target} type = {wargoal_type} }} }} }}"
    )
}

fn iterator_clausewitz_shape(
    iterator: &str,
    condition: &str,
    effect: &str,
    scope: &str,
    target: &str,
    wargoal_type: &str,
) -> String {
    if effect == "create_wargoal" {
        format!(
            "{iterator} = {{ limit = {{ {condition} = yes }} {scope} = {{ create_wargoal = {{ target = {target} type = {wargoal_type} }} }} }}"
        )
    } else {
        format!("{iterator} = {{ limit = {{ {condition} = yes }} {scope} = {{ {effect} = {{ id = {target} }} }} }}")
    }
}
