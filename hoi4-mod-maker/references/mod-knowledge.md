# Mod Knowledge Before Edits

Use this whenever the user asks to modify an existing HOI4 mod or submod.

## Required First Step

Run:

```text
hoi4skill mod-knowledge "M:\path\mod_or_launcher.mod" --output mod_knowledge.json
```

If the target declares dependencies, pass each available dependency root or launcher file:

```text
hoi4skill mod-knowledge "M:\path\submod.mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
```

Do not edit from memory alone. The generated `mod_knowledge.json` is the factual boundary for the next step.

## What It Must Determine

- `mod_kind`: `standalone_mod`, `submod`, or `unknown_no_descriptor`.
- `descriptor`: local `descriptor.mod` path, metadata, tags, and dependency names.
- `launcher_mod_files`: launcher-side `.mod` files that point at the mod root.
- `dependency_names` and `dependency_mod_roots`.
- existing country tags, tag-to-country-file mappings, `common/countries` files, and history-country files.
- country creation and leader syntax: `country_creation_syntax`, `dependency_country_creation_styles`, `country_leader_traits`, `characters`, `history_character_uses`, and `legacy_country_leaders`.
- local history-state and province facts: `history_state_files`, `history_states`, and `province_definitions`.
- focus trees, focus ID prefixes, focus icons, and current tree country tags.
- event namespaces, files, event-type counts, and highest namespace numbers.
- decision categories and national-spirit picture usage.
- localisation language headers, BOM style, and sample localised content.
- registered GFX sprites and texture paths.
- local content-file samples that were actually read.

## Anti-Hallucination Contract

The AI may use only facts observed in `knowledge_base`, `markdown_summary`, target files it has just read, or dependency/game indexes it has explicitly built.

If a tag, namespace, sprite, technology category, state ID, province ID, scripted effect, country file, country leader trait, character ID, leader syntax, or localisation key is absent, say it is unknown and verify locally before using it.

For submods, dependency names in `descriptor.mod` or launcher `.mod` are not enough. If the requested content depends on inherited tags, sprites, technologies, scripted effects, state IDs, province IDs, or event namespaces, build or validate with `--mod-path` dependency roots or `build-game-index` before claiming the reference exists.

For country creation and country leader work, standalone mods default to modern `common/characters` plus `history/countries` `recruit_character` unless the user asks for legacy syntax. Submods must follow `dependency_country_creation_styles`: if the dependency uses legacy `create_country_leader`, generate compatible legacy blocks; if it uses modern `common/characters`, generate compatible character records and recruit them. If no dependency root was indexed, report country/leader syntax as unknown instead of guessing.

Mod display names are not country-content localisation. Keep names in `descriptor.mod` and the launcher-side `.mod`; never create `*_mod_name` keys under `l_simp_chinese:`.

State/province work must respect `history_states` and `province_definitions`. `capital` in `history/countries` is a province ID. If the target mod has no local `history/states` or `map/definition.csv`, report state/province facts as unknown locally and index the dependency/game root or ask for explicit IDs.

## How To Use The Summary

Use `markdown_summary` as the first context block for a model-generated edit plan. It should inform:

- whether this is a standalone mod or submod,
- which dependencies must be respected,
- which country and country leader syntax is observed locally and in dependencies,
- which history-state files, state IDs, province samples, buildings, resources, and province-definition facts are observed locally,
- which file conventions are already present,
- which ID prefixes and namespaces should be extended,
- which localisation file style is safe,
- which sprite names can be referenced,
- which facts still need verification.

When the summary says `unknown_no_descriptor`, stop and ask for the real mod root or launcher `.mod` file.

For complex existing-mod edits, prefer generating a complete model context pack:

```text
hoi4skill prepare-edit-context --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output edit_context.md
```

Read `edit_context.md` before writing. Its `Write Gate` is the first boundary: `BLOCKED` means stop, `VERIFY_FIRST` means collect the listed evidence first, and `READY_FOR_NARROW_WRITE` still allows edits only inside the reported edit surface. Its `Unknown Facts` and `Blocked Until Verified` sections explain the missing evidence behind that gate.
