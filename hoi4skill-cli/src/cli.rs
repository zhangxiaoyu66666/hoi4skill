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
        "validate" => super::cmd_validate(&args),
        "detect-hoi4-path" => super::cmd_detect_hoi4_path(&args),
        "build-game-index" => super::cmd_build_game_index(&args),
        "scan-mod-style" => super::cmd_scan_mod_style(&args),
        "mod-knowledge" | "summarize-mod" | "mod-dossier" => super::cmd_mod_knowledge(&args),
        "plan-history-edit" | "history-edit-plan" | "plan-state-edit" => {
            super::cmd_plan_history_edit(&args)
        }
        "import-mod-ir" => super::cmd_import_mod_ir(&args),
        "focus-copy-prompt" => super::cmd_focus_copy_prompt(&args),
        "idea-copy-prompt" | "national-spirit-copy-prompt" => super::cmd_idea_copy_prompt(&args),
        "icon-preview" => super::cmd_icon_preview(&args),
        "register-gfx-icons" | "register-icons" | "register-gfx" => {
            super::cmd_register_gfx_icons(&args)
        }
        "parse-focus-layout" => super::cmd_parse_focus_layout(&args),
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
