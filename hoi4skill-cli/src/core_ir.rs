//! Core IR contract report for large-mod authoring.
//!
//! This command is intentionally read-only. It gives CLI, tests, future desktop
//! UI, and agent integrations one explicit contract for the shared IR boundary.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_core_ir_contract(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let json = core_ir_contract_json();
    write_or_print(&json, value(&map, "output"))?;
    Ok(())
}

fn core_ir_contract_json() -> String {
    format!(
        concat!(
            "{{\n",
            "  \"schema\": \"hoi4skill.core_ir_contract.v1\",\n",
            "  \"ok\": true,\n",
            "  \"status\": \"core_ir_contract_ready\",\n",
            "  \"phase\": \"P81\",\n",
            "  \"ir_types\": [{}],\n",
            "  \"report_schemas\": {},\n",
            "  \"required_operation_fields\": {},\n",
            "  \"required_gate_fields\": {},\n",
            "  \"runtime_acceptance_layers\": {},\n",
            "  \"writer_invariants\": {},\n",
            "  \"ai_boundary\": {},\n",
            "  \"next_commands\": {}\n",
            "}}\n"
        ),
        core_ir_type_rows_json(),
        json_array(&[
            "hoi4skill.author_compiler_plan.v1".to_string(),
            "hoi4skill.mod_transaction_plan.v1".to_string(),
            "hoi4skill.scope_container_contract.v1".to_string(),
            "hoi4skill.writer_readiness_gate.v1".to_string(),
            "hoi4skill.runtime_evidence_gate.v1".to_string(),
            "hoi4skill.large_mod_release_gate.v1".to_string(),
        ]),
        json_array(&[
            "operation_id".to_string(),
            "lane".to_string(),
            "source_kind".to_string(),
            "source_ref".to_string(),
            "target_file".to_string(),
            "changed_files".to_string(),
            "required_scopes".to_string(),
            "required_symbols".to_string(),
            "dependencies".to_string(),
            "rollback_record".to_string(),
            "questions".to_string(),
        ]),
        json_array(&[
            "schema".to_string(),
            "ok".to_string(),
            "status".to_string(),
            "blockers".to_string(),
            "next_commands".to_string(),
            "stop_conditions".to_string(),
        ]),
        json_array(&[
            "prewrite_evidence_ready".to_string(),
            "code_ready".to_string(),
            "runtime_log_ready".to_string(),
            "playable_ready".to_string(),
        ]),
        json_array(&[
            "writers only consume ModTransaction operations".to_string(),
            "apply requires --execute --final-check --atomic".to_string(),
            "writers may only change declared changed_files".to_string(),
            "unknown symbols, stale evidence, wrong scope, wrong container, missing assets, and runtime blockers stop release".to_string(),
            "prewrite_evidence_ready must not be treated as playable_ready".to_string(),
        ]),
        json_str("AI may provide AuthorIntent, candidates, questions, and repair suggestions; Rust writers assemble final HOI4 files from indexed local evidence only."),
        json_array(&[
            "hoi4skill core-capability-audit --phase P81 --require-passed".to_string(),
            "hoi4skill author-compiler-plan --text <request> --game-root <hoi4> --mod-root <mod> --output .hoi4skill/author_plan.json".to_string(),
            "hoi4skill mod-transaction-plan --input .hoi4skill/author_plan.json --output .hoi4skill/transaction.json".to_string(),
        ])
    )
}

fn core_ir_type_rows_json() -> String {
    core_ir_type_rows()
        .iter()
        .map(|row| {
            format!(
                "{{\"name\": {}, \"role\": {}, \"producer\": {}, \"consumer\": {}, \"must_include_evidence\": {}}}",
                json_str(row.name),
                json_str(row.role),
                json_str(row.producer),
                json_str(row.consumer),
                json_bool(row.must_include_evidence)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

struct CoreIrTypeRow {
    name: &'static str,
    role: &'static str,
    producer: &'static str,
    consumer: &'static str,
    must_include_evidence: bool,
}

fn core_ir_type_rows() -> Vec<CoreIrTypeRow> {
    vec![
        CoreIrTypeRow {
            name: "AuthorIntent",
            role: "normalized user request, document row, or asset input",
            producer: "author-compiler-plan",
            consumer: "mod-transaction-plan",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "ModTransaction",
            role: "atomic dependency graph of operations and changed files",
            producer: "mod-transaction-plan",
            consumer: "writer-readiness-gate and mod-transaction-apply",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "Operation",
            role: "single focus/event/idea/history/map/gui/asset/localisation action",
            producer: "author compiler or schema-specific planner",
            consumer: "system writer",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "ScopeContract",
            role: "container and scope compatibility rules from local indexed code",
            producer: "scope-container-contract",
            consumer: "scope-compat-audit and symbol-registration-audit",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "WriterPlan",
            role: "declared writer policy, apply flags, target files, and rollback record",
            producer: "writer-readiness-gate",
            consumer: "mod-transaction-apply",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "RuntimeEvidence",
            role: "validation, error-log, route, GUI, and map runtime acceptance layer",
            producer: "runtime-evidence-gate",
            consumer: "large-mod-release-gate",
            must_include_evidence: true,
        },
        CoreIrTypeRow {
            name: "ReleaseManifest",
            role:
                "reproducible release summary with inputs, knowledge version, reports, and rollback",
            producer: "large-mod-release-gate",
            consumer: "large-mod-release-workflow",
            must_include_evidence: true,
        },
    ]
}
