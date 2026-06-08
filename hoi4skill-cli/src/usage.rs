//! Human-facing command usage text for the CLI.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn print_usage() {
    println!(
        r#"hoi4skill {}

Usage:
  hoi4skill scaffold --name "My Mod" --output "M:\path\mod" [--launcher-file]
  hoi4skill detect-hoi4-path [--hoi4-path "C:\path\Hearts of Iron IV"] [--output hoi4_path.json]
  hoi4skill validate "M:\path\mod"
  hoi4skill validate "M:\path\mod" --game-root "C:\path\Hearts of Iron IV" [--mod-path "M:\path\dependency.mod"]
  hoi4skill build-game-index --game-root "C:\path\Hearts of Iron IV" [--mod-path "M:\path\dependency.mod"] [--output game_index.json]
  hoi4skill scan-mod-style "M:\path\mod" [--output mod_style.json] [--max-sprites 400]
  hoi4skill mod-knowledge "M:\path\mod" [--mod-path "M:\path\dependency.mod"] [--output mod_knowledge.json] [--max-items 80]
  hoi4skill plan-history-edit "M:\path\mod" --text "给 GER 的 64 州加工厂" [--state-id 64] [--province-id 6521] [--capital 6521] [--game-root "C:\path\Hearts of Iron IV"] [--mod-path "M:\path\dependency.mod"] [--output history_plan.json]
  hoi4skill import-mod-ir "M:\path\mod" [--output imported_ir.json] [--max-items 1000]
  hoi4skill focus-copy-prompt "M:\path\modA" ["M:\path\modB"] [--output focus_prompt.md] [--style full|compact]
  hoi4skill idea-copy-prompt "M:\path\modA" ["M:\path\modB"] [--output idea_prompt.md] [--style full|compact] [--all-categories]
  hoi4skill icon-preview --mod-root "M:\path\mod" [--output "M:\preview"] [--max-icons 800]
  hoi4skill register-gfx-icons --mod-root "M:\path\mod" --prefix sov_nep [--category all|dynamic|focus-idea|event|decision] [--output report.json]
  hoi4skill parse-focus-layout --input layout.txt --tag SOV --prefix sov_alt [--output plan.json]
  hoi4skill apply-focus-layout --input layout.txt --mod-root "M:\path\mod" --tag SOV --prefix sov_alt
  hoi4skill parse-focus-excel --input tree.xlsx --tag SOV --prefix sov_alt [--sheet FocusTree] [--format focus-tree|json] [--output focus_tree.txt]
  hoi4skill apply-focus-excel --input tree.xlsx --mod-root "M:\path\mod" --tag SOV --prefix sov_alt [--sheet FocusTree]
  hoi4skill parse-focus-copy-cards --input focus_copy.txt [--output focus_prompts.md]
  hoi4skill parse-feature-cards --input cards.txt --tag SOV --prefix sov_nep [--output plan.json]
  hoi4skill apply-feature-cards --input cards.txt --mod-root "M:\path\mod" --tag SOV --prefix sov_nep
  hoi4skill parse-event-cards --input events.txt --tag SOV --prefix sov_nep [--output plan.json]
  hoi4skill apply-event-cards --input events.txt --mod-root "M:\path\mod" --tag SOV --prefix sov_nep
  hoi4skill emit-hoi4yaml --input cards.txt --kind feature-cards|event-cards|focus-layout --tag SOV --prefix sov_nep [--output mod.yaml]
  hoi4skill run-workflow --input copy.txt [--mod-root "M:\path\mod"] --tag SOV --prefix sov_nep [--dry-run] [--output workflow_report.json]
  hoi4skill generate-mod --text "给德国加一个国策，完成后获得3个军工厂" --output "M:\path\mod" [--tag GER] [--prefix ger_demo] [--name "My Mod"] [--source-root "M:\path\source_mod"] [--game-root "C:\path\Hearts of Iron IV"] [--mod-path "M:\path\dependency.mod"] [--report report.json]
  hoi4skill country-localisation-template --tag FER --name "远东铁路共和国" --prefix fer_rail [--output FER_l_simp_chinese.yml]
  hoi4skill translate-localisation --mod-root "M:\path\mod" --from <source_language> --to <target_language> [--format prompt|yml|json] [--output translated.md]
  hoi4skill translate-localisation --mod-root "M:\path\mod" --from french --to german --translated-input translated_l_german.yml --apply [--report loc_apply_report.json]
  hoi4skill analyze-error-log --input "%USERPROFILE%\Documents\Paradox Interactive\Hearts of Iron IV\logs\error.log" [--mod-root "M:\path\mod"] [--output report.json]

This binary has no Python or PowerShell dependency.
"#,
        env!("CARGO_PKG_VERSION")
    );
}
