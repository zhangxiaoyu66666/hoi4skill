//! Common-directory coverage audit.
//!
//! P16 is deliberately read-only. It compares the actual game `common/*`
//! directory surface against what this CLI can write, plan, index, or only route
//! through, so future common-system work has a machine-readable gate.

#[allow(unused_imports)]
use crate::*;

struct CommonCoverageRow {
    dir: String,
    coverage_level: &'static str,
    evidence: Vec<String>,
    risk: &'static str,
    recommended_action: &'static str,
    game_has: bool,
    target_has: bool,
    parent_has: bool,
    blocking: bool,
}

struct CommonReleaseRow {
    dir: String,
    coverage_level: &'static str,
    validation_command: String,
    remaining_risk: String,
    severity: &'static str,
    blocking: bool,
    repair_entry: String,
}

pub(crate) fn cmd_common_coverage_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;

    let game_dirs = common_subdirs(&game_root)?;
    let target_dirs = optional_common_subdirs(&target_root)?;
    let parent_dirs = parent_common_subdirs(&parent_roots)?;
    let mut all_dirs = game_dirs.clone();
    all_dirs.extend(target_dirs.iter().cloned());
    all_dirs.extend(parent_dirs.iter().cloned());

    let rows = all_dirs
        .iter()
        .map(|dir| {
            let info = common_coverage_info(dir);
            let target_has = target_dirs.contains(dir);
            let coverage_level = info.0;
            CommonCoverageRow {
                dir: dir.clone(),
                coverage_level,
                evidence: info.1,
                risk: info.2,
                recommended_action: info.3,
                game_has: game_dirs.contains(dir),
                target_has,
                parent_has: parent_dirs.contains(dir),
                blocking: target_has && coverage_level == "none",
            }
        })
        .collect::<Vec<_>>();
    let mut blockers = rows
        .iter()
        .filter(|row| row.blocking)
        .map(|row| {
            format!(
                "target mod uses common/{} but CLI coverage_level is none",
                row.dir
            )
        })
        .collect::<Vec<_>>();

    for required in required_non_none_common_dirs().iter().copied() {
        if game_dirs.contains(required) && common_coverage_info(required).0 == "none" {
            blockers.push(format!(
                "required common/{required} must not be coverage_level none"
            ));
        }
    }

    let ok = blockers.is_empty();
    let json = common_coverage_json(
        ok,
        &game_root,
        &target_root,
        &parent_roots,
        &rows,
        &blockers,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_common_release_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;

    let _game_dirs = common_subdirs(&game_root)?;
    let target_dirs = optional_common_subdirs(&target_root)?;
    let parent_dirs = parent_common_subdirs(&parent_roots)?;
    let mut rows = Vec::new();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut cosmetic_risks = Vec::new();

    for dir in &target_dirs {
        let info = common_coverage_info(dir);
        let row = common_release_row(dir, info, parent_dirs.contains(dir));
        match row.severity {
            "blocking" => blockers.push(row.repair_entry.clone()),
            "cosmetic_risk" => cosmetic_risks.push(row.repair_entry.clone()),
            "warning" => warnings.push(row.repair_entry.clone()),
            _ => {}
        }
        rows.push(row);
    }

    let ok = blockers.is_empty();
    let validation = format!(
        "hoi4skill validate {} --game-root {} --strict-code-index --output validation.json",
        target_root.display(),
        game_root.display()
    );
    let coverage = format!(
        "hoi4skill common-coverage-audit --mod-root {} --game-root {}{} --require-passed --output common_coverage.json",
        target_root.display(),
        game_root.display(),
        common_release_parent_args(&parent_roots),
    );
    let runtime = format!(
        "hoi4skill runtime-error-regression --mod-root {} --error-log <after error.log> --baseline baseline.json --require-passed --output runtime_regression.json",
        target_root.display()
    );
    let playable = "hoi4skill playable-acceptance-gate --validation validation.json --error-regression runtime_regression.json --route-guide route_guide.json --require-passed --output playable_gate.json".to_string();
    let repair_checklist = rows
        .iter()
        .filter(|row| row.severity != "ok")
        .map(|row| row.repair_entry.clone())
        .collect::<Vec<_>>();
    let json = common_release_gate_json(
        ok,
        &game_root,
        &target_root,
        &parent_roots,
        &rows,
        &[coverage, validation, runtime, playable],
        &blockers,
        &warnings,
        &cosmetic_risks,
        &repair_checklist,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn common_subdirs(root: &Path) -> Result<BTreeSet<String>, String> {
    let common = root.join("common");
    if !common.is_dir() {
        return Err(format!("{}: common directory is missing", common.display()));
    }
    optional_common_subdirs(root)
}

fn optional_common_subdirs(root: &Path) -> Result<BTreeSet<String>, String> {
    let common = root.join("common");
    let mut dirs = BTreeSet::new();
    if !common.exists() {
        return Ok(dirs);
    }
    if !common.is_dir() {
        return Err(format!(
            "{}: common path is not a directory",
            common.display()
        ));
    }
    for entry in fs::read_dir(&common).map_err(|e| format!("read dir {}: {e}", common.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                dirs.insert(name.to_string());
            }
        }
    }
    Ok(dirs)
}

fn parent_common_subdirs(parent_roots: &[PathBuf]) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for root in parent_roots {
        out.extend(optional_common_subdirs(root)?);
    }
    Ok(out)
}

fn required_non_none_common_dirs() -> &'static [&'static str] {
    &[
        "national_focus",
        "ideas",
        "decisions",
        "scripted_effects",
        "scripted_triggers",
        "scripted_guis",
        "dynamic_modifiers",
        "technologies",
    ]
}

fn common_coverage_info(dir: &str) -> (&'static str, Vec<String>, &'static str, &'static str) {
    match dir {
        "national_focus" => (
            "writer",
            vec![
                "apply-focus-layout".to_string(),
                "apply-focus-excel".to_string(),
                "validate --strict-code-index".to_string(),
            ],
            "covered_high_value_writer",
            "can_generate_with_final_check",
        ),
        "decisions" => (
            "writer",
            vec![
                "apply-feature-cards".to_string(),
                "apply-decision-intent".to_string(),
                "gui-request-workflow".to_string(),
            ],
            "covered_writer_complex_gui_decisions_need_extra_gate",
            "can_generate_or_reuse_category_with_final_check",
        ),
        "ideas" => (
            "writer",
            vec![
                "apply-feature-cards".to_string(),
                "compile-intent".to_string(),
                "validate dynamic modifier misuse".to_string(),
            ],
            "covered_writer_do_not_mix_advisors_or_dynamic_modifiers",
            "can_generate_national_spirits_with_scope_check",
        ),
        "dynamic_modifiers" => (
            "writer",
            vec![
                "plan-dynamic-modifier-change".to_string(),
                "author-intent-workflow".to_string(),
            ],
            "covered_writer_variable_protocol_still_needs_parent_template_checks",
            "can_generate_dynamic_modifier_protocol",
        ),
        "scripted_effects" => (
            "writer",
            vec![
                "apply-feature-cards".to_string(),
                "plan-dynamic-modifier-change".to_string(),
                "strict-code-index".to_string(),
            ],
            "covered_writer_nested_semantics_depend_on_strict_index",
            "can_generate_helpers_with_final_check",
        ),
        "scripted_triggers" => (
            "writer",
            vec![
                "apply-feature-cards".to_string(),
                "condition-plan".to_string(),
            ],
            "covered_writer_complex_trigger_templates_still_need_expansion",
            "can_generate_helpers_with_final_check",
        ),
        "scripted_guis" => (
            "writer",
            vec![
                "apply-gui-intent".to_string(),
                "gui-request-workflow".to_string(),
                "gui-playability-gate".to_string(),
            ],
            "covered_writer_visual_editor_not_done",
            "can_generate_conservative_gui_skeletons",
        ),
        "technologies" => (
            "writer",
            vec![
                "apply-feature-cards".to_string(),
                "tech-equipment-plan".to_string(),
                "tech-scope-audit".to_string(),
            ],
            "covered_minimal_writer_research_tree_layout_half_covered",
            "can_generate_minimal_technology_skeletons",
        ),
        "characters"
        | "military_industrial_organization"
        | "ideologies"
        | "country_tags"
        | "countries"
        | "country_leader"
        | "unit_leader" => (
            "planner",
            vec![
                "intent-plan".to_string(),
                "scope audit".to_string(),
                "strict-code-index references".to_string(),
            ],
            "planner_only_or_style_dependent_writer",
            "plan_and_require_explicit_user_authorization_before_write",
        ),
        "scripted_localisation" | "scripted_localization" => (
            "planner",
            vec![
                "scripted-localisation-plan".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_schema_specific_writer_pending",
            "plan_definition_and_block_duplicate_keys_before_write",
        ),
        "opinion_modifiers" => (
            "planner",
            vec![
                "opinion-modifier-plan".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_simple_writer_pending",
            "plan_definition_and_block_duplicate_keys_before_write",
        ),
        "game_rules" => (
            "planner",
            vec![
                "game-rule-plan".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_options_need_schema_writer",
            "plan_definition_and_require_schema_writer_for_options",
        ),
        "bookmarks" => (
            "planner",
            vec![
                "bookmark-plan".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_bookmark_country_entries_pending",
            "plan_definition_and_require_schema_writer_for_country_entries",
        ),
        "bop" => (
            "planner",
            vec![
                "bop-plan".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_range_schema_writer_pending",
            "plan_definition_and_require_schema_writer_for_ranges",
        ),
        "ai_strategy_plans" => (
            "planner",
            vec![
                "ai-strategy-plan-file".to_string(),
                "common definition duplicate scan".to_string(),
            ],
            "planner_only_strategy_contents_need_indexed_types",
            "plan_definition_and_verify_strategy_types_before_write",
        ),
        "intelligence_agencies"
        | "intelligence_agency_upgrades"
        | "operations"
        | "operation_phases"
        | "operation_tokens" => (
            "planner",
            vec![
                "system-pack-plan --pack intelligence_operations".to_string(),
                "system-pack-apply gate".to_string(),
            ],
            "p18_system_pack_schema_skeleton_requires_fill_and_runtime_check",
            "plan_as_intelligence_operations_package_then_apply_transactional_skeleton",
        ),
        "ai_strategy" | "ai_focuses" | "ai_templates" | "ai_equipment" | "ai_navy" => (
            "planner",
            vec![
                "system-pack-plan --pack ai_behavior".to_string(),
                "ai-behavior-audit".to_string(),
                "system-pack-apply gate".to_string(),
            ],
            "p20_ai_behavior_audit_only_writer_pending",
            "plan_and_audit_as_ai_behavior_package_before_write",
        ),
        "technology_tags" | "technology_sharing" | "equipment_groups" | "special_projects" => (
            "planner",
            vec![
                "system-pack-plan --pack technology_depth".to_string(),
                "system-pack-apply gate".to_string(),
            ],
            "p18_system_pack_schema_skeleton_requires_fill_and_runtime_check",
            "plan_as_technology_depth_package_then_apply_transactional_skeleton",
        ),
        "occupation_laws" | "resistance_activity" | "resistance_compliance_modifiers" => (
            "planner",
            vec![
                "system-pack-plan --pack occupation_resistance".to_string(),
                "system-pack-apply gate".to_string(),
            ],
            "p18_system_pack_schema_skeleton_requires_fill_and_runtime_check",
            "plan_as_occupation_resistance_package_then_apply_transactional_skeleton",
        ),
        "profile_backgrounds"
        | "profile_pictures"
        | "ribbons"
        | "medals"
        | "unit_medals"
        | "map_modes"
        | "focus_inlay_windows"
        | "frontend" => (
            "planner",
            vec![
                "ui-cosmetic-common-plan".to_string(),
                "gui/playability visual runtime smoke".to_string(),
            ],
            "p21_ui_cosmetic_plan_only_release_warning",
            "plan_assets_templates_and_visual_smoke_without_blocking_core_content",
        ),
        "on_actions" => (
            "planner",
            vec![
                "on-action-insert-plan".to_string(),
                "on-action-graph".to_string(),
                "trigger-source-graph".to_string(),
                "dead-event-audit".to_string(),
            ],
            "planner_only_insert_writer_pending",
            "scan_routes_and_block_unproven_trigger_paths_before_write",
        ),
        "buildings" | "resources" | "units" | "wargoals" | "modifier_definitions" | "modifiers" => {
            (
                "index_only",
                vec![
                    "build-game-index".to_string(),
                    "code-catalog".to_string(),
                    "validate --strict-code-index".to_string(),
                ],
                "indexed_but_no_dedicated_writer",
                "validate_references_or_build_dedicated_writer_first",
            )
        }
        _ => (
            "none",
            Vec::new(),
            "no_productized_cli_coverage",
            "template_learning_or_new_phase_required_before_write",
        ),
    }
}

fn common_coverage_json(
    ok: bool,
    game_root: &Path,
    target_root: &Path,
    parent_roots: &[PathBuf],
    rows: &[CommonCoverageRow],
    blockers: &[String],
) -> String {
    let writer_count = rows
        .iter()
        .filter(|row| row.coverage_level == "writer")
        .count();
    let planner_count = rows
        .iter()
        .filter(|row| row.coverage_level == "planner")
        .count();
    let index_only_count = rows
        .iter()
        .filter(|row| row.coverage_level == "index_only")
        .count();
    let route_only_count = rows
        .iter()
        .filter(|row| row.coverage_level == "route_only")
        .count();
    let none_count = rows
        .iter()
        .filter(|row| row.coverage_level == "none")
        .count();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"target_mod_root\": {},\n  \"parent_mod_roots\": {},\n  \"common_dir_count\": {},\n  \"counts\": {{\"writer\": {}, \"planner\": {}, \"index_only\": {}, \"route_only\": {}, \"none\": {}}},\n  \"rows\": [{}],\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.common_coverage_audit.v1"),
        json_bool(ok),
        json_str(if ok { "common_coverage_ready" } else { "common_coverage_blocked" }),
        json_str(&game_root.display().to_string()),
        json_str(&target_root.display().to_string()),
        json_array(
            &parent_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
        ),
        rows.len(),
        writer_count,
        planner_count,
        index_only_count,
        route_only_count,
        none_count,
        rows.iter()
            .map(common_coverage_row_json)
            .collect::<Vec<_>>()
            .join(", "),
        json_array(blockers),
        json_str("P16 audits actual game common directories and fails when the target mod uses a common directory with coverage_level none")
    )
}

fn common_coverage_row_json(row: &CommonCoverageRow) -> String {
    format!(
        "{{\"dir\": {}, \"path\": {}, \"coverage_level\": {}, \"evidence\": {}, \"risk\": {}, \"recommended_action\": {}, \"game_has\": {}, \"target_has\": {}, \"parent_has\": {}, \"blocking\": {}}}",
        json_str(&row.dir),
        json_str(&format!("common/{}", row.dir)),
        json_str(row.coverage_level),
        json_array(&row.evidence),
        json_str(row.risk),
        json_str(row.recommended_action),
        json_bool(row.game_has),
        json_bool(row.target_has),
        json_bool(row.parent_has),
        json_bool(row.blocking),
    )
}

fn common_release_row(
    dir: &str,
    info: (&'static str, Vec<String>, &'static str, &'static str),
    parent_has: bool,
) -> CommonReleaseRow {
    let coverage_level = info.0;
    let remaining_risk = common_release_remaining_risk(dir, coverage_level, info.2, parent_has);
    let severity = common_release_severity(dir, coverage_level, info.2);
    let blocking = severity == "blocking";
    let validation_command = common_release_validation_command(dir, coverage_level);
    let repair_entry = format!(
        "common/{dir}: severity={severity}; coverage={coverage_level}; risk={remaining_risk}; retrieve_context=`hoi4skill prepare-edit-context --path common/{dir} --strict-code-index`"
    );
    CommonReleaseRow {
        dir: dir.to_string(),
        coverage_level,
        validation_command,
        remaining_risk,
        severity,
        blocking,
        repair_entry,
    }
}

fn common_release_severity(dir: &str, coverage_level: &str, risk: &str) -> &'static str {
    if coverage_level == "none" && common_release_core_dir(dir) {
        "blocking"
    } else if risk.starts_with("p21_") {
        "cosmetic_risk"
    } else if coverage_level == "writer" {
        "ok"
    } else {
        "warning"
    }
}

fn common_release_core_dir(dir: &str) -> bool {
    required_non_none_common_dirs().contains(&dir)
        || matches!(
            dir,
            "on_actions"
                | "characters"
                | "country_tags"
                | "countries"
                | "ideologies"
                | "military_industrial_organization"
        )
}

fn common_release_remaining_risk(
    dir: &str,
    coverage_level: &str,
    risk: &str,
    parent_has: bool,
) -> String {
    let mut parts = vec![risk.to_string()];
    if coverage_level != "writer" {
        parts.push(format!(
            "coverage_level_{coverage_level}_requires_human_or_ai_plan_gate"
        ));
    }
    if parent_has {
        parts.push("parent_mod_uses_same_common_dir_review_override_risk".to_string());
    }
    if dir == "scripted_guis" {
        parts.push("requires_gui_visual_runtime_smoke".to_string());
    }
    parts.join(";")
}

fn common_release_validation_command(dir: &str, coverage_level: &str) -> String {
    match coverage_level {
        "writer" => format!("hoi4skill validate <mod> --strict-code-index --path common/{dir}"),
        "planner" => format!("hoi4skill common-coverage-audit --mod-root <mod> --game-root <hoi4>; then run the listed planner for common/{dir}"),
        "index_only" => format!("hoi4skill build-game-index --game-root <hoi4>; validate references before writing common/{dir}"),
        "route_only" => format!("hoi4skill route-blocker-audit --mod-root <mod>; inspect common/{dir} trigger paths"),
        _ => format!("rg -n \"\" <game-or-parent>/common/{dir}; add a productized CLI strategy before writing"),
    }
}

fn common_release_parent_args(parent_roots: &[PathBuf]) -> String {
    parent_roots
        .iter()
        .map(|root| format!(" --mod-path {}", root.display()))
        .collect::<Vec<_>>()
        .join("")
}

fn common_release_gate_json(
    ok: bool,
    game_root: &Path,
    target_root: &Path,
    parent_roots: &[PathBuf],
    rows: &[CommonReleaseRow],
    integration_commands: &[String],
    blockers: &[String],
    warnings: &[String],
    cosmetic_risks: &[String],
    repair_checklist: &[String],
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"target_mod_root\": {},\n  \"parent_mod_roots\": {},\n  \"target_common_dir_count\": {},\n  \"used_common_dirs\": {},\n  \"rows\": [{}],\n  \"blocking\": {},\n  \"warning\": {},\n  \"cosmetic_risk\": {},\n  \"integration_commands\": {},\n  \"ai_repair_checklist\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.common_release_gate.v1"),
        json_bool(ok),
        json_str(if ok {
            "common_release_gate_passed"
        } else {
            "common_release_gate_blocked"
        }),
        json_str(&game_root.display().to_string()),
        json_str(&target_root.display().to_string()),
        json_array(
            &parent_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
        ),
        rows.len(),
        json_array(&rows.iter().map(|row| row.dir.clone()).collect::<Vec<_>>()),
        rows.iter()
            .map(common_release_row_json)
            .collect::<Vec<_>>()
            .join(", "),
        json_array(blockers),
        json_array(warnings),
        json_array(cosmetic_risks),
        json_array(integration_commands),
        json_array(repair_checklist),
        json_str("P22 release gate summarizes actual target common directories, separates blocking/warning/cosmetic_risk, and gives AI only verified repair entries plus retrieval commands")
    )
}

fn common_release_row_json(row: &CommonReleaseRow) -> String {
    format!(
        "{{\"dir\": {}, \"path\": {}, \"coverage_level\": {}, \"validation_command\": {}, \"remaining_risk\": {}, \"severity\": {}, \"blocking\": {}, \"repair_entry\": {}}}",
        json_str(&row.dir),
        json_str(&format!("common/{}", row.dir)),
        json_str(row.coverage_level),
        json_str(&row.validation_command),
        json_str(&row.remaining_risk),
        json_str(row.severity),
        json_bool(row.blocking),
        json_str(&row.repair_entry),
    )
}
