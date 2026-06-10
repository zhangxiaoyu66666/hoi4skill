# Local Clausewitz Code Library

The local library solves a different problem from `build-game-index`.

- `build-game-index` answers whether a TAG, ID, sprite, modifier, technology, state, or province exists.
- `build-clausewitz-library` answers how real HOI4 files structure and nest the relevant code.

The vanilla library is always built from the user's installed game. Dependency roots supplied with `--mod-path` are factual indexes only and never enter the code library automatically.

A mod code layer is loaded only when the user's literal request explicitly asks to load, reference, or imitate that specific mod's code and its path is supplied with `--code-mod-path`. It is stored separately and searched before the vanilla layer without replacing it.

## Build

```text
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV"
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV" --code-mod-path "M:\path\requested_mod" --request "加载 requested_mod 的模组代码作为参考"
```

The default vanilla location is `%LOCALAPPDATA%\hoi4skill\clausewitz-library` on Windows and the user's cache directory elsewhere. Authorized mod layers are stored under the adjacent `mod-code-libraries` directory. Use `--output` for a different vanilla location.

## Query

```text
hoi4skill query-clausewitz-library --system focus --query "socialist workers revolution"
hoi4skill query-clausewitz-library --system event --query "uprising country event"
hoi4skill query-clausewitz-library --system idea --query "planned economy national spirit"
hoi4skill query-clausewitz-library --system decision --query "organize resistance"
```

The output contains exact source paths and complete examples. Use syntax and nesting as evidence, but do not copy IDs, prose, balance, or unrelated mechanics.

## Automatic Context

`prepare-edit-context` automatically builds the vanilla library when `--game-root` is supplied and no library exists. It does not load dependency mod code merely because `--mod-path` is present. When the user explicitly requests a mod code reference, supply the same literal request plus `--code-mod-path`; the authorized mod layer is then searched first.

If no semantic match exists, retrieval returns a concise real example from the same system. The model must not replace a missing match with invented syntax.

The Rust generator remains the final writer for focuses, ideas, events, and decisions. Retrieved code is used to understand the language and to extend missing generator capabilities safely.
