//! Command-line routing for the binary.
//!
//! Command implementations still live in the crate root during the incremental
//! split, but routing is isolated here so new commands do not enlarge `main`.

use crate::error::{CliError, CliResult};

pub fn run(mut args: Vec<String>) -> CliResult<()> {
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
        super::print_usage();
        return Ok(());
    }

    let command = args.remove(0);
    let result = match command.as_str() {
        "scaffold" => super::cmd_scaffold(&args),
        "plan-large-mod" | "large-mod-blueprint" => super::cmd_plan_large_mod(&args),
        "init-large-mod" | "scaffold-large-mod" => super::cmd_init_large_mod(&args),
        "split-work-packages" | "large-mod-work-packages" => super::cmd_split_work_packages(&args),
        "generate-work-package" | "plan-work-package" | "work-package-plan" => {
            super::cmd_generate_work_package(&args)
        }
        "work-package-start-brief" | "package-start-brief" | "start-work-package" => {
            super::cmd_work_package_start_brief(&args)
        }
        "work-package-start-briefs" | "package-start-briefs" | "start-work-packages" => {
            super::cmd_work_package_start_briefs(&args)
        }
        "work-package-authoring-pack" | "authoring-pack" | "package-authoring-pack" => {
            super::cmd_work_package_authoring_pack(&args)
        }
        "work-package-claim" | "claim-work-package" | "package-claim" => {
            super::cmd_work_package_claim(&args)
        }
        "work-package-release-claim" | "release-work-package-claim" | "package-release-claim" => {
            super::cmd_work_package_release_claim(&args)
        }
        "work-package-claims" | "claims-work-packages" | "package-claims" => {
            super::cmd_work_package_claims(&args)
        }
        "work-package-dispatch-board" | "dispatch-board" | "package-dispatch-board" => {
            super::cmd_work_package_dispatch_board(&args)
        }
        "work-package-status" | "large-mod-status" | "package-status" => {
            super::cmd_work_package_status(&args)
        }
        "check-work-package-boundary" | "work-package-boundary" | "package-boundary" => {
            super::cmd_check_work_package_boundary(&args)
        }
        "large-mod-ci-plan" | "work-package-ci-plan" | "ci-plan-large-mod" => {
            super::cmd_large_mod_ci_plan(&args)
        }
        "large-mod-release-gate" | "release-gate" | "work-package-release-gate" => {
            super::cmd_large_mod_release_gate(&args)
        }
        "large-mod-dispatch-gate" | "dispatch-gate" | "work-package-dispatch-gate" => {
            super::cmd_large_mod_dispatch_gate(&args)
        }
        "identify-work-packages" | "changed-work-packages" | "affected-work-packages" => {
            super::cmd_identify_work_packages(&args)
        }
        "split-changed-work-packages" | "write-changed-work-packages" => {
            super::cmd_split_changed_work_packages(&args)
        }
        "work-package-readiness" | "package-readiness" | "large-mod-readiness" => {
            super::cmd_work_package_readiness(&args)
        }
        "work-package-handoff" | "package-handoff" | "handoff-work-package" => {
            super::cmd_work_package_handoff(&args)
        }
        "work-package-review-checklist" | "package-review-checklist" | "review-work-package" => {
            super::cmd_work_package_review_checklist(&args)
        }
        "work-package-merge-gate" | "package-merge-gate" | "merge-work-package" => {
            super::cmd_work_package_merge_gate(&args)
        }
        "work-package-merge-gates" | "package-merge-gates" | "merge-work-packages" => {
            super::cmd_work_package_merge_gates(&args)
        }
        "work-package-playtest-report" | "package-playtest-report" | "playtest-report" => {
            super::cmd_work_package_playtest_report(&args)
        }
        "large-mod-merge-gate" | "merge-gate" | "mod-merge-gate" => {
            super::cmd_large_mod_merge_gate(&args)
        }
        "large-mod-review-queue" | "review-queue" | "mod-review-queue" => {
            super::cmd_large_mod_review_queue(&args)
        }
        "large-mod-dashboard" | "mod-dashboard" | "work-package-dashboard" => {
            super::cmd_large_mod_dashboard(&args)
        }
        "large-mod-next-actions" | "next-actions" | "work-package-next-actions" => {
            super::cmd_large_mod_next_actions(&args)
        }
        "large-mod-production-snapshot" | "production-snapshot" | "mod-production-snapshot" => {
            super::cmd_large_mod_production_snapshot(&args)
        }
        "large-mod-production-brief" | "production-brief" | "mod-production-brief" => {
            super::cmd_large_mod_production_brief(&args)
        }
        "large-mod-fix-queue" | "fix-queue" | "repair-queue" => {
            super::cmd_large_mod_fix_queue(&args)
        }
        "large-mod-regression-plan" | "regression-plan" | "fix-regression-plan" => {
            super::cmd_large_mod_regression_plan(&args)
        }
        "large-mod-regression-gate" | "regression-gate" | "fix-regression-gate" => {
            super::cmd_large_mod_regression_gate(&args)
        }
        "large-mod-regression-brief" | "regression-brief" | "fix-regression-brief" => {
            super::cmd_large_mod_regression_brief(&args)
        }
        "large-mod-risk-register" | "risk-register" | "mod-risk-register" => {
            super::cmd_large_mod_risk_register(&args)
        }
        "large-mod-ownership-map" | "ownership-map" | "work-package-ownership-map" => {
            super::cmd_large_mod_ownership_map(&args)
        }
        "large-mod-dependency-graph" | "dependency-graph" | "work-package-dependency-graph" => {
            super::cmd_large_mod_dependency_graph(&args)
        }
        "large-mod-milestone-plan" | "milestone-plan" | "work-package-milestones" => {
            super::cmd_large_mod_milestone_plan(&args)
        }
        "large-mod-execution-queue" | "execution-queue" | "work-package-queue" => {
            super::cmd_large_mod_execution_queue(&args)
        }
        "large-mod-evidence-pack" | "evidence-pack" | "report-pack" => {
            super::cmd_large_mod_evidence_pack(&args)
        }
        "large-mod-review-brief" | "review-brief" | "release-review-brief" => {
            super::cmd_large_mod_review_brief(&args)
        }
        "large-mod-release-bundle" | "release-bundle" | "release-candidate-bundle" => {
            super::cmd_large_mod_release_bundle(&args)
        }
        "large-mod-release-brief" | "release-brief" | "release-candidate-brief" => {
            super::cmd_large_mod_release_brief(&args)
        }
        "large-mod-release-notes" | "release-notes" | "release-notes-draft" => {
            super::cmd_large_mod_release_notes(&args)
        }
        "large-mod-playtest-plan" | "playtest-plan" | "qa-plan" => {
            super::cmd_large_mod_playtest_plan(&args)
        }
        "large-mod-playtest-gate" | "playtest-gate" | "qa-gate" => {
            super::cmd_large_mod_playtest_gate(&args)
        }
        "large-mod-playtest-brief" | "playtest-brief" | "qa-brief" => {
            super::cmd_large_mod_playtest_brief(&args)
        }
        "validate" => super::cmd_validate(&args),
        "detect-hoi4-path" => super::cmd_detect_hoi4_path(&args),
        "build-game-index" => super::cmd_build_game_index(&args),
        "code-catalog" | "hoi4-code-catalog" | "build-code-catalog" => {
            super::cmd_code_catalog(&args)
        }
        "check-code-symbol" | "check-hoi4-code" | "classify-code-symbol" => {
            super::cmd_check_code_symbol(&args)
        }
        "compile-intent" | "compile-hoi4-intent" | "llm-intent" => super::cmd_compile_intent(&args),
        "check-text-alignment" | "text-alignment" | "check-output-text" => {
            super::cmd_check_text_alignment(&args)
        }
        "clausewitz-reference" | "code-reference" | "syntax-reference" => {
            super::cmd_clausewitz_reference(&args)
        }
        "build-clausewitz-library" | "build-code-library" => {
            super::cmd_build_clausewitz_library(&args)
        }
        "query-clausewitz-library" | "query-code-library" | "search-hoi4-code" => {
            super::cmd_query_clausewitz_library(&args)
        }
        "resolve-country-tag" | "country-tag-resolution" | "tag-resolution" => {
            super::cmd_resolve_country_tag(&args)
        }
        "scan-mod-style" => super::cmd_scan_mod_style(&args),
        "mod-knowledge" | "summarize-mod" | "mod-dossier" => super::cmd_mod_knowledge(&args),
        "build-mod-index" | "mod-index" => super::cmd_build_mod_index(&args),
        "query-symbol" | "find-symbol" | "symbol-info" => super::cmd_query_symbol(&args),
        "impact" | "impact-analysis" | "symbol-impact" => super::cmd_impact(&args),
        "reserve-id" | "reserve-ids" | "allocate-id" | "allocate-ids" => {
            super::cmd_reserve_id(&args)
        }
        "check-namespace" | "namespace-check" | "audit-namespace" => {
            super::cmd_check_namespace(&args)
        }
        "loc-audit" | "localisation-audit" | "localization-audit" => super::cmd_loc_audit(&args),
        "loc-sync-report" | "localisation-sync-report" | "localization-sync-report" => {
            super::cmd_loc_sync_report(&args)
        }
        "gfx-audit" | "sprite-audit" | "asset-audit" => super::cmd_gfx_audit(&args),
        "logic-audit" | "reachability-audit" | "focus-logic-audit" => super::cmd_logic_audit(&args),
        "asset-pack-plan" | "plan-assets" | "work-package-assets" => {
            super::cmd_asset_pack_plan(&args)
        }
        "feature-context" | "large-mod-context" | "work-package-context" => {
            super::cmd_feature_context(&args)
        }
        "prepare-edit-context" | "edit-context" | "preflight-context" => {
            super::cmd_prepare_edit_context(&args)
        }
        "plan-history-edit" | "history-edit-plan" | "plan-state-edit" => {
            super::cmd_plan_history_edit(&args)
        }
        "import-mod-ir" => super::cmd_import_mod_ir(&args),
        "doctor-skill-install" | "cleanup-old-skills" | "repair-skill-install" => {
            super::cmd_doctor_skill_install(&args)
        }
        "focus-copy-prompt" => super::cmd_focus_copy_prompt(&args),
        "idea-copy-prompt" | "national-spirit-copy-prompt" => super::cmd_idea_copy_prompt(&args),
        "icon-preview" => super::cmd_icon_preview(&args),
        "register-gfx-icons" | "register-icons" | "register-gfx" => {
            super::cmd_register_gfx_icons(&args)
        }
        "parse-focus-layout" => super::cmd_parse_focus_layout(&args),
        "render-focus-code" | "generate-focus-code" | "focus-template-code" => {
            super::cmd_render_focus_code(&args)
        }
        "apply-focus-layout" => super::cmd_apply_focus_layout(&args),
        "parse-focus-excel" | "parse-focus-xlsx" | "focus-excel-skeleton" => {
            super::cmd_parse_focus_excel(&args)
        }
        "apply-focus-excel" | "apply-focus-xlsx" => super::cmd_apply_focus_excel(&args),
        "parse-focus-copy-cards" => super::cmd_parse_focus_copy_cards(&args),
        "parse-feature-cards" => super::cmd_parse_feature_cards(&args),
        "apply-feature-cards" => super::cmd_apply_feature_cards(&args),
        "parse-event-cards" => super::cmd_parse_event_cards(&args),
        "apply-event-cards" => super::cmd_apply_event_cards(&args),
        "emit-hoi4yaml" => super::cmd_emit_hoi4yaml(&args),
        "run-workflow" => super::cmd_run_workflow(&args),
        "generate-mod" | "one-shot-mod" | "one-sentence-mod" => super::cmd_generate_mod(&args),
        "country-localisation-template" | "country-localization-template" => {
            super::cmd_country_localisation_template(&args)
        }
        "translate-localisation"
        | "translate-localization"
        | "quick-translate-localisation"
        | "quick-translate-localization" => super::cmd_translate_localisation(&args),
        "analyze-error-log" | "parse-error-log" => super::cmd_analyze_error_log(&args),
        _ => return Err(CliError::usage(format!("unknown command: {command}"))),
    };
    result.map_err(CliError::message)
}
