//! P64 scope/container contract.
//!
//! This is the shared contract writers can cite before placing modifiers,
//! resources, GUI assets, localisation, history, MIO, technology, or equipment
//! into a container.

#[allow(unused_imports)]
use crate::*;

struct ScopeContainerRow {
    container: &'static str,
    allowed_families: Vec<&'static str>,
    exclusive: bool,
    rule: &'static str,
}

pub(crate) fn cmd_scope_container_contract(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &mod_paths)?;
    let rows = scope_container_rows();
    let typo_samples = vec!["political_p_gain".to_string()];
    let typo_registered = typo_samples
        .iter()
        .filter(|sample| index.modifiers.contains(*sample))
        .cloned()
        .collect::<Vec<_>>();
    let blockers = typo_registered
        .iter()
        .map(|sample| format!("typo sample `{sample}` is registered; update the regression sample"))
        .collect::<Vec<_>>();
    let ok = blockers.is_empty();
    let report = scope_container_contract_json(
        ok,
        &game_root,
        &mod_paths,
        &index,
        &rows,
        &typo_samples,
        &blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn scope_container_rows() -> Vec<ScopeContainerRow> {
    vec![
        ScopeContainerRow {
            container: "country",
            allowed_families: vec!["country", "shared"],
            exclusive: false,
            rule: "country/tag-wide modifiers only; no state, MIO, equipment-only, GUI, GFX, or localisation payloads",
        },
        ScopeContainerRow {
            container: "national_spirit",
            allowed_families: vec!["country", "shared"],
            exclusive: false,
            rule: "national spirits are country idea containers; they may reference registered idea pictures but not MIO/state-only modifiers",
        },
        ScopeContainerRow {
            container: "dynamic_modifier",
            allowed_families: vec!["country", "state", "shared"],
            exclusive: false,
            rule: "dynamic modifiers must use the matching country or state application helper and explicit variable parameters",
        },
        ScopeContainerRow {
            container: "state",
            allowed_families: vec!["state", "shared"],
            exclusive: true,
            rule: "state/province/local modifiers and resources stay in state/history/map scoped writers",
        },
        ScopeContainerRow {
            container: "mio",
            allowed_families: vec!["mio", "equipment", "shared"],
            exclusive: true,
            rule: "MIO modifiers, policies, traits, equipment groups, and design bonuses stay in MIO containers",
        },
        ScopeContainerRow {
            container: "character",
            allowed_families: vec!["character", "country", "shared"],
            exclusive: true,
            rule: "leader/advisor/commander fields must not be mixed; usage still needs character-scope-audit",
        },
        ScopeContainerRow {
            container: "technology",
            allowed_families: vec!["technology", "equipment", "shared"],
            exclusive: true,
            rule: "technology effects reference indexed technology/equipment and stay out of national-spirit modifier blocks",
        },
        ScopeContainerRow {
            container: "equipment_unit",
            allowed_families: vec!["equipment", "unit", "shared"],
            exclusive: true,
            rule: "equipment and unit IDs are not country-wide modifiers; route them to equipment, template, stockpile, or OOB writers",
        },
        ScopeContainerRow {
            container: "focus_field",
            allowed_families: vec!["trigger", "effect", "field"],
            exclusive: true,
            rule: "available/bypass/prerequisite consume triggers; completion_reward consumes effects",
        },
        ScopeContainerRow {
            container: "decision_field",
            allowed_families: vec!["trigger", "effect", "field"],
            exclusive: true,
            rule: "visible/available consume triggers; complete_effect/remove_effect consume effects",
        },
        ScopeContainerRow {
            container: "event_field",
            allowed_families: vec!["trigger", "effect", "field"],
            exclusive: true,
            rule: "trigger consumes triggers; immediate/option/hidden_effect consume effects and must preserve event scope",
        },
        ScopeContainerRow {
            container: "gui_gfx_localisation",
            allowed_families: vec!["resource", "scripted_gui", "localisation"],
            exclusive: true,
            rule: "GUI, GFX, and localisation are resources/text; never assemble them inside gameplay effect containers",
        },
        ScopeContainerRow {
            container: "map_history",
            allowed_families: vec!["state", "province", "map", "history"],
            exclusive: true,
            rule: "owner/controller/resources/buildings/VP/province topology require map/history evidence and changed-only writers",
        },
    ]
}

fn scope_contract_modifier_family(modifier: &str) -> &'static str {
    let lower = modifier.to_ascii_lowercase();
    if lower.contains("mio") || lower.contains("industrial_organization") {
        "mio"
    } else if lower.contains("state") || lower.contains("local") || lower.contains("resource") {
        "state"
    } else if lower.contains("equipment") || lower.contains("production") {
        "equipment"
    } else if lower.contains("research") || lower.contains("technology") {
        "technology"
    } else if lower.contains("leader") || lower.contains("advisor") || lower.contains("commander") {
        "character"
    } else if lower.contains("political")
        || lower.contains("stability")
        || lower.contains("war_support")
    {
        "country"
    } else {
        "shared"
    }
}

fn scope_container_contract_json(
    ok: bool,
    game_root: &Path,
    mod_paths: &[PathBuf],
    index: &GameIndex,
    rows: &[ScopeContainerRow],
    typo_samples: &[String],
    blockers: &[String],
) -> String {
    let mut family_counts: BTreeMap<String, i64> = BTreeMap::new();
    for modifier in &index.modifiers {
        *family_counts
            .entry(scope_contract_modifier_family(modifier).to_string())
            .or_default() += 1;
    }
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.scope_container_contract.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "scope_container_contract_ready"
        } else {
            "blocked"
        }),
    );
    map.insert(
        "game_root".to_string(),
        json_str(&game_root.display().to_string()),
    );
    map.insert(
        "dependency_mod_roots".to_string(),
        json_array(
            &mod_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert(
        "modifier_count".to_string(),
        index.modifiers.len().to_string(),
    );
    map.insert("effect_count".to_string(), index.effects.len().to_string());
    map.insert(
        "trigger_count".to_string(),
        index.triggers.len().to_string(),
    );
    map.insert(
        "modifier_family_counts".to_string(),
        json_i64_entries(&family_counts),
    );
    map.insert("containers".to_string(), scope_container_rows_json(rows));
    map.insert("scope_stack_model".to_string(), scope_stack_model_json());
    map.insert(
        "conditional_effect_contract".to_string(),
        conditional_effect_contract_json(),
    );
    map.insert(
        "wrong_container_samples".to_string(),
        wrong_container_samples_json(),
    );
    map.insert("typo_samples".to_string(), json_array(typo_samples));
    map.insert(
        "unregistered_typo_samples".to_string(),
        json_array(
            &typo_samples
                .iter()
                .filter(|sample| !index.modifiers.contains(*sample))
                .cloned()
                .collect::<Vec<_>>(),
        ),
    );
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "unknown families are not writable; ask the user or extend the local index".to_string(),
            "unregistered modifiers, effects, triggers, sprites, tags, states, provinces, and technologies are hard errors".to_string(),
            "iterator effects must declare iterator, limit scope, effect scope, and ROOT/PREV/FROM meaning before apply".to_string(),
            "shared modifiers need local evidence before broad use".to_string(),
            "scope-compat-audit and symbol-registration-audit remain final gates".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn scope_stack_model_json() -> String {
    let rows = [
        (
            "ROOT",
            "original caller scope; usually the country, state, or event target that started the block",
        ),
        (
            "THIS",
            "current block scope; changes as iterators and nested scopes enter child blocks",
        ),
        (
            "PREV",
            "previous iterator or parent scope; commonly the iterated country/state selected by every_*",
        ),
        (
            "FROM",
            "event/source scope passed by trigger or event caller; may chain as FROM.FROM only when explicitly validated",
        ),
    ];
    format!(
        "[{}]",
        rows.iter()
            .map(|(token, rule)| {
                let mut map = BTreeMap::new();
                map.insert("token".to_string(), json_str(token));
                map.insert("rule".to_string(), json_str(rule));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn conditional_effect_contract_json() -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "pattern".to_string(),
        json_str("iterator = { limit = { triggers } scoped_effects }"),
    );
    map.insert(
        "example".to_string(),
        json_str("every_other_country + limit + ROOT/PREV effect block"),
    );
    map.insert(
        "required_fields".to_string(),
        json_array(&[
            "iterator".to_string(),
            "limit_triggers".to_string(),
            "effect_scope".to_string(),
            "root_scope".to_string(),
            "prev_scope".to_string(),
            "registered_effects".to_string(),
        ]),
    );
    map.insert(
        "forbidden_shortcuts".to_string(),
        json_array(&[
            "tag = XXX direct grant when user requested conditional execution".to_string(),
            "unknown trigger names inside limit".to_string(),
            "unknown effect names inside iterator body".to_string(),
            "unexplained ROOT/PREV/FROM usage".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn wrong_container_samples_json() -> String {
    let rows = [
        (
            "national_spirit",
            "mio_cost_reduction",
            "wrong_container",
            "MIO modifier belongs to MIO, not national spirit",
        ),
        (
            "national_spirit",
            "state_resource_oil",
            "wrong_scope",
            "state/province/resource modifier belongs to state/map scope",
        ),
        (
            "mio",
            "political_power_gain",
            "wrong_container",
            "country political modifier belongs to country/idea scope unless local evidence says shared",
        ),
        (
            "focus_completion_reward",
            "political_p_gain",
            "unknown_modifier",
            "unregistered typo must fail instead of silently doing nothing",
        ),
    ];
    format!(
        "[{}]",
        rows.iter()
            .map(|(container, symbol, error, reason)| {
                let mut map = BTreeMap::new();
                map.insert("container".to_string(), json_str(container));
                map.insert("symbol".to_string(), json_str(symbol));
                map.insert("error".to_string(), json_str(error));
                map.insert("reason".to_string(), json_str(reason));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn scope_container_rows_json(rows: &[ScopeContainerRow]) -> String {
    format!(
        "[{}]",
        rows.iter()
            .map(|row| {
                let mut map = BTreeMap::new();
                map.insert("container".to_string(), json_str(row.container));
                map.insert(
                    "allowed_families".to_string(),
                    json_array(
                        &row.allowed_families
                            .iter()
                            .map(|value| (*value).to_string())
                            .collect::<Vec<_>>(),
                    ),
                );
                map.insert(
                    "exclusive".to_string(),
                    json_bool(row.exclusive).to_string(),
                );
                map.insert("rule".to_string(), json_str(row.rule));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}
