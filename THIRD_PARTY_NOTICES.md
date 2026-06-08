# Third-Party Notices

hoi4skill itself is licensed under GPL-3.0-only.

## Reference Material

This public repository does not vendor third-party research material. Keep private research notes and reference checkouts outside the published Git history.

If future work directly copies, translates, or adapts third-party code, documentation, examples, images, or data, add an entry here with:

- source project or author,
- source URL,
- license,
- files affected,
- whether the material is copied, adapted, or only referenced.

## Rust Dependencies

The Rust CLI depends on open-source crates resolved by Cargo and does not vendor their source code in this repository.

- `calamine` by the calamine contributors, MIT license, used to read `.xlsx`, `.xls`, `.xlsm`, `.xlsb`, and `.ods` worksheet values for Excel-drawn focus trees.
- `zip` by the zip-rs contributors, MIT license, used only as a dev-dependency for synthetic `.xlsx` test fixtures.

## Game And Mod Assets

This repository must not redistribute Hearts of Iron IV game assets or third-party mod assets unless the relevant rights holder permits it.

Generated test data and tiny synthetic fixtures are acceptable when they are clearly original and not copied from the game or another mod.
