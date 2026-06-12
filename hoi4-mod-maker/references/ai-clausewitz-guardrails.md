# AI Clausewitz Guardrails

Use this reference when a Codex, ChatGPT, Claude, Cursor, OpenCode, or other model is asked to create or edit HOI4 script. The goal is not to make the prompt scarier. The goal is to remove the model's opportunity to invent Clausewitz code.

Core rule:

```text
The model plans. Rust generators write template-owned Clausewitz. Validation decides.
```

Prompt memory, internet snippets, and a plausible-looking block are not evidence.

## What Counts As Unsafe Manual Script

Manual Clausewitz writing means any model-authored patch that directly changes game-script structure, field names, IDs, wrappers, or effect/trigger nesting in these high-risk surfaces:

- `common/national_focus`
- `common/decisions`
- `common/ideas`
- `events`
- `history/countries`
- `history/states`
- `common/scripted_effects`
- `common/scripted_triggers`
- `common/scripted_guis`
- `interface/*.gfx`
- `interface/*.gui`

National focuses, decisions, events, and national spirits are stricter than the rest. They are template-owned systems. The model may provide only structured layout/card data, then run the matching Rust writer. It must not patch the generated blocks by hand unless the user's literal request says to handwrite Clausewitz code or directly edit that script file.

Requests such as "create a mod", "make it complete", "fix it", "continue", "add content", "generate a focus tree", or "repair this feature" are not manual-edit authorization.

## Failure Modes To Block

| Failure mode | Typical bad output | Required gate |
| --- | --- | --- |
| Near-match field names | `mutual_exclusion`, `completion_rewards`, `ai_willdo` | `hoi4skill validate` must reject near matches. |
| Wrong scope | `add_building_construction` directly inside `completion_reward` | Use state-scoped helpers or verified state scope. |
| Invented resources | `GFX_goal_new_revolution`, fake idea pictures, fake portraits | Use indexed `interface/*.gfx` sprites or report missing evidence. |
| TAG confusion | Treating a faction, route, committee, or file prefix as a country TAG | Run the Country TAG Evidence Gate before writers. |
| State/province confusion | `capital = 64` because state 64 exists | Run `plan-history-edit`; remember `capital` uses a province ID. |
| Focus layout drift | `relative_position_id = <parent>` plus worksheet absolute `x/y` | Use the CLI writer's opening-focus anchor. |
| Unauthorized mod imitation | Loading dependency mod code because `--mod-path` was supplied | Use `--code-mod-path` only when the user explicitly asks to load/reference/imitate that mod's code. |
| Tool bypass | Checking Python, installing ImportExcel, or writing helper scripts after the user or skill forbids it | Use Rust `hoi4skill` commands only; the workbook importer is `parse-focus-excel` / `apply-focus-excel`. |
| Missing game root fallback | Asking to manually create a mod without hoi4skill validation because HOI4 was not found | Run `detect-hoi4-path`; if no valid path is selected, ask only for the HOI4 install path. |
| Placeholder completion | `TODO`, `描述`, `具体效果待补充`, generic focus text | Player-facing localisation must be finished before success is claimed. |
| Scope creep | New country files, state history, units, GUI, English loc created by a simple new-mod request | Requirement Scope Contract and request-scope validation. |
| False success | Static syntax passed but game `error.log` was never checked | Report static validation separately from in-game clean status. |

## Required Pipeline

1. Copy the literal user request into a Requirement Scope Contract.
2. Run `hoi4skill detect-hoi4-path`. If it returns no valid `selected`, ask only for the HOI4 install path and stop. Do not offer manual creation, unvalidated generation, or skipping `hoi4skill validate`.
3. Resolve the target country from local evidence, not from a hard-coded list:
   `hoi4skill resolve-country-tag --text "<request>" --game-root "<HOI4 root>" --source-root "<source mod>" --mod-path "<dependency>"`
4. Build the existing-mod dossier:
   `hoi4skill mod-knowledge "<mod-root-or-launcher.mod>" --mod-path "<dependency>" --output mod_knowledge.json`
5. With the game root, index game/dependency resources and retrieve real code:
   `hoi4skill prepare-edit-context --input "<copy-or-workbook>" --request "<request>" --mod-root "<mod-root>" --tag <TAG> --prefix <prefix> --game-root "<HOI4 root>" --mod-path "<dependency>" --output edit_context.md`
6. Read these `edit_context.md` sections before any write:
   - `Requirement Scope Contract`
   - `Write Gate`
   - `Retrieved Clausewitz Code Library`
   - `Dry Run Plan`
   - `Unknown Facts`
   - `Blocked Until Verified`
7. Convert the request to structured input:
   - focus layout or Excel grid,
   - decision/national-spirit/technology/special-GUI/scripted-helper/state-effect cards,
   - event cards,
   - country/leader cards only when explicitly authorized.
8. Apply with the matching Rust writer. Do not manually edit generated focus, decision, event, or national-spirit blocks.
9. Validate with the indexed root:
   `hoi4skill validate "<mod-root>" --game-root "<HOI4 root>" --mod-path "<dependency>" --request "<request>"`
10. If the user launches the game, analyze the relevant log:
   `hoi4skill analyze-error-log --input "<HOI4 user folder>\\logs\\error.log" --mod-root "<mod-root>" --output error_report.json`

If a model cannot run these commands, it should output the structured cards and say which command must be run next. It must not fill the gap by inventing Clausewitz script.

## Writable Surface Matrix

| System | Model may author | Writer / gate | Manual script policy |
| --- | --- | --- | --- |
| National focuses | Titles, descriptions, ID hints, layout, prerequisites, mutual exclusions, icon meaning, reward intent | `parse-focus-layout`, `parse-focus-excel`, `apply-focus-layout`, `apply-focus-excel`, `run-workflow` | Forbidden unless explicit direct manual edit request. |
| Decisions | Decision cards: category, cost, duration, visible/available intent, complete effect intent | `parse-feature-cards`, `apply-feature-cards`, `run-workflow` | Forbidden unless explicit direct manual edit request. |
| National spirits | Idea cards: name, picture meaning, modifier intent, add/remove routing | `parse-feature-cards`, `apply-feature-cards`, `run-workflow` | Forbidden unless explicit direct manual edit request. |
| Events | Event cards: type, title, description, options, effect intent, trigger intent | `parse-event-cards`, `apply-event-cards`, `run-workflow` | Forbidden unless explicit direct manual edit request. |
| Scripted effects/triggers | Structured helper cards and verified small helper intent | `parse-feature-cards`, `apply-feature-cards`, `run-workflow`, local documentation lookup | Allowed only after retrieved code/docs prove scope and syntax; unresolved prose stays as TODO rather than fake code. |
| History states/countries | Explicit state/province/TAG facts and intended changes | `plan-history-edit`, `build-game-index`, `mod-knowledge` | Direct edits require observed local or indexed evidence. |
| GFX/interface | Semantic asset names, desired category, verified sprite selection | `register-gfx-icons`, `icon-preview`, `build-game-index` | Never invent sprite keys or random filenames. |
| Localisation | Finished player-facing text with stable keys | Writers plus `country-localisation-template` and validation | Allowed, but preserve keys, HOI4 tokens, language headers, and UTF-8 BOM. |

## Third-Party AI System Prompt Block

Paste this into another model's project instruction, `AGENTS.md`, `CLAUDE.md`, or equivalent:

```text
You are not allowed to freehand HOI4 Clausewitz script.

You are not allowed to use Python, pip, conda, uv, PowerShell ImportExcel, or ad-hoc helper scripts for this HOI4 workflow.

Before any HOI4 script write:
1. Read hoi4-mod-maker/SKILL.md.
2. Read hoi4-mod-maker/references/ai-clausewitz-guardrails.md.
3. Run `hoi4skill detect-hoi4-path`. If it cannot find a valid path, ask only for the HOI4 install path and stop.
4. Build or read mod_knowledge.json for the target mod.
5. For multi-system work, build edit_context.md with prepare-edit-context and obey the Write Gate.
6. Use only facts observed in the target mod, dependency/game indexes, retrieved Clausewitz examples, or the user's explicit request.

For common/national_focus, common/decisions, common/ideas, and events:
- You may author only structured layout/cards.
- The Rust hoi4skill writer must emit the final Clausewitz blocks.
- If the writer cannot express a needed mechanic, extend the writer or stop; do not bypass it with approximate script.
- A general request to create, complete, continue, fix, or add content is not permission for manual script editing.

Never invent:
- country TAGs,
- focus IDs based on grid positions such as focus_3_0,
- GFX sprite names,
- state IDs or province IDs,
- effect/trigger/modifier names,
- event namespaces,
- localisation keys that are not wired to generated script.

After generation, run indexed validation. Treat unresolved resources and validation warnings as unfinished work. Do not claim in-game clean status until the relevant HOI4 error.log has been checked.
```

## Patch Acceptance Checklist

Do not accept another AI's patch unless all of these are true:

- The changed file list matches the Requirement Scope Contract.
- The patch does not create unrelated country definitions, country history, state history, initial units, characters, English localisation, decisions, technologies, GUI files, or placeholders.
- For focuses, decisions, ideas, and events, the diff came from a Rust writer report, not freehand model text.
- `common/national_focus` has exact required fields and no position fallback IDs such as `focus_1_0`, `focus_2_1`, or `focus_3_0`.
- `relative_position_id` follows the generated opening-focus anchor rule.
- Sprites, portraits, technologies, buildings, ideologies, equipment, states, provinces, and modifiers are known from local files or indexed roots.
- Simplified Chinese localisation is grouped in the target TAG localisation file when generating country content.
- Player-facing titles and descriptions are finished prose, not implementation notes.
- `hoi4skill validate` passes with `--game-root` when a game install is available.
- Any in-game `error.log` errors are mapped back to the feature plan before patching.

## Recommended Repository Controls

Prompt rules help, but machine checks are better. For future hardening, prefer controls that make the wrong path inconvenient:

- Add a repo-level `AGENTS.md` that points to this file and the active `hoi4-mod-maker/SKILL.md`.
- Keep template-owned files protected in AI workflows: models submit cards; only `hoi4skill` writes the script.
- Store `workflow_plan.json`, `workflow_report.json`, or `edit_context.md` beside complex generated changes for audit.
- Reject diffs that touch high-risk folders without a matching writer report.
- Add a strict policy check that compares changed files against the literal request scope.
- Add validator rules for every recurring hallucination, not only the latest example.
- Make unresolved game resources fatal in release/Workshop mode.

## Practical Rule Of Thumb

If the model is about to type a `{` into a HOI4 script file, pause and ask:

1. Did local evidence prove this block shape?
2. Is this one of the template-owned systems?
3. Can a Rust writer emit it from structured input?
4. Will indexed validation catch wrong IDs, scope, and resources?
5. Is there an `error.log` repair loop after launch?

If any answer is no, the next step is evidence or generator work, not more Clausewitz text.
