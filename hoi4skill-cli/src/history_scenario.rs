//! Composite history scenario planning.
//!
//! This keeps start-date authoring split by HOI4 file scope. The model can
//! describe a start-date country, leader, state, technology, war, and OOB setup,
//! but the CLI must prove the target tag, leader, state, technologies, war
//! template, and OOB evidence before any writer is allowed to assemble code.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
struct ScenarioStateMatch {
    id: i64,
    name_key: Option<String>,
    localized_name: Option<String>,
    file: Option<String>,
    source: String,
    owner: Option<String>,
    controller: Option<String>,
    province_sample: Vec<i64>,
}

#[derive(Clone)]
struct ScenarioLeaderMatch {
    id: String,
    file: String,
    style: String,
    roles: Vec<String>,
    match_reason: String,
}

#[derive(Clone)]
struct ScenarioOobMatch {
    id: String,
    file: Option<String>,
    source: String,
}

pub(crate) fn cmd_history_scenario_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
    let mut roots = vec![game_root.clone()];
    roots.extend(dependency_roots.iter().cloned());
    if let Some(root) = &mod_root {
        roots.push(root.clone());
    }

    let tag = value(&map, "tag")
        .map(str::to_string)
        .or_else(|| infer_history_scenario_tag(&text, &index))
        .unwrap_or_default();
    let enemy_tag = value(&map, "enemy-tag")
        .or_else(|| value(&map, "war-target"))
        .map(str::to_string)
        .or_else(|| infer_history_scenario_enemy_tag(&text, &index, &tag));
    let leader_name = value(&map, "leader-name").map(str::to_string);
    let leader_character = value(&map, "leader-character").map(str::to_string);
    let tech_year = option_i64(&map, "technology-year")
        .or_else(|| option_i64(&map, "tech-year"))
        .or_else(|| infer_history_scenario_year(&text))
        .unwrap_or(1936);
    let state_id = option_i64(&map, "state-id").or_else(|| first_labeled_number(&text, "state"));
    let state_query = value(&map, "state-name")
        .map(str::to_string)
        .or_else(|| infer_history_scenario_state_query(&text, &index));
    let mut state_roots = vec![game_root.clone()];
    state_roots.extend(dependency_roots.iter().cloned());
    let state_match = resolve_history_scenario_state(
        mod_root.as_deref(),
        &state_roots,
        &index,
        state_id,
        state_query.as_deref(),
    )?;
    let technologies = collect_history_scenario_technologies(&roots, tech_year, &index)?;
    let leader_matches = resolve_history_scenario_leader(
        &roots,
        &index,
        leader_name.as_deref(),
        leader_character.as_deref(),
    )?;
    let oob = resolve_history_scenario_oob(&roots, &tag, value(&map, "oob"))?;
    let war_template = find_history_scenario_war_template(&roots)?;

    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    let mut warnings = Vec::new();

    if tag.is_empty() {
        blockers.push("target country tag is not inferred; provide --tag".to_string());
    } else if !index.country_tags.contains(&tag) {
        blockers.push(format!("target country tag `{tag}` is not indexed"));
    }
    if let Some(enemy) = &enemy_tag {
        if !index.country_tags.contains(enemy) {
            blockers.push(format!("enemy country tag `{enemy}` is not indexed"));
        }
    } else if history_scenario_requests_war(&text) {
        blockers.push(
            "start-war request needs --enemy-tag or an inferred indexed enemy tag".to_string(),
        );
    }
    if state_match.is_none() {
        blockers.push("requested state is not verified by --state-id, history/states, or localised STATE_* evidence".to_string());
        questions.push(format!(
            "Which indexed state id should receive {} owner/controller and OOB deployment?",
            if tag.is_empty() { "<TAG>" } else { &tag }
        ));
    }
    if technologies.is_empty() {
        blockers.push(format!(
            "no indexed technologies with start_year/year `{tech_year}` were found"
        ));
    }
    if leader_name.is_some() || leader_character.is_some() {
        if leader_matches.is_empty() {
            blockers.push("requested leader is not verified in common/characters or legacy history leader blocks".to_string());
            questions.push("Provide the existing leader character id or authorize creating a new leader first.".to_string());
        }
    } else if history_scenario_requests_leader(&text) {
        blockers.push(
            "leader request needs --leader-name or --leader-character from indexed local evidence"
                .to_string(),
        );
        questions.push("Which indexed leader should become the active country leader?".to_string());
    } else {
        questions.push("Which leader should become the active country leader?".to_string());
    }
    if history_scenario_requests_war(&text) && war_template.is_none() {
        blockers.push(
            "no history/diplomacy war template was observed; do not invent start-war syntax"
                .to_string(),
        );
    }
    if oob.is_none() {
        blockers.push("no OOB id was supplied or discovered in target country history".to_string());
        questions.push("Which existing OOB should be moved into the verified state, or should a new OOB be explicitly created?".to_string());
    }
    if let Some(state) = &state_match {
        if state.province_sample.is_empty() {
            blockers.push(format!(
                "state `{}` has no province sample for OOB deployment location",
                state.id
            ));
        }
        if state.source != "local_mod" {
            warnings.push("state owner/controller change targets an inherited state; apply must require explicit override approval and changed-file copy evidence".to_string());
        }
    }

    let ok = blockers.is_empty();
    let planned_effects = history_scenario_planned_effects(
        &tag,
        enemy_tag.as_deref(),
        leader_matches.first(),
        state_match.as_ref(),
        tech_year,
        &technologies,
        oob.as_ref(),
    );
    let planned_files = history_scenario_planned_files(
        &tag,
        enemy_tag.as_deref(),
        state_match.as_ref(),
        oob.as_ref(),
    );
    let json = render_history_scenario_plan_json(HistoryScenarioPlanView {
        ok,
        game_root: &game_root,
        mod_root: mod_root.as_deref(),
        text: &text,
        tag: &tag,
        enemy_tag: enemy_tag.as_deref(),
        leader_name: leader_name.as_deref(),
        leader_character: leader_character.as_deref(),
        tech_year,
        state_query: state_query.as_deref(),
        state_match: state_match.as_ref(),
        technologies: &technologies,
        leader_matches: &leader_matches,
        oob: oob.as_ref(),
        war_template: war_template.as_deref(),
        planned_effects: &planned_effects,
        planned_files: &planned_files,
        blockers: &blockers,
        warnings: &warnings,
        questions: &questions,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_scenario_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            target_root
                .join(".hoi4skill")
                .join("history_scenario_apply")
        });
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("history-scenario-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("history-scenario-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.history_scenario_plan.v1\"") {
        blockers.push("input is not a history-scenario-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input plan is not ok; fix blockers before apply".to_string());
    }

    let tag = json_string_field(&plan, "tag").unwrap_or_default();
    let enemy_tag = json_string_field(&plan, "enemy_tag");
    let tech_year = history_scenario_json_i64_field(&plan, "technology_year").unwrap_or(1936);
    let state_id = history_scenario_json_i64_field(&plan, "id");
    let technologies = json_string_array_field(&plan, "technologies");
    let planned_effects = json_string_array_field(&plan, "planned_effects");
    let planned_files = json_string_array_field(&plan, "planned_files");
    let leader_id = first_history_scenario_leader_id(&plan);
    let oob_id = first_history_scenario_oob_id(&plan);

    if tag.is_empty() {
        blockers.push("input plan is missing target tag".to_string());
    }
    if state_id.is_none() {
        blockers.push("input plan is missing verified state id".to_string());
    }
    if technologies.is_empty() {
        blockers.push("input plan is missing verified technologies".to_string());
    }
    if leader_id.is_none() {
        blockers.push("input plan is missing verified leader id".to_string());
    }
    if oob_id.is_none() {
        blockers.push("input plan is missing verified OOB id".to_string());
    }
    if enemy_tag.is_some() && !plan.contains("\"war_template\": \"") {
        blockers.push("input plan is missing observed war template".to_string());
    }

    let prefix = value(&map, "prefix").unwrap_or("history_scenario");
    let write_plan = if blockers.is_empty() {
        history_scenario_apply_write_plan(HistoryScenarioApplyInput {
            prefix,
            target_root: &target_root,
            output_dir: &output_dir,
            input_plan: &input,
            tag: &tag,
            enemy_tag: enemy_tag.as_deref(),
            state_id,
            tech_year,
            technologies: &technologies,
            leader_id: leader_id.as_deref(),
            oob_id: oob_id.as_deref(),
            planned_files: &planned_files,
            planned_effects: &planned_effects,
        })
    } else {
        Vec::new()
    };

    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        for (_, path, _) in &write_plan {
            if path.exists() {
                blockers.push(format!(
                    "transaction target already exists and will not be overwritten: {}",
                    path.display()
                ));
            }
        }
    }
    if blockers.is_empty() {
        match write_history_scenario_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_history_scenario_files(&changed));
                blockers.push(
                    "history scenario transaction failed and rollback was attempted".to_string(),
                );
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }

    let ok = blockers.is_empty();
    let report = history_scenario_apply_json(
        &input,
        &target_root,
        &output_dir,
        ok,
        &changed_files,
        &blockers,
        &rollback_blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !ok) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_transaction_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
    let mut roots = vec![game_root.clone()];
    roots.extend(dependency_roots.iter().cloned());
    if let Some(root) = &mod_root {
        roots.push(root.clone());
    }

    let tag = value(&map, "tag")
        .map(str::to_string)
        .or_else(|| infer_history_scenario_tag(&text, &index))
        .unwrap_or_default();
    let enemy_tag = value(&map, "enemy-tag")
        .or_else(|| value(&map, "war-target"))
        .map(str::to_string)
        .or_else(|| infer_history_scenario_enemy_tag(&text, &index, &tag));
    let leader_name = value(&map, "leader-name").map(str::to_string);
    let leader_character = value(&map, "leader-character").map(str::to_string);
    let tech_year = option_i64(&map, "technology-year")
        .or_else(|| option_i64(&map, "tech-year"))
        .or_else(|| infer_history_scenario_year(&text))
        .unwrap_or(1936);
    let state_id = option_i64(&map, "state-id").or_else(|| first_labeled_number(&text, "state"));
    let state_query = value(&map, "state-name")
        .map(str::to_string)
        .or_else(|| infer_history_scenario_state_query(&text, &index));
    let mut state_roots = vec![game_root.clone()];
    state_roots.extend(dependency_roots.iter().cloned());
    let state_match = resolve_history_scenario_state(
        mod_root.as_deref(),
        &state_roots,
        &index,
        state_id,
        state_query.as_deref(),
    )?;
    let technologies = collect_history_scenario_technologies(&roots, tech_year, &index)?;
    let leader_matches = resolve_history_scenario_leader(
        &roots,
        &index,
        leader_name.as_deref(),
        leader_character.as_deref(),
    )?;
    let oob = resolve_history_scenario_oob(&roots, &tag, value(&map, "oob"))?;
    let war_template = find_history_scenario_war_template(&roots)?;

    let mut blockers = Vec::new();
    if tag.is_empty() || !index.country_tags.contains(&tag) {
        blockers.push("country_history component missing indexed target tag".to_string());
    }
    if state_match.is_none() {
        blockers.push("state_history component missing verified state".to_string());
    }
    if technologies.is_empty() {
        blockers.push(format!(
            "country_history component missing indexed `{tech_year}` technologies"
        ));
    }
    if leader_name.is_some() || leader_character.is_some() {
        if leader_matches.is_empty() {
            blockers.push("country_history component missing verified leader".to_string());
        }
    } else if history_scenario_requests_leader(&text) {
        blockers.push(
            "country_history component missing explicit leader-name or leader-character"
                .to_string(),
        );
    }
    if oob.is_none() {
        blockers.push("oob component missing verified OOB id".to_string());
    }
    if let Some(enemy) = &enemy_tag {
        if !index.country_tags.contains(enemy) {
            blockers.push(format!(
                "diplomacy component enemy tag `{enemy}` is not indexed"
            ));
        }
        if war_template.is_none() {
            blockers.push("diplomacy component missing observed war template".to_string());
        }
    } else if history_scenario_requests_war(&text) {
        blockers.push("diplomacy component missing enemy tag".to_string());
    }
    if let Some(state) = &state_match {
        if state.province_sample.is_empty() {
            blockers.push("oob component missing province inside verified state".to_string());
        }
    }

    let ok = blockers.is_empty();
    let planned_effects = history_scenario_planned_effects(
        &tag,
        enemy_tag.as_deref(),
        leader_matches.first(),
        state_match.as_ref(),
        tech_year,
        &technologies,
        oob.as_ref(),
    );
    let planned_files = history_scenario_planned_files(
        &tag,
        enemy_tag.as_deref(),
        state_match.as_ref(),
        oob.as_ref(),
    );
    let json = render_history_transaction_plan_json(HistoryTransactionPlanView {
        ok,
        game_root: &game_root,
        mod_root: mod_root.as_deref(),
        text: &text,
        tag: &tag,
        enemy_tag: enemy_tag.as_deref(),
        tech_year,
        state_match: state_match.as_ref(),
        technologies: &technologies,
        leader_matches: &leader_matches,
        oob: oob.as_ref(),
        war_template: war_template.as_deref(),
        planned_effects: &planned_effects,
        planned_files: &planned_files,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_transaction_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.history_transaction_plan.v1\"") {
        blockers.push("input is not a history-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("history transaction plan is not ok".to_string());
    }
    for component in ["country_history", "state_history", "oob", "diplomacy"] {
        if !plan.contains(&format!(
            "\"component\": \"{component}\", \"status\": \"ready\""
        )) {
            blockers.push(format!(
                "history transaction component `{component}` is not ready"
            ));
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.history_transaction_audit.v1"),
        json_bool(ok),
        json_str(if ok { "history_transaction_ok" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&blockers),
        json_str("country/state/OOB/diplomacy components must all be ready before a start-date transaction can apply")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_startdate_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let taxonomy_text = value(&map, "taxonomy")
        .map(normalize_path)
        .transpose()?
        .map(|path| read_utf8_lossy(&path).map(|text| (path, text)))
        .transpose()?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.history_transaction_plan.v1\"") {
        blockers.push("input is not a history-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("history transaction plan is not ok".to_string());
    }
    for component in ["country_history", "state_history", "oob"] {
        if !plan.contains(&format!(
            "\"component\": \"{component}\", \"status\": \"ready\""
        )) {
            blockers.push(format!("start-date component `{component}` is not ready"));
        }
    }
    if json_string_field(&plan, "enemy_tag").is_some()
        && !plan.contains("\"component\": \"diplomacy\", \"status\": \"ready\"")
    {
        blockers.push("diplomacy component is not ready for start-war request".to_string());
    }
    let planned_files = json_string_array_field(&plan, "planned_files");
    for required in ["history/countries", "history/states", "history/units"] {
        if !planned_files.iter().any(|file| file.contains(required)) {
            blockers.push(format!("planned_files missing `{required}` surface"));
        }
    }
    if json_string_array_field(&plan, "technologies").is_empty() {
        blockers.push("history transaction has no verified technology list".to_string());
    }
    if first_history_scenario_oob_id(&plan).is_none() {
        blockers.push("history transaction has no verified OOB id".to_string());
    }
    if first_history_scenario_leader_id(&plan).is_none() {
        blockers.push("history transaction has no verified leader id".to_string());
    }
    if let Some((taxonomy_path, taxonomy)) = &taxonomy_text {
        if !taxonomy.contains("\"schema\": \"hoi4skill.unit_taxonomy.v1\"") {
            blockers.push(format!(
                "taxonomy `{}` is not a unit-taxonomy report",
                taxonomy_path.display()
            ));
        }
        if taxonomy.contains("\"class\": \"special_or_unknown\"")
            && !map.flags.contains("allow-unknown-units")
        {
            blockers.push("unit taxonomy contains special_or_unknown entries; confirm unit classes before OOB apply".to_string());
        }
    } else {
        blockers
            .push("missing --taxonomy built from local game/parent unit definitions".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"taxonomy\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.history_startdate_gate.v1"),
        json_bool(ok),
        json_str(if ok { "history_startdate_ready" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_optional_str(
            taxonomy_text
                .as_ref()
                .map(|(path, _)| path.display().to_string())
                .as_deref()
        ),
        json_array(&planned_files),
        json_array(&blockers),
        json_array(&[
            "country/history, state/history, OOB, technology, and diplomacy evidence must be in one transaction before apply".to_string(),
            "unit taxonomy must come from local game/parent/target code; do not hardcode parent-mod special units".to_string(),
            "OOB deployment must use province evidence from the verified target state".to_string(),
        ])
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_startdate_closure_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let reports = repeated_values(&map, "report")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    let mut report_texts = Vec::new();
    for report in &reports {
        report_texts.push((report.clone(), read_utf8_lossy(report)?));
    }

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.history_transaction_plan.v1\"") {
        blockers.push("input is not a history-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("history transaction plan is not ok".to_string());
    }
    for component in ["country_history", "state_history", "oob"] {
        if !plan.contains(&format!(
            "\"component\": \"{component}\", \"status\": \"ready\""
        )) {
            blockers.push(format!(
                "start-date core component `{component}` is not ready"
            ));
        }
    }
    if json_string_field(&plan, "enemy_tag").is_some()
        && !plan.contains("\"component\": \"diplomacy\", \"status\": \"ready\"")
    {
        blockers.push("diplomacy start-war component is not ready".to_string());
    }
    if json_string_array_field(&plan, "technologies").is_empty() {
        blockers.push("technology list is missing from history transaction".to_string());
    }
    if first_history_scenario_leader_id(&plan).is_none() {
        blockers.push("active leader evidence is missing from history transaction".to_string());
    }
    if first_history_scenario_oob_id(&plan).is_none() {
        blockers.push("OOB evidence is missing from history transaction".to_string());
    }

    let has_air_oob = report_texts
        .iter()
        .any(|(_, text)| startdate_report_has_kind(text, "air"));
    let has_naval_oob = report_texts
        .iter()
        .any(|(_, text)| startdate_report_has_kind(text, "naval"));
    let has_bookmark = report_texts
        .iter()
        .any(|(_, text)| text.contains("bookmark") || text.contains("common/bookmarks"));
    let has_stockpile = report_texts.iter().any(|(_, text)| {
        text.contains("stockpile")
            || text.contains("\"schema\": \"hoi4skill.technology_equipment_plan.v1\"")
            || text.contains("\"equipment\"")
    });

    if map.flags.contains("require-air-oob") && !has_air_oob {
        blockers.push("required air OOB report is missing".to_string());
    }
    if map.flags.contains("require-naval-oob") && !has_naval_oob {
        blockers.push("required naval OOB report is missing".to_string());
    }
    if map.flags.contains("require-bookmark") && !has_bookmark {
        blockers.push("required bookmark/common plan report is missing".to_string());
    } else if !has_bookmark {
        warnings.push("bookmark plan not supplied; scenario is start-date ready but bookmark UI is not closed".to_string());
    }
    if map.flags.contains("require-stockpile") && !has_stockpile {
        blockers.push("required stockpile/equipment report is missing".to_string());
    }

    let ok = blockers.is_empty();
    let json = render_startdate_closure_plan_json(
        ok,
        &input,
        &reports,
        &plan,
        has_air_oob,
        has_naval_oob,
        has_bookmark,
        has_stockpile,
        &warnings,
        &blockers,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_history_transaction_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| {
            target_root
                .join(".hoi4skill")
                .join("history_transaction_apply")
        });
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("history-transaction-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("history-transaction-apply requires --final-check".to_string());
    }
    if !plan.contains("\"schema\": \"hoi4skill.history_transaction_plan.v1\"") {
        blockers.push("input is not a history-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input transaction plan is not ok".to_string());
    }
    let tag = json_string_field(&plan, "tag").unwrap_or_default();
    let enemy_tag = json_string_field(&plan, "enemy_tag");
    let tech_year = history_scenario_json_i64_field(&plan, "technology_year").unwrap_or(1936);
    let state_id = history_scenario_json_i64_field(&plan, "id");
    let technologies = json_string_array_field(&plan, "technologies");
    let planned_effects = json_string_array_field(&plan, "planned_effects");
    let planned_files = json_string_array_field(&plan, "planned_files");
    let leader_id = first_history_scenario_leader_id(&plan);
    let oob_id = first_history_scenario_oob_id(&plan);
    if tag.is_empty() || state_id.is_none() || technologies.is_empty() || oob_id.is_none() {
        blockers
            .push("transaction plan is missing tag/state/technologies/OOB evidence".to_string());
    }
    if !planned_files
        .iter()
        .any(|file| file.contains("history/countries"))
    {
        blockers.push("transaction plan is missing country history file component".to_string());
    }
    let write_plan = if blockers.is_empty() {
        history_scenario_apply_write_plan(HistoryScenarioApplyInput {
            prefix: "history_transaction",
            target_root: &target_root,
            output_dir: &output_dir,
            input_plan: &input,
            tag: &tag,
            enemy_tag: enemy_tag.as_deref(),
            state_id,
            tech_year,
            technologies: &technologies,
            leader_id: leader_id.as_deref(),
            oob_id: oob_id.as_deref(),
            planned_files: &planned_files,
            planned_effects: &planned_effects,
        })
    } else {
        Vec::new()
    };
    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        for (_, path, _) in &write_plan {
            if path.exists() {
                blockers.push(format!(
                    "transaction target already exists and will not be overwritten: {}",
                    path.display()
                ));
            }
        }
    }
    if blockers.is_empty() {
        match write_history_scenario_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_history_scenario_files(&changed));
                blockers.push("history transaction failed and rollback was attempted".to_string());
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }
    let ok = blockers.is_empty();
    let report = history_transaction_apply_json(
        &input,
        &target_root,
        &output_dir,
        ok,
        &changed_files,
        &blockers,
        &rollback_blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !ok) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct HistoryScenarioPlanView<'a> {
    ok: bool,
    game_root: &'a Path,
    mod_root: Option<&'a Path>,
    text: &'a str,
    tag: &'a str,
    enemy_tag: Option<&'a str>,
    leader_name: Option<&'a str>,
    leader_character: Option<&'a str>,
    tech_year: i64,
    state_query: Option<&'a str>,
    state_match: Option<&'a ScenarioStateMatch>,
    technologies: &'a [String],
    leader_matches: &'a [ScenarioLeaderMatch],
    oob: Option<&'a ScenarioOobMatch>,
    war_template: Option<&'a str>,
    planned_effects: &'a [String],
    planned_files: &'a [String],
    blockers: &'a [String],
    warnings: &'a [String],
    questions: &'a [String],
}

fn render_history_scenario_plan_json(view: HistoryScenarioPlanView<'_>) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"request_text\": {},\n  \"targets\": {{\"tag\": {}, \"enemy_tag\": {}, \"leader_name\": {}, \"leader_character\": {}, \"state_query\": {}, \"technology_year\": {}}},\n  \"state\": {},\n  \"leader_matches\": {},\n  \"oob\": {},\n  \"war_template\": {},\n  \"technologies\": {},\n  \"planned_effects\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"warnings\": {},\n  \"questions\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.history_scenario_plan.v1"),
        json_bool(view.ok),
        json_str(if view.ok {
            "history_scenario_ready"
        } else {
            "blocked"
        }),
        json_str(&view.game_root.display().to_string()),
        json_optional_str(view.mod_root.map(|root| root.display().to_string()).as_deref()),
        json_str(view.text),
        json_str(view.tag),
        json_optional_str(view.enemy_tag),
        json_optional_str(view.leader_name),
        json_optional_str(view.leader_character),
        json_optional_str(view.state_query),
        view.tech_year,
        history_scenario_state_json(view.state_match),
        history_scenario_leaders_json(view.leader_matches),
        history_scenario_oob_json(view.oob),
        json_optional_str(view.war_template),
        json_array(view.technologies),
        json_array(view.planned_effects),
        json_array(view.planned_files),
        json_array(view.blockers),
        json_array(view.warnings),
        json_array(view.questions),
        json_array(&[
            "country history owns leader recruitment, politics, technology, and load_oob references".to_string(),
            "history/states owns owner/controller/core/building/resource start-date state data".to_string(),
            "history/units owns deployment; deployment location must be a province inside the verified state".to_string(),
            "history/diplomacy start wars require observed war-block template evidence".to_string(),
            "AI must choose from indexed tags, characters, technologies, states, OOB IDs, and templates; unknown symbols are hard blockers".to_string(),
        ])
    )
}

struct HistoryTransactionPlanView<'a> {
    ok: bool,
    game_root: &'a Path,
    mod_root: Option<&'a Path>,
    text: &'a str,
    tag: &'a str,
    enemy_tag: Option<&'a str>,
    tech_year: i64,
    state_match: Option<&'a ScenarioStateMatch>,
    technologies: &'a [String],
    leader_matches: &'a [ScenarioLeaderMatch],
    oob: Option<&'a ScenarioOobMatch>,
    war_template: Option<&'a str>,
    planned_effects: &'a [String],
    planned_files: &'a [String],
    blockers: &'a [String],
}

fn render_history_transaction_plan_json(view: HistoryTransactionPlanView<'_>) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"request_text\": {},\n  \"tag\": {},\n  \"enemy_tag\": {},\n  \"technology_year\": {},\n  \"state\": {},\n  \"leader_matches\": {},\n  \"oob\": {},\n  \"war_template\": {},\n  \"technologies\": {},\n  \"components\": {},\n  \"planned_effects\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.history_transaction_plan.v1"),
        json_bool(view.ok),
        json_str(if view.ok { "history_transaction_ready" } else { "blocked" }),
        json_str(&view.game_root.display().to_string()),
        json_optional_str(view.mod_root.map(|root| root.display().to_string()).as_deref()),
        json_str(view.text),
        json_str(view.tag),
        json_optional_str(view.enemy_tag),
        view.tech_year,
        history_scenario_state_json(view.state_match),
        history_scenario_leaders_json(view.leader_matches),
        history_scenario_oob_json(view.oob),
        json_optional_str(view.war_template),
        json_array(view.technologies),
        history_transaction_components_json(&view),
        json_array(view.planned_effects),
        json_array(view.planned_files),
        json_array(view.blockers),
        json_array(&[
            "history/countries owns leader, politics, technologies, ideas, stockpile, and OOB references".to_string(),
            "history/states owns owner/controller/core/buildings/resources/victory_points/population".to_string(),
            "history/units owns deployment and OOB structure".to_string(),
            "history/diplomacy owns start wars and must use observed templates".to_string(),
            "all components must be ready or the transaction must not write files".to_string(),
        ])
    )
}

fn history_transaction_components_json(view: &HistoryTransactionPlanView<'_>) -> String {
    let country_ready = !view.tag.is_empty()
        && !view.technologies.is_empty()
        && !view.leader_matches.is_empty()
        && view.oob.is_some();
    let state_ready = view.state_match.is_some();
    let oob_ready = view
        .state_match
        .is_some_and(|state| !state.province_sample.is_empty())
        && view.oob.is_some();
    let diplomacy_ready = if view.enemy_tag.is_some() {
        view.war_template.is_some()
    } else {
        true
    };
    format!(
        "[{}, {}, {}, {}]",
        history_transaction_component_json(
            "country_history",
            country_ready,
            "history/countries",
            "leader, technology, stockpile, politics, and OOB reference",
        ),
        history_transaction_component_json(
            "state_history",
            state_ready,
            "history/states",
            "owner, controller, core, buildings, resources, population, victory points",
        ),
        history_transaction_component_json(
            "oob",
            oob_ready,
            "history/units",
            "deployment inside verified state province sample",
        ),
        history_transaction_component_json(
            "diplomacy",
            diplomacy_ready,
            "history/diplomacy",
            "start-war template evidence",
        )
    )
}

fn history_transaction_component_json(
    component: &str,
    ready: bool,
    path_family: &str,
    owns: &str,
) -> String {
    format!(
        "{{\"component\": {}, \"status\": {}, \"path_family\": {}, \"owns\": {}}}",
        json_str(component),
        json_str(if ready { "ready" } else { "blocked" }),
        json_str(path_family),
        json_str(owns)
    )
}

struct HistoryScenarioApplyInput<'a> {
    prefix: &'a str,
    target_root: &'a Path,
    output_dir: &'a Path,
    input_plan: &'a Path,
    tag: &'a str,
    enemy_tag: Option<&'a str>,
    state_id: Option<i64>,
    tech_year: i64,
    technologies: &'a [String],
    leader_id: Option<&'a str>,
    oob_id: Option<&'a str>,
    planned_files: &'a [String],
    planned_effects: &'a [String],
}

fn history_scenario_apply_write_plan(
    input: HistoryScenarioApplyInput<'_>,
) -> Vec<(String, PathBuf, String)> {
    let state_id = input.state_id.unwrap_or_default();
    let leader_id = input.leader_id.unwrap_or("<verified_leader>");
    let oob_id = input.oob_id.unwrap_or("<verified_oob>");
    vec![
        (
            ".hoi4skill/history_scenario_apply/README.md".to_string(),
            input.output_dir.join("README.md"),
            history_scenario_apply_readme(&input),
        ),
        (
            ".hoi4skill/history_scenario_apply/country_history_patch.txt".to_string(),
            input.output_dir.join("country_history_patch.txt"),
            history_scenario_country_patch(
                input.tag,
                leader_id,
                input.tech_year,
                input.technologies,
                oob_id,
            ),
        ),
        (
            ".hoi4skill/history_scenario_apply/state_history_patch.txt".to_string(),
            input.output_dir.join("state_history_patch.txt"),
            history_scenario_state_patch(input.tag, state_id),
        ),
        (
            ".hoi4skill/history_scenario_apply/oob_patch.txt".to_string(),
            input.output_dir.join("oob_patch.txt"),
            history_scenario_oob_patch(input.tag, oob_id, state_id),
        ),
        (
            ".hoi4skill/history_scenario_apply/diplomacy_patch.txt".to_string(),
            input.output_dir.join("diplomacy_patch.txt"),
            history_scenario_diplomacy_patch(input.tag, input.enemy_tag),
        ),
    ]
}

fn history_scenario_apply_readme(input: &HistoryScenarioApplyInput<'_>) -> String {
    let mut out = String::new();
    out.push_str("# HOI4Skill History Scenario Apply Pack\n\n");
    out.push_str("This pack is generated from a verified `history-scenario-plan` report. It is intentionally a patch pack, not a direct overwrite of inherited history files.\n\n");
    out.push_str(&format!("- input_plan: `{}`\n", input.input_plan.display()));
    out.push_str(&format!(
        "- target_mod: `{}`\n",
        input.target_root.display()
    ));
    out.push_str(&format!("- prefix: `{}`\n", input.prefix));
    out.push_str(&format!("- tag: `{}`\n", input.tag));
    if let Some(enemy) = input.enemy_tag {
        out.push_str(&format!("- enemy_tag: `{enemy}`\n"));
    }
    if let Some(state_id) = input.state_id {
        out.push_str(&format!("- state_id: `{state_id}`\n"));
    }
    out.push_str(&format!("- technology_year: `{}`\n", input.tech_year));
    out.push_str("\n## Planned Effects\n\n");
    for effect in input.planned_effects {
        out.push_str(&format!("- {effect}\n"));
    }
    out.push_str("\n## Target Files From Plan\n\n");
    for file in input.planned_files {
        out.push_str(&format!("- `{file}`\n"));
    }
    out.push_str("\n## Writer Rule\n\n");
    out.push_str("Copy these snippets only through a changed-file writer after verifying the exact parent or local file. Do not paste them blindly into unrelated history files.\n");
    out
}

fn history_scenario_country_patch(
    tag: &str,
    leader_id: &str,
    tech_year: i64,
    technologies: &[String],
    oob_id: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Country history patch for {tag}\n"));
    out.push_str("# Merge into the verified history/countries file for the target tag.\n\n");
    out.push_str(&format!("recruit_character = {leader_id}\n"));
    out.push_str(&format!("oob = \"{oob_id}\"\n"));
    out.push_str(&format!(
        "# {tech_year} technologies verified by history-scenario-plan\n"
    ));
    out.push_str("set_technology = {\n");
    for tech in technologies {
        out.push_str(&format!("\t{tech} = 1\n"));
    }
    out.push_str("}\n");
    out
}

fn history_scenario_state_patch(tag: &str, state_id: i64) -> String {
    format!(
        "# State history patch for state {state_id}\n# Merge into the verified state = {{ id = {state_id} ... history = {{ ... }} }} block.\nhistory = {{\n\towner = {tag}\n\tcontroller = {tag}\n\tadd_core_of = {tag}\n}}\n"
    )
}

fn history_scenario_oob_patch(tag: &str, oob_id: &str, state_id: i64) -> String {
    format!(
        "# OOB patch for {tag} using `{oob_id}`\n# Move initial divisions to a province inside verified state {state_id}; use the exact province from the plan's state.province_sample.\n# Do not invent division template syntax; preserve existing division_template and equipment blocks.\n"
    )
}

fn history_scenario_diplomacy_patch(tag: &str, enemy_tag: Option<&str>) -> String {
    match enemy_tag {
        Some(enemy) => format!(
            "# Diplomacy history patch\n# Create or edit a history/diplomacy file by copying the observed parent war template shape.\n# Required participants: attacker/initiator side `{tag}`, defender/opponent side `{enemy}`.\n# Do not invent a new war block shape if the plan did not observe one.\n"
        ),
        None => "# No start-war request was verified in this scenario.\n".to_string(),
    }
}

fn write_history_scenario_transaction(
    write_plan: &[(String, PathBuf, String)],
) -> Result<Vec<String>, (String, Vec<PathBuf>)> {
    let mut changed = Vec::new();
    for (_, path, content) in write_plan {
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                return Err((format!("create {}: {err}", parent.display()), changed));
            }
        }
        if let Err(err) = fs::write(path, content) {
            return Err((format!("write {}: {err}", path.display()), changed));
        }
        changed.push(path.clone());
    }
    Ok(changed
        .into_iter()
        .map(|path| path.display().to_string())
        .collect())
}

fn rollback_history_scenario_files(changed: &[PathBuf]) -> Vec<String> {
    let mut blockers = Vec::new();
    for path in changed.iter().rev() {
        if let Err(err) = fs::remove_file(path) {
            blockers.push(format!("rollback failed for {}: {err}", path.display()));
        }
    }
    blockers
}

fn history_scenario_apply_json(
    input: &Path,
    target_root: &Path,
    output_dir: &Path,
    ok: bool,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"target_root\": {},\n  \"output_dir\": {},\n  \"changed_files\": {},\n  \"blockers\": {},\n  \"rollback_blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.history_scenario_apply.v1"),
        json_bool(ok),
        json_str(if ok { "history_scenario_patch_pack_written" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_str(&target_root.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_array(changed_files),
        json_array(blockers),
        json_array(rollback_blockers),
        json_str("apply writes a reviewable patch pack only; exact history files still require changed-file merge plus final validation")
    )
}

fn history_transaction_apply_json(
    input: &Path,
    target_root: &Path,
    output_dir: &Path,
    ok: bool,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"target_root\": {},\n  \"output_dir\": {},\n  \"changed_files\": {},\n  \"blockers\": {},\n  \"rollback_blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.history_transaction_apply.v1"),
        json_bool(ok),
        json_str(if ok { "history_transaction_patch_pack_written" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_str(&target_root.display().to_string()),
        json_str(&output_dir.display().to_string()),
        json_array(changed_files),
        json_array(blockers),
        json_array(rollback_blockers),
        json_str("history transaction apply writes a reviewable multi-file patch pack only; exact changed-file merge still requires final validation")
    )
}

fn render_startdate_closure_plan_json(
    ok: bool,
    input: &Path,
    reports: &[PathBuf],
    plan: &str,
    has_air_oob: bool,
    has_naval_oob: bool,
    has_bookmark: bool,
    has_stockpile: bool,
    warnings: &[String],
    blockers: &[String],
) -> String {
    let planned_files = json_string_array_field(plan, "planned_files");
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.startdate_closure_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "startdate_closure_ready"
        } else {
            "blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert(
        "reports".to_string(),
        json_array(
            &reports
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "components".to_string(),
        startdate_closure_components_json(
            plan,
            has_air_oob,
            has_naval_oob,
            has_bookmark,
            has_stockpile,
        ),
    );
    map.insert("planned_files".to_string(), json_array(&planned_files));
    map.insert("warnings".to_string(), json_array(warnings));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "next_commands".to_string(),
        json_array(&[
            "hoi4skill history-startdate-gate --input history_transaction.json --taxonomy unit_taxonomy.json --require-passed".to_string(),
            "hoi4skill history-transaction-apply --input history_transaction.json --mod-root <target> --execute --final-check --require-passed".to_string(),
            "hoi4skill validate <mod> --game-root <hoi4> --strict-code-index".to_string(),
        ]),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "startdate closure requires one transaction graph for country history, state history, OOB, technology, and diplomacy".to_string(),
            "bookmark, air OOB, naval OOB, and stockpile reports are optional unless required by flags or user request".to_string(),
            "new tag, character, bookmark, air wing, navy, or stockpile creation requires explicit user authorization".to_string(),
            "OOB deployment must use province/base evidence from local game/parent/target files".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn startdate_report_has_kind(text: &str, kind: &str) -> bool {
    json_string_field(text, "kind").as_deref() == Some(kind)
        || json_string_field(text, "oob_kind").as_deref() == Some(kind)
        || text.contains(&format!("\"kind\":\"{kind}\""))
        || text.contains(&format!("\"oob_kind\":\"{kind}\""))
}

fn startdate_closure_components_json(
    plan: &str,
    has_air_oob: bool,
    has_naval_oob: bool,
    has_bookmark: bool,
    has_stockpile: bool,
) -> String {
    let rows = [
        (
            "country_history",
            plan.contains("\"component\": \"country_history\", \"status\": \"ready\""),
            "leader, politics, technology, OOB reference, stockpile hook",
        ),
        (
            "state_history",
            plan.contains("\"component\": \"state_history\", \"status\": \"ready\""),
            "owner, controller, core, state start-date data",
        ),
        (
            "army_oob",
            plan.contains("\"component\": \"oob\", \"status\": \"ready\""),
            "army deployment inside verified province set",
        ),
        ("air_oob", has_air_oob, "air wings and air base evidence"),
        ("naval_oob", has_naval_oob, "navies and naval base evidence"),
        (
            "technology",
            !json_string_array_field(plan, "technologies").is_empty(),
            "indexed start-year technologies",
        ),
        (
            "stockpile",
            has_stockpile,
            "equipment and stockpile evidence",
        ),
        (
            "diplomacy",
            json_string_field(plan, "enemy_tag").is_none()
                || plan.contains("\"component\": \"diplomacy\", \"status\": \"ready\""),
            "war/faction/truce/diplomacy history evidence",
        ),
        (
            "bookmark",
            has_bookmark,
            "common/bookmarks start-date UI evidence",
        ),
    ];
    format!(
        "[{}]",
        rows.iter()
            .map(|(component, ready, owns)| {
                let mut map = BTreeMap::new();
                map.insert("component".to_string(), json_str(component));
                map.insert(
                    "status".to_string(),
                    json_str(if *ready { "ready" } else { "not_supplied" }),
                );
                map.insert("owns".to_string(), json_str(owns));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn history_scenario_json_i64_field(text: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{key}\":");
    let idx = text.find(&pattern)?;
    let rest = text[idx + pattern.len()..].trim_start();
    let mut value = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() || (value.is_empty() && ch == '-') {
            value.push(ch);
        } else if !value.is_empty() {
            break;
        } else {
            return None;
        }
    }
    value.parse::<i64>().ok()
}

fn first_history_scenario_leader_id(plan: &str) -> Option<String> {
    let start = plan.find("\"leader_matches\"")?;
    let end = plan[start..]
        .find("\"oob\"")
        .map(|idx| start + idx)
        .unwrap_or(plan.len());
    json_string_field(&plan[start..end], "id")
}

fn first_history_scenario_oob_id(plan: &str) -> Option<String> {
    let start = plan.find("\"oob\"")?;
    let end = plan[start..]
        .find("\"war_template\"")
        .map(|idx| start + idx)
        .unwrap_or(plan.len());
    json_string_field(&plan[start..end], "id")
}

fn infer_history_scenario_tag(text: &str, index: &GameIndex) -> Option<String> {
    let _ = index;
    first_tag(text)
}

fn infer_history_scenario_enemy_tag(
    text: &str,
    index: &GameIndex,
    target_tag: &str,
) -> Option<String> {
    let _ = index;
    if let Some(tag) = first_tag(text) {
        (tag != target_tag).then_some(tag)
    } else {
        None
    }
}

fn history_scenario_requests_leader(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("领导人")
        || text.contains("在台上")
        || text.contains("上台")
        || lower.contains("leader")
}

fn infer_history_scenario_year(text: &str) -> Option<i64> {
    for token in token_candidates(text) {
        if let Ok(year) = token.parse::<i64>() {
            if (1900..=1950).contains(&year) {
                return Some(year);
            }
        }
    }
    None
}

fn infer_history_scenario_state_query(text: &str, index: &GameIndex) -> Option<String> {
    if let Some(state_key) = state_name_hint(text) {
        return Some(state_key);
    }
    let normalized_text = normalize_scenario_name(text);
    for (key, id) in &index.state_names {
        if normalized_text.contains(&normalize_scenario_name(key)) {
            return Some(key.clone());
        }
        if let Some(localized) = index.localisation_entries.get(key) {
            let normalized = normalize_scenario_name(localized);
            if !normalized.is_empty()
                && (normalized_text.contains(&normalized) || normalized.contains(&normalized_text))
            {
                return Some(key.clone());
            }
        }
        if normalized_text.contains(&id.to_string()) {
            return Some(key.clone());
        }
    }
    None
}

fn resolve_history_scenario_state(
    mod_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: &GameIndex,
    requested_id: Option<i64>,
    query: Option<&str>,
) -> Result<Option<ScenarioStateMatch>, String> {
    let mut local_states = if let Some(root) = mod_root {
        scan_history_state_styles(root)?
    } else {
        Vec::new()
    };
    let dependency_states = scan_dependency_history_states(dependency_roots)?;
    let mut id = requested_id;
    let mut name_key = None;
    let mut localized_name = None;
    if id.is_none() {
        if let Some(query) = query {
            if let Some((key, matched_id, localized)) = resolve_state_query_from_index(index, query)
            {
                id = Some(matched_id);
                name_key = Some(key);
                localized_name = localized;
            }
        }
    }
    let id = match id {
        Some(id) => id,
        None => return Ok(None),
    };
    if let Some(state) = find_state_by_id(&local_states, id) {
        return Ok(Some(ScenarioStateMatch {
            id,
            name_key: state.name.clone().or(name_key),
            localized_name: localized_name.or_else(|| {
                state
                    .name
                    .as_ref()
                    .and_then(|key| index.localisation_entries.get(key).cloned())
            }),
            file: Some(state.file.clone()),
            source: "local_mod".to_string(),
            owner: state.owner.clone(),
            controller: state.controller.clone(),
            province_sample: state.province_sample.clone(),
        }));
    }
    for dependency in dependency_states {
        if dependency.state.id == Some(id) {
            return Ok(Some(ScenarioStateMatch {
                id,
                name_key: dependency.state.name.clone().or(name_key),
                localized_name: localized_name.or_else(|| {
                    dependency
                        .state
                        .name
                        .as_ref()
                        .and_then(|key| index.localisation_entries.get(key).cloned())
                }),
                file: Some(dependency.state.file.clone()),
                source: format!("dependency:{}", dependency.root),
                owner: dependency.state.owner.clone(),
                controller: dependency.state.controller.clone(),
                province_sample: dependency.state.province_sample.clone(),
            }));
        }
    }
    if index.state_ids.contains(&id) {
        return Ok(Some(ScenarioStateMatch {
            id,
            name_key,
            localized_name,
            file: None,
            source: "game_index".to_string(),
            owner: None,
            controller: None,
            province_sample: Vec::new(),
        }));
    }
    local_states.clear();
    Ok(None)
}

fn resolve_state_query_from_index(
    index: &GameIndex,
    query: &str,
) -> Option<(String, i64, Option<String>)> {
    let normalized_query = normalize_scenario_name(query);
    for (key, id) in &index.state_names {
        let normalized_key = normalize_scenario_name(key);
        let localized = index.localisation_entries.get(key).cloned();
        let normalized_localized = localized
            .as_deref()
            .map(normalize_scenario_name)
            .unwrap_or_default();
        if normalized_query == normalized_key
            || normalized_query == id.to_string()
            || (!normalized_localized.is_empty()
                && (normalized_query == normalized_localized
                    || normalized_query.contains(&normalized_localized)
                    || normalized_localized.contains(&normalized_query)))
        {
            return Some((key.clone(), *id, localized));
        }
    }
    None
}

fn collect_history_scenario_technologies(
    roots: &[PathBuf],
    year: i64,
    index: &GameIndex,
) -> Result<Vec<String>, String> {
    let mut out = BTreeSet::new();
    for root in roots {
        let tech_root = root.join("common").join("technologies");
        if !tech_root.exists() {
            continue;
        }
        for file in collect_files(&tech_root)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
                continue;
            }
            let text = strip_comments(&read_utf8_lossy(&file)?);
            let wrappers = direct_blocks_named(&text, "technologies");
            let roots = if wrappers.is_empty() {
                vec![text]
            } else {
                wrappers
            };
            for wrapper in roots {
                for (id, block) in direct_child_blocks(&wrapper) {
                    if !index.technologies.contains(&id) {
                        continue;
                    }
                    let matches_year = ["start_year", "year"].iter().any(|key| {
                        block_assignment(&block, key).as_deref() == Some(&year.to_string())
                    });
                    if matches_year {
                        out.insert(id);
                    }
                }
            }
        }
    }
    Ok(out.into_iter().collect())
}

fn resolve_history_scenario_leader(
    roots: &[PathBuf],
    index: &GameIndex,
    leader_name: Option<&str>,
    leader_character: Option<&str>,
) -> Result<Vec<ScenarioLeaderMatch>, String> {
    let mut matches = Vec::new();
    let Some(query) = leader_character.or(leader_name) else {
        return Ok(matches);
    };
    let normalized_query = normalize_scenario_name(query);
    for root in roots {
        for character in scan_character_styles(root, usize::MAX)? {
            if !character.roles.iter().any(|role| role == "country_leader") {
                continue;
            }
            let localized = index.localisation_entries.get(&character.id);
            let id_match = normalize_scenario_name(&character.id).contains(&normalized_query)
                || normalized_query.contains(&normalize_scenario_name(&character.id));
            let loc_match = localized
                .map(|value| {
                    let normalized = normalize_scenario_name(value);
                    !normalized.is_empty()
                        && (normalized == normalized_query
                            || normalized.contains(&normalized_query)
                            || normalized_query.contains(&normalized))
                })
                .unwrap_or(false);
            if Some(character.id.as_str()) == leader_character || id_match || loc_match {
                matches.push(ScenarioLeaderMatch {
                    id: character.id,
                    file: character.file,
                    style: "common_character".to_string(),
                    roles: character.roles,
                    match_reason: if loc_match {
                        "localised_name".to_string()
                    } else {
                        "id_or_explicit".to_string()
                    },
                });
            }
        }
        for leader in scan_legacy_country_leaders(root, usize::MAX)? {
            let name = leader.name.clone().unwrap_or_default();
            let localized = index.localisation_entries.get(&name);
            let name_match = normalize_scenario_name(&name).contains(&normalized_query);
            let loc_match = localized
                .map(|value| {
                    let normalized = normalize_scenario_name(value);
                    !normalized.is_empty()
                        && (normalized == normalized_query
                            || normalized.contains(&normalized_query)
                            || normalized_query.contains(&normalized))
                })
                .unwrap_or(false);
            if name_match || loc_match {
                matches.push(ScenarioLeaderMatch {
                    id: name,
                    file: leader.file,
                    style: "legacy_create_country_leader".to_string(),
                    roles: vec!["country_leader".to_string()],
                    match_reason: if loc_match {
                        "legacy_localised_name".to_string()
                    } else {
                        "legacy_name".to_string()
                    },
                });
            }
        }
    }
    matches.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    matches.dedup_by(|a, b| a.id == b.id && a.file == b.file && a.style == b.style);
    Ok(matches)
}

fn resolve_history_scenario_oob(
    roots: &[PathBuf],
    tag: &str,
    requested: Option<&str>,
) -> Result<Option<ScenarioOobMatch>, String> {
    if let Some(id) = requested {
        return Ok(Some(ScenarioOobMatch {
            id: id.to_string(),
            file: find_oob_file(roots, id)?,
            source: "explicit".to_string(),
        }));
    }
    if tag.is_empty() {
        return Ok(None);
    }
    for root in roots {
        for file in txt_files(root, "history/countries")? {
            if history_country_tag_from_path(&file) != tag {
                continue;
            }
            let text = strip_comments(&read_utf8_lossy(&file)?);
            let oob_id = block_assignment(&text, "oob")
                .or_else(|| block_assignment(&text, "load_oob"))
                .or_else(|| assignment_values_in_text(&text, "oob").into_iter().next())
                .or_else(|| {
                    assignment_values_in_text(&text, "load_oob")
                        .into_iter()
                        .next()
                });
            if let Some(id) = oob_id {
                return Ok(Some(ScenarioOobMatch {
                    file: find_oob_file(roots, &id)?,
                    id,
                    source: format!("country_history:{}", rel_slash(root, &file)),
                }));
            }
        }
    }
    Ok(None)
}

fn find_oob_file(roots: &[PathBuf], id: &str) -> Result<Option<String>, String> {
    let expected = format!("{id}.txt");
    for root in roots {
        let units = root.join("history").join("units");
        if !units.exists() {
            continue;
        }
        for file in collect_files(&units)? {
            if file
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| name.eq_ignore_ascii_case(&expected))
            {
                return Ok(Some(rel_slash(root, &file)));
            }
        }
    }
    Ok(None)
}

fn find_history_scenario_war_template(roots: &[PathBuf]) -> Result<Option<String>, String> {
    for root in roots {
        let diplomacy = root.join("history").join("diplomacy");
        if !diplomacy.exists() {
            continue;
        }
        for file in collect_files(&diplomacy)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
                continue;
            }
            let text = strip_comments(&read_utf8_lossy(&file)?);
            if !blocks_named(&text, "war").is_empty()
                || text.contains("attacker =")
                || text.contains("defender =")
            {
                return Ok(Some(rel_slash(root, &file)));
            }
        }
    }
    Ok(None)
}

fn history_scenario_requests_war(text: &str) -> bool {
    text.contains("战争")
        || text.contains("打仗")
        || text.contains("开战")
        || text.contains("处于战争")
        || text.to_ascii_lowercase().contains("war")
}

fn history_scenario_planned_effects(
    tag: &str,
    enemy_tag: Option<&str>,
    leader: Option<&ScenarioLeaderMatch>,
    state: Option<&ScenarioStateMatch>,
    tech_year: i64,
    technologies: &[String],
    oob: Option<&ScenarioOobMatch>,
) -> Vec<String> {
    let mut out = Vec::new();
    let tag_label = if tag.is_empty() { "<TAG>" } else { tag };
    if let Some(leader) = leader {
        out.push(format!(
            "{leader_id} becomes active leader for {tag_label}",
            leader_id = leader.id
        ));
    }
    if let Some(state) = state {
        out.push(format!(
            "history/states state {} owner/controller become {tag_label}",
            state.id
        ));
        if let Some(province) = state.province_sample.first() {
            out.push(format!(
                "history/units deploys the selected OOB in state {} via province {}",
                state.id, province
            ));
        }
    }
    out.push(format!(
        "{tag_label} receives all indexed {tech_year} technologies ({})",
        technologies.len()
    ));
    if let Some(enemy) = enemy_tag {
        out.push(format!(
            "history/diplomacy starts a war between {tag_label} and {enemy}"
        ));
    }
    if let Some(oob) = oob {
        out.push(format!(
            "history/countries/{tag_label} keeps load_oob/oob id `{}`",
            oob.id
        ));
    }
    out.push(format!(
        "{tag_label} requested startup conditions must be represented only through verified local history, character, state, diplomacy, technology, and OOB evidence"
    ));
    out
}

fn history_scenario_planned_files(
    tag: &str,
    enemy_tag: Option<&str>,
    state: Option<&ScenarioStateMatch>,
    oob: Option<&ScenarioOobMatch>,
) -> Vec<String> {
    let mut out = if tag.is_empty() {
        vec!["history/countries/<TAG>.txt changed-only patch after --tag is provided".to_string()]
    } else {
        vec![format!("history/countries/{tag}.txt changed-only patch")]
    };
    if let Some(state) = state {
        out.push(
            state.file.clone().unwrap_or_else(|| {
                format!("history/states/<state_{}_source_override>.txt", state.id)
            }),
        );
    }
    if let Some(oob) = oob {
        out.push(
            oob.file
                .clone()
                .unwrap_or_else(|| format!("history/units/{}.txt", oob.id)),
        );
    }
    if enemy_tag.is_some() {
        out.push("history/diplomacy/<observed-war-template-derived-file>.txt".to_string());
    }
    out
}

fn history_scenario_state_json(state: Option<&ScenarioStateMatch>) -> String {
    let Some(state) = state else {
        return "null".to_string();
    };
    format!(
        "{{\"id\": {}, \"name_key\": {}, \"localized_name\": {}, \"file\": {}, \"source\": {}, \"owner\": {}, \"controller\": {}, \"province_sample\": {}}}",
        state.id,
        json_optional_str(state.name_key.as_deref()),
        json_optional_str(state.localized_name.as_deref()),
        json_optional_str(state.file.as_deref()),
        json_str(&state.source),
        json_optional_str(state.owner.as_deref()),
        json_optional_str(state.controller.as_deref()),
        json_i64_array(&state.province_sample)
    )
}

fn history_scenario_leaders_json(leaders: &[ScenarioLeaderMatch]) -> String {
    format!(
        "[{}]",
        leaders
            .iter()
            .map(|leader| {
                format!(
                    "{{\"id\": {}, \"file\": {}, \"style\": {}, \"roles\": {}, \"match_reason\": {}}}",
                    json_str(&leader.id),
                    json_str(&leader.file),
                    json_str(&leader.style),
                    json_array(&leader.roles),
                    json_str(&leader.match_reason)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn history_scenario_oob_json(oob: Option<&ScenarioOobMatch>) -> String {
    let Some(oob) = oob else {
        return "null".to_string();
    };
    format!(
        "{{\"id\": {}, \"file\": {}, \"source\": {}}}",
        json_str(&oob.id),
        json_optional_str(oob.file.as_deref()),
        json_str(&oob.source)
    )
}

fn normalize_scenario_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .flat_map(char::to_lowercase)
        .collect()
}
