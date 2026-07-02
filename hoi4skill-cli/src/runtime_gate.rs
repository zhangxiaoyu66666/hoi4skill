//! P15 runtime error-log regression and playable acceptance gates.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_runtime_error_baseline(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let error_log = normalize_path(&require_value(&map, "error-log")?)?;
    let text = read_utf8_lossy(&error_log)?;
    let diagnostics = analyze_error_log(&text, None);
    let raw = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.raw.clone())
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"error_log\": {},\n  \"diagnostic_count\": {},\n  \"baseline_raw\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.runtime_error_baseline.v1"),
        json_str("baseline_ready"),
        json_str(&error_log.display().to_string()),
        raw.len(),
        json_array(&raw),
        json_str("baseline records existing diagnostics so regression checks can report only new runtime errors")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_runtime_error_regression(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let error_log = normalize_path(&require_value(&map, "error-log")?)?;
    let baseline = normalize_path(&require_value(&map, "baseline")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let changed_files = runtime_changed_files(&map)?;
    let text = read_utf8_lossy(&error_log)?;
    let baseline_text = read_utf8_lossy(&baseline)?;
    let baseline_raw = json_string_array_field(&baseline_text, "baseline_raw");
    let baseline_set = baseline_raw.iter().cloned().collect::<BTreeSet<_>>();
    let mut diagnostics = analyze_error_log(&text, mod_root.as_deref());
    diagnostics.retain(|diagnostic| !baseline_set.contains(&diagnostic.raw));
    if !changed_files.is_empty() {
        diagnostics
            .retain(|diagnostic| diagnostic_runtime_touches_changed(diagnostic, &changed_files));
    }
    let rows = diagnostics
        .iter()
        .map(error_log_diagnostic_json)
        .collect::<Vec<_>>();
    let ok = diagnostics.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"error_log\": {},\n  \"baseline\": {},\n  \"changed_files\": {},\n  \"new_diagnostic_count\": {},\n  \"new_diagnostics\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.runtime_error_regression.v1"),
        json_bool(ok),
        json_str(if ok { "no_runtime_regression" } else { "runtime_regression_found" }),
        json_str(&error_log.display().to_string()),
        json_str(&baseline.display().to_string()),
        json_array(&changed_files),
        diagnostics.len(),
        rows.join(", "),
        json_str("runtime regression compares post-run error.log to baseline and optionally narrows diagnostics to changed files")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(format!("{} new runtime diagnostics", diagnostics.len()));
    }
    Ok(())
}

pub(crate) fn cmd_playable_smoke_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let package = require_value(&map, "package")?;
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"package\": {},\n  \"required_evidence\": {},\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.playable_smoke_plan.v1"),
        json_str("playable_smoke_plan_ready"),
        json_str(&mod_root.display().to_string()),
        json_str(&game_root.display().to_string()),
        json_str(&package),
        json_array(&[
            "validate --strict-code-index report".to_string(),
            "runtime-error-baseline before launch".to_string(),
            "runtime-error-regression after launch".to_string(),
            "route-guide or package playtest report".to_string(),
        ]),
        json_array(&[
            "hoi4skill validate <mod> --game-root <hoi4> --strict-code-index --output validation.json".to_string(),
            "hoi4skill runtime-error-baseline --error-log <before error.log> --output baseline.json".to_string(),
            "hoi4skill runtime-error-regression --error-log <after error.log> --baseline baseline.json --require-passed --output regression.json".to_string(),
        ]),
        json_str("playable smoke is a plan only; final acceptance requires validation, runtime regression, and route/playtest evidence")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_playable_acceptance_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let validation = read_optional_report_bool(
        &map,
        "validation",
        &["\"ok\": true", "\"status\": \"passed\""],
    )?;
    let regression = read_optional_report_bool(
        &map,
        "error-regression",
        &["\"ok\": true", "no_runtime_regression"],
    )?;
    let route = read_optional_report_bool(&map, "route-guide", &["\"ok\": true", "route_ready"])?;
    let mut blockers = Vec::new();
    if !validation {
        blockers.push("validation evidence is missing or not passing".to_string());
    }
    if !regression {
        blockers.push("runtime error regression evidence is missing or not passing".to_string());
    }
    if !route {
        blockers.push("route/playability evidence is missing or not passing".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"validation_ready\": {},\n  \"runtime_regression_ready\": {},\n  \"route_ready\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.playable_acceptance_gate.v1"),
        json_bool(ok),
        json_str(if ok { "playable_acceptance_passed" } else { "blocked" }),
        json_bool(validation),
        json_bool(regression),
        json_bool(route),
        json_array(&blockers),
        json_str("release/playable acceptance requires static validation plus runtime error regression plus route or playtest evidence")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_runtime_evidence_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let transaction = value(&map, "transaction").map(normalize_path).transpose()?;
    let changed_files = runtime_evidence_changed_files(&map, transaction.as_deref())?;
    let validation = read_optional_report_bool(
        &map,
        "validation",
        &["\"ok\": true", "\"status\": \"passed\""],
    )?;
    let regression = read_optional_report_bool(
        &map,
        "error-regression",
        &["\"ok\": true", "no_runtime_regression"],
    )?;
    let route = read_optional_report_bool(
        &map,
        "route-guide",
        &[
            "\"ok\": true",
            "route_ready",
            "route_guide_ready",
            "route_plan_ready",
        ],
    )?;
    let route_prewrite_only = read_optional_report_text(&map, "route-guide")?
        .is_some_and(|text| text.contains("\"schema\": \"hoi4skill.transaction_route_plan.v1\""));
    let map_runtime = read_optional_report_bool(
        &map,
        "map-runtime",
        &["\"ok\": true", "map_runtime_gate_ready"],
    )?;
    let gui_runtime = read_optional_report_bool(
        &map,
        "gui-runtime",
        &[
            "\"ok\": true",
            "\"runtime_evidence_ready\": true",
            "\"playable_ready\": true",
        ],
    )?;
    let lanes = runtime_evidence_lanes(&changed_files);
    let requires_route = lanes.iter().any(|lane| {
        matches!(
            lane.as_str(),
            "focus" | "event" | "decision" | "on_action" | "scripted_effect"
        )
    });
    let requires_map = lanes
        .iter()
        .any(|lane| matches!(lane.as_str(), "map" | "history"));
    let requires_gui = lanes.iter().any(|lane| lane == "gui");
    let mut blockers = Vec::new();
    if mod_root.is_none() {
        blockers.push("runtime-evidence-gate requires --mod-root".to_string());
    }
    if game_root.is_none() {
        blockers.push("runtime-evidence-gate requires --game-root".to_string());
    }
    if !validation {
        blockers.push("validation evidence is missing or not passing".to_string());
    }
    if !regression {
        blockers.push("runtime error regression evidence is missing or not passing".to_string());
    }
    if requires_route && !route {
        blockers.push("route evidence is required for focus/event/decision changes".to_string());
    }
    if requires_map && !map_runtime {
        blockers.push("map runtime evidence is required for map/history/OOB changes".to_string());
    }
    if requires_gui && !gui_runtime {
        blockers.push("GUI runtime evidence is required for GUI changes".to_string());
    }
    if changed_files.is_empty() {
        blockers.push(
            "runtime-evidence-gate needs changed_files from --transaction or --changed".to_string(),
        );
    }
    let ok = blockers.is_empty();
    let status = if ok && route_prewrite_only {
        "prewrite_evidence_ready"
    } else if ok {
        "runtime_evidence_ready"
    } else {
        "blocked"
    };
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"transaction\": {},\n  \"changed_files\": {},\n  \"lanes\": {},\n  \"checks\": {{\"validation_ready\": {}, \"runtime_regression_ready\": {}, \"route_required\": {}, \"route_ready\": {}, \"route_prewrite_only\": {}, \"map_runtime_required\": {}, \"map_runtime_ready\": {}, \"gui_runtime_required\": {}, \"gui_runtime_ready\": {}}},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.runtime_evidence_gate.v1"),
        json_bool(ok),
        json_str(status),
        json_optional_str(mod_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_optional_str(game_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_optional_str(transaction.as_ref().map(|path| path.display().to_string()).as_deref()),
        json_array(&changed_files),
        json_array(&lanes),
        json_bool(validation),
        json_bool(regression),
        json_bool(requires_route),
        json_bool(route),
        json_bool(route_prewrite_only),
        json_bool(requires_map),
        json_bool(map_runtime),
        json_bool(requires_gui),
        json_bool(gui_runtime),
        json_array(&blockers),
        json_array(&runtime_evidence_next_commands(requires_route, requires_map, requires_gui)),
        json_str("Do not claim playable acceptance from prewrite_evidence_ready; transaction-route-plan is pre-write topology evidence and must be followed by real route/runtime checks after writers apply.")
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_runtime_release_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let validation = read_optional_report_bool(
        &map,
        "validation",
        &["\"ok\": true", "\"status\": \"passed\""],
    )?;
    let regression = read_optional_report_bool(
        &map,
        "error-regression",
        &["\"ok\": true", "no_runtime_regression"],
    )?;
    let runtime_evidence = read_optional_report_bool(
        &map,
        "runtime-evidence",
        &[
            "\"ok\": true",
            "runtime_evidence_ready",
            "prewrite_evidence_ready",
        ],
    )?;
    let map_runtime = read_optional_report_bool(
        &map,
        "map-runtime",
        &["\"ok\": true", "map_runtime_gate_ready"],
    )?;
    let gui_runtime = read_optional_report_bool(
        &map,
        "gui-runtime",
        &[
            "\"ok\": true",
            "\"runtime_evidence_ready\": true",
            "\"playable_ready\": true",
        ],
    )?;
    let loc_audit =
        read_optional_report_bool(&map, "loc-audit", &["\"ok\": true", "\"status\": \"ok\""])?;
    let gfx_audit =
        read_optional_report_bool(&map, "gfx-audit", &["\"ok\": true", "\"status\": \"ok\""])?;
    let required_phases = if map.flags.contains("require-p101-p108") {
        (101..=108).map(|id| format!("P{id}")).collect::<Vec<_>>()
    } else {
        repeated_values(&map, "require-phase")
            .into_iter()
            .map(|phase| phase.to_ascii_uppercase())
            .collect::<Vec<_>>()
    };
    let phase_reports = runtime_release_phase_reports(&map)?;
    let phase_status = runtime_release_phase_status_json(&required_phases, &phase_reports);
    let missing_phases = runtime_release_missing_phases(&required_phases, &phase_reports);
    let mut blockers = Vec::new();
    if mod_root.is_none() {
        blockers.push("runtime-release-gate requires --mod-root".to_string());
    }
    if game_root.is_none() {
        blockers.push("runtime-release-gate requires --game-root".to_string());
    }
    if !validation {
        blockers.push("strict validation evidence is missing or not passing".to_string());
    }
    if !regression {
        blockers.push("runtime error regression evidence is missing or not passing".to_string());
    }
    if !runtime_evidence {
        blockers.push("runtime-evidence-gate report is missing or not passing".to_string());
    }
    if map.flags.contains("require-map-runtime") && !map_runtime {
        blockers.push("map-runtime-gate report is missing or not passing".to_string());
    }
    if map.flags.contains("require-gui-runtime") && !gui_runtime {
        blockers.push("GUI runtime report is missing or not passing".to_string());
    }
    if map.flags.contains("require-loc-audit") && !loc_audit {
        blockers.push("loc-audit report is missing or not passing".to_string());
    }
    if map.flags.contains("require-gfx-audit") && !gfx_audit {
        blockers.push("gfx-audit report is missing or not passing".to_string());
    }
    for phase in &missing_phases {
        blockers.push(format!("{phase} phase report is missing or not ready"));
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"checks\": {{\"validation_ready\": {}, \"runtime_error_regression_ready\": {}, \"runtime_evidence_ready\": {}, \"map_runtime_required\": {}, \"map_runtime_ready\": {}, \"gui_runtime_required\": {}, \"gui_runtime_ready\": {}, \"loc_audit_required\": {}, \"loc_audit_ready\": {}, \"gfx_audit_required\": {}, \"gfx_audit_ready\": {}}},\n  \"required_phases\": {},\n  \"phase_status\": {},\n  \"missing_phases\": {},\n  \"blocker_count\": {},\n  \"blockers\": {},\n  \"repair_context_commands\": {},\n  \"release_gate_rule\": {}\n}}\n",
        json_str("hoi4skill.runtime_release_gate.v1"),
        json_bool(ok),
        json_str(if ok { "runtime_release_ready" } else { "blocked" }),
        json_optional_str(mod_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_optional_str(game_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_bool(validation),
        json_bool(regression),
        json_bool(runtime_evidence),
        json_bool(map.flags.contains("require-map-runtime")),
        json_bool(map_runtime),
        json_bool(map.flags.contains("require-gui-runtime")),
        json_bool(gui_runtime),
        json_bool(map.flags.contains("require-loc-audit")),
        json_bool(loc_audit),
        json_bool(map.flags.contains("require-gfx-audit")),
        json_bool(gfx_audit),
        json_array(&required_phases),
        phase_status,
        json_array(&missing_phases),
        blockers.len(),
        json_array(&blockers),
        json_array(&runtime_release_repair_commands()),
        json_str("Release is blocked unless strict validation, runtime error regression, runtime evidence, required lane runtime reports, and required P-stage reports all pass.")
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn runtime_changed_files(map: &ArgMap) -> Result<Vec<String>, String> {
    let mut out = repeated_values(map, "changed")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    for file in repeated_values(map, "changed-file") {
        let text = read_utf8_lossy(&normalize_path(file)?)?;
        out.extend(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string),
        );
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn runtime_evidence_changed_files(
    map: &ArgMap,
    transaction: Option<&Path>,
) -> Result<Vec<String>, String> {
    let mut out = runtime_changed_files(map)?;
    if let Some(transaction) = transaction {
        let text = read_utf8_lossy(transaction)?;
        out.extend(json_string_array_field(&text, "changed_files"));
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn runtime_evidence_lanes(changed_files: &[String]) -> Vec<String> {
    let mut lanes = BTreeSet::new();
    for file in changed_files {
        let normalized = file.replace('\\', "/").to_ascii_lowercase();
        let lane = if normalized.starts_with("common/national_focus/") {
            "focus"
        } else if normalized.starts_with("events/") {
            "event"
        } else if normalized.starts_with("common/decisions/") {
            "decision"
        } else if normalized.starts_with("common/ideas/") {
            "idea"
        } else if normalized.starts_with("common/on_actions/") {
            "on_action"
        } else if normalized.starts_with("common/scripted_effects/") {
            "scripted_effect"
        } else if normalized.starts_with("gfx/")
            || normalized.ends_with(".gfx")
            || normalized.contains("sprite")
            || normalized.contains("flag")
        {
            "asset"
        } else if normalized.starts_with("interface/")
            || normalized.starts_with("common/scripted_guis/")
            || normalized.ends_with(".gui")
        {
            "gui"
        } else if normalized.starts_with("map/") {
            "map"
        } else if normalized.starts_with("history/") {
            "history"
        } else if normalized.starts_with("localisation/") || normalized.starts_with("localization/")
        {
            "localisation"
        } else {
            "content"
        };
        lanes.insert(lane.to_string());
    }
    lanes.into_iter().collect()
}

fn runtime_evidence_next_commands(
    requires_route: bool,
    requires_map: bool,
    requires_gui: bool,
) -> Vec<String> {
    let mut commands = vec![
        "hoi4skill validate <mod> --game-root <hoi4> --strict-code-index --output .hoi4skill/validation.json".to_string(),
        "hoi4skill runtime-error-regression --error-log <after/error.log> --baseline .hoi4skill/runtime_baseline.json --require-passed --output .hoi4skill/runtime_error_regression.json".to_string(),
    ];
    if requires_route {
        commands.push("hoi4skill route-blocker-audit --mod-root <mod> --target-event <event> --require-passed --output .hoi4skill/route_blockers.json".to_string());
    }
    if requires_map {
        commands.push("hoi4skill map-runtime-gate --error-log <error.log> --map-log <map.log> --baseline <baseline> --require-passed --output .hoi4skill/map_runtime_gate.json".to_string());
    }
    if requires_gui {
        commands.push("hoi4skill gui-runtime-evidence-contract --mod-root <mod> --require-passed --output .hoi4skill/gui_runtime_evidence_contract.json".to_string());
    }
    commands
}

fn runtime_release_phase_reports(map: &ArgMap) -> Result<Vec<String>, String> {
    let mut reports = Vec::new();
    for key in ["phase-report", "phase", "report"] {
        for path in repeated_values(map, key) {
            reports.push(read_utf8_lossy(&normalize_path(path)?)?);
        }
    }
    Ok(reports)
}

fn runtime_release_missing_phases(required: &[String], reports: &[String]) -> Vec<String> {
    required
        .iter()
        .filter(|phase| !runtime_release_phase_ready(phase, reports))
        .cloned()
        .collect()
}

fn runtime_release_phase_ready(phase: &str, reports: &[String]) -> bool {
    reports.iter().any(|text| {
        (json_report_contains_marker(text, &format!("\"phase_filter\": \"{phase}\""))
            || json_report_contains_marker(text, &format!("\"id\": \"{phase}\"")))
            && (json_report_contains_marker(text, "\"ok\": true")
                || json_report_contains_marker(text, "\"ready\": true")
                || json_report_contains_marker(text, "\"all_selected_ready\": true"))
    })
}

fn runtime_release_phase_status_json(required: &[String], reports: &[String]) -> String {
    let rows = required
        .iter()
        .map(|phase| {
            format!(
                "{{\"phase\": {}, \"ready\": {}}}",
                json_str(phase),
                json_bool(runtime_release_phase_ready(phase, reports))
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", rows.join(", "))
}

fn runtime_release_repair_commands() -> Vec<String> {
    vec![
        "hoi4skill validate <mod> --game-root <hoi4> --strict-code-index --output .hoi4skill/validation.json".to_string(),
        "hoi4skill runtime-error-regression --error-log <after/error.log> --baseline .hoi4skill/runtime_baseline.json --require-passed --output .hoi4skill/runtime_error_regression.json".to_string(),
        "hoi4skill runtime-evidence-gate --mod-root <mod> --game-root <hoi4> --transaction <transaction.json> --validation .hoi4skill/validation.json --error-regression .hoi4skill/runtime_error_regression.json --require-passed --output .hoi4skill/runtime_evidence_gate.json".to_string(),
        "hoi4skill validate-repair-context <mod> --game-root <hoi4> --request <literal-user-request> --output .hoi4skill/ai_repair_context.json".to_string(),
    ]
}

fn diagnostic_runtime_touches_changed(
    diagnostic: &ErrorLogDiagnostic,
    changed_files: &[String],
) -> bool {
    let haystack = format!(
        "{}\n{}\n{}",
        diagnostic.raw,
        diagnostic.file.as_deref().unwrap_or(""),
        diagnostic.resolved_file.as_deref().unwrap_or("")
    )
    .replace('\\', "/")
    .to_ascii_lowercase();
    changed_files.iter().any(|file| {
        let file = file.replace('\\', "/").to_ascii_lowercase();
        haystack.contains(&file)
    })
}

fn read_optional_report_bool(
    map: &ArgMap,
    key: &str,
    pass_markers: &[&str],
) -> Result<bool, String> {
    let Some(path) = value(map, key) else {
        return Ok(false);
    };
    let text = read_utf8_lossy(&normalize_path(path)?)?;
    Ok(pass_markers
        .iter()
        .any(|marker| json_report_contains_marker(&text, marker)))
}

fn read_optional_report_text(map: &ArgMap, key: &str) -> Result<Option<String>, String> {
    let Some(path) = value(map, key) else {
        return Ok(None);
    };
    Ok(Some(read_utf8_lossy(&normalize_path(path)?)?))
}

fn json_report_contains_marker(text: &str, marker: &str) -> bool {
    if text.contains(marker) {
        return true;
    }
    let compact_text = text
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    let compact_marker = marker
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    compact_text.contains(&compact_marker)
}

fn json_string_array_field(text: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{key}\"");
    let Some(start) = text.find(&needle) else {
        return Vec::new();
    };
    let Some(array_start_rel) = text[start..].find('[') else {
        return Vec::new();
    };
    let mut values = Vec::new();
    let mut chars = text[start + array_start_rel + 1..].chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut current = String::new();
    while let Some(ch) = chars.next() {
        if in_string {
            if escaped {
                current.push(match ch {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                values.push(current.clone());
                current.clear();
                in_string = false;
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == ']' {
            break;
        }
    }
    values
}
