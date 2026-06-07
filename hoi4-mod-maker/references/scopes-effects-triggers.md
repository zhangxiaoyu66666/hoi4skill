# Scopes, Effects, And Triggers

HOI4 script is mostly about putting the right code in the right scope. Before writing an unfamiliar command, check the local documentation linked in `wiki-code-index.md`.

## Scope Rule Of Thumb

- Country effects go in country scope: focus rewards, decision `complete_effect`, country event options.
- State effects go in state scope: building construction, resources, cores, state flags.
- Character effects go in character scope: leaders, advisors, traits.
- Triggers belong in `available`, `visible`, `trigger`, `limit`, `allowed`, and similar condition blocks.
- Effects belong in `completion_reward`, `complete_effect`, `option`, `hidden_effect`, `select_effect`, and scripted effects.

If a block says `limit = { ... }`, its children are triggers. If a block says `completion_reward = { ... }`, its children are effects.

## Safe Country Effects

Verify exact syntax in `documentation/effects_documentation.md` when values are complex.

- `add_political_power = 100`
- `add_stability = 0.05`
- `add_war_support = 0.05`
- `add_ideas = my_idea`
- `remove_ideas = my_idea`
- `add_timed_idea = { idea = my_idea days = 180 }`
- `country_event = { id = my_mod.1 days = 3 }`
- `set_country_flag = my_flag`
- `clr_country_flag = my_flag`
- `set_variable = { var = my_var value = 1 }`
- `add_to_variable = { var = my_var value = 1 }`

## Safe State Effects

Enter a state scope first.

```hoi4
random_owned_controlled_state = {
	limit = { is_core_of = ROOT }
	add_extra_state_shared_building_slots = 1
	add_building_construction = {
		type = industrial_complex
		level = 1
		instant_build = yes
	}
}
```

Common state effects:

- `add_building_construction`
- `add_extra_state_shared_building_slots`
- `add_core_of`
- `remove_core_of`
- `set_state_flag`

## Common Triggers

Country scope:

- `tag = TAG`
- `original_tag = TAG`
- `has_completed_focus = TAG_focus`
- `has_country_flag = my_flag`
- `has_idea = my_idea`
- `has_government = democratic`
- `has_war = no`
- `is_major = yes`

State scope:

- `is_core_of = ROOT`
- `is_owned_and_controlled_by = ROOT`
- `is_controlled_by = ROOT`
- `owner = { tag = TAG }`
- `controller = { tag = TAG }`

## Tooltip Helpers

Use:

```hoi4
custom_effect_tooltip = my_mod_tooltip
hidden_effect = {
	set_country_flag = my_hidden_flag
}
```

For trigger tooltips, prefer `custom_override_tooltip` when available in the target version. The local docs note `custom_trigger_tooltip` as a compatibility alias.

## Common Mistakes

- Writing `modifier = { ... }` inside an effect block. Modifiers belong in ideas, traits, laws, dynamic modifiers, or decision modifier blocks.
- Writing triggers such as `has_war = no` directly inside `completion_reward`.
- Writing effects such as `add_stability = 0.05` inside `available` or `limit`.
- Calling `add_building_construction` from country scope without a state scope wrapper.
- Reusing event IDs without scanning the namespace's max existing number.
- Defining event namespaces with `namespace = ...`; HOI4 event files use top-level `add_namespace = ...` before event bodies.
- Using event IDs outside a declared namespace, or outside the valid number range `1..200000`.

## Documentation Lines Found During Research

The local 2026 install listed these relevant headings:

- Effects: `add_political_power`, `add_stability`, `add_war_support`, `add_ideas`, `add_timed_idea`, `country_event`, `news_event`, `hidden_effect`, `custom_effect_tooltip`, `random_owned_controlled_state`, `add_building_construction`.
- Triggers: `tag`, `original_tag`, `has_completed_focus`, `has_country_flag`, `is_core_of`, `is_owned_and_controlled_by`, `custom_trigger_tooltip`.
- Decisions: `allowed`, `visible`, `available`, `target_root_trigger`, `target_trigger`, `state_trigger`.

Use these as anchors for `Select-String`, not as a replacement for reading the full generated entry.
