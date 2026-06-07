# Copy To Code Workflow

Use this when the user gives a prose idea, route description, Tieba post draft, lore paragraph, or one-sentence feature request and expects working HOI4 mod files.

## Pipeline

```text
User prose
  -> Requirement extraction
  -> Feature Plan
  -> HOI4 system mapping
  -> File Plan
  -> Script IDs and localisation keys
  -> Code generation
  -> Static validation
  -> In-game test notes
  -> Error-log repair loop
```

Do not jump from prose directly to `.txt` files when the request has more than one moving part.

Do not treat a "verifiable demo" or conservative skeleton as a completed feature. For player-facing HOI4 content, extract route narrative and style first, then finish titles, descriptions, localisation, script wiring, validation, and the final report. Never answer with an excuse that the prose will be added later.

For focus copy, write from the internal first-person perspective of the country, route, faction, party, army, government, or interest group. Do not output third-party observer, encyclopedia, historian, or outside commentary.

## One-Command Rust CLI Path

When the compiled `hoi4skill` CLI is available, use `generate-mod` for a one-sentence request that should become a new mod folder. For an existing mod, run `mod-knowledge` first, then use `run-workflow` for mixed Chinese prose, focus sketches, decision/national-spirit/technology/special-GUI/scripted-helper cards, and event cards.

```text
hoi4skill generate-mod --text "给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。" --output "M:\path\new_mod"
hoi4skill generate-mod --text "给远东铁路共和国加一个国策，完成后获得3个军工厂。" --source-root "M:\path\source_mod" --output "M:\path\new_mod"
hoi4skill mod-knowledge "M:\path\existing_mod_or_launcher.mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
hoi4skill plan-history-edit "M:\path\existing_mod" --text "edit history/states owner for state_id 64" --state-id 64 --game-root "C:\path\Hearts of Iron IV" --mod-path "M:\path\dependency.mod" --output history_plan.json
hoi4skill run-workflow --input "M:\path\copy.txt" --tag SOV --prefix sov_nep --dry-run --output workflow_plan.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\mod" --tag SOV --prefix sov_nep --output workflow_report.json
```

`mod-knowledge` creates the pre-edit dossier: standalone/submod classification, descriptor/launcher metadata, dependency names, dependency roots supplied with `--mod-path`, observed tags, focus trees, namespaces, localisation style, decision categories, idea pictures, GFX sprites, and model-readable `markdown_summary`. Treat missing facts as unknown, not as permission to invent.

`plan-history-edit` is the follow-up gate for `history/countries`, `history/states`, state IDs, province IDs, capitals, victory points, owner/controller, cores, buildings, and resources. Use it before direct history edits; if it returns `direct_history_edit_allowed = false`, report the skipped reasons and switch to state-scoped scripted effects or ask for a game/dependency index.

The report contains:

- `detected`: which input sections were found.
- `country_source`: for `generate-mod`, the localisation and country-file evidence used to infer a tag, or `null`.
- `plans`: parsed Feature Plans before file writes.
- `changed_files`: files created or edited when `--mod-root` is supplied and `--dry-run` is not set.
- `validation`: static validation result after writes or against the supplied mod root.
- `next_steps`: what to check before in-game testing.

## Input Contract

Accept loose prose, but try to infer these fields:

- `target`: country tag, state, ideology, faction, character, or global system.
- `feature_type`: focus, event, decision, idea, character, state edit, scripted effect, scripted trigger, on_action, GUI hook.
- `player_facing_text`: title, description, option text, tooltip text.
- `gameplay_effect`: political power, factories, stability, war support, ideas, events, cores, resources, flags, variables.
- `conditions`: visible, available, trigger, prerequisite, target scope.
- `balance`: cost, days, cooldown, reward size, AI factor.
- `dependencies`: vanilla, DLC, Kaiserredux, other mod.
- `style_source`: existing target mod files to imitate.
- `icons`: requested icon style, existing sprite key, or image file path.

For one-sentence MOD creation, pass known source roots with `--source-root`, `--game-root`, or `--mod-path` so the CLI can read country localisation, verify `common/country_tags -> common/countries`, and choose the correct tag. Ask a question only when `target` or `feature_type` cannot be inferred safely.

## Feature Plan Shape

Before writing code for complex prose, make a compact plan in this shape:

```yaml
feature:
  name: "短功能名"
  target: "TAG or state/system"
  type: "focus/event/decision/idea/..."
  intent: "玩家会看到什么、点什么、得到什么"
  files:
    create: []
    edit: []
  ids:
    prefix: "my_mod"
    focus: []
    events: []
    decisions: []
    ideas: []
  localisation:
    language: "simp_chinese"
    keys: []
  icons:
    requested: []
    sprites: []
    preview: ""
  effects:
    country: []
    state: []
  triggers:
    visible: []
    available: []
  validation:
    static: []
    in_game: []
```

The final answer can be shorter, but the internal implementation should follow this structure.

## Prose Classification

Map common Chinese wording to HOI4 systems:

- “国策、路线、分支、完成后” -> `common/national_focus`
- “树形排版、多行国策名、互斥” -> `references/focus-tree-layout.md`
- “事件、新闻、弹窗、选项、剧情” -> `events`
- “事件：、类型：、标题：、描述：、选项A：、效果A：” -> `references/event-cards.md`
- “决议、花费政治点、冷却、任务、行动” -> `common/decisions` and maybe `common/decisions/categories`
- “民族精神、buff、debuff、改革效果、长期加成” -> `common/ideas`
- “决议：、民族精神：、花费：、效果：、移除：” -> `references/decision-idea-cards.md`
- “独有科技：、特殊科技：、科技：” -> `common/technologies` plus localisation; generated IDs end with `_tech`
- “特殊GUI：、GUI：、界面：、用途：” -> `common/scripted_guis` plus `interface/*.gui`; generated IDs end with `_gui`
- “脚本效果：、scripted_effect：” -> `common/scripted_effects`; generated IDs end with `_effect`
- “脚本触发：、scripted_trigger：” -> `common/scripted_triggers`; generated IDs end with `_trigger`
- “州效果：、州编辑：、州改动：、省份效果：” -> `common/scripted_effects/<prefix>_state_effects.txt`; generated IDs end with `_state_effect`
- “核心、工厂、资源、防空、基础设施、胜利点” -> `history/states` or state-scoped effects
- “历史文件、省份、province、首都、州ID、STATE_、胜利点” -> verify with `references/history-states-provinces.md`, `mod_knowledge.json`, and/or `build-game-index`; `capital` is a province ID
- “创建国家、国家TAG、国家名、cosmetic名” -> `common/country_tags`, `common/countries`, `history/countries`, and target TAG localisation; see `references/country-creation-leaders.md`
- “领袖、顾问、将领、country_leader特质” -> `common/characters`, `common/country_leader`, or legacy `history/countries` `create_country_leader`; choose style from `mod_knowledge.json`
- “开局触发、每月触发、战争胜利触发” -> `common/on_actions`
- “按钮、面板、GUI” -> `common/scripted_guis` plus interface files; avoid auto-generating complex GUI unless the target mod already has a clear pattern
- “图标、icon、图片、dds、png、预览” -> `interface/*.gfx` plus `gfx/interface/...` and `hoi4skill icon-preview`

Persistent-effect routing rule:

- A focus can directly grant immediate effects in `completion_reward`.
- A focus cannot directly hold long-term country modifiers.
- For persistent effects, emit an idea/national-spirit feature and make the focus add it with `add_ideas`.
- For temporary long-term states, also choose the ending event, focus, or decision that removes it with `remove_ideas`.

## File Plan Rules

Prefer adding small files over editing large copied vanilla files.

For country and leader work, standalone mods default to modern `common/characters` plus `recruit_character`; submods follow the dependency mod's observed syntax from `mod_knowledge.json` and `--mod-path`.

For state/province work, read `history_states` and `province_definitions` from `mod_knowledge.json` first, then run `hoi4skill plan-history-edit` before direct history writes. If the target mod has no local `history/states` or `map/definition.csv`, use `hoi4skill build-game-index` on the game root and dependency roots, or require explicit state/province IDs from the user. Do not infer IDs from Chinese place names.

Use existing files when:

- the target mod already has a country focus tree file; `hoi4skill run-workflow` and `apply-focus-layout` will extend a matching `focus_tree` for the target tag and place new rows below the current max `y`,
- the target event namespace exists; `hoi4skill run-workflow` and `apply-event-cards` will append to the existing namespace file, continue from the current max event number, and avoid duplicating the same generated event card on rerun,
- the target decision category already exists; `hoi4skill run-workflow` and `apply-feature-cards` scan category definitions, decision files, and Simplified Chinese localisation, then append a matching target-tag category instead of creating a duplicate,
- the target idea file is clearly country-specific; `hoi4skill run-workflow` and `apply-feature-cards` can append a target-tag `common/ideas` file and skip large shared minister/advisor files,
- the request is a technology card; `hoi4skill run-workflow` and `apply-feature-cards` create a minimal `technologies = { ... }` skeleton under `common/technologies`,
- the request is a special-GUI card; `hoi4skill run-workflow` and `apply-feature-cards` create only a conservative `scripted_gui` hook plus `.gui` window skeleton, then leave complex variable/button wiring for inspected target-mod patterns,
- the request is a scripted-effect or scripted-trigger card; `hoi4skill run-workflow` and `apply-feature-cards` create `common/scripted_effects` or `common/scripted_triggers` helper files and keep unresolved prose as TODO comments,
- the request is a state-effect card; `hoi4skill run-workflow` and `apply-feature-cards` create `common/scripted_effects/<prefix>_state_effects.txt`, using `州ID` wrappers when supplied and state-scope helpers otherwise,

Create new files when:

- no safe existing file matches,
- the feature is independent,
- editing a huge inherited file would make review risky.

## ID And Localisation Rules

- Prefix IDs with the mod or target country style.
- For existing mods, scan nearby IDs first.
- Event files use top-level `add_namespace = prefix` before event bodies; one file may declare multiple namespaces.
- Event IDs use `prefix.N` inside a declared namespace, with `N` in `1..200000`.
- Localisation keys should mirror scripted IDs.
- Chinese requests default to `localisation/simp_chinese`.
- Generate new localisation as `key:0 "文本"` with UTF-8 BOM.
- Do not generate mod display-name localisation such as `<prefix>_mod_name:0 "..."` or `chinaprc_1979_mod_name:0 "..."`; mod names belong in `descriptor.mod` and the launcher-side `.mod` file.
- National-spirit IDs must end with `_idea`; keep their localisation in the national-spirit section, not the focus-tree section.
- Unique-technology IDs generated from cards end with `_tech`.
- Special-GUI IDs generated from cards end with `_gui`.
- Scripted-effect IDs generated from cards end with `_effect`.
- Scripted-trigger IDs generated from cards end with `_trigger`.
- State-effect IDs generated from cards end with `_state_effect`.
- When generating a full country file, group one country's localisation in this order: country tag/name, cosmetic name, focus tree, national spirits, decisions, events, unique technologies, special GUI.
- Use `hoi4skill country-localisation-template --tag <TAG> --name "<国家名>" --prefix <prefix>` to create that grouped skeleton when the CLI is available.
- Existing loose localisation forms may be preserved.

## Code Generation Order

Write files in this order:

1. Scripted helpers if reused.
2. Ideas if events/focuses/decisions grant them.
3. Events if focuses/decisions trigger them.
4. Technology entries if the feature adds unique research.
5. Decisions and decision categories.
6. Focus entries or focus tree files.
7. Icon assets and GFX sprite entries if needed.
8. Icon preview if icons were created, changed, or selected.
9. Localisation last, after all keys are known.

This order reduces missing references.

## Example: One Sentence To Code

User prose:

```text
给德国加一个国策，完成后获得3个军工厂，并触发一个新闻事件。
```

Feature Plan:

```yaml
feature:
  name: "German Rearmament Push"
  target: "GER"
  type: "focus + news_event"
  intent: "Germany completes a focus, gains arms factories, and shows a news popup"
  files:
    edit:
      - "common/national_focus/<germany focus file>"
    create:
      - "events/<prefix>_events.txt"
      - "localisation/simp_chinese/<TAG>_l_simp_chinese.yml"
  ids:
    prefix: "my_mod"
    focus:
      - "GER_my_mod_rearmament_push"
    events:
      - "my_mod.1"
  effects:
    country:
      - "news_event = { id = my_mod.1 }"
    state:
      - "random_owned_controlled_state + add_building_construction arms_factory level 3"
```

Then generate:

- focus block with `completion_reward`,
- event file with `add_namespace = my_mod`,
- localisation for focus and event,
- validation notes.

## Example: Lore Paragraph To Decision

User prose:

```text
意大利海军内部提出一个大胆计划：集中资源整训舰队，短期内牺牲政治资本，换来海军经验和一点战争支持。
```

Feature Plan:

```yaml
feature:
  name: "Fleet Reform Plan"
  target: "ITA"
  type: "decision"
  intent: "Spend political power for navy experience and war support"
  ids:
    prefix: "ita_reform"
    decision_category: "ita_reform_navy"
    decision: "ita_reform_train_the_fleet"
  balance:
    cost: 50
    days_remove: 30
    reward:
      navy_experience: 25
      add_war_support: 0.02
```

Then generate:

- decision category,
- decision with `visible`, `available`, `complete_effect`,
- localisation.

## Validation Loop

After code generation:

1. Run `hoi4skill validate <mod-root>`.
2. Fix static errors.
3. Tell the user which in-game path to test.
4. If the user provides `error.log`, map each error back to the Feature Plan and patch only the affected files.

Common repair mapping:

- missing localisation -> add key to the active language file,
- unknown effect/trigger -> check local game documentation,
- invalid scope -> move code into the correct country/state/character scope,
- duplicate event ID -> scan namespace max number and renumber,
- missing sprite -> reuse a known sprite or add `interface/*.gfx` plus asset.
- icon does not display -> verify the sprite key, `texturefile`, image extension, and preview gallery status.

## Final Report Shape

Keep completion reports concrete:

```text
已把这段文案落成一个决议：
- 新增 common/decisions/<file>.txt
- 新增 common/decisions/categories/<file>.txt
- 更新 localisation/simp_chinese/<file>.yml
- 校验：hoi4skill validate 通过
- 进游戏测试：开 TAG，在决议页查看“显示名”
```

Do not claim a feature is fully working until static validation and the required in-game check path are both addressed.
