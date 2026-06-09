---
name: hoi4-mod-maker
description: Build, extend, and validate Hearts of Iron IV mod files from natural-language requests. Use when Codex is asked to create or edit HOI4/钢铁雄心4 mods, including national focuses, ideas, events, decisions, localisation, countries, states, leaders, technologies, scripted effects, scripted triggers, descriptor.mod, or Steam Workshop-ready mod folders.
---

# HOI4 Mod Maker

## Core Workflow

Turn the user's mod idea into concrete HOI4 files, using the existing mod's style when a mod folder is present.

1. Locate the target mod root.
   - Prefer the user's provided folder.
   - If no folder exists, create a skeleton with `hoi4skill scaffold`.
   - Treat the folder containing `descriptor.mod` as the mod root.
2. For existing mods, build a modification knowledge base before editing.
   - Run `hoi4skill mod-knowledge <mod-root-or-launcher.mod> --output mod_knowledge.json`.
   - Determine whether the target is a standalone mod or a submod from `descriptor.mod` and launcher-side `.mod` dependencies.
   - If it is a submod, pass available dependency roots with `--mod-path` before claiming inherited tags, sprites, technologies, scripted values, state/province IDs, or localisation exist.
   - Use `knowledge_base` and `markdown_summary` as the source of truth for ID prefixes, localisation languages, namespace names, focus tree style, decision category style, state/province facts, scripted helper files, and icon/GFX sprite style.
   - If the request says "一句话", infer sensible defaults instead of asking, unless the country/tag or feature target is impossible to infer.
3. Convert the request into a small implementation plan.
   - Name the feature.
   - List touched systems such as focus, idea, event, decision, state, country history, technology, or localisation.
   - Pick stable IDs with a unique mod prefix.
   - For narrative or promotional prose, follow `references/copy-to-code-workflow.md` and translate the prose into a Feature Plan before writing files.
   - For plain-text national focus tree sketches, follow `references/focus-tree-layout.md`.
   - If the user asks for a focus tree, focus route, or several focuses but does not provide a layout, use the default five-stage focus template from `references/focus-tree-layout.md` instead of inventing scattered `x/y` coordinates.
   - For decision, national-spirit, technology, or special-GUI cards, follow `references/decision-idea-cards.md`.
   - For event cards, follow `references/event-cards.md`.
   - For country creation, country leaders, or country leader traits, follow `references/country-creation-leaders.md`.
   - For fast localisation translation between languages, follow `references/localisation-translation.md`.
4. Before writing generated workflow content, run a dry run.
   - Run `hoi4skill run-workflow --input <copy.txt> --tag <TAG> --prefix <prefix> --dry-run --output workflow_plan.json`.
   - Read the dry-run plan before writing files. Confirm target tag, prefix, touched systems, generated IDs, localisation targets, skipped reasons, warnings, and validation expectations.
   - If using a narrower `apply-*` path instead of `run-workflow`, first run the matching `parse-*` command and inspect the generated plan.
   - If the plan touches `history/countries`, `history/states`, state IDs, province IDs, or capitals, run `hoi4skill plan-history-edit` and follow its `decision`, `checks`, `warnings`, and `skipped` entries before writing.
5. Edit or create the minimum required files.
   - Preserve existing formatting and folder organization.
   - Keep unrelated mod content unchanged.
   - Add Simplified Chinese localisation when the request is Chinese.
   - Add English localisation only when the mod already uses it or the user asks.
   - For icon work, support `.dds`, `.png`, and `.tga`; add or reuse `interface/*.gfx` sprite definitions, preview icons when useful, and run `register-gfx-icons` before referencing new custom images.
   - For national-focus icons, first read the target mod/dependencies/game `interface/*.gfx` sprites through `mod-knowledge`, `build-game-index --game-root`, or `run-workflow/apply-focus-layout --game-root`; choose only verified `GFX_goal*` sprite names. If no verified goal icon is available, use `GFX_goal_unknown` and report that icon indexing is missing instead of inventing a sprite key.
   - When `register-gfx-icons` sees a non-English/non-ASCII image filename, it automatically translates the local filename into a semantic English filename, renames the asset, updates matching `interface/*.gfx` texturefile references, and then registers sprites. If the filename is already English/ASCII, it is left unchanged. If the filename cannot be translated semantically, skip that image and report it in `skipped_assets` for the AI/user summary; never invent random names such as `SOV_12347.png`.
6. Validate.
   - Run `hoi4skill validate <mod-root>` after edits.
   - Also run any repo-specific checks if the mod provides them.
7. After the mod is launched in HOI4, analyze `error.log`.
   - Run `hoi4skill analyze-error-log --input "<HOI4 user folder>\\logs\\error.log" --mod-root <mod-root> --output error_report.json`.
   - Treat new log entries as repair evidence. Do not claim the feature is in-game clean until the relevant `error.log` output has been checked.
8. Report exactly what was created or changed, which gates were run, and any remaining in-game test steps.

## Hard Output Rules

These are non-negotiable for AI-generated HOI4 content:

- Generated focus IDs must use only ASCII letters, digits, and underscores. Before writing focuses, scan existing `common/national_focus/*.txt`; do not collide with any existing `focus = { id = ... }`. If an ID is taken, rename the generated focus with a stable numeric suffix and update prerequisites, mutual exclusions, and localisation keys to match.
- National-focus mutual exclusion uses exactly `mutually_exclusive = { focus = <id> }`. Never write `mutual_exclusion`, `mutual_exclusive`, `mutually_exclusion`, or other approximate spellings.
- All national-focus keys must use exact HOI4 field names. Never pluralize, shorten, translate, or approximate fields such as `prerequisite`, `relative_position_id`, `completion_reward`, `ai_will_do`, `cancel_if_invalid`, `continue_if_invalid`, or `available_if_capitulated`.
- Event files must use exact structural fields too: top-level `add_namespace`, event `is_triggered_only`, `fire_only_once`, `mean_time_to_happen`, `immediate`, and `option`. Near-match spellings are fatal validation errors.
- When generating a focus tree without a user-supplied visual layout, use the default `x/y` structure: row `y=0` has one opening focus at `x=0`; row `y=1` has two to four expansion focuses with an `x` gap of 2; row `y=2` has one phase-result focus at `x=0`; row `y=3` has two to four expansion focuses with an `x` gap of 2; row `y=4` has one closing-result focus at `x=0`. Do not scatter focuses randomly.
- National-focus `icon = ...` values must come from verified `GFX_goal*` sprites in the target mod, dependency mods, or game `interface/*.gfx`. Do not invent icon names from the focus title.
- Simplified Chinese country-content localisation must be written to the target country TAG file, for example `localisation/simp_chinese/SOV_l_simp_chinese.yml` or `localisation/simp_chinese/FER_l_simp_chinese.yml`. Never output feature-prefix localisation files such as `sov_nep_l_simp_chinese.yml`, `ger_build_army_industry_l_simp_chinese.yml`, or `<prefix>_l_simp_chinese.yml`.
- Mod display names belong only in `descriptor.mod` and the launcher-side `.mod` file. Never generate localisation keys such as `<prefix>_mod_name`, `chinaprc_1979_mod_name`, or any `*_mod_name:0 "..."` entry under `l_simp_chinese:`.
- Focus descriptions must be finished, stylized HOI4 Chinese national-focus prose. They must sound like in-universe policy, route struggle, state-building, revolutionary mobilisation, military reform, or economic reconstruction. Do not output placeholders such as `具体效果待补充`, `描述`, `TODO`, or raw effect explanations.
- Do not use "first make a verifiable demo", "conservative script skeleton", "I will add route narrative later", or similar excuses as a generation strategy or final report. A runnable skeleton is not a finished HOI4 feature until route narrative, stylized titles/descriptions, localisation, and script wiring are all completed.
- Focus prose must use the internal first-person perspective of the target country, route, faction, army, party, government, or interest group. Do not write third-party observer, encyclopedia, historian, or outside commentary.
- National-spirit descriptions must be finished, stylized HOI4 Chinese national-spirit prose. They describe a persistent state, institution, social contradiction, reform legacy, mobilisation condition, or historical burden. Do not write them as focus actions, future policy promises, or generic text such as `正在影响国家`.
- National-spirit scripted IDs and localisation keys must end with `_idea`, so models cannot confuse ideas with focuses.
- If the target TAG is unknown, infer it from local localisation plus `common/country_tags` and `common/countries` when a source mod is available. If it still cannot be determined, stop and ask; do not invent prefix-based localisation.
- When editing an existing mod, do not generate code from memory alone. First build/read `mod_knowledge.json`; facts missing from it are unknown until verified in local files or dependency indexes.
- For country creation and country leaders, standalone mods use modern `common/characters` plus `history/countries` `recruit_character` by default unless the user asks for legacy syntax. Submods must follow the dependency mod's observed country/leader syntax from `mod_knowledge.json` and `--mod-path`; if dependency syntax is not indexed, report it as unknown instead of guessing.
- For history files, states, provinces, capitals, cores, resources, buildings, and victory points, follow `references/history-states-provinces.md`.
- Never edit `history/states` or `history/countries` from a place name, localisation key, focus text, or memory alone. Direct history edits require local file evidence or indexed game/dependency evidence.
- Before any direct `history/states` edit, run `hoi4skill plan-history-edit`. If `direct_history_edit_allowed` is false, do not write `history/states`; report the skipped reason and use a state-scoped scripted effect or ask for missing IDs.
- State IDs identify `history/states` blocks/files. Province IDs come from state `provinces = { ... }` or `map/definition.csv`. `victory_points = { <province_id> <points> }` uses province IDs.
- `capital = ...` in `history/countries` uses a province ID, not a state ID. If a value also matches a known state ID, warn and verify manually before writing.
- Buildings and resources in `history/states` are state-level data. Gameplay rewards should normally use state-scoped effects such as `random_owned_controlled_state` plus `add_building_construction`, not direct start-date state-file edits.

## Reference Files

Read only the reference needed for the current task:

- `references/file-map.md`: folder and file locations for common HOI4 systems.
- `references/mod-knowledge.md`: pre-edit mod/submod classification and knowledge-base rules to avoid hallucinating tags, namespaces, sprites, and dependency content.
- `references/copy-to-code-workflow.md`: full pipeline from player-facing prose to Feature Plan, HOI4 file plan, code generation, validation, and repair.
- `references/implementation-patterns.md`: ID naming, one-sentence request handling, and common feature patterns.
- `references/focus-tree-layout.md`: plain-text focus tree sketch syntax, including row/column layout and `互斥` branch handling.
- `references/decision-idea-cards.md`: plain Chinese card syntax for decisions, national spirits, unique technologies, and special GUI skeletons.
- `references/technology-trees.md`: researched rules for `common/technologies`, research folders, paths, categories, technology icons, and safe technology-tree integration.
- `references/country-creation-leaders.md`: researched rules for country tag creation, `common/countries`, `history/countries`, country leader traits, modern `common/characters`, and legacy `create_country_leader`.
- `references/history-states-provinces.md`: researched rules for `history/states`, province IDs, `map/definition.csv`, capitals, cores, victory points, buildings, and resources.
- `references/event-cards.md`: plain Chinese card syntax for country, news, and state events.
- `references/localisation-translation.md`: fast translation workflow for reading any localisation language folder and producing any target language while preserving HOI4 tokens.
- `references/wiki-code-index.md`: HOI4 Wiki, ParaWiki, and local game documentation lookup routes for current code names.
- `references/hoi4-script-snippets.md`: copy-adapt templates for focuses, events, decisions, ideas, localisation, GFX, and state effects.
- `references/gfx-icon-preview.md`: icon asset rules, `.dds`/`.png`/`.tga` handling, sprite mapping, batch registration, conflict reports, and preview workflow.
- `references/scopes-effects-triggers.md`: scope discipline, high-use effect/trigger names, and where to verify them in game documentation.
- `references/validation.md`: syntax, localisation, and in-game verification checklist.

## Rust CLI

Use `hoi4skill.exe` for local work. It combines scaffolding, parsing, file generation, validation, style scanning, icon preview, error-log analysis, and optional structured YAML emission in one binary. Do not call PowerShell or Python helper scripts; this skill is Rust-only.

If this skill was installed from the release package, prefer the bundled Windows binary at `bin/windows-x64/hoi4skill.exe` inside the skill folder when `hoi4skill` is not already on `PATH`. If the skill was installed from the source tree or on a non-Windows machine, build the CLI from `hoi4skill-cli` with `cargo build --release` before running CLI commands.

```text
hoi4skill scaffold --name "My HOI4 Mod" --output "M:\path\my_hoi4_mod" --launcher-file
hoi4skill generate-mod --text "给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。" --output "M:\path\my_hoi4_mod"
hoi4skill generate-mod --text "给远东铁路共和国加一个国策，完成后获得3个军工厂。" --source-root "M:\path\source_mod" --output "M:\path\my_hoi4_mod"
hoi4skill mod-knowledge "M:\path\mod_or_launcher.mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
hoi4skill plan-history-edit "M:\path\mod" --text "edit history/states owner for state_id 64" --state-id 64 --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output history_plan.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --output workflow_report.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output workflow_report.json
hoi4skill run-workflow --input "M:\path\copy.txt" --tag SOV --prefix sov_nep --dry-run --output workflow_plan.json
hoi4skill import-mod-ir "M:\path\mod" --max-items 1000 --output imported_ir.json
hoi4skill icon-preview --mod-root "M:\path\mod" --output "M:\preview"
hoi4skill register-gfx-icons --mod-root "M:\path\mod" --prefix sov_nep --category all --output gfx_report.json
hoi4skill parse-focus-layout --input "M:\path\layout.txt" --tag SOV --prefix sov_alt --output focus_plan.json
hoi4skill apply-focus-layout --input "M:\path\layout.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_alt --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod"
hoi4skill parse-focus-excel --input "M:\path\focus_tree.xlsx" --tag SOV --prefix sov_alt --sheet FocusTree --output focus_tree.txt
hoi4skill apply-focus-excel --input "M:\path\focus_tree.xlsx" --mod-root "M:\path\mod" --tag SOV --prefix sov_alt --sheet FocusTree
hoi4skill parse-feature-cards --input "M:\path\cards.txt" --tag SOV --prefix sov_nep --output feature_plan.json
hoi4skill parse-event-cards --input "M:\path\events.txt" --tag SOV --prefix sov_nep --output event_plan.json
hoi4skill idea-copy-prompt "M:\path\modA" "M:\path\modB" --style compact --output idea_prompt.md
hoi4skill country-localisation-template --tag FER --name "远东铁路共和国" --prefix fer_rail --idea FER_fragmented_railway_authority=分裂的铁路主权 --output FER_l_simp_chinese.yml
hoi4skill translate-localisation --mod-root "M:\path\mod" --from english --to simp_chinese --format prompt --output loc_en_to_zh_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\mod" --from french --to german --format prompt --output loc_fr_to_de_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\mod" --from french --to german --translated-input translated_l_german.yml --apply --report loc_apply_report.json
hoi4skill translate-localisation --mod-root "M:\path\mod" --from russian --to japanese --format yml --output-dir "M:\path\mod\localisation\japanese"
hoi4skill validate "M:\path\mod"
hoi4skill analyze-error-log --input "%USERPROFILE%\Documents\Paradox Interactive\Hearts of Iron IV\logs\error.log" --mod-root "M:\path\mod" --output error_report.json
```

`run-workflow` accepts mixed Chinese prose/cards, detects focus-tree sketches, decision/national-spirit/technology/special-GUI/scripted-helper/state-effect cards, and event cards, then writes the generated files when `--mod-root` is supplied. Its JSON report includes detected sections, generated plans, changed files, validation errors/warnings, and next steps. When the target mod already has a `focus_tree` whose country block resolves to the target tag, focus generation extends that existing tree and shifts new focus rows below the current max `y`; otherwise it creates a new focus file. When `--input` points to `.xlsx/.xls/.xlsm/.xlsb/.ods`, `run-workflow` first renders the worksheet as a Markdown table, then appends a normalized `国策树：` sketch so models see the table while the CLI still has deterministic focus-layout text to parse. When `--game-root` and optional `--mod-path` are supplied, generated focuses choose missing icons from verified indexed `GFX_goal*` sprites.

`parse-focus-excel` and `apply-focus-excel` read `.xlsx`, `.xls`, `.xlsm`, `.xlsb`, or `.ods` files where AI or a human drew a national focus tree as a worksheet grid. Every non-empty non-connector cell becomes a focus. Cell text may include lines such as `ID: english_id`, `icon: GFX_goal...`, and `completion_reward: 1个军工厂`. For OOXML workbooks (`.xlsx`/`.xlsm`), the importer also reads worksheet drawing text boxes/shapes and merges an anchored Chinese title with the English ID stored in the underlying cell. The importer expands worksheet columns into HOI4 `x` coordinates with a minimum gap of 2 on the same `y` row, so if one focus is `x = 1`, the adjacent same-row focus is at least `x = 3`. Prerequisites are still inferred from the nearest valid earlier row, but relative placement now anchors to the first/start focus by default so later focuses share one `relative_position_id` base and can be moved together.

Excel mutual exclusion is explicit-only. A cell containing `互斥` links only the nearest focus on its left and right in the same worksheet row. The importer writes that exact pair to each focus and to the JSON `mutually_exclusive` list. It must not infer additional mutual exclusions from branches, ideology, proximity, or mirrored layout.

`mod-knowledge` is the required pre-edit dossier for existing mods. It resolves a directory, `descriptor.mod`, or launcher-side `.mod` file; classifies the target as `standalone_mod`, `submod`, or `unknown_no_descriptor`; reads local descriptor/launcher metadata, dependency names, focus trees, event namespaces, country tags, history countries, history states, province definition summaries, localisation style, decisions, ideas, GFX sprites, and content samples; then emits a JSON `knowledge_base` plus a model-readable `markdown_summary`. Use it before `run-workflow`, `apply-*`, or manual edits.

`plan-history-edit` is the required gate before direct `history/countries` or `history/states` work. It reads local state files, `map/definition.csv`, dependency roots supplied with `--mod-path`, and optional `--game-root` index facts; then reports whether state IDs, province IDs, and capital province IDs are known. If facts are missing, it returns skipped reasons instead of guessing. For focus rewards and temporary changes, prefer the generated state-scoped scripted-effect strategy.

For event cards, the writer scans existing namespaces first. A matching namespace is appended in place with the next safe event number, while new namespaces use `events/<prefix>_events.txt`. Re-running the same generated card is idempotent because the event block carries a stable `hoi4skill_card` marker.

For new custom art under `gfx/interface`, run `register-gfx-icons` before referencing sprite keys in focuses, ideas, events, decisions, or special GUI. The command first translates and renames non-English image filenames to semantic English names, leaves already-English filenames unchanged, then writes generated `interface/<prefix>_*.gfx` files for dynamic GUI icons, focus icons, idea pictures, event pictures, and decision/category pictures. It reverse-lookups existing `texturefile` registrations and avoids sprite-name collisions by reusing exact same-name/same-texture sprites or appending a numeric suffix when a sprite name already points to a different image. The JSON report includes the original texture path, new English filename, and remarks for each registered sprite.

For decision cards, the writer scans existing decision categories, decision files, and Simplified Chinese localisation first. A matching target-country category is reused when safe; otherwise it creates the generated category and decision file.

For national-spirit cards, the writer scans existing target-country `common/ideas` files first. A safe country-wrapper file is reused, while large shared minister/advisor/character idea files are skipped; otherwise it creates `<prefix>_ideas.txt`.

For focus layouts, the writer scans existing target-country focus trees and all existing focus IDs before writing. It extends an existing target-country tree when one exists, shifts rows below the current max `y`, and renames generated IDs before writing if any ID already exists elsewhere in the mod. For missing focus icons, pass `--game-root` and dependency `--mod-path` so the writer can reuse verified `GFX_goal*` icons from `interface/*.gfx`; otherwise unresolved icons must stay `GFX_goal_unknown` instead of falling back to guessed sprite names.

For technology cards, the writer creates a minimal unique-technology skeleton under `common/technologies/<prefix>_technologies.txt`, with IDs ending in `_tech`.

For special-GUI cards, the writer creates a conservative skeleton only: `common/scripted_guis/<prefix>_scripted_guis.txt` plus `interface/<prefix>.gui`, with IDs ending in `_gui`. Do not treat this as a complete complex GUI; inspect the target mod before wiring variables, buttons, scripted loc, or custom views.

For scripted-effect and scripted-trigger cards, the writer creates `common/scripted_effects/<prefix>_scripted_effects.txt` or `common/scripted_triggers/<prefix>_scripted_triggers.txt`, with IDs ending in `_effect` or `_trigger`. Unresolved natural-language code stays as TODO comments.

For state-effect cards, the writer creates `common/scripted_effects/<prefix>_state_effects.txt`, with IDs ending in `_state_effect`. It uses `州ID` when supplied, otherwise it emits a state-scope helper and leaves state-name resolution notes instead of editing `history/states` directly.

`generate-mod` is the one-sentence path. It scaffolds a new mod folder, infers country tags, converts the sentence into internal focus/decision/national-spirit/event/technology/special-GUI/scripted-helper/state-effect cards, writes files, then embeds validation in the report. When `--source-root`, `--game-root`, or `--mod-path` is supplied, it first reads `localisation/**/*.yml`, matches country names against the request, and verifies the tag through `common/country_tags` plus the mapped `common/countries` file before falling back to the built-in common-country table.

`idea-copy-prompt` learns national-spirit copywriting from `common/ideas` plus Simplified Chinese localisation. By default it filters to `country` and `hidden_ideas` so advisor, designer, and law copy does not pollute national-spirit style.

`country-localisation-template` outputs one country's Simplified Chinese localisation skeleton in fixed sections: country tag/name, cosmetic name, focus tree, national spirits, decisions, events, unique technologies, and special GUI. National-spirit IDs passed with `--idea` are normalized to end with `_idea` so AI-generated text does not confuse them with focus IDs.

`translate-localisation` reads localisation from any language folder or input file, compares source keys against any target language, skips keys already present unless `--include-existing` is used, and emits either an AI translation prompt, JSON report, or target-language `.yml` scaffold. After translating the extracted content, run it again with `--translated-input <file-or-dir> --apply` to inject translated values back into `localisation/<target_language>` and report `missing_after_apply`. This is not limited to English or Simplified Chinese: `english -> simp_chinese`, `french -> german`, `russian -> japanese`, and other HOI4 language folder names follow the same closed-loop workflow. The CLI does not machine-translate by itself; translate the quoted values with the model, preserve keys and HOI4 tokens exactly, then apply and validate.

## Natural-Language Modding Rules

For a one-sentence request, prefer a complete small feature over a partial file fragment.

For a longer story, route teaser, design pitch, or Tieba-style prose, first extract playable requirements:

- target country, tag, faction, ideology, or state,
- feature type such as focus, event, decision, idea, character, state edit, or scripted helper,
- player-facing text,
- effects and triggers,
- balance numbers,
- required dependencies or DLC,
- files to edit or create.

If the prose contains only flavor and no gameplay effect, propose a conservative playable effect before coding.

Example request:

```text
给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。
```

Expected output:

- A focus entry in the German focus tree or a new focus file if no tree exists.
- An event namespace and event with an `id`.
- Localisation for focus title, focus description, event title, event description, and event option.
- Effects that add buildings to valid controlled states.
- A final note telling the user where to place the focus in the tree if coordinates were inferred.

When details are missing:

- Infer `simp_chinese` localisation for Chinese requests.
- Infer a unique prefix from the mod folder name or existing IDs.
- Infer conservative balance values.
- Avoid overwriting vanilla files unless the mod already uses replace paths or the user explicitly asks.
- If a change requires DLC-specific mechanics, state that dependency in the final answer.
- If the user mentions icons or focus/idea/decision art, build or update the icon preview and report the preview HTML path.
- If the user sends a visual focus tree sketch, preserve its rows and branches; treat `互斥` as a mutual-exclusion marker.
- If the user asks for a focus tree or route but does not provide a visual sketch, use the default five-stage focus layout: 1 opener, 2-4 expansion focuses, 1 phase result, 2-4 expansion focuses, 1 closing result.
- If the user sends feature cards starting with `决议：`, `民族精神：`, `独有科技：`, `特殊GUI：`, `脚本效果：`, `脚本触发：`, or `州效果：`, parse them as structured feature cards before generating files.
- If the user sends cards starting with `事件：`, parse them as structured event cards before generating files.

## HOI4 Authoring Guidelines

- Use stable, unique IDs: `<prefix>_<feature>_<thing>`.
- Scan existing focus IDs before adding national focuses; focus IDs must not collide across the mod.
- Use event namespaces with top-level `add_namespace = <namespace>` before event bodies; one event file may declare multiple namespaces.
- Event body IDs must be inside a declared namespace, for example `id = sov_nep.1`; event numbers `1..200000` are valid.
- Keep localisation keys identical to scripted IDs where possible.
- National-spirit scripted IDs and localisation keys must end with `_idea`.
- Use national spirits for long-term focus effects: focus `completion_reward` may `add_ideas`, and temporary long-term states must be removed later with `remove_ideas`. Use direct focus rewards only for immediate effects; never put `modifier = { ... }` directly inside a focus `completion_reward`.
- Prefer new files inside the mod over editing copied vanilla mega-files.
- Avoid `replace_path` unless the request requires total replacement.
- When adding state buildings, choose explicit state IDs only after inspecting existing targets or vanilla state files.
- Keep balance conservative and easy for players to understand.
- Use comments sparingly and only to explain non-obvious compatibility or placement choices.
- When an effect, trigger, modifier, or scope is uncertain, look it up in `references/wiki-code-index.md` and the user's local HOI4 `documentation/*.md` before writing it.

## Validation Discipline

Always run the validator after generating files. Treat missing localisation UTF-8 BOM as a fatal validation error, fix it before reporting success, and do not claim the mod is fully tested in HOI4 until the relevant in-game `error.log` has been checked.

After an in-game launch test, analyze the current HOI4 `error.log` with `hoi4skill analyze-error-log`. Summarize new errors and repair hints for the user; do not treat static validation as a substitute for the in-game log.

Do not claim Workshop readiness unless `descriptor.mod`, launcher `.mod` metadata, thumbnail policy, and an in-game launch test have all been handled.
