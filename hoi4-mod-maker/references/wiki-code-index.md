# HOI4 Wiki And Code Index

Use this file when a request needs HOI4 script names, effect syntax, trigger syntax, scope rules, or a system template that is not obvious from the target mod.

## Source Priority

1. Nearby files in the target mod.
2. Local HOI4 game documentation from the user's install.
3. The HOI4 Wiki or ParaWiki page for the relevant system.
4. `hoi4yaml` references when generating through the optional structured backend.

Do not rely on memory for effect, trigger, modifier, or DLC-specific names when a local documentation file is available.

## Local Game Documentation

HOI4 ships generated documentation with the game. Locate the user's install first, then substitute that path for `<HOI4>`:

```text
<HOI4>
```

Important files:

- `documentation/effects_documentation.md`: effect names, supported scopes, and examples.
- `documentation/triggers_documentation.md`: trigger names, supported scopes, and examples.
- `documentation/modifiers_documentation.md`: modifier names and expected value style.
- `documentation/dynamic_variables_documentation.md`: dynamic variable syntax.
- `documentation/script_concept_documentation.md`: bindable localisation, contextual localisation, collections, script constants.
- `documentation/loc_formatter_documentation.md`: localisation formatter names used in tooltips.
- `documentation/loc_objects_documentation.md`: localisation scope objects and properties.
- `common/decisions/_documentation.md`: decision block semantics and performance notes.
- `common/scripted_guis/_documentation.md`: scripted GUI structure.
- `common/on_actions/_documentation.md`: on action structure.
- `common/characters/_documentation.md`: character data structure.

Fast lookup commands:

```text
Select-String -Path "<HOI4>\\documentation\\effects_documentation.md" -Pattern "^## add_political_power" -Context 0,12
Select-String -Path "<HOI4>\\documentation\\triggers_documentation.md" -Pattern "^## has_completed_focus" -Context 0,12
Select-String -Path "<HOI4>\\documentation\\modifiers_documentation.md" -Pattern "^## stability_factor" -Context 0,12
```

If the user has a different install path, locate the game first and replace `<HOI4>`.

## Wiki Entry Points

Official Paradox Wiki:

- Event modding: https://hoi4.paradoxwikis.com/wiki/Event_modding
- Focus modding: https://hoi4.paradoxwikis.com/wiki/Focus_modding
- Decision modding: https://hoi4.paradoxwikis.com/wiki/Decision_modding
- Localisation: https://hoi4.paradoxwikis.com/wiki/Localisation
- Effects: https://hoi4.paradoxwikis.com/wiki/Effects
- Triggers: https://hoi4.paradoxwikis.com/wiki/Triggers
- Scopes: https://hoi4.paradoxwikis.com/wiki/Scopes

Chinese ParaWiki mirrors:

- 事件修改: https://hoi4.parawikis.com/zh-mo/Event_modding
- 国策制作: https://hoi4.parawikis.com/wiki/%E5%9B%BD%E7%AD%96%E5%88%B6%E4%BD%9C
- 决议修改: https://hoi4.parawikis.com/zh-hans/%E5%86%B3%E8%AE%AE%E4%BF%AE%E6%94%B9
- 本地化: https://hoi4.parawikis.com/zh-hans/%E6%9C%AC%E5%9C%B0%E5%8C%96
- 指令 / effects: https://hoi4.parawikis.com/wiki/%E6%8C%87%E4%BB%A4
- 条件 / triggers: https://hoi4.parawikis.com/wiki/%E6%9D%A1%E4%BB%B6
- 作用域 / scopes: https://hoi4.parawikis.com/wiki/%E4%BD%9C%E7%94%A8%E5%9F%9F

The official site may return a client challenge in some environments, and mirrors can be slow. When that happens, use the local game documentation as the authoritative code-name source and keep the Wiki URLs as human reading links.

## Common Lookup Targets

Use these names as starting points, then verify the exact syntax in the docs when generating non-trivial code.

Country effects:

- `add_political_power`
- `add_stability`
- `add_war_support`
- `add_ideas`
- `remove_ideas`
- `add_timed_idea`
- `country_event`
- `news_event`
- `set_country_flag`
- `clr_country_flag`
- `add_to_variable`
- `set_variable`

State effects:

- `add_building_construction`
- `add_extra_state_shared_building_slots`
- `add_core_of`
- `remove_core_of`
- `set_state_flag`
- `set_demilitarized_zone`

Country triggers:

- `tag`
- `original_tag`
- `has_completed_focus`
- `has_country_flag`
- `has_idea`
- `has_government`
- `has_war`
- `is_major`

State triggers:

- `is_core_of`
- `is_owned_and_controlled_by`
- `is_controlled_by`
- `owner`
- `controller`

Meta and tooltip helpers:

- `if`
- `limit`
- `hidden_effect`
- `custom_effect_tooltip`
- `custom_override_tooltip`
- `custom_trigger_tooltip` exists for compatibility, but prefer `custom_override_tooltip` when writing new trigger tooltips if the local docs recommend it.

## Copying Policy

Do not paste large Wiki tables into generated answers or skill files. Put stable templates in `hoi4-script-snippets.md`, and point users to the Wiki/local docs for exhaustive lists.
