//! Generic Chinese card parsing and suggestion inference used by multiple generators.

#[allow(unused_imports)]
use crate::*;

pub(crate) struct Card {
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) fields: BTreeMap<String, String>,
}

pub(crate) fn parse_cards(text: &str, allowed: &[&str]) -> Vec<Card> {
    let mut cards = Vec::new();
    let mut current: Option<Card> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.chars().all(|c| c == '-') {
            if let Some(card) = current.take() {
                cards.push(card);
            }
            continue;
        }
        if let Some((key, val)) = split_field(trimmed) {
            if allowed.contains(&key) {
                if let Some(card) = current.take() {
                    cards.push(card);
                }
                current = Some(Card {
                    kind: key.to_string(),
                    title: val.to_string(),
                    fields: BTreeMap::new(),
                });
            } else if let Some(card) = current.as_mut() {
                card.fields.insert(key.to_string(), val.to_string());
            }
        } else if let Some(card) = current.as_mut() {
            card.fields
                .entry("描述".to_string())
                .and_modify(|s| {
                    s.push('\n');
                    s.push_str(trimmed);
                })
                .or_insert_with(|| trimmed.to_string());
        }
    }
    if let Some(card) = current.take() {
        cards.push(card);
    }
    cards
}

pub(crate) fn split_field(line: &str) -> Option<(&str, &str)> {
    let (idx, sep) = line.char_indices().find(|(_, c)| *c == ':' || *c == '：')?;
    let value_start = idx + sep.len_utf8();
    Some((line[..idx].trim(), line[value_start..].trim()))
}

pub(crate) fn join_existing_fields(
    fields: &BTreeMap<String, String>,
    keys: &[&str],
) -> Option<String> {
    let values = keys
        .iter()
        .filter_map(|key| fields.get(*key))
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.join("；"))
    }
}

#[derive(Clone)]
pub(crate) struct Suggestion {
    pub(crate) kind: String,
    pub(crate) code: String,
    pub(crate) source: String,
    pub(crate) note: String,
}

impl Suggestion {
    pub(crate) fn new(kind: &str, code: &str, source: &str, note: &str) -> Self {
        Self {
            kind: kind.to_string(),
            code: code.to_string(),
            source: source.to_string(),
            note: note.to_string(),
        }
    }
}

pub(crate) fn suggest_common(
    ty: &str,
    effects: &str,
    cost: Option<&str>,
    duration: Option<&str>,
    condition: Option<&str>,
    removal: Option<&str>,
) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if let Some(cost) = cost.and_then(parse_int) {
        out.push(Suggestion::new(
            "decision_cost",
            &format!("cost = {cost}"),
            "",
            "Decision political power cost.",
        ));
    }
    if let Some(days) = duration.and_then(parse_int) {
        out.push(Suggestion::new(
            "days_remove",
            &format!("days_remove = {days}"),
            "",
            "",
        ));
    }
    if let Some(cond) = condition {
        for raw in split_cn_list(cond) {
            out.extend(suggest_trigger(raw));
        }
    }
    for raw in split_cn_list(effects) {
        let percent = parse_percent(raw);
        let number = parse_int(raw);
        if raw.contains("政治点") || raw.contains("政治力量") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("add_political_power = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("稳定") {
            if let Some(v) = percent {
                let code = if ty == "idea" {
                    format!("stability_factor = {}", fmt_float(v))
                } else {
                    format!("add_stability = {}", fmt_float(v))
                };
                out.push(Suggestion::new(
                    if ty == "idea" {
                        "idea_modifier"
                    } else {
                        "country_effect"
                    },
                    &code,
                    raw,
                    "",
                ));
            }
        } else if raw.contains("战争支持") || raw.contains("战争支援") {
            if let Some(v) = percent {
                let code = if ty == "idea" {
                    format!("war_support_factor = {}", fmt_float(v))
                } else {
                    format!("add_war_support = {}", fmt_float(v))
                };
                out.push(Suggestion::new(
                    if ty == "idea" {
                        "idea_modifier"
                    } else {
                        "country_effect"
                    },
                    &code,
                    raw,
                    "",
                ));
            }
        } else if raw.contains("海军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("navy_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("陆军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("army_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("空军经验") {
            if let Some(n) = number {
                out.push(Suggestion::new(
                    "country_effect",
                    &format!("air_experience = {n}"),
                    raw,
                    "",
                ));
            }
        } else if raw.contains("消费品") {
            if let Some(v) = percent {
                out.push(Suggestion::new(
                    "idea_modifier_candidate",
                    &format!("consumer_goods_factor = {}", fmt_float(v)),
                    raw,
                    "Verify modifier name against local game documentation or nearby mod code.",
                ));
            }
        } else if raw.contains("建造速度") || raw.contains("建设速度") {
            if let Some(v) = percent {
                out.push(Suggestion::new(
                    "idea_modifier_candidate",
                    &format!("production_speed_buildings_factor = {}", fmt_float(v)),
                    raw,
                    "Verify modifier name against local game documentation or nearby mod code.",
                ));
            }
        } else if raw.contains("基础设施") || raw.contains("基建") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = infrastructure level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("防空") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = anti_air_building level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("船坞") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = dockyard level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("炼油") || raw.contains("合成油") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = synthetic_refinery level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("民用工厂") || raw.contains("民工") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = industrial_complex level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("军用工厂") || raw.contains("军工") {
            out.push(Suggestion::new(
                "state_effect_candidate",
                "add_building_construction = { type = arms_factory level = <number> instant_build = yes }",
                raw,
                "Must run inside a state scope.",
            ));
        } else if let Some((resource, amount)) = state_resource_effect(raw) {
            out.push(Suggestion::new(
                "state_effect_candidate",
                &format!("add_resource = {{ type = {resource} amount = {amount} }}"),
                raw,
                "Must run inside a state scope.",
            ));
        } else if raw.contains("移除核心") {
            if let Some(tag) = ascii_tag_from_text(raw) {
                out.push(Suggestion::new(
                    "state_effect_candidate",
                    &format!("remove_core_of = {tag}"),
                    raw,
                    "Must run inside a state scope.",
                ));
            } else {
                out.push(Suggestion::new(
                    "raw_effect",
                    raw,
                    raw,
                    "Resolve the country tag before removing a core.",
                ));
            }
        } else if raw.contains("添加核心") || raw.contains("获得核心") {
            if let Some(tag) = ascii_tag_from_text(raw) {
                out.push(Suggestion::new(
                    "state_effect_candidate",
                    &format!("add_core_of = {tag}"),
                    raw,
                    "Must run inside a state scope.",
                ));
            } else {
                out.push(Suggestion::new(
                    "raw_effect",
                    raw,
                    raw,
                    "Resolve the country tag before adding a core.",
                ));
            }
        } else if raw.contains("添加民族精神") || raw.contains("获得民族精神") {
            let idea_name = raw.replace("添加民族精神", "").replace("获得民族精神", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!("add_ideas = <idea id for {}>", idea_name.trim()),
                raw,
                "",
            ));
        } else if raw.contains("移除民族精神") {
            let idea_name = raw.replace("移除民族精神", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!("remove_ideas = <idea id for {}>", idea_name.trim()),
                raw,
                "",
            ));
        } else if raw.contains("触发新闻") {
            let event_name = raw.replace("触发新闻", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!(
                    "news_event = {{ id = <event id for {}> }}",
                    event_name.trim()
                ),
                raw,
                "",
            ));
        } else if raw.contains("触发事件") || raw.contains("触发国家事件") {
            let event_name = raw.replace("触发国家事件", "").replace("触发事件", "");
            out.push(Suggestion::new(
                "country_effect_candidate",
                &format!(
                    "country_event = {{ id = <event id for {}> }}",
                    event_name.trim()
                ),
                raw,
                "",
            ));
        } else if raw.contains("设置旗标") || raw.contains("设置国家旗标") {
            let flag = slugify(
                raw.replace("设置国家旗标", "")
                    .replace("设置旗标", "")
                    .trim(),
                "my_flag",
            );
            out.push(Suggestion::new(
                "country_effect",
                &format!("set_country_flag = {flag}"),
                raw,
                "",
            ));
        } else if !raw.trim().is_empty() {
            out.push(Suggestion::new(
                "raw_effect",
                raw,
                raw,
                "Needs Codex mapping before final code.",
            ));
        }
    }
    if let Some(removal) = removal {
        if removal.contains("不可") || removal.contains("不能") || removal.contains("永久") {
            out.push(Suggestion::new(
                "idea_field",
                "removal_cost = -1",
                removal,
                "",
            ));
        }
    }
    out
}

pub(crate) fn state_resource_effect(raw: &str) -> Option<(&'static str, i64)> {
    let resource = if raw.contains("steel") || raw.contains("钢") {
        "steel"
    } else if raw.contains("aluminium") || raw.contains("aluminum") || raw.contains("铝") {
        "aluminium"
    } else if raw.contains("oil") || raw.contains("石油") {
        "oil"
    } else if raw.contains("rubber") || raw.contains("橡胶") {
        "rubber"
    } else if raw.contains("tungsten") || raw.contains("钨") {
        "tungsten"
    } else if raw.contains("chromium") || raw.contains("铬") {
        "chromium"
    } else {
        return None;
    };
    let amount = parse_int(raw).unwrap_or(1);
    Some((resource, amount))
}

pub(crate) fn suggest_trigger(text: &str) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if let Some(rest) = text.strip_prefix("完成国策") {
        out.push(Suggestion::new(
            "trigger_candidate",
            &format!("has_completed_focus = <focus id for {}>", rest.trim()),
            text,
            "Resolve the Chinese focus title to a real focus ID before code generation.",
        ));
    } else if let Some(rest) = text.strip_prefix("拥有民族精神") {
        out.push(Suggestion::new(
            "trigger_candidate",
            &format!("has_idea = <idea id for {}>", rest.trim()),
            text,
            "",
        ));
    } else if text.contains("和平") || text.contains("无战争") {
        out.push(Suggestion::new("trigger", "has_war = no", text, ""));
    } else if text.contains("战争中") || text.contains("正在战争") {
        out.push(Suggestion::new("trigger", "has_war = yes", text, ""));
    } else {
        out.push(Suggestion::new(
            "raw_trigger",
            text,
            text,
            "Needs Codex mapping before final code.",
        ));
    }
    out
}

pub(crate) fn split_cn_list(text: &str) -> Vec<&str> {
    text.split(['，', ',', '；', ';', '、'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

pub(crate) fn parse_percent(text: &str) -> Option<f64> {
    let idx = text.find('%')?;
    let prefix = &text[..idx];
    parse_last_number(prefix).map(|v| v / 100.0)
}

pub(crate) fn parse_int(text: &str) -> Option<i64> {
    parse_last_number(text).map(|v| v as i64)
}

pub(crate) fn parse_last_number(text: &str) -> Option<f64> {
    let mut current = String::new();
    let mut last = None;
    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
            current.push(ch);
        } else if !current.is_empty() {
            if let Ok(v) = current.parse::<f64>() {
                last = Some(v);
            }
            current.clear();
        }
    }
    if !current.is_empty() {
        if let Ok(v) = current.parse::<f64>() {
            last = Some(v);
        }
    }
    last
}

pub(crate) fn fmt_float(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub(crate) fn normalize_event_type(value: Option<&str>) -> &str {
    match value.unwrap_or("") {
        v if v.contains("新闻") || v.eq_ignore_ascii_case("news_event") => "news_event",
        v if v.contains("省份") || v.contains("州") || v.eq_ignore_ascii_case("state_event") => {
            "state_event"
        }
        _ => "country_event",
    }
}

pub(crate) fn option_key(s: &str) -> String {
    match s.trim() {
        "" | "A" | "a" | "一" => "a".to_string(),
        "B" | "b" | "二" => "b".to_string(),
        "C" | "c" | "三" => "c".to_string(),
        "D" | "d" | "四" => "d".to_string(),
        other => slugify(other, "a"),
    }
}
