//! Model-facing Clausewitz reference tables built from local game indexes.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_clausewitz_reference(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let dependency_roots = dependency_mod_roots(&map)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let index = game_root
        .as_ref()
        .map(|path| {
            build_game_index_with_profile(
                path,
                &dependency_roots,
                GameIndexProfile::ClausewitzReference,
            )
        })
        .transpose()?;
    let markdown = render_clausewitz_reference_table(index.as_ref());
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn render_clausewitz_reference_table(index: Option<&GameIndex>) -> String {
    let mut out = String::new();
    out.push_str("- rule: use this table as a local syntax bridge; if the verified column says missing, query the Clausewitz code library or leave a TODO instead of guessing.\n");
    out.push_str("- rule: LLM output should name intent and structured cards; Rust writers emit the final Clausewitz blocks.\n\n");
    out.push_str(
        "| Intent | System | Verified primitive | Use this shape | Do not write | Notes |\n",
    );
    out.push_str("|---|---|---|---|---|---|\n");
    for row in reference_rows() {
        out.push_str(&format!(
            "| {} | {} | {} | `{}` | `{}` | {} |\n",
            row.intent,
            row.system,
            verified_primitive(&row, index),
            row.use_shape,
            row.avoid,
            row.notes
        ));
    }
    out
}

pub(crate) struct ReferenceRow {
    intent: &'static str,
    system: &'static str,
    kind: ReferenceKind,
    primitive: &'static str,
    use_shape: &'static str,
    avoid: &'static str,
    notes: &'static str,
}

#[derive(Copy, Clone)]
pub(crate) enum ReferenceKind {
    Effect,
    Trigger,
    Modifier,
    ResourceRule,
}

pub(crate) fn reference_rows() -> Vec<ReferenceRow> {
    vec![
        ReferenceRow {
            intent: "战争中条件",
            system: "trigger/available/limit",
            kind: ReferenceKind::Trigger,
            primitive: "has_war",
            use_shape: "has_war = yes",
            avoid: "complete_effect = { has_war = yes }",
            notes: "trigger 只能写在条件上下文，不是执行效果。",
        },
        ReferenceRow {
            intent: "完成国策条件",
            system: "trigger/available/limit",
            kind: ReferenceKind::Trigger,
            primitive: "has_completed_focus",
            use_shape: "has_completed_focus = TAG_focus_id",
            avoid: "completion_reward = { has_completed_focus = TAG_focus_id }",
            notes: "focus id 必须来自当前 mod 或索引依赖。",
        },
        ReferenceRow {
            intent: "拥有民族精神条件",
            system: "trigger/available/limit",
            kind: ReferenceKind::Trigger,
            primitive: "has_idea",
            use_shape: "has_idea = my_spirit_idea",
            avoid: "complete_effect = { has_idea = my_spirit_idea }",
            notes: "检查状态用 has_idea；添加/移除状态用 add_ideas/remove_ideas。",
        },
        ReferenceRow {
            intent: "政治点增减",
            system: "focus/event/decision effect",
            kind: ReferenceKind::Effect,
            primitive: "add_political_power",
            use_shape: "add_political_power = 50",
            avoid: "political_power_weekly = 0.25",
            notes: "长期每周政治点应做成已验证 modifier 的民族精神。",
        },
        ReferenceRow {
            intent: "稳定度即时变化",
            system: "focus/event/decision effect",
            kind: ReferenceKind::Effect,
            primitive: "add_stability",
            use_shape: "add_stability = 0.05",
            avoid: "stability = 5",
            notes: "百分比写为小数。",
        },
        ReferenceRow {
            intent: "稳定度长期修正",
            system: "national spirit modifier",
            kind: ReferenceKind::Modifier,
            primitive: "stability_factor",
            use_shape: "modifier = { stability_factor = 0.05 }",
            avoid: "completion_reward = { modifier = { stability_factor = 0.05 } }",
            notes: "长期修正放民族精神，国策只 add_ideas/remove_ideas。",
        },
        ReferenceRow {
            intent: "战争支持即时变化",
            system: "focus/event/decision effect",
            kind: ReferenceKind::Effect,
            primitive: "add_war_support",
            use_shape: "add_war_support = 0.10",
            avoid: "war_support = 10",
            notes: "百分比写为小数。",
        },
        ReferenceRow {
            intent: "战争支持长期修正",
            system: "national spirit modifier",
            kind: ReferenceKind::Modifier,
            primitive: "war_support_factor",
            use_shape: "modifier = { war_support_factor = 0.10 }",
            avoid: "add_war_support_factor = 0.10",
            notes: "民族精神、动态修正和即时效果不要混用。",
        },
        ReferenceRow {
            intent: "添加民族精神",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "add_ideas",
            use_shape: "add_ideas = my_spirit_idea",
            avoid: "add_idea = my_spirit_idea",
            notes: "idea ID 必须已在 common/ideas 注册。",
        },
        ReferenceRow {
            intent: "移除民族精神",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "remove_ideas",
            use_shape: "remove_ideas = my_spirit_idea",
            avoid: "remove_idea = my_spirit_idea",
            notes: "用于国策链结束旧民族精神。",
        },
        ReferenceRow {
            intent: "触发国家事件",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "country_event",
            use_shape: "country_event = { id = namespace.1 }",
            avoid: "event = namespace.1",
            notes: "事件定义本身在 events/*.txt。",
        },
        ReferenceRow {
            intent: "触发新闻事件",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "news_event",
            use_shape: "news_event = { id = namespace.1 }",
            avoid: "news_event = { title = ... } inside option",
            notes: "effect 里只触发已有事件，不内联定义新闻。",
        },
        ReferenceRow {
            intent: "外交关系变化",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "add_opinion_modifier",
            use_shape: "add_opinion_modifier = { target = TAG modifier = opinion_modifier_id }",
            avoid: "USA = { add_opinion = KOR = 20 }",
            notes: "数值关系要先定义 opinion modifier，不能写控制台命令式 add_opinion。",
        },
        ReferenceRow {
            intent: "生成部队",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "create_unit",
            use_shape: "create_unit = { division = \"...\" owner = TAG count = 1 }",
            avoid: "spawn_units = { division = { division = \"infantry\" } }",
            notes: "也可用 load_oob 调已验证 OOB；不要猜模板名。",
        },
        ReferenceRow {
            intent: "加载 OOB",
            system: "country effect",
            kind: ReferenceKind::Effect,
            primitive: "load_oob",
            use_shape: "load_oob = \"TAG_1936\"",
            avoid: "spawn_divisions = ...",
            notes: "适合标准起始部队或预制增援。",
        },
        ReferenceRow {
            intent: "州建筑",
            system: "state-scoped effect",
            kind: ReferenceKind::Effect,
            primitive: "add_building_construction",
            use_shape: "STATE = { add_building_construction = { type = arms_factory level = 1 instant_build = yes } }",
            avoid: "completion_reward = { add_building_construction = ... }",
            notes: "必须先进入州作用域。",
        },
        ReferenceRow {
            intent: "国策图标",
            system: "national focus resource",
            kind: ReferenceKind::ResourceRule,
            primitive: "focus_goal_sprites",
            use_shape: "icon = GFX_goal_or_focus_from_goal_gfx",
            avoid: "icon = GFX_made_up_name",
            notes: "从 game/dependency interface/*.gfx 的 goal/focus sprite 语义选择。",
        },
        ReferenceRow {
            intent: "民族精神图片",
            system: "idea resource",
            kind: ReferenceKind::ResourceRule,
            primitive: "idea_pictures",
            use_shape: "picture = bare_name_for_GFX_idea_bare_name",
            avoid: "picture = GFX_idea_bare_name",
            notes: "注册名是 GFX_idea_x，引用写 x。",
        },
    ]
}

pub(crate) fn verified_primitive(row: &ReferenceRow, index: Option<&GameIndex>) -> String {
    let Some(index) = index else {
        return format!("{} (not indexed)", row.primitive);
    };
    let ok = match row.kind {
        ReferenceKind::Effect => index.effects.contains(row.primitive),
        ReferenceKind::Trigger => index.triggers.contains(row.primitive),
        ReferenceKind::Modifier => index.modifiers.contains(row.primitive),
        ReferenceKind::ResourceRule => match row.primitive {
            "focus_goal_sprites" => !index.focus_goal_sprites.is_empty(),
            "idea_pictures" => !index.idea_pictures.is_empty(),
            _ => false,
        },
    };
    if ok {
        format!("{} (indexed)", row.primitive)
    } else {
        format!("{} (missing from index)", row.primitive)
    }
}
