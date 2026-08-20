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
        "adopt-large-mod" | "large-mod-adopt-existing" | "bootstrap-existing-large-mod" => {
            super::cmd_adopt_large_mod(&args)
        }
        "init-large-mod" | "scaffold-large-mod" => super::cmd_init_large_mod(&args),
        "split-work-packages" | "large-mod-work-packages" => super::cmd_split_work_packages(&args),
        "generate-work-package" | "plan-work-package" | "work-package-plan" => {
            super::cmd_generate_work_package(&args)
        }
        "run-work-package" | "large-mod-work-package-run" | "produce-work-package" => {
            super::cmd_run_work_package(&args)
        }
        "run-work-packages" | "large-mod-run-packages" | "execute-work-package-queue" => {
            super::cmd_run_work_packages(&args)
        }
        "ingest-work-package-request"
        | "work-package-ingest-request"
        | "fill-work-package-inputs" => super::cmd_ingest_work_package_request(&args),
        "author-work-package" | "work-package-author" | "produce-from-request" => {
            super::cmd_author_work_package(&args)
        }
        "author-work-packages" | "work-packages-author" | "produce-requests" => {
            super::cmd_author_work_packages(&args)
        }
        "apply-work-package-repair" | "work-package-repair-apply" | "apply-repair-inputs" => {
            super::cmd_apply_work_package_repair(&args)
        }
        "large-mod-apply-repair-queue" | "apply-repair-queue" | "batch-apply-repair-inputs" => {
            super::cmd_large_mod_apply_repair_queue(&args)
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
        "large-mod-run-summary" | "run-summary" | "work-package-run-summary" => {
            super::cmd_large_mod_run_summary(&args)
        }
        "large-mod-request-template" | "request-template" | "work-package-request-template" => {
            super::cmd_large_mod_request_template(&args)
        }
        "large-mod-authoring-gate" | "authoring-gate" | "mod-authoring-gate" => {
            super::cmd_large_mod_authoring_gate(&args)
        }
        "large-mod-authoring-bootstrap" | "authoring-bootstrap" | "mod-authoring-bootstrap" => {
            super::cmd_large_mod_authoring_bootstrap(&args)
        }
        "large-mod-author-queue-refresh" | "author-queue-refresh" | "refresh-author-queue" => {
            super::cmd_large_mod_author_queue_refresh(&args)
        }
        "large-mod-input-repair-queue" | "input-repair-queue" | "work-package-repair-queue" => {
            super::cmd_large_mod_input_repair_queue(&args)
        }
        "large-mod-repair-cycle" | "repair-cycle" | "batch-repair-cycle" => {
            super::cmd_large_mod_repair_cycle(&args)
        }
        "large-mod-context-evidence-brief" | "context-evidence-brief" | "repair-context-brief" => {
            super::cmd_large_mod_context_evidence_brief(&args)
        }
        "large-mod-context-evidence-answers"
        | "context-evidence-answers"
        | "repair-context-answers" => super::cmd_large_mod_context_evidence_answers(&args),
        "large-mod-context-evidence-verify"
        | "context-evidence-verify"
        | "repair-context-verify" => super::cmd_large_mod_context_evidence_verify(&args),
        "large-mod-next-actions" | "next-actions" | "work-package-next-actions" => {
            super::cmd_large_mod_next_actions(&args)
        }
        "large-mod-production-snapshot" | "production-snapshot" | "mod-production-snapshot" => {
            super::cmd_large_mod_production_snapshot(&args)
        }
        "large-mod-production-brief" | "production-brief" | "mod-production-brief" => {
            super::cmd_large_mod_production_brief(&args)
        }
        "large-mod-production-gate" | "production-gate" => {
            super::cmd_large_mod_production_gate(&args)
        }
        "large-mod-semi-auto-gate"
        | "large-mod-semiauto-gate"
        | "semi-auto-gate"
        | "semiauto-gate"
        | "semiauto-capability" => super::cmd_large_mod_semi_auto_gate(&args),
        "large-mod-production-evidence-refresh"
        | "production-evidence-refresh"
        | "refresh-production-evidence" => super::cmd_large_mod_production_evidence_refresh(&args),
        "large-mod-package-context-refresh"
        | "package-context-refresh"
        | "refresh-package-contexts" => super::cmd_large_mod_package_context_refresh(&args),
        "large-mod-ai-context-contract" | "ai-context-contract" | "mod-ai-context-contract" => {
            super::cmd_large_mod_ai_context_contract(&args)
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
        "large-mod-capability-audit" | "capability-audit" | "mod-capability-audit" => {
            super::cmd_large_mod_capability_audit(&args)
        }
        "core-capability-audit" | "phase-capability-audit" | "p0-capability-audit" => {
            super::cmd_core_capability_audit(&args)
        }
        "core-ir-contract" | "ir-contract" | "core-schema-contract" => {
            super::cmd_core_ir_contract(&args)
        }
        "overall-core-audit" | "core-completion-audit" | "large-mod-core-audit" => {
            super::cmd_overall_core_audit(&args)
        }
        "author-compiler-plan" | "compile-author-intent" | "author-compile-plan" => {
            super::cmd_author_compiler_plan(&args)
        }
        "common-coverage-audit" | "common-coverage" | "audit-common-coverage" => {
            super::cmd_common_coverage_audit(&args)
        }
        "large-mod-content-audit" | "content-audit" | "postwrite-content-audit" => {
            super::cmd_large_mod_content_audit(&args)
        }
        "large-mod-ai-output-insurance" | "ai-output-insurance" | "bad-ai-insurance" => {
            super::cmd_large_mod_ai_output_insurance(&args)
        }
        "lane-isolation-audit" | "work-package-lane-isolation" | "audit-lane-isolation" => {
            super::cmd_lane_isolation_audit(&args)
        }
        "large-mod-country-dry-run" | "country-dry-run" | "sample-country-package" => {
            super::cmd_large_mod_country_dry_run(&args)
        }
        "large-mod-sample-acceptance" | "sample-acceptance" | "country-sample-acceptance" => {
            super::cmd_large_mod_sample_acceptance(&args)
        }
        "large-mod-batch-acceptance" | "batch-acceptance" | "multi-package-acceptance" => {
            super::cmd_large_mod_batch_acceptance(&args)
        }
        "large-mod-capability-smoke" | "capability-smoke" | "large-mod-e2e-smoke" => {
            super::cmd_large_mod_capability_smoke(&args)
        }
        "large-mod-real-readiness-smoke"
        | "real-readiness-smoke"
        | "real-mod-smoke"
        | "kxc-readiness-smoke" => super::cmd_large_mod_real_readiness_smoke(&args),
        "large-mod-user-request-smoke" | "user-request-smoke" | "real-generation-smoke" => {
            super::cmd_large_mod_user_request_smoke(&args)
        }
        "large-mod-completion-audit" | "completion-audit" | "large-mod-goal-audit" => {
            super::cmd_large_mod_completion_audit(&args)
        }
        "large-mod-gui-pattern-audit" | "gui-pattern-audit" | "learn-gui-patterns" => {
            super::cmd_large_mod_gui_pattern_audit(&args)
        }
        "large-mod-gui-authoring-pack" | "gui-authoring-pack" | "gui-request-template" => {
            super::cmd_large_mod_gui_authoring_pack(&args)
        }
        "gui-style-template" | "large-mod-gui-style-template" | "parent-gui-style-template" => {
            super::cmd_gui_style_template(&args)
        }
        "gui-style-package-validate"
        | "large-mod-gui-style-package-validate"
        | "validate-gui-style-package" => super::cmd_gui_style_package_validate(&args),
        "gui-style-reuse-gate" | "large-mod-gui-style-reuse-gate" | "gui-style-gate" => {
            super::cmd_gui_style_reuse_gate(&args)
        }
        "gui-kind-template-gate"
        | "large-mod-gui-kind-template-gate"
        | "gui-template-kind-gate" => super::cmd_gui_kind_template_gate(&args),
        "gui-mount-audit" | "large-mod-gui-mount-audit" | "audit-gui-mounts" => {
            super::cmd_gui_mount_audit(&args)
        }
        "validate-gui-intent" | "large-mod-validate-gui-intent" | "gui-intent-validate" => {
            super::cmd_validate_gui_intent(&args)
        }
        "apply-gui-intent" | "large-mod-apply-gui-intent" | "write-gui-intent" => {
            super::cmd_apply_gui_intent(&args)
        }
        "gui-request-workflow" | "large-mod-gui-request-workflow" | "run-gui-request" => {
            super::cmd_gui_request_workflow(&args)
        }
        "gui-one-shot-gate" | "large-mod-gui-one-shot-gate" | "gui-one-shot" => {
            super::cmd_gui_one_shot_gate(&args)
        }
        "gui-objective-audit" | "large-mod-gui-objective-audit" | "gui-goal-audit" => {
            super::cmd_gui_objective_audit(&args)
        }
        "gui-output-audit" | "large-mod-gui-output-audit" | "audit-gui-output" => {
            super::cmd_gui_output_audit(&args)
        }
        "gui-layout-audit" | "audit-gui-layout" => super::cmd_gui_layout_audit(&args),
        "gui-error-regression" | "large-mod-gui-error-regression" | "gui-error-log-gate" => {
            super::cmd_gui_error_regression(&args)
        }
        "gui-visual-smoke" | "large-mod-gui-visual-smoke" | "gui-playable-smoke" => {
            super::cmd_gui_visual_smoke(&args)
        }
        "gui-game-smoke-plan" | "large-mod-gui-game-smoke-plan" | "gui-runtime-smoke-plan" => {
            super::cmd_gui_game_smoke_plan(&args)
        }
        "gui-runtime-launch-plan" | "large-mod-gui-runtime-launch-plan" | "gui-launch-plan" => {
            super::cmd_gui_runtime_launch_plan(&args)
        }
        "gui-runtime-open-plan" | "large-mod-gui-runtime-open-plan" | "gui-open-plan" => {
            super::cmd_gui_runtime_open_plan(&args)
        }
        "gui-runtime-session-plan" | "large-mod-gui-runtime-session-plan" | "gui-session-plan" => {
            super::cmd_gui_runtime_session_plan(&args)
        }
        "gui-runtime-runner" | "large-mod-gui-runtime-runner" | "gui-launch-runner" => {
            super::cmd_gui_runtime_runner(&args)
        }
        "gui-runtime-smoke-executor"
        | "large-mod-gui-runtime-smoke-executor"
        | "gui-smoke-executor" => super::cmd_gui_runtime_smoke_executor(&args),
        "gui-runtime-automation-pack"
        | "large-mod-gui-runtime-automation-pack"
        | "gui-playtest-automation-pack" => super::cmd_gui_runtime_automation_pack(&args),
        "gui-runtime-window-probe" | "large-mod-gui-runtime-window-probe" | "gui-window-probe" => {
            super::cmd_gui_runtime_window_probe(&args)
        }
        "gui-runtime-window-capture-plan"
        | "large-mod-gui-runtime-window-capture-plan"
        | "gui-window-capture-plan" => super::cmd_gui_runtime_window_capture_plan(&args),
        "gui-runtime-log-probe" | "large-mod-gui-runtime-log-probe" | "gui-log-probe" => {
            super::cmd_gui_runtime_log_probe(&args)
        }
        "gui-runtime-screenshot" | "large-mod-gui-runtime-screenshot" | "gui-screenshot-check" => {
            super::cmd_gui_runtime_screenshot(&args)
        }
        "gui-runtime-visual-probe" | "large-mod-gui-runtime-visual-probe" | "gui-visual-probe" => {
            super::cmd_gui_runtime_visual_probe(&args)
        }
        "gui-runtime-visual-matrix"
        | "large-mod-gui-runtime-visual-matrix"
        | "gui-visual-matrix" => super::cmd_gui_runtime_visual_matrix(&args),
        "gui-runtime-text-fit-matrix"
        | "large-mod-gui-runtime-text-fit-matrix"
        | "gui-text-fit-matrix" => super::cmd_gui_runtime_text_fit_matrix(&args),
        "gui-runtime-style-match" | "large-mod-gui-runtime-style-match" | "gui-style-match" => {
            super::cmd_gui_runtime_style_match(&args)
        }
        "gui-runtime-layout-probe" | "large-mod-gui-runtime-layout-probe" | "gui-layout-probe" => {
            super::cmd_gui_runtime_layout_probe(&args)
        }
        "gui-runtime-click-plan" | "large-mod-gui-runtime-click-plan" | "gui-click-plan" => {
            super::cmd_gui_runtime_click_plan(&args)
        }
        "gui-runtime-pixel-probe" | "large-mod-gui-runtime-pixel-probe" | "gui-pixel-probe" => {
            super::cmd_gui_runtime_pixel_probe(&args)
        }
        "gui-runtime-click-probe" | "large-mod-gui-runtime-click-probe" | "gui-click-probe" => {
            super::cmd_gui_runtime_click_probe(&args)
        }
        "gui-runtime-state-probe" | "large-mod-gui-runtime-state-probe" | "gui-state-probe" => {
            super::cmd_gui_runtime_state_probe(&args)
        }
        "gui-runtime-evidence-bundle"
        | "large-mod-gui-runtime-evidence-bundle"
        | "gui-evidence-bundle" => super::cmd_gui_runtime_evidence_bundle(&args),
        "gui-runtime-evidence-contract"
        | "large-mod-gui-runtime-evidence-contract"
        | "gui-playable-evidence-contract" => super::cmd_gui_runtime_evidence_contract(&args),
        "gui-runtime-evidence-collect"
        | "large-mod-gui-runtime-evidence-collect"
        | "gui-collect-runtime-evidence" => super::cmd_gui_runtime_evidence_collect(&args),
        "gui-runtime-evidence-manifest"
        | "large-mod-gui-runtime-evidence-manifest"
        | "gui-evidence-manifest" => super::cmd_gui_runtime_evidence_manifest(&args),
        "gui-runtime-evidence-runbook"
        | "large-mod-gui-runtime-evidence-runbook"
        | "gui-runtime-runbook" => super::cmd_gui_runtime_evidence_runbook(&args),
        "gui-runtime-evidence-timeline"
        | "large-mod-gui-runtime-evidence-timeline"
        | "gui-runtime-timeline" => super::cmd_gui_runtime_evidence_timeline(&args),
        "gui-runtime-one-shot-gate"
        | "large-mod-gui-runtime-one-shot-gate"
        | "gui-playability-one-shot-gate" => super::cmd_gui_runtime_one_shot_gate(&args),
        "gui-delivery-report" | "large-mod-gui-delivery-report" | "gui-final-delivery" => {
            super::cmd_gui_delivery_report(&args)
        }
        "gui-game-smoke-report"
        | "large-mod-gui-game-smoke-report"
        | "gui-runtime-smoke-report" => super::cmd_gui_game_smoke_report(&args),
        "gui-playability-gate" | "large-mod-gui-playability-gate" | "gui-release-gate" => {
            super::cmd_gui_playability_gate(&args)
        }
        "ui-cosmetic-common-plan" | "p21-ui-cosmetic-plan" | "cosmetic-common-plan" => {
            super::cmd_ui_cosmetic_common_plan(&args)
        }
        "ui-cosmetic-common-apply" | "p21-ui-cosmetic-apply" | "cosmetic-common-apply" => {
            super::cmd_ui_cosmetic_common_apply(&args)
        }
        "gui-goal-gate" | "large-mod-gui-goal-gate" | "gui-generation-goal-gate" => {
            super::cmd_gui_goal_gate(&args)
        }
        "gui-blocked-context-gate"
        | "large-mod-gui-blocked-context-gate"
        | "gui-repair-context-gate" => super::cmd_gui_blocked_context_gate(&args),
        "gui-unknown-blockers-gate"
        | "large-mod-gui-unknown-blockers-gate"
        | "gui-unknown-input-gate" => super::cmd_gui_unknown_blockers_gate(&args),
        "gui-repair-loop" | "large-mod-gui-repair-loop" | "gui-ai-repair-loop" => {
            super::cmd_gui_repair_loop(&args)
        }
        "gui-syntax-safety-gate" | "large-mod-gui-syntax-safety-gate" | "gui-raw-syntax-gate" => {
            super::cmd_gui_syntax_safety_gate(&args)
        }
        "gui-text-alignment-gate" | "large-mod-gui-text-alignment-gate" | "gui-text-gate" => {
            super::cmd_gui_text_alignment_gate(&args)
        }
        "gui-resource-resolution-gate"
        | "large-mod-gui-resource-resolution-gate"
        | "gui-resource-gate" => super::cmd_gui_resource_resolution_gate(&args),
        "validate" => super::cmd_validate(&args),
        "validate-repair-context" | "repair-context" | "ai-repair-context" => {
            super::cmd_validate_repair_context(&args)
        }
        "validation-baseline" | "write-validation-baseline" | "baseline-validation" => {
            super::cmd_validation_baseline(&args)
        }
        "detect-hoi4-path" => super::cmd_detect_hoi4_path(&args),
        "build-game-index" => super::cmd_build_game_index(&args),
        "code-catalog" | "hoi4-code-catalog" | "build-code-catalog" => {
            super::cmd_code_catalog(&args)
        }
        "documentation-catalog" | "documentation-query" | "search-documentation" => {
            super::cmd_documentation_catalog(&args)
        }
        "check-code-symbol" | "check-hoi4-code" | "classify-code-symbol" => {
            super::cmd_check_code_symbol(&args)
        }
        "compile-intent" | "compile-hoi4-intent" | "llm-intent" => super::cmd_compile_intent(&args),
        "author-intent" | "apply-author-intent" | "execute-author-intent" => {
            super::cmd_author_intent(&args)
        }
        "author-intent-workflow"
        | "write-author-intent"
        | "semi-auto-author-intent"
        | "semiauto-author-intent" => super::cmd_author_intent_workflow(&args),
        "large-mod-author-workflow"
        | "semi-auto-large-mod"
        | "semiauto-large-mod"
        | "author-large-mod" => super::cmd_large_mod_author_workflow(&args),
        "large-mod-production-workflow"
        | "large-mod-produce"
        | "produce-large-mod"
        | "semiauto-produce-large-mod" => super::cmd_large_mod_production_workflow(&args),
        "large-mod-repair-workflow"
        | "repair-large-mod"
        | "large-mod-repair-and-rerun"
        | "semiauto-repair-large-mod" => super::cmd_large_mod_repair_workflow(&args),
        "large-mod-build-workflow"
        | "build-large-mod"
        | "large-mod-full-workflow"
        | "semiauto-build-large-mod" => super::cmd_large_mod_build_workflow(&args),
        "large-mod-build-acceptance" | "build-acceptance" | "large-mod-acceptance" => {
            super::cmd_large_mod_build_acceptance(&args)
        }
        "author-intent-plan" | "plan-author-intent" | "route-author-intent" => {
            super::cmd_author_intent_plan(&args)
        }
        "plan-dynamic-modifier-change"
        | "dynamic-modifier-plan"
        | "compile-dynamic-modifier-change" => super::cmd_plan_dynamic_modifier_change(&args),
        "apply-intent-patch-plan" | "apply-intent" | "apply-compiled-intent" => {
            super::cmd_apply_intent_patch_plan(&args)
        }
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
        "resolve-mod-dependencies" | "mod-dependencies" | "dependency-resolution" => {
            super::cmd_resolve_mod_dependencies(&args)
        }
        "scan-mod-style" => super::cmd_scan_mod_style(&args),
        "mod-knowledge" | "summarize-mod" | "mod-dossier" => super::cmd_mod_knowledge(&args),
        "knowledge-base-refresh"
        | "knowledge-delta-refresh"
        | "kb-refresh"
        | "refresh-knowledge-base" => super::cmd_knowledge_base_refresh(&args),
        "knowledge-template-summarize"
        | "template-summarize"
        | "summarize-templates"
        | "writing-style-profile"
        | "event-prose-plan" => super::cmd_knowledge_template_summarize(&args),
        "evidence-db-audit" | "local-evidence-db-audit" | "knowledge-evidence-audit" => {
            super::cmd_evidence_db_audit(&args)
        }
        "knowledge-compatibility-plan"
        | "compatibility-refresh-plan"
        | "incremental-knowledge-plan"
        | "steamdb-compat-plan" => super::cmd_knowledge_compatibility_plan(&args),
        "symbol-registration-audit" | "registered-symbol-audit" | "symbol-audit" => {
            super::cmd_symbol_registration_audit(&args)
        }
        "scope-container-contract" | "scope-contract" => super::cmd_scope_container_contract(&args),
        "condition-plan" | "trigger-plan" | "precondition-plan" => super::cmd_condition_plan(&args),
        "modifier-scope-catalog" | "scope-catalog" => super::cmd_modifier_scope_catalog(&args),
        "scope-compat-audit" | "modifier-scope-audit" => super::cmd_scope_compat_audit(&args),
        "iterator-effect-plan" | "conditional-effect-plan" => {
            super::cmd_iterator_effect_plan(&args)
        }
        "iterator-scope-audit" | "iterator-audit" => super::cmd_iterator_scope_audit(&args),
        "weak-ai-regression-suite" | "weak-ai-suite" | "bad-ai-regression-suite" => {
            super::cmd_weak_ai_regression_suite(&args)
        }
        "semantic-repair-search" | "semantic-symbol-repair" | "repair-search" => {
            super::cmd_semantic_repair_search(&args)
        }
        "author-mod-intent" | "mod-intent" | "plan-mod-intent" => {
            super::cmd_author_mod_intent(&args)
        }
        "author-one-shot" | "one-shot-author" | "plan-one-shot" => {
            super::cmd_author_one_shot(&args)
        }
        "mod-transaction-plan" | "plan-mod-transaction" | "transaction-plan" => {
            super::cmd_mod_transaction_plan(&args)
        }
        "mod-transaction-apply" | "apply-mod-transaction" | "transaction-apply" => {
            super::cmd_mod_transaction_apply(&args)
        }
        "writer-coverage-audit" | "audit-writer-coverage" => {
            super::cmd_writer_coverage_audit(&args)
        }
        "writer-readiness-gate" | "writer-apply-gate" => super::cmd_writer_readiness_gate(&args),
        "apply-author-transaction" => super::cmd_apply_author_transaction(&args),
        "register-content" | "content-register" => super::cmd_register_content(&args),
        "code-template-recommend" | "recommend-code-template" => {
            super::cmd_code_template_recommend(&args)
        }
        "assemble-code" | "code-assemble" => super::cmd_assemble_code(&args),
        "event-chain-graph" | "event-graph" => super::cmd_event_chain_graph(&args),
        "event-chain-author-plan" | "route-logic-graph" | "event-route-author-plan" => {
            super::cmd_event_chain_author_plan(&args)
        }
        "trigger-source-graph" | "event-trigger-sources" => super::cmd_trigger_source_graph(&args),
        "on-action-graph" | "on-actions-graph" => super::cmd_on_action_graph(&args),
        "on-action-insert-plan" | "plan-on-action-insert" | "event-entry-plan" => {
            super::cmd_on_action_insert_plan(&args)
        }
        "common-writer-registry" | "common-system-catalog" | "common-writer-catalog" => {
            super::cmd_common_writer_registry(&args)
        }
        "common-writer-plan" | "common-definition-plan" => super::cmd_common_writer_plan(&args),
        "common-writer-apply" | "common-definition-apply" => super::cmd_common_writer_apply(&args),
        "scripted-localisation-plan" | "scripted-localization-plan" | "scripted-loc-plan" => {
            super::cmd_scripted_localisation_plan(&args)
        }
        "opinion-modifier-plan" | "opinion-plan" => super::cmd_opinion_modifier_plan(&args),
        "game-rule-plan" | "gamerule-plan" => super::cmd_game_rule_plan(&args),
        "bookmark-plan" => super::cmd_bookmark_plan(&args),
        "bop-plan" | "balance-of-power-plan" => super::cmd_bop_plan(&args),
        "ai-strategy-plan-file" | "ai-strategy-file-plan" => {
            super::cmd_ai_strategy_plan_file(&args)
        }
        "system-pack-plan" | "p18-system-pack-plan" => super::cmd_system_pack_plan(&args),
        "intelligence-system-pack-plan" | "operations-system-pack-plan" => {
            super::cmd_intelligence_system_pack_plan(&args)
        }
        "ai-behavior-system-pack-plan" | "ai-system-pack-plan" => {
            super::cmd_ai_behavior_system_pack_plan(&args)
        }
        "technology-depth-system-pack-plan" | "tech-depth-system-pack-plan" => {
            super::cmd_technology_depth_system_pack_plan(&args)
        }
        "occupation-resistance-system-pack-plan" | "occupation-system-pack-plan" => {
            super::cmd_occupation_resistance_system_pack_plan(&args)
        }
        "system-pack-apply" | "p18-system-pack-apply" => super::cmd_system_pack_apply(&args),
        "dead-event-audit" | "dead-events" => super::cmd_dead_event_audit(&args),
        "route-blocker-audit" | "why-event-not-triggering" => super::cmd_route_blocker_audit(&args),
        "transaction-route-plan" | "route-transaction-plan" | "route-release-gate" => {
            super::cmd_transaction_route_plan(&args)
        }
        "route-guide" | "event-route-guide" | "gameplay-route-guide" => {
            super::cmd_route_guide(&args)
        }
        "icon-generate-plan" | "plan-icon-generation" => super::cmd_icon_generate_plan(&args),
        "import-generated-icon" | "generated-icon-import" => {
            super::cmd_import_generated_icon(&args)
        }
        "focus-ideation-plan" | "focus-sketch" => super::cmd_focus_ideation_plan(&args),
        "export-mod" | "export-mod-plan" | "export-mod-apply" | "mod-export-plan" => {
            super::cmd_export_mod(&args)
        }
        "state-batch-plan" | "batch-state-plan" => super::cmd_state_batch_plan(&args),
        "state-batch-apply" | "batch-state-apply" => super::cmd_state_batch_apply(&args),
        "oob-plan" | "initial-units-plan" => super::cmd_oob_plan(&args),
        "oob-relocation-plan" | "history-units-relocation-plan" | "plan-oob-relocation" => {
            super::cmd_oob_relocation_plan(&args)
        }
        "oob-relocation-apply" | "history-units-relocation-apply" | "apply-oob-relocation" => {
            super::cmd_oob_relocation_apply(&args)
        }
        "unit-taxonomy-build" | "build-unit-taxonomy" | "p24-unit-taxonomy-build" => {
            super::cmd_unit_taxonomy_build(&args)
        }
        "unit-intent-classify" | "classify-unit-intent" | "p24-unit-intent-classify" => {
            super::cmd_unit_intent_classify(&args)
        }
        "unit-taxonomy-audit" | "audit-unit-taxonomy" | "p24-unit-taxonomy-audit" => {
            super::cmd_unit_taxonomy_audit(&args)
        }
        "oob-template-resolve" | "resolve-oob-template" | "unit-oob-resolve" => {
            super::cmd_oob_template_resolve(&args)
        }
        "division-template-plan" | "plan-division-template" | "p25-division-template-plan" => {
            super::cmd_division_template_plan(&args)
        }
        "division-template-apply" | "apply-division-template" | "p25-division-template-apply" => {
            super::cmd_division_template_apply(&args)
        }
        "division-template-audit" | "audit-division-template" | "p25-division-template-audit" => {
            super::cmd_division_template_audit(&args)
        }
        "oob-kind-classify" | "classify-oob-kind" | "p26-oob-kind-classify" => {
            super::cmd_oob_kind_classify(&args)
        }
        "air-oob-plan" | "plan-air-oob" | "p26-air-oob-plan" => super::cmd_air_oob_plan(&args),
        "naval-oob-plan" | "plan-naval-oob" | "p26-naval-oob-plan" => {
            super::cmd_naval_oob_plan(&args)
        }
        "oob-kind-apply" | "apply-oob-kind" | "p26-oob-kind-apply" => {
            super::cmd_oob_kind_apply(&args)
        }
        "tech-equipment-plan" | "technology-equipment-plan" => {
            super::cmd_tech_equipment_plan(&args)
        }
        "history-country-plan" | "country-history-plan" => super::cmd_history_country_plan(&args),
        "history-scenario-plan" | "start-history-scenario-plan" | "history-start-plan" => {
            super::cmd_history_scenario_plan(&args)
        }
        "history-scenario-apply" | "apply-history-scenario" | "history-start-apply" => {
            super::cmd_history_scenario_apply(&args)
        }
        "scenario-compiler-plan"
        | "startdate-scenario-compiler"
        | "history-transaction-plan"
        | "start-history-transaction-plan" => super::cmd_history_transaction_plan(&args),
        "history-transaction-apply" | "apply-history-transaction" => {
            super::cmd_history_transaction_apply(&args)
        }
        "history-transaction-audit" | "audit-history-transaction" => {
            super::cmd_history_transaction_audit(&args)
        }
        "history-startdate-gate" | "history-scenario-gate" | "startdate-history-gate" => {
            super::cmd_history_startdate_gate(&args)
        }
        "startdate-closure-plan" | "history-startdate-closure-plan" | "scenario-closure-plan" => {
            super::cmd_startdate_closure_plan(&args)
        }
        "ambiguity-report" | "intent-ambiguity-report" | "question-ambiguity" => {
            super::cmd_ambiguity_report(&args)
        }
        "answer-ambiguity" | "resolve-ambiguity" | "ambiguity-answer" => {
            super::cmd_answer_ambiguity(&args)
        }
        "ambiguity-gate" | "confirm-ambiguity-gate" | "resolved-intent-gate" => {
            super::cmd_ambiguity_gate(&args)
        }
        "parent-oob-compat-smoke" | "parent-oob-smoke" | "oob-parent-compat-smoke" => {
            super::cmd_parent_oob_compat_smoke(&args)
        }
        "parent-history-compat-smoke" | "parent-history-smoke" | "history-parent-compat-smoke" => {
            super::cmd_parent_history_compat_smoke(&args)
        }
        "parent-compat-release-gate" | "parent-compat-gate" | "parent-smoke-release-gate" => {
            super::cmd_parent_compat_release_gate(&args)
        }
        "mio-intent-plan" | "mio-plan" => super::cmd_mio_intent_plan(&args),
        "tech-scope-audit" | "technology-scope-audit" => super::cmd_tech_scope_audit(&args),
        "equipment-scope-audit" => super::cmd_equipment_scope_audit(&args),
        "modifier-family-catalog" | "modifier-family" => super::cmd_modifier_family_catalog(&args),
        "parent-mod-diff-plan" | "parent-diff-plan" => super::cmd_parent_mod_diff_plan(&args),
        "override-risk-audit" | "parent-override-audit" => super::cmd_override_risk_audit(&args),
        "dependency-freshness-check" | "freshness-check" => {
            super::cmd_dependency_freshness_check(&args)
        }
        "stale-template-audit" | "template-stale-audit" => super::cmd_stale_template_audit(&args),
        "stale-plan-gate" | "plan-stale-gate" => super::cmd_stale_plan_gate(&args),
        "runtime-error-baseline" | "error-baseline" => super::cmd_runtime_error_baseline(&args),
        "runtime-error-regression" | "error-regression" => {
            super::cmd_runtime_error_regression(&args)
        }
        "runtime-evidence-gate" | "runtime-evidence" | "runtime-acceptance-gate" => {
            super::cmd_runtime_evidence_gate(&args)
        }
        "runtime-release-gate"
        | "runtime-log-release-gate"
        | "release-runtime-gate"
        | "p109-runtime-release-gate" => super::cmd_runtime_release_gate(&args),
        "playable-smoke-plan" | "playability-smoke-plan" => super::cmd_playable_smoke_plan(&args),
        "playable-acceptance-gate" | "playability-gate" => {
            super::cmd_playable_acceptance_gate(&args)
        }
        "common-release-gate" | "p22-common-release-gate" => super::cmd_common_release_gate(&args),
        "console-command-help" | "console-help" => super::cmd_console_command_help(&args),
        "gameplay-guide" | "route-gameplay-guide" => super::cmd_gameplay_guide(&args),
        "ideology-intent-plan" | "plan-ideology-intent" => super::cmd_ideology_intent_plan(&args),
        "ideology-batch-copy-plan" | "ideology-copy-plan" => {
            super::cmd_ideology_batch_copy_plan(&args)
        }
        "politics-intent-plan" | "politics-plan" => super::cmd_politics_intent_plan(&args),
        "party-popularity-plan" | "party-plan" => super::cmd_party_popularity_plan(&args),
        "cosmetic-tag-batch-plan" | "cosmetic-batch-plan" => {
            super::cmd_cosmetic_tag_batch_plan(&args)
        }
        "cosmetic-transition-plan" | "cosmetic-transition" => {
            super::cmd_cosmetic_transition_plan(&args)
        }
        "flag-copy-plan" | "cosmetic-flag-copy-plan" => super::cmd_flag_copy_plan(&args),
        "flag-image-import"
        | "import-flag-image"
        | "flag-triplet-import"
        | "asset-import-apply"
        | "flag-triplet-build" => super::cmd_flag_image_import(&args),
        "asset-import-plan" | "asset-plan" | "p107-asset-import-plan" => {
            super::cmd_asset_import_plan(&args)
        }
        "country-name-batch-plan" | "cosmetic-name-plan" => {
            super::cmd_country_name_batch_plan(&args)
        }
        "formation-chain-plan" | "country-formation-plan" => super::cmd_formation_chain_plan(&args),
        "country-setup-plan" | "new-country-plan" | "start-country-plan" => {
            super::cmd_country_setup_plan(&args)
        }
        "country-setup-apply" | "new-country-apply" | "apply-country-setup" => {
            super::cmd_country_setup_apply(&args)
        }
        "character-intent-plan" | "character-plan" => super::cmd_character_intent_plan(&args),
        "character-template-recommend" | "character-template" => {
            super::cmd_character_template_recommend(&args)
        }
        "portrait-register-plan" | "portrait-plan" => super::cmd_portrait_register_plan(&args),
        "character-scope-audit" | "character-usage-audit" => {
            super::cmd_character_scope_audit(&args)
        }
        "diplomatic-effect-plan" | "diplomacy-plan" => super::cmd_diplomatic_effect_plan(&args),
        "iterator-diplomacy-plan" | "diplomacy-iterator-plan" => {
            super::cmd_iterator_diplomacy_plan(&args)
        }
        "ai-strategy-audit" | "ai-strategy-plan" => super::cmd_ai_strategy_audit(&args),
        "ai-behavior-audit" | "p20-ai-behavior-audit" | "ai-route-balance-audit" => {
            super::cmd_ai_behavior_audit(&args)
        }
        "ai-behavior-apply" | "p20-ai-behavior-apply" | "apply-ai-behavior" => {
            super::cmd_ai_behavior_apply(&args)
        }
        "war-goal-plan" | "wargoal-plan" => super::cmd_war_goal_plan(&args),
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
        "localisation-token-report"
        | "localization-token-report"
        | "loc-token-report"
        | "inspect-localisation-tokens"
        | "localisation-token-resolve"
        | "localization-token-resolve"
        | "inspect-localization-tokens" => super::cmd_localisation_token_report(&args),
        "localisation-token-check"
        | "localization-token-check"
        | "loc-token-check"
        | "check-localisation-tokens"
        | "check-localization-tokens" => super::cmd_localisation_token_check(&args),
        "author-placeholder-plan"
        | "authoring-placeholder-plan"
        | "localisation-placeholder-plan"
        | "localization-placeholder-plan"
        | "localisation-placeholder-resolve-plan"
        | "localization-placeholder-resolve-plan"
        | "localisation-token-resolve-plan"
        | "localization-token-resolve-plan" => super::cmd_author_placeholder_plan(&args),
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
        "ai-repair-prompt" | "repair-prompt" | "prepare-repair-prompt" => {
            super::cmd_ai_repair_prompt(&args)
        }
        "repair-failed-output" | "failed-output-pack" | "package-failed-output" => {
            super::cmd_repair_failed_output(&args)
        }
        "ai-repair-bundle" | "repair-bundle" | "one-shot-repair-bundle" => {
            super::cmd_ai_repair_bundle(&args)
        }
        "plan-history-edit" | "history-edit-plan" | "plan-state-edit" => {
            super::cmd_plan_history_edit(&args)
        }
        "map-data-audit" | "audit-map-data" | "map-audit" => super::cmd_map_data_audit(&args),
        "map-intent-plan" | "plan-map-intent" | "map-data-plan" => {
            super::cmd_map_intent_plan(&args)
        }
        "province-query" | "query-provinces" | "map-province-query" => {
            super::cmd_province_query(&args)
        }
        "state-transaction-plan" | "plan-state-transaction" => {
            super::cmd_state_transaction_plan(&args)
        }
        "state-transaction-apply" | "apply-state-transaction" => {
            super::cmd_state_transaction_apply(&args)
        }
        "supply-network-plan" | "plan-supply-network" | "map-network-plan" => {
            super::cmd_supply_network_plan(&args)
        }
        "supply-network-apply" | "apply-supply-network" | "map-network-apply" => {
            super::cmd_supply_network_apply(&args)
        }
        "strategic-region-plan" | "plan-strategic-region" | "air-region-plan" => {
            super::cmd_strategic_region_plan(&args)
        }
        "map-topology-plan" | "plan-map-topology" | "topology-plan" => {
            super::cmd_map_topology_plan(&args)
        }
        "map-topology-gate" | "topology-gate" => super::cmd_map_topology_gate(&args),
        "map-override-risk-audit" | "map-override-audit" | "audit-map-overrides" => {
            super::cmd_map_override_risk_audit(&args)
        }
        "map-runtime-gate" | "map-log-gate" => super::cmd_map_runtime_gate(&args),
        "map-transaction-gate" | "map-data-transaction-gate" | "map-risk-gate" => {
            super::cmd_map_transaction_gate(&args)
        }
        "map-release-gate" | "map-data-release-gate" => super::cmd_map_release_gate(&args),
        "hoi4-runtime-session-plan" | "runtime-session-plan" => {
            super::cmd_hoi4_runtime_session_plan(&args)
        }
        "hoi4-runtime-session-runner" | "runtime-session-runner" => {
            super::cmd_hoi4_runtime_session_runner(&args)
        }
        "import-mod-ir" => super::cmd_import_mod_ir(&args),
        "doctor-skill-install" | "cleanup-old-skills" | "repair-skill-install" => {
            super::cmd_doctor_skill_install(&args)
        }
        "focus-copy-prompt" => super::cmd_focus_copy_prompt(&args),
        "idea-copy-prompt" | "national-spirit-copy-prompt" => super::cmd_idea_copy_prompt(&args),
        "event-copy-prompt" | "event-writing-prompt" => super::cmd_event_copy_prompt(&args),
        "event-style-profile" | "event-copy-style-profile" | "event-style-summary" => {
            super::cmd_event_style_profile(&args)
        }
        "work-package-style-context" | "style-context" | "mod-style-context" => {
            super::cmd_work_package_style_context(&args)
        }
        "icon-preview" => super::cmd_icon_preview(&args),
        "register-gfx-icons" | "register-icons" | "register-gfx" => {
            super::cmd_register_gfx_icons(&args)
        }
        "register-gui-asset"
        | "register-gui-sprite"
        | "gui-asset-register"
        | "icon-register-plan"
        | "sprite-register-plan" => super::cmd_register_gui_asset(&args),
        "parse-focus-layout" => super::cmd_parse_focus_layout(&args),
        "render-focus-code" | "generate-focus-code" | "focus-template-code" => {
            super::cmd_render_focus_code(&args)
        }
        "apply-focus-layout" => super::cmd_apply_focus_layout(&args),
        "apply-focus-intent" | "apply-focus-intent-patch" | "apply-focus-effect-intent" => {
            super::cmd_apply_focus_intent(&args)
        }
        "parse-focus-excel" | "parse-focus-xlsx" | "focus-excel-skeleton" => {
            super::cmd_parse_focus_excel(&args)
        }
        "apply-focus-excel" | "apply-focus-xlsx" => super::cmd_apply_focus_excel(&args),
        "parse-focus-copy-cards" => super::cmd_parse_focus_copy_cards(&args),
        "parse-feature-cards" => super::cmd_parse_feature_cards(&args),
        "apply-feature-cards" => super::cmd_apply_feature_cards(&args),
        "apply-decision-intent"
        | "apply-decision-intent-patch"
        | "apply-decision-effect-intent" => super::cmd_apply_decision_intent(&args),
        "parse-event-cards" => super::cmd_parse_event_cards(&args),
        "apply-event-cards" => super::cmd_apply_event_cards(&args),
        "apply-event-intent" | "apply-event-intent-patch" | "apply-event-option-intent" => {
            super::cmd_apply_event_intent(&args)
        }
        "event-trigger-report" | "event-triggers" | "event-trigger-conditions" => {
            super::cmd_event_trigger_report(&args)
        }
        "emit-hoi4yaml" => super::cmd_emit_hoi4yaml(&args),
        "run-workflow" => super::cmd_run_workflow(&args),
        "generate-mod" | "one-shot-mod" | "one-sentence-mod" => super::cmd_generate_mod(&args),
        "country-localisation-template" | "country-localization-template" => {
            super::cmd_country_localisation_template(&args)
        }
        "localisation-glossary"
        | "localization-glossary"
        | "translation-glossary"
        | "terminology-glossary" => super::cmd_localisation_glossary(&args),
        "translate-localisation"
        | "translate-localization"
        | "translation-plan"
        | "quick-translate-localisation"
        | "quick-translate-localization" => super::cmd_translate_localisation(&args),
        "analyze-error-log" | "parse-error-log" => super::cmd_analyze_error_log(&args),
        _ => return Err(CliError::usage(format!("unknown command: {command}"))),
    };
    result.map_err(CliError::message)
}
