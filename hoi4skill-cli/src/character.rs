//! Character, advisor, commander, and portrait authoring plans.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_character_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = character_game_index(&map)?;
    let tag = value(&map, "tag").map(str::to_string);
    let role = value(&map, "role")
        .map(str::to_string)
        .unwrap_or_else(|| infer_character_role(&text).to_string());
    let name = value(&map, "name")
        .map(str::to_string)
        .unwrap_or_else(|| infer_character_name(&text));
    let id = value(&map, "id")
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}_character", slugify(&name, "new")));
    let ideology = value(&map, "ideology").map(str::to_string);
    let traits = repeated_values(&map, "trait")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let portrait = value(&map, "portrait").map(str::to_string);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if let Some(tag) = &tag {
        if !index.country_tags.contains(tag) {
            blockers.push(format!("country tag `{tag}` is not indexed"));
        }
    }
    if !known_character_role(&role) {
        blockers.push(format!("character role `{role}` is not supported"));
    }
    if let Some(ideology) = &ideology {
        if !index.ideologies.contains(ideology) {
            blockers.push(format!("leader ideology `{ideology}` is not indexed"));
        }
    } else if character_text_requests_ideology(&text) {
        blockers.push(
            "character ideology is mentioned but not explicit; provide --ideology from indexed local evidence"
                .to_string(),
        );
        questions.push("Which indexed ideology should this character use?".to_string());
    }
    for item in &traits {
        if !index.traits.contains(item) {
            blockers.push(format!("trait `{item}` is not indexed"));
        }
    }
    if let Some(portrait) = &portrait {
        if !portrait_reference_exists(portrait, &index) {
            questions.push(format!(
                "portrait `{portrait}` is not indexed or readable; run portrait-register-plan or provide an existing sprite"
            ));
        }
    } else {
        questions.push(
            "which portrait file or existing portrait sprite should this character use?"
                .to_string(),
        );
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"tag\": {},\n  \"role\": {},\n  \"id\": {},\n  \"name\": {},\n  \"ideology\": {},\n  \"traits\": {},\n  \"portrait\": {},\n  \"planned_files\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.character_intent_plan.v1"),
        json_bool(ok),
        json_str(if ok { "character_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_optional_str(tag.as_deref()),
        json_str(&role),
        json_str(&id),
        json_str(&name),
        json_optional_str(ideology.as_deref()),
        json_array(&traits),
        json_optional_str(portrait.as_deref()),
        json_array(&[
            "common/characters/generated_characters.txt".to_string(),
            "history/countries/<TAG>.txt recruit_character entry when tag is supplied".to_string(),
            "localisation/simp_chinese/generated_characters_l_simp_chinese.yml".to_string(),
        ]),
        json_array(&character_operations(&role, &id, tag.as_deref())),
        json_array(&blockers),
        json_array(&questions),
        json_str("character plans separate leader/advisor/commander roles; portraits and traits must be indexed or explicitly registered before final code")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_character_template_recommend(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let role = require_value(&map, "role")?;
    let templates = character_templates()
        .into_iter()
        .filter(|template| template.role == role)
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"role\": {},\n  \"templates\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.character_template_recommend.v1"),
        json_bool(!templates.is_empty()),
        json_str(if templates.is_empty() { "no_template" } else { "template_ready" }),
        json_str(&role),
        render_character_templates(&templates),
        json_str("AI may select a template; Rust writers still validate trait, portrait, ideology, and recruit usage")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_portrait_register_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let role = value(&map, "role").unwrap_or("leader");
    let file = require_value(&map, "file")?;
    let path = normalize_path(&file)?;
    let sprite = value(&map, "sprite")
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "GFX_portrait_{}",
                slugify(&file_stem_string(&path), "portrait")
            )
        });
    let mut blockers = Vec::new();
    if !path.exists() {
        blockers.push(format!("portrait file `{}` does not exist", path.display()));
    }
    if !matches!(
        path.extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "dds" | "png" | "tga"
    ) {
        blockers.push("portrait file extension must be dds, png, or tga".to_string());
    }
    let ok = blockers.is_empty();
    let texturefile = format!(
        "gfx/leaders/generated/{}",
        path.file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("portrait.dds")
    );
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"role\": {},\n  \"file\": {},\n  \"sprite\": {},\n  \"texturefile\": {},\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.portrait_register_plan.v1"),
        json_bool(ok),
        json_str(if ok { "portrait_plan_ready" } else { "blocked" }),
        json_str(role),
        json_str(&path.display().to_string()),
        json_str(&sprite),
        json_str(&texturefile),
        json_array(&[
            "interface/generated_portraits.gfx".to_string(),
            texturefile.clone(),
        ]),
        json_array(&blockers),
        json_str("portrait sprites must be registered before character plans reference them")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_character_scope_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let character = require_value(&map, "character")?;
    let usage = require_value(&map, "usage")?;
    let role = value(&map, "role").unwrap_or("leader");
    let mut blockers = Vec::new();
    if !known_character_usage(&usage) {
        blockers.push(format!("character usage `{usage}` is not supported"));
    }
    if !character_usage_compatible(role, &usage) {
        blockers.push(format!(
            "role `{role}` is not compatible with usage `{usage}`"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"character\": {},\n  \"role\": {},\n  \"usage\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.character_scope_audit.v1"),
        json_bool(ok),
        json_str(if ok { "character_scope_ok" } else { "blocked" }),
        json_str(&character),
        json_str(role),
        json_str(&usage),
        json_array(&blockers),
        json_str("leaders/advisors/commanders have different fields; do not mix role containers or recruitment usage")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

struct CharacterTemplate {
    role: &'static str,
    id: &'static str,
    required_fields: Vec<&'static str>,
}

fn character_game_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn infer_character_role(text: &str) -> &'static str {
    if text.contains("顾问") || text.contains("内阁") {
        "advisor"
    } else if text.contains("将领") || text.contains("元帅") || text.contains("司令") {
        "commander"
    } else {
        "leader"
    }
}

fn infer_character_name(text: &str) -> String {
    for marker in ["加", "创建", "新增"] {
        if let Some(after) = text.split(marker).nth(1) {
            let name = after
                .split(['，', ',', '。', '\n', ' '])
                .next()
                .unwrap_or("")
                .trim();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    "新角色".to_string()
}

fn character_text_requests_ideology(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    text.contains("意识形态")
        || text.contains("主义")
        || text.contains("共产")
        || text.contains("民主")
        || text.contains("中立")
        || text.contains("法西斯")
        || lower.contains("ideology")
        || lower.contains("commun")
        || lower.contains("democrat")
        || lower.contains("neutral")
        || lower.contains("fasc")
}

fn known_character_role(role: &str) -> bool {
    matches!(
        role,
        "leader" | "advisor" | "commander" | "field_marshal" | "corps_commander" | "navy_leader"
    )
}

fn known_character_usage(usage: &str) -> bool {
    matches!(
        usage,
        "recruit_character"
            | "create_country_leader"
            | "promote_character"
            | "advisor_slot"
            | "corps_commander"
            | "field_marshal"
            | "navy_leader"
    )
}

fn character_usage_compatible(role: &str, usage: &str) -> bool {
    match role {
        "leader" => matches!(usage, "recruit_character" | "create_country_leader"),
        "advisor" => matches!(usage, "recruit_character" | "advisor_slot"),
        "commander" | "field_marshal" | "corps_commander" => {
            matches!(
                usage,
                "recruit_character" | "promote_character" | "field_marshal" | "corps_commander"
            )
        }
        "navy_leader" => matches!(
            usage,
            "recruit_character" | "promote_character" | "navy_leader"
        ),
        _ => false,
    }
}

fn portrait_reference_exists(value: &str, index: &GameIndex) -> bool {
    let path = PathBuf::from(value);
    path.exists() || index.leader_portraits.contains(value) || index.sprites.contains(value)
}

fn character_operations(role: &str, id: &str, tag: Option<&str>) -> Vec<String> {
    let mut out = vec![format!("create common/characters entry `{id}` as {role}")];
    if let Some(tag) = tag {
        out.push(format!(
            "add `recruit_character = {id}` to history/countries for {tag}"
        ));
    }
    out
}

fn character_templates() -> Vec<CharacterTemplate> {
    vec![
        CharacterTemplate {
            role: "leader",
            id: "country_leader_character",
            required_fields: vec!["id", "name", "ideology", "traits", "portrait"],
        },
        CharacterTemplate {
            role: "advisor",
            id: "political_advisor_character",
            required_fields: vec!["id", "name", "advisor_slot", "traits", "portrait"],
        },
        CharacterTemplate {
            role: "commander",
            id: "unit_leader_character",
            required_fields: vec!["id", "name", "unit_leader_role", "traits", "portrait"],
        },
    ]
}

fn render_character_templates(templates: &[CharacterTemplate]) -> String {
    templates
        .iter()
        .map(|template| {
            let fields = template
                .required_fields
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>();
            format!(
                "{{\"id\": {}, \"role\": {}, \"required_fields\": {}}}",
                json_str(template.id),
                json_str(template.role),
                json_array(&fields)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn file_stem_string(path: &Path) -> String {
    path.file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("portrait")
        .to_string()
}
