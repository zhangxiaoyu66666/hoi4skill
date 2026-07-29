//! Non-visual map-data planning gates.
//!
//! These commands classify map edits and gather evidence. They do not write
//! topology, bitmap, adjacency, state, railway, or supply files.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_map_data_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let mut roots = Vec::new();
    if let Some(game_root) = game_root {
        roots.push(MapDataRoot {
            layer: "game".to_string(),
            path: game_root,
        });
    }
    for root in dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)? {
        roots.push(MapDataRoot {
            layer: "parent".to_string(),
            path: root,
        });
    }
    roots.push(MapDataRoot {
        layer: "target".to_string(),
        path: mod_root,
    });
    let report = map_data_audit_json(&roots)?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed")
        && (report.contains("\"status\": \"blocked\"") || report.contains("\"blocking_count\": 1"))
    {
        return Err("map-data-audit has blocking items".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = history_plan_input_text(&map)?;
    if text.trim().is_empty() {
        return Err("map-intent-plan requires --text or --input".to_string());
    }
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), false)?;
    let game_index = game_root
        .as_ref()
        .map(|root| build_game_index_with_mod_paths(root, &dependency_roots))
        .transpose()?;
    let report = map_intent_plan_json(
        &text,
        mod_root.as_deref(),
        game_root.as_deref(),
        &dependency_roots,
        game_index.as_ref(),
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("map-intent-plan has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_province_query(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = value(&map, "text").unwrap_or("").trim().to_string();
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    if mod_root.is_none() && game_root.is_none() {
        return Err("province-query requires --mod-root or --game-root".to_string());
    }
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), false)?;
    let state_ids = repeated_values(&map, "state-id")
        .into_iter()
        .chain(repeated_values(&map, "state"))
        .filter_map(parse_int)
        .collect::<Vec<_>>();
    let explicit_provinces = repeated_values(&map, "province-id")
        .into_iter()
        .chain(repeated_values(&map, "province"))
        .filter_map(parse_int)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let building_filters = repeated_values(&map, "with-building")
        .into_iter()
        .chain(repeated_values(&map, "building"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let vp_only = map.flags.contains("with-victory-point")
        || map.flags.contains("vp-only")
        || map.flags.contains("victory-point-only");
    let report = province_query_json(
        mod_root.as_deref(),
        game_root.as_deref(),
        &dependency_roots,
        &state_ids,
        &explicit_provinces,
        &building_filters,
        vp_only,
        &text,
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("province-query has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_state_transaction_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let text = history_plan_input_text(&map)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let game_index = game_root
        .as_ref()
        .map(|root| build_game_index_with_mod_paths(root, &dependency_roots))
        .transpose()?;
    let states = repeated_values(&map, "state")
        .into_iter()
        .filter_map(parse_int)
        .collect::<Vec<_>>();
    if states.is_empty() && text.trim().is_empty() {
        return Err("state-transaction-plan requires --state <id>".to_string());
    }
    let operations = if states.is_empty() {
        let Some(index) = game_index.as_ref() else {
            return Err("state-transaction-plan --text requires --game-root for indexed state/resource localisation".to_string());
        };
        state_transaction_operations_from_text(
            &text,
            &mod_root,
            game_root.as_deref(),
            &dependency_roots,
            index,
        )?
    } else {
        state_transaction_operations(
            &map,
            &mod_root,
            game_root.as_deref(),
            &dependency_roots,
            game_index.as_ref(),
            &states,
        )?
    };
    let blockers = state_transaction_blockers(&operations);
    let ok = blockers.is_empty();
    let report = state_transaction_plan_json(
        &mod_root,
        game_root.as_deref(),
        &dependency_roots,
        game_index.is_some(),
        &operations,
        &blockers,
        ok,
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_state_transaction_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.state_transaction_plan.v1\"") {
        blockers.push("input is not a state-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input plan is not ok".to_string());
    }
    if !map.flags.contains("execute") {
        blockers.push("state-transaction-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("state-transaction-apply requires --final-check".to_string());
    }
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    if map.flags.contains("write-overrides") && mod_root.is_none() {
        blockers.push("state-transaction-apply --write-overrides requires --mod-root".to_string());
    }
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("state_transaction_apply"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let changed_files = state_transaction_plan_changed_files(&plan);
    let changed_path = output_dir.join("changed_files.txt");
    fs::write(&changed_path, changed_files.join("\n"))
        .map_err(|e| format!("write {}: {e}", changed_path.display()))?;
    let rollback_path = output_dir.join("rollback_plan.md");
    fs::write(
        &rollback_path,
        state_transaction_rollback_markdown(&input, &changed_files),
    )
    .map_err(|e| format!("write {}: {e}", rollback_path.display()))?;
    let mut written_files = Vec::new();
    let mut backup_files = Vec::new();
    if map.flags.contains("write-overrides") && blockers.is_empty() {
        let Some(mod_root) = mod_root.as_deref() else {
            unreachable!("mod-root blocker should have fired");
        };
        match state_transaction_write_target_overrides(&plan, mod_root, &output_dir) {
            Ok((written, backups)) => {
                written_files = written;
                backup_files = backups;
            }
            Err(err) => blockers.push(err),
        }
    }
    let ok = blockers.is_empty();
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.state_transaction_apply.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"execute\": {},\n  \"final_check\": {},\n  \"write_overrides\": {},\n  \"output_dir\": {},\n  \"changed_files\": {},\n  \"written_files\": {},\n  \"backup_files\": {},\n  \"changed_files_report\": {},\n  \"rollback_plan\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok && map.flags.contains("write-overrides") {
            "state_transaction_overrides_written"
        } else if ok {
            "state_transaction_review_pack_ready"
        } else {
            "blocked"
        }),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("execute")),
        json_bool(map.flags.contains("final-check")),
        json_bool(map.flags.contains("write-overrides")),
        json_str(&output_dir.display().to_string()),
        json_array(&changed_files),
        json_array(&written_files),
        json_array(&backup_files),
        json_str(&changed_path.display().to_string()),
        json_str(&rollback_path.display().to_string()),
        blockers.len(),
        json_array(&blockers),
        json_str("P43 apply emits a review pack and rollback manifest; final release still requires validate/runtime/map release gates")
    );
    write_or_print(&manifest, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_supply_network_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let game_index = game_root
        .as_ref()
        .map(|root| build_game_index_with_mod_paths(root, &dependency_roots))
        .transpose()?;
    let state_id = value(&map, "state-id")
        .or_else(|| value(&map, "state"))
        .and_then(parse_int);
    let report = supply_network_plan_json(
        &mod_root,
        game_root.as_deref(),
        &dependency_roots,
        game_index.as_ref(),
        state_id,
        &map,
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("supply-network-plan has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_supply_network_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.supply_network_plan.v1\"") {
        blockers.push("input is not a supply-network-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("input supply-network-plan is not ok".to_string());
    }
    if !map.flags.contains("execute") {
        blockers.push("supply-network-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("supply-network-apply requires --final-check".to_string());
    }
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    if map.flags.contains("write-overrides") && mod_root.is_none() {
        blockers.push("supply-network-apply --write-overrides requires --mod-root".to_string());
    }
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("supply_network_apply"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let changed_files = json_string_array_field_simple(&plan, "changed_files");
    let changed_path = output_dir.join("changed_files.txt");
    fs::write(&changed_path, changed_files.join("\n"))
        .map_err(|e| format!("write {}: {e}", changed_path.display()))?;
    let rollback_path = output_dir.join("rollback_plan.md");
    fs::write(
        &rollback_path,
        state_transaction_rollback_markdown(&input, &changed_files),
    )
    .map_err(|e| format!("write {}: {e}", rollback_path.display()))?;
    let mut written_files = Vec::new();
    let mut backup_files = Vec::new();
    if map.flags.contains("write-overrides") && blockers.is_empty() {
        let Some(mod_root) = mod_root.as_deref() else {
            unreachable!("mod-root blocker should have fired");
        };
        match supply_network_write_target_overrides(&plan, mod_root, &output_dir) {
            Ok((written, backups)) => {
                written_files = written;
                backup_files = backups;
            }
            Err(err) => blockers.push(err),
        }
    }
    let ok = blockers.is_empty();
    let manifest = format!(
        "{{\n  \"schema\": \"hoi4skill.supply_network_apply.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"execute\": {},\n  \"final_check\": {},\n  \"write_overrides\": {},\n  \"output_dir\": {},\n  \"changed_files\": {},\n  \"written_files\": {},\n  \"backup_files\": {},\n  \"changed_files_report\": {},\n  \"rollback_plan\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok && map.flags.contains("write-overrides") {
            "supply_network_overrides_written"
        } else if ok {
            "supply_network_review_pack_ready"
        } else {
            "blocked"
        }),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("execute")),
        json_bool(map.flags.contains("final-check")),
        json_bool(map.flags.contains("write-overrides")),
        json_str(&output_dir.display().to_string()),
        json_array(&changed_files),
        json_array(&written_files),
        json_array(&backup_files),
        json_str(&changed_path.display().to_string()),
        json_str(&rollback_path.display().to_string()),
        blockers.len(),
        json_array(&blockers),
        json_str("P45 apply emits a review pack only; final release still requires validate/runtime/map release gates")
    );
    write_or_print(&manifest, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_strategic_region_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let game_index = game_root
        .as_ref()
        .map(|root| build_game_index_with_mod_paths(root, &dependency_roots))
        .transpose()?;
    let report = strategic_region_plan_json(
        &mod_root,
        game_root.as_deref(),
        &dependency_roots,
        game_index.as_ref(),
        &map,
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("strategic-region-plan has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_topology_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let game_index = game_root
        .as_ref()
        .map(|root| build_game_index_with_mod_paths(root, &dependency_roots))
        .transpose()?;
    let report = map_topology_plan_json(
        &mod_root,
        game_root.as_deref(),
        &dependency_roots,
        game_index.as_ref(),
        &map,
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("map-topology-plan has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_topology_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.map_topology_plan.v1\"") {
        blockers.push("input is not a map-topology-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("map-topology-plan is not ok".to_string());
    }
    if !map.flags.contains("manual-confirmed") {
        blockers.push(
            "map-topology-gate requires --manual-confirmed for high-risk topology edits"
                .to_string(),
        );
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": \"hoi4skill.map_topology_gate.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"manual_confirmation\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "map_topology_gate_ready" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("manual-confirmed")),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "high-risk topology plans are release candidates only after manual confirmation".to_string(),
            "bitmap, definition, adjacency, state, strategic-region, railway, supply, OOB, and weather references must be reviewed before release".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_map_override_risk_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let report = map_override_risk_audit_json(
        &mod_root,
        game_root.as_deref(),
        &dependency_roots,
        value(&map, "plan").map(PathBuf::from).as_deref(),
    )?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("map-override-risk-audit has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_runtime_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let report = map_runtime_gate_json(&map)?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("map-runtime-gate has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_release_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let reports = repeated_values(&map, "report")
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    let report = map_release_gate_json(&mod_root, &reports)?;
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && report.contains("\"ok\": false") {
        return Err("map-release-gate has unresolved blockers".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_map_transaction_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let reports = repeated_values(&map, "report")
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if reports.is_empty() {
        return Err("map-transaction-gate requires at least one --report".to_string());
    }
    let mut rows = Vec::new();
    let mut schemas = BTreeSet::new();
    let mut blockers = Vec::new();
    for report in &reports {
        let text = read_utf8_lossy(report)?;
        let schema = json_schema_name(&text).unwrap_or_else(|| "unknown".to_string());
        let ok = text.contains("\"ok\": true");
        if !ok {
            blockers.push(format!("map report `{}` is not ok", report.display()));
        }
        schemas.insert(schema.clone());
        rows.push(format!(
            "{{\"path\": {}, \"schema\": {}, \"ok\": {}}}",
            json_str(&report.display().to_string()),
            json_str(&schema),
            json_bool(ok)
        ));
    }
    for required in [
        "hoi4skill.map_data_audit.v1",
        "hoi4skill.map_intent_plan.v1",
    ] {
        if !schemas.contains(required) {
            blockers.push(format!("map-transaction-gate requires `{required}` report"));
        }
    }
    if schemas.contains("hoi4skill.map_topology_plan.v1")
        && !schemas.contains("hoi4skill.map_topology_gate.v1")
    {
        blockers.push(
            "topology plan evidence requires map-topology-gate manual confirmation".to_string(),
        );
    }
    let has_apply_or_plan = schemas.iter().any(|schema| {
        matches!(
            schema.as_str(),
            "hoi4skill.state_transaction_plan.v1"
                | "hoi4skill.state_transaction_apply.v1"
                | "hoi4skill.supply_network_plan.v1"
                | "hoi4skill.supply_network_apply.v1"
                | "hoi4skill.strategic_region_plan.v1"
                | "hoi4skill.province_query.v1"
                | "hoi4skill.map_topology_plan.v1"
        )
    });
    if !has_apply_or_plan {
        blockers.push("map-transaction-gate requires at least one province/state/supply/strategic/topology report".to_string());
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"report_count\": {},\n  \"reports\": [{}],\n  \"schemas\": {},\n  \"risk_lanes\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.map_transaction_gate.v1"),
        json_bool(ok),
        json_str(if ok { "map_transaction_ready" } else { "blocked" }),
        reports.len(),
        rows.join(", "),
        json_array(&schemas.iter().cloned().collect::<Vec<_>>()),
        json_array(&map_transaction_risk_lanes(&schemas)),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "hoi4skill map-runtime-gate --report <runtime logs> --require-passed".to_string(),
            "hoi4skill map-release-gate --mod-root <mod> --report <reports...> --require-passed".to_string(),
        ]),
        json_array(&[
            "state/province/VP/resource/building edits require indexed low-risk state evidence".to_string(),
            "supply and strategic-region edits require endpoint/province ownership evidence".to_string(),
            "topology reports are never sufficient without map-topology-gate manual confirmation".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct MapDataRoot {
    layer: String,
    path: PathBuf,
}

fn map_transaction_risk_lanes(schemas: &BTreeSet<String>) -> Vec<String> {
    let mut lanes = Vec::new();
    if schemas.contains("hoi4skill.state_transaction_plan.v1")
        || schemas.contains("hoi4skill.state_transaction_apply.v1")
        || schemas.contains("hoi4skill.province_query.v1")
    {
        lanes.push("low:state_history_or_province_set".to_string());
    }
    if schemas.contains("hoi4skill.supply_network_plan.v1")
        || schemas.contains("hoi4skill.supply_network_apply.v1")
        || schemas.contains("hoi4skill.strategic_region_plan.v1")
    {
        lanes.push("medium:supply_or_strategic_region".to_string());
    }
    if schemas.contains("hoi4skill.map_topology_plan.v1") {
        lanes.push("high:topology_manual_gate_required".to_string());
    }
    lanes
}

struct StateTransactionOperation {
    state_id: i64,
    field: String,
    old_value: String,
    new_value: String,
    source_layer: String,
    source_file: String,
    risk: String,
    ok: bool,
    blocker: Option<String>,
}

#[derive(Clone)]
struct StateResourceTextRequest {
    state_id: i64,
    state_query: String,
    state_name_key: String,
    state_localised_name: Option<String>,
    resource_id: String,
    resource_query: String,
    amount: i64,
    raw_segment: String,
}

struct StateResourceTextPlan {
    requests: Vec<StateResourceTextRequest>,
    blockers: Vec<String>,
}

struct SupplyRouteTextEndpoint {
    state_id: i64,
    state_query: String,
    state_name_key: String,
    state_localised_name: Option<String>,
    province_id: i64,
    province_localised_name: Option<String>,
    victory_point_value: i64,
    source_layer: String,
    source_file: String,
}

struct SupplyRouteTextPlan {
    endpoints: Vec<SupplyRouteTextEndpoint>,
    blockers: Vec<String>,
    questions: Vec<String>,
    requested_fortification: bool,
}

struct MapDataFile {
    layer: String,
    root: String,
    relative_path: String,
    risk: &'static str,
    category: &'static str,
    exists: bool,
}

fn map_data_audit_json(roots: &[MapDataRoot]) -> Result<String, String> {
    let mut files = Vec::new();
    let mut state_count = 0usize;
    let mut province_count = 0usize;
    for root in roots {
        state_count += scan_history_state_styles(&root.path)?.len();
        province_count += scan_province_definitions(&root.path)?
            .iter()
            .map(|summary| summary.province_count)
            .sum::<usize>();
        for spec in map_data_file_specs() {
            let path = root.path.join(spec.0);
            files.push(MapDataFile {
                layer: root.layer.clone(),
                root: root.path.display().to_string(),
                relative_path: map_data_slashes(spec.0),
                risk: spec.1,
                category: spec.2,
                exists: path.exists(),
            });
        }
        for extra in map_data_globbed_files(&root.path, "map/strategicregions")? {
            files.push(MapDataFile {
                layer: root.layer.clone(),
                root: root.path.display().to_string(),
                relative_path: extra,
                risk: "medium",
                category: "strategic_region",
                exists: true,
            });
        }
    }
    let target_has_map = files.iter().any(|file| {
        file.layer == "target" && file.exists && file.relative_path.starts_with("map/")
    });
    let has_parent = roots.iter().any(|root| root.layer == "parent");
    let mut blockers = Vec::new();
    if !has_parent {
        blockers.push(
            "no parent/dependency mod root was indexed; inherited map facts may be unknown"
                .to_string(),
        );
    }
    let status = if blockers.is_empty() {
        "map_data_audit_ready"
    } else {
        "blocked"
    };
    let next_commands = vec![
        "hoi4skill map-intent-plan --text <request> --mod-root <target> --game-root <HOI4 root> --mod-path <parent>".to_string(),
        "hoi4skill plan-history-edit <target> --text <request> --game-root <HOI4 root> --mod-path <parent>".to_string(),
        "hoi4skill state-batch-plan --state <id> --game-root <HOI4 root> --require-passed".to_string(),
    ];
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_data_audit.v1\",\n  \"status\": {},\n  \"ok\": {},\n  \"root_count\": {},\n  \"target_has_local_map_files\": {},\n  \"history_state_count\": {},\n  \"definition_province_count\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"files\": [\n{}\n  ],\n  \"risk_policy\": {},\n  \"next_commands\": {}\n}}\n",
        json_str(status),
        json_bool(blockers.is_empty()),
        roots.len(),
        json_bool(target_has_map),
        state_count,
        province_count,
        blockers.len(),
        json_array(&blockers),
        files
            .iter()
            .map(map_data_file_json)
            .collect::<Vec<_>>()
            .join(",\n"),
        json_array(&map_data_risk_policy()),
        json_array(&next_commands)
    ))
}

fn map_data_file_specs() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("history/states", "low", "state_history"),
        ("map/definition.csv", "high", "topology"),
        ("map/provinces.bmp", "high", "topology_bitmap"),
        ("map/terrain.bmp", "high", "topology_bitmap"),
        ("map/rivers.bmp", "high", "topology_bitmap"),
        ("map/adjacencies.csv", "high", "adjacency"),
        ("map/default.map", "high", "topology"),
        ("map/railways.txt", "medium", "supply_network"),
        ("map/supply_nodes.txt", "medium", "supply_network"),
        ("map/weatherpositions.txt", "medium", "weather"),
        ("map/supplyareas", "medium", "supply_area"),
        ("map/strategicregions", "medium", "strategic_region"),
    ]
}

fn map_data_slashes(raw: &str) -> String {
    raw.replace('\\', "/")
}

fn map_data_globbed_files(root: &Path, relative_dir: &str) -> Result<Vec<String>, String> {
    let dir = root.join(relative_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
            out.push(rel_slash(root, &path));
        }
    }
    out.sort();
    Ok(out)
}

fn map_data_file_json(file: &MapDataFile) -> String {
    format!(
        "    {{\"layer\": {}, \"root\": {}, \"relative_path\": {}, \"risk\": {}, \"category\": {}, \"exists\": {}}}",
        json_str(&file.layer),
        json_str(&file.root),
        json_str(&file.relative_path),
        json_str(file.risk),
        json_str(file.category),
        json_bool(file.exists)
    )
}

fn map_data_risk_policy() -> Vec<String> {
    vec![
        "low risk state history edits still require indexed state/province/resource/building evidence".to_string(),
        "medium risk railways, supply nodes, weather positions, and strategic regions are plan-only until endpoint and ownership checks pass".to_string(),
        "high risk topology files are never written from raw AI output; require map-topology-plan and explicit user approval".to_string(),
        "missing parent/game map evidence is blocking for inherited submods".to_string(),
    ]
}

fn state_transaction_operations(
    map: &ArgMap,
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: Option<&GameIndex>,
    states: &[i64],
) -> Result<Vec<StateTransactionOperation>, String> {
    let target_states = scan_history_state_styles(mod_root)?;
    let parent_states = scan_dependency_history_states(dependency_roots)?;
    let game_states = if let Some(game_root) = game_root {
        scan_history_state_styles(game_root)?
    } else {
        Vec::new()
    };
    let mut out = Vec::new();
    for state_id in states {
        let evidence =
            state_transaction_evidence(*state_id, &target_states, &parent_states, &game_states);
        let state_known =
            evidence.is_some() || index.is_some_and(|index| index.state_ids.contains(state_id));
        let (source_layer, source_file, risk) = evidence.clone().unwrap_or_else(|| {
            (
                "unknown".to_string(),
                format!("history/states/<state_{state_id}>.txt"),
                "blocking".to_string(),
            )
        });
        if !state_known {
            out.push(StateTransactionOperation {
                state_id: *state_id,
                field: "state".to_string(),
                old_value: "unknown".to_string(),
                new_value: state_id.to_string(),
                source_layer,
                source_file,
                risk,
                ok: false,
                blocker: Some(format!("state id `{state_id}` is not indexed or observed")),
            });
            continue;
        }
        let old_state =
            state_transaction_state_view(*state_id, &target_states, &parent_states, &game_states);
        if let Some(owner) = value(map, "owner") {
            out.push(state_transaction_tag_op(
                *state_id,
                "owner",
                old_state.as_ref().and_then(|state| state.owner.clone()),
                owner,
                &source_layer,
                &source_file,
                &risk,
                index,
            ));
        }
        if let Some(controller) = value(map, "controller") {
            out.push(state_transaction_tag_op(
                *state_id,
                "controller",
                old_state
                    .as_ref()
                    .and_then(|state| state.controller.clone()),
                controller,
                &source_layer,
                &source_file,
                &risk,
                index,
            ));
        }
        if let Some(population) = value(map, "population").or_else(|| value(map, "manpower")) {
            let ok = parse_int(population).is_some();
            out.push(StateTransactionOperation {
                state_id: *state_id,
                field: "manpower".to_string(),
                old_value: old_state
                    .as_ref()
                    .and_then(|state| state.manpower)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "absent".to_string()),
                new_value: population.to_string(),
                source_layer: source_layer.clone(),
                source_file: source_file.clone(),
                risk: risk.clone(),
                ok,
                blocker: (!ok).then(|| format!("manpower `{population}` is not an integer")),
            });
        }
        if let Some(category) = value(map, "state-category") {
            out.push(StateTransactionOperation {
                state_id: *state_id,
                field: "state_category".to_string(),
                old_value: old_state
                    .as_ref()
                    .and_then(|state| state.state_category.clone())
                    .unwrap_or_else(|| "absent".to_string()),
                new_value: category.to_string(),
                source_layer: source_layer.clone(),
                source_file: source_file.clone(),
                risk: risk.clone(),
                ok: true,
                blocker: None,
            });
        }
        for core in repeated_values(map, "core")
            .into_iter()
            .chain(repeated_values(map, "add-core"))
        {
            out.push(state_transaction_tag_op(
                *state_id,
                "add_core_of",
                None,
                core,
                &source_layer,
                &source_file,
                &risk,
                index,
            ));
        }
        for core in repeated_values(map, "remove-core") {
            out.push(state_transaction_tag_op(
                *state_id,
                "remove_core_of",
                None,
                core,
                &source_layer,
                &source_file,
                &risk,
                index,
            ));
        }
        for resource in repeated_values(map, "resource") {
            out.push(state_transaction_resource_op(
                *state_id,
                resource,
                &source_layer,
                &source_file,
                &risk,
                index,
                old_state.as_ref(),
            ));
        }
        for vp in repeated_values(map, "victory-point") {
            out.push(state_transaction_vp_op(
                *state_id,
                vp,
                &source_layer,
                &source_file,
                &risk,
                index,
                old_state.as_ref(),
            ));
        }
        for building in repeated_values(map, "building") {
            out.push(state_transaction_building_op(
                *state_id,
                building,
                &source_layer,
                &source_file,
                &risk,
                index,
            ));
        }
    }
    Ok(out)
}

fn state_transaction_operations_from_text(
    text: &str,
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: &GameIndex,
) -> Result<Vec<StateTransactionOperation>, String> {
    let target_states = scan_history_state_styles(mod_root)?;
    let parent_states = scan_dependency_history_states(dependency_roots)?;
    let game_states = if let Some(game_root) = game_root {
        scan_history_state_styles(game_root)?
    } else {
        Vec::new()
    };
    let plan = compile_state_resource_text_plan(text, index);
    let mut out = Vec::new();
    for blocker in plan.blockers {
        out.push(StateTransactionOperation {
            state_id: 0,
            field: "state_resource_text".to_string(),
            old_value: "unresolved".to_string(),
            new_value: text.to_string(),
            source_layer: "text".to_string(),
            source_file: "<natural-language-request>".to_string(),
            risk: "blocking".to_string(),
            ok: false,
            blocker: Some(blocker),
        });
    }
    for request in plan.requests {
        let evidence = state_transaction_evidence(
            request.state_id,
            &target_states,
            &parent_states,
            &game_states,
        );
        let (source_layer, source_file, risk) = evidence.clone().unwrap_or_else(|| {
            (
                "game_index".to_string(),
                format!("history/states/<state_{}>.txt", request.state_id),
                "override_requires_confirmation".to_string(),
            )
        });
        let old_state = state_transaction_state_view(
            request.state_id,
            &target_states,
            &parent_states,
            &game_states,
        );
        out.push(state_transaction_resource_op(
            request.state_id,
            &format!("{}={}", request.resource_id, request.amount),
            &source_layer,
            &source_file,
            &risk,
            Some(index),
            old_state.as_ref(),
        ));
    }
    if out.is_empty() {
        out.push(StateTransactionOperation {
            state_id: 0,
            field: "state_resource_text".to_string(),
            old_value: "unresolved".to_string(),
            new_value: text.to_string(),
            source_layer: "text".to_string(),
            source_file: "<natural-language-request>".to_string(),
            risk: "blocking".to_string(),
            ok: false,
            blocker: Some("no state resource request was found in text".to_string()),
        });
    }
    Ok(out)
}

fn compile_state_resource_text_plan(text: &str, index: &GameIndex) -> StateResourceTextPlan {
    let mut requests = Vec::new();
    let mut blockers = Vec::new();
    let mut pending_place_text = String::new();
    for segment in split_state_resource_segments(text) {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let Some((amount, resource_query, place_part)) =
            parse_state_resource_action_segment(segment)
        else {
            if !pending_place_text.is_empty() {
                pending_place_text.push(' ');
            }
            pending_place_text.push_str(segment);
            continue;
        };
        let mut place_text = String::new();
        if !pending_place_text.trim().is_empty() {
            place_text.push_str(pending_place_text.trim());
            place_text.push(' ');
        }
        place_text.push_str(place_part.trim());
        let state_matches = resolve_state_mentions(&place_text, index);
        let resource_id = resolve_resource_query(&resource_query, index);
        if state_matches.is_empty() {
            blockers.push(format!(
                "state name `{}` was not found in indexed STATE_* localisation",
                place_text.trim()
            ));
        }
        if resource_id.is_none() {
            blockers.push(format!(
                "resource `{resource_query}` was not found in indexed resource localisation"
            ));
        }
        if let Some(resource_id) = resource_id {
            for state in state_matches {
                requests.push(StateResourceTextRequest {
                    state_id: state.state_id,
                    state_query: state.query,
                    state_name_key: state.name_key,
                    state_localised_name: state.localised_name,
                    resource_id: resource_id.clone(),
                    resource_query: resource_query.clone(),
                    amount,
                    raw_segment: segment.to_string(),
                });
            }
        }
        pending_place_text.clear();
    }
    if requests.is_empty() && blockers.is_empty() && !text.trim().is_empty() {
        blockers.push("no `<state> add <amount> <resource>` pattern was found".to_string());
    }
    StateResourceTextPlan { requests, blockers }
}

fn compile_supply_route_text_plan(
    text: &str,
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: &GameIndex,
) -> Result<SupplyRouteTextPlan, String> {
    let requested_fortification = map_text_requests_fortification(text);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    let mentions = resolve_state_mentions_in_text_order(text, index);
    if mentions.len() < 2 {
        blockers.push(
            "railway route request needs two indexed state names, for example `<from state> to <to state>`"
                .to_string(),
        );
        questions.push("Which two exact states should the railway connect?".to_string());
        if requested_fortification {
            blockers.push("fortification request needs explicit province ids or state ids; no forts were auto-placed along the railway".to_string());
            questions.push(
                "Which exact provinces or states should receive forts protecting the railway?"
                    .to_string(),
            );
        }
        return Ok(SupplyRouteTextPlan {
            endpoints: Vec::new(),
            blockers,
            questions,
            requested_fortification,
        });
    }

    let target_states = mod_root
        .map(scan_history_state_styles)
        .transpose()?
        .unwrap_or_default();
    let parent_states = scan_dependency_history_states(dependency_roots)?;
    let game_states = game_root
        .map(scan_history_state_styles)
        .transpose()?
        .unwrap_or_default();

    let route_states = [mentions[0].1.clone(), mentions[1].1.clone()];
    let mut endpoints = Vec::new();
    for state in route_states {
        let evidence = province_query_evidence(
            state.state_id,
            mod_root,
            &target_states,
            &parent_states,
            game_root,
            &game_states,
        );
        let Some(evidence) = evidence else {
            blockers.push(format!(
                "state `{}` was resolved to `{}` but no history/states evidence was found",
                state.query, state.state_id
            ));
            continue;
        };
        let Some((province_id, value)) =
            largest_victory_point_for_state(&evidence.root, &evidence.state)?
        else {
            blockers.push(format!(
                "state `{}` ({}) has no victory_points block; ask user for an endpoint province id",
                state.query, state.state_id
            ));
            questions.push(format!(
                "Which province id should be used as the railway endpoint for state `{}` ({})?",
                state.query, state.state_id
            ));
            continue;
        };
        if !index.province_ids.contains(&province_id) {
            blockers.push(format!(
                "largest victory point province `{province_id}` in state `{}` is not indexed",
                state.state_id
            ));
            continue;
        }
        endpoints.push(SupplyRouteTextEndpoint {
            state_id: state.state_id,
            state_query: state.query,
            state_name_key: state.name_key,
            state_localised_name: state.localised_name,
            province_id,
            province_localised_name: preferred_localisation_alias(
                index,
                &format!("VICTORY_POINTS_{province_id}"),
            ),
            victory_point_value: value,
            source_layer: evidence.layer,
            source_file: evidence.state.file,
        });
    }

    if endpoints.len() == 2 {
        blockers.push(format!(
            "railway path from province `{}` to `{}` needs ordered intermediate province ids; direct endpoint-only writing is blocked",
            endpoints[0].province_id, endpoints[1].province_id
        ));
        questions.push(format!(
            "Which ordered province ids should the railway pass through between `{}` and `{}`?",
            endpoints[0].province_id, endpoints[1].province_id
        ));
        questions.push(format!(
            "Confirm whether supply nodes should be added only at endpoint provinces `{}` and `{}`.",
            endpoints[0].province_id, endpoints[1].province_id
        ));
    }
    if requested_fortification {
        blockers.push("fortification request needs explicit province ids or state ids; no forts were auto-placed along the railway".to_string());
        questions.push("Which exact provinces or states should receive forts protecting the railway? If none, say no forts.".to_string());
    }

    Ok(SupplyRouteTextPlan {
        endpoints,
        blockers,
        questions,
        requested_fortification,
    })
}

#[derive(Clone)]
struct ResolvedStateMention {
    state_id: i64,
    query: String,
    name_key: String,
    localised_name: Option<String>,
}

fn split_state_resource_segments(text: &str) -> Vec<String> {
    text.split(|ch: char| matches!(ch, ',' | '，' | ';' | '；' | '\n' | '\r' | '、'))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_state_resource_action_segment(segment: &str) -> Option<(i64, String, String)> {
    let (amount, number_start, number_end) = first_number_span(segment)?;
    let before_number = &segment[..number_start];
    let after_number = &segment[number_end..];
    if !before_number.contains("添加")
        && !before_number.contains("增加")
        && !before_number.contains("加")
        && !before_number.to_ascii_lowercase().contains("add")
    {
        return None;
    }
    let place_part = before_number
        .rsplit_once("添加")
        .or_else(|| before_number.rsplit_once("增加"))
        .or_else(|| before_number.rsplit_once("加"))
        .map(|(left, _)| left)
        .unwrap_or(before_number)
        .trim()
        .to_string();
    let resource_query = clean_resource_query(after_number);
    if resource_query.is_empty() {
        return None;
    }
    Some((amount, resource_query, place_part))
}

fn first_number_span(text: &str) -> Option<(i64, usize, usize)> {
    let mut start = None;
    let mut end = 0;
    for (idx, original_ch) in text.char_indices() {
        let ch = normalize_number_char(original_ch);
        if ch.is_ascii_digit() || (start.is_none() && (ch == '-' || ch == '+')) {
            if start.is_none() {
                start = Some(idx);
            }
            end = idx + original_ch.len_utf8();
        } else if let Some(start) = start {
            let value = parse_int(&text[start..end])?;
            return Some((value, start, end));
        }
    }
    let start = start?;
    Some((parse_int(&text[start..end])?, start, end))
}

fn clean_resource_query(value: &str) -> String {
    let mut out = value
        .trim()
        .trim_matches(|ch: char| {
            ch.is_ascii_punctuation()
                || matches!(ch, '。' | '，' | '；' | '、' | '：' | ':' | ' ' | '\t')
        })
        .to_string();
    for suffix in ["资源", "矿产", "矿脉", "矿石", "矿"] {
        if out.ends_with(suffix) && out.len() > suffix.len() {
            out.truncate(out.len() - suffix.len());
            break;
        }
    }
    out
}

fn resolve_state_mentions(text: &str, index: &GameIndex) -> Vec<ResolvedStateMention> {
    let normalized_text = normalize_state_resource_name(text);
    let mut matches = Vec::new();
    let mut seen = BTreeSet::new();
    let mut aliases = Vec::new();
    for (key, id) in &index.state_names {
        let localised_names = localisation_aliases_for_key(index, key);
        let primary_localised = index
            .localisation_entries
            .get(key)
            .cloned()
            .or_else(|| localised_names.iter().next().cloned());
        for alias in state_name_aliases(key, &localised_names) {
            let normalized_alias = normalize_state_resource_name(&alias);
            if normalized_alias.is_empty() {
                continue;
            }
            aliases.push((
                normalized_alias.len(),
                normalized_alias,
                alias,
                *id,
                key.clone(),
                primary_localised.clone(),
            ));
        }
    }
    aliases.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, normalized_alias, alias, state_id, name_key, localised_name) in aliases {
        if normalized_text.contains(&normalized_alias) && seen.insert(state_id) {
            let matched_localised_name = if alias == name_key {
                localised_name
            } else {
                Some(alias.clone())
            };
            matches.push(ResolvedStateMention {
                state_id,
                query: alias,
                name_key,
                localised_name: matched_localised_name,
            });
        }
    }
    matches.sort_by_key(|state| state.state_id);
    matches
}

fn resolve_state_mentions_in_text_order(
    text: &str,
    index: &GameIndex,
) -> Vec<(usize, ResolvedStateMention)> {
    let normalized_text = normalize_state_resource_name(text);
    let mut candidates = Vec::new();
    for (key, id) in &index.state_names {
        let localised_names = localisation_aliases_for_key(index, key);
        let primary_localised = index
            .localisation_entries
            .get(key)
            .cloned()
            .or_else(|| localised_names.iter().next().cloned());
        for alias in state_name_aliases(key, &localised_names) {
            let normalized_alias = normalize_state_resource_name(&alias);
            if normalized_alias.is_empty() {
                continue;
            }
            let Some(position) = normalized_text.find(&normalized_alias) else {
                continue;
            };
            let matched_localised_name = if alias == *key {
                primary_localised.clone()
            } else {
                Some(alias.clone())
            };
            candidates.push((
                position,
                normalized_alias.len(),
                ResolvedStateMention {
                    state_id: *id,
                    query: alias,
                    name_key: key.clone(),
                    localised_name: matched_localised_name,
                },
            ));
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then(right.1.cmp(&left.1)));
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (position, _, state) in candidates {
        if seen.insert(state.state_id) {
            out.push((position, state));
        }
    }
    out
}

fn state_name_aliases(key: &str, localised_names: &BTreeSet<String>) -> Vec<String> {
    let mut out = vec![key.to_string()];
    for localised in localised_names {
        let clean = localised.trim();
        if !clean.is_empty() {
            out.push(clean.to_string());
            for suffix in ["省", "市", "地区"] {
                if !clean.ends_with(suffix) {
                    out.push(format!("{clean}{suffix}"));
                }
            }
        }
    }
    out
}

fn largest_victory_point_for_state(
    root: &Path,
    state: &HistoryStateStyle,
) -> Result<Option<(i64, i64)>, String> {
    let Some(state_id) = state.id else {
        return Ok(None);
    };
    let path = root.join(&state.file);
    let text = strip_comments(&read_utf8_lossy(&path)?);
    let mut best: Option<(i64, i64)> = None;
    for block in direct_blocks_named(&text, "state") {
        if block_assignment(&block, "id").and_then(|value| value.parse::<i64>().ok())
            != Some(state_id)
        {
            continue;
        }
        for history in direct_blocks_named(&block, "history") {
            for vp_block in direct_blocks_named(&history, "victory_points") {
                for pair in collect_i64_from_text(&vp_block).chunks(2) {
                    let [province, value] = pair else {
                        continue;
                    };
                    if best.is_none_or(|(_, best_value)| *value > best_value) {
                        best = Some((*province, *value));
                    }
                }
            }
        }
    }
    Ok(best)
}

fn map_text_requests_fortification(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("要塞")
        || text.contains("堡垒")
        || text.contains("防线")
        || lower.contains("fort")
        || lower.contains("bunker")
}

fn resolve_resource_query(query: &str, index: &GameIndex) -> Option<String> {
    let normalized_query = normalize_resource_query_name(query);
    if index.resources.contains(query) {
        return Some(query.to_string());
    }
    for resource in &index.resources {
        if normalize_resource_query_name(resource) == normalized_query {
            return Some(resource.clone());
        }
        for key in [
            format!("state_resource_{resource}"),
            format!("temporary_state_resource_{resource}"),
        ] {
            for localised in localisation_aliases_for_key(index, &key) {
                let normalized_localised = normalize_resource_query_name(&localised);
                if !normalized_localised.is_empty() && normalized_localised == normalized_query {
                    return Some(resource.clone());
                }
            }
        }
    }
    None
}

fn localisation_aliases_for_key(index: &GameIndex, key: &str) -> BTreeSet<String> {
    let mut out = index
        .localisation_entry_aliases
        .get(key)
        .cloned()
        .unwrap_or_default();
    if let Some(value) = index.localisation_entries.get(key) {
        out.insert(value.clone());
    }
    out
}

fn preferred_localisation_alias(index: &GameIndex, key: &str) -> Option<String> {
    let aliases = localisation_aliases_for_key(index, key);
    aliases
        .iter()
        .find(|value| value.chars().any(is_cjk_char))
        .cloned()
        .or_else(|| aliases.iter().next().cloned())
}

fn is_cjk_char(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

fn normalize_state_resource_name(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                Some(ch.to_ascii_lowercase())
            } else if matches!(ch, '省' | '市' | '州' | '地' | '区' | ' ' | '\t') {
                None
            } else if ch.is_ascii_punctuation()
                || matches!(ch, '，' | '。' | '、' | '；' | '：' | '（' | '）')
            {
                None
            } else {
                Some(ch)
            }
        })
        .collect()
}

fn normalize_resource_query_name(value: &str) -> String {
    let normalized_typo = value.replace('媒', "煤");
    let mut out = normalize_state_resource_name(&normalized_typo);
    for suffix in ["资源", "矿产", "矿脉", "矿石", "矿"] {
        let suffix = normalize_state_resource_name(suffix);
        if out.ends_with(&suffix) && out.len() > suffix.len() {
            out.truncate(out.len() - suffix.len());
            break;
        }
    }
    out
}

fn state_transaction_evidence(
    state_id: i64,
    target_states: &[HistoryStateStyle],
    parent_states: &[DependencyHistoryState],
    game_states: &[HistoryStateStyle],
) -> Option<(String, String, String)> {
    if let Some(state) = find_state_by_id(target_states, state_id) {
        return Some(("target".to_string(), state.file.clone(), "low".to_string()));
    }
    if let Some(state) = find_dependency_state_by_id(parent_states, state_id) {
        return Some((
            "parent".to_string(),
            state.state.file,
            "override_requires_confirmation".to_string(),
        ));
    }
    if let Some(state) = find_state_by_id(game_states, state_id) {
        return Some((
            "game".to_string(),
            state.file.clone(),
            "override_requires_confirmation".to_string(),
        ));
    }
    None
}

fn state_transaction_state_view(
    state_id: i64,
    target_states: &[HistoryStateStyle],
    parent_states: &[DependencyHistoryState],
    game_states: &[HistoryStateStyle],
) -> Option<HistoryStateStyle> {
    find_state_by_id(target_states, state_id)
        .cloned()
        .or_else(|| find_dependency_state_by_id(parent_states, state_id).map(|state| state.state))
        .or_else(|| find_state_by_id(game_states, state_id).cloned())
}

struct ProvinceQueryEvidence {
    state: HistoryStateStyle,
    layer: String,
    root: PathBuf,
    risk: String,
}

struct SupplyNetworkOperation {
    kind: String,
    province_ids: Vec<i64>,
    level: Option<i64>,
    source_file: String,
    risk: String,
    ok: bool,
    blocker: Option<String>,
}

struct NetworkFileEvidence {
    layer: String,
    relative_path: String,
    exists: bool,
}

struct StrategicRegionSummary {
    layer: String,
    file: String,
    id: Option<i64>,
    name: Option<String>,
    provinces: Vec<i64>,
}

fn map_topology_plan_json(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: Option<&GameIndex>,
    map: &ArgMap,
) -> Result<String, String> {
    let mut roots = Vec::new();
    if let Some(root) = game_root {
        roots.push(MapDataRoot {
            layer: "game".to_string(),
            path: root.to_path_buf(),
        });
    }
    for root in dependency_roots {
        roots.push(MapDataRoot {
            layer: "parent".to_string(),
            path: root.clone(),
        });
    }
    roots.push(MapDataRoot {
        layer: "target".to_string(),
        path: mod_root.to_path_buf(),
    });
    let required_files = map_topology_required_files(&roots);
    let missing_required = [
        "map/definition.csv",
        "map/provinces.bmp",
        "map/terrain.bmp",
        "map/rivers.bmp",
        "map/adjacencies.csv",
        "map/default.map",
        "map/continent.txt",
        "map/ambient_object.txt",
    ]
    .into_iter()
    .filter(|relative_path| {
        !required_files
            .iter()
            .any(|file| file.relative_path == *relative_path && file.exists)
    })
    .map(str::to_string)
    .collect::<Vec<_>>();
    let new_province = value(map, "new-province")
        .or_else(|| value(map, "province-id"))
        .and_then(parse_int);
    let delete_province = value(map, "delete-province").and_then(parse_int);
    let rgb = value(map, "rgb").and_then(parse_rgb_triplet);
    let state_id = value(map, "state-id")
        .or_else(|| value(map, "state"))
        .and_then(parse_int);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if index.is_none() {
        blockers.push(
            "map-topology-plan requires --game-root so province and adjacency ids can be indexed"
                .to_string(),
        );
    }
    if !missing_required.is_empty() {
        blockers.push(format!(
            "topology evidence files missing: {}",
            missing_required.join(", ")
        ));
    }
    if new_province.is_none()
        && delete_province.is_none()
        && repeated_values(map, "adjacency").is_empty()
    {
        blockers.push("no topology operation requested; specify --new-province, --delete-province, or --adjacency".to_string());
    }
    if let Some(id) = new_province {
        if index.is_some_and(|index| index.province_ids.contains(&id)) {
            blockers.push(format!(
                "new province id `{id}` already exists in indexed map data"
            ));
        }
        if rgb.is_none() {
            blockers.push("new province requires --rgb R,G,B".to_string());
        } else if map_rgb_exists(&roots, rgb.unwrap())? {
            blockers.push(format!(
                "new province rgb `{}` already exists in definition.csv",
                rgb_string(rgb.unwrap())
            ));
        }
        if state_id.is_none() {
            blockers.push("new province requires --state-id for state assignment".to_string());
        }
        if !map.flags.contains("definition-row") {
            blockers.push("new province requires --definition-row evidence".to_string());
        }
        if !map.flags.contains("province-pixel-evidence") {
            blockers.push("new province requires --province-pixel-evidence".to_string());
        }
    }
    let delete_references = if let Some(id) = delete_province {
        map_topology_references(&roots, id)?
    } else {
        Vec::new()
    };
    for adjacency in repeated_values(map, "adjacency") {
        let values = collect_i64_from_text(adjacency);
        if values.len() < 2 {
            blockers.push(format!(
                "adjacency `{adjacency}` must contain two province ids"
            ));
        }
        for id in values.iter().take(3) {
            if index.is_some_and(|index| !index.province_ids.contains(id)) {
                blockers.push(format!(
                    "adjacency `{adjacency}` references unindexed province `{id}`"
                ));
            }
        }
    }
    if !map.flags.contains("review-only") {
        questions.push("Topology edits are high risk; rerun with --review-only to generate a review pack, then gate with --manual-confirmed.".to_string());
        blockers
            .push("map-topology-plan is review-only; direct apply is not supported".to_string());
    }
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_topology_plan.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"risk\": \"high\",\n  \"review_only\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"new_province\": {},\n  \"delete_province\": {},\n  \"rgb\": {},\n  \"state_id\": {},\n  \"required_files\": [{}],\n  \"delete_references\": {},\n  \"adjacency_requests\": {},\n  \"changed_files\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "map_topology_review_pack_ready" } else { "blocked" }),
        json_bool(map.flags.contains("review-only")),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.map(|root| root.display().to_string()).as_deref()),
        json_optional_i64(new_province),
        json_optional_i64(delete_province),
        json_optional_str(rgb.map(rgb_string).as_deref()),
        json_optional_i64(state_id),
        required_files
            .iter()
            .map(supply_network_file_json)
            .collect::<Vec<_>>()
            .join(", "),
        json_array(&delete_references),
        json_array(&repeated_values(map, "adjacency").into_iter().map(str::to_string).collect::<Vec<_>>()),
        json_array(&map_topology_changed_files(new_province, delete_province, !repeated_values(map, "adjacency").is_empty())),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&["hoi4skill map-topology-gate --input <plan> --manual-confirmed --require-passed".to_string()]),
        json_array(&[
            "new province requires unique RGB, definition row, province pixel evidence, and state assignment".to_string(),
            "adjacency endpoints, through province, and deleted province references must be indexed local evidence".to_string(),
            "topology plans never apply bitmap or CSV edits directly from AI output".to_string(),
        ])
    ))
}

fn map_override_risk_audit_json(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    plan: Option<&Path>,
) -> Result<String, String> {
    let mut rows = Vec::new();
    for spec in map_data_file_specs() {
        let rel = map_data_slashes(spec.0);
        let target = mod_root.join(spec.0);
        if target.exists() {
            for parent in dependency_roots {
                let parent_file = parent.join(spec.0);
                if parent_file.exists() {
                    rows.push(map_override_row_json(
                        &rel,
                        "parent",
                        spec.1,
                        &target,
                        &parent_file,
                    )?);
                }
            }
            if let Some(game_root) = game_root {
                let game_file = game_root.join(spec.0);
                if game_file.exists() {
                    rows.push(map_override_row_json(
                        &rel, "game", spec.1, &target, &game_file,
                    )?);
                }
            }
        }
    }
    for rel in map_data_globbed_files(mod_root, "map/strategicregions")? {
        let target = mod_root.join(rel.replace('/', "\\"));
        for parent in dependency_roots {
            let parent_file = parent.join(rel.replace('/', "\\"));
            if parent_file.exists() {
                rows.push(map_override_row_json(
                    &rel,
                    "parent",
                    "medium",
                    &target,
                    &parent_file,
                )?);
            }
        }
    }
    let mut blockers = Vec::new();
    let stale_changed_files = if let Some(plan) = plan {
        if !plan.exists() {
            blockers.push(format!("map plan `{}` does not exist", plan.display()));
            Vec::new()
        } else {
            let text = read_utf8_lossy(plan)?;
            json_string_array_field_simple(&text, "changed_files")
                .into_iter()
                .filter(|rel| !mod_root.join(rel.replace('/', "\\")).exists())
                .collect::<Vec<_>>()
        }
    } else {
        Vec::new()
    };
    if !stale_changed_files.is_empty() {
        blockers.push(format!(
            "stale map plan references missing target files: {}",
            stale_changed_files.join(", ")
        ));
    }
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_override_risk_audit.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"override_count\": {},\n  \"overrides\": [{}],\n  \"stale_changed_files\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "map_override_risk_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.map(|root| root.display().to_string()).as_deref()),
        rows.len(),
        rows.join(", "),
        json_array(&stale_changed_files),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "target map files overriding parent/game files must list target and source hashes".to_string(),
            "stale map plans cannot be reused after files disappear or parent evidence changes".to_string(),
            "high-risk topology overrides require P47 gate and P50 release evidence".to_string(),
        ])
    ))
}

fn map_override_row_json(
    rel: &str,
    source_layer: &str,
    risk: &str,
    target: &Path,
    source: &Path,
) -> Result<String, String> {
    let target_hash = simple_file_hash_hex(target)?;
    let source_hash = simple_file_hash_hex(source)?;
    Ok(format!(
        "{{\"relative_path\": {}, \"source_layer\": {}, \"risk\": {}, \"target_hash\": {}, \"source_hash\": {}, \"changed\": {}}}",
        json_str(rel),
        json_str(source_layer),
        json_str(risk),
        json_str(&target_hash),
        json_str(&source_hash),
        json_bool(target_hash != source_hash)
    ))
}

fn simple_file_hash_hex(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&bytes, &mut hasher);
    Ok(format!("{:016x}", std::hash::Hasher::finish(&hasher)))
}

fn map_runtime_gate_json(map: &ArgMap) -> Result<String, String> {
    let baseline = value(map, "baseline")
        .map(normalize_path)
        .transpose()?
        .filter(|path| path.exists())
        .map(|path| read_utf8_lossy(&path))
        .transpose()?
        .unwrap_or_default();
    let log_paths = ["error-log", "map-log", "setup-log"]
        .into_iter()
        .filter_map(|key| value(map, key).map(|value| (key, PathBuf::from(value))))
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut blockers = Vec::new();
    if log_paths.is_empty() {
        blockers
            .push("map-runtime-gate requires --error-log, --map-log, or --setup-log".to_string());
    }
    for (kind, path) in &log_paths {
        if !path.exists() {
            blockers.push(format!("{kind} `{}` does not exist", path.display()));
            continue;
        }
        for line in read_utf8_lossy(path)?.lines() {
            if map_runtime_line_is_relevant(line) && !baseline.contains(line) {
                findings.push(format!("{kind}:{}", line.trim()));
            }
        }
    }
    if !findings.is_empty() {
        blockers.push(format!(
            "new map runtime errors detected: {}",
            findings.len()
        ));
    }
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_runtime_gate.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"log_count\": {},\n  \"new_error_count\": {},\n  \"new_errors\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"manual_smoke_checklist\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "map_runtime_gate_ready" } else { "blocked" }),
        log_paths.len(),
        findings.len(),
        json_array(&findings),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "launch HOI4 with the target mod and parent mod enabled".to_string(),
            "open the affected state/region and verify map mode, supply map mode, and air map mode".to_string(),
            "compare error.log/map.log/setup.log against the saved baseline".to_string(),
        ]),
        json_array(&[
            "new province/state/adjacency/strategic-region/supply runtime errors block release".to_string(),
            "baseline log lines are reported separately by the caller and do not block".to_string(),
        ])
    ))
}

fn map_runtime_line_is_relevant(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    [
        "province not found",
        "state has no owner",
        "duplicate province",
        "invalid adjacency",
        "strategic region",
        "supply node",
        "railway",
        "map invalid",
        "definition.csv",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn map_release_gate_json(mod_root: &Path, reports: &[PathBuf]) -> Result<String, String> {
    let mut blockers = Vec::new();
    if reports.is_empty() {
        blockers
            .push("map-release-gate requires --report entries for P41-P49 evidence".to_string());
    }
    let mut report_rows = Vec::new();
    let mut has_runtime = false;
    let mut has_topology_gate = false;
    for report in reports {
        if !report.exists() {
            blockers.push(format!("report `{}` does not exist", report.display()));
            continue;
        }
        let text = read_utf8_lossy(report)?;
        let schema = json_schema_name(&text).unwrap_or_else(|| "unknown".to_string());
        if schema == "hoi4skill.map_runtime_gate.v1" {
            has_runtime = true;
        }
        if schema == "hoi4skill.map_topology_gate.v1" {
            has_topology_gate = true;
        }
        let ok = text.contains("\"ok\": true") && !text.contains("\"status\": \"blocked\"");
        if !ok {
            blockers.push(format!("report `{}` is not ok", report.display()));
        }
        report_rows.push(format!(
            "{{\"path\": {}, \"schema\": {}, \"ok\": {}}}",
            json_str(&report.display().to_string()),
            json_str(&schema),
            json_bool(ok)
        ));
    }
    if !has_runtime {
        blockers.push("map-release-gate requires a P49 map-runtime-gate report".to_string());
    }
    if reports.iter().any(|path| {
        path.exists()
            && read_utf8_lossy(path)
                .map(|text| text.contains("hoi4skill.map_topology_plan.v1"))
                .unwrap_or(false)
    }) && !has_topology_gate
    {
        blockers.push("topology plan evidence requires a map-topology-gate report".to_string());
    }
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_release_gate.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"report_count\": {},\n  \"reports\": [{}],\n  \"release_manifest\": {},\n  \"rollback_plan\": {},\n  \"manual_smoke_checklist\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "map_release_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        reports.len(),
        report_rows.join(", "),
        json_array(&["all supplied P41-P49 reports are ok".to_string(), "runtime regression evidence is present".to_string()]),
        json_array(&["restore changed map/history files from VCS or backup if runtime smoke fails".to_string()]),
        json_array(&[
            "start game with mod and parent enabled".to_string(),
            "open supply/air/map modes for changed provinces and states".to_string(),
            "verify no new error.log/map.log/setup.log lines".to_string(),
        ]),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "P41-P49 blockers must be zero before release".to_string(),
            "topology high-risk plans require manual confirmation gate".to_string(),
            "runtime/map log regression evidence is mandatory".to_string(),
        ])
    ))
}

fn json_schema_name(text: &str) -> Option<String> {
    let marker = "\"schema\":";
    let start = text.find(marker)? + marker.len();
    let rest = text[start..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn map_topology_required_files(roots: &[MapDataRoot]) -> Vec<NetworkFileEvidence> {
    let paths = [
        "map/definition.csv",
        "map/provinces.bmp",
        "map/terrain.bmp",
        "map/rivers.bmp",
        "map/adjacencies.csv",
        "map/default.map",
        "map/continent.txt",
        "map/ambient_object.txt",
    ];
    let mut out = Vec::new();
    for root in roots {
        for relative_path in paths {
            out.push(NetworkFileEvidence {
                layer: root.layer.clone(),
                relative_path: relative_path.to_string(),
                exists: root.path.join(relative_path).exists(),
            });
        }
    }
    out
}

fn parse_rgb_triplet(raw: &str) -> Option<(i64, i64, i64)> {
    let values = collect_i64_from_text(raw);
    (values.len() == 3 && values.iter().all(|value| (0..=255).contains(value)))
        .then_some((values[0], values[1], values[2]))
}

fn rgb_string(rgb: (i64, i64, i64)) -> String {
    format!("{},{},{}", rgb.0, rgb.1, rgb.2)
}

fn map_rgb_exists(roots: &[MapDataRoot], rgb: (i64, i64, i64)) -> Result<bool, String> {
    for root in roots {
        let path = root.path.join("map/definition.csv");
        if path.exists() {
            for line in read_utf8_lossy(&path)?.lines() {
                let cols = line.split(';').map(str::trim).collect::<Vec<_>>();
                if cols.len() >= 4
                    && parse_int(cols[1]) == Some(rgb.0)
                    && parse_int(cols[2]) == Some(rgb.1)
                    && parse_int(cols[3]) == Some(rgb.2)
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn map_topology_references(roots: &[MapDataRoot], province: i64) -> Result<Vec<String>, String> {
    let mut refs = BTreeSet::new();
    for root in roots {
        for rel in ["history/states", "history/units", "map/strategicregions"] {
            for file in txt_files(&root.path, rel)? {
                let text = read_utf8_lossy(&file)?;
                if collect_i64_from_text(&text).contains(&province) {
                    refs.insert(format!("{}:{}", root.layer, rel_slash(&root.path, &file)));
                }
            }
        }
        for rel in [
            "map/railways.txt",
            "map/supply_nodes.txt",
            "map/adjacencies.csv",
            "map/weatherpositions.txt",
        ] {
            let file = root.path.join(rel);
            if file.exists() && collect_i64_from_text(&read_utf8_lossy(&file)?).contains(&province)
            {
                refs.insert(format!("{}:{rel}", root.layer));
            }
        }
    }
    Ok(refs.into_iter().collect())
}

fn map_topology_changed_files(
    new_province: Option<i64>,
    delete_province: Option<i64>,
    has_adjacency: bool,
) -> Vec<String> {
    let mut files = BTreeSet::new();
    if new_province.is_some() || delete_province.is_some() {
        for file in [
            "map/definition.csv",
            "map/provinces.bmp",
            "map/terrain.bmp",
            "history/states/<state>",
        ] {
            files.insert(file.to_string());
        }
    }
    if has_adjacency {
        files.insert("map/adjacencies.csv".to_string());
    }
    files.into_iter().collect()
}

fn strategic_region_plan_json(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: Option<&GameIndex>,
    map: &ArgMap,
) -> Result<String, String> {
    let mut roots = Vec::new();
    if let Some(root) = game_root {
        roots.push(MapDataRoot {
            layer: "game".to_string(),
            path: root.to_path_buf(),
        });
    }
    for root in dependency_roots {
        roots.push(MapDataRoot {
            layer: "parent".to_string(),
            path: root.clone(),
        });
    }
    roots.push(MapDataRoot {
        layer: "target".to_string(),
        path: mod_root.to_path_buf(),
    });
    let regions = scan_strategic_regions(&roots)?;
    let parent_region_files = roots
        .iter()
        .filter(|root| root.layer == "parent")
        .filter(|root| root.path.join("map/strategicregions").exists())
        .count();
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if index.is_none() {
        blockers.push(
            "strategic-region-plan requires --game-root so province ids can be indexed".to_string(),
        );
    }
    if !dependency_roots.is_empty() && parent_region_files == 0 {
        blockers.push("parent mod strategicregions were not indexed; do not rewrite air regions from stale evidence".to_string());
    }
    blockers.extend(strategic_region_duplicate_province_blockers(&regions));
    let operations = strategic_region_operations(map, &regions, index);
    blockers.extend(
        operations
            .iter()
            .filter(|operation| operation.contains("\"ok\": false"))
            .map(|operation| {
                if operation.contains("province_not_indexed") {
                    "strategic region operation references an unindexed province".to_string()
                } else if operation.contains("region_not_found") {
                    "strategic region operation references an unknown region id".to_string()
                } else if operation.contains("name_conflict") {
                    "strategic region rename conflicts with an existing region name".to_string()
                } else {
                    "strategic region operation is incomplete".to_string()
                }
            }),
    );
    if operations.is_empty() {
        questions.push("Specify --region-id with --province, --move-province <province=region>, --name, or --weather-position <province>.".to_string());
    }
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.strategic_region_plan.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"region_count\": {},\n  \"regions\": [{}],\n  \"operations\": [{}],\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"changed_files\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "strategic_region_plan_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.map(|root| root.display().to_string()).as_deref()),
        regions.len(),
        regions
            .iter()
            .take(40)
            .map(strategic_region_json)
            .collect::<Vec<_>>()
            .join(", "),
        operations.join(", "),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&strategic_region_changed_files(&regions, map)),
        json_array(&[
            "a province cannot belong to two strategic regions in the effective local evidence set".to_string(),
            "region id and name changes must reference an indexed existing strategic region".to_string(),
            "weather or air-region requests must list affected province ids and remain review-only until runtime gates pass".to_string(),
            "missing parent-mod strategicregions evidence blocks submod edits based on stale templates".to_string(),
        ])
    ))
}

fn scan_strategic_regions(roots: &[MapDataRoot]) -> Result<Vec<StrategicRegionSummary>, String> {
    let mut out = Vec::new();
    for root in roots {
        for file in txt_files(&root.path, "map/strategicregions")? {
            let rel = rel_slash(&root.path, &file);
            let text = strip_comments(&read_utf8_lossy(&file)?);
            for block in direct_blocks_named(&text, "strategic_region") {
                out.push(StrategicRegionSummary {
                    layer: root.layer.clone(),
                    file: rel.clone(),
                    id: block_assignment(&block, "id").and_then(|value| value.parse().ok()),
                    name: block_assignment(&block, "name"),
                    provinces: direct_blocks_named(&block, "provinces")
                        .into_iter()
                        .flat_map(|block| collect_i64_from_text(&block))
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                });
            }
        }
    }
    Ok(out)
}

fn strategic_region_json(region: &StrategicRegionSummary) -> String {
    format!(
        "{{\"layer\": {}, \"file\": {}, \"id\": {}, \"name\": {}, \"province_count\": {}, \"province_sample\": {}}}",
        json_str(&region.layer),
        json_str(&region.file),
        json_optional_i64(region.id),
        json_optional_str(region.name.as_deref()),
        region.provinces.len(),
        json_i64_array(&region.provinces.iter().take(20).copied().collect::<Vec<_>>())
    )
}

fn strategic_region_duplicate_province_blockers(regions: &[StrategicRegionSummary]) -> Vec<String> {
    let effective_layer = if regions.iter().any(|region| region.layer == "target") {
        "target"
    } else if regions.iter().any(|region| region.layer == "parent") {
        "parent"
    } else {
        "game"
    };
    let mut seen: BTreeMap<i64, BTreeSet<i64>> = BTreeMap::new();
    for region in regions
        .iter()
        .filter(|region| region.layer == effective_layer)
    {
        if let Some(region_id) = region.id {
            for province in &region.provinces {
                seen.entry(*province).or_default().insert(region_id);
            }
        }
    }
    seen.into_iter()
        .filter_map(|(province, regions)| {
            (regions.len() > 1).then(|| {
                format!(
                    "province `{province}` appears in multiple strategic regions: {}",
                    regions
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
        })
        .collect()
}

fn strategic_region_operations(
    map: &ArgMap,
    regions: &[StrategicRegionSummary],
    index: Option<&GameIndex>,
) -> Vec<String> {
    let mut out = Vec::new();
    let region_id = value(map, "region-id")
        .or_else(|| value(map, "region"))
        .and_then(parse_int);
    for province in repeated_values(map, "province") {
        out.push(strategic_region_operation_json(
            "add_province",
            parse_int(province),
            region_id,
            value(map, "name"),
            regions,
            index,
        ));
    }
    for raw in repeated_values(map, "move-province") {
        let values = collect_i64_from_text(raw);
        out.push(strategic_region_operation_json(
            "move_province",
            values.first().copied(),
            values.get(1).copied().or(region_id),
            None,
            regions,
            index,
        ));
    }
    if let Some(name) = value(map, "name") {
        out.push(strategic_region_operation_json(
            "rename_region",
            None,
            region_id,
            Some(name),
            regions,
            index,
        ));
    }
    for province in repeated_values(map, "weather-position") {
        out.push(strategic_region_operation_json(
            "weather_position_review",
            parse_int(province),
            region_id,
            None,
            regions,
            index,
        ));
    }
    out
}

fn strategic_region_operation_json(
    kind: &str,
    province: Option<i64>,
    region_id: Option<i64>,
    name: Option<&str>,
    regions: &[StrategicRegionSummary],
    index: Option<&GameIndex>,
) -> String {
    let province_ok =
        province.is_none_or(|id| index.is_some_and(|index| index.province_ids.contains(&id)));
    let region_ok = region_id.is_some_and(|id| regions.iter().any(|region| region.id == Some(id)));
    let name_conflict = name.is_some_and(|name| {
        regions
            .iter()
            .any(|region| region.name.as_deref() == Some(name))
    });
    let ok = match kind {
        "rename_region" => region_ok && !name_conflict && name.is_some(),
        "weather_position_review" => province_ok && province.is_some(),
        _ => province_ok && province.is_some() && region_ok,
    };
    let blocker = if ok {
        "null".to_string()
    } else if !province_ok {
        json_str("province_not_indexed")
    } else if !region_ok && kind != "weather_position_review" {
        json_str("region_not_found")
    } else if name_conflict {
        json_str("name_conflict")
    } else {
        json_str("incomplete_operation")
    };
    format!(
        "{{\"kind\": {}, \"province\": {}, \"region_id\": {}, \"name\": {}, \"ok\": {}, \"blocker\": {}}}",
        json_str(kind),
        json_optional_i64(province),
        json_optional_i64(region_id),
        json_optional_str(name),
        json_bool(ok),
        blocker
    )
}

fn strategic_region_changed_files(regions: &[StrategicRegionSummary], map: &ArgMap) -> Vec<String> {
    let region_id = value(map, "region-id")
        .or_else(|| value(map, "region"))
        .and_then(parse_int);
    regions
        .iter()
        .filter(|region| region_id.is_none() || region.id == region_id)
        .map(|region| region.file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn supply_network_plan_json(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: Option<&GameIndex>,
    state_id: Option<i64>,
    map: &ArgMap,
) -> Result<String, String> {
    let mut roots = Vec::new();
    if let Some(root) = game_root {
        roots.push(MapDataRoot {
            layer: "game".to_string(),
            path: root.to_path_buf(),
        });
    }
    for root in dependency_roots {
        roots.push(MapDataRoot {
            layer: "parent".to_string(),
            path: root.clone(),
        });
    }
    roots.push(MapDataRoot {
        layer: "target".to_string(),
        path: mod_root.to_path_buf(),
    });
    let network_files = supply_network_file_evidence(&roots);
    let has_network_file = network_files.iter().any(|file| file.exists);
    let target_states = scan_history_state_styles(mod_root)?;
    let parent_states = scan_dependency_history_states(dependency_roots)?;
    let game_states = game_root
        .map(scan_history_state_styles)
        .transpose()?
        .unwrap_or_default();
    let state_view = state_id.and_then(|id| {
        state_transaction_state_view(id, &target_states, &parent_states, &game_states)
    });
    let existing_supply_nodes = scan_existing_supply_nodes(&roots)?;
    let existing_railways = scan_existing_railway_lines(&roots)?;
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if index.is_none() {
        blockers.push(
            "supply-network-plan requires --game-root so province endpoints can be indexed"
                .to_string(),
        );
    }
    if !has_network_file {
        blockers.push(
            "no indexed railways.txt or supply_nodes.txt found in game, parent, or target roots"
                .to_string(),
        );
    }
    if repeated_values(map, "railway").is_empty()
        && repeated_values(map, "supply-node").is_empty()
        && repeated_values(map, "naval-base").is_empty()
        && repeated_values(map, "air-base").is_empty()
        && repeated_values(map, "infrastructure").is_empty()
    {
        questions.push("Specify --railway <from-to=level>, --supply-node <province>, --naval-base <province>, --air-base <level>, or --infrastructure <level>.".to_string());
        blockers.push("no supply-network operation was requested".to_string());
    }
    let mut operations = Vec::new();
    for railway in repeated_values(map, "railway") {
        operations.push(supply_network_railway_op(
            railway,
            index,
            &existing_railways,
            has_network_file,
        ));
    }
    for node in repeated_values(map, "supply-node") {
        operations.push(supply_network_supply_node_op(
            node,
            index,
            &existing_supply_nodes,
            has_network_file,
        ));
    }
    for province in repeated_values(map, "naval-base") {
        operations.push(supply_network_state_province_building_op(
            "naval_base",
            province,
            index,
            state_id,
            state_view.as_ref(),
        ));
    }
    for level in repeated_values(map, "air-base") {
        operations.push(supply_network_state_level_building_op(
            "air_base", level, index, state_id,
        ));
    }
    for level in repeated_values(map, "infrastructure") {
        operations.push(supply_network_state_level_building_op(
            "infrastructure",
            level,
            index,
            state_id,
        ));
    }
    blockers.extend(operations.iter().filter_map(|op| op.blocker.clone()));
    let changed_files = supply_network_changed_files(&operations);
    let ok = blockers.is_empty();
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.supply_network_plan.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"state_id\": {},\n  \"network_files\": [{}],\n  \"operation_count\": {},\n  \"operations\": [\n{}\n  ],\n  \"changed_files\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "supply_network_plan_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.map(|root| root.display().to_string()).as_deref()),
        json_optional_i64(state_id),
        network_files
            .iter()
            .map(supply_network_file_json)
            .collect::<Vec<_>>()
            .join(", "),
        operations.len(),
        operations
            .iter()
            .map(supply_network_operation_json)
            .collect::<Vec<_>>()
            .join(",\n"),
        json_array(&changed_files),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&["hoi4skill supply-network-apply --input <plan> --execute --final-check --require-passed".to_string()]),
        json_array(&[
            "railway endpoints and supply node provinces must be indexed local province ids".to_string(),
            "supply node province must not already exist in target, parent, or game supply_nodes.txt".to_string(),
            "naval base province must belong to the selected state when --state-id is supplied".to_string(),
            "network files are review-pack only until runtime/map release gates pass".to_string(),
        ])
    ))
}

fn supply_network_file_evidence(roots: &[MapDataRoot]) -> Vec<NetworkFileEvidence> {
    let mut out = Vec::new();
    for root in roots {
        for relative_path in ["map/railways.txt", "map/supply_nodes.txt"] {
            out.push(NetworkFileEvidence {
                layer: root.layer.clone(),
                relative_path: relative_path.to_string(),
                exists: root.path.join(relative_path).exists(),
            });
        }
    }
    out
}

fn supply_network_file_json(file: &NetworkFileEvidence) -> String {
    format!(
        "{{\"layer\": {}, \"relative_path\": {}, \"exists\": {}}}",
        json_str(&file.layer),
        json_str(&file.relative_path),
        json_bool(file.exists)
    )
}

fn scan_existing_supply_nodes(roots: &[MapDataRoot]) -> Result<BTreeSet<i64>, String> {
    let mut nodes = BTreeSet::new();
    for root in roots {
        let path = root.path.join("map/supply_nodes.txt");
        if path.exists() {
            collect_i64_from_text(&read_utf8_lossy(&path)?)
                .into_iter()
                .for_each(|id| {
                    nodes.insert(id);
                });
        }
    }
    Ok(nodes)
}

fn scan_existing_railway_lines(roots: &[MapDataRoot]) -> Result<Vec<Vec<i64>>, String> {
    let mut lines = Vec::new();
    for root in roots {
        let path = root.path.join("map/railways.txt");
        if path.exists() {
            for line in read_utf8_lossy(&path)?.lines() {
                let ids = collect_i64_from_text(line);
                if ids.len() >= 2 {
                    lines.push(ids);
                }
            }
        }
    }
    Ok(lines)
}

fn collect_i64_from_text(text: &str) -> Vec<i64> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || (ch == '-' && buf.is_empty()) {
            buf.push(ch);
        } else if !buf.is_empty() {
            if let Ok(value) = buf.parse::<i64>() {
                out.push(value);
            }
            buf.clear();
        }
    }
    if !buf.is_empty() {
        if let Ok(value) = buf.parse::<i64>() {
            out.push(value);
        }
    }
    out
}

fn supply_network_railway_op(
    raw: &str,
    index: Option<&GameIndex>,
    existing_railways: &[Vec<i64>],
    has_network_file: bool,
) -> SupplyNetworkOperation {
    let values = collect_i64_from_text(raw);
    let endpoints = values.iter().take(2).copied().collect::<Vec<_>>();
    let level = values.get(2).copied().or(Some(1));
    let endpoints_ok = endpoints.len() == 2;
    let level_ok = level.is_some_and(|level| level > 0);
    let indexed = endpoints
        .iter()
        .all(|id| index.is_some_and(|index| index.province_ids.contains(id)));
    let duplicate = endpoints_ok
        && existing_railways
            .iter()
            .any(|line| line.contains(&endpoints[0]) && line.contains(&endpoints[1]));
    let ok = has_network_file && endpoints_ok && level_ok && indexed && !duplicate;
    SupplyNetworkOperation {
        kind: "railway".to_string(),
        province_ids: endpoints,
        level,
        source_file: "map/railways.txt".to_string(),
        risk: "medium".to_string(),
        ok,
        blocker: (!ok).then(|| {
            if !has_network_file {
                "railway operation requires indexed map/railways.txt".to_string()
            } else if !endpoints_ok {
                format!("railway `{raw}` must contain two province endpoints")
            } else if !indexed {
                format!(
                    "railway `{raw}` contains province endpoints not indexed in map/definition.csv"
                )
            } else if duplicate {
                format!("railway `{raw}` duplicates an existing railway connection")
            } else {
                format!("railway `{raw}` level must be positive")
            }
        }),
    }
}

fn supply_network_supply_node_op(
    raw: &str,
    index: Option<&GameIndex>,
    existing_supply_nodes: &BTreeSet<i64>,
    has_network_file: bool,
) -> SupplyNetworkOperation {
    let province = parse_int(raw.trim());
    let indexed =
        province.is_some_and(|id| index.is_some_and(|index| index.province_ids.contains(&id)));
    let duplicate = province.is_some_and(|id| existing_supply_nodes.contains(&id));
    let ok = has_network_file && province.is_some() && indexed && !duplicate;
    SupplyNetworkOperation {
        kind: "supply_node".to_string(),
        province_ids: province.into_iter().collect(),
        level: None,
        source_file: "map/supply_nodes.txt".to_string(),
        risk: "medium".to_string(),
        ok,
        blocker: (!ok).then(|| {
            if !has_network_file {
                "supply node operation requires indexed map/supply_nodes.txt".to_string()
            } else if province.is_none() {
                format!("supply node province `{raw}` is not an integer")
            } else if !indexed {
                format!("supply node province `{raw}` is not indexed")
            } else {
                format!("supply node province `{raw}` already exists")
            }
        }),
    }
}

fn supply_network_state_province_building_op(
    building: &str,
    raw: &str,
    index: Option<&GameIndex>,
    state_id: Option<i64>,
    state: Option<&HistoryStateStyle>,
) -> SupplyNetworkOperation {
    let province = parse_int(raw.trim());
    let indexed =
        province.is_some_and(|id| index.is_some_and(|index| index.province_ids.contains(&id)));
    let building_indexed = index.is_some_and(|index| index.buildings.contains(building));
    let in_state = match (province, state) {
        (Some(id), Some(state)) => state.province_sample.contains(&id),
        _ => false,
    };
    let ok = province.is_some() && indexed && building_indexed && state_id.is_some() && in_state;
    SupplyNetworkOperation {
        kind: building.to_string(),
        province_ids: province.into_iter().collect(),
        level: Some(1),
        source_file: state_id
            .map(|id| format!("history/states/{id}"))
            .unwrap_or_else(|| "history/states/<missing-state-id>".to_string()),
        risk: "low".to_string(),
        ok,
        blocker: (!ok).then(|| {
            if province.is_none() {
                format!("{building} province `{raw}` is not an integer")
            } else if !indexed {
                format!("{building} province `{raw}` is not indexed")
            } else if !building_indexed {
                format!("building `{building}` is not indexed")
            } else if state_id.is_none() {
                format!("{building} province `{raw}` requires --state-id")
            } else {
                format!(
                    "{building} province `{raw}` is not in state `{}`",
                    state_id.unwrap_or_default()
                )
            }
        }),
    }
}

fn supply_network_state_level_building_op(
    building: &str,
    raw: &str,
    index: Option<&GameIndex>,
    state_id: Option<i64>,
) -> SupplyNetworkOperation {
    let level = parse_int(raw.trim());
    let building_indexed = index.is_some_and(|index| index.buildings.contains(building));
    let ok = state_id.is_some() && level.is_some_and(|level| level >= 0) && building_indexed;
    SupplyNetworkOperation {
        kind: building.to_string(),
        province_ids: Vec::new(),
        level,
        source_file: state_id
            .map(|id| format!("history/states/{id}"))
            .unwrap_or_else(|| "history/states/<missing-state-id>".to_string()),
        risk: "low".to_string(),
        ok,
        blocker: (!ok).then(|| {
            if state_id.is_none() {
                format!("building `{building}` requires --state-id")
            } else if !building_indexed {
                format!("building `{building}` is not indexed")
            } else {
                format!("building `{building}` level `{raw}` must be a non-negative integer")
            }
        }),
    }
}

fn supply_network_changed_files(operations: &[SupplyNetworkOperation]) -> Vec<String> {
    operations
        .iter()
        .map(|op| op.source_file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn supply_network_operation_json(op: &SupplyNetworkOperation) -> String {
    format!(
        "    {{\"kind\": {}, \"province_ids\": {}, \"level\": {}, \"source_file\": {}, \"risk\": {}, \"ok\": {}, \"blocker\": {}}}",
        json_str(&op.kind),
        json_i64_array(&op.province_ids),
        json_optional_i64(op.level),
        json_str(&op.source_file),
        json_str(&op.risk),
        json_bool(op.ok),
        json_optional_str(op.blocker.as_deref())
    )
}

struct SupplyNetworkDirectOperation {
    kind: String,
    province_ids: Vec<i64>,
    level: Option<i64>,
    source_file: String,
    ok: bool,
}

fn supply_network_write_target_overrides(
    plan: &str,
    mod_root: &Path,
    output_dir: &Path,
) -> Result<(Vec<String>, Vec<String>), String> {
    let operations = supply_network_direct_operations(plan);
    if operations.is_empty() {
        return Err("supply network plan has no writable operations".to_string());
    }
    if operations.iter().any(|op| !op.ok) {
        return Err("supply network plan contains non-ok operations".to_string());
    }
    if let Some(op) = operations
        .iter()
        .find(|op| !matches!(op.kind.as_str(), "railway" | "supply_node"))
    {
        return Err(format!(
            "direct supply-network writer does not yet support `{}`; use state-transaction for state buildings",
            op.kind
        ));
    }
    let backup_dir = output_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| format!("create {}: {e}", backup_dir.display()))?;
    let mut by_file: BTreeMap<String, Vec<&SupplyNetworkDirectOperation>> = BTreeMap::new();
    for op in &operations {
        by_file.entry(op.source_file.clone()).or_default().push(op);
    }
    let mut written = Vec::new();
    let mut backups = Vec::new();
    for (source_file, ops) in by_file {
        let target = mod_root.join(source_file.replace('/', "\\"));
        if !target.exists() {
            return Err(format!(
                "target network file `{}` does not exist; direct writer will not copy parent/game map files",
                target.display()
            ));
        }
        let original = read_utf8_lossy(&target)?;
        let mut edited = original.clone();
        if !edited.ends_with('\n') {
            edited.push('\n');
        }
        for op in ops {
            match op.kind.as_str() {
                "railway" => {
                    if op.province_ids.len() < 2 {
                        return Err("railway operation missing two endpoints".to_string());
                    }
                    edited.push_str(&format!(
                        "{} {} {}\n",
                        op.level.unwrap_or(1),
                        op.province_ids[0],
                        op.province_ids[1]
                    ));
                }
                "supply_node" => {
                    let Some(province) = op.province_ids.first() else {
                        return Err("supply_node operation missing province".to_string());
                    };
                    edited.push_str(&format!("{province}\n"));
                }
                _ => unreachable!("filtered above"),
            }
        }
        let backup_name = source_file
            .chars()
            .map(|ch| if ch == '/' || ch == '\\' { '_' } else { ch })
            .collect::<String>();
        let backup = backup_dir.join(format!("{backup_name}.bak"));
        fs::write(&backup, original).map_err(|e| format!("write {}: {e}", backup.display()))?;
        fs::write(&target, edited).map_err(|e| format!("write {}: {e}", target.display()))?;
        written.push(source_file);
        backups.push(backup.display().to_string());
    }
    Ok((written, backups))
}

fn supply_network_direct_operations(plan: &str) -> Vec<SupplyNetworkDirectOperation> {
    plan.lines()
        .filter(|line| line.contains("\"kind\":"))
        .filter_map(|line| {
            Some(SupplyNetworkDirectOperation {
                kind: json_field_in_fragment(line, "kind")?,
                province_ids: json_i64_array_in_fragment(line, "province_ids"),
                level: json_i64_field_in_fragment(line, "level"),
                source_file: json_field_in_fragment(line, "source_file")?,
                ok: line.contains("\"ok\": true"),
            })
        })
        .collect()
}

fn json_i64_array_in_fragment(text: &str, field: &str) -> Vec<i64> {
    let pattern = format!("\"{field}\": [");
    let Some(start) = text.find(&pattern) else {
        return Vec::new();
    };
    let rest = &text[start + pattern.len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    collect_i64_from_text(&rest[..end])
}

fn json_i64_field_in_fragment(text: &str, field: &str) -> Option<i64> {
    let pattern = format!("\"{field}\":");
    let start = text.find(&pattern)? + pattern.len();
    let rest = text[start..].trim_start();
    if rest.starts_with("null") {
        return None;
    }
    collect_i64_from_text(rest).first().copied()
}

fn province_query_json(
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    state_ids: &[i64],
    explicit_provinces: &[i64],
    building_filters: &[String],
    vp_only: bool,
    text: &str,
) -> Result<String, String> {
    let target_states = mod_root
        .map(scan_history_state_styles)
        .transpose()?
        .unwrap_or_default();
    let parent_states = scan_dependency_history_states(dependency_roots)?;
    let game_states = game_root
        .map(scan_history_state_styles)
        .transpose()?
        .unwrap_or_default();
    let game_index = game_root
        .map(|root| build_game_index_with_mod_paths(root, dependency_roots))
        .transpose()?;
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    let mut warnings = Vec::new();
    if map_text_has_place_hint(text) && state_ids.is_empty() && explicit_provinces.is_empty() {
        blockers.push("place/name query requires explicit --state-id or --province-id; do not infer from localisation".to_string());
        questions
            .push("Which exact state id or province id should this place name use?".to_string());
    }
    if state_ids.len() > 1 {
        blockers.push(
            "multiple state ids were supplied; choose one state for a reusable province set"
                .to_string(),
        );
        questions.push("Run province-query once per state id, then pass the chosen province_set_id to OOB/VP/railway/supply planning.".to_string());
    }
    let evidence = state_ids.first().and_then(|state_id| {
        province_query_evidence(
            *state_id,
            mod_root,
            &target_states,
            &parent_states,
            game_root,
            &game_states,
        )
    });
    if !state_ids.is_empty() && evidence.is_none() {
        blockers.push(format!(
            "state id `{}` was not found in target, parent, or game history/states",
            state_ids[0]
        ));
    }
    let mut province_ids = explicit_provinces.to_vec();
    let mut source_layer = if explicit_provinces.is_empty() {
        "unresolved".to_string()
    } else {
        "explicit".to_string()
    };
    let mut source_file = if explicit_provinces.is_empty() {
        "unresolved".to_string()
    } else {
        "user_input".to_string()
    };
    let mut risk = if explicit_provinces.is_empty() {
        "blocked".to_string()
    } else {
        "explicit_user_input".to_string()
    };
    let state_id = evidence.as_ref().and_then(|evidence| evidence.state.id);
    let state_province_count = evidence
        .as_ref()
        .map(|evidence| evidence.state.province_count)
        .unwrap_or(0);
    if let Some(evidence) = &evidence {
        source_layer = evidence.layer.clone();
        source_file = evidence.state.file.clone();
        risk = evidence.risk.clone();
        province_ids = province_query_state_provinces(&evidence.root, &evidence.state)?
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if province_ids.len() < evidence.state.province_count {
            warnings.push("scanner returned a truncated province sample; exact writers must reread the source state file before apply".to_string());
        }
        for building in building_filters {
            if !evidence.state.buildings.iter().any(|item| item == building) {
                blockers.push(format!(
                    "building `{building}` was not observed in state `{}`",
                    evidence.state.id.unwrap_or_default()
                ));
            } else {
                warnings.push(format!(
                    "building filter `{building}` is state-scoped evidence; province-level placement still needs an explicit province id when the writer changes map objects"
                ));
            }
        }
        if vp_only {
            province_ids = evidence
                .state
                .victory_point_provinces
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
        }
    } else if vp_only || !building_filters.is_empty() {
        blockers.push("state-scoped filters require --state-id".to_string());
    }
    if let Some(index) = &game_index {
        for province in &province_ids {
            if !index.province_ids.contains(province) {
                blockers.push(format!("province id `{province}` is not indexed"));
            }
        }
    } else if game_root.is_none() {
        questions.push(
            "Provide --game-root so province ids can be checked against map/definition.csv."
                .to_string(),
        );
    }
    province_ids.sort_unstable();
    province_ids.dedup();
    if province_ids.is_empty() && blockers.is_empty() {
        blockers.push("province query returned no provinces".to_string());
    }
    let ok = blockers.is_empty();
    let province_set_id = match state_id {
        Some(id) if vp_only => format!("state_{id}_victory_points"),
        Some(id) => format!("state_{id}_provinces"),
        None => "explicit_province_set".to_string(),
    };
    let filters = province_query_filter_json(vp_only, building_filters);
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.province_query.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"province_set_id\": {},\n  \"state_id\": {},\n  \"state_province_count\": {},\n  \"province_count\": {},\n  \"province_ids\": {},\n  \"source_layer\": {},\n  \"source_file\": {},\n  \"risk\": {},\n  \"filters\": {},\n  \"warnings\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"consumer_commands\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "province_query_ready" } else { "blocked" }),
        json_str(&province_set_id),
        json_optional_i64(state_id),
        state_province_count,
        province_ids.len(),
        json_i64_array(&province_ids),
        json_str(&source_layer),
        json_str(&source_file),
        json_str(&risk),
        filters,
        json_array(&warnings),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&[
            "hoi4skill oob-relocation-plan --target-state-id <state> --province-set <province_set_id>".to_string(),
            "hoi4skill state-transaction-plan --state <state> --victory-point <province>=<value>".to_string(),
            "hoi4skill map-network-plan --province-set <province_set_id>".to_string(),
        ]),
        json_array(&[
            "place names and localisation keys are not province ids; unresolved names must ask the user for a state/province id".to_string(),
            "VP-only province sets must come from history/states victory_points evidence".to_string(),
            "state-scoped buildings do not prove province-level object placement".to_string(),
            "later OOB, VP, railway, and supply writers must consume this province set instead of inventing ids".to_string(),
        ])
    ))
}

fn province_query_evidence(
    state_id: i64,
    mod_root: Option<&Path>,
    target_states: &[HistoryStateStyle],
    parent_states: &[DependencyHistoryState],
    game_root: Option<&Path>,
    game_states: &[HistoryStateStyle],
) -> Option<ProvinceQueryEvidence> {
    if let (Some(root), Some(state)) = (mod_root, find_state_by_id(target_states, state_id)) {
        return Some(ProvinceQueryEvidence {
            state: state.clone(),
            layer: "target".to_string(),
            root: root.to_path_buf(),
            risk: "low".to_string(),
        });
    }
    if let Some(state) = find_dependency_state_by_id(parent_states, state_id) {
        return Some(ProvinceQueryEvidence {
            state: state.state,
            layer: "parent".to_string(),
            root: PathBuf::from(state.root),
            risk: "override_requires_confirmation".to_string(),
        });
    }
    if let (Some(root), Some(state)) = (game_root, find_state_by_id(game_states, state_id)) {
        return Some(ProvinceQueryEvidence {
            state: state.clone(),
            layer: "game".to_string(),
            root: root.to_path_buf(),
            risk: "override_requires_confirmation".to_string(),
        });
    }
    None
}

fn province_query_state_provinces(
    root: &Path,
    state: &HistoryStateStyle,
) -> Result<Vec<i64>, String> {
    let Some(state_id) = state.id else {
        return Ok(state.province_sample.clone());
    };
    let path = root.join(&state.file);
    let text = strip_comments(&read_utf8_lossy(&path)?);
    for block in direct_blocks_named(&text, "state") {
        if block_assignment(&block, "id").and_then(|value| value.parse::<i64>().ok())
            == Some(state_id)
        {
            let provinces = state_province_ids(&block);
            if !provinces.is_empty() {
                return Ok(provinces);
            }
        }
    }
    Ok(state.province_sample.clone())
}

fn province_query_filter_json(vp_only: bool, buildings: &[String]) -> String {
    format!(
        "{{\"victory_points_only\": {}, \"buildings\": {}}}",
        json_bool(vp_only),
        json_array(buildings)
    )
}

fn state_transaction_tag_op(
    state_id: i64,
    field: &str,
    old: Option<String>,
    new_value: &str,
    source_layer: &str,
    source_file: &str,
    risk: &str,
    index: Option<&GameIndex>,
) -> StateTransactionOperation {
    let ok = index.is_none_or(|index| index.country_tags.contains(new_value));
    StateTransactionOperation {
        state_id,
        field: field.to_string(),
        old_value: old.unwrap_or_else(|| "absent".to_string()),
        new_value: new_value.to_string(),
        source_layer: source_layer.to_string(),
        source_file: source_file.to_string(),
        risk: risk.to_string(),
        ok,
        blocker: (!ok).then(|| format!("country tag `{new_value}` is not indexed")),
    }
}

fn state_transaction_resource_op(
    state_id: i64,
    raw: &str,
    source_layer: &str,
    source_file: &str,
    risk: &str,
    index: Option<&GameIndex>,
    old_state: Option<&HistoryStateStyle>,
) -> StateTransactionOperation {
    let (name, amount) = raw.split_once('=').unwrap_or((raw, ""));
    let name = name.trim();
    let amount = amount.trim();
    let resource_ok = index.is_none_or(|index| index.resources.contains(name));
    let amount_ok = parse_int(amount).is_some();
    let ok = resource_ok && amount_ok;
    StateTransactionOperation {
        state_id,
        field: format!("resource:{name}"),
        old_value: old_state
            .map(|state| {
                if state.resources.iter().any(|resource| resource == name) {
                    "present"
                } else {
                    "absent"
                }
            })
            .unwrap_or("unknown")
            .to_string(),
        new_value: amount.to_string(),
        source_layer: source_layer.to_string(),
        source_file: source_file.to_string(),
        risk: risk.to_string(),
        ok,
        blocker: (!ok).then(|| {
            if !resource_ok {
                format!("resource `{name}` is not indexed")
            } else {
                format!("resource amount `{amount}` is not an integer")
            }
        }),
    }
}

fn state_transaction_vp_op(
    state_id: i64,
    raw: &str,
    source_layer: &str,
    source_file: &str,
    risk: &str,
    index: Option<&GameIndex>,
    old_state: Option<&HistoryStateStyle>,
) -> StateTransactionOperation {
    let (province, value) = raw.split_once('=').unwrap_or((raw, ""));
    let province_id = parse_int(province.trim());
    let value_ok = parse_int(value.trim()).is_some();
    let province_indexed =
        province_id.is_some_and(|id| index.is_none_or(|index| index.province_ids.contains(&id)));
    let province_in_state = province_id.is_some_and(|id| {
        old_state
            .map(|state| state.province_sample.contains(&id))
            .unwrap_or(true)
    });
    let ok = value_ok && province_indexed && province_in_state;
    StateTransactionOperation {
        state_id,
        field: "victory_points".to_string(),
        old_value: province_id
            .map(|id| {
                old_state
                    .map(|state| {
                        if state.victory_point_provinces.contains(&id) {
                            "present"
                        } else {
                            "absent"
                        }
                    })
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|| "invalid_province".to_string()),
        new_value: raw.to_string(),
        source_layer: source_layer.to_string(),
        source_file: source_file.to_string(),
        risk: risk.to_string(),
        ok,
        blocker: (!ok).then(|| {
            if province_id.is_none() {
                format!("victory point province `{province}` is not an integer")
            } else if !province_indexed {
                format!("victory point province `{province}` is not indexed")
            } else if !province_in_state {
                format!("victory point province `{province}` is not in state `{state_id}`")
            } else {
                format!("victory point value `{value}` is not an integer")
            }
        }),
    }
}

fn state_transaction_building_op(
    state_id: i64,
    raw: &str,
    source_layer: &str,
    source_file: &str,
    risk: &str,
    index: Option<&GameIndex>,
) -> StateTransactionOperation {
    let (building, level) = raw.split_once('=').unwrap_or((raw, ""));
    let building = building.trim();
    let level_value = parse_int(level.trim());
    let building_ok = index.is_none_or(|index| index.buildings.contains(building));
    let level_ok = level_value.is_some_and(|level| level >= 0);
    let max_ok = match (index, level_value) {
        (Some(index), Some(level)) => index
            .building_max_levels
            .get(building)
            .is_none_or(|max| level <= i64::from(*max)),
        _ => true,
    };
    let ok = building_ok && level_ok && max_ok;
    StateTransactionOperation {
        state_id,
        field: format!("building:{building}"),
        old_value: "unknown".to_string(),
        new_value: level.trim().to_string(),
        source_layer: source_layer.to_string(),
        source_file: source_file.to_string(),
        risk: risk.to_string(),
        ok,
        blocker: (!ok).then(|| {
            if !building_ok {
                format!("building `{building}` is not indexed")
            } else if !level_ok {
                format!(
                    "building level `{}` is not a non-negative integer",
                    level.trim()
                )
            } else {
                format!(
                    "building level `{}` exceeds indexed max level",
                    level.trim()
                )
            }
        }),
    }
}

fn state_transaction_blockers(operations: &[StateTransactionOperation]) -> Vec<String> {
    operations
        .iter()
        .filter_map(|operation| operation.blocker.clone())
        .collect()
}

fn state_transaction_plan_json(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    indexed_validation: bool,
    operations: &[StateTransactionOperation],
    blockers: &[String],
    ok: bool,
) -> String {
    let changed_files = operations
        .iter()
        .map(|op| op.source_file.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    format!(
        "{{\n  \"schema\": \"hoi4skill.state_transaction_plan.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"dependency_roots\": {},\n  \"indexed_validation\": {},\n  \"operation_count\": {},\n  \"changed_files\": {},\n  \"operations\": [\n{}\n  ],\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_bool(ok),
        json_str(if ok { "state_transaction_plan_ready" } else { "blocked" }),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.map(|root| root.display().to_string()).as_deref()),
        json_array(
            &dependency_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
        ),
        json_bool(indexed_validation),
        operations.len(),
        json_array(&changed_files),
        operations
            .iter()
            .map(state_transaction_operation_json)
            .collect::<Vec<_>>()
            .join(",\n"),
        blockers.len(),
        json_array(blockers),
        json_array(&["hoi4skill state-transaction-apply --input <plan> --execute --final-check --require-passed".to_string()]),
        json_array(&[
            "state id must be indexed or observed in target, parent, or game state files".to_string(),
            "victory point province must belong to the target state when state evidence is available".to_string(),
            "resources, buildings, country tags, and province ids must come from indexed evidence when --game-root is supplied".to_string(),
            "parent/game state edits are override risks and must stay review-pack scoped until release gates pass".to_string(),
        ])
    )
}

fn state_transaction_operation_json(op: &StateTransactionOperation) -> String {
    format!(
        "    {{\"state_id\": {}, \"field\": {}, \"old_value\": {}, \"new_value\": {}, \"source_layer\": {}, \"source_file\": {}, \"risk\": {}, \"ok\": {}, \"blocker\": {}}}",
        op.state_id,
        json_str(&op.field),
        json_str(&op.old_value),
        json_str(&op.new_value),
        json_str(&op.source_layer),
        json_str(&op.source_file),
        json_str(&op.risk),
        json_bool(op.ok),
        json_optional_str(op.blocker.as_deref())
    )
}

fn state_resource_text_plan_json(plan: &StateResourceTextPlan) -> String {
    format!(
        "[{}]",
        plan.requests
            .iter()
            .map(|request| {
                format!(
                    "{{\"state_id\": {}, \"state_query\": {}, \"state_name_key\": {}, \"state_localised_name\": {}, \"resource_id\": {}, \"resource_query\": {}, \"amount\": {}, \"raw_segment\": {}}}",
                    request.state_id,
                    json_str(&request.state_query),
                    json_str(&request.state_name_key),
                    json_optional_str(request.state_localised_name.as_deref()),
                    json_str(&request.resource_id),
                    json_str(&request.resource_query),
                    request.amount,
                    json_str(&request.raw_segment)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn supply_route_text_plan_json(plan: &SupplyRouteTextPlan) -> String {
    format!(
        "[{{\"endpoint_count\": {}, \"requested_fortification\": {}, \"endpoints\": {}, \"suggested_supply_operations\": {}, \"blockers\": {}, \"questions\": {}}}]",
        plan.endpoints.len(),
        json_bool(plan.requested_fortification),
        format_args!(
            "[{}]",
            plan.endpoints
                .iter()
                .map(|endpoint| {
                    format!(
                        "{{\"state_id\": {}, \"state_query\": {}, \"state_name_key\": {}, \"state_localised_name\": {}, \"province_id\": {}, \"province_localised_name\": {}, \"victory_point_value\": {}, \"source_layer\": {}, \"source_file\": {}}}",
                        endpoint.state_id,
                        json_str(&endpoint.state_query),
                        json_str(&endpoint.state_name_key),
                        json_optional_str(endpoint.state_localised_name.as_deref()),
                        endpoint.province_id,
                        json_optional_str(endpoint.province_localised_name.as_deref()),
                        endpoint.victory_point_value,
                        json_str(&endpoint.source_layer),
                        json_str(&endpoint.source_file)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        supply_route_suggested_operations_json(plan),
        json_array(&plan.blockers),
        json_array(&plan.questions)
    )
}

fn supply_route_suggested_operations_json(plan: &SupplyRouteTextPlan) -> String {
    if plan.endpoints.len() != 2 {
        return "[]".to_string();
    }
    let left = plan.endpoints[0].province_id;
    let right = plan.endpoints[1].province_id;
    json_array(&[
        format!("railway_endpoint_pair={left}-{right}=1"),
        format!("supply-node={left}"),
        format!("supply-node={right}"),
        "fortification=blocked_until_user_provides_explicit_province_or_state_ids".to_string(),
    ])
}

fn state_transaction_plan_changed_files(plan: &str) -> Vec<String> {
    json_string_array_field_simple(plan, "changed_files")
}

struct StateTransactionDirectOperation {
    field: String,
    new_value: String,
    source_layer: String,
    source_file: String,
    ok: bool,
}

fn state_transaction_write_target_overrides(
    plan: &str,
    mod_root: &Path,
    output_dir: &Path,
) -> Result<(Vec<String>, Vec<String>), String> {
    let operations = state_transaction_direct_operations(plan);
    if operations.is_empty() {
        return Err("state transaction has no writable operations".to_string());
    }
    if operations.iter().any(|op| !op.ok) {
        return Err("state transaction contains non-ok operations".to_string());
    }
    if let Some(op) = operations.iter().find(|op| op.source_layer != "target") {
        return Err(format!(
            "direct state override writing only supports target source files; `{}` came from `{}`",
            op.source_file, op.source_layer
        ));
    }
    if let Some(op) = operations
        .iter()
        .find(|op| !state_transaction_direct_field_supported(&op.field))
    {
        return Err(format!(
            "direct state override writer does not yet support field `{}`",
            op.field
        ));
    }
    let backup_dir = output_dir.join("backups");
    fs::create_dir_all(&backup_dir).map_err(|e| format!("create {}: {e}", backup_dir.display()))?;
    let mut by_file: BTreeMap<String, Vec<&StateTransactionDirectOperation>> = BTreeMap::new();
    for op in &operations {
        by_file.entry(op.source_file.clone()).or_default().push(op);
    }
    let mut written = Vec::new();
    let mut backups = Vec::new();
    for (source_file, ops) in by_file {
        let target = mod_root.join(source_file.replace('/', "\\"));
        if !target.exists() {
            return Err(format!(
                "target state file `{}` does not exist; parent/game override copy is not implemented in P52 direct writer",
                target.display()
            ));
        }
        let original = read_utf8_lossy(&target)?;
        let mut edited = original.clone();
        for op in ops {
            edited = replace_clausewitz_assignment(&edited, &op.field, &op.new_value).ok_or_else(
                || {
                    format!(
                        "field `{}` was not found in `{}`; direct writer refuses blind insertion",
                        op.field,
                        target.display()
                    )
                },
            )?;
        }
        let backup_name = source_file
            .chars()
            .map(|ch| if ch == '/' || ch == '\\' { '_' } else { ch })
            .collect::<String>();
        let backup = backup_dir.join(format!("{backup_name}.bak"));
        fs::write(&backup, original).map_err(|e| format!("write {}: {e}", backup.display()))?;
        fs::write(&target, edited).map_err(|e| format!("write {}: {e}", target.display()))?;
        written.push(source_file);
        backups.push(backup.display().to_string());
    }
    Ok((written, backups))
}

fn state_transaction_direct_operations(plan: &str) -> Vec<StateTransactionDirectOperation> {
    plan.lines()
        .filter(|line| line.contains("\"state_id\":"))
        .filter_map(|line| {
            Some(StateTransactionDirectOperation {
                field: json_field_in_fragment(line, "field")?,
                new_value: json_field_in_fragment(line, "new_value")?,
                source_layer: json_field_in_fragment(line, "source_layer")?,
                source_file: json_field_in_fragment(line, "source_file")?,
                ok: line.contains("\"ok\": true"),
            })
        })
        .collect()
}

fn state_transaction_direct_field_supported(field: &str) -> bool {
    matches!(
        field,
        "owner" | "controller" | "manpower" | "state_category"
    )
}

fn replace_clausewitz_assignment(text: &str, key: &str, new_value: &str) -> Option<String> {
    let key_pos = text.find(key)?;
    let mut eq_pos = key_pos + key.len();
    while text
        .as_bytes()
        .get(eq_pos)
        .is_some_and(u8::is_ascii_whitespace)
    {
        eq_pos += 1;
    }
    if text.as_bytes().get(eq_pos) != Some(&b'=') {
        return None;
    }
    let mut value_start = eq_pos + 1;
    while text
        .as_bytes()
        .get(value_start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        value_start += 1;
    }
    let mut value_end = value_start;
    while text.as_bytes().get(value_end).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'"')
    }) {
        value_end += 1;
    }
    let mut out = String::new();
    out.push_str(&text[..value_start]);
    out.push_str(new_value);
    out.push_str(&text[value_end..]);
    Some(out)
}

fn json_field_in_fragment(text: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{field}\"");
    let pos = text.find(&pattern)?;
    let after = &text[pos + pattern.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn json_string_array_field_simple(text: &str, key: &str) -> Vec<String> {
    let marker = format!("\"{key}\": [");
    let Some(start) = text.find(&marker) else {
        return Vec::new();
    };
    let rest = &text[start + marker.len()..];
    let Some(end) = rest.find(']') else {
        return Vec::new();
    };
    rest[..end]
        .split(',')
        .filter_map(|raw| {
            let trimmed = raw.trim().trim_matches('"');
            (!trimmed.is_empty()).then(|| trimmed.replace("\\\"", "\"").replace("\\\\", "\\"))
        })
        .collect()
}

fn state_transaction_rollback_markdown(input: &Path, changed_files: &[String]) -> String {
    let mut out = String::new();
    out.push_str("# State Transaction Rollback Plan\n\n");
    out.push_str(&format!("- input: `{}`\n", input.display()));
    out.push_str("- action: review changed files, restore from VCS or backup before release if validation/runtime gates fail.\n\n");
    out.push_str("## Changed Files\n\n");
    for file in changed_files {
        out.push_str(&format!("- `{file}`\n"));
    }
    out
}

fn map_intent_plan_json(
    text: &str,
    mod_root: Option<&Path>,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    index: Option<&GameIndex>,
) -> Result<String, String> {
    let lanes = classify_map_lanes(text);
    let state_id = first_labeled_number(text, "state");
    let province_id = first_labeled_number(text, "province");
    let state_resource_plan = if lanes.iter().any(|lane| lane.lane == "state_resources") {
        index.map(|index| compile_state_resource_text_plan(text, index))
    } else {
        None
    };
    let supply_route_plan = if lanes.iter().any(|lane| lane.lane == "supply_network")
        && map_text_has_route_hint(text)
    {
        match index {
            Some(index) => Some(compile_supply_route_text_plan(
                text,
                mod_root,
                game_root,
                dependency_roots,
                index,
            )?),
            None => None,
        }
    } else {
        None
    };
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    let compiled_state_resource_ready = state_resource_plan
        .as_ref()
        .is_some_and(|plan| !plan.requests.is_empty() && plan.blockers.is_empty());
    let compiled_supply_route_started = supply_route_plan
        .as_ref()
        .is_some_and(|plan| !plan.endpoints.is_empty() || !plan.blockers.is_empty());
    if map_text_has_place_hint(text)
        && state_id.is_none()
        && province_id.is_none()
        && !compiled_state_resource_ready
        && !compiled_supply_route_started
    {
        blockers.push(
            "place name was detected but no explicit state/province id was verified".to_string(),
        );
        questions.push("Provide --state-id or --province-id, or run province-query after indexing game/parent map data.".to_string());
    }
    if let Some(plan) = &state_resource_plan {
        blockers.extend(plan.blockers.iter().cloned());
    }
    if let Some(plan) = &supply_route_plan {
        blockers.extend(plan.blockers.iter().cloned());
        questions.extend(plan.questions.iter().cloned());
    }
    if lanes.iter().any(|lane| lane.risk == "high") {
        blockers.push(
            "topology request detected; route through map-topology-plan, not direct map writes"
                .to_string(),
        );
        questions.push("Confirm bitmap/definition/adjacency evidence and whether this is a review-only topology pack.".to_string());
    }
    if index.is_none() {
        questions.push("Provide --game-root and --mod-path so state, province, resource, building, railway, and supply evidence can be indexed.".to_string());
    }
    let ok = blockers.is_empty();
    let next_commands = if ok {
        vec![
            "hoi4skill state-transaction-plan --mod-root <target> --game-root <HOI4 root> --text <request> --require-passed".to_string(),
            "hoi4skill province-query --state-id <id> --game-root <HOI4 root>".to_string(),
        ]
    } else {
        vec![
            "hoi4skill map-data-audit --mod-root <target> --game-root <HOI4 root> --mod-path <parent>".to_string(),
            "hoi4skill ambiguity-report --text <request> --game-root <HOI4 root> --mod-root <target>".to_string(),
        ]
    };
    Ok(format!(
        "{{\n  \"schema\": \"hoi4skill.map_intent_plan.v1\",\n  \"status\": {},\n  \"ok\": {},\n  \"mod_root\": {},\n  \"game_index_available\": {},\n  \"request_text\": {},\n  \"detected_state_id\": {},\n  \"detected_province_id\": {},\n  \"compiled_state_resources\": {},\n  \"compiled_supply_routes\": {},\n  \"lanes\": [\n{}\n  ],\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str(if ok { "map_intent_ready" } else { "blocked" }),
        json_bool(ok),
        json_optional_str(mod_root.map(|root| root.display().to_string()).as_deref()),
        json_bool(index.is_some()),
        json_str(text),
        json_optional_i64(state_id),
        json_optional_i64(province_id),
        state_resource_plan
            .as_ref()
            .map(state_resource_text_plan_json)
            .unwrap_or_else(|| "[]".to_string()),
        supply_route_plan
            .as_ref()
            .map(supply_route_text_plan_json)
            .unwrap_or_else(|| "[]".to_string()),
        lanes
            .iter()
            .map(map_intent_lane_json)
            .collect::<Vec<_>>()
            .join(",\n"),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&next_commands),
        json_array(&map_intent_rules())
    ))
}

struct MapIntentLane {
    lane: &'static str,
    risk: &'static str,
    reason: &'static str,
}

fn classify_map_lanes(text: &str) -> Vec<MapIntentLane> {
    let mut lanes = Vec::new();
    push_lane_if(
        &mut lanes,
        text,
        &[
            "owner",
            "controller",
            "核心",
            "core",
            "人口",
            "manpower",
            "state_category",
        ],
        "state_history",
        "low",
        "state ownership, cores, manpower, or category requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &["资源", "钢", "铝", "橡胶", "石油", "钨", "铬", "resource"],
        "state_resources",
        "low",
        "state resource change requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &["胜利点", "vp", "victory point", "南昌"],
        "victory_points",
        "low",
        "victory point or named city target requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &[
            "建筑",
            "工厂",
            "机场",
            "港口",
            "基础设施",
            "building",
            "air base",
            "naval base",
            "infrastructure",
        ],
        "state_buildings",
        "low",
        "state building change requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &["部署", "location", "oob", "师", "部队"],
        "oob_location",
        "low",
        "OOB location or province set requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &["铁路", "railway", "补给中心", "supply node", "补给节点"],
        "supply_network",
        "medium",
        "railway or supply node requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &["战略区域", "空区", "strategic region", "weather"],
        "strategic_regions",
        "medium",
        "strategic region or weather position requested",
    );
    push_lane_if(
        &mut lanes,
        text,
        &[
            "新增省份",
            "新省份",
            "岛屿",
            "海峡",
            "adjacency",
            "definition.csv",
            "provinces.bmp",
            "terrain.bmp",
            "rivers.bmp",
        ],
        "topology",
        "high",
        "map topology or bitmap change requested",
    );
    if lanes.is_empty() {
        lanes.push(MapIntentLane {
            lane: "unknown_map_intent",
            risk: "unknown",
            reason: "no supported map-data lane was detected",
        });
    }
    lanes
}

fn push_lane_if(
    lanes: &mut Vec<MapIntentLane>,
    text: &str,
    needles: &[&str],
    lane: &'static str,
    risk: &'static str,
    reason: &'static str,
) {
    let lower = text.to_ascii_lowercase();
    if needles
        .iter()
        .any(|needle| text.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
    {
        lanes.push(MapIntentLane { lane, risk, reason });
    }
}

fn map_intent_lane_json(lane: &MapIntentLane) -> String {
    format!(
        "    {{\"lane\": {}, \"risk\": {}, \"reason\": {}}}",
        json_str(lane.lane),
        json_str(lane.risk),
        json_str(lane.reason)
    )
}

fn map_text_has_place_hint(text: &str) -> bool {
    ["江西", "南昌", "华北", "华南", "东北", "STATE_"]
        .iter()
        .any(|needle| text.contains(needle))
}

fn map_text_has_route_hint(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    (text.contains('从') && text.contains('到'))
        || text.contains("连接")
        || text.contains("链接")
        || lower.contains("from")
        || lower.contains(" to ")
}

fn map_intent_rules() -> Vec<String> {
    vec![
        "Chinese place names are candidates only; they do not authorize state or province IDs.".to_string(),
        "State history, supply network, strategic region, and topology lanes must not consume each other's effects.".to_string(),
        "Railway endpoints may be inferred only from indexed largest victory points; intermediate path provinces and forts require explicit user confirmation.".to_string(),
        "High-risk topology requests cannot be applied by weak AI output.".to_string(),
        "Every state, province, resource, building, railway endpoint, and supply node must come from indexed local evidence or explicit user input.".to_string(),
    ]
}
