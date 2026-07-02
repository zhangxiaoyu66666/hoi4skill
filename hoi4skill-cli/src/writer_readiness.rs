//! P65 writer readiness gate.
//!
//! The gate checks a transaction or author compiler plan before apply. It keeps
//! weak AI from slipping an operation into a system without a known writer
//! policy, and makes review-pack/manual gates explicit.

#[allow(unused_imports)]
use crate::*;

struct WriterReadinessRow {
    system: &'static str,
    command: &'static str,
    status: &'static str,
    mutation: &'static str,
    final_gates: Vec<&'static str>,
}

pub(crate) fn cmd_writer_readiness_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let systems = writer_systems_from_plan(&text);
    let changed_files = writer_json_string_array(&text, "changed_files");
    let rows = writer_readiness_rows();
    let mut blockers = Vec::new();
    if systems.is_empty() {
        blockers.push("input has no operation system/lane evidence".to_string());
    }
    if changed_files.is_empty() {
        blockers.push("input has no changed_files manifest".to_string());
    }
    for system in &systems {
        let Some(row) = rows.iter().find(|row| row.system == system) else {
            blockers.push(format!("system `{system}` has no registered writer policy"));
            continue;
        };
        match row.status {
            "direct_writer" => {}
            "review_pack_writer" if map.flags.contains("allow-review-pack") => {}
            "manual_gate" if map.flags.contains("allow-manual-gate") => {}
            status => blockers.push(format!(
                "system `{system}` writer status `{status}` is not allowed for apply"
            )),
        }
    }
    if !map.flags.contains("execute") {
        blockers.push("writer readiness for apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("writer readiness for apply requires --final-check".to_string());
    }
    if !map.flags.contains("atomic") {
        blockers.push("writer readiness for apply requires --atomic".to_string());
    }
    let ok = blockers.is_empty();
    let report = writer_readiness_gate_json(ok, &input, &systems, &changed_files, &rows, &blockers);
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn writer_readiness_rows() -> Vec<WriterReadinessRow> {
    vec![
        writer_row(
            "focus",
            "apply-focus-layout / apply-focus-excel",
            "direct_writer",
            "focus_files",
            vec!["validate --strict-code-index", "check-text-alignment"],
        ),
        writer_row(
            "event",
            "apply-event-cards",
            "direct_writer",
            "event_files",
            vec![
                "route-blocker-audit",
                "validate --strict-code-index",
                "check-text-alignment",
            ],
        ),
        writer_row(
            "idea",
            "apply-feature-cards",
            "direct_writer",
            "idea_files",
            vec![
                "symbol-registration-audit",
                "scope-compat-audit",
                "validate --strict-code-index",
            ],
        ),
        writer_row(
            "dynamic_modifier",
            "apply-feature-cards",
            "direct_writer",
            "dynamic_modifier_files",
            vec!["scope-container-contract", "validate --strict-code-index"],
        ),
        writer_row(
            "decision",
            "apply-feature-cards",
            "direct_writer",
            "decision_files",
            vec!["route-blocker-audit", "validate --strict-code-index"],
        ),
        writer_row(
            "state_history",
            "state-transaction-apply --write-overrides",
            "direct_writer",
            "target_state_override",
            vec!["map-data-audit", "validate --strict-code-index"],
        ),
        writer_row(
            "map",
            "state-transaction-apply / supply-network-apply",
            "direct_writer",
            "state_or_network_override",
            vec!["map-release-gate"],
        ),
        writer_row(
            "asset",
            "flag-image-import / import-generated-icon / register-gfx-icons",
            "direct_writer",
            "asset_manifest",
            vec!["gfx-audit", "validate --strict-code-index"],
        ),
        writer_row(
            "localisation",
            "translate-localisation --apply",
            "direct_writer",
            "localisation_files",
            vec!["localisation-token-check", "check-text-alignment"],
        ),
        writer_row(
            "gui",
            "apply-gui-intent",
            "direct_writer",
            "gui_files",
            vec![
                "gui-runtime-evidence-contract",
                "validate --strict-code-index",
            ],
        ),
        writer_row(
            "history",
            "history-transaction-apply",
            "review_pack_writer",
            "history_patch_pack",
            vec!["validate --strict-code-index"],
        ),
        writer_row(
            "oob",
            "history-transaction-apply / oob-kind-apply",
            "review_pack_writer",
            "oob_patch_pack",
            vec!["unit-taxonomy-audit", "validate --strict-code-index"],
        ),
        writer_row(
            "map_topology",
            "map-topology-gate",
            "manual_gate",
            "no_direct_write",
            vec!["manual confirmation", "map-release-gate"],
        ),
        writer_row(
            "common_high_value",
            "common-release-gate",
            "planned_writer",
            "no_direct_write",
            vec!["common-release-gate"],
        ),
    ]
}

fn writer_row(
    system: &'static str,
    command: &'static str,
    status: &'static str,
    mutation: &'static str,
    final_gates: Vec<&'static str>,
) -> WriterReadinessRow {
    WriterReadinessRow {
        system,
        command,
        status,
        mutation,
        final_gates,
    }
}

fn writer_systems_from_plan(text: &str) -> Vec<String> {
    let mut systems = Vec::new();
    systems.extend(writer_json_string_values(text, "system"));
    systems.extend(writer_json_string_values(text, "lane"));
    systems.sort();
    systems.dedup();
    systems
}

fn writer_json_string_values(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    let marker = format!("\"{key}\"");
    while let Some(pos) = rest.find(&marker) {
        rest = &rest[pos + marker.len()..];
        if let Some(value) = writer_json_string_after_colon(rest) {
            values.push(value);
        }
    }
    values
}

fn writer_json_string_after_colon(text: &str) -> Option<String> {
    let colon = text.find(':')?;
    let mut rest = text[colon + 1..].trim_start();
    rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for ch in rest.chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn writer_json_string_array(text: &str, key: &str) -> Vec<String> {
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

fn writer_readiness_gate_json(
    ok: bool,
    input: &Path,
    systems: &[String],
    changed_files: &[String],
    rows: &[WriterReadinessRow],
    blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.writer_readiness_gate.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok { "writer_ready" } else { "writer_blocked" }),
    );
    map.insert("input".to_string(), json_str(&input.display().to_string()));
    map.insert("systems".to_string(), json_array(systems));
    map.insert("changed_files".to_string(), json_array(changed_files));
    map.insert("writers".to_string(), writer_rows_json(rows));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "writers may only touch transaction changed_files".to_string(),
            "apply requires --execute --final-check --atomic".to_string(),
            "review-pack and manual gates must be explicitly allowed".to_string(),
            "final gates must include strict validation plus text/scope/symbol/runtime gates as applicable".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn writer_rows_json(rows: &[WriterReadinessRow]) -> String {
    format!(
        "[{}]",
        rows.iter()
            .map(|row| {
                let mut map = BTreeMap::new();
                map.insert("system".to_string(), json_str(row.system));
                map.insert("command".to_string(), json_str(row.command));
                map.insert("status".to_string(), json_str(row.status));
                map.insert("mutation".to_string(), json_str(row.mutation));
                map.insert(
                    "final_gates".to_string(),
                    json_array(
                        &row.final_gates
                            .iter()
                            .map(|gate| (*gate).to_string())
                            .collect::<Vec<_>>(),
                    ),
                );
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}
