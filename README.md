# hoi4skill

Chinese-first Hearts of Iron IV mod authoring tools for Codex-style workflows.

This repository contains:

- `hoi4-mod-maker/`: the installable skill package and reference docs.
- `hoi4skill-cli/`: a Rust CLI backend with no Python or PowerShell runtime dependency.

## What It Does

The Rust CLI helps turn low-friction Chinese modding input into safer HOI4 files:

- scaffold a mod folder,
- scan an existing mod before edits,
- validate common HOI4 static mistakes,
- generate focus trees from plain text sketches,
- parse and apply decision, national spirit, event, technology, special GUI, scripted helper, and state-effect cards,
- register GFX icons under `interface/*.gfx`,
- build a game/mod index for tags, sprites, states, provinces, technologies, and other references,
- plan state/history edits without guessing state IDs or province IDs,
- analyze `error.log` for reverse repair.

## Build

```text
cd hoi4skill-cli
cargo build --release
```

The compiled binary is:

```text
hoi4skill-cli/target/release/hoi4skill.exe
```

Quick check:

```text
hoi4skill-cli/target/release/hoi4skill.exe --help
```

## Example Commands

```text
hoi4skill scaffold --name "My HOI4 Mod" --output "M:\path\my_mod" --launcher-file
hoi4skill mod-knowledge "M:\path\existing_mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\existing_mod" --tag SOV --prefix sov_nep --output workflow_report.json
hoi4skill plan-history-edit "M:\path\existing_mod" --text "edit history/states owner for state_id 64" --state-id 64 --game-root "C:\path\Hearts of Iron IV" --output history_plan.json
hoi4skill validate "M:\path\existing_mod" --game-root "C:\path\Hearts of Iron IV"
```

## Repository Hygiene For Public Releases

Do not publish local or copyrighted game data by accident:

- Do not commit `hoi4skill-cli/target/`.
- Do not commit `_scratch/`.
- Do not commit private research notes, local generated demos, or packaged archives.
- Do not commit local full game folders, Steam folders, or extracted HOI4 files.
- Do not commit third-party mods unless their license explicitly allows redistribution.

## License

hoi4skill is released under the GNU General Public License v3.0 only.
See [LICENSE](LICENSE).

Hearts of Iron IV and Paradox Interactive names are trademarks of their respective owners. This project is an unofficial modding tool and does not include game assets.
