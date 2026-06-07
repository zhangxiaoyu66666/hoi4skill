# Feature Cards

Use this when the user wants to describe decisions, national spirits, unique technologies, scripted helpers, or a conservative special-GUI skeleton without writing HOI4 syntax.

## Decision Card

```text
决议：鼓励奈普曼投资
目标：SOV
分类：新经济政策
花费：50政治点
冷却：30天
条件：完成国策 继续新经济政策
效果：获得1座民用工厂，稳定度+2%
图标：generic_political_discourse
描述：允许私营资本重新参与国家建设。
```

Expected output:

- `common/decisions/<prefix>_decisions.txt`
- `common/decisions/categories/<prefix>_categories.txt`
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`

The parser converts obvious lines into candidate HOI4 fields:

- `花费：50政治点` -> `cost = 50`
- `冷却：30天` -> `days_remove = 30`
- `条件：完成国策 X` -> `has_completed_focus = <id for X>`
- `稳定度+2%` in a decision effect -> `add_stability = 0.02`
- `获得1座民用工厂` -> state-scoped factory effect candidate

Factory rewards need a state scope. Codex must wrap them with `random_owned_controlled_state`, `capital_scope`, or a specific state ID.

## When To Use A National Spirit

Use a national spirit when an effect should remain true after the focus completes: long-term stability, consumer goods, construction speed, factory output, research speed, recruitment, daily/weekly political power, or any other persistent modifier.

Do not use a national spirit for one-shot rewards such as political power, army/navy/air experience, triggering an event, setting a flag, unlocking a decision, or adding a building. Those belong in `completion_reward`, event options, or decision `complete_effect`.

National focuses do not carry long-term modifiers directly. If a focus creates a lasting state, create an idea card, have the focus use `add_ideas = <idea_id>`, and if the state is temporary, define the ending focus/event/decision that calls `remove_ideas = <idea_id>`.

## Idea Card

```text
民族精神：新经济政策复兴
目标：SOV
效果：稳定度+5%，建造速度+5%，消费品工厂-3%
移除：不可手动移除
图标：GFX_idea_nep_revival
描述：市场活力重新回到苏维埃经济之中。
```

Expected output:

- `common/ideas/<prefix>_ideas.txt`
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`

The parser converts obvious lines into candidate idea fields:

- `稳定度+5%` -> `stability_factor = 0.05`
- `战争支持+2%` -> `war_support_factor = 0.02`
- `移除：不可手动移除` -> `removal_cost = -1`

Some modifier names, such as consumer goods or construction speed, should be verified in local game documentation or nearby mod code before final code generation.

## Technology Card

```text
独有科技：铁路调度算法
目标：FER
分类：engineering
年份：1938
研究花费：2
效果：补给效率+5%
描述：铁路调度进入新的自动化阶段。
```

Expected output:

- `common/technologies/<prefix>_technologies.txt`
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`

Generated technology IDs end with `_tech`, for example `fer_rail_technology_0_tech`. The Rust writer creates a minimal `technologies = { ... }` skeleton with `research_cost`, `start_year`, `folder`, and `categories`. Verify folder placement, `path` links, categories, localisation, and optional technology sprites against the target game/mod before treating the technology as fully integrated into a research tree. See `technology-trees.md`.

## Special GUI Card

```text
特殊GUI：铁路运力面板
目标：FER
用途：显示铁路运力、军列占用和瓶颈州。
描述：一个供后续 scripted GUI 逻辑连接的状态面板。
```

Expected output:

- `common/scripted_guis/<prefix>_scripted_guis.txt`
- `interface/<prefix>.gui`
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`

Localisation descriptions are mandatory for player-facing cards. Do not generate placeholder text such as `具体效果待补充`, `正在影响国家`, `描述`, or `TODO`. National spirits must read as persistent state/institution/social-condition prose, not as focus actions.

Generated GUI IDs end with `_gui`, for example `fer_rail_gui_0_gui`. The Rust writer only creates a conservative `scripted_gui` hook plus a minimal `.gui` window skeleton. Variables, buttons, scripted localisation, and open/close wiring must be adapted from the target mod's existing GUI pattern.

## Scripted Effect Card

```text
脚本效果：铁路瓶颈修复
范围：州
效果：军工+2
```

Expected output:

- `common/scripted_effects/<prefix>_scripted_effects.txt`

Generated scripted-effect IDs end with `_effect`, for example `fer_rail_scripted_effect_0_effect`. If `范围：州` or `scope：state` is present, state effects such as `add_building_construction` are emitted directly inside the scripted effect. Otherwise the writer treats the helper as country-scoped and wraps state-building effects in `random_owned_controlled_state`.

## Scripted Trigger Card

```text
脚本触发：战时铁路管制可用
条件：战争中
```

Expected output:

- `common/scripted_triggers/<prefix>_scripted_triggers.txt`

Generated scripted-trigger IDs end with `_trigger`, for example `fer_rail_scripted_trigger_0_trigger`. The parser converts simple trigger prose such as `战争中` and `和平` into concrete trigger lines, while unresolved references stay as TODO comments.

## State Effect Card

```text
州效果：莫斯科工业修复
州ID：64
目标：FER
建筑：军工+2，基础设施+1
资源：钢+8，铝+2
核心：FER
```

Expected output:

- `common/scripted_effects/<prefix>_state_effects.txt`

Generated state-effect IDs end with `_state_effect`, for example `fer_rail_state_effect_0_state_effect`. The writer creates a scripted effect helper, not a direct edit to `history/states`. If `州ID` is present, the helper wraps the effect in that state ID; otherwise it emits a state-scope helper and leaves a state-name resolution comment. Common mappings include `军工`, `民工`, `基础设施`, `防空`, `船坞`, `炼油厂`, `钢`, `铝`, `石油`, `橡胶`, `钨`, `铬`, and `核心：TAG`.

## Multiple Cards

Separate cards with a blank line or `---`:

```text
民族精神：市场回暖
目标：SOV
效果：稳定度+3%
移除：不可手动移除

---

决议：鼓励奈普曼投资
目标：SOV
分类：新经济政策
花费：50政治点
条件：拥有民族精神 市场回暖
效果：获得1座民用工厂
```

## Rust CLI Helper

Run:

```text
hoi4skill parse-feature-cards --input cards.txt --tag SOV --prefix sov_nep
```

Optional output file:

```text
hoi4skill parse-feature-cards --input cards.txt --output feature_plan.json --tag SOV --prefix sov_nep
```

The helper produces a Feature Plan. To write files directly, use `hoi4skill apply-feature-cards`.

When writing decisions to an existing mod, `apply-feature-cards` scans `common/decisions/categories`, `common/decisions`, and Simplified Chinese localisation first. If the card has `分类`, it can reuse a category whose ID or localisation title matches and whose `allowed` or `visible` tag fits the target country. If the card omits `分类`, it can reuse a non-`scripted_gui` category for the target tag. If no safe category is found, it creates the generated category and decision files.

When writing national spirits to an existing mod, `apply-feature-cards` scans `common/ideas` for target-country files. A file named like the target tag, or one already containing idea IDs with that tag prefix, is reused when it has a `country = { ... }` wrapper. Large shared minister/advisor/character idea files are skipped. If no safe target file is found, it creates `<prefix>_ideas.txt`.

When writing technology cards, `apply-feature-cards` creates `common/technologies/<prefix>_technologies.txt` and appends unique entries inside a `technologies = { ... }` wrapper.

When writing special-GUI cards, `apply-feature-cards` creates `common/scripted_guis/<prefix>_scripted_guis.txt` and `interface/<prefix>.gui`. Re-running the same card is idempotent.

When writing scripted-effect or scripted-trigger cards, `apply-feature-cards` creates `common/scripted_effects/<prefix>_scripted_effects.txt` or `common/scripted_triggers/<prefix>_scripted_triggers.txt`. Re-running the same card is idempotent.

When writing state-effect cards, `apply-feature-cards` creates `common/scripted_effects/<prefix>_state_effects.txt`. Re-running the same card is idempotent.

## Code Generation Rules

For decisions:

1. Create or reuse a decision category.
2. Add `allowed`, `visible`, `available`, `cost`, `days_remove`, and `complete_effect`.
3. Add localisation for category, decision, and description.
4. Use the target mod's existing decision style when available.

For ideas:

1. Add an `ideas = { country = { ... } }` entry or append inside an existing target-country `country = { ... }` wrapper.
2. Generate a scripted ID ending with `_idea`, for example `FER_fragmented_railway_authority_idea`.
3. Add `picture`, `removal_cost`, and `modifier`.
4. Add localisation for name and `_desc` in the country's localisation file under the national-spirit section.
5. If a focus or decision grants the idea, ensure that feature references the generated idea ID.

For technologies:

1. Generate a scripted ID ending with `_tech`.
2. Add or append inside `technologies = { ... }`.
3. Include `research_cost`, `start_year`, `folder`, and `categories`.
4. Add localisation for name and `_desc`.
5. Verify folder/category/path/icon integration against game documentation or the target mod.

For special GUI:

1. Generate a scripted ID ending with `_gui`.
2. Add a `scripted_gui = { ... }` hook.
3. Add a minimal `guiTypes = { ... }` window skeleton.
4. Add localisation for the GUI title and description.
5. Inspect existing target-mod GUI code before wiring variables, buttons, scripted loc, or triggers.

For scripted helpers:

1. Generate scripted-effect IDs ending with `_effect` and scripted-trigger IDs ending with `_trigger`.
2. Add top-level named blocks under `common/scripted_effects` or `common/scripted_triggers`.
3. Keep unresolved natural-language effects/triggers as TODO comments instead of pretending they are valid HOI4 code.
4. Use `范围：州` only when the caller will invoke the helper from state scope.

For state effects:

1. Generate scripted-effect IDs ending with `_state_effect`.
2. Prefer `州ID` when the user already knows the target state.
3. If only a state name is supplied, keep a resolution TODO instead of guessing.
4. Do not edit `history/states` directly until the state id and target file are verified.
5. Put buildings, resources, and `add_core_of` / `remove_core_of` inside a state scope.

## Limits

Cards are intentionally plain. They cannot express every HOI4 mechanic. If a card mentions complex GUI, variables, scripted triggers, target decisions, technology folders, research sharing, or DLC-specific systems, turn it into a Feature Plan and inspect the target mod before coding.
