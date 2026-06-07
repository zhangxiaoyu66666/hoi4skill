# Implementation Patterns

## One-Sentence Request Pipeline

For requests like "给日本加一个决议，花政治点换海军经验":

1. Identify target country, feature type, cost, effect, and visible text.
2. Choose a prefix from the mod name or existing IDs.
3. Create the smallest complete feature:
   - decision category and decision,
   - effect and availability,
   - localisation,
   - validation notes.
4. Use conservative numbers when the user gives no balance values.
5. Tell the user where the feature appears in game.

## ID Prefixes

Use an existing prefix if visible in the mod. Otherwise derive one from the mod folder:

- `Great Revival` -> `great_revival`
- `CN_FocusPack` -> `cn_focuspack`
- Chinese-only folder names -> use a short romanized or neutral prefix such as `hoi4skill`

Avoid prefixes that collide with vanilla tags or common mods.

## National Focus Pattern

Use a focus when the request mentions 国策, focus tree, route, branch, or long-term country progression.

Minimal pieces:

- `common/national_focus/<prefix>_<tag>_focus.txt`
- `focus_tree` with a unique tree ID or an edit to the existing tree.
- `focus = { id = ... icon = ... x = ... y = ... cost = 10 completion_reward = { ... } }`
- localisation keys: `<focus_id>` and `<focus_id>_desc`.

Immediate rewards stay in `completion_reward`. Long-term modifiers do not: route them through a national spirit, add it from the focus with `add_ideas`, and remove it later with `remove_ideas` only if the state is temporary.

Placement:

- If existing focuses have coordinates, continue the local grid.
- If creating a new isolated focus, place it at `x = 0`, `y = 0` and mention that visual placement can be adjusted.

## Idea Pattern

Use an idea when the request mentions 民族精神, national spirit, buff, debuff, advisor, designer, or law.

Also use an idea when a focus description asks for a lasting country modifier, such as construction speed, consumer goods, factory output, research speed, recruitable population, or daily/weekly power. In that case the focus is only the trigger; the idea is the long-term state.

Minimal pieces:

- `common/ideas/<prefix>_ideas.txt`
- `ideas = { country = { <idea_id> = { ... modifier = { ... } } } }`
- localisation keys: `<idea_id>` and optionally `<idea_id>_desc`.

## Event Pattern

Use an event when the request mentions event, 新闻, popup, choice, narrative, notification, or chain.

Minimal pieces:

- `events/<prefix>_events.txt`
- one or more top-level `add_namespace = <prefix>` declarations before event bodies
- `country_event` or `news_event`
- `id`, `title`, `desc`, `picture`, `is_triggered_only = yes`, `option`
- localisation for title, desc, and options.

Triggering:

- Use `country_event = { id = <namespace>.<number> }` inside a focus, decision, or effect; the ID namespace must be declared in the event file, and event numbers `1..200000` are valid.
- Use `news_event` for global narrative popups.

## Decision Pattern

Use a decision when the request mentions 决议, repeatable action, spending political power, timed mission, or map interaction.

Minimal pieces:

- `common/decisions/<prefix>_decisions.txt`
- category with icon and visibility
- decision with `cost`, `available`, `complete_effect`, and optional `days_remove`
- localisation for category and decision.

## State And Building Pattern

When adding factories, dockyards, infrastructure, resources, or cores:

- Prefer explicit state IDs from existing mod files, dependency files, vanilla state files, or `build-game-index`.
- Check `mod_knowledge.json` `history_states` and `province_definitions` before using a state ID or province ID.
- Remember that `capital` in `history/countries` uses a province ID, not a state ID.
- For focus rewards, use `random_owned_controlled_state` only when the exact state is not important.
- For capital-focused rewards, use `capital_scope` or controlled owned state logic when appropriate.
- Edit `history/states/*.txt` directly only for start-date map setup, owner/core/province/resource/victory-point changes, and only after the target file is verified.
- If the exact state file is unknown, create a state-scoped scripted effect helper and report the unresolved state lookup instead of guessing.

Example effect shape:

```hoi4
random_owned_controlled_state = {
  limit = { is_core_of = ROOT }
  add_extra_state_shared_building_slots = 3
  add_building_construction = {
    type = arms_factory
    level = 3
    instant_build = yes
  }
}
```

## Country And Leader Pattern

Use a full country creation plan when the request mentions 创建国家, 新国家, 国家TAG, 国家名, cosmetic名, starting leader, or country leader traits.

Minimal country pieces:

- `common/country_tags/<file>.txt` with `<TAG> = "countries/<CountryFile>.txt"`.
- `common/countries/<CountryFile>.txt` with graphical cultures and color.
- `history/countries/<TAG> - <Name>.txt` with capital, politics, technologies, ideas, and leaders.
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml` with country name, cosmetic names, and generated country content.

Leader style:

- Standalone mods default to modern `common/characters/<TAG>.txt` plus `history/countries` `recruit_character`.
- If the user explicitly requests legacy syntax, or the existing standalone mod consistently uses legacy syntax, write `create_country_leader` in `history/countries`.
- Submods follow the dependency mod's observed style from `mod_knowledge.json` and `--mod-path`; if dependency style is unknown, stop and report that it must be indexed.

Country leader traits live in `common/country_leader/*.txt`, usually under `leader_traits = { ... }`. They are not national spirits and must not end with `_idea`.

## Localisation Pattern

For Chinese country-content requests, create or update `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`, for example `SOV_l_simp_chinese.yml`. The prefix is for scripted IDs, namespaces, and generated code files; it is never the localisation filename for a country's content.

Mod display names are not country-content localisation. Do not generate `<prefix>_mod_name`, `chinaprc_1979_mod_name`, or any `*_mod_name` key under `l_simp_chinese:`. Use `descriptor.mod` and the launcher-side `.mod` file for mod names.

Use:

```yaml
l_simp_chinese:
  key:0 "显示文本"
  key_desc:0 "描述文本"
```

Keep localisation concise and playable. Do not leave placeholder English text, `TODO`, `具体效果待补充`, `正在影响国家`, or other placeholder text in Chinese files.

For focuses, events, decisions, and national spirits, a conservative script skeleton is acceptable only as an internal construction step. It is not a final deliverable. Before reporting completion, finish the route narrative, stylized title/description, localisation keys, and the script connection that makes the feature reachable.

Focus descriptions should speak from inside the target country, route, faction, party, government, army, or interest group. Avoid third-party observer, encyclopedia, historian, or outside commentary.

Technology-tree features need extra verification: adding `common/technologies` entries is only a minimal script step until folder placement, categories, paths, localisation, and optional technology sprites have been checked against the target game/mod. See `technology-trees.md`.
