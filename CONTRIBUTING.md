# Contributing

Thanks for helping improve hoi4skill.

## Ground Rules

- Keep the project Rust-only for public CLI functionality unless a new dependency is clearly justified.
- Do not add PowerShell or Python helper scripts as required runtime paths.
- Do not commit Paradox game files, workshop mods, local test dumps, or generated `_scratch` output.
- Preserve Simplified Chinese authoring support and UTF-8 localisation behavior.
- Add or update tests for generator, validator, parser, and workflow changes.

## Development Checks

Run these before sending a patch:

```text
cd hoi4skill-cli
cargo fmt
cargo test --release
cargo clippy --release -- -D warnings
cargo build --release
```

## License Of Contributions

By contributing, you agree that your contribution is licensed under GPL-3.0-only, the same license as this repository.

If your change incorporates third-party code, examples, images, or data, document the source and license in `THIRD_PARTY_NOTICES.md` and keep the original notices intact.
