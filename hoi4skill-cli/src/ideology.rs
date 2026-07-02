//! Ideology authoring plans.
//!
//! These commands turn one-sentence ideology requests into evidence-bound plans.
//! They do not copy vanilla files or write final ideology code directly.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_ideology_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = if game_root.is_some() {
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?
    } else {
        Vec::new()
    };
    let index = game_root
        .as_deref()
        .map(|root| build_game_index_with_mod_paths(root, &mod_paths))
        .transpose()?;
    let parent = value(&map, "parent")
        .map(str::to_string)
        .unwrap_or_else(|| infer_parent_ideology(&text).to_string());
    let title = value(&map, "title")
        .map(str::to_string)
        .unwrap_or_else(|| infer_ideology_title(&text));
    let id = value(&map, "id")
        .map(str::to_string)
        .unwrap_or_else(|| slugify(&title, "sub_ideology"));
    let mut blockers = Vec::new();
    if let Some(index) = &index {
        if !index.ideologies.contains(&parent) {
            blockers.push(format!(
                "parent ideology `{parent}` is not present in indexed game/mod code"
            ));
        }
        if index.ideologies.contains(&id) {
            blockers.push(format!(
                "ideology id `{id}` already exists as a root ideology"
            ));
        }
    } else {
        blockers.push("missing --game-root; parent ideology cannot be verified".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"parent_ideology\": {},\n  \"sub_ideology_id\": {},\n  \"display_name\": {},\n  \"indexed_roots\": {},\n  \"planned_files\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.ideology_intent_plan.v1"),
        json_bool(ok),
        json_str(if ok { "ideology_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_str(&parent),
        json_str(&id),
        json_str(&title),
        render_indexed_roots(index.as_ref()),
        json_array(&[
            "common/ideologies/generated_ideologies.txt".to_string(),
            "localisation/simp_chinese/generated_ideologies_l_simp_chinese.yml".to_string(),
        ]),
        json_array(&[
            format!("add `{id}` under `{parent}.types`"),
            format!("add localisation `{id}:0 \"{title}\"`"),
            "do not create country tags, flags, or cosmetic tags unless the user also requested them".to_string(),
        ]),
        json_array(&blockers),
        json_str("AI may name and explain the ideology; Rust must verify the parent ideology against game plus explicit parent-mod indexes and assemble final code")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_ideology_batch_copy_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &mod_paths)?;
    let new_ideology = value(&map, "new-ideology")
        .map(str::to_string)
        .unwrap_or_else(|| infer_new_ideology(&text));
    let source_def = value(&map, "source-def")
        .map(str::to_string)
        .or_else(|| infer_source_ideology_after(&text, "def"))
        .unwrap_or_default();
    let source_name = value(&map, "source-name")
        .map(str::to_string)
        .or_else(|| infer_source_ideology_after(&text, "显示名称"))
        .unwrap_or_default();
    let source_flag = value(&map, "source-flag")
        .map(str::to_string)
        .unwrap_or_else(|| {
            infer_source_ideology_after(&text, "国旗").unwrap_or_else(|| source_name.clone())
        });
    let tags = index.country_tags.iter().cloned().collect::<Vec<_>>();
    let mut blockers = Vec::new();
    if source_def.is_empty() {
        blockers.push(
            "source DEF ideology is not explicit; provide --source-def or name one in --text"
                .to_string(),
        );
    }
    if source_name.is_empty() {
        blockers.push(
            "source display-name ideology is not explicit; provide --source-name or name one in --text"
                .to_string(),
        );
    }
    if source_flag.is_empty() {
        blockers.push(
            "source flag ideology is not explicit; provide --source-flag or name one in --text"
                .to_string(),
        );
    }
    for ideology in [&source_def, &source_name, &source_flag] {
        if !ideology.is_empty() && !index.ideologies.contains(ideology) {
            blockers.push(format!(
                "source ideology `{ideology}` is not present in indexed code"
            ));
        }
    }
    if tags.is_empty() {
        blockers
            .push("no country tags were indexed; cannot plan all-tag ideology copies".to_string());
    }
    let ok = blockers.is_empty();
    let examples = tags
        .iter()
        .take(8)
        .map(|tag| {
            format!(
                "{{\"tag\": {}, \"target_name_key\": {}, \"source_name_key\": {}, \"target_def_key\": {}, \"source_def_key\": {}, \"target_flag\": {}, \"source_flag\": {}}}",
                json_str(tag),
                json_str(&format!("{tag}_{new_ideology}")),
                json_str(&format!("{tag}_{source_name}")),
                json_str(&format!("{tag}_{new_ideology}_DEF")),
                json_str(&format!("{tag}_{source_def}_DEF")),
                json_str(&format!("gfx/flags/{}_{}.tga", tag, new_ideology)),
                json_str(&format!("gfx/flags/{}_{}.tga", tag, source_flag))
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"new_ideology\": {},\n  \"tag_count\": {},\n  \"source_def_ideology\": {},\n  \"source_name_ideology\": {},\n  \"source_flag_ideology\": {},\n  \"example_count\": {},\n  \"examples\": [{}],\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.ideology_batch_copy_plan.v1"),
        json_bool(ok),
        json_str(if ok { "batch_copy_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_str(&new_ideology),
        tags.len(),
        json_str(&source_def),
        json_str(&source_name),
        json_str(&source_flag),
        examples.len(),
        examples.join(", "),
        json_array(&[
            "localisation/simp_chinese/generated_ideology_country_names_l_simp_chinese.yml".to_string(),
            "gfx/flags/<TAG>_<new_ideology>.tga or generated flag aliases".to_string(),
        ]),
        json_array(&blockers),
        json_str("batch copy plans must use indexed source ideologies and country tags; ask the user before copying flags when a source flag file is missing")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_politics_intent_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = ideology_game_index(&map)?;
    let tag = value(&map, "tag").map(str::to_string);
    let ruling_party = value(&map, "ruling-party")
        .map(str::to_string)
        .unwrap_or_else(|| infer_ruling_party(&text).to_string());
    let election_allowed = if text.contains("禁止选举") || text.contains("取消选举") {
        Some(false)
    } else if text.contains("选举") || text.contains("民主") {
        Some(true)
    } else {
        None
    };
    let mut blockers = Vec::new();
    if let Some(tag) = &tag {
        if !index.country_tags.contains(tag) {
            blockers.push(format!(
                "country tag `{tag}` is not present in indexed game/mod code"
            ));
        }
    }
    if !index.ideologies.contains(&ruling_party) {
        blockers.push(format!(
            "ruling party ideology `{ruling_party}` is not present in indexed game/mod code"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"tag\": {},\n  \"ruling_party\": {},\n  \"elections_allowed\": {},\n  \"indexed_roots\": {},\n  \"operations\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.politics_intent_plan.v1"),
        json_bool(ok),
        json_str(if ok { "politics_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_optional_str(tag.as_deref()),
        json_str(&ruling_party),
        json_optional_bool(election_allowed),
        render_indexed_roots(Some(&index)),
        json_array(&[
            format!("set_politics.ruling_party = {ruling_party}"),
            "apply only in country/tag scope".to_string(),
            "ask before changing elections if the user did not specify election behavior".to_string(),
        ]),
        json_array(&blockers),
        json_str("politics plans must use indexed ideologies and country tags; Rust writers assemble set_politics or set_ruling_party only after scope checks")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_party_popularity_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = ideology_game_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let ruling_party = require_value(&map, "ruling-party")?;
    let popularities = repeated_values(&map, "popularity")
        .into_iter()
        .map(parse_popularity_pair)
        .collect::<Result<Vec<_>, _>>()?;
    let mut blockers = Vec::new();
    if !index.country_tags.contains(&tag) {
        blockers.push(format!(
            "country tag `{tag}` is not present in indexed game/mod code"
        ));
    }
    if !index.ideologies.contains(&ruling_party) {
        blockers.push(format!(
            "ruling party ideology `{ruling_party}` is not present in indexed game/mod code"
        ));
    }
    let total = popularities.iter().map(|(_, value)| *value).sum::<i64>();
    for (ideology, value) in &popularities {
        if !index.ideologies.contains(ideology) {
            blockers.push(format!("popularity ideology `{ideology}` is not indexed"));
        }
        if *value < 0 || *value > 100 {
            blockers.push(format!(
                "popularity `{ideology}={value}` must be between 0 and 100"
            ));
        }
    }
    if !popularities.is_empty() && total > 100 {
        blockers.push(format!("party popularity total {total} exceeds 100"));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"ruling_party\": {},\n  \"popularity_total\": {},\n  \"popularities\": [{}],\n  \"operations\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.party_popularity_plan.v1"),
        json_bool(ok),
        json_str(if ok { "party_plan_ready" } else { "blocked" }),
        json_str(&tag),
        json_str(&ruling_party),
        total,
        render_popularity_rows(&popularities),
        json_array(&[
            format!("set_politics.ruling_party = {ruling_party}"),
            "set_popularities with indexed ideology keys".to_string(),
            "country/tag scope only".to_string(),
        ]),
        json_array(&blockers),
        json_str("set_politics, set_popularities, add_popularity, and set_ruling_party are country/tag-scope operations")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_cosmetic_tag_batch_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = ideology_game_index(&map)?;
    let new_ideology = require_value(&map, "new-ideology")?;
    let source_name = value(&map, "source-name").unwrap_or("");
    let source_def = value(&map, "source-def").unwrap_or("");
    let source_flag = value(&map, "source-flag").unwrap_or(source_name);
    let requested_tags = repeated_values(&map, "tag")
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let tags = if requested_tags.is_empty() {
        index.country_tags.iter().cloned().collect::<Vec<_>>()
    } else {
        requested_tags
    };
    let mut blockers = Vec::new();
    if source_name.is_empty() {
        blockers.push("missing --source-name; do not default to any official ideology".to_string());
    }
    if source_def.is_empty() {
        blockers.push("missing --source-def; do not default to any official ideology".to_string());
    }
    if source_flag.is_empty() {
        blockers.push("missing --source-flag; do not default to any official ideology".to_string());
    }
    for ideology in [source_name, source_def, source_flag] {
        if !ideology.is_empty() && !index.ideologies.contains(ideology) {
            blockers.push(format!("source ideology `{ideology}` is not indexed"));
        }
    }
    for tag in &tags {
        if !index.country_tags.contains(tag) {
            blockers.push(format!("country tag `{tag}` is not indexed"));
        }
    }
    if tags.is_empty() {
        blockers.push("no country tags selected for cosmetic tag batch plan".to_string());
    }
    let ok = blockers.is_empty();
    let examples = tags
        .iter()
        .take(8)
        .map(|tag| {
            let cosmetic = format!("{tag}_{new_ideology}");
            format!(
                "{{\"tag\": {}, \"cosmetic_tag\": {}, \"name_key\": {}, \"def_key\": {}, \"adj_key\": {}, \"source_name_key\": {}, \"source_def_key\": {}, \"source_flag\": {}}}",
                json_str(tag),
                json_str(&cosmetic),
                json_str(&cosmetic),
                json_str(&format!("{cosmetic}_DEF")),
                json_str(&format!("{cosmetic}_ADJ")),
                json_str(&format!("{tag}_{source_name}")),
                json_str(&format!("{tag}_{source_def}_DEF")),
                json_str(&format!("gfx/flags/{}_{}.tga", tag, source_flag))
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"new_ideology\": {},\n  \"tag_count\": {},\n  \"source_name\": {},\n  \"source_def\": {},\n  \"source_flag\": {},\n  \"examples\": [{}],\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.cosmetic_tag_batch_plan.v1"),
        json_bool(ok),
        json_str(if ok { "cosmetic_batch_plan_ready" } else { "blocked" }),
        json_str(&new_ideology),
        tags.len(),
        json_str(source_name),
        json_str(source_def),
        json_str(source_flag),
        examples.join(", "),
        json_array(&[
            "localisation/simp_chinese/generated_cosmetic_tags_l_simp_chinese.yml".to_string(),
            "interface/generated_cosmetic_flags.gfx or flag copy manifest".to_string(),
            "common/scripted_effects/generated_cosmetic_transitions.txt when transitions are requested".to_string(),
        ]),
        json_array(&blockers),
        json_array(&[
            "If a source flag file is missing, ask whether to generate a placeholder, copy another ideology flag, or skip that tag.".to_string(),
            "If the target cosmetic tag already exists, ask whether to reuse, overwrite, or create a new suffix.".to_string(),
        ]),
        json_str("cosmetic tag batches copy indexed names/DEF/ADJ/flags by plan; final writes must validate localisation, flag assets, and set_cosmetic_tag usage")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_cosmetic_transition_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = ideology_game_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let cosmetic = require_value(&map, "cosmetic")?;
    let ruling_party = value(&map, "ruling-party").map(str::to_string).or_else(|| {
        let inferred = infer_ruling_party(&text);
        index
            .ideologies
            .contains(inferred)
            .then(|| inferred.to_string())
    });
    let leader = value(&map, "leader-character").map(str::to_string);
    let loc_state = cosmetic_localisation_state(&index, &cosmetic);
    let flag_state = flag_triplet_state(&index.game_root, &cosmetic);
    let cosmetic_exists = loc_state.iter().any(|(_, exists)| *exists)
        || flag_state.iter().any(|(_, exists)| *exists)
        || index
            .country_name_tags
            .values()
            .any(|tags| tags.contains(&cosmetic));
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !index.country_tags.contains(&tag) {
        blockers.push(format!("country tag `{tag}` is not indexed"));
    }
    if let Some(party) = &ruling_party {
        if !index.ideologies.contains(party) {
            blockers.push(format!("ruling party ideology `{party}` is not indexed"));
        }
    }
    if !effect_available_or_unknown(&index, "set_cosmetic_tag") {
        blockers.push("effect `set_cosmetic_tag` is not indexed".to_string());
    }
    if ruling_party.is_some() && !effect_available_or_unknown(&index, "set_politics") {
        blockers.push("effect `set_politics` is not indexed".to_string());
    }
    if leader.is_some() && !effect_available_or_unknown(&index, "recruit_character") {
        blockers.push("effect `recruit_character` is not indexed".to_string());
    }
    for (key, exists) in &loc_state {
        if !exists {
            questions.push(format!(
                "localisation key `{key}` is missing; create it or choose an existing cosmetic tag"
            ));
        }
    }
    for (path, exists) in &flag_state {
        if !exists {
            questions.push(format!(
                "flag asset `{}` is missing; copy, generate, or select another flag source",
                path.display()
            ));
        }
    }
    if text.contains("领导人") && leader.is_none() {
        questions.push("which indexed character should be recruited or set as leader?".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"tag\": {},\n  \"cosmetic_tag\": {},\n  \"cosmetic_exists\": {},\n  \"ruling_party\": {},\n  \"leader_character\": {},\n  \"localisation_state\": [{}],\n  \"flag_state\": [{}],\n  \"planned_files\": {},\n  \"formation_transaction\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.cosmetic_transition_plan.v1"),
        json_bool(ok),
        json_str(if ok { "cosmetic_transition_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_str(&tag),
        json_str(&cosmetic),
        json_bool(cosmetic_exists),
        json_optional_str(ruling_party.as_deref()),
        json_optional_str(leader.as_deref()),
        render_key_state_rows(&loc_state),
        render_path_state_rows(&flag_state),
        json_array(&[
            "localisation/simp_chinese/generated_cosmetic_tags_l_simp_chinese.yml".to_string(),
            "gfx/flags/<cosmetic>.tga plus medium/small variants when needed".to_string(),
            "common/scripted_effects/generated_cosmetic_transitions.txt".to_string(),
        ]),
        json_array(&formation_operations(&cosmetic, ruling_party.as_deref(), leader.as_deref())),
        json_array(&blockers),
        json_array(&questions),
        json_str("cosmetic transitions must prove or create localisation and flag assets; set_cosmetic_tag stays in country/tag scope and formation order is explicit")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_flag_copy_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = ideology_game_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let from_ideology = require_value(&map, "from-ideology")?;
    let target_flag = value(&map, "target-flag-id")
        .map(str::to_string)
        .unwrap_or_else(|| require_value(&map, "to-cosmetic").unwrap_or_else(|_| tag.clone()));
    let source_flag = format!("{tag}_{from_ideology}");
    let source_state = flag_triplet_state(&index.game_root, &source_flag);
    let target_state = flag_triplet_state(&index.game_root, &target_flag);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !index.country_tags.contains(&tag) {
        blockers.push(format!("country tag `{tag}` is not indexed"));
    }
    if !index.ideologies.contains(&from_ideology) {
        blockers.push(format!("source ideology `{from_ideology}` is not indexed"));
    }
    for (path, exists) in &source_state {
        if !exists {
            blockers.push(format!("source flag asset `{}` is missing", path.display()));
        }
    }
    for (path, exists) in &target_state {
        if *exists {
            questions.push(format!(
                "target flag asset `{}` already exists; reuse, overwrite, or choose a new cosmetic flag id?",
                path.display()
            ));
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"from_ideology\": {},\n  \"source_flag_id\": {},\n  \"target_flag_id\": {},\n  \"source_state\": [{}],\n  \"target_state\": [{}],\n  \"operations\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.flag_copy_plan.v1"),
        json_bool(ok),
        json_str(if ok { "flag_copy_plan_ready" } else { "blocked" }),
        json_str(&tag),
        json_str(&from_ideology),
        json_str(&source_flag),
        json_str(&target_flag),
        render_path_state_rows(&source_state),
        render_path_state_rows(&target_state),
        json_array(&[
            format!("copy normal/medium/small `{source_flag}` flag triplet to `{target_flag}`"),
            "do not reference a cosmetic flag until all three variants are present or explicitly generated".to_string(),
        ]),
        json_array(&blockers),
        json_array(&questions),
        json_str("flag copy plans require indexed tags, indexed ideologies, and a complete source flag triplet")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_country_name_batch_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let index = ideology_game_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let cosmetic = require_value(&map, "cosmetic")?;
    let name = require_value(&map, "name")?;
    let def = value(&map, "def").unwrap_or(&name);
    let adj = require_value(&map, "adj")?;
    let loc_state = cosmetic_localisation_state(&index, &cosmetic);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !index.country_tags.contains(&tag) {
        blockers.push(format!("country tag `{tag}` is not indexed"));
    }
    for (key, exists) in &loc_state {
        if *exists {
            questions.push(format!(
                "localisation key `{key}` already exists; reuse or overwrite?"
            ));
        }
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"tag\": {},\n  \"cosmetic_tag\": {},\n  \"entries\": {},\n  \"existing_keys\": [{}],\n  \"planned_files\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.country_name_batch_plan.v1"),
        json_bool(ok),
        json_str(if ok { "country_name_plan_ready" } else { "blocked" }),
        json_str(&tag),
        json_str(&cosmetic),
        json_raw_object(&BTreeMap::from([
            (cosmetic.clone(), json_str(&name)),
            (format!("{cosmetic}_DEF"), json_str(def)),
            (format!("{cosmetic}_ADJ"), json_str(&adj)),
        ])),
        render_key_state_rows(&loc_state),
        json_array(&["localisation/simp_chinese/generated_cosmetic_tags_l_simp_chinese.yml".to_string()]),
        json_array(&blockers),
        json_array(&questions),
        json_str("country name plans register cosmetic, DEF, and ADJ keys together so localisation placeholders can resolve them later")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_formation_chain_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let index = ideology_game_index(&map)?;
    let tag = require_value(&map, "tag")?;
    let cosmetic = value(&map, "cosmetic")
        .map(str::to_string)
        .or_else(|| infer_unique_country_or_cosmetic_from_text(&index, &text));
    let ruling_party = value(&map, "ruling-party").map(str::to_string).or_else(|| {
        let inferred = infer_ruling_party(&text);
        index
            .ideologies
            .contains(inferred)
            .then(|| inferred.to_string())
    });
    let leader = value(&map, "leader-character").map(str::to_string);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !index.country_tags.contains(&tag) {
        blockers.push(format!("country tag `{tag}` is not indexed"));
    }
    if cosmetic.is_none() {
        questions.push(
            "which cosmetic tag should formation use? provide --cosmetic or indexed country-name evidence"
                .to_string(),
        );
    }
    if let Some(party) = &ruling_party {
        if !index.ideologies.contains(party) {
            blockers.push(format!("ruling party ideology `{party}` is not indexed"));
        }
    }
    if text.contains("领导人") && leader.is_none() {
        questions.push("which indexed character should become leader after formation?".to_string());
    }
    let mut required_effects = vec!["set_cosmetic_tag"];
    if ruling_party.is_some() {
        required_effects.push("set_politics");
    }
    if leader.is_some() {
        required_effects.push("recruit_character");
    }
    for effect in required_effects {
        if !effect_available_or_unknown(&index, effect) {
            blockers.push(format!("effect `{effect}` is not indexed"));
        }
    }
    let ok = blockers.is_empty();
    let cosmetic_for_ops = cosmetic.as_deref().unwrap_or("<cosmetic>");
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"text\": {},\n  \"tag\": {},\n  \"cosmetic_tag\": {},\n  \"ruling_party\": {},\n  \"leader_character\": {},\n  \"formation_transaction\": {},\n  \"trigger_links\": {},\n  \"blockers\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.formation_chain_plan.v1"),
        json_bool(ok),
        json_str(if ok { "formation_chain_plan_ready" } else { "blocked" }),
        json_str(&text),
        json_str(&tag),
        json_optional_str(cosmetic.as_deref()),
        json_optional_str(ruling_party.as_deref()),
        json_optional_str(leader.as_deref()),
        json_array(&formation_operations(
            cosmetic_for_ops,
            ruling_party.as_deref(),
            leader.as_deref(),
        )),
        json_array(&[
            "focus completion_reward or event option calls one generated scripted_effect".to_string(),
            "generated scripted_effect applies set_cosmetic_tag before politics and character recruitment".to_string(),
            "route graph must later prove which focus/event/decision triggers the formation".to_string(),
        ]),
        json_array(&blockers),
        json_array(&questions),
        json_str("formation chains are ordered transactions; AI may describe the route, but Rust validates effect symbols and trigger links before final code")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn infer_parent_ideology(text: &str) -> &'static str {
    if text.contains("中立") {
        "neutrality"
    } else if text.contains("民主") {
        "democratic"
    } else if text.contains("共产") || text.contains("社会主义") {
        "communism"
    } else if text.contains("法西斯") {
        "fascism"
    } else {
        "neutrality"
    }
}

fn infer_ruling_party(text: &str) -> &'static str {
    if text.contains("中立") {
        "neutrality"
    } else if text.contains("民主") {
        "democratic"
    } else if text.contains("共产") || text.contains("社会主义") {
        "communism"
    } else if text.contains("法西斯") {
        "fascism"
    } else {
        "neutrality"
    }
}

fn infer_ideology_title(text: &str) -> String {
    for marker in ["子意识形态", "意识形态"] {
        if let Some(after) = text.split(marker).nth(1) {
            let title = after
                .trim_matches(['，', ',', '：', ':', ' ', '\n'])
                .split(['，', ',', '。', '\n'])
                .next()
                .unwrap_or("")
                .trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }
    "新意识形态".to_string()
}

fn infer_new_ideology(text: &str) -> String {
    if text.contains("社会主义") {
        "socialism".to_string()
    } else if text.contains("中国特色") {
        "socialism_with_chinese_characteristics".to_string()
    } else {
        slugify(&infer_ideology_title(text), "new_ideology")
    }
}

fn infer_source_ideology_after(text: &str, marker: &str) -> Option<String> {
    let after = text.split(marker).nth(1)?;
    if after.contains("共产主义") {
        Some("communism".to_string())
    } else if after.contains("民主主义") || after.contains("民主") {
        Some("democratic".to_string())
    } else if after.contains("中立") {
        Some("neutrality".to_string())
    } else if after.contains("法西斯") {
        Some("fascism".to_string())
    } else {
        None
    }
}

fn ideology_game_index(map: &ArgMap) -> Result<GameIndex, String> {
    let game_root = normalize_path(&require_value(map, "game-root")?)?;
    let mod_root = value(map, "mod-root").map(normalize_path).transpose()?;
    let mod_paths = dependency_mod_roots_for_optional_edited_mod(map, mod_root.as_deref(), true)?;
    build_game_index_with_mod_paths(&game_root, &mod_paths)
}

fn parse_popularity_pair(raw: &str) -> Result<(String, i64), String> {
    let Some((ideology, value)) = raw.split_once('=') else {
        return Err(format!("--popularity expects ideology=value, got `{raw}`"));
    };
    let value = value
        .trim()
        .trim_end_matches('%')
        .parse::<i64>()
        .map_err(|_| format!("--popularity value must be an integer percentage, got `{raw}`"))?;
    Ok((ideology.trim().to_string(), value))
}

fn render_popularity_rows(values: &[(String, i64)]) -> String {
    values
        .iter()
        .map(|(ideology, value)| {
            format!(
                "{{\"ideology\": {}, \"value\": {}}}",
                json_str(ideology),
                value
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn cosmetic_localisation_state(index: &GameIndex, cosmetic: &str) -> Vec<(String, bool)> {
    [
        cosmetic.to_string(),
        format!("{cosmetic}_DEF"),
        format!("{cosmetic}_ADJ"),
    ]
    .into_iter()
    .map(|key| {
        let exists = index.localisation_entries.contains_key(&key);
        (key, exists)
    })
    .collect()
}

fn flag_triplet_state(game_root: &Path, flag_id: &str) -> Vec<(PathBuf, bool)> {
    [
        game_root
            .join("gfx")
            .join("flags")
            .join(format!("{flag_id}.tga")),
        game_root
            .join("gfx")
            .join("flags")
            .join("medium")
            .join(format!("{flag_id}.tga")),
        game_root
            .join("gfx")
            .join("flags")
            .join("small")
            .join(format!("{flag_id}.tga")),
    ]
    .into_iter()
    .map(|path| {
        let exists = path.exists();
        (path, exists)
    })
    .collect()
}

fn render_key_state_rows(values: &[(String, bool)]) -> String {
    values
        .iter()
        .map(|(key, exists)| {
            format!(
                "{{\"key\": {}, \"exists\": {}}}",
                json_str(key),
                json_bool(*exists)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_path_state_rows(values: &[(PathBuf, bool)]) -> String {
    values
        .iter()
        .map(|(path, exists)| {
            format!(
                "{{\"path\": {}, \"exists\": {}}}",
                json_str(&path.display().to_string()),
                json_bool(*exists)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn effect_available_or_unknown(index: &GameIndex, effect: &str) -> bool {
    index.effects.is_empty() || index.effects.contains(effect)
}

fn formation_operations(
    cosmetic: &str,
    ruling_party: Option<&str>,
    leader: Option<&str>,
) -> Vec<String> {
    let mut operations = vec![format!("set_cosmetic_tag = {cosmetic}")];
    if let Some(party) = ruling_party {
        operations.push(format!("set_politics.ruling_party = {party}"));
    }
    if let Some(leader) = leader {
        operations.push(format!("recruit_character = {leader}"));
    }
    operations
}

fn infer_unique_country_or_cosmetic_from_text(index: &GameIndex, text: &str) -> Option<String> {
    let mut matches = BTreeSet::new();
    for (name, tags) in &index.country_name_tags {
        if text.contains(name) {
            for tag in tags {
                matches.insert(tag.clone());
            }
        }
    }
    (matches.len() == 1).then(|| matches.into_iter().next().unwrap())
}

fn render_indexed_roots(index: Option<&GameIndex>) -> String {
    index
        .map(|index| {
            let roots = index
                .indexed_roots
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            json_array(&roots)
        })
        .unwrap_or_else(|| "[]".to_string())
}
