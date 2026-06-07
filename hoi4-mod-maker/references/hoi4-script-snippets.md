# HOI4 Script Snippets

Copy-adapt these snippets when generating small features. Always preserve nearby mod style when it differs from these defaults.

## Descriptor

```hoi4
version="1.0"
tags={
	"Alternative History"
}
name="My HOI4 Mod"
supported_version="*"
```

For a local launcher `.mod` file, add:

```hoi4
path="C:/Users/<user>/Documents/Paradox Interactive/Hearts of Iron IV/mod/my_mod"
```

Use forward slashes in launcher paths when possible.

## Event File

```hoi4
add_namespace = my_mod

country_event = {
	id = my_mod.1
	title = my_mod.1.t
	desc = my_mod.1.d
	picture = GFX_report_event_generic
	is_triggered_only = yes

	option = {
		name = my_mod.1.a
		add_political_power = 50
	}
}
```

Declare one or more `add_namespace = ...` lines at the top level before event bodies. Each event body ID must use a declared namespace, and numbers `1..200000` are valid inside that namespace.

Trigger from an effect:

```hoi4
country_event = { id = my_mod.1 days = 3 }
```

Use `news_event = { ... }` only for global narrative popups. Do not put a `news_event = { ... }` definition inside an effect block.

## Event Card Input

```text
事件：新经济政策的未来
类型：国家事件
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

Convert it to an event plan first, then write the HOI4 event and localisation.

## National Focus Tree

```hoi4
focus_tree = {
	id = my_tag_focus_tree
	country = {
		factor = 0
		modifier = {
			add = 10
			tag = TAG
		}
	}

	focus = {
		id = TAG_my_focus
		icon = GFX_goal_generic_construct_civ_factory
		x = 0
		y = 0
		cost = 10
		available = {
			has_war = no
		}
		completion_reward = {
			add_political_power = 100
		}
		ai_will_do = {
			factor = 1
		}
	}
}
```

Common additions:

```hoi4
prerequisite = { focus = TAG_previous_focus }
mutually_exclusive = { focus = TAG_other_branch }
relative_position_id = TAG_previous_focus
```

## Focus Reward: Factories In A Valid State Scope

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

`add_building_construction` is a state effect. Enter a state scope first.

## Focus Reward: Long-Term National Spirit

```hoi4
completion_reward = {
	add_ideas = my_mod_reform_idea
}
```

The idea carries the lasting `modifier = { ... }` in `common/ideas`. If the effect is temporary, remove it at the explicit ending boundary:

```hoi4
remove_ideas = my_mod_reform_idea
```

Do not put `modifier = { ... }` directly inside a focus `completion_reward`.

## Decision Category And Decision

Category file, usually `common/decisions/categories/<prefix>.txt`:

```hoi4
my_mod_category = {
	icon = GFX_decision_generic_political_reform
	picture = GFX_decision_category_generic_political_reform
	allowed = {
		original_tag = TAG
	}
	visible = {
		tag = TAG
	}
	visible_when_empty = yes
}
```

Decision file, usually `common/decisions/<prefix>_decisions.txt`:

```hoi4
my_mod_category = {
	my_mod_decision = {
		icon = generic_political_discourse
		cost = 50
		days_remove = 30
		visible = {
			tag = TAG
		}
		available = {
			has_war = no
		}
		complete_effect = {
			add_political_power = 25
		}
		ai_will_do = {
			factor = 1
		}
	}
}
```

For targeted decisions, read `common/decisions/_documentation.md` first. Prefer `target_root_trigger` and `target_trigger` over expensive per-frame `visible` checks.

## Idea / National Spirit

```hoi4
ideas = {
	country = {
		my_mod_spirit = {
			picture = generic_production_bonus
			removal_cost = -1
			modifier = {
				stability_factor = 0.05
				industrial_capacity_factory = 0.05
			}
		}
	}
}
```

Grant it from a country effect:

```hoi4
add_ideas = my_mod_spirit
```

Timed spirit:

```hoi4
add_timed_idea = {
	idea = my_mod_spirit
	days = 180
}
```

## Unique Technology

```hoi4
technologies = {
	my_mod_unique_tech = {
		research_cost = 1
		start_year = 1936
		folder = {
			name = special_forces_folder
			position = { x = 0 y = 0 }
		}
		categories = {
			special_forces
		}
	}
}
```

Verify `folder`, `position`, `path`, `categories`, localisation, and optional technology sprites against the target game/mod before relying on a generated technology in a real research tree. A minimal `technologies = { ... }` block is not enough to claim the tech tree is finished.

## Scripted GUI Skeleton

```hoi4
scripted_gui = {
	my_mod_panel_gui = {
		context_type = country_context
		window_name = "my_mod_panel_gui_window"
		visible = {
			tag = TAG
		}
		triggers = {
			always = yes
		}
		effects = {
		}
	}
}
```

```hoi4
guiTypes = {
	containerWindowType = {
		name = "my_mod_panel_gui_window"
		position = { x = 0 y = 0 }
		size = { width = 420 height = 180 }
		moveable = yes
		orientation = upper_left
	}
}
```

Generated GUI skeletons are hooks, not finished complex interfaces. Copy existing target-mod GUI patterns before adding buttons, variables, scripted localisation, or custom open/close flows.

## Scripted Effect

```hoi4
my_mod_railway_bottleneck_effect = {
	# scope = state
	add_building_construction = { type = arms_factory level = 2 instant_build = yes }
}
```

For country-scope callers, enter a state scope before using state effects:

```hoi4
my_mod_country_railway_bottleneck_effect = {
	random_owned_controlled_state = {
		limit = { is_core_of = ROOT }
		add_building_construction = { type = arms_factory level = 2 instant_build = yes }
	}
}
```

## Scripted Trigger

```hoi4
my_mod_wartime_railway_control_trigger = {
	has_war = yes
}
```

## State Effect Helper

```hoi4
my_mod_moscow_industry_state_effect = {
	# state_id = 64
	64 = {
		add_building_construction = { type = arms_factory level = 2 instant_build = yes }
		add_building_construction = { type = infrastructure level = 1 instant_build = yes }
		add_resource = { type = steel amount = 8 }
		add_core_of = FER
	}
}
```

If no state id is known yet, generate a state-scope helper and resolve the target state before wiring it from a country-scope focus, decision, or event.

## Scripted Trigger And Effect

```hoi4
my_mod_can_start_reform = {
	tag = TAG
	has_war = no
}
```

```hoi4
my_mod_reform_reward = {
	add_political_power = 100
	add_stability = 0.05
}
```

Use scripted helpers when the same condition or reward appears more than once.

## Localisation

Use UTF-8 with BOM for HOI4 localisation files.

```yaml
l_simp_chinese:
  TAG_my_focus:0 "重整军备"
  TAG_my_focus_desc:0 "国家将集中力量重建军备体系。"
  my_mod.1.t:0 "新的方向"
  my_mod.1.d:0 "局势已经发生变化。"
  my_mod.1.a:0 "继续前进。"
  my_mod_category:0 "国家改革"
  my_mod_decision:0 "推动改革"
  my_mod_spirit:0 "改革热情"
  my_mod_spirit_desc:0 "改革正在凝聚国家力量。"
```

Existing mods may contain loose forms such as `key:"文本"` or `key: "文本"`. Preserve them when editing, but generate new lines as `key:0 "文本"`.

## GFX Sprite

```hoi4
spriteType = {
	name = "GFX_my_mod_focus_icon"
	texturefile = "gfx/interface/goals/my_mod_focus_icon.dds"
}
```

PNG is also acceptable for mod assets:

```hoi4
spriteType = {
	name = "GFX_my_mod_focus_icon_png"
	texturefile = "gfx/interface/goals/my_mod_focus_icon.png"
}
```

Only reference a custom sprite if the image exists or the change also adds the image and `interface/*.gfx` entry. Large mods may use sprite names without a `GFX_` prefix; scan existing `interface/*.gfx` before deciding an icon is missing. When icon choice matters, run `hoi4skill icon-preview` and report the generated `index.html`.

## On Action Hook

```hoi4
on_actions = {
	on_startup = {
		effect = {
			TAG = { add_political_power = 50 }
		}
	}
}
```

Use on actions sparingly. For one-sentence requests, a focus, event, or decision is usually safer and easier to test.
