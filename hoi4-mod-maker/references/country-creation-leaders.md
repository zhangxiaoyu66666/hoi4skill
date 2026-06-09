# Country Creation And Country Leaders

Use this when the user asks to create a country, add country leaders, add leader traits, or edit a country's starting history.

## Required Country Files

A playable country is not just a tag. Create or update the complete set:

- `common/country_tags/<file>.txt`: maps the tag to a country definition, for example `FER = "countries/FarEasternRailway.txt"`.
- `common/countries/<CountryFile>.txt`: graphical cultures and map color.
- `history/countries/<TAG> - <Name>.txt`: capital, politics, popularities, technologies, ideas, OOB, characters, and starting leaders.
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`: country name keys such as `TAG`, `TAG_DEF`, `TAG_ADJ`, cosmetic names, focuses, ideas, decisions, events, technologies, and GUI text.

Optional but common:

- `common/characters/<TAG>.txt`: modern character records for leaders, advisors, generals, field marshals, navy leaders, and scientists.
- `common/country_leader/<prefix>_leader_traits.txt`: country leader trait definitions.
- `gfx/leaders/<TAG>/...` and matching `interface/*.gfx` sprites when the target style uses registered portrait sprites.

## Portrait Resource Rules

Leader portraits are verified resources, not names to invent. When `--game-root` or dependency `--mod-path` is available, run or rely on `build-game-index`/`prepare-edit-context` and choose from indexed `leader_portraits` (`GFX_portrait_*`). Match by country tag/name, leader name when present, role words such as president/chairman/king/general/advisor, and ideology words such as democratic, communist, fascist, monarchist, non-aligned, anarchist, or syndicalist.

Modern `common/characters` portrait references keep the registered sprite name:

```hoi4
portraits = {
  civilian = {
    large = GFX_portrait_FER_alexei_smirnov
  }
}
```

Legacy `create_country_leader` blocks may use a direct file path only when that exact `gfx/leaders/...` asset exists or is explicitly supplied by the user:

```hoi4
picture = "gfx/leaders/CPC/Lidazhao.dds"
```

Do not create placeholder names such as `GFX_portrait_TAG_leader` or `gfx/leaders/TAG/leader.dds` unless the matching sprite/file is actually created and registered.

## Country Leader Traits

Country leader traits are defined under `common/country_leader/*.txt`, normally in a wrapper:

```hoi4
leader_traits = {
  my_reformer_trait = {
    random = no
    political_power_factor = 0.10
    stability_factor = 0.05
  }
}
```

These are leader traits, not national spirits. Do not add `_idea` to country leader trait IDs. Localise player-visible trait IDs and descriptions in the target TAG localisation file when the trait will be shown to players.

## Modern Leader Style

For standalone mods, default to the modern character style unless the user explicitly requests legacy syntax:

```hoi4
characters = {
  FER_alexei_smirnov = {
    name = FER_alexei_smirnov
    portraits = {
      civilian = {
        large = GFX_portrait_FER_alexei_smirnov
      }
    }
    country_leader = {
      ideology = conservatism
      traits = { fer_railway_reformer_trait }
      expire = "1965.1.1.1"
      id = -1
    }
  }
}
```

Then recruit the character from `history/countries/<TAG> - <Name>.txt`:

```hoi4
recruit_character = FER_alexei_smirnov
```

This style keeps leaders, advisors, and commanders in one structured file and is the safest default for new independent mods.

## Legacy Leader Style

Some large mods and older files create leaders directly inside history country files:

```hoi4
create_country_leader = {
  name = "李大钊"
  desc = "POLITICS_CPC_LIDAZHAO_DESC"
  picture = "gfx/leaders/CPC/Lidazhao.dds"
  expire = "1965.1.1"
  ideology = Li_Dazhaoism
  traits = { CPC_Socialist_pragmatism CPC_Nationalism }
}
```

Use this only when the target mod or dependency mod already uses this pattern, or when the user explicitly asks for this syntax.

## Standalone Versus Submod Rule

- `standalone_mod`: create countries with modern `common/characters` plus `recruit_character` by default. Use legacy `create_country_leader` only if the user specifies it or the existing standalone mod already consistently uses it.
- `submod`: follow the dependency mod's observed country and leader syntax. If the dependency uses legacy `create_country_leader`, write compatible legacy history blocks. If the dependency uses modern `common/characters`, add matching character records and recruit them. If dependency roots were not indexed with `--mod-path`, report the syntax as unknown instead of guessing.

`hoi4skill mod-knowledge` reports:

- `country_tag_mappings`
- `country_definition_files`
- `country_leader_traits`
- `characters`
- `history_character_uses`
- `legacy_country_leaders`
- `country_creation_syntax`
- `dependency_country_creation_styles`

Use these fields before generating country or leader files.

## Hard Rules

- Country tags are exactly three ASCII uppercase letters.
- Scripted IDs for characters and leader traits must be ASCII identifiers.
- Do not invent `SOV_12347.png` style portrait names. If an image filename needs translation, use a semantic English name based on the local filename and report skipped assets when translation is not safe.
- Do not invent `GFX_portrait_*` references; use indexed leader portraits, local `interface/*.gfx` evidence, or a verified legacy `gfx/leaders/...` picture path.
- Do not confuse country leader traits with national spirits. National spirits end with `_idea`; leader traits do not.
- Do not generate mod display-name localisation such as `<prefix>_mod_name`; mod names belong in `descriptor.mod` and the launcher-side `.mod`.
- Do not claim a created country is complete until tag mapping, common country definition, history country file, and localisation are all present.
