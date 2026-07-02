//! P13 MIO, technology, equipment, and modifier-family scope gates.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_mio_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = scope_system_index(&map)?;
    let modifiers = repeated_values(&map, "modifier")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    for modifier in &modifiers {
        validate_modifier_family(&index, modifier, "mio", &mut blockers);
    }
    if modifiers.is_empty() {
        questions.push("which indexed MIO modifier or trait should this use?".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"modifiers\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.mio_intent_plan.v1"),
        json_bool(ok),
        json_str(if ok { "mio_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_array(&modifiers),
        json_array(&[
            "common/military_industrial_organization/generated_mios.txt".to_string(),
            "localisation/simp_chinese/generated_mios_l_simp_chinese.yml".to_string(),
        ]),
        json_array(&blockers),
        json_array(&questions),
        json_str("MIO authoring may use only MIO-family modifiers in MIO containers; do not convert MIO bonuses into national spirits")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_tech_scope_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = scope_system_index(&map)?;
    let technology = require_value(&map, "technology")?;
    let effect = require_value(&map, "effect")?;
    let mut blockers = Vec::new();
    if !index.technologies.contains(&technology) {
        blockers.push(format!("technology `{technology}` is not indexed"));
    }
    if !index.effects.contains(&effect) {
        blockers.push(format!("effect `{effect}` is not indexed"));
    }
    if !technology_effect_compatible(&effect) {
        blockers.push(format!(
            "effect `{effect}` is not classified as safe for technology plans"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"technology\": {},\n  \"effect\": {},\n  \"compatible_containers\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.tech_scope_audit.v1"),
        json_bool(ok),
        json_str(if ok { "technology_scope_ok" } else { "blocked" }),
        json_str(&technology),
        json_str(&effect),
        json_array(&["technology".to_string(), "country_history".to_string(), "focus_effect".to_string()]),
        json_array(&blockers),
        json_str("technology effects must reference indexed technology IDs and stay out of MIO/equipment-only containers")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_equipment_scope_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = scope_system_index(&map)?;
    let equipment = require_value(&map, "equipment")?;
    let container = require_value(&map, "container")?;
    let mut blockers = Vec::new();
    if !index.equipment_types.contains(&equipment) {
        blockers.push(format!("equipment type `{equipment}` is not indexed"));
    }
    if !equipment_container_compatible(&container) {
        blockers.push(format!(
            "equipment `{equipment}` is not safe in `{container}`"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"equipment\": {},\n  \"container\": {},\n  \"allowed_containers\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.equipment_scope_audit.v1"),
        json_bool(ok),
        json_str(if ok { "equipment_scope_ok" } else { "blocked" }),
        json_str(&equipment),
        json_str(&container),
        json_array(&[
            "equipment".to_string(),
            "technology".to_string(),
            "mio".to_string(),
            "stockpile_effect".to_string(),
            "production_line".to_string(),
        ]),
        json_array(&blockers),
        json_str("equipment IDs may be used in equipment, technology, MIO, stockpile, or production contexts; not as country-wide national-spirit modifiers")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_modifier_family_catalog(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = scope_system_index(&map)?;
    let family = require_value(&map, "family")?;
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    if !matches!(
        family.as_str(),
        "country" | "state" | "mio" | "character" | "technology" | "equipment" | "shared"
    ) {
        return Err(format!("unknown modifier family `{family}`"));
    }
    let rows = index
        .modifiers
        .iter()
        .filter(|modifier| modifier_family(modifier) == family)
        .take(max_items)
        .map(|modifier| {
            format!(
                "{{\"modifier\": {}, \"family\": {}, \"rule\": {}}}",
                json_str(modifier),
                json_str(&family),
                json_str(modifier_family_rule(&family))
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"family\": {},\n  \"reported_count\": {},\n  \"modifiers\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.modifier_family_catalog.v1"),
        json_str("modifier_family_ready"),
        json_str(&family),
        rows.len(),
        rows.join(", "),
        json_str("modifier families are routing hints; final writes still require scope-compatible containers")
    );
    write_or_print(&json, value(&map, "output"))
}

fn scope_system_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn validate_modifier_family(
    index: &GameIndex,
    modifier: &str,
    expected: &str,
    blockers: &mut Vec<String>,
) {
    if !index.modifiers.contains(modifier) {
        blockers.push(format!("modifier `{modifier}` is not indexed"));
        return;
    }
    let family = modifier_family(modifier);
    if family != expected && family != "shared" {
        blockers.push(format!(
            "modifier `{modifier}` is `{family}`, not `{expected}`"
        ));
    }
}

fn modifier_family(modifier: &str) -> &'static str {
    let lower = modifier.to_ascii_lowercase();
    if lower.contains("mio") || lower.contains("industrial_organization") {
        "mio"
    } else if lower.contains("state")
        || lower.contains("local_")
        || lower.contains("resistance")
        || lower.contains("compliance")
        || lower.contains("building")
    {
        "state"
    } else if lower.contains("character") || lower.contains("operative") || lower.contains("leader")
    {
        "character"
    } else if lower.contains("technology") || lower.contains("research") || lower.contains("tech_")
    {
        "technology"
    } else if lower.contains("equipment")
        || lower.contains("armor")
        || lower.contains("aircraft")
        || lower.contains("ship")
        || lower.contains("weapon")
    {
        "equipment"
    } else if lower.contains("stability")
        || lower.contains("political_power")
        || lower.contains("war_support")
        || lower.contains("production")
        || lower.contains("consumer_goods")
        || lower.contains("justify_war_goal")
    {
        "country"
    } else {
        "shared"
    }
}

fn modifier_family_rule(family: &str) -> &'static str {
    match family {
        "country" => {
            "country/tag modifier; allowed in national spirits and country-scope dynamic modifiers"
        }
        "state" => "state/local modifier; keep in state containers",
        "mio" => "MIO modifier; keep in military industrial organization containers",
        "character" => "character modifier; keep in character/advisor/commander containers",
        "technology" => "technology modifier; keep in technology bonus or research contexts",
        "equipment" => {
            "equipment modifier; keep in equipment, designer, MIO, or technology contexts"
        }
        "shared" => "shared modifier; require local evidence before final write",
        _ => "unknown modifier family",
    }
}

fn technology_effect_compatible(effect: &str) -> bool {
    matches!(
        effect,
        "set_technology" | "add_tech_bonus" | "add_research_slot" | "add_doctrine_cost_reduction"
    ) || effect.contains("technology")
        || effect.contains("tech")
        || effect.contains("research")
}

fn equipment_container_compatible(container: &str) -> bool {
    matches!(
        container,
        "equipment" | "technology" | "mio" | "stockpile_effect" | "production_line" | "designer"
    )
}
