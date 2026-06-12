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
- Do not offer manual mod creation, unvalidated generation, or skipping
  `hoi4skill validate` as a fallback.
- For national focuses, decisions, events, and national spirits, models provide
  structured layout/cards; Rust writers emit the Clausewitz files.
- Do not create country tags, country history, state history, initial units,
  characters, English localisation, GUI, technologies, or other extra systems
  unless the literal user request authorizes them.
