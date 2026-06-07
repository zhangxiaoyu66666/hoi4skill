# Event Cards

Use this when the user wants to describe events without writing HOI4 event syntax.

## Basic Event Card

```text
事件：新经济政策的未来
类型：国家事件
目标：SOV
命名空间：sov_nep
标题：新经济政策的未来
描述：党内围绕新经济政策展开了激烈争论。
图片：GFX_report_event_generic
触发：完成国策 继续新经济政策
选项A：继续试验
效果A：政治点+50，稳定度+2%
选项B：回到计划经济
效果B：稳定度-5%，设置旗标 end_nep
```

Expected output:

- `events/<prefix>_events.txt`
- `localisation/simp_chinese/<TAG>_l_simp_chinese.yml`

The parser produces an Event Feature Plan with:

- event type: `country_event`, `news_event`, or `state_event`,
- namespace,
- event ID candidate,
- title and description localisation keys,
- option localisation keys,
- trigger candidates,
- option effect candidates,
- event file and localisation file paths.

## Field Rules

- `事件：` starts a new event card.
- `类型：国家事件` -> `country_event`.
- `类型：新闻事件` -> `news_event`.
- `类型：省份事件` or `类型：州事件` -> `state_event`.
- `命名空间：sov_nep` -> top-level `add_namespace = sov_nep`.
- `触发：完成国策 X` -> candidate `has_completed_focus = <id for X>`.
- `选项A：...` creates option `a`.
- `效果A：...` maps to option `a` effects.
- `隐藏效果A：...` maps to `hidden_effect` inside option `a`.
- `AI权重A：50` is a candidate `ai_chance` value.

## Rust CLI Helper

Run:

```text
hoi4skill parse-event-cards --input events.txt --tag SOV --prefix sov_nep
```

Optional output file:

```text
hoi4skill parse-event-cards --input events.txt --output event_plan.json --tag SOV --prefix sov_nep
```

The helper produces a Feature Plan. To write files directly, use `hoi4skill apply-event-cards`.

When writing to an existing mod, `apply-event-cards` scans `events/*.txt` first. If the namespace already exists, it appends to that event file and continues from the current max event number. New CLI-generated events include a stable `hoi4skill_card` comment so re-running the same card does not create duplicate higher-numbered events.

## Code Generation Rules

When converting an event plan into HOI4 code:

1. Scan existing event namespaces and pick the next safe number.
2. Write `add_namespace = <namespace>` once per namespace at the top level before event bodies; one event file may declare multiple namespaces.
3. Put every event body under `country_event`, `news_event`, or `state_event`; its `id` must use a declared namespace, for example `id = sov_nep.1`.
4. Event numbers `1..200000` are valid inside a namespace.
5. Use `is_triggered_only = yes` unless the user explicitly wants a random event with MTTH.
6. Add a `picture` if the card provides one or the target mod has a default style.
7. Add every option with `name = <event_id>.<option_key>`.
8. Convert option effects only after checking scope.
9. Add localisation for title, desc, and every option.

## Triggering Events

If a focus, decision, or effect should fire the event, use:

```hoi4
country_event = { id = sov_nep.1 }
```

For delayed firing:

```hoi4
country_event = { id = sov_nep.1 days = 3 }
```

For news popups, use `news_event` only when the event itself is defined as `news_event`.

## Limits

Event cards are for structure and common effects. Complex event chains, random lists, scripted localisation, dynamic variables, and target-scoped events should be turned into a Feature Plan and checked against nearby mod code before final generation.
