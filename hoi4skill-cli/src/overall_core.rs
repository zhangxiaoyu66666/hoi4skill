//! Overall core completion audit.
//!
//! P61 is a read-only "truth table" for the non-visual large-mod compiler. It
//! reports which lanes can parse, plan, transact, apply, final-check, and enter
//! runtime/release gates, using local game/target/dependency paths as evidence.

#[allow(unused_imports)]
use crate::*;

struct OverallSystemRow {
    system: &'static str,
    coverage_level: &'static str,
    natural_language: bool,
    plan: bool,
    transaction: bool,
    apply: bool,
    final_check: bool,
    runtime_gate: bool,
    command: &'static str,
    evidence: &'static str,
    gap: &'static str,
    target_evidence: String,
}

pub(crate) fn cmd_overall_core_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;

    if !game_root.is_dir() {
        return Err(format!(
            "game root is not a directory: {}",
            game_root.display()
        ));
    }
    if !target_root.is_dir() {
        return Err(format!(
            "mod root is not a directory: {}",
            target_root.display()
        ));
    }

    let rows = overall_system_rows(&target_root);
    let partial_systems = rows
        .iter()
        .filter(|row| row.coverage_level != "writer")
        .map(|row| format!("{}:{}", row.system, row.coverage_level))
        .collect::<Vec<_>>();
    let missing_runtime = rows
        .iter()
        .filter(|row| !row.runtime_gate)
        .map(|row| row.system.to_string())
        .collect::<Vec<_>>();
    let blockers = core_safety_blockers(&rows);
    let ok = blockers.is_empty();

    let report = overall_core_audit_json(
        ok,
        &game_root,
        &target_root,
        &parent_roots,
        &rows,
        &partial_systems,
        &missing_runtime,
        &blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if let Some(output) = value(&map, "markdown-output") {
        write_or_print(
            &overall_core_audit_markdown(ok, &rows, &partial_systems, &missing_runtime, &blockers),
            Some(output),
        )?;
    }
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn overall_system_rows(target_root: &Path) -> Vec<OverallSystemRow> {
    let specs = vec![
        (
            "author_compiler",
            "planner",
            true,
            true,
            true,
            false,
            true,
            false,
            "author-compiler-plan",
            "planned P62; current lanes use author-one-shot/lane-isolation-audit",
            "needs one command for text/docx/xlsx/csv to transaction",
            &[".hoi4skill", "common", "events", "localisation"][..],
        ),
        (
            "knowledge_freshness",
            "writer",
            false,
            true,
            true,
            true,
            true,
            false,
            "knowledge-delta-refresh + stale-plan-gate",
            "hoi4skill-cli/src/knowledge.rs; hoi4skill-cli/src/parent_mod.rs",
            "",
            &[".hoi4skill"][..],
        ),
        (
            "scope_symbol_contract",
            "writer",
            false,
            true,
            true,
            true,
            true,
            false,
            "symbol-registration-audit + scope-compat-audit",
            "hoi4skill-cli/src/safety.rs; hoi4skill-cli/src/scope_systems.rs",
            "",
            &["common"][..],
        ),
        (
            "focus",
            "writer",
            true,
            true,
            true,
            true,
            true,
            true,
            "apply-focus-layout / apply-focus-excel / mod-transaction-apply",
            "hoi4skill-cli/src/focus_layout.rs; hoi4skill-cli/src/focus_excel.rs",
            "",
            &["common/national_focus"][..],
        ),
        (
            "event",
            "planner",
            true,
            true,
            true,
            false,
            true,
            true,
            "apply-event-cards + event-chain-graph",
            "hoi4skill-cli/src/event_cards.rs; hoi4skill-cli/src/route.rs",
            "needs direct atomic event-chain writer coverage",
            &["events"][..],
        ),
        (
            "idea",
            "planner",
            true,
            true,
            true,
            false,
            true,
            false,
            "apply-feature-cards / assemble-code",
            "hoi4skill-cli/src/feature_cards.rs; hoi4skill-cli/src/transaction.rs",
            "needs direct idea writer for national spirits/advisors/laws separation",
            &["common/ideas"][..],
        ),
        (
            "dynamic_modifier",
            "planner",
            true,
            true,
            true,
            false,
            true,
            false,
            "dynamic modifier cards + scripted_effect helpers",
            "hoi4skill-cli/src/feature_cards.rs; hoi4skill-cli/src/safety.rs",
            "needs stronger parent-template parameter collision checks",
            &["common/dynamic_modifiers", "common/scripted_effects"][..],
        ),
        (
            "decision",
            "planner",
            true,
            true,
            true,
            false,
            true,
            true,
            "apply-feature-cards",
            "hoi4skill-cli/src/feature_cards.rs",
            "needs direct decision writer and route/runtime gate integration",
            &["common/decisions"][..],
        ),
        (
            "history_oob",
            "review_pack",
            true,
            true,
            true,
            false,
            true,
            false,
            "history-transaction-plan / history-transaction-apply",
            "hoi4skill-cli/src/history_scenario.rs; hoi4skill-cli/src/unit_taxonomy.rs",
            "history remains review-pack by policy; needs P66 split for land/air/navy apply",
            &["history/countries", "history/states", "history/units"][..],
        ),
        (
            "map",
            "planner",
            true,
            true,
            true,
            true,
            true,
            true,
            "map-data-audit / state-transaction-apply / map-release-gate",
            "hoi4skill-cli/src/map_data.rs",
            "topology stays manual gate; strategic-region direct apply still limited",
            &["map", "history/states"][..],
        ),
        (
            "asset",
            "planner",
            true,
            true,
            true,
            false,
            true,
            true,
            "flag-image-import / register-gfx-icons / author-placeholder-plan",
            "hoi4skill-cli/src/flags.rs; hoi4skill-cli/src/icons.rs",
            "needs P68 full asset transaction apply and rollback manifest",
            &["gfx", "interface"][..],
        ),
        (
            "localisation",
            "writer",
            true,
            true,
            true,
            true,
            true,
            false,
            "localisation-token-check / translate-localisation / check-text-alignment",
            "hoi4skill-cli/src/localisation_tokens.rs; hoi4skill-cli/src/localisation_translate.rs",
            "",
            &["localisation"][..],
        ),
        (
            "gui",
            "planner",
            true,
            true,
            true,
            true,
            true,
            true,
            "gui-request-workflow / apply-gui-intent / gui-runtime-evidence-collect",
            "hoi4skill-cli/src/large_mod.rs",
            "needs P69 stable control schema for future visual editor",
            &["common/scripted_guis", "interface"][..],
        ),
        (
            "route_guide",
            "planner",
            true,
            true,
            false,
            false,
            true,
            false,
            "route-simulation-plan / gameplay-route-guide",
            "hoi4skill-cli/src/route.rs",
            "needs P67 route simulation over focus/event/decision/gui edges",
            &["events", "common/national_focus", "common/decisions"][..],
        ),
        (
            "common_high_value",
            "planner",
            true,
            true,
            false,
            false,
            true,
            false,
            "common-coverage-audit / common-release-gate",
            "hoi4skill-cli/src/common_coverage.rs; hoi4skill-cli/src/common_writers.rs",
            "needs writers for on_actions, scripted_localisation, opinion_modifiers, bookmarks, game_rules, BOP, AI strategy",
            &["common"][..],
        ),
        (
            "runtime_release",
            "planner",
            false,
            true,
            true,
            false,
            true,
            true,
            "hoi4-runtime-session-plan / large-mod-release-workflow",
            "hoi4skill-cli/src/runtime_session.rs; hoi4skill-cli/src/large_mod.rs",
            "needs P70 single release workflow wiring every gate",
            &[".hoi4skill"][..],
        ),
        (
            "weak_ai_regression",
            "planner",
            false,
            true,
            false,
            false,
            true,
            false,
            "weak-ai-regression-suite",
            "planned P70; current insurance is large-mod-ai-output-insurance",
            "needs release-time sample suite for typo/scope/container/token/map/gui/dead-route failures",
            &[".hoi4skill"][..],
        ),
    ];

    specs
        .into_iter()
        .map(
            |(
                system,
                coverage_level,
                natural_language,
                plan,
                transaction,
                apply,
                final_check,
                runtime_gate,
                command,
                evidence,
                gap,
                probes,
            )| OverallSystemRow {
                system,
                coverage_level,
                natural_language,
                plan,
                transaction,
                apply,
                final_check,
                runtime_gate,
                command,
                evidence,
                gap,
                target_evidence: target_probe_summary(target_root, probes),
            },
        )
        .collect()
}

fn target_probe_summary(target_root: &Path, probes: &[&str]) -> String {
    let hits = probes
        .iter()
        .filter(|probe| target_root.join(probe).exists())
        .copied()
        .collect::<Vec<_>>();
    if hits.is_empty() {
        "not_present_in_target_surface".to_string()
    } else {
        format!("target_has:{}", hits.join(","))
    }
}

fn core_safety_blockers(rows: &[OverallSystemRow]) -> Vec<String> {
    let required = [
        "knowledge_freshness",
        "scope_symbol_contract",
        "localisation",
    ];
    rows.iter()
        .filter(|row| required.contains(&row.system))
        .filter(|row| {
            row.coverage_level == "none" || !row.plan || !row.final_check || row.command.is_empty()
        })
        .map(|row| format!("core safety system `{}` is not ready", row.system))
        .collect()
}

fn overall_core_audit_json(
    ok: bool,
    game_root: &Path,
    target_root: &Path,
    parent_roots: &[PathBuf],
    rows: &[OverallSystemRow],
    partial_systems: &[String],
    missing_runtime: &[String],
    blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.overall_core_audit.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "overall_core_report_ready"
        } else {
            "overall_core_blocked"
        }),
    );
    map.insert(
        "game_root".to_string(),
        json_str(&game_root.display().to_string()),
    );
    map.insert(
        "mod_root".to_string(),
        json_str(&target_root.display().to_string()),
    );
    map.insert(
        "parent_mod_roots".to_string(),
        json_array(
            &parent_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert("system_count".to_string(), rows.len().to_string());
    map.insert(
        "writer_count".to_string(),
        rows.iter()
            .filter(|row| row.coverage_level == "writer")
            .count()
            .to_string(),
    );
    map.insert("partial_systems".to_string(), json_array(partial_systems));
    map.insert(
        "missing_runtime_gate".to_string(),
        json_array(missing_runtime),
    );
    map.insert("systems".to_string(), render_overall_system_rows(rows));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "next_commands".to_string(),
        json_array(&[
            "hoi4skill core-capability-audit --phase P61 --require-passed".to_string(),
            "hoi4skill author-compiler-plan --mod-root <target> --game-root <hoi4> --mod-path <parent> --text <request> --output .hoi4skill/author_compiler_plan.json".to_string(),
            "hoi4skill stale-plan-gate --input .hoi4skill/transaction.json --knowledge .hoi4skill/kb.json --require-passed".to_string(),
            "hoi4skill weak-ai-regression-suite --mod-root <target> --game-root <hoi4> --mod-path <parent> --require-passed --output .hoi4skill/weak_ai_regression.json".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn render_overall_system_rows(rows: &[OverallSystemRow]) -> String {
    format!(
        "[{}]",
        rows.iter()
            .map(|row| {
                let mut map = BTreeMap::new();
                map.insert("system".to_string(), json_str(row.system));
                map.insert("coverage_level".to_string(), json_str(row.coverage_level));
                map.insert(
                    "natural_language".to_string(),
                    json_bool(row.natural_language).to_string(),
                );
                map.insert("plan".to_string(), json_bool(row.plan).to_string());
                map.insert(
                    "transaction".to_string(),
                    json_bool(row.transaction).to_string(),
                );
                map.insert("apply".to_string(), json_bool(row.apply).to_string());
                map.insert(
                    "final_check".to_string(),
                    json_bool(row.final_check).to_string(),
                );
                map.insert(
                    "runtime_gate".to_string(),
                    json_bool(row.runtime_gate).to_string(),
                );
                map.insert("command".to_string(), json_str(row.command));
                map.insert("evidence".to_string(), json_str(row.evidence));
                map.insert(
                    "target_evidence".to_string(),
                    json_str(&row.target_evidence),
                );
                map.insert("gap".to_string(), json_str(row.gap));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn overall_core_audit_markdown(
    ok: bool,
    rows: &[OverallSystemRow],
    partial_systems: &[String],
    missing_runtime: &[String],
    blockers: &[String],
) -> String {
    let mut out = String::new();
    out.push_str("# HOI4Skill Overall Core Audit\n\n");
    out.push_str(&format!(
        "- status: `{}`\n",
        if ok {
            "overall_core_report_ready"
        } else {
            "overall_core_blocked"
        }
    ));
    out.push_str(&format!("- system_count: `{}`\n", rows.len()));
    out.push_str(&format!(
        "- partial_systems: `{}`\n",
        if partial_systems.is_empty() {
            "none".to_string()
        } else {
            partial_systems.join(", ")
        }
    ));
    out.push_str(&format!(
        "- missing_runtime_gate: `{}`\n\n",
        if missing_runtime.is_empty() {
            "none".to_string()
        } else {
            missing_runtime.join(", ")
        }
    ));
    out.push_str("## Systems\n\n");
    for row in rows {
        out.push_str(&format!(
            "- `{}` level=`{}` nl=`{}` plan=`{}` transaction=`{}` apply=`{}` final=`{}` runtime=`{}` command=`{}` gap=`{}`\n",
            row.system,
            row.coverage_level,
            row.natural_language,
            row.plan,
            row.transaction,
            row.apply,
            row.final_check,
            row.runtime_gate,
            row.command,
            if row.gap.is_empty() { "none" } else { row.gap }
        ));
    }
    if !blockers.is_empty() {
        out.push_str("\n## Blockers\n\n");
        for blocker in blockers {
            out.push_str(&format!("- {blocker}\n"));
        }
    }
    out
}
