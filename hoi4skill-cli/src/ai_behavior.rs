//! P20 AI behavior and balance audit.
//!
//! AI authoring is useful only when its references are real and its route
//! behavior can be compared with the player route. This command is a pre-writer
//! gate for focus/strategy/template/equipment/event-option AI work.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_ai_behavior_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &mod_paths)?;

    let focuses = repeated_owned(&map, "focus");
    let technologies = repeated_owned(&map, "technology");
    let equipment = repeated_owned(&map, "equipment");
    let wargoal_types = repeated_owned(&map, "wargoal-type");
    let target_tags = repeated_owned(&map, "target-tag");
    let player_route = repeated_owned(&map, "player-route");
    let ai_route = repeated_owned(&map, "ai-route");
    let strategy_types = repeated_owned(&map, "strategy-type");
    let target_filter = value(&map, "target-filter");
    let route_generated = map.flags.contains("route-generated") || !player_route.is_empty();

    let mut blockers = Vec::new();
    validate_indexed_values("focus", &focuses, &index.focus_ids, &mut blockers);
    validate_indexed_values(
        "technology",
        &technologies,
        &index.technologies,
        &mut blockers,
    );
    validate_indexed_values(
        "equipment",
        &equipment,
        &index.equipment_types,
        &mut blockers,
    );
    validate_indexed_values(
        "wargoal type",
        &wargoal_types,
        &index.wargoal_types,
        &mut blockers,
    );
    validate_indexed_values(
        "target tag",
        &target_tags,
        &index.country_tags,
        &mut blockers,
    );
    for strategy_type in &strategy_types {
        if !matches!(
            strategy_type.as_str(),
            "declare_war" | "befriend" | "protect" | "conquer" | "antagonize" | "ignore"
        ) {
            blockers.push(format!(
                "AI strategy type `{strategy_type}` is not whitelisted"
            ));
        }
    }
    if route_generated && !target_tags.is_empty() && target_filter.is_none() {
        blockers.push(
            "route-generated AI preferences with target tags need --target-filter; do not hard-code a fixed tag shortcut"
                .to_string(),
        );
    }

    let route_diff = route_diff_rows(&player_route, &ai_route);
    let ai_chance = mod_root
        .as_deref()
        .map(scan_ai_chance)
        .transpose()?
        .unwrap_or_default();
    let ok = blockers.is_empty();
    let json = ai_behavior_audit_json(AiBehaviorReport {
        ok,
        game_root: &game_root,
        mod_root: mod_root.as_deref(),
        focuses: &focuses,
        technologies: &technologies,
        equipment: &equipment,
        wargoal_types: &wargoal_types,
        target_tags: &target_tags,
        target_filter,
        strategy_types: &strategy_types,
        route_diff: &route_diff,
        ai_chance: &ai_chance,
        blockers: &blockers,
    });
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_ai_behavior_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let audit = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !map.flags.contains("execute") {
        blockers.push("ai-behavior-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("ai-behavior-apply requires --final-check".to_string());
    }
    if !audit.contains("\"schema\": \"hoi4skill.ai_behavior_audit.v1\"") {
        blockers.push("input is not an ai-behavior-audit report".to_string());
    }
    if !audit.contains("\"ok\": true") {
        blockers.push("input AI behavior audit is not ok".to_string());
    }
    let mod_root = json_string_field(&audit, "mod_root")
        .filter(|value| !value.is_empty())
        .map(|path| normalize_path(&path))
        .transpose()?;
    if mod_root.is_none() {
        blockers.push("input audit is missing mod_root; apply needs a target mod".to_string());
    }
    let prefix = value(&map, "prefix").unwrap_or("ai_behavior");
    let focuses = json_string_array_field(&audit, "focuses");
    let technologies = json_string_array_field(&audit, "technologies");
    let equipment = json_string_array_field(&audit, "equipment");
    let wargoal_types = json_string_array_field(&audit, "wargoal_types");
    let target_tags = json_string_array_field(&audit, "target_tags");
    let strategy_types = json_string_array_field(&audit, "strategy_types");
    let route_diff = json_string_array_field(&audit, "route_diff");
    if focuses.is_empty()
        && technologies.is_empty()
        && equipment.is_empty()
        && strategy_types.is_empty()
    {
        blockers.push("AI behavior apply needs at least one focus, technology, equipment, or strategy reference".to_string());
    }
    let mut write_plan = Vec::new();
    if let Some(mod_root) = mod_root.as_ref() {
        for (relative, content) in ai_behavior_skeleton_files(AiBehaviorSkeleton {
            prefix,
            focuses: &focuses,
            technologies: &technologies,
            equipment: &equipment,
            wargoal_types: &wargoal_types,
            target_tags: &target_tags,
            strategy_types: &strategy_types,
            route_diff: &route_diff,
        }) {
            let path = mod_root.join(Path::new(&relative));
            if path.exists() {
                blockers.push(format!(
                    "transaction target already exists and will not be overwritten: {}",
                    path.display()
                ));
            }
            write_plan.push((relative, path, content));
        }
    }

    let mut changed_files = Vec::new();
    let mut rollback_blockers = Vec::new();
    if blockers.is_empty() {
        match write_ai_behavior_transaction(&write_plan) {
            Ok(changed) => changed_files = changed,
            Err((err, changed)) => {
                rollback_blockers.push(err);
                rollback_blockers.extend(rollback_ai_behavior_files(&changed));
                blockers
                    .push("AI behavior transaction failed and rollback was attempted".to_string());
                changed_files = changed
                    .into_iter()
                    .map(|path| path.display().to_string())
                    .collect();
            }
        }
    }

    let ok = blockers.is_empty();
    let report = ai_behavior_apply_json(
        &input,
        ok,
        prefix,
        &changed_files,
        &blockers,
        &rollback_blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if (map.flags.contains("require-passed") || !blockers.is_empty()) && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn repeated_owned(map: &ArgMap, key: &str) -> Vec<String> {
    repeated_values(map, key)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn validate_indexed_values(
    label: &str,
    values: &[String],
    index: &BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    for value in values {
        if !index.contains(value) {
            blockers.push(format!("{label} `{value}` is not indexed"));
        }
    }
}

fn route_diff_rows(player_route: &[String], ai_route: &[String]) -> Vec<String> {
    let player = player_route.iter().cloned().collect::<BTreeSet<_>>();
    let ai = ai_route.iter().cloned().collect::<BTreeSet<_>>();
    let mut rows = Vec::new();
    for focus in player.difference(&ai) {
        rows.push(format!("player_only:{focus}"));
    }
    for focus in ai.difference(&player) {
        rows.push(format!("ai_only:{focus}"));
    }
    for focus in player.intersection(&ai) {
        rows.push(format!("shared:{focus}"));
    }
    rows
}

#[derive(Default)]
struct AiChanceScan {
    file_count: usize,
    ai_chance_count: usize,
    invalid_count: usize,
    invalid_items: Vec<String>,
}

fn scan_ai_chance(root: &Path) -> Result<AiChanceScan, String> {
    let mut scan = AiChanceScan::default();
    for relative in ["events", "common/decisions"] {
        let dir = root.join(relative);
        if !dir.is_dir() {
            continue;
        }
        for file in collect_files(&dir)? {
            if file.extension().and_then(OsStr::to_str) != Some("txt") {
                continue;
            }
            scan.file_count += 1;
            let text = read_utf8_lossy(&file)?;
            for value in assignment_values_in_text(&text, "ai_chance") {
                scan.ai_chance_count += 1;
                let value = value.trim();
                if value.starts_with('{') || value.parse::<i64>().is_ok() {
                    continue;
                }
                scan.invalid_count += 1;
                scan.invalid_items.push(relative_slash_path(root, &file));
            }
        }
    }
    Ok(scan)
}

struct AiBehaviorReport<'a> {
    ok: bool,
    game_root: &'a Path,
    mod_root: Option<&'a Path>,
    focuses: &'a [String],
    technologies: &'a [String],
    equipment: &'a [String],
    wargoal_types: &'a [String],
    target_tags: &'a [String],
    target_filter: Option<&'a str>,
    strategy_types: &'a [String],
    route_diff: &'a [String],
    ai_chance: &'a AiChanceScan,
    blockers: &'a [String],
}

fn ai_behavior_audit_json(report: AiBehaviorReport<'_>) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.ai_behavior_audit.v1"),
    );
    map.insert("ok".to_string(), json_bool(report.ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if report.ok {
            "ai_behavior_audit_ready"
        } else {
            "ai_behavior_audit_blocked"
        }),
    );
    map.insert(
        "game_root".to_string(),
        json_str(&report.game_root.display().to_string()),
    );
    map.insert(
        "mod_root".to_string(),
        json_optional_str(
            report
                .mod_root
                .map(|path| path.to_string_lossy())
                .as_deref(),
        ),
    );
    map.insert("focuses".to_string(), json_array(report.focuses));
    map.insert("technologies".to_string(), json_array(report.technologies));
    map.insert("equipment".to_string(), json_array(report.equipment));
    map.insert(
        "wargoal_types".to_string(),
        json_array(report.wargoal_types),
    );
    map.insert("target_tags".to_string(), json_array(report.target_tags));
    map.insert(
        "target_filter".to_string(),
        json_optional_str(report.target_filter),
    );
    map.insert(
        "strategy_types".to_string(),
        json_array(report.strategy_types),
    );
    map.insert("route_diff".to_string(), json_array(report.route_diff));
    map.insert(
        "ai_chance".to_string(),
        ai_chance_scan_json(report.ai_chance),
    );
    map.insert(
        "integration_commands".to_string(),
        json_array(&[
            "hoi4skill route-blocker-audit --mod-root <mod> --target-event <event> --require-passed".to_string(),
            "hoi4skill runtime-error-regression --error-log <error.log> --baseline <baseline.json> --mod-root <mod> --require-passed".to_string(),
        ]),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "AI focus/strategy/template references must be indexed".to_string(),
            "route-generated AI preferences must not collapse into a fixed tag shortcut"
                .to_string(),
            "event and decision ai_chance must remain visible to route/blocker review".to_string(),
        ]),
    );
    map.insert("blockers".to_string(), json_array(report.blockers));
    json_raw_object(&map) + "\n"
}

fn ai_chance_scan_json(scan: &AiChanceScan) -> String {
    format!(
        "{{\"file_count\": {}, \"ai_chance_count\": {}, \"invalid_count\": {}, \"invalid_items\": {}}}",
        scan.file_count,
        scan.ai_chance_count,
        scan.invalid_count,
        json_array(&scan.invalid_items)
    )
}

struct AiBehaviorSkeleton<'a> {
    prefix: &'a str,
    focuses: &'a [String],
    technologies: &'a [String],
    equipment: &'a [String],
    wargoal_types: &'a [String],
    target_tags: &'a [String],
    strategy_types: &'a [String],
    route_diff: &'a [String],
}

fn ai_behavior_skeleton_files(input: AiBehaviorSkeleton<'_>) -> Vec<(String, String)> {
    let stem = sanitize_ai_behavior_file_stem(input.prefix);
    vec![
        (
            format!("common/ai_strategy/{stem}_ai_strategy.txt"),
            ai_strategy_skeleton(&stem, &input),
        ),
        (
            format!("common/ai_strategy_plans/{stem}_ai_strategy_plan.txt"),
            ai_strategy_plan_skeleton(&stem, &input),
        ),
        (
            format!("common/ai_focuses/{stem}_ai_focuses.txt"),
            ai_focuses_skeleton(&stem, &input),
        ),
        (
            format!("common/ai_templates/{stem}_ai_templates.txt"),
            ai_templates_skeleton(&stem, &input),
        ),
    ]
}

fn ai_strategy_skeleton(stem: &str, input: &AiBehaviorSkeleton<'_>) -> String {
    format!(
        "# Generated by hoi4skill P20 ai-behavior-apply.\n# prefix = {stem}\n# strategy_types = {}\n# target_tags = {}\n# target_filter must stay in the audited route context; do not replace it with a fixed shortcut.\n# wargoal_types = {}\n",
        input.strategy_types.join(", "),
        input.target_tags.join(", "),
        input.wargoal_types.join(", ")
    )
}

fn ai_strategy_plan_skeleton(stem: &str, input: &AiBehaviorSkeleton<'_>) -> String {
    format!(
        "# Generated by hoi4skill P20 ai-behavior-apply.\n# plan_id = {stem}_plan\n# route_diff = {}\n# technologies = {}\n# Fill with schema-specific AI strategy plan entries only after route-blocker-audit passes.\n",
        input.route_diff.join(", "),
        input.technologies.join(", ")
    )
}

fn ai_focuses_skeleton(stem: &str, input: &AiBehaviorSkeleton<'_>) -> String {
    format!(
        "# Generated by hoi4skill P20 ai-behavior-apply.\n# focus_preference_set = {stem}\n# indexed_focuses = {}\n# Keep player_only/ai_only differences visible in review; do not hide route divergence.\n",
        input.focuses.join(", ")
    )
}

fn ai_templates_skeleton(stem: &str, input: &AiBehaviorSkeleton<'_>) -> String {
    format!(
        "# Generated by hoi4skill P20 ai-behavior-apply.\n# template_set = {stem}\n# indexed_equipment = {}\n# Fill division/template details only from indexed unit and equipment evidence.\n",
        input.equipment.join(", ")
    )
}

fn sanitize_ai_behavior_file_stem(value: &str) -> String {
    let stem = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "ai_behavior".to_string()
    } else {
        stem
    }
}

fn write_ai_behavior_transaction(
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
        .iter()
        .map(|path| path.display().to_string())
        .collect())
}

fn rollback_ai_behavior_files(changed: &[PathBuf]) -> Vec<String> {
    let mut blockers = Vec::new();
    for path in changed.iter().rev() {
        if let Err(err) = fs::remove_file(path) {
            blockers.push(format!("rollback remove {}: {err}", path.display()));
        }
    }
    blockers
}

fn ai_behavior_apply_json(
    input: &Path,
    ok: bool,
    prefix: &str,
    changed_files: &[String],
    blockers: &[String],
    rollback_blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.ai_behavior_apply.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "ai_behavior_applied"
        } else {
            "ai_behavior_apply_blocked"
        }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert("prefix".to_string(), json_str(prefix));
    map.insert(
        "transaction".to_string(),
        json_str(if ok {
            "committed_ai_behavior_skeleton_files"
        } else if changed_files.is_empty() {
            "not_started_no_files_changed"
        } else {
            "rollback_attempted"
        }),
    );
    map.insert("changed_files".to_string(), json_array(changed_files));
    map.insert(
        "rollback_ok".to_string(),
        json_bool(rollback_blockers.is_empty()).to_string(),
    );
    map.insert(
        "rollback_blockers".to_string(),
        json_array(rollback_blockers),
    );
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "final_check".to_string(),
        json_str("run route-blocker-audit, validate --strict-code-index, and runtime-error-regression after filling AI behavior skeletons"),
    );
    json_raw_object(&map) + "\n"
}
