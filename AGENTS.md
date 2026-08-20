# Codex Workspace Rules

These rules are for this workspace only. Do not copy them into the user's global
OpenCode/Codex/Claude skill directories while testing.

For HOI4 work in this repository:

- Use the Rust `hoi4skill` binary for workbook import, generation, validation,
  resource lookup, and error-log analysis.
- If the user says not to use Python, do not run `python`, `py`, `python3`,
  `pip`, `conda`, `uv`, PowerShell `ImportExcel`, or ad-hoc helper scripts for
  the HOI4 workflow.
- Before generated HOI4 content that needs game evidence, run
  `hoi4skill detect-hoi4-path`. If no valid `selected` path is returned, ask
  only for the Hearts of Iron IV install path.
- When a task needs Clausewitz syntax or gameplay effect evidence, use the
  local game index and `hoi4skill clausewitz-reference --game-root <HOI4 root>`
  or `prepare-edit-context --game-root <HOI4 root>` before final script output.
- Do not offer manual mod creation, unvalidated generation, or skipping
  `hoi4skill validate` as a fallback.
- For national focuses, decisions, events, and national spirits, models provide
  structured layout/cards; Rust writers emit the Clausewitz files.
- When the user provides a source file, workbook, focus title, event title,
  decision title, national-spirit title, or other player-visible text, run
  `hoi4skill check-text-alignment` or `hoi4skill validate --text-source <file>`
  before the final answer. Missing user-provided text is an unfinished result.
- For localisation translation, persist the first approved rendering of every
  recurring institution, title, ideology, place, and proper noun with
  `hoi4skill localisation-glossary`. Regenerate prompts after adding terms and
  run the whole-target `--check`; never bypass or hand-wave a glossary mismatch.
- Final HOI4 output must be checked against the local game/dependency codebase:
  use `hoi4skill validate <mod> --game-root <HOI4 root> --strict-code-index`
  (or `run-workflow ... --game-root <HOI4 root> --final-check`). If a generated
  effect, modifier, sprite, picture, technology, tag, or other indexed symbol is
  absent from the code index, report it as a bug the AI must fix.
- If a low-level writer is used directly (`apply-focus-layout`,
  `apply-focus-excel`, `apply-feature-cards`, or `apply-event-cards`), pass
  `--game-root <HOI4 root> --final-check` so the command cannot bypass final
  code-index and text-alignment checks.
- Do not create country tags, country history, state history, initial units,
  characters, English localisation, GUI, technologies, or other extra systems
  unless the literal user request authorizes them.
