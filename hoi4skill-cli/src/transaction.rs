//! P3 structured author transactions and narrow Rust-side code assembly.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_author_mod_intent(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let prefix = value(&map, "prefix").unwrap_or("generated");
    let kind = value(&map, "kind")
        .map(str::to_string)
        .unwrap_or_else(|| infer_content_kind(&text).to_string());
    let title = value(&map, "title")
        .map(str::to_string)
        .unwrap_or_else(|| infer_content_title(&text, &kind));
    let id = value(&map, "id")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_{}", prefix, slugify(&title, "content")));
    let effect_text = infer_effect_text(&text);
    let json = render_author_transaction_json(&kind, &id, &title, &effect_text);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_author_one_shot(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    if map.flags.contains("execute") {
        return Err("author-one-shot only writes a transaction plan; use apply-author-transaction --execute after review".to_string());
    }
    let text = author_one_shot_text(&map)?;
    let prefix = value(&map, "prefix").unwrap_or("generated");
    let kind = value(&map, "kind")
        .map(str::to_string)
        .unwrap_or_else(|| infer_content_kind(&text).to_string());
    let title = value(&map, "title")
        .map(str::to_string)
        .unwrap_or_else(|| infer_content_title(&text, &kind));
    let id = value(&map, "id")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_{}", prefix, slugify(&title, "content")));
    let effect_text = infer_effect_text(&text);
    let transaction = render_author_transaction_json(&kind, &id, &title, &effect_text);
    let json =
        render_author_one_shot_plan_json(&text, &kind, &id, &title, &effect_text, &transaction);
    let output = value(&map, "plan-output").or_else(|| value(&map, "output"));
    write_or_print(&json, output)
}

fn author_one_shot_text(map: &ArgMap) -> Result<String, String> {
    if let Some(text) = value(map, "text").or_else(|| value(map, "request")) {
        return Ok(text.to_string());
    }
    if let Some(input) = value(map, "input") {
        return read_text_document(&normalize_path(input)?);
    }
    Err("author-one-shot requires --text, --request, or --input".to_string())
}

pub(crate) fn cmd_register_content(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let kind = require_value(&map, "kind")?;
    let id = require_value(&map, "id")?;
    let title = value(&map, "title").unwrap_or(&id);
    let effect_text = value(&map, "effect").unwrap_or("");
    let json = render_content_registry_json(&kind, &id, title, effect_text, "registered_from_args");
    write_or_print(&json, content_registry_output(&map).as_deref())
}

pub(crate) fn cmd_apply_author_transaction(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let kind = json_field(&text, "kind").unwrap_or_else(|| "unknown".to_string());
    let id = json_field(&text, "id").unwrap_or_else(|| "unknown".to_string());
    let title = json_field(&text, "title").unwrap_or_else(|| id.clone());
    let effect_text = json_field(&text, "effect_text").unwrap_or_default();
    let status = if map.flags.contains("execute") {
        "registry_written"
    } else {
        "plan_only"
    };
    let json = render_content_registry_json(&kind, &id, &title, &effect_text, status);
    write_or_print(&json, content_registry_output(&map).as_deref())
}

pub(crate) fn cmd_mod_transaction_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, Some(&mod_root), false)?;
    let input_path = value(&map, "input").map(normalize_path).transpose()?;
    let text = if let Some(text) = value(&map, "text").or_else(|| value(&map, "request")) {
        text.to_string()
    } else if let Some(input) = &input_path {
        read_text_document(input)?
    } else {
        String::new()
    };
    let child_plans = repeated_values(&map, "plan")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    if text.trim().is_empty() && child_plans.is_empty() {
        return Err("mod-transaction-plan requires --text, --input, or --plan".to_string());
    }
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !mod_root.exists() {
        blockers.push(format!("mod root `{}` does not exist", mod_root.display()));
    }
    if game_root.is_none() {
        blockers.push(
            "mod transaction requires --game-root for strict code-index evidence".to_string(),
        );
    }
    if mod_transaction_contains_raw_clausewitz(&text) {
        blockers.push("raw Clausewitz block detected; AI must submit structured intent or child plan, not final engine code".to_string());
        questions.push("Convert the raw code into structured fields such as system, id, title, effects, triggers, and changed files.".to_string());
    }
    let mut operation_json = Vec::new();
    let mut changed_files = BTreeSet::new();
    if !text.trim().is_empty() {
        let systems = mod_transaction_systems_from_text(&text, &map);
        for system in systems {
            let file = mod_transaction_default_changed_file(system, &map);
            changed_files.insert(file.clone());
            let operation_id = format!("op_{:03}", operation_json.len() + 1);
            operation_json.push(mod_transaction_operation_json(
                &operation_id,
                system,
                "direct_intent",
                "planned",
                &file,
                "input_text",
                "input_text",
                "medium",
                "system_writer",
                &mod_transaction_required_scopes(system),
                &mod_transaction_required_symbols(system),
                true,
                None,
            ));
        }
    }
    for plan in &child_plans {
        if !plan.exists() {
            blockers.push(format!("child plan `{}` does not exist", plan.display()));
            continue;
        }
        let plan_text = read_utf8_lossy(plan)?;
        let schema = json_field(&plan_text, "schema").unwrap_or_else(|| "unknown".to_string());
        let plan_ok =
            plan_text.contains("\"ok\": true") && !plan_text.contains("\"status\": \"blocked\"");
        let plan_changed_files = mod_transaction_json_string_array(&plan_text, "changed_files");
        let systems =
            mod_transaction_systems_from_child_plan(&schema, &plan_text, &plan_changed_files);
        if plan_changed_files.is_empty() {
            blockers.push(format!(
                "child plan `{}` has no changed_files evidence",
                plan.display()
            ));
        }
        for file in &plan_changed_files {
            changed_files.insert(file.clone());
        }
        if !plan_ok {
            blockers.push(format!("child plan `{}` is not ok", plan.display()));
        }
        for system in systems {
            let system_changed_files =
                mod_transaction_changed_files_for_system(system, &plan_changed_files);
            let operation_id = format!("op_{:03}", operation_json.len() + 1);
            operation_json.push(mod_transaction_operation_json(
                &operation_id,
                system,
                "child_plan",
                if plan_ok {
                    "planned"
                } else {
                    "blocked_child_plan"
                },
                &system_changed_files.join(";"),
                &schema,
                &plan.display().to_string(),
                "inherited",
                "child_plan_writer",
                &mod_transaction_required_scopes(system),
                &mod_transaction_required_symbols(system),
                plan_ok,
                (!plan_ok).then_some("child plan is not ok"),
            ));
        }
    }
    if operation_json.is_empty() {
        blockers.push("transaction has no operations".to_string());
    }
    if operation_json
        .iter()
        .any(|operation| operation.contains("\"source_evidence\": \"\""))
    {
        blockers.push("operation without source evidence is not allowed".to_string());
    }
    let changed_files = changed_files.into_iter().collect::<Vec<_>>();
    let dependency_edges = mod_transaction_dependency_edges(&changed_files);
    let rollback_records = mod_transaction_rollback_record_json_array(&changed_files);
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"transaction_stage\": {},\n  \"requires_atomic\": true,\n  \"mod_root\": {},\n  \"game_root\": {},\n  \"dependency_roots\": {},\n  \"intent_source\": {},\n  \"operation_count\": {},\n  \"operations\": [\n{}\n  ],\n  \"dependency_graph\": {},\n  \"changed_files\": {},\n  \"created_files\": [],\n  \"deleted_files\": [],\n  \"review_pack_contract\": {},\n  \"rollback_records\": [\n{}\n  ],\n  \"evidence\": {},\n  \"final_gates\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"next_commands\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.mod_transaction_plan.v1"),
        json_bool(ok),
        json_str(if ok { "mod_transaction_plan_ready" } else { "blocked" }),
        json_str("plan"),
        json_str(&mod_root.display().to_string()),
        json_optional_str(game_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        json_array(
            &dependency_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
        ),
        json_str(&mod_transaction_intent_source(input_path.as_deref(), !text.trim().is_empty(), !child_plans.is_empty())),
        operation_json.len(),
        operation_json.join(",\n"),
        json_array(&dependency_edges),
        json_array(&changed_files),
        mod_transaction_review_pack_contract_json(),
        rollback_records,
        json_array(&mod_transaction_evidence(&mod_root, game_root.as_deref(), &dependency_roots, input_path.as_deref(), &child_plans)),
        json_array(&[
            "validate --strict-code-index".to_string(),
            "check-text-alignment or validate --text-source when user-visible text is present".to_string(),
            "runtime/map/gui release gate when changed files touch those systems".to_string(),
            "writer-readiness-gate --execute --final-check --atomic".to_string(),
        ]),
        blockers.len(),
        json_array(&blockers),
        json_array(&questions),
        json_array(&["hoi4skill mod-transaction-apply --input <transaction.json> --execute --final-check --atomic --require-passed".to_string()]),
        json_array(&[
            "AI may provide structured intent or child plans only; raw Clausewitz engine code cannot bypass Rust writers".to_string(),
            "every operation must include source evidence and changed_files ownership".to_string(),
            "P71 transaction bus owns dependencies, rollback records, and atomic apply gates before any writer mutates files".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_mod_transaction_apply(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let plan = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !plan.contains("\"schema\": \"hoi4skill.mod_transaction_plan.v1\"") {
        blockers.push("input is not a mod-transaction-plan report".to_string());
    }
    if !plan.contains("\"ok\": true") {
        blockers.push("mod transaction plan is not ok".to_string());
    }
    if !map.flags.contains("execute") {
        blockers.push("mod-transaction-apply requires --execute".to_string());
    }
    if !map.flags.contains("final-check") {
        blockers.push("mod-transaction-apply requires --final-check".to_string());
    }
    if !map.flags.contains("atomic") {
        blockers.push("mod-transaction-apply requires --atomic".to_string());
    }
    let output_dir = value(&map, "output-dir")
        .map(normalize_path)
        .transpose()?
        .unwrap_or_else(|| PathBuf::from(".hoi4skill").join("mod_transaction_apply"));
    fs::create_dir_all(&output_dir).map_err(|e| format!("create {}: {e}", output_dir.display()))?;
    let changed_files = mod_transaction_json_string_array(&plan, "changed_files");
    let dependency_edges = mod_transaction_dependency_edges(&changed_files);
    let rollback_records = mod_transaction_rollback_record_json_array(&changed_files);
    let changed_path = output_dir.join("changed_files.txt");
    fs::write(&changed_path, changed_files.join("\n"))
        .map_err(|e| format!("write {}: {e}", changed_path.display()))?;
    let rollback_path = output_dir.join("rollback_plan.md");
    fs::write(
        &rollback_path,
        mod_transaction_rollback_markdown(&input, &changed_files),
    )
    .map_err(|e| format!("write {}: {e}", rollback_path.display()))?;
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"transaction_stage\": {},\n  \"input\": {},\n  \"execute\": {},\n  \"final_check\": {},\n  \"atomic\": {},\n  \"output_dir\": {},\n  \"dependency_graph\": {},\n  \"changed_files\": {},\n  \"created_files\": [],\n  \"deleted_files\": [],\n  \"changed_files_report\": {},\n  \"rollback_plan\": {},\n  \"review_pack\": {},\n  \"rollback_records\": [\n{}\n  ],\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.mod_transaction_apply.v1"),
        json_bool(ok),
        json_str(if ok { "mod_transaction_review_pack_ready" } else { "blocked" }),
        json_str("apply_preflight"),
        json_str(&input.display().to_string()),
        json_bool(map.flags.contains("execute")),
        json_bool(map.flags.contains("final-check")),
        json_bool(map.flags.contains("atomic")),
        json_str(&output_dir.display().to_string()),
        json_array(&dependency_edges),
        json_array(&changed_files),
        json_str(&changed_path.display().to_string()),
        json_str(&rollback_path.display().to_string()),
        mod_transaction_review_pack_json(&output_dir, &changed_path, &rollback_path),
        rollback_records,
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "P71 apply emits a review/rollback pack and requires execute/final-check/atomic before downstream writers mutate files".to_string(),
            "final release still requires strict validation, text alignment, runtime, GUI, map, and asset gates as applicable".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_writer_coverage_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let rows = writer_coverage_rows();
    let direct_missing = rows
        .iter()
        .filter(|row| row.status != "direct_writer" && row.status != "manual_gate")
        .map(|row| format!("{}:{}", row.system, row.command))
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    if map.flags.contains("require-direct-writers") && !direct_missing.is_empty() {
        blockers.push(format!(
            "systems without direct writers: {}",
            direct_missing.join(", ")
        ));
    }
    let ok = blockers.is_empty();
    let report = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"writer_count\": {},\n  \"writers\": [{}],\n  \"direct_writer_missing\": {},\n  \"blocking_count\": {},\n  \"blockers\": {},\n  \"rules\": {}\n}}\n",
        json_str("hoi4skill.writer_coverage_audit.v1"),
        json_bool(ok),
        json_str(if ok { "writer_coverage_report_ready" } else { "blocked" }),
        json_optional_str(mod_root.as_ref().map(|root| root.display().to_string()).as_deref()),
        rows.len(),
        rows.iter()
            .map(writer_coverage_row_json)
            .collect::<Vec<_>>()
            .join(", "),
        json_array(&direct_missing),
        blockers.len(),
        json_array(&blockers),
        json_array(&[
            "P52 is complete only when high-frequency systems have direct writers or an explicit review-pack-only policy".to_string(),
            "direct writers must still run through mod-transaction-apply --execute --final-check --atomic".to_string(),
            "review-pack writers are safe but do not count as final direct mutation coverage".to_string(),
        ])
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_code_template_recommend(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let kind = value(&map, "kind").unwrap_or("all");
    let templates = code_templates()
        .into_iter()
        .filter(|template| kind == "all" || template.kind == kind)
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"kind\": {},\n  \"template_count\": {},\n  \"templates\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.code_template_recommend.v1"),
        json_bool(!templates.is_empty()),
        json_str(if templates.is_empty() { "no_template" } else { "templates_ready" }),
        json_str(kind),
        templates.len(),
        render_code_templates(&templates),
        json_str("AI may choose among these template IDs; Rust assembly still validates symbols and scope before final code")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_assemble_code(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let kind = require_value(&map, "kind")?;
    if kind != "national_spirit" {
        return Err("P3 assemble-code currently supports --kind national_spirit".to_string());
    }
    let id = require_value(&map, "id")?;
    let picture = value(&map, "picture").unwrap_or("generic_political_reform");
    let modifier = require_value(&map, "modifier")?;
    let value_raw = require_value(&map, "value")?;
    let index = transaction_game_index(&map)?;
    let mut blockers = Vec::new();
    if !index.modifiers.contains(&modifier) {
        blockers.push(format!("modifier `{modifier}` is not registered"));
    }
    let scope = transaction_modifier_scope(&modifier);
    if !transaction_modifier_scope_compatible(scope, "national_spirit") {
        blockers.push(format!(
            "modifier `{modifier}` is `{scope}` scope and cannot be assembled into a national spirit"
        ));
    }
    let ok = blockers.is_empty();
    let code = if ok {
        format!(
            "ideas = {{\n  country = {{\n    {id} = {{\n      picture = {picture}\n      modifier = {{\n        {modifier} = {value_raw}\n      }}\n    }}\n  }}\n}}\n"
        )
    } else {
        String::new()
    };
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"kind\": {},\n  \"id\": {},\n  \"template\": {},\n  \"modifier\": {},\n  \"scope_class\": {},\n  \"blockers\": {},\n  \"code\": {}\n}}\n",
        json_str("hoi4skill.assemble_code.v1"),
        json_bool(ok),
        json_str(if ok { "assembled" } else { "blocked" }),
        json_str(&kind),
        json_str(&id),
        json_str("national_spirit_modifier"),
        json_str(&modifier),
        json_str(scope),
        json_array(&blockers),
        json_str(&code)
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct CodeTemplate {
    id: &'static str,
    kind: &'static str,
    required_fields: Vec<&'static str>,
    note: &'static str,
}

struct WriterCoverageRow {
    system: &'static str,
    command: &'static str,
    status: &'static str,
    mutation: &'static str,
    final_gate: &'static str,
}

fn writer_coverage_rows() -> Vec<WriterCoverageRow> {
    vec![
        WriterCoverageRow {
            system: "content_registry",
            command: "apply-author-transaction",
            status: "direct_writer",
            mutation: "registry_manifest",
            final_gate: "content_registry_review",
        },
        WriterCoverageRow {
            system: "history",
            command: "history-transaction-apply",
            status: "review_pack_writer",
            mutation: "patch_pack",
            final_gate: "validate --strict-code-index",
        },
        WriterCoverageRow {
            system: "state_history",
            command: "state-transaction-apply --write-overrides",
            status: "direct_writer",
            mutation: "target_state_override",
            final_gate: "validate --strict-code-index",
        },
        WriterCoverageRow {
            system: "map_supply",
            command: "supply-network-apply --write-overrides",
            status: "direct_writer",
            mutation: "target_network_override",
            final_gate: "map-release-gate",
        },
        WriterCoverageRow {
            system: "map_topology",
            command: "map-topology-gate",
            status: "manual_gate",
            mutation: "no_direct_write",
            final_gate: "manual-confirmed + map-release-gate",
        },
        WriterCoverageRow {
            system: "gui",
            command: "apply-gui-intent",
            status: "direct_writer",
            mutation: "gui_files_after_final_check",
            final_gate: "gui-runtime-evidence-contract",
        },
        WriterCoverageRow {
            system: "asset",
            command: "import-generated-icon",
            status: "direct_writer",
            mutation: "asset_manifest",
            final_gate: "validate asset references",
        },
        WriterCoverageRow {
            system: "localisation",
            command: "translate-localisation --apply",
            status: "direct_writer",
            mutation: "localisation_files",
            final_gate: "localisation-token-check",
        },
    ]
}

fn writer_coverage_row_json(row: &WriterCoverageRow) -> String {
    format!(
        "{{\"system\": {}, \"command\": {}, \"status\": {}, \"mutation\": {}, \"final_gate\": {}}}",
        json_str(row.system),
        json_str(row.command),
        json_str(row.status),
        json_str(row.mutation),
        json_str(row.final_gate)
    )
}

fn code_templates() -> Vec<CodeTemplate> {
    vec![
        CodeTemplate {
            id: "national_spirit_modifier",
            kind: "national_spirit",
            required_fields: vec!["id", "picture", "modifier", "value"],
            note: "country-scope idea modifier block",
        },
        CodeTemplate {
            id: "focus_completion_reward",
            kind: "focus",
            required_fields: vec!["id", "completion_reward"],
            note: "focus shell with completion_reward assembled from verified effects",
        },
        CodeTemplate {
            id: "country_event_option",
            kind: "event",
            required_fields: vec!["namespace", "id", "title", "desc", "option"],
            note: "country_event skeleton with registered localisation keys",
        },
        CodeTemplate {
            id: "iterator_limit_effect",
            kind: "scripted_effect",
            required_fields: vec!["iterator", "condition", "effect"],
            note: "iterator + limit + scoped effect shape",
        },
    ]
}

fn infer_content_kind(text: &str) -> &'static str {
    if text.contains("民族精神") {
        "national_spirit"
    } else if text.contains("事件") {
        "event"
    } else if text.contains("决议") {
        "decision"
    } else if text.contains("国策") {
        "focus"
    } else {
        "content"
    }
}

fn infer_content_title(text: &str, kind: &str) -> String {
    let marker = match kind {
        "national_spirit" => "民族精神",
        "event" => "事件",
        "decision" => "决议",
        "focus" => "国策",
        _ => "",
    };
    if marker.is_empty() {
        return "生成内容".to_string();
    }
    let after = text.split(marker).nth(1).unwrap_or(text);
    after
        .split(['：', ':', '\n', '，', ','])
        .find(|part| {
            let trimmed = part.trim();
            !trimmed.is_empty() && trimmed != "添加" && trimmed != "增加"
        })
        .map(|part| {
            part.trim()
                .trim_start_matches("添加")
                .trim_start_matches("增加")
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "生成内容".to_string())
}

fn infer_effect_text(text: &str) -> String {
    text.split("效果")
        .nth(1)
        .map(|part| part.trim_matches(['：', ':', '\n', ' ']).to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
}

fn render_author_transaction_json(kind: &str, id: &str, title: &str, effect_text: &str) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"kind\": {},\n  \"id\": {},\n  \"title\": {},\n  \"effect_text\": {},\n  \"operations\": {},\n  \"next_commands\": {}\n}}\n",
        json_str("hoi4skill.author_mod_intent.v1"),
        json_str("transaction_planned"),
        json_str(kind),
        json_str(id),
        json_str(title),
        json_str(effect_text),
        json_array(&[
            "register-content".to_string(),
            "code-template-recommend".to_string(),
            "assemble-code after symbol-registration-audit and scope-compat-audit pass".to_string(),
        ]),
        json_array(&[
            "hoi4skill apply-author-transaction --input <transaction.json> --execute --output .hoi4skill/content_registry.json".to_string(),
            "hoi4skill symbol-registration-audit --kind modifier --symbol <modifier> --require-passed".to_string(),
        ])
    )
}

fn mod_transaction_contains_raw_clausewitz(text: &str) -> bool {
    [
        "completion_reward = {",
        "ideas = {",
        "country_event = {",
        "namespace =",
        "option = {",
        "visible = {",
        "available = {",
        "effect = {",
        "state = {",
        "strategic_region = {",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn mod_transaction_systems_from_text(text: &str, map: &ArgMap) -> Vec<&'static str> {
    let mut systems = BTreeSet::new();
    for system in repeated_values(map, "system") {
        systems.insert(mod_transaction_normalize_system(system));
    }
    let lower = text.to_ascii_lowercase();
    for (needles, system) in [
        (&["国策", "focus"][..], "focus"),
        (&["事件", "event"], "event"),
        (&["民族精神", "idea", "national spirit"], "idea"),
        (&["决议", "decision"], "decision"),
        (&["gui", "界面", "窗口"], "gui"),
        (&["history", "开局", "oob", "科技"], "history"),
        (&["地图", "省份", "铁路", "补给", "战略区域", "map"], "map"),
        (&["图标", "国旗", "asset", "sprite"], "asset"),
        (
            &["本地化", "翻译", "localisation", "localization"],
            "localisation",
        ),
    ] {
        if needles
            .iter()
            .any(|needle| text.contains(needle) || lower.contains(&needle.to_ascii_lowercase()))
        {
            systems.insert(system);
        }
    }
    if systems.is_empty() {
        systems.insert("content");
    }
    systems.into_iter().collect()
}

fn mod_transaction_normalize_system(system: &str) -> &'static str {
    match system.to_ascii_lowercase().as_str() {
        "focus" | "national_focus" => "focus",
        "event" | "events" => "event",
        "idea" | "national_spirit" | "spirit" => "idea",
        "decision" | "decisions" => "decision",
        "gui" | "interface" => "gui",
        "history" | "oob" | "country_history" | "state_history" => "history",
        "map" | "province" | "state" => "map",
        "asset" | "sprite" | "flag" | "icon" => "asset",
        "localisation" | "localization" | "loc" => "localisation",
        _ => "content",
    }
}

fn mod_transaction_default_changed_file(system: &str, map: &ArgMap) -> String {
    let tag = value(map, "tag").unwrap_or("TAG");
    let prefix = value(map, "prefix").unwrap_or("generated");
    match system {
        "focus" => format!("common/national_focus/{tag}.txt"),
        "event" => format!("events/{prefix}_events.txt"),
        "idea" => format!("common/ideas/{prefix}_ideas.txt"),
        "decision" => format!("common/decisions/{prefix}_decisions.txt"),
        "gui" => format!("interface/{prefix}.gui"),
        "history" => format!("history/countries/{tag}.txt"),
        "map" => "history/states/<state>.txt".to_string(),
        "asset" => format!("interface/{prefix}_sprites.gfx"),
        "localisation" => format!("localisation/simp_chinese/{prefix}_l_simp_chinese.yml"),
        _ => format!(".hoi4skill/{prefix}_content.txt"),
    }
}

fn mod_transaction_system_from_schema(schema: &str) -> &'static str {
    if schema.contains("focus") {
        "focus"
    } else if schema.contains("event") || schema.contains("route") || schema.contains("on_action") {
        "event"
    } else if schema.contains("idea") || schema.contains("dynamic_modifier") {
        "idea"
    } else if schema.contains("decision") {
        "decision"
    } else if schema.contains("gui") {
        "gui"
    } else if schema.contains("history") || schema.contains("oob") || schema.contains("tech") {
        "history"
    } else if schema.contains("map")
        || schema.contains("province")
        || schema.contains("supply")
        || schema.contains("strategic_region")
    {
        "map"
    } else if schema.contains("asset") || schema.contains("icon") || schema.contains("flag") {
        "asset"
    } else if schema.contains("localisation") || schema.contains("localization") {
        "localisation"
    } else {
        "content"
    }
}

fn mod_transaction_systems_from_child_plan(
    schema: &str,
    text: &str,
    changed_files: &[String],
) -> Vec<&'static str> {
    let mut systems = BTreeSet::new();
    for system in mod_transaction_json_string_values(text, "system") {
        systems.insert(mod_transaction_normalize_system(&system));
    }
    for file in changed_files {
        systems.insert(mod_transaction_normalize_system(mod_transaction_file_lane(
            file,
        )));
    }
    if systems.is_empty() {
        systems.insert(mod_transaction_system_from_schema(schema));
    }
    systems.into_iter().collect()
}

fn mod_transaction_changed_files_for_system(system: &str, changed_files: &[String]) -> Vec<String> {
    let mut files = changed_files
        .iter()
        .filter(|file| mod_transaction_normalize_system(mod_transaction_file_lane(file)) == system)
        .cloned()
        .collect::<Vec<_>>();
    if files.is_empty() {
        files = changed_files.to_vec();
    }
    files
}

fn mod_transaction_operation_json(
    operation_id: &str,
    system: &str,
    intent_kind: &str,
    status: &str,
    changed_file: &str,
    source_kind: &str,
    source_evidence: &str,
    risk: &str,
    writer: &str,
    required_scopes: &[String],
    required_symbols: &[String],
    ok: bool,
    blocker: Option<&str>,
) -> String {
    format!(
        "    {{\"operation_id\": {}, \"lane\": {}, \"system\": {}, \"intent_kind\": {}, \"status\": {}, \"target_file\": {}, \"changed_file\": {}, \"source_kind\": {}, \"source_evidence\": {}, \"risk\": {}, \"writer\": {}, \"required_scopes\": {}, \"required_symbols\": {}, \"ok\": {}, \"blocker\": {}}}",
        json_str(operation_id),
        json_str(system),
        json_str(system),
        json_str(intent_kind),
        json_str(status),
        json_str(changed_file),
        json_str(changed_file),
        json_str(source_kind),
        json_str(source_evidence),
        json_str(risk),
        json_str(writer),
        json_array(required_scopes),
        json_array(required_symbols),
        json_bool(ok),
        json_optional_str(blocker)
    )
}

fn mod_transaction_required_scopes(system: &str) -> Vec<String> {
    match system {
        "focus" => vec!["focus".to_string(), "country_reward".to_string()],
        "event" => vec!["event".to_string(), "country_or_state_event".to_string()],
        "idea" => vec!["country".to_string(), "idea_container".to_string()],
        "decision" => vec!["country".to_string(), "decision_category".to_string()],
        "gui" => vec!["scripted_gui".to_string(), "interface_window".to_string()],
        "history" => vec![
            "country_history".to_string(),
            "state_history".to_string(),
            "oob".to_string(),
        ],
        "map" => vec![
            "state".to_string(),
            "province".to_string(),
            "map_file".to_string(),
        ],
        "asset" => vec!["gfx".to_string(), "sprite".to_string()],
        "localisation" => vec!["localisation_key".to_string(), "token".to_string()],
        _ => vec!["content".to_string()],
    }
}

fn mod_transaction_required_symbols(system: &str) -> Vec<String> {
    match system {
        "focus" => vec![
            "focus_id".to_string(),
            "effect".to_string(),
            "trigger".to_string(),
        ],
        "event" => vec![
            "event_id".to_string(),
            "effect".to_string(),
            "trigger".to_string(),
        ],
        "idea" => vec![
            "idea_id".to_string(),
            "modifier".to_string(),
            "sprite".to_string(),
        ],
        "decision" => vec![
            "decision_id".to_string(),
            "decision_category".to_string(),
            "effect".to_string(),
        ],
        "gui" => vec!["scripted_gui_id".to_string(), "window_name".to_string()],
        "history" => vec![
            "country_tag".to_string(),
            "state_id".to_string(),
            "province_id".to_string(),
            "technology".to_string(),
            "unit_type".to_string(),
        ],
        "map" => vec![
            "state_id".to_string(),
            "province_id".to_string(),
            "building".to_string(),
            "resource".to_string(),
        ],
        "asset" => vec!["sprite_id".to_string(), "texture_path".to_string()],
        "localisation" => vec!["localisation_key".to_string(), "scripted_token".to_string()],
        _ => vec!["content_id".to_string()],
    }
}

fn mod_transaction_dependency_edges(changed_files: &[String]) -> Vec<String> {
    let lanes = changed_files
        .iter()
        .map(|file| mod_transaction_file_lane(file))
        .collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    if lanes.contains("localisation")
        && lanes
            .iter()
            .any(|lane| ["focus", "event", "idea", "decision", "gui"].contains(lane))
    {
        edges.push("localisation -> content_text_refs".to_string());
    }
    if lanes.contains("asset")
        && lanes
            .iter()
            .any(|lane| ["focus", "idea", "decision", "event", "gui"].contains(lane))
    {
        edges.push("asset -> sprite_refs".to_string());
    }
    if lanes.contains("map") && lanes.contains("history") {
        edges.push("map/province evidence -> history/OOB placement".to_string());
    }
    if lanes.contains("gui") && lanes.contains("decision") {
        edges.push("decision route -> scripted_gui mount".to_string());
    }
    if lanes.contains("event") && lanes.contains("focus") {
        edges.push("focus completion_reward -> event trigger".to_string());
    }
    if edges.is_empty() {
        edges.push("single_lane_transaction".to_string());
    }
    edges
}

fn mod_transaction_file_lane(file: &str) -> &'static str {
    let normalized = file.replace('\\', "/").to_ascii_lowercase();
    if normalized.starts_with("common/national_focus/") {
        "focus"
    } else if normalized.starts_with("events/") {
        "event"
    } else if normalized.starts_with("common/ideas/") {
        "idea"
    } else if normalized.starts_with("common/decisions/") {
        "decision"
    } else if normalized.starts_with("gfx/")
        || normalized.contains("sprite")
        || normalized.contains("flag")
        || normalized.ends_with(".gfx")
    {
        "asset"
    } else if normalized.starts_with("interface/")
        || normalized.starts_with("common/scripted_guis/")
        || normalized.ends_with(".gui")
    {
        "gui"
    } else if normalized.starts_with("history/") {
        "history"
    } else if normalized.starts_with("map/") || normalized.starts_with("history/states/") {
        "map"
    } else if normalized.starts_with("localisation/") || normalized.starts_with("localization/") {
        "localisation"
    } else {
        "content"
    }
}

fn mod_transaction_json_string_values(text: &str, key: &str) -> Vec<String> {
    let marker = format!("\"{key}\":");
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find(&marker) {
        let value_start = offset + start + marker.len();
        let rest = text[value_start..].trim_start();
        if let Some(rest) = rest.strip_prefix('"') {
            let mut value = String::new();
            let mut escaped = false;
            for ch in rest.chars() {
                if escaped {
                    value.push(match ch {
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
                    out.push(value);
                    break;
                } else {
                    value.push(ch);
                }
            }
        }
        offset = value_start;
    }
    out.sort();
    out.dedup();
    out
}

fn mod_transaction_rollback_record_json_array(changed_files: &[String]) -> String {
    changed_files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            format!(
                "    {{\"id\": {}, \"target_file\": {}, \"strategy\": {}, \"required_before_write\": true}}",
                json_str(&format!("rollback_{:03}", index + 1)),
                json_str(file),
                json_str("backup_or_vcs_restore")
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

fn mod_transaction_review_pack_contract_json() -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "required_fields".to_string(),
        json_array(&[
            "operations".to_string(),
            "dependency_graph".to_string(),
            "changed_files".to_string(),
            "created_files".to_string(),
            "deleted_files".to_string(),
            "rollback_records".to_string(),
            "questions".to_string(),
            "blockers".to_string(),
            "final_gates".to_string(),
        ]),
    );
    map.insert(
        "apply_requirements".to_string(),
        json_array(&[
            "--execute".to_string(),
            "--final-check".to_string(),
            "--atomic".to_string(),
            "--require-passed".to_string(),
        ]),
    );
    map.insert(
        "writer_boundary".to_string(),
        json_str("writers consume ModTransaction operations and may not bypass review pack"),
    );
    json_raw_object(&map)
}

fn mod_transaction_review_pack_json(
    output_dir: &Path,
    changed_path: &Path,
    rollback_path: &Path,
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "output_dir".to_string(),
        json_str(&output_dir.display().to_string()),
    );
    map.insert(
        "changed_files_report".to_string(),
        json_str(&changed_path.display().to_string()),
    );
    map.insert(
        "rollback_plan".to_string(),
        json_str(&rollback_path.display().to_string()),
    );
    map.insert(
        "ready_for_downstream_writers".to_string(),
        json_bool(true).to_string(),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "review pack is generated before downstream writer mutation".to_string(),
            "rollback plan must be preserved with the apply report".to_string(),
            "changed_files are the only repair/apply surface for follow-up agents".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn mod_transaction_intent_source(
    input: Option<&Path>,
    has_text: bool,
    has_child_plans: bool,
) -> String {
    if let Some(input) = input {
        input
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("file")
            .to_string()
    } else if has_text {
        "text".to_string()
    } else if has_child_plans {
        "child_plans".to_string()
    } else {
        "unknown".to_string()
    }
}

fn mod_transaction_evidence(
    mod_root: &Path,
    game_root: Option<&Path>,
    dependency_roots: &[PathBuf],
    input_path: Option<&Path>,
    child_plans: &[PathBuf],
) -> Vec<String> {
    let mut out = vec![format!("target:{}", mod_root.display())];
    if let Some(game_root) = game_root {
        out.push(format!("game:{}", game_root.display()));
    }
    for root in dependency_roots {
        out.push(format!("parent:{}", root.display()));
    }
    if let Some(input_path) = input_path {
        out.push(format!("intent_source:{}", input_path.display()));
    }
    for child in child_plans {
        out.push(format!("child_plan:{}", child.display()));
    }
    out
}

fn mod_transaction_json_string_array(text: &str, key: &str) -> Vec<String> {
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

fn mod_transaction_rollback_markdown(input: &Path, changed_files: &[String]) -> String {
    let mut out = String::new();
    out.push_str("# Mod Transaction Rollback Plan\n\n");
    out.push_str(&format!("- input: `{}`\n", input.display()));
    out.push_str("- action: restore changed files from VCS or backup if any final gate fails.\n\n");
    out.push_str("## Changed Files\n\n");
    for file in changed_files {
        out.push_str(&format!("- `{file}`\n"));
    }
    out
}

fn render_author_one_shot_plan_json(
    request: &str,
    kind: &str,
    id: &str,
    title: &str,
    effect_text: &str,
    transaction: &str,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"direct_write\": false,\n  \"request\": {},\n  \"transaction_plan\": {},\n  \"registered_items\": [{{\"kind\": {}, \"id\": {}, \"title\": {}, \"effect_text\": {}}}],\n  \"questions\": [],\n  \"blockers\": [],\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.author_one_shot_plan.v1"),
        json_str("transaction_plan_ready"),
        json_str(request),
        transaction,
        json_str(kind),
        json_str(id),
        json_str(title),
        json_str(effect_text),
        json_array(&[
            "hoi4skill apply-author-transaction --input <plan.transaction_plan> --execute --final-check".to_string(),
            "hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index".to_string(),
        ]),
        json_str("author-one-shot never writes final game files; every ID must be registered and applied through reviewable transaction gates")
    )
}

fn render_content_registry_json(
    kind: &str,
    id: &str,
    title: &str,
    effect_text: &str,
    status: &str,
) -> String {
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"records\": [{{\"kind\": {}, \"id\": {}, \"title\": {}, \"effect_text\": {}}}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.content_registry.v1"),
        json_str(status),
        json_str(kind),
        json_str(id),
        json_str(title),
        json_str(effect_text),
        json_str("registered content may be referenced by later writers; unregistered IDs must not be assembled into final code")
    )
}

fn content_registry_output(map: &ArgMap) -> Option<String> {
    value(map, "output").map(str::to_string).or_else(|| {
        value(map, "mod-root").map(|root| {
            PathBuf::from(root)
                .join(".hoi4skill")
                .join("content_registry.json")
                .display()
                .to_string()
        })
    })
}

fn render_code_templates(templates: &[CodeTemplate]) -> String {
    templates
        .iter()
        .map(|template| {
            let fields = template
                .required_fields
                .iter()
                .map(|field| (*field).to_string())
                .collect::<Vec<_>>();
            format!(
                "{{\"id\": {}, \"kind\": {}, \"required_fields\": {}, \"note\": {}}}",
                json_str(template.id),
                json_str(template.kind),
                json_array(&fields),
                json_str(template.note)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn json_field(text: &str, field: &str) -> Option<String> {
    let pattern = format!("\"{field}\"");
    let pos = text.find(&pattern)?;
    parse_json_string_after_transaction_colon(&text[pos + pattern.len()..])
}

fn parse_json_string_after_transaction_colon(text: &str) -> Option<String> {
    let colon = text.find(':')?;
    let mut chars = text[colon + 1..].chars().peekable();
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
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

fn transaction_game_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn transaction_modifier_scope(modifier: &str) -> &'static str {
    let lower = modifier.to_ascii_lowercase();
    if lower.contains("mio") {
        "mio"
    } else if lower.contains("state") || lower.contains("resource") || lower.contains("local_") {
        "state"
    } else if lower.contains("political_power")
        || lower.contains("stability")
        || lower.contains("war_support")
        || lower.contains("justify_war_goal")
    {
        "country_tag"
    } else {
        "shared"
    }
}

fn transaction_modifier_scope_compatible(scope: &str, container: &str) -> bool {
    matches!(
        (scope, container),
        ("country_tag", "national_spirit") | ("shared", _)
    )
}
