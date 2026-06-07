# Text Focus Tree Layout

Use this when the user sketches a national focus tree as plain text.

## Goal

Allow authors to write something that visually resembles a focus tree:

```text
斯大林宪法
第一个五年计划   互斥       继续新经济政策
快速工业化  强化国家       发财吧农民   奈普曼入党
```

Then turn it into a focus plan with:

- focus IDs,
- localisation titles,
- `x` and `y` positions,
- inferred `prerequisite`,
- `mutually_exclusive` links,
- later effects, icons, and descriptions.

## Parsing Rules

- Each non-empty line is one focus tree row.
- Two or more spaces, or tabs, separate columns.
- A token named `互斥`, `x`, or `X` marks mutual exclusivity between the nearest focus token on its left and right.
- `y` equals the row number, starting at `0`.
- `x` is centered around the row's focus count.
- A focus defaults to a prerequisite from the nearest focus in the previous row.
- If this inferred prerequisite is wrong, edit the Feature Plan before generating code.

The example above means:

```yaml
row 0:
  - 斯大林宪法
row 1:
  - 第一个五年计划
  - 继续新经济政策
  mutually_exclusive:
    - 第一个五年计划 <-> 继续新经济政策
row 2:
  - 快速工业化
  - 强化国家
  - 发财吧农民
  - 奈普曼入党
```

Expected branch interpretation:

- `第一个五年计划` requires `斯大林宪法`.
- `继续新经济政策` requires `斯大林宪法`.
- `快速工业化` and `强化国家` continue the first-five-year-plan branch unless the user says otherwise.
- `发财吧农民` and `奈普曼入党` continue the NEP branch unless the user says otherwise.

## Rust CLI Helper

Save the sketch as `layout.txt`, then run:

```text
hoi4skill parse-focus-layout --input layout.txt --tag SOV --prefix sov_alt
```

Optional output file:

```text
hoi4skill parse-focus-layout --input layout.txt --output focus_plan.json --tag SOV --prefix sov_alt
```

The helper produces JSON. To write files directly, use `hoi4skill apply-focus-layout`.

When writing to an existing mod, `apply-focus-layout` first looks for `common/national_focus/*.txt` files with a `focus_tree` whose `country` block references the target tag. If a matching tree exists, it inserts the new focus blocks into that tree and offsets their `y` values below the existing max row. If no matching tree exists, it creates the normal generated focus file.

Focus IDs must stay ASCII. For AI-generated sketches, prefer giving an explicit English ID hint after the Chinese title:

```text
斯大林宪法 | stalin_constitution
继续新经济政策 [continue_nep]
开放太平洋贸易 (id: pacific_trade)
```

When no hint is present, the Rust CLI tries to convert common Chinese political, industrial, military, and route words into English fragments. If a title is too unusual to map, it falls back to a safe numbered ASCII ID while keeping the Chinese localisation title.

## Feature Plan Extension

When a prose request includes a focus-tree sketch, include:

```yaml
focus_layout:
  source: "plain_text"
  row_separator: "newline"
  column_separator: "two spaces or tab"
  mutual_token: "互斥"
  rows:
    - ["斯大林宪法"]
    - ["第一个五年计划", "互斥", "继续新经济政策"]
    - ["快速工业化", "强化国家", "发财吧农民", "奈普曼入党"]
```

Then resolve:

- title -> ID,
- ID -> localisation key,
- row/column -> `x/y`,
- line relations -> `prerequisite`,
- `互斥` -> `mutually_exclusive`.

## Code Shape

Generated focus blocks should look like:

```hoi4
focus = {
	id = SOV_stalin_constitution
	icon = GFX_goal_generic_political_reform
	x = 0
	y = 0
	cost = 10
	completion_reward = {
		add_political_power = 50
	}
}

focus = {
	id = SOV_first_five_year_plan
	icon = GFX_goal_generic_construct_civ_factory
	x = -1
	y = 1
	cost = 10
	prerequisite = { focus = SOV_stalin_constitution }
	mutually_exclusive = { focus = SOV_continue_nep }
	completion_reward = {
		add_stability = 0.03
	}
}
```

Long-term modifiers do not belong inside a focus reward. If a focus should create an enduring bonus or penalty, generate a national spirit in `common/ideas`, add it from `completion_reward` with `add_ideas`, and remove it later with `remove_ideas` when the state is temporary.

## Limits

Plain text layout is good for visual structure, not full mechanics. Effects, icons, descriptions, bypasses, available triggers, and AI weights still come from prose or follow-up defaults.

If the layout is ambiguous, preserve the author's tree shape and ask only about prerequisites that would change gameplay.
