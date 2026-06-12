# HOI4 Agent Rules

This repository is for the `hoi4-mod-maker` skill. Read `hoi4-mod-maker/SKILL.md` first, then `hoi4-mod-maker/references/ai-clausewitz-guardrails.md`.

Hard rules for any AI agent:

- Use the Rust `hoi4skill` binary for workbook import, generation, validation, resource lookup, and error-log analysis.
- Do not use Python, `py`, `python3`, `pip`, `conda`, `uv`, PowerShell `ImportExcel`, or ad-hoc helper scripts for HOI4 mod generation.
- Before generated HOI4 content that needs game evidence, run `hoi4skill detect-hoi4-path`. If no valid `selected` path is returned, ask only for the Hearts of Iron IV install path.
- Never offer manual mod creation, unvalidated generation, or skipping `hoi4skill validate` as a fallback.
- For national focuses, decisions, events, and national spirits, models provide structured layout/cards; Rust writers emit the Clausewitz files.
- Do not create country tags, country history, state history, initial units, characters, English localisation, GUI, technologies, or other extra systems unless the literal user request authorizes them.
