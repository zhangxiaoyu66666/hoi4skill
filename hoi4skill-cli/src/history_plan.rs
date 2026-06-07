//! Evidence-first planning for history, state, and province edits.
//!
//! Directly rewriting `history/states` is one of the easiest ways for an AI
//! authoring flow to damage a HOI4 mod. This command turns the research rules
//! into a machine-readable gate: report what is actually observed, accept only
//! explicit IDs or indexed facts, and prefer state-scoped helpers when the
//! evidence is not strong enough for a direct history edit.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_plan_history_edit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = map
        .positionals
        .first()
        .cloned()
        .or_else(|| map.values.get("mod-root").cloned())
        .ok_or_else(|| "missing mod root".to_string())?;
    let root = normalize_path(&root)?;
    let text = history_plan_input_text(&map)?;
    let dependency_roots = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_roots))
        .transpose()?;
    let plan =
        render_history_edit_plan(&root, &dependency_roots, game_index.as_ref(), &map, &text)?;
    write_or_print(&plan, value(&map, "output"))
}

pub(crate) fn history_plan_input_text(map: &ArgMap) -> Result<String, String> {
    if let Some(input) = value(map, "input") {
        return read_utf8_lossy(&normalize_path(input)?);
    }
    Ok(value(map, "text").unwrap_or("").to_string())
}

pub(crate) fn render_history_edit_plan(
    root: &Path,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
    map: &ArgMap,
    text: &str,
) -> Result<String, String> {
    let local_states = scan_history_state_styles(root)?;
    let local_provinces = scan_province_definitions(root)?;
    let dependency_states = scan_dependency_history_states(dependency_roots)?;
    let dependency_provinces = scan_dependency_province_definitions(dependency_roots)?;
    let requested_state_id =
        option_i64(map, "state-id").or_else(|| first_labeled_number(text, "state"));
    let requested_province_id =
        option_i64(map, "province-id").or_else(|| first_labeled_number(text, "province"));
    let requested_capital_id =
        option_i64(map, "capital").or_else(|| option_i64(map, "capital-province-id"));
    let requested_tag = value(map, "tag")
        .map(str::to_string)
        .or_else(|| first_tag(text));
    let target_name = value(map, "target")
        .map(str::to_string)
        .or_else(|| state_name_hint(text));
    let direct_requested = map.flags.contains("direct-history-edit")
        || history_text_contains_any(
            text,
            &[
                "history/states",
                "开局",
                "初始",
                "owner",
                "controller",
                "胜利点",
                "victory point",
            ],
        );
    let reward_like = history_text_contains_any(
        text,
        &[
            "国策奖励",
            "奖励",
            "完成后",
            "临时",
            "即时",
            "工厂",
            "资源",
            "核心",
        ],
    ) && !direct_requested;

    let local_state_match = requested_state_id.and_then(|id| find_state_by_id(&local_states, id));
    let dependency_state_match =
        requested_state_id.and_then(|id| find_dependency_state_by_id(&dependency_states, id));
    let game_state_known = requested_state_id
        .map(|id| game_index.is_some_and(|index| index.state_ids.contains(&id)))
        .unwrap_or(false);
    let province_known = requested_province_id
        .map(|id| {
            province_id_known(
                id,
                &local_states,
                &local_provinces,
                &dependency_states,
                &dependency_provinces,
                game_index,
            )
        })
        .unwrap_or(false);
    let capital_known = requested_capital_id
        .map(|id| {
            province_id_known(
                id,
                &local_states,
                &local_provinces,
                &dependency_states,
                &dependency_provinces,
                game_index,
            )
        })
        .unwrap_or(false);
    let capital_hits_state_id = requested_capital_id
        .map(|id| state_id_known(id, &local_states, &dependency_states, game_index))
        .unwrap_or(false);

    let mut warnings = Vec::new();
    let mut skipped = Vec::new();
    let mut recommended_strategy = "state_scoped_scripted_effect";
    let mut direct_history_edit_allowed = false;

    if local_states.is_empty() && dependency_states.is_empty() && game_index.is_none() {
        warnings.push("local state/province facts are unknown; no history/states, dependency state files, or game index were provided".to_string());
        skipped.push("direct history/states edit skipped until state/province facts are indexed or supplied explicitly".to_string());
    }
    if dependency_roots.is_empty() && looks_like_submod(root)? {
        warnings.push("descriptor declares dependencies, but no --mod-path dependency roots were provided; inherited state/province facts remain unknown".to_string());
    }
    if requested_capital_id.is_some() && !capital_known {
        warnings
            .push("capital was supplied but is not verified as a known province id".to_string());
    }
    if requested_capital_id.is_some() && capital_hits_state_id {
        warnings.push("capital value also matches a known state id; HOI4 country history capital expects a province id, so verify manually before writing".to_string());
    }
    if requested_province_id.is_some() && !province_known {
        warnings.push("province id was supplied but is not present in observed province lists or province definitions".to_string());
    }
    if requested_state_id.is_some()
        && local_state_match.is_none()
        && dependency_state_match.is_none()
        && !game_state_known
    {
        warnings.push("state id was supplied but is not present in local states, dependency states, or the game index".to_string());
    }
    if target_name.is_some() && requested_state_id.is_none() && requested_province_id.is_none() {
        warnings.push("place/name hint was found but no explicit state/province id was supplied; do not infer IDs from localisation or Chinese place names".to_string());
    }

    if direct_requested {
        if local_state_match.is_some() {
            recommended_strategy = "direct_local_history_state_edit";
            direct_history_edit_allowed = true;
        } else if dependency_state_match.is_some() {
            recommended_strategy = "submod_override_requires_explicit_user_approval";
            skipped.push("dependency state matched, but copying/overriding dependency or vanilla history/states should be reported before writing".to_string());
        } else {
            recommended_strategy = "blocked_until_state_file_verified";
            skipped.push(
                "direct edit requested but no local target history/states file was verified"
                    .to_string(),
            );
        }
    } else if reward_like {
        recommended_strategy = "state_scoped_scripted_effect";
    }

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.history_edit_plan.v1\",\n");
    out.push_str(&format!(
        "  \"mod_root\": {},\n",
        json_str(&root.display().to_string())
    ));
    out.push_str(&format!("  \"request_text\": {},\n", json_str(text)));
    out.push_str(&format!(
        "  \"requested\": {{\"tag\": {}, \"target_hint\": {}, \"state_id\": {}, \"province_id\": {}, \"capital_province_id\": {}}},\n",
        json_optional_str(requested_tag.as_deref()),
        json_optional_str(target_name.as_deref()),
        json_optional_i64(requested_state_id),
        json_optional_i64(requested_province_id),
        json_optional_i64(requested_capital_id)
    ));
    out.push_str(&format!(
        "  \"evidence\": {{\"local_history_states\": {}, \"local_province_definitions\": {}, \"dependency_history_states\": {}, \"dependency_province_definitions\": {}, \"game_index_available\": {}, \"game_index_state_count\": {}, \"game_index_province_count\": {}}},\n",
        history_states_json(&local_states),
        province_definitions_json(&local_provinces),
        dependency_history_states_json(&dependency_states),
        dependency_province_definitions_json(&dependency_provinces),
        json_bool(game_index.is_some()),
        game_index.map(|index| index.state_ids.len()).unwrap_or(0),
        game_index.map(|index| index.province_ids.len()).unwrap_or(0)
    ));
    out.push_str(&format!(
        "  \"checks\": {{\"state_id_known\": {}, \"state_file_local\": {}, \"state_file_dependency\": {}, \"province_id_known\": {}, \"capital_province_id_known\": {}, \"capital_value_also_state_id\": {}}},\n",
        json_bool(requested_state_id.is_some_and(|id| state_id_known(id, &local_states, &dependency_states, game_index))),
        json_optional_str(local_state_match.map(|state| state.file.as_str())),
        json_optional_str(dependency_state_match.as_ref().map(|state| state.state.file.as_str())),
        json_bool(province_known),
        json_bool(capital_known),
        json_bool(capital_hits_state_id)
    ));
    out.push_str(&format!(
        "  \"decision\": {{\"recommended_strategy\": {}, \"direct_history_edit_allowed\": {}, \"safe_generated_targets\": {}, \"warnings\": {}, \"skipped\": {}}},\n",
        json_str(recommended_strategy),
        json_bool(direct_history_edit_allowed),
        json_array(&safe_history_targets(value(map, "prefix").unwrap_or("mod"), recommended_strategy)),
        json_array(&warnings),
        json_array(&skipped)
    ));
    out.push_str(&format!(
        "  \"prompt_rules\": {}\n",
        json_array(&history_prompt_rules())
    ));
    out.push_str("}\n");
    Ok(out)
}

#[derive(Clone)]
pub(crate) struct DependencyHistoryState {
    pub(crate) root: String,
    pub(crate) state: HistoryStateStyle,
}

#[derive(Clone)]
pub(crate) struct DependencyProvinceDefinition {
    pub(crate) root: String,
    pub(crate) definition: ProvinceDefinitionSummary,
}

pub(crate) fn scan_dependency_history_states(
    roots: &[PathBuf],
) -> Result<Vec<DependencyHistoryState>, String> {
    let mut out = Vec::new();
    for root in roots {
        for state in scan_history_state_styles(root)? {
            out.push(DependencyHistoryState {
                root: root.display().to_string(),
                state,
            });
        }
    }
    Ok(out)
}

pub(crate) fn scan_dependency_province_definitions(
    roots: &[PathBuf],
) -> Result<Vec<DependencyProvinceDefinition>, String> {
    let mut out = Vec::new();
    for root in roots {
        for definition in scan_province_definitions(root)? {
            out.push(DependencyProvinceDefinition {
                root: root.display().to_string(),
                definition,
            });
        }
    }
    Ok(out)
}

pub(crate) fn dependency_history_states_json(values: &[DependencyHistoryState]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| {
                format!(
                    "{{\"root\": {}, \"state\": {}}}",
                    json_str(&value.root),
                    history_states_json(std::slice::from_ref(&value.state))
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn dependency_province_definitions_json(
    values: &[DependencyProvinceDefinition],
) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| {
                format!(
                    "{{\"root\": {}, \"definition\": {}}}",
                    json_str(&value.root),
                    province_definitions_json(std::slice::from_ref(&value.definition))
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn option_i64(map: &ArgMap, key: &str) -> Option<i64> {
    value(map, key).and_then(parse_int)
}

pub(crate) fn first_labeled_number(text: &str, label: &str) -> Option<i64> {
    let lower = text.to_ascii_lowercase();
    let labels = match label {
        "state" => ["state_id", "state id", "州id", "州 id", "地区id", "地区 id"],
        "province" => [
            "province_id",
            "province id",
            "省份id",
            "省份 id",
            "省id",
            "省 id",
        ],
        _ => return None,
    };
    for label in labels {
        let Some(idx) = lower.find(label) else {
            continue;
        };
        if let Some(number) = first_number_after(&text[idx + label.len()..]) {
            return Some(number);
        }
    }
    None
}

pub(crate) fn first_number_after(text: &str) -> Option<i64> {
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || (current.is_empty() && ch == '-') {
            current.push(ch);
        } else if !current.is_empty() {
            break;
        }
    }
    current.parse::<i64>().ok()
}

pub(crate) fn first_tag(text: &str) -> Option<String> {
    for token in token_candidates(text) {
        if looks_like_tag(token) {
            return Some(token.to_string());
        }
    }
    None
}

pub(crate) fn state_name_hint(text: &str) -> Option<String> {
    for token in token_candidates(text) {
        if token.starts_with("STATE_") {
            return Some(token.to_string());
        }
    }
    None
}

pub(crate) fn history_text_contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(crate) fn find_state_by_id(
    states: &[HistoryStateStyle],
    id: i64,
) -> Option<&HistoryStateStyle> {
    states.iter().find(|state| state.id == Some(id))
}

pub(crate) fn find_dependency_state_by_id(
    states: &[DependencyHistoryState],
    id: i64,
) -> Option<DependencyHistoryState> {
    states
        .iter()
        .find(|state| state.state.id == Some(id))
        .cloned()
}

pub(crate) fn state_id_known(
    id: i64,
    local_states: &[HistoryStateStyle],
    dependency_states: &[DependencyHistoryState],
    game_index: Option<&GameIndex>,
) -> bool {
    find_state_by_id(local_states, id).is_some()
        || find_dependency_state_by_id(dependency_states, id).is_some()
        || game_index.is_some_and(|index| index.state_ids.contains(&id))
}

pub(crate) fn province_id_known(
    id: i64,
    local_states: &[HistoryStateStyle],
    local_provinces: &[ProvinceDefinitionSummary],
    dependency_states: &[DependencyHistoryState],
    dependency_provinces: &[DependencyProvinceDefinition],
    game_index: Option<&GameIndex>,
) -> bool {
    local_states
        .iter()
        .any(|state| state.province_sample.contains(&id))
        || local_provinces
            .iter()
            .any(|definition| definition.sample_ids.contains(&id))
        || dependency_states
            .iter()
            .any(|state| state.state.province_sample.contains(&id))
        || dependency_provinces
            .iter()
            .any(|definition| definition.definition.sample_ids.contains(&id))
        || game_index.is_some_and(|index| index.province_ids.contains(&id))
}

pub(crate) fn looks_like_submod(root: &Path) -> Result<bool, String> {
    let descriptor = root.join("descriptor.mod");
    if !descriptor.exists() {
        return Ok(false);
    }
    let text = read_utf8_lossy(&descriptor)?;
    Ok(!descriptor_list_values(&text, "dependencies").is_empty())
}

pub(crate) fn safe_history_targets(prefix: &str, strategy: &str) -> Vec<String> {
    match strategy {
        "state_scoped_scripted_effect" => {
            vec![format!(
                "common/scripted_effects/{prefix}_state_effects.txt"
            )]
        }
        "direct_local_history_state_edit" => vec!["verified local history/states file".to_string()],
        "submod_override_requires_explicit_user_approval" => {
            vec![
                "new local history/states override only after user confirms target state file"
                    .to_string(),
            ]
        }
        _ => Vec::new(),
    }
}

pub(crate) fn history_prompt_rules() -> Vec<String> {
    vec![
        "capital in history/countries is a province id, not a state id".to_string(),
        "do not infer state or province ids from Chinese place names, STATE_* keys, focus text, or localisation alone".to_string(),
        "when local state/province facts are missing, report them as unknown and request --game-root, --mod-path, or explicit ids".to_string(),
        "prefer state-scoped scripted effects for focus rewards or temporary gameplay changes".to_string(),
        "edit history/states only for start-date map setup and only after verifying the exact local state file".to_string(),
        "when a dependency state is found, report that a submod override may copy or replace dependency behavior before writing".to_string(),
    ]
}
