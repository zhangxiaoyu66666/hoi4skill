---
name: hoi4-mod-maker
description: Build, extend, and validate Hearts of Iron IV mod files from natural-language requests. Use when Codex is asked to create or edit HOI4/钢铁雄心4 mods, including national focuses, ideas, events, decisions, localisation, countries, states, leaders, technologies, scripted effects, scripted triggers, descriptor.mod, or Steam Workshop-ready mod folders.
---

# HOI4 Mod Maker

## Installation Self-Check

Before any other HOI4 command in a new agent session, run this skill's bundled binary:

`hoi4skill doctor-skill-install --fix`

- The command scans the global and project skill roots used by Codex, Claude Code, OpenCode, and generic Agent Skills.
- It infers the currently running `hoi4-mod-maker` directory from the bundled executable, keeps that copy, and automatically removes other directories whose `SKILL.md` frontmatter has the exact same skill name.
- It never deletes a directory without a verified matching `SKILL.md`. If the current copy cannot be inferred, it refuses automatic deletion instead of guessing.
- After cleanup, continue using the same bundled binary. Never fall back to a backup, versioned-old, cached, or repository-copy skill.

## Country TAG Evidence Gate

This gate runs before scaffolding, parsing a workbook, choosing a prefix, or writing any content.

1. Resolve the target against the local game/dependency knowledge base:
   `hoi4skill resolve-country-tag --text "<literal user request>" --game-root "<HOI4 root>" [--source-root "<source mod>"] [--mod-path "<dependency>"] --output tag_resolution.json`
2. Read `resolved_tag`, `source`, `exists_in_index`, and `decision`. Pass only that `resolved_tag` to later commands.
   Resolution is data-driven for every country: read country names and aliases from the game/dependency/source-mod localisation, verify the mapping through `common/country_tags` and `common/countries`, and rank target-country wording above enemy/background-country wording. Do not maintain or rely on a small hard-coded country list.
3. When `decision = reuse_existing_tag`, writing `common/country_tags/*`, `common/countries/*`, or `history/countries/*` is forbidden. A new mod may add content for an existing country without redefining that country.
4. Creating a new country TAG requires both:
   - the user's literal request explicitly says to establish/create a new country or establish/create a new/custom TAG; and
   - the resolver is rerun with an explicit `--tag <TAG> --allow-new-tag`.
   No inferred necessity, narrative context, regime change, revolution, independence, new government, or deleted/new mod folder can substitute for that literal authorization.
5. A mod title, route, ideology, party, faction, government, revolutionary committee, army, resistance group, namespace, or file/ID prefix is not a country TAG and never authorizes one. Resolve whatever country the user names from the local knowledge base; do not turn an organisation or prefix into a country.
6. A parser echoing a supplied `--tag` is not evidence that the TAG exists. Only the resolver's indexed result or verified source-mod country mapping is evidence.
7. Never fall back to a backup, versioned-old, cached, or separately discovered skill/binary after the active bundled command fails. Use this skill's own `bin/windows-x64/hoi4skill.exe`; if it cannot perform the required gate, stop instead of using an older executable.

## Local Clausewitz Code Library

The model must retrieve real HOI4 code before generating or changing any Clausewitz-backed system. Prompt knowledge and remembered syntax are not evidence.

1. Build the vanilla base library from the user's own game installation:
   `hoi4skill build-clausewitz-library --game-root "<HOI4 root>"`
2. `--mod-path` is dependency evidence for validation and resource lookup; it never authorizes or loads that mod's code into the Clausewitz library.
3. Load a separate mod-code layer only when the user's literal request explicitly says to load, reference, or imitate that specific mod:
   `hoi4skill build-clausewitz-library --game-root "<HOI4 root>" --code-mod-path "<requested mod>" --request "<literal user request>"`
4. The library indexes complete, source-attributed blocks for focus trees/focuses, country/news/state/leader events, national spirits, decisions/categories, characters, scripted effects/triggers, country/state history, and GFX sprite registrations.
5. Query it directly when investigating syntax:
   `hoi4skill query-clausewitz-library --system focus --query "<feature meaning>"`
6. `prepare-edit-context --game-root ...` automatically creates only the vanilla library when missing. Add `--code-mod-path` only under the explicit user authorization above. Retrieved mod code is layered ahead of vanilla and never replaces the vanilla base.
7. Copy only block ownership, exact field names, nesting, and locally verified usage patterns. Never copy source IDs, country-specific narrative, balance values, or unrelated effects.
8. Retrieval does not authorize manual writes to template-owned systems. The Rust generators still own final focus, event, decision, and national-spirit output. If retrieved syntax exposes a missing generator field, extend the parser/writer and tests first.
9. The library is generated locally and must not be bundled, committed, or redistributed because it contains excerpts from the user's game/mod files.

## Requirement Scope Contract

Before planning files, copy the user's literal request into a scope contract. A request to create a new mod authorizes a new mod folder, not every HOI4 subsystem.

- Authorize only systems named by the user or strictly required to wire those systems at runtime.
- Print the exact planned file list before writing. Do not add a file that is absent from that list without new user authorization or verified runtime necessity.
- Do not create empty placeholder files.
- Do not redefine an existing vanilla country tag, create country history, initial units, characters/leaders, state history, decisions, technologies, GUI, or extra localisation languages merely because a new mod is being created.
- A Chinese request authorizes Simplified Chinese localisation. English localisation requires an explicit request or an existing target-mod convention.
- Counts such as "事件不少于4个" and "民族精神不少于5个" authorize those systems and set minimum counts; they do not authorize unrelated systems.
- Spreadsheet focus titles and positions are immutable. Preserve names and geometry exactly; never "improve" them by renaming or aesthetic rearrangement.
- Narrative context such as a revolution, uprising, postwar situation, ideology, or start year may shape descriptions and effects for existing spreadsheet cells, but it never authorizes adding, removing, splitting, merging, or renaming focus nodes.
- "Create a mod", "1936 start", "after the uprising", and similar setup language do not authorize `common/countries`, `common/country_tags`, `history/countries`, or `history/states`. Those files require a literal user request naming that subsystem.
- Validation warnings about unresolved sprites, modifiers, technologies, equipment, sub-units, states, or provinces are unfinished work. Do not call them harmless and do not report success.

## Template-Only Generation Contract

National focuses, decisions, events, and national spirits are template-owned systems. The model must not directly write or patch their Clausewitz blocks unless the user explicitly asks for direct manual Clausewitz/file editing.

- National focuses: the model may supply only structured layout/spec data such as ID, title, description, `x/y`, prerequisite, mutual exclusion, icon meaning, and completion effects. Use `apply-focus-excel`, `apply-focus-layout`, `render-focus-code`, or `run-workflow` to emit the fixed tree header and focus blocks.
- Decisions: the model may supply only decision cards. Use `apply-feature-cards` or `run-workflow`; the Rust writer owns category/decision wrappers, field names, localisation, and formatting.
- National spirits: the model may supply only national-spirit cards. Use `apply-feature-cards` or `run-workflow`; the Rust writer owns `ideas = { country = { ... } }`, `_idea` IDs, `picture` syntax, modifiers, and localisation.
- Events: the model may supply only event cards. Use `apply-event-cards` or `run-workflow`; the Rust writer owns namespaces, event types, IDs, options, localisation, and structural fields.
- Do not use generic file-write or edit tools on `common/national_focus`, `common/decisions`, `common/ideas`, or `events` for generated content.
- If a requested field or mechanic is not expressible by the current structured input, extend the Rust parser/writer and add tests first. Never bypass the generator by hand-writing approximate script.
- After tool generation, inspect the emitted plan/diff and validate with the indexed game root. The model may revise structured input and rerun the tool, but must not repair generated blocks manually.
- Exception: manual editing is allowed only when the user explicitly requests hand-written Clausewitz code or direct edits to one of those script files. Requests such as "create a mod", "make it complete", "fix it", "continue", or "add content" do not grant this exception. The model may not infer the exception from convenience, complexity, missing generator support, or a deleted/new mod.
- Even under the explicit manual-edit exception, read the canonical Rust-rendered template first, preserve its fixed wrappers and fields, and run indexed validation afterward.

## Core Workflow

Turn the user's mod idea into concrete HOI4 files, using the existing mod's style when a mod folder is present.

1. Locate the target mod root.
   - Prefer the user's provided folder.
   - If no folder exists, create a minimal descriptor-only skeleton with `hoi4skill scaffold`; content writers create only the directories required by authorized systems.
   - Treat the folder containing `descriptor.mod` as the mod root.
2. Resolve the country TAG, then build a modification knowledge base before editing.
   - Run the Country TAG Evidence Gate above for both new and existing mods. This is the fast local knowledge-base bootstrap and must finish before any `--tag` is passed to a parser/writer.
   - Run `hoi4skill mod-knowledge <mod-root-or-launcher.mod> --output mod_knowledge.json`.
   - Determine whether the target is a standalone mod or a submod from `descriptor.mod` and launcher-side `.mod` dependencies.
   - If it is a submod, pass available dependency roots with `--mod-path` before claiming inherited tags, sprites, technologies, scripted values, state/province IDs, or localisation exist. When launcher metadata is available, prefer `--auto-mod-paths --launcher-dir "<HOI4 user mod dir>"`; if dependency resolution is missing or ambiguous, stop and ask for exact dependency roots instead of guessing.
   - Use `knowledge_base` and `markdown_summary` as the source of truth for ID prefixes, localisation languages, namespace names, focus tree style, decision category style, state/province facts, scripted helper files, and icon/GFX sprite style.
   - If the request says "一句话", infer sensible defaults instead of asking, unless the country/tag or feature target is impossible to infer.
   - For any multi-system edit, also run `hoi4skill prepare-edit-context --input <copy-or-workbook> --request "<literal user request>" --mod-root <mod-root> --tag <TAG> --prefix <prefix> --game-root <hoi4-root> --output edit_context.md` and read `Requirement Scope Contract`, `Write Gate`, and `Retrieved Clausewitz Code Library` as the first model context blocks before writing code.
3. Convert the request into a small implementation plan.
   - Name the feature.
   - List authorized systems and the exact planned files. Systems absent from the literal request are forbidden by default.
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
   - For spreadsheet focus trees, run `parse-focus-excel` without `--format` or explicitly with `--format markdown`. Its default review output shows the literal worksheet titles, occupied cells, simulated `x/y`, node count, and explicit mutual exclusions. Do not use `--format focus-tree` as model context and do not infer that generated IDs mean the worksheet lacks content.
   - If the plan touches `history/countries`, `history/states`, state IDs, province IDs, or capitals, run `hoi4skill plan-history-edit` and follow its `decision`, `checks`, `warnings`, and `skipped` entries before writing.
   - For focuses, decisions, events, and national spirits, never transition from the dry-run plan to a manual file edit. Apply the same structured input through the matching Rust writer.
5. Edit or create the minimum required files.
   - Preserve existing formatting and folder organization.
   - Keep unrelated mod content unchanged.
   - Add Simplified Chinese localisation when the request is Chinese.
   - Add English localisation only when the mod already uses it or the user asks.
   - For icon work, support `.dds`, `.png`, and `.tga`; add or reuse `interface/*.gfx` sprite definitions, preview icons when useful, and run `register-gfx-icons` before referencing new custom images.
   - For national-focus icons, first read the target mod/dependencies/game `interface/goals*.gfx` sprites through `mod-knowledge`, `build-game-index --game-root`, or `run-workflow/apply-focus-layout --game-root`; choose only verified focus icon sprite names such as `GFX_goal...` or `GFX_focus...` by matching the focus meaning. For national spirits, decisions, decision categories, events, and leader portraits, use the same verified-resource rule with `GFX_idea_*`, `GFX_decision_*`, `GFX_decision_category_*`, `GFX_report_event_*`, and `GFX_portrait_*` registrations. Match by ideology, country/region, role, and feature meaning; if no verified resource is available, use the documented fallback or report missing indexing instead of inventing a sprite key.
   - When `register-gfx-icons` sees a non-English/non-ASCII image filename, it automatically translates the local filename into a semantic English filename, renames the asset, updates matching `interface/*.gfx` texturefile references, and then registers sprites. If the filename is already English/ASCII, it is left unchanged. If the filename cannot be translated semantically, skip that image and report it in `skipped_assets` for the AI/user summary; never invent random names such as `SOV_12347.png`.
6. Validate.
   - Run `hoi4skill validate <mod-root> --game-root <hoi4-root> --request "<literal user request>"` after edits whenever the game installation is available.
   - For a new-mod request, request-scope validation rejects unrequested country definitions, country/state history, initial units, characters, English localisation, decisions, technologies, or custom GUI directories even if a model created them manually.
   - Treat unresolved-resource warnings as failures to finish, not as permission to claim success.
   - Also run any repo-specific checks if the mod provides them.
7. After the mod is launched in HOI4, analyze `error.log`.
   - Run `hoi4skill analyze-error-log --input "<HOI4 user folder>\\logs\\error.log" --mod-root <mod-root> --output error_report.json`.
   - Treat new log entries as repair evidence. Do not claim the feature is in-game clean until the relevant `error.log` output has been checked.
8. Report exactly what was created or changed, which gates were run, and any remaining in-game test steps.

## Hard Output Rules

These are non-negotiable for AI-generated HOI4 content:

- Generated focus IDs must use only ASCII letters, digits, and underscores. Before writing focuses, scan existing `common/national_focus/*.txt`; do not collide with any existing `focus = { id = ... }`. If an ID is taken, rename the generated focus with a stable numeric suffix and update prerequisites, mutual exclusions, and localisation keys to match.
- Every `focus_tree` must use exactly `country = { factor = 0 modifier = { add = 10 tag = <TAG> } }`. Scalar forms such as `country = KOR` are not loadable here. Never emit `default_focus`; it is not part of this national-focus tree template.
- National-focus mutual exclusion uses exactly `mutually_exclusive = { focus = <id> }`. Never write `mutual_exclusion`, `mutual_exclusive`, `mutually_exclusion`, or other approximate spellings.
- All national-focus keys must use exact HOI4 field names. Never pluralize, shorten, translate, or approximate fields such as `prerequisite`, `relative_position_id`, `completion_reward`, `ai_will_do`, `cancel_if_invalid`, `continue_if_invalid`, or `available_if_capitulated`.
- Event files must use exact structural fields too: top-level `add_namespace`, event `is_triggered_only`, `fire_only_once`, `mean_time_to_happen`, `immediate`, and `option`. Near-match spellings are fatal validation errors.
- When generating a focus tree without a user-supplied visual layout, use the default `x/y` structure: row `y=0` has one opening focus at `x=0`; row `y=1` has two to four expansion focuses with an `x` gap of 2; row `y=2` has one phase-result focus at `x=0`; row `y=3` has two to four expansion focuses with an `x` gap of 2; row `y=4` has one closing-result focus at `x=0`. Do not scatter focuses randomly.
- The opening focus uses absolute `x/y` and no `relative_position_id`. Every later focus keeps its real progression parent only in `prerequisite`, but its `relative_position_id` must point to the single opening focus and its `x/y` must be calculated relative to that opening focus. Never set `relative_position_id` to the previous/prerequisite focus.
- National-focus `icon = ...` values must come from verified focus icon sprites in the target mod, dependency mods, or game `interface/goals*.gfx`; vanilla commonly uses both `GFX_goal...` and `GFX_focus...`. National spirits, decisions, events, and leader portraits follow their own verified `interface/*.gfx` registrations. Do not invent icon or portrait names from the title.
- National-spirit pictures must come from verified `GFX_idea_<name>` registrations in target, dependency, or game `interface/*.gfx`. Register with `name = "GFX_idea_<name>"`, but reference it in `common/ideas` as `picture = <name>` without the `GFX_idea_` prefix.
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
- `references/clausewitz-code-library.md`: locally build and query source-attributed real HOI4 code blocks before generation.
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
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV"
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV" --code-mod-path "M:\path\requested_mod" --request "加载 requested_mod 的模组代码作为参考"
hoi4skill query-clausewitz-library --system focus --query "communist workers revolution" --max-results 6
hoi4skill generate-mod --text "给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。" --output "M:\path\my_hoi4_mod"
hoi4skill generate-mod --text "给远东铁路共和国加一个国策，完成后获得3个军工厂。" --source-root "M:\path\source_mod" --output "M:\path\my_hoi4_mod"
hoi4skill mod-knowledge "M:\path\mod_or_launcher.mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
hoi4skill prepare-edit-context --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output edit_context.md
hoi4skill plan-history-edit "M:\path\mod" --text "edit history/states owner for state_id 64" --state-id 64 --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output history_plan.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --output workflow_report.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output workflow_report.json
hoi4skill run-workflow --input "M:\path\copy.txt" --tag SOV --prefix sov_nep --dry-run --output workflow_plan.json
hoi4skill import-mod-ir "M:\path\mod" --max-items 1000 --output imported_ir.json
hoi4skill icon-preview --mod-root "M:\path\mod" --output "M:\preview"
hoi4skill register-gfx-icons --mod-root "M:\path\mod" --prefix sov_nep --category all --output gfx_report.json
hoi4skill parse-focus-layout --input "M:\path\layout.txt" --tag SOV --prefix sov_alt --output focus_plan.json
hoi4skill apply-focus-layout --input "M:\path\layout.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_alt --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod"
hoi4skill parse-focus-excel --input "M:\path\focus_tree.xlsx" --tag SOV --prefix sov_alt --sheet FocusTree --output focus_review.md
hoi4skill apply-focus-excel --input "M:\path\focus_tree.xlsx" --mod-root "M:\path\mod" --tag SOV --prefix sov_alt --sheet FocusTree
hoi4skill parse-feature-cards --input "M:\path\cards.txt" --tag SOV --prefix sov_nep --output feature_plan.json
hoi4skill parse-event-cards --input "M:\path\events.txt" --tag SOV --prefix sov_nep --output event_plan.json
hoi4skill idea-copy-prompt "M:\path\modA" "M:\path\modB" --style compact --output idea_prompt.md
hoi4skill country-localisation-template --tag FER --name "远东铁路共和国" --prefix fer_rail --idea FER_fragmented_railway_authority=分裂的铁路主权 --output FER_l_simp_chinese.yml
hoi4skill translate-localisation --mod-root "M:\path\mod" --from english --to simp_chinese --format prompt --output loc_en_to_zh_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\mod" --from french --to german --format prompt --output loc_fr_to_de_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\mod" --from french --to german --translated-input translated_l_german.yml --apply --report loc_apply_report.json
hoi4skill translate-localisation --mod-root "M:\path\mod" --from russian --to japanese --format yml --output-dir "M:\path\mod\localisation\japanese"
hoi4skill validate "M:\path\mod" --game-root "C:\path\Hearts of Iron IV" --request "literal user request"
hoi4skill analyze-error-log --input "%USERPROFILE%\Documents\Paradox Interactive\Hearts of Iron IV\logs\error.log" --mod-root "M:\path\mod" --output error_report.json
```

`run-workflow` accepts mixed Chinese prose/cards, detects focus-tree sketches, decision/national-spirit/technology/special-GUI/scripted-helper/state-effect cards, and event cards, then writes the generated files when `--mod-root` is supplied. Its JSON report includes detected sections, generated plans, changed files, validation errors/warnings, and next steps. When the target mod already has a `focus_tree` whose country block resolves to the target tag, focus generation extends that existing tree and shifts new focus rows below the current max `y`; otherwise it creates a new focus file. When `--input` points to `.xlsx/.xls/.xlsm/.xlsb/.ods`, `run-workflow` keeps a structured worksheet layout beside the model-readable Markdown table and applies that structure directly; it must never flatten cells into whitespace-delimited prose and parse them again. Excel focus titles, node count, rows, columns, blank-column spacing, and explicit mutual exclusions are immutable input. Do not rename, paraphrase, split, merge, add, remove, or recenter those focuses. When `--game-root` and optional `--mod-path` are supplied, generated focuses, national spirits, decisions, decision categories, and events choose missing art from verified indexed sprites by matching ideology, country/region, role, and feature meaning; leader work must use indexed `GFX_portrait_*` or verified legacy `gfx/leaders/...` paths.

`parse-focus-excel` and `apply-focus-excel` read `.xlsx`, `.xls`, `.xlsm`, `.xlsb`, or `.ods` files where AI or a human drew a national focus tree as a worksheet grid. `parse-focus-excel` defaults to a model-readable Markdown review; script output requires explicit `--format focus-tree`. Every non-empty non-connector cell becomes a focus. Cell text may include lines such as `ID: english_id`, `icon: GFX_goal...`, and `completion_reward: 1个军工厂`. For OOXML workbooks (`.xlsx`/`.xlsm`), the importer also reads worksheet drawing text boxes/shapes and merges an anchored Chinese title with the English ID stored in the underlying cell. The importer expands worksheet columns into HOI4 `x` coordinates with a minimum gap of 2 on the same `y` row, so if one focus is `x = 1`, the adjacent same-row focus is at least `x = 3`. Prerequisites are still inferred from the nearest valid earlier row, but relative placement now anchors to the first/start focus by default so later focuses share one `relative_position_id` base and can be moved together.

Never manually pair `relative_position_id = <parent>` with the worksheet's absolute `x/y` values. HOI4 treats those coordinates as offsets from that parent, so branch drift compounds on every row. Use the CLI-generated opening-focus anchor and its relative offsets unchanged.

Excel mutual exclusion is explicit-only. A cell containing `互斥` links only the nearest focus on its left and right in the same worksheet row. The importer writes that exact pair to each focus and to the JSON `mutually_exclusive` list. It must not infer additional mutual exclusions from branches, ideology, proximity, or mirrored layout.

`mod-knowledge` is the required pre-edit dossier for existing mods. It resolves a directory, `descriptor.mod`, or launcher-side `.mod` file; classifies the target as `standalone_mod`, `submod`, or `unknown_no_descriptor`; reads local descriptor/launcher metadata, dependency names, focus trees, event namespaces, country tags, history countries, history states, province definition summaries, localisation style, decisions, ideas, GFX sprites, and content samples; then emits a JSON `knowledge_base` plus a model-readable `markdown_summary`. Use it before `run-workflow`, `apply-*`, or manual edits.

`prepare-edit-context` packages the user's request, a `Write Gate`, `hoi4skill.ai_context_contract.v1`, `mod-knowledge` markdown summary, a non-writing dry-run workflow plan, validation status, unknown-fact list, blocked-until-verified list, and local file excerpts into one Markdown file. When `--output` is used, it also writes `.hoi4skill/ai_context_contract.json` unless `--no-context-contract-sidecar` is passed; use `--context-contract-output <path>` for a custom sidecar path. Use the Markdown as the model's first context block so it sees enough verified evidence before generating code. The AI context contract is the machine-readable authority for `final_code_allowed_by_context`, allowed edit surfaces, missing evidence, verification steps, unknown facts, and blocked-until-verified items. If the `Write Gate` status is `BLOCKED` or `VERIFY_FIRST`, or the contract says `final_code_allowed_by_context=false`, follow its verification steps before writing final game script; when it is ready, write only inside the allowed edit surface.

The edit context also includes `hoi4skill.edit_context_repair_insurance.v1`. Feed this block back to weak-model repair attempts together with the failed patch: it extracts dry-run safety blockers, validation errors, and warnings; attaches strict-index related code candidates when available; lists `compile-intent`, `check-code-symbol`, `validate-repair-context`, and final validation commands; and forbids hiding failures by deleting user text, inventing syntax, or swapping dynamic modifiers into national spirits.

For final intent compilation in CI, release, or writer handoff paths, run `compile-intent` with `--game-root <HOI4 root> --strict-code-index --require-final-code` (or `--fail-on-draft` / `--no-draft`). Draft-only intent JSON may still contain useful mapped suggestions, but if `safety.final_code_allowed=false`, `patch_plan.can_apply=false`, `status=blocked`, or any `blockers` / `errors` array is non-empty, the AI must stop and repair the mapping instead of copying the candidate code.
Read `effect_strategies` in intent reports before writing: `replace_national_spirit_with_swap_ideas` is the only accepted strategy for replacing national spirits; `create_national_spirit_definition` plus `add_existing_or_generated_national_spirit` is the accepted strategy for user-requested new national spirits; `dynamic_modifier_scripted_effect_protocol` is the accepted strategy for variable-driven dynamic modifiers. Do not reinterpret those as raw modifier blocks or ordinary `add_ideas` fallbacks.
Final validation also rejects indexed dynamic modifiers used through `add_ideas`, `remove_ideas`, `has_idea`, or `swap_ideas`; fix those by using `add_dynamic_modifier`/`remove_dynamic_modifier`/`has_dynamic_modifier` or the verified scripted-effect protocol.
When this failure appears in `validate-repair-context`, handle category `dynamic_modifier_misuse` as a semantic design error: inspect the reported dynamic modifier ID, keep it as a dynamic modifier unless the user explicitly asks to create a new national spirit, and rerun strict validation after repair.

Use `ai-repair-prompt --edit-context edit_context.md --repair-context ai_repair_context.json --failed-patch failed.patch` to assemble the weak-model retry prompt. The prompt requires a narrow repair summary, changed files, patch plan, user questions, and validation commands; it is not a license to redesign the work package.

Use `repair-failed-output --input validation.json|error_log_report.json|logic_audit.json|loc_audit.json|gfx_audit.json|error.log --output failed_output.md` when the failed material is a validator report, audit report, or HOI4 error log instead of a hand-written patch. `validate --output validation.json` embeds `mod_root`, `game_root`, `dependency_mods`, and changed-file boundaries; `analyze-error-log --output error_log_report.json --changed-only`, `logic-audit --output logic_audit.json --changed-only`, `loc-audit --output loc_audit.json --changed-only`, and `gfx-audit --output gfx_audit.json --changed-only` embed `mod_root` and `changed_files`. `repair-failed-output` recovers those roots and changed-file boundaries automatically, then writes dependency-aware `validate-repair-context` suggestions and a repair-only-listed-files rule. For older reports or raw logs without embedded roots, pass `--mod-root`, `--game-root`, `--mod-path`, and optionally `--changed <path>` explicitly. Then pass `failed_output.md` to `ai-repair-prompt --failed-patch`.

Use `ai-repair-bundle <mod-root> --game-root <HOI4 root> --edit-context edit_context.md` for the one-shot failed-generation loop. If no edit context exists yet, pass `--input <request file> --tag <TAG> --prefix <prefix>` and the bundle writes `edit_context.md` first. For dependency-backed submods, add either exact `--mod-path <dependency>` arguments or `--auto-mod-paths --launcher-dir "<HOI4 user mod dir>"`; the bundle forwards resolved dependency evidence into strict validation, `ai_repair_context.json`, and the repair prompt so weak-model repair attempts do not hallucinate that dependency symbols are absent. It then runs strict validation, writes `validation.json`, writes `ai_repair_context.json`, converts validation failures into `failed_output.md`, then writes `repair_prompt.md` plus a manifest. Add `--logic-audit --changed-only --changed <path>` after event/focus work to merge broken event chains, orphan events, unsafe cycles, empty event options, and event flag lifecycle issues into the same failed output pack. Add `--loc-audit --changed-only --changed <path>` after localisation work to merge missing keys, orphan keys, duplicate keys, and HOI4 colour/control-token issues into the repair prompt. Add `--gfx-audit --changed-only --changed <path>` after icon/art work to merge missing sprite registrations, missing texture files, orphan sprites, and unregistered images into the repair prompt. Add `--error-log <HOI4 error.log>` or `--error-log-report error_log_report.json` after an in-game launch; the bundle will merge validator failures and runtime log diagnostics into the same failed output pack. Add `--package <work-package-id> --changed <path>` or `--package <work-package-id> --from-git` to also run the work-package boundary gate and include boundary violations in the repair prompt. A validation, logic-audit, loc-audit, gfx-audit, or boundary finding is expected and does not stop the bundle; other infrastructure errors still stop.
When `ai-repair-bundle` writes under `.hoi4skill/repair_bundle*`, `large-mod-release-gate`, `large-mod-release-bundle`, and `large-mod-release-brief` automatically collect `repair_bundle.json`, nested `ai_repair_context.json` / `validation_repair_context.json` / `repair_context.json`, plus nested validation/audit reports. `status=repair_prompt_ready`, `status=needs_repair`, `validation_ok=false`, `effective_errors>0`, or non-empty `repair_items` is blocking release evidence; do not publish while a repair bundle is still asking the AI to fix generated code.

For large-mod work packages, run `check-work-package-boundary --from-git --strict-names --fail-on-violation` after writing so the boundary gate checks the actual git working tree, not an AI-supplied changed-file list, and exits nonzero when files leave the package boundary. Use `identify-work-packages --from-git` or `split-changed-work-packages --from-git` when the changed files should be routed to packages automatically.

Before release, run `prepare-edit-context --input <request-or-workbook> --mod-root <mod-root> --tag <TAG> --prefix <prefix> --game-root <HOI4 root> --output .hoi4skill/edit_context.md`, `check-text-alignment --mod-root <mod-root> --input <request-or-workbook> --output .hoi4skill/text_alignment.json`, `large-mod-dependency-graph --mod-root <mod-root> --output .hoi4skill/dependency_graph.json`, `validate <mod-root> --game-root <HOI4 root> --strict-code-index --output .hoi4skill/validation.json`, `analyze-error-log --input <error.log> --mod-root <mod-root> --output .hoi4skill/error_log_report.json` after a real game launch, `large-mod-merge-gate --mod-root <mod-root> --output .hoi4skill/merge_gate.json`, `resolve-mod-dependencies <mod-root> --output .hoi4skill/mod_dependencies.json` plus `mod-knowledge <mod-root> --output .hoi4skill/mod_knowledge.json` for dependency-backed submods, `large-mod-playtest-gate --mod-root <mod-root> --output .hoi4skill/playtest_gate.json`, then `large-mod-release-gate --mod-root <mod-root> --output .hoi4skill/release_gate.json`. A passed package playtest report must include `validation_report` and `error_log_report`; otherwise playtest gate adds `playtest_missing_release_evidence`. The release gate requires clean `ai_context_contract.json`, `text_alignment.json`, `dependency_graph.json`, a final `validation.json` with `schema=hoi4skill.validation_report.v1`, `strict_code_index=true`, and non-null `game_root`, plus clean `error_log_report.json`, `merge_gate.json`, and `playtest_gate.json`; clean context contract means `schema=hoi4skill.ai_context_contract.v1`, `write_gate_status=READY_FOR_NARROW_WRITE`, `strict_code_index=true`, `final_code_allowed_by_context=true`, non-empty `allowed_edit_surface`, non-empty `verification_steps`, and empty `unknown_facts` / `blocked_until_verified`. Clean text alignment means `schema=hoi4skill.text_alignment.v1`, `ok=true`, and `missing_count=0`, so user-provided titles/descriptions/tooltips were not dropped. If `descriptor.mod` declares dependencies, it also requires `mod_dependencies.json` / `dependency_resolution.json` / `resolved_dependencies.json` with every dependency `status=resolved`, plus `mod_knowledge.json` / `knowledge_base.json` so submod syntax and dependency facts are not guessed. It automatically reads `.hoi4skill/plan_*.json`, `ai_context_contract.json`, `context_contract.json`, `placeholder_plan.json`, `author_placeholder_plan.json`, `gfx_register*.json`, `gfx_registration*.json`, `gfx_report.json`, `intent.json`, `intent_compile.json`, `compile_intent.json`, `dynamic_modifier_change_plan.json`, `ai_repair_context.json`, `validation_repair_context.json`, and `repair_context.json`; draft work-package plans from `generate-work-package --dry-run` carry `final_code_allowed=false` and must block release until the package is actually written, checked, handed off, and the stale plan is removed or replaced by clean evidence. Any `playtest_complete=false`, `final_code_allowed_by_context=false`, `final_code_allowed=false`, `can_apply=false`, `status=blocked`, `status=missing`, `status=ambiguous`, `status=unresolved`, `status=needs_repair`, `status=skipped`, `status=draft`, `status=needs_input`, `status=questions_required`, non-empty `questions`, non-empty `asset_questions`, non-empty `country_questions`, non-empty `repair_items`, non-empty `skipped_assets`, non-empty `unknown_facts`, non-empty `blocked_until_verified`, non-empty `blockers`, or non-empty `errors` blocks release even if a report also says `ok=true`.
For large-mod package authoring, start with `run-work-package --mod-root <mod-root> --package <package_id> --request "<literal user request>" --game-root <HOI4 root> --output-dir .hoi4skill/work_package_runs/<package_id>`. This creates a guarded authoring pack and a `hoi4skill.work_package_run.v1` manifest with six content lanes: national focuses, event chains, decisions, national spirits, dynamic modifiers, and localisation. Treat the manifest as an execution contract: models may provide intent/layout/cards/text only, while `compile-intent`, `apply-focus-layout`, `apply-event-cards`, `apply-feature-cards`, `plan-dynamic-modifier-change`, and localisation/token checks must produce or verify final assets before any handoff or release claim.
The release gate scans every occurrence of these fields inside a report, not only the top-level summary. A report with top-level `ok=true` or `blocking_count=0` is still blocking if any nested check says `ok=false`, a gate boolean is false, a known issue or warning counter is nonzero, a `warnings` array is non-empty, or a nested `status` is `blocked` / `skipped` / `draft` / `needs_input` / `needs_review` / `warnings` / `errors` / `failed` / `question_required`, regardless of ordinary JSON whitespace around the field separator.
Malformed report files also block release: empty files, non-object JSON, unclosed delimiters, unterminated strings, invalid string escapes, missing commas or colons, trailing commas, trailing garbage, or multiple top-level JSON values are treated as `needs_review` evidence, not as missing harmless metadata.
Every JSON report collected by the release gate or release bundle must also carry the expected `hoi4skill.*` schema for its filename/report kind. Empty `{}` reports, schema-less reports, external fake schemas, or mismatched schemas such as `loc_audit.json` carrying `hoi4skill.gfx_audit.v1` or `ci_plan.json` carrying `hoi4skill.large_mod_playtest_plan.v1` are blocking evidence rather than clean proof.

Final validation with `--strict-code-index` / `--final-check` also checks localisation control tokens. It rejects raw author placeholders such as `【中华民国领导人】`, unclosed colours, unregistered `£icon` tokens, unindexed `[TAG.GetName]` country scopes, and Chinese/non-ASCII scripted localisation scopes that should have been compiled from author placeholders first. When these fail, run `validate-repair-context`; localisation token failures are grouped as `localisation_token_mapping` and include explicit questions for the user about missing GFX sprites, country tags, or cosmetic tag aliases.

`plan-history-edit` is the required gate before direct `history/countries` or `history/states` work. It reads local state files, `map/definition.csv`, dependency roots supplied with `--mod-path`, and optional `--game-root` index facts; then reports whether state IDs, province IDs, and capital province IDs are known. If facts are missing, it returns skipped reasons instead of guessing. For focus rewards and temporary changes, prefer the generated state-scoped scripted-effect strategy.

For event cards, the writer scans existing namespaces first. A matching namespace is appended in place with the next safe event number, while new namespaces use `events/<prefix>_events.txt`. Re-running the same generated card is idempotent because the event block carries a stable `hoi4skill_card` marker.

After writing event chains, run `logic-audit --changed-only --changed <event-file>` alongside validation. It reports broken follow-up event IDs, triggered-only events with no local entry, unsafe event-chain cycles, and empty event options where a button has no gameplay effect, hidden effect, tooltip effect, or follow-up event.

For new custom art under `gfx/interface`, run `register-gfx-icons` before referencing sprite keys in focuses, ideas, events, decisions, or special GUI. The command first translates and renames non-English image filenames to semantic English names, leaves already-English filenames unchanged, then writes generated `interface/<prefix>_*.gfx` files for dynamic GUI icons, focus icons, idea pictures, event pictures, and decision/category pictures. It reverse-lookups existing `texturefile` registrations and avoids sprite-name collisions by reusing exact same-name/same-texture sprites or appending a numeric suffix when a sprite name already points to a different image. The JSON report includes `hoi4skill.gfx_registration.v1`, the original texture path, new English filename, and remarks for each registered sprite; write it under `.hoi4skill/gfx_register*.json`, `.hoi4skill/gfx_registration*.json`, or `.hoi4skill/gfx_report.json` so release gates block on `assets_skipped` / `skipped_assets` instead of letting an AI invent sprite names.

For decision cards, the writer scans existing decision categories, decision files, and Simplified Chinese localisation first. A matching target-country category is reused when safe; otherwise it creates the generated category and decision file.

For national-spirit cards, the writer scans existing target-country `common/ideas` files first. A safe country-wrapper file is reused, while large shared minister/advisor/character idea files are skipped; otherwise it creates `<prefix>_ideas.txt`.

For focus layouts, the writer scans existing target-country focus trees and all existing focus IDs before writing. It extends an existing target-country tree when one exists, shifts rows below the current max `y`, and renames generated IDs before writing if any ID already exists elsewhere in the mod. For missing focus icons, pass `--game-root` and dependency `--mod-path` so the writer can reuse verified focus sprites from `interface/goals*.gfx`; otherwise unresolved icons must stay `GFX_goal_unknown` instead of falling back to guessed sprite names.

For technology cards, the writer creates a minimal unique-technology skeleton under `common/technologies/<prefix>_technologies.txt`, with IDs ending in `_tech`.

For special-GUI cards, the writer creates a conservative skeleton only: `common/scripted_guis/<prefix>_scripted_guis.txt` plus `interface/<prefix>.gui`, with IDs ending in `_gui`. Do not treat this as a complete complex GUI; inspect the target mod before wiring variables, buttons, scripted loc, or custom views.

For scripted-effect and scripted-trigger cards, the writer creates `common/scripted_effects/<prefix>_scripted_effects.txt` or `common/scripted_triggers/<prefix>_scripted_triggers.txt`, with IDs ending in `_effect` or `_trigger`. Unresolved natural-language code may appear only in draft context; final output with `--strict-code-index` / `--final-check` must reject every generated `TODO` marker instead of treating it as accepted code.

For state-effect cards, the writer creates `common/scripted_effects/<prefix>_state_effects.txt`, with IDs ending in `_state_effect`. It uses `州ID` when supplied, otherwise it emits a state-scope helper and leaves state-name resolution notes instead of editing `history/states` directly.

`generate-mod` is the one-sentence path. It scaffolds a new mod folder, infers country tags, converts the sentence into internal focus/decision/national-spirit/event/technology/special-GUI/scripted-helper/state-effect cards, writes files, then embeds validation in the report. When `--source-root`, `--game-root`, or `--mod-path` is supplied, it first reads `localisation/**/*.yml`, matches country names against the request, and verifies the tag through `common/country_tags` plus the mapped `common/countries` file before falling back to the built-in common-country table.

`idea-copy-prompt` learns national-spirit copywriting from `common/ideas` plus Simplified Chinese localisation. By default it filters to `country` and `hidden_ideas` so advisor, designer, and law copy does not pollute national-spirit style.

`event-style-profile` is the safe style-learning path for user-selected reference mods. It scans event scripts and localisation, then emits `hoi4skill.event_style_profile.v1` as Markdown or JSON: length statistics, option cadence, event-type mix, scene cues, built-in template contract, and anti-copy rules. Use this profile as AI context before writing event chains. Do not paste full sampled event prose into prompts unless the user explicitly requests direct quotation; generated events still go through `apply-event-cards --final-check` and strict validation.

`event-copy-prompt` is the richer event-writing prompt when the model needs examples and built-in templates. Prefer `event-style-profile --format json` for automated pipelines and `event-copy-prompt` for human review or a single assisted drafting session.

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
- Treat game, dependency Mod, and target Mod roots in load order. Normal indexing and validation must prune lower-layer paths masked by a higher layer's `replace_path`. Use `--replace-path-diagnostics` only while adapting to a game update; it reads and hashes masked files for comparison but never makes their symbols valid again. Leave it off for routine work.
- When adding state buildings, choose explicit state IDs only after inspecting existing targets or vanilla state files.
- Keep balance conservative and easy for players to understand.
- Use comments sparingly and only to explain non-obvious compatibility or placement choices.
- When an effect, trigger, modifier, or scope is uncertain, look it up in `references/wiki-code-index.md` and the user's local HOI4 `documentation/*.md` before writing it.

## Validation Discipline

Always run the validator after generating files. Treat missing localisation UTF-8 BOM as a fatal validation error, fix it before reporting success, and do not claim the mod is fully tested in HOI4 until the relevant in-game `error.log` has been checked.

After an in-game launch test, analyze the current HOI4 `error.log` with `hoi4skill analyze-error-log`. Summarize new errors and repair hints for the user; do not treat static validation as a substitute for the in-game log.

Do not claim Workshop readiness unless `descriptor.mod`, launcher `.mod` metadata, thumbnail policy, and an in-game launch test have all been handled.
