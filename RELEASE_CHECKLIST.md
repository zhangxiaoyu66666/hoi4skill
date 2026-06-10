# Release Checklist

Use this before publishing a public source archive or GitHub repository.

## License

- `LICENSE` contains GNU GPL v3 text.
- `hoi4skill-cli/Cargo.toml` uses `license = "GPL-3.0-only"`.
- `README.md` states GPL-3.0-only.
- Third-party notices are listed in `THIRD_PARTY_NOTICES.md`.

## Files To Exclude

- `hoi4skill-cli/target/`
- `_scratch/`
- local Steam or HOI4 game directories
- copied workshop mods or user mods without redistribution permission
- private logs, absolute local paths, and temporary JSON reports

## Validation

```text
cd hoi4skill-cli
cargo fmt
cargo test --release
cargo clippy --release -- -D warnings
cargo build --release
```

## Smoke Test

```text
target\release\hoi4skill.exe --help
target\release\hoi4skill.exe doctor-skill-install
target\release\hoi4skill.exe build-clausewitz-library --game-root "<HOI4>" --output "..\_scratch\clausewitz-library"
target\release\hoi4skill.exe query-clausewitz-library --library "..\_scratch\clausewitz-library" --system event --query "country event"
target\release\hoi4skill.exe scaffold --name "GPL Smoke Test" --output "..\_scratch\release-smoke" --launcher-file
target\release\hoi4skill.exe validate "..\_scratch\release-smoke"
```

## Release Notes

Mention:

- CLI version,
- license,
- major commands added,
- known limits,
- that the project is unofficial and ships no Paradox assets.
