//! Decision, idea, technology, GUI, and scripted-helper card parsing and application.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_parse_feature_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = require_value(&map, "input")?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let text = read_utf8_lossy(&normalize_path(&input)?)?;
    let json = parse_decision_idea_cards_json(&text, tag, prefix);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn parse_decision_idea_cards_json(text: &str, tag: &str, prefix: &str) -> String {
    let cards = parse_cards(text, FEATURE_CARD_HEADERS);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str("  \"features\": [\n");
    for (idx, card) in cards.iter().enumerate() {
        comma(&mut out, idx, "    ");
        let ty = feature_card_type(&card.kind).unwrap_or("feature");
        let id = feature_card_id(card, prefix, ty, idx);
        let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
        let loc_file = target_localisation_relative_path(target);
        let files = if ty == "decision" {
            vec![
                format!("common/decisions/{prefix}_decisions.txt"),
                format!("common/decisions/categories/{prefix}_categories.txt"),
                loc_file,
            ]
        } else if ty == "idea" {
            vec![format!("common/ideas/{prefix}_ideas.txt"), loc_file]
        } else if ty == "technology" {
            vec![
                format!("common/technologies/{prefix}_technologies.txt"),
                loc_file,
            ]
        } else if ty == "gui" {
            vec![
                format!("common/scripted_guis/{prefix}_scripted_guis.txt"),
                format!("interface/{prefix}.gui"),
                loc_file,
            ]
        } else if ty == "scripted_effect" {
            vec![format!(
                "common/scripted_effects/{prefix}_scripted_effects.txt"
            )]
        } else if ty == "scripted_trigger" {
            vec![format!(
                "common/scripted_triggers/{prefix}_scripted_triggers.txt"
            )]
        } else if ty == "state_effect" {
            vec![format!(
                "common/scripted_effects/{prefix}_state_effects.txt"
            )]
        } else {
            vec![loc_file]
        };
        let condition =
            join_existing_fields(&card.fields, &["条件", "可见", "可用", "前置", "前置国策"]);
        let suggestions = feature_card_suggestions(card, ty, &id, target, condition.as_deref());
        out.push_str(&format!(
            "{{\"type\": {}, \"title\": {}, \"target\": {}, \"id\": {}, \"fields\": {}, \"files\": {}, \"suggestions\": {}}}",
            json_str(ty),
            json_str(&card.title),
            json_str(target),
            json_str(&id),
            json_object(&card.fields),
            json_array(&files),
            suggestions_json(&suggestions)
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn feature_card_suggestions(
    card: &Card,
    ty: &str,
    id: &str,
    target: &str,
    condition: Option<&str>,
) -> Vec<Suggestion> {
    match ty {
        "decision" | "idea" => suggest_common(
            ty,
            card.fields.get("效果").map(String::as_str).unwrap_or(""),
            card.fields.get("花费").map(String::as_str),
            card.fields
                .get("冷却")
                .or_else(|| card.fields.get("持续"))
                .map(String::as_str),
            condition,
            card.fields.get("移除").map(String::as_str),
        ),
        "technology" => vec![Suggestion::new(
            "technology_script",
            &format!(
                "technologies = {{\n{}}}",
                render_technology_inner_block(card, id, target)
            ),
            &card.title,
            "Minimal unique technology skeleton; place it in common/technologies.",
        )],
        "gui" => vec![
            Suggestion::new(
                "scripted_gui",
                &format!(
                    "scripted_gui = {{\n{}}}",
                    render_scripted_gui_inner_block(card, id, target)
                ),
                &card.title,
                "Scripted GUI hook; connect it from decisions, events, or existing UI logic.",
            ),
            Suggestion::new(
                "interface_gui",
                &format!(
                    "guiTypes = {{\n{}}}",
                    render_interface_gui_window_block(card, id)
                ),
                &card.title,
                "Minimal interface window skeleton.",
            ),
        ],
        "scripted_effect" => vec![Suggestion::new(
            "scripted_effect",
            &render_scripted_effect_block(card, id),
            &card.title,
            "Reusable scripted effect; call it from focuses, decisions, events, or scripted GUI effects.",
        )],
        "scripted_trigger" => vec![Suggestion::new(
            "scripted_trigger",
            &render_scripted_trigger_block(card, id),
            &card.title,
            "Reusable scripted trigger; call it from visible, available, trigger, or limit blocks.",
        )],
        "state_effect" => vec![Suggestion::new(
            "state_effect",
            &render_state_effect_block(card, id, target),
            &card.title,
            "State-scoped helper; call it from a state scope or review the generated state-id wrapper.",
        )],
        _ => Vec::new(),
    }
}

pub(crate) fn cmd_apply_feature_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = require_value(&map, "input")?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let dependency_mods = dependency_mod_roots(&map)?;
    let game_index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|path| build_game_index_with_mod_paths(&path, &dependency_mods))
        .transpose()?;
    if game_index.is_none() && !dependency_mods.is_empty() {
        return Err("--mod-path requires --game-root during feature-card generation".to_string());
    }
    let text = read_utf8_lossy(&normalize_path(&input)?)?;
    let cards = parse_cards(&text, FEATURE_CARD_HEADERS);
    let changed =
        apply_feature_cards_to_mod_with_index(&mod_root, &cards, tag, prefix, game_index.as_ref())?;

    println!("Applied feature cards: {}", cards.len());
    if changed.is_empty() {
        println!("No file changes were needed.");
    } else {
        println!("Changed:");
        for path in changed {
            println!("  {}", path.display());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn apply_feature_cards_to_mod(
    mod_root: &Path,
    cards: &[Card],
    tag: &str,
    prefix: &str,
) -> Result<Vec<PathBuf>, String> {
    apply_feature_cards_to_mod_with_index(mod_root, cards, tag, prefix, None)
}

pub(crate) fn apply_feature_cards_to_mod_with_index(
    mod_root: &Path,
    cards: &[Card],
    tag: &str,
    prefix: &str,
    game_index: Option<&GameIndex>,
) -> Result<Vec<PathBuf>, String> {
    let decision_targets = scan_decision_category_targets(mod_root)?;
    let idea_targets = scan_idea_file_targets(mod_root)?;
    let icon_catalog = collect_feature_icon_catalog(mod_root, game_index)?;
    let mut categories: BTreeMap<String, GeneratedDecisionCategory> = BTreeMap::new();
    let mut decision_blocks: Vec<(String, String)> = Vec::new();
    let mut existing_decision_appends: BTreeMap<PathBuf, BTreeMap<String, Vec<(String, String)>>> =
        BTreeMap::new();
    let mut idea_blocks: Vec<(String, String)> = Vec::new();
    let mut existing_idea_appends: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
    let mut technology_blocks: Vec<(String, String)> = Vec::new();
    let mut scripted_gui_blocks: Vec<(String, String)> = Vec::new();
    let mut interface_gui_blocks: Vec<(String, String)> = Vec::new();
    let mut scripted_effect_blocks: Vec<(String, String)> = Vec::new();
    let mut scripted_trigger_blocks: Vec<(String, String)> = Vec::new();
    let mut state_effect_blocks: Vec<(String, String)> = Vec::new();
    let mut loc_entries: BTreeMap<String, String> = BTreeMap::new();

    for (idx, card) in cards.iter().enumerate() {
        match card.kind.as_str() {
            "决议" => {
                let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
                let category_title = card
                    .fields
                    .get("分类")
                    .map(String::as_str)
                    .unwrap_or("国家决议");
                let category_id = format!(
                    "{}_{}",
                    prefix,
                    slugify(category_title, &format!("category_{idx}"))
                );
                let decision_id = feature_card_id(card, prefix, "decision", idx);
                let decision_icon = resolve_decision_icon(card, &icon_catalog.decision_icons);
                if let Some(existing) = select_decision_category_target(
                    &decision_targets,
                    target,
                    category_title,
                    card.fields.contains_key("分类"),
                ) {
                    existing_decision_appends
                        .entry(existing.path.clone())
                        .or_default()
                        .entry(existing.id.clone())
                        .or_default()
                        .push((
                            decision_id.clone(),
                            render_decision_inner_block_with_icon(
                                card,
                                &decision_id,
                                target,
                                idx,
                                &decision_icon,
                            ),
                        ));
                } else {
                    let category_picture = resolve_decision_category_picture(
                        card,
                        category_title,
                        &icon_catalog.decision_category_pictures,
                    );
                    let category_icon = format!("GFX_decision_{decision_icon}");
                    categories.entry(category_id.clone()).or_insert_with(|| {
                        GeneratedDecisionCategory {
                            target: target.to_string(),
                            icon: category_icon,
                            picture: category_picture,
                        }
                    });
                    decision_blocks.push((
                        decision_id.clone(),
                        render_decision_block_with_icon(
                            card,
                            &category_id,
                            &decision_id,
                            target,
                            idx,
                            &decision_icon,
                        ),
                    ));
                    loc_entries.insert(category_id, category_title.to_string());
                }
                loc_entries.insert(decision_id.clone(), card.title.clone());
                loc_entries.insert(
                    format!("{decision_id}_desc"),
                    card.fields
                        .get("描述")
                        .cloned()
                        .unwrap_or_else(|| fallback_decision_description(card)),
                );
            }
            "民族精神" => {
                let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
                let idea_id = feature_card_id(card, prefix, "idea", idx);
                let picture = resolve_idea_picture(card, &icon_catalog.idea_pictures);
                if let Some(existing) = select_idea_file_target(&idea_targets, target) {
                    existing_idea_appends
                        .entry(existing.path.clone())
                        .or_default()
                        .push((
                            idea_id.clone(),
                            render_idea_inner_block_with_picture(card, &idea_id, &picture),
                        ));
                } else {
                    idea_blocks.push((
                        idea_id.clone(),
                        render_idea_block_with_picture(card, &idea_id, &picture),
                    ));
                }
                loc_entries.insert(idea_id.clone(), card.title.clone());
                loc_entries.insert(
                    format!("{idea_id}_desc"),
                    card.fields
                        .get("描述")
                        .cloned()
                        .unwrap_or_else(|| fallback_idea_description(card)),
                );
            }
            kind if is_technology_card(kind) => {
                let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
                let technology_id = feature_card_id(card, prefix, "technology", idx);
                technology_blocks.push((
                    technology_id.clone(),
                    render_technology_inner_block(card, &technology_id, target),
                ));
                loc_entries.insert(technology_id.clone(), card.title.clone());
                loc_entries.insert(
                    format!("{technology_id}_desc"),
                    card.fields
                        .get("描述")
                        .cloned()
                        .unwrap_or_else(|| format!("{}是一项独有技术。", card.title)),
                );
            }
            kind if is_gui_card(kind) => {
                let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
                let gui_id = feature_card_id(card, prefix, "gui", idx);
                scripted_gui_blocks.push((
                    gui_id.clone(),
                    render_scripted_gui_inner_block(card, &gui_id, target),
                ));
                interface_gui_blocks.push((
                    format!("{gui_id}_window"),
                    render_interface_gui_window_block(card, &gui_id),
                ));
                loc_entries.insert(gui_id.clone(), card.title.clone());
                loc_entries.insert(
                    format!("{gui_id}_desc"),
                    card.fields
                        .get("描述")
                        .or_else(|| card.fields.get("用途"))
                        .cloned()
                        .unwrap_or_else(|| format!("{}界面。", card.title)),
                );
            }
            kind if is_scripted_effect_card(kind) => {
                let effect_id = feature_card_id(card, prefix, "scripted_effect", idx);
                scripted_effect_blocks.push((
                    effect_id.clone(),
                    render_scripted_effect_block(card, &effect_id),
                ));
            }
            kind if is_scripted_trigger_card(kind) => {
                let trigger_id = feature_card_id(card, prefix, "scripted_trigger", idx);
                scripted_trigger_blocks.push((
                    trigger_id.clone(),
                    render_scripted_trigger_block(card, &trigger_id),
                ));
            }
            kind if is_state_effect_card(kind) => {
                let target = card.fields.get("目标").map(String::as_str).unwrap_or(tag);
                let state_effect_id = feature_card_id(card, prefix, "state_effect", idx);
                state_effect_blocks.push((
                    state_effect_id.clone(),
                    render_state_effect_block(card, &state_effect_id, target),
                ));
            }
            _ => {}
        }
    }

    let mut changed = Vec::new();
    for (path, appends) in existing_decision_appends {
        if append_decisions_to_existing_categories(&path, &appends)? {
            changed.push(path);
        }
    }
    for (path, appends) in existing_idea_appends {
        if append_ideas_to_existing_country_wrapper(&path, &appends)? {
            changed.push(path);
        }
    }
    if !categories.is_empty() {
        let blocks = categories
            .iter()
            .map(|(id, category)| {
                (
                    id.clone(),
                    render_decision_category_block_with_icons(
                        id,
                        &category.target,
                        &category.icon,
                        &category.picture,
                    ),
                )
            })
            .collect::<Vec<_>>();
        let path = mod_root
            .join("common")
            .join("decisions")
            .join("categories")
            .join(format!("{prefix}_categories.txt"));
        if append_unique_blocks(
            &path,
            "# Generated decision categories by hoi4skill\n",
            &blocks,
        )? {
            changed.push(path);
        }
    }
    if !decision_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("decisions")
            .join(format!("{prefix}_decisions.txt"));
        if append_unique_blocks(
            &path,
            "# Generated decisions by hoi4skill\n",
            &decision_blocks,
        )? {
            changed.push(path);
        }
    }
    if !idea_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("ideas")
            .join(format!("{prefix}_ideas.txt"));
        if append_unique_blocks(&path, "# Generated ideas by hoi4skill\n", &idea_blocks)? {
            changed.push(path);
        }
    }
    if !technology_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("technologies")
            .join(format!("{prefix}_technologies.txt"));
        if append_blocks_to_named_wrapper(
            &path,
            "technologies",
            "# Generated technologies by hoi4skill\n",
            &technology_blocks,
        )? {
            changed.push(path);
        }
    }
    if !scripted_gui_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("scripted_guis")
            .join(format!("{prefix}_scripted_guis.txt"));
        if append_blocks_to_named_wrapper(
            &path,
            "scripted_gui",
            "# Generated scripted GUI hooks by hoi4skill\n",
            &scripted_gui_blocks,
        )? {
            changed.push(path);
        }
    }
    if !interface_gui_blocks.is_empty() {
        let path = mod_root.join("interface").join(format!("{prefix}.gui"));
        if append_blocks_to_named_wrapper(
            &path,
            "guiTypes",
            "# Generated GUI windows by hoi4skill\n",
            &interface_gui_blocks,
        )? {
            changed.push(path);
        }
    }
    if !scripted_effect_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("scripted_effects")
            .join(format!("{prefix}_scripted_effects.txt"));
        if append_unique_blocks(
            &path,
            "# Generated scripted effects by hoi4skill\n",
            &scripted_effect_blocks,
        )? {
            changed.push(path);
        }
    }
    if !scripted_trigger_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("scripted_triggers")
            .join(format!("{prefix}_scripted_triggers.txt"));
        if append_unique_blocks(
            &path,
            "# Generated scripted triggers by hoi4skill\n",
            &scripted_trigger_blocks,
        )? {
            changed.push(path);
        }
    }
    if !state_effect_blocks.is_empty() {
        let path = mod_root
            .join("common")
            .join("scripted_effects")
            .join(format!("{prefix}_state_effects.txt"));
        if append_unique_blocks(
            &path,
            "# Generated state-scoped effects by hoi4skill\n",
            &state_effect_blocks,
        )? {
            changed.push(path);
        }
    }
    if !loc_entries.is_empty() {
        let path = target_localisation_path(mod_root, tag);
        if append_localisation_entries(&path, &loc_entries)? {
            changed.push(path);
        }
    }

    Ok(changed)
}

pub(crate) fn feature_card_id(card: &Card, prefix: &str, ty: &str, idx: usize) -> String {
    let fallback = if ty == "idea" {
        format!("spirit_{idx}")
    } else {
        format!("{ty}_{idx}")
    };
    let id = format!(
        "{}_{}",
        sanitize_identifier_part(prefix, "mod"),
        slugify(&card.title, &fallback)
    );
    if ty == "idea" {
        ensure_idea_id_suffix(&id)
    } else if ty == "technology" {
        ensure_identifier_suffix(&id, "technology", "_tech")
    } else if ty == "gui" {
        ensure_identifier_suffix(&id, "gui", "_gui")
    } else if ty == "scripted_effect" {
        ensure_identifier_suffix(&id, "scripted_effect", "_effect")
    } else if ty == "scripted_trigger" {
        ensure_identifier_suffix(&id, "scripted_trigger", "_trigger")
    } else if ty == "state_effect" {
        ensure_identifier_suffix(&id, "state_effect", "_state_effect")
    } else {
        id
    }
}

pub(crate) fn ensure_identifier_suffix(id: &str, fallback: &str, suffix: &str) -> String {
    let id = sanitize_identifier_part(id, fallback);
    if id.ends_with(suffix) {
        id
    } else {
        format!("{id}{suffix}")
    }
}

pub(crate) fn ensure_idea_id_suffix(id: &str) -> String {
    let id = sanitize_identifier_part(id, "idea");
    if id.ends_with("_idea") {
        id
    } else {
        format!("{id}_idea")
    }
}

pub(crate) fn ensure_idea_localisation_key_suffix(id: &str) -> String {
    let id = sanitize_localisation_key(id, "idea");
    if id.ends_with("_idea") {
        id
    } else {
        format!("{id}_idea")
    }
}

pub(crate) fn render_decision_category_block_with_icons(
    id: &str,
    target: &str,
    icon: &str,
    picture: &str,
) -> String {
    format!(
        "{id} = {{\n\ticon = {icon}\n\tpicture = {picture}\n\tallowed = {{\n\t\toriginal_tag = {target}\n\t}}\n\tvisible = {{\n\t\ttag = {target}\n\t}}\n\tvisible_when_empty = yes\n}}\n"
    )
}

pub(crate) struct GeneratedDecisionCategory {
    pub(crate) target: String,
    pub(crate) icon: String,
    pub(crate) picture: String,
}

#[derive(Clone)]
pub(crate) struct DecisionCategoryTarget {
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) path: PathBuf,
    pub(crate) target_tags: BTreeSet<String>,
    pub(crate) scripted_gui: bool,
    pub(crate) decision_count: usize,
}

#[derive(Default, Clone)]
pub(crate) struct DecisionCategoryDefinition {
    pub(crate) title: Option<String>,
    pub(crate) target_tags: BTreeSet<String>,
    pub(crate) scripted_gui: bool,
}

pub(crate) fn scan_decision_category_targets(
    root: &Path,
) -> Result<Vec<DecisionCategoryTarget>, String> {
    let localisation = collect_focus_localisation_map(root)?;
    let definitions = scan_decision_category_definitions(root, &localisation)?;
    let mut targets = Vec::new();
    let decisions_root = root.join("common").join("decisions");
    if !decisions_root.exists() {
        return Ok(targets);
    }
    for file in collect_files(&decisions_root)? {
        let norm = slash_path(&file);
        if !norm.ends_with(".txt") || norm.contains("/common/decisions/categories/") {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for (id, range) in direct_block_ranges(&text) {
            let definition =
                definitions
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| DecisionCategoryDefinition {
                        title: localisation.get(&id).cloned(),
                        target_tags: decision_category_tags(&range.content),
                        scripted_gui: range.content.contains("scripted_gui"),
                    });
            targets.push(DecisionCategoryTarget {
                id,
                title: definition.title,
                path: file.clone(),
                target_tags: definition.target_tags,
                scripted_gui: definition.scripted_gui,
                decision_count: direct_block_ranges(&range.content).len(),
            });
        }
    }
    Ok(targets)
}

pub(crate) fn scan_decision_category_definitions(
    root: &Path,
    localisation: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, DecisionCategoryDefinition>, String> {
    let mut definitions = BTreeMap::new();
    let categories_root = root.join("common").join("decisions").join("categories");
    if !categories_root.exists() {
        return Ok(definitions);
    }
    for file in collect_files(&categories_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for (id, range) in direct_block_ranges(&text) {
            definitions.insert(
                id.clone(),
                DecisionCategoryDefinition {
                    title: localisation.get(&id).cloned(),
                    target_tags: decision_category_tags(&range.content),
                    scripted_gui: range.content.contains("scripted_gui"),
                },
            );
        }
    }
    Ok(definitions)
}

pub(crate) fn decision_category_tags(text: &str) -> BTreeSet<String> {
    let mut tags = BTreeSet::new();
    for key in ["tag", "original_tag"] {
        for value in assignment_values_in_text(text, key) {
            let value = value.trim_matches('"').to_ascii_uppercase();
            if looks_like_tag(&value) {
                tags.insert(value);
            }
        }
    }
    tags
}

pub(crate) fn select_decision_category_target<'a>(
    targets: &'a [DecisionCategoryTarget],
    target: &str,
    category_title: &str,
    explicit_category: bool,
) -> Option<&'a DecisionCategoryTarget> {
    let target = sanitize_identifier_part(target, "TAG").to_ascii_uppercase();
    let requested_slug = slugify(category_title, "");
    targets
        .iter()
        .filter(|candidate| {
            let target_ok =
                candidate.target_tags.is_empty() || candidate.target_tags.contains(&target);
            if !target_ok {
                return false;
            }
            if explicit_category {
                decision_category_name_matches(candidate, category_title, &requested_slug)
            } else {
                !candidate.scripted_gui && !candidate.target_tags.is_empty()
            }
        })
        .max_by_key(|candidate| {
            let name_match = usize::from(decision_category_name_matches(
                candidate,
                category_title,
                &requested_slug,
            ));
            let target_match = usize::from(candidate.target_tags.contains(&target));
            (name_match, target_match, candidate.decision_count)
        })
}

pub(crate) fn decision_category_name_matches(
    candidate: &DecisionCategoryTarget,
    category_title: &str,
    requested_slug: &str,
) -> bool {
    candidate.id == category_title
        || (!requested_slug.is_empty()
            && (candidate.id == requested_slug
                || candidate.id.ends_with(&format!("_{requested_slug}"))))
        || candidate
            .title
            .as_deref()
            .is_some_and(|title| title.trim() == category_title.trim())
}

pub(crate) fn render_decision_block_with_icon(
    card: &Card,
    category_id: &str,
    decision_id: &str,
    target: &str,
    idx: usize,
    icon: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("{category_id} = {{\n"));
    out.push_str(&render_decision_inner_block_with_icon(
        card,
        decision_id,
        target,
        idx,
        icon,
    ));
    out.push_str("}\n");
    out
}

pub(crate) fn render_decision_inner_block_with_icon(
    card: &Card,
    decision_id: &str,
    target: &str,
    idx: usize,
    icon: &str,
) -> String {
    let cost = card
        .fields
        .get("花费")
        .and_then(|s| parse_int(s))
        .unwrap_or(50);
    let days = card
        .fields
        .get("冷却")
        .or_else(|| card.fields.get("持续"))
        .and_then(|s| parse_int(s));
    let condition = join_existing_fields(&card.fields, &["条件", "可用", "前置", "前置国策"]);
    let suggestions = suggest_common(
        "decision",
        card.fields.get("效果").map(String::as_str).unwrap_or(""),
        None,
        None,
        condition.as_deref(),
        None,
    );
    let trigger_lines = concrete_suggestion_lines(&suggestions, &["trigger"]);
    let (effect_lines, effect_comments) = decision_effect_lines(&suggestions);
    let mut out = String::new();
    out.push_str(&format!("\t{decision_id} = {{\n"));
    out.push_str(&format!("\t\ticon = {icon}\n"));
    out.push_str(&format!("\t\tcost = {cost}\n"));
    if let Some(days) = days {
        out.push_str(&format!("\t\tdays_remove = {days}\n"));
    }
    out.push_str("\t\tvisible = {\n");
    out.push_str(&format!("\t\t\ttag = {target}\n"));
    out.push_str("\t\t}\n");
    out.push_str("\t\tavailable = {\n");
    if trigger_lines.is_empty() {
        out.push_str("\t\t\talways = yes\n");
    } else {
        for line in trigger_lines {
            out.push_str(&format!("\t\t\t{line}\n"));
        }
    }
    out.push_str("\t\t}\n");
    out.push_str("\t\tcomplete_effect = {\n");
    if effect_lines.is_empty() && effect_comments.is_empty() {
        out.push_str(&format!(
            "\t\t\t# TODO: add effects for card {} ({})\n",
            idx + 1,
            card.title
        ));
    }
    for comment in effect_comments {
        out.push_str(&format!("\t\t\t# {comment}\n"));
    }
    for line in effect_lines {
        out.push_str(&indent_lines(&line, "\t\t\t"));
    }
    out.push_str("\t\t}\n");
    out.push_str("\t\tai_will_do = {\n\t\t\tfactor = 1\n\t\t}\n");
    out.push_str("\t}\n");
    out
}

#[cfg(test)]
pub(crate) fn render_idea_block(card: &Card, idea_id: &str) -> String {
    let picture = resolve_idea_picture(card, &BTreeSet::new());
    render_idea_block_with_picture(card, idea_id, &picture)
}

pub(crate) fn render_idea_block_with_picture(card: &Card, idea_id: &str, picture: &str) -> String {
    let mut out = String::new();
    out.push_str("ideas = {\n\tcountry = {\n");
    out.push_str(&render_idea_inner_block_with_picture(
        card, idea_id, picture,
    ));
    out.push_str("\t}\n}\n");
    out
}

pub(crate) fn render_idea_inner_block_with_picture(
    card: &Card,
    idea_id: &str,
    picture: &str,
) -> String {
    let suggestions = suggest_common(
        "idea",
        card.fields.get("效果").map(String::as_str).unwrap_or(""),
        None,
        None,
        None,
        card.fields.get("移除").map(String::as_str),
    );
    let modifier_lines =
        concrete_suggestion_lines(&suggestions, &["idea_modifier", "idea_modifier_candidate"]);
    let field_lines = concrete_suggestion_lines(&suggestions, &["idea_field"]);
    let mut out = String::new();
    out.push_str(&format!("\t\t{idea_id} = {{\n"));
    out.push_str(&format!("\t\t\tpicture = {picture}\n"));
    for line in field_lines {
        out.push_str(&format!("\t\t\t{line}\n"));
    }
    out.push_str("\t\t\tmodifier = {\n");
    if modifier_lines.is_empty() {
        out.push_str("\t\t\t\t# TODO: add idea modifiers from card effects\n");
    } else {
        for line in modifier_lines {
            out.push_str(&format!("\t\t\t\t{line}\n"));
        }
    }
    out.push_str("\t\t\t}\n");
    out.push_str("\t\t}\n");
    out
}

#[derive(Default)]
pub(crate) struct FeatureIconCatalog {
    pub(crate) idea_pictures: BTreeSet<String>,
    pub(crate) decision_icons: BTreeSet<String>,
    pub(crate) decision_category_pictures: BTreeSet<String>,
}

pub(crate) fn collect_feature_icon_catalog(
    mod_root: &Path,
    game_index: Option<&GameIndex>,
) -> Result<FeatureIconCatalog, String> {
    let mut catalog = FeatureIconCatalog::default();
    let interface_root = mod_root.join("interface");
    if interface_root.exists() {
        for file in collect_files(&interface_root)? {
            if file.extension().and_then(OsStr::to_str).unwrap_or("") != "gfx" {
                continue;
            }
            let text = read_utf8_lossy(&file)?;
            collect_idea_pictures(&text, &mut catalog.idea_pictures);
            collect_decision_icons(&text, &mut catalog.decision_icons);
            collect_decision_category_pictures(&text, &mut catalog.decision_category_pictures);
        }
    }
    if let Some(index) = game_index {
        catalog
            .idea_pictures
            .extend(index.idea_pictures.iter().cloned());
        catalog
            .decision_icons
            .extend(index.decision_icons.iter().cloned());
        catalog
            .decision_category_pictures
            .extend(index.decision_category_pictures.iter().cloned());
    }
    Ok(catalog)
}

pub(crate) fn resolve_idea_picture(card: &Card, catalog: &BTreeSet<String>) -> String {
    if let Some(explicit) = card.fields.get("图标").map(|value| value.trim_matches('"')) {
        let normalized = explicit.strip_prefix("GFX_idea_").unwrap_or(explicit);
        if is_reference_identifier(normalized) {
            return normalized.to_string();
        }
        let semantic_title = format!("{} {explicit}", card.title);
        return choose_idea_picture_from_catalog(&semantic_title, catalog)
            .unwrap_or_else(|| "generic_production_bonus".to_string());
    }
    choose_idea_picture_from_catalog(&card.title, catalog)
        .unwrap_or_else(|| "generic_production_bonus".to_string())
}

pub(crate) fn choose_idea_picture_from_catalog(
    title: &str,
    catalog: &BTreeSet<String>,
) -> Option<String> {
    let mut best: Option<(i32, bool, String)> = None;
    for picture in catalog {
        let score = semantic_reference_score(title, picture, 8);
        let country_match = semantic_reference_country_match(title, picture);
        if best
            .as_ref()
            .is_none_or(|(best_score, best_country_match, best_picture)| {
                score > *best_score
                    || (score == *best_score && country_match && !*best_country_match)
                    || (score == *best_score
                        && country_match == *best_country_match
                        && picture < best_picture)
            })
        {
            best = Some((score, country_match, picture.clone()));
        }
    }
    best.map(|(_, _, picture)| picture)
}

pub(crate) fn resolve_decision_icon(card: &Card, catalog: &BTreeSet<String>) -> String {
    if let Some(explicit) = card.fields.get("图标").map(|value| value.trim_matches('"')) {
        let normalized = explicit.strip_prefix("GFX_decision_").unwrap_or(explicit);
        if is_reference_identifier(normalized) && !normalized.starts_with("category_") {
            return normalized.to_string();
        }
        let semantic_title = format!("{} {explicit}", card.title);
        return choose_semantic_reference_from_catalog(&semantic_title, catalog)
            .unwrap_or_else(|| "generic_political_discourse".to_string());
    }
    choose_semantic_reference_from_catalog(&card.title, catalog)
        .unwrap_or_else(|| "generic_political_discourse".to_string())
}

pub(crate) fn resolve_decision_category_picture(
    card: &Card,
    category_title: &str,
    catalog: &BTreeSet<String>,
) -> String {
    if let Some(explicit) = card
        .fields
        .get("分类图片")
        .or_else(|| card.fields.get("分类图标"))
        .map(|value| value.trim_matches('"'))
    {
        if is_reference_identifier(explicit) {
            return explicit.to_string();
        }
    }
    let semantic_title = format!("{} {}", category_title, card.title);
    choose_semantic_reference_from_catalog(&semantic_title, catalog)
        .unwrap_or_else(|| "GFX_decision_category_generic_political_reform".to_string())
}

pub(crate) fn choose_semantic_reference_from_catalog(
    title: &str,
    catalog: &BTreeSet<String>,
) -> Option<String> {
    let mut best: Option<(i32, bool, String)> = None;
    for item in catalog {
        let score = semantic_reference_score(title, item, 8);
        let country_match = semantic_reference_country_match(title, item);
        if best
            .as_ref()
            .is_none_or(|(best_score, best_country_match, best_item)| {
                score > *best_score
                    || (score == *best_score && country_match && !*best_country_match)
                    || (score == *best_score
                        && country_match == *best_country_match
                        && item < best_item)
            })
        {
            best = Some((score, country_match, item.clone()));
        }
    }
    best.map(|(_, _, item)| item)
}

pub(crate) fn render_technology_inner_block(
    card: &Card,
    technology_id: &str,
    target: &str,
) -> String {
    let research_cost = card
        .fields
        .get("花费")
        .or_else(|| card.fields.get("研究花费"))
        .and_then(|s| parse_int(s))
        .unwrap_or(1)
        .max(1);
    let start_year = card
        .fields
        .get("年份")
        .or_else(|| card.fields.get("起始年份"))
        .and_then(|s| parse_int(s))
        .unwrap_or(1936);
    let folder = card
        .fields
        .get("文件夹")
        .or_else(|| card.fields.get("科技文件夹"))
        .map(|value| sanitize_identifier_part(value, "special_folder"))
        .unwrap_or_else(|| "special_forces_folder".to_string());
    let categories = technology_categories_from_card(card);
    let mut out = String::new();
    out.push_str(&format!("\t{technology_id} = {{\n"));
    out.push_str(&format!("\t\tresearch_cost = {research_cost}\n"));
    out.push_str(&format!("\t\tstart_year = {start_year}\n"));
    out.push_str("\t\tfolder = {\n");
    out.push_str(&format!("\t\t\tname = {folder}\n"));
    out.push_str("\t\t\tposition = { x = 0 y = 0 }\n");
    out.push_str("\t\t}\n");
    out.push_str("\t\tcategories = {\n");
    for category in categories {
        out.push_str(&format!("\t\t\t{category}\n"));
    }
    out.push_str("\t\t}\n");
    out.push_str(&format!("\t\t# target = {target}\n"));
    if let Some(effect) = card.fields.get("效果") {
        out.push_str(&format!("\t\t# effect note: {}\n", flatten_ws(effect)));
    }
    out.push_str("\t}\n");
    out
}

pub(crate) fn technology_categories_from_card(card: &Card) -> Vec<String> {
    let raw = card
        .fields
        .get("分类")
        .or_else(|| card.fields.get("类别"))
        .or_else(|| card.fields.get("科技分类"))
        .map(String::as_str)
        .unwrap_or("special_forces");
    let mut categories = split_cn_list(raw)
        .into_iter()
        .map(|value| sanitize_identifier_part(value, "special_forces"))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if categories.is_empty() {
        categories.push("special_forces".to_string());
    }
    categories.sort();
    categories.dedup();
    categories
}

pub(crate) fn render_scripted_gui_inner_block(card: &Card, gui_id: &str, target: &str) -> String {
    let window_name = format!("{gui_id}_window");
    let mut out = String::new();
    out.push_str(&format!("\t{gui_id} = {{\n"));
    out.push_str("\t\tcontext_type = country_context\n");
    out.push_str(&format!("\t\twindow_name = \"{window_name}\"\n"));
    out.push_str("\t\tvisible = {\n");
    out.push_str(&format!("\t\t\ttag = {target}\n"));
    out.push_str("\t\t}\n");
    out.push_str("\t\ttriggers = {\n");
    out.push_str("\t\t\talways = yes\n");
    out.push_str("\t\t}\n");
    out.push_str("\t\teffects = {\n");
    if let Some(effect) = card.fields.get("效果") {
        out.push_str(&format!("\t\t\t# TODO: {}\n", flatten_ws(effect)));
    }
    out.push_str("\t\t}\n");
    out.push_str("\t}\n");
    out
}

pub(crate) fn render_interface_gui_window_block(card: &Card, gui_id: &str) -> String {
    let window_name = format!("{gui_id}_window");
    let title_name = format!("{gui_id}_title");
    let body_name = format!("{gui_id}_body");
    let body_text = card
        .fields
        .get("用途")
        .or_else(|| card.fields.get("描述"))
        .map(String::as_str)
        .unwrap_or(&card.title);
    let mut out = String::new();
    out.push_str("\tcontainerWindowType = {\n");
    out.push_str(&format!("\t\tname = \"{window_name}\"\n"));
    out.push_str("\t\tposition = { x = 0 y = 0 }\n");
    out.push_str("\t\tsize = { width = 420 height = 180 }\n");
    out.push_str("\t\tmoveable = yes\n");
    out.push_str("\t\torientation = upper_left\n");
    out.push_str("\t\tinstantTextBoxType = {\n");
    out.push_str(&format!("\t\t\tname = \"{title_name}\"\n"));
    out.push_str("\t\t\tposition = { x = 16 y = 12 }\n");
    out.push_str("\t\t\tsize = { width = 388 height = 32 }\n");
    out.push_str(&format!("\t\t\ttext = \"{gui_id}\"\n"));
    out.push_str("\t\t\tfont = \"hoi_18mbs\"\n");
    out.push_str("\t\t}\n");
    out.push_str("\t\tinstantTextBoxType = {\n");
    out.push_str(&format!("\t\t\tname = \"{body_name}\"\n"));
    out.push_str("\t\t\tposition = { x = 16 y = 52 }\n");
    out.push_str("\t\t\tsize = { width = 388 height = 96 }\n");
    out.push_str(&format!(
        "\t\t\ttext = \"{}\"\n",
        localisation_value(body_text)
    ));
    out.push_str("\t\t\tfont = \"hoi_16mbs\"\n");
    out.push_str("\t\t}\n");
    out.push_str("\t}\n");
    out
}

pub(crate) fn render_scripted_effect_block(card: &Card, effect_id: &str) -> String {
    let scope = scripted_helper_scope(card);
    let effect_text =
        join_existing_fields(&card.fields, &["效果", "动作", "内容", "执行"]).unwrap_or_default();
    let suggestions = suggest_common("scripted_effect", &effect_text, None, None, None, None);
    let (effect_lines, effect_comments) = scripted_effect_lines(&suggestions, scope);
    let mut out = String::new();
    out.push_str(&format!("{effect_id} = {{\n"));
    out.push_str(&format!("\t# title = {}\n", flatten_ws(&card.title)));
    out.push_str(&format!("\t# scope = {scope}\n"));
    if effect_lines.is_empty() && effect_comments.is_empty() {
        if effect_text.trim().is_empty() {
            out.push_str("\t# TODO: add scripted effect body\n");
        } else {
            out.push_str(&format!("\t# TODO: {}\n", flatten_ws(&effect_text)));
        }
    }
    for comment in effect_comments {
        out.push_str(&format!("\t# {comment}\n"));
    }
    for line in effect_lines {
        out.push_str(&indent_lines(&line, "\t"));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_scripted_trigger_block(card: &Card, trigger_id: &str) -> String {
    let trigger_text = join_existing_fields(
        &card.fields,
        &["条件", "触发", "可用", "可见", "限制", "内容"],
    )
    .unwrap_or_default();
    let suggestions = split_cn_list(&trigger_text)
        .into_iter()
        .flat_map(suggest_trigger)
        .collect::<Vec<_>>();
    let (trigger_lines, trigger_comments) = trigger_lines_and_comments(&suggestions);
    let mut out = String::new();
    out.push_str(&format!("{trigger_id} = {{\n"));
    out.push_str(&format!("\t# title = {}\n", flatten_ws(&card.title)));
    if trigger_lines.is_empty() && trigger_comments.is_empty() {
        if trigger_text.trim().is_empty() {
            out.push_str("\t# TODO: add scripted trigger body\n");
        } else {
            out.push_str(&format!("\t# TODO: {}\n", flatten_ws(&trigger_text)));
        }
    }
    for comment in trigger_comments {
        out.push_str(&format!("\t# {comment}\n"));
    }
    for line in trigger_lines {
        out.push_str(&indent_lines(&line, "\t"));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_state_effect_block(card: &Card, effect_id: &str, target: &str) -> String {
    let state_target = state_card_target(card);
    let suggestions = state_card_suggestions(card, target);
    let (effect_lines, effect_comments) = state_effect_body_lines(&suggestions);
    let mut out = String::new();
    out.push_str(&format!("{effect_id} = {{\n"));
    out.push_str(&format!("\t# title = {}\n", flatten_ws(&card.title)));
    if let Some(state_id) = state_target.id {
        out.push_str(&format!("\t# state_id = {state_id}\n"));
        out.push_str(&format!("\t{state_id} = {{\n"));
        render_state_effect_body(
            &mut out,
            "\t\t",
            &effect_lines,
            &effect_comments,
            card,
            state_target.label.as_deref(),
        );
        out.push_str("\t}\n");
    } else {
        out.push_str("\t# scope = state\n");
        if let Some(label) = state_target.label.as_deref() {
            out.push_str(&format!("\t# state = {}\n", flatten_ws(label)));
        }
        render_state_effect_body(
            &mut out,
            "\t",
            &effect_lines,
            &effect_comments,
            card,
            state_target.label.as_deref(),
        );
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_state_effect_body(
    out: &mut String,
    indent: &str,
    effect_lines: &[String],
    effect_comments: &[String],
    card: &Card,
    state_label: Option<&str>,
) {
    if effect_lines.is_empty() && effect_comments.is_empty() {
        out.push_str(&format!(
            "{indent}# TODO: add state effects for {}\n",
            card.title
        ));
    }
    if let Some(label) = state_label {
        if !label.trim().is_empty() && parse_plain_i64(label).is_none() {
            out.push_str(&format!(
                "{indent}# TODO: resolve state name `{}` to a state id before using a fixed wrapper\n",
                flatten_ws(label)
            ));
        }
    }
    for comment in effect_comments {
        out.push_str(&format!("{indent}# {comment}\n"));
    }
    for line in effect_lines {
        out.push_str(&indent_lines(line, indent));
    }
}

#[derive(Default)]
pub(crate) struct StateCardTarget {
    pub(crate) id: Option<i64>,
    pub(crate) label: Option<String>,
}

pub(crate) fn state_card_target(card: &Card) -> StateCardTarget {
    let id = first_existing_field(
        &card.fields,
        &["州ID", "省份ID", "地区ID", "state_id", "State ID", "id"],
    )
    .and_then(parse_plain_i64)
    .or_else(|| parse_plain_i64(&card.title));
    let label = first_existing_field(
        &card.fields,
        &["州", "省份", "地区", "目标州", "state", "State"],
    )
    .map(str::to_string)
    .or_else(|| {
        if id.is_none() && !card.title.trim().is_empty() && !is_state_effect_title(&card.title) {
            Some(card.title.clone())
        } else {
            None
        }
    });
    StateCardTarget { id, label }
}

pub(crate) fn is_state_effect_title(value: &str) -> bool {
    value.contains("效果")
        || value.contains("修复")
        || value.contains("建设")
        || value.contains("资源")
        || value.contains("核心")
}

pub(crate) fn first_existing_field<'a>(
    fields: &'a BTreeMap<String, String>,
    keys: &[&str],
) -> Option<&'a str> {
    keys.iter()
        .filter_map(|key| fields.get(*key))
        .map(String::as_str)
        .find(|value| !value.trim().is_empty())
}

pub(crate) fn parse_plain_i64(value: &str) -> Option<i64> {
    value.trim().parse::<i64>().ok()
}

pub(crate) fn state_card_suggestions(card: &Card, target: &str) -> Vec<Suggestion> {
    let mut out = Vec::new();
    let effect_text = join_existing_fields(&card.fields, &["效果", "建筑", "建设", "资源", "内容"])
        .unwrap_or_default();
    out.extend(suggest_common(
        "state_effect",
        &effect_text,
        None,
        None,
        None,
        None,
    ));
    if let Some(core) = join_existing_fields(&card.fields, &["核心", "添加核心", "add_core_of"])
    {
        push_core_state_suggestions(&mut out, &core, target, false);
    }
    if let Some(core) = join_existing_fields(&card.fields, &["移除核心", "remove_core_of"]) {
        push_core_state_suggestions(&mut out, &core, target, true);
    }
    out
}

pub(crate) fn push_core_state_suggestions(
    out: &mut Vec<Suggestion>,
    text: &str,
    target: &str,
    remove: bool,
) {
    for raw in split_cn_list(text) {
        let tag = if raw.contains("目标") || raw.contains("本国") || raw.contains("该国") {
            Some(sanitize_identifier_part(target, "TAG").to_ascii_uppercase())
        } else {
            ascii_tag_from_text(raw)
        };
        if let Some(tag) = tag {
            let key = if remove {
                "remove_core_of"
            } else {
                "add_core_of"
            };
            out.push(Suggestion::new(
                "state_effect_candidate",
                &format!("{key} = {tag}"),
                raw,
                "Must run inside a state scope.",
            ));
        } else {
            out.push(Suggestion::new(
                "raw_effect",
                raw,
                raw,
                "Resolve the country tag before adding or removing state cores.",
            ));
        }
    }
}

pub(crate) fn ascii_tag_from_text(text: &str) -> Option<String> {
    let mut current = String::new();
    let mut best = None;
    for ch in text.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }
        if !current.is_empty() {
            let candidate = sanitize_identifier_part(&current, "").to_ascii_uppercase();
            if (2..=8).contains(&candidate.len()) {
                best = Some(candidate);
            }
            current.clear();
        }
    }
    best
}

pub(crate) fn state_effect_body_lines(suggestions: &[Suggestion]) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut comments = Vec::new();
    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "state_effect_candidate" => {
                if let Some(code) = materialize_state_effect(suggestion) {
                    lines.push(code);
                } else {
                    comments.push(format!(
                        "{} -> {} ({})",
                        suggestion.source, suggestion.code, suggestion.note
                    ));
                }
            }
            "country_effect" => comments.push(format!(
                "{} -> {} (country-scoped effect; do not place directly in a state helper)",
                suggestion.source, suggestion.code
            )),
            "country_effect_candidate" | "raw_effect" => comments.push(format!(
                "{} -> {} ({})",
                suggestion.source, suggestion.code, suggestion.note
            )),
            _ => {}
        }
    }
    (lines, comments)
}

pub(crate) fn scripted_helper_scope(card: &Card) -> &'static str {
    let scope = join_existing_fields(&card.fields, &["范围", "作用域", "scope", "Scope"])
        .unwrap_or_default()
        .to_ascii_lowercase();
    if scope.contains("state")
        || scope.contains("州")
        || scope.contains("省份")
        || scope.contains("地区")
    {
        "state"
    } else if scope.contains("character") || scope.contains("角色") {
        "character"
    } else {
        "country"
    }
}

pub(crate) fn scripted_effect_lines(
    suggestions: &[Suggestion],
    scope: &str,
) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut comments = Vec::new();
    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "country_effect" => {
                if let Some(code) = concrete_suggestion_code(suggestion) {
                    lines.push(code);
                }
            }
            "state_effect_candidate" => {
                if let Some(code) = materialize_state_effect(suggestion) {
                    if scope == "state" {
                        lines.push(code);
                    } else {
                        lines.push(format!(
                            "random_owned_controlled_state = {{\n\tlimit = {{ is_core_of = ROOT }}\n\t{code}\n}}"
                        ));
                    }
                } else {
                    comments.push(format!(
                        "{} -> {} ({})",
                        suggestion.source, suggestion.code, suggestion.note
                    ));
                }
            }
            "country_effect_candidate" | "raw_effect" => comments.push(format!(
                "{} -> {} ({})",
                suggestion.source, suggestion.code, suggestion.note
            )),
            _ => {}
        }
    }
    (lines, comments)
}

pub(crate) fn trigger_lines_and_comments(suggestions: &[Suggestion]) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut comments = Vec::new();
    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "trigger" => {
                if let Some(code) = concrete_suggestion_code(suggestion) {
                    lines.push(code);
                }
            }
            "trigger_candidate" | "raw_trigger" => comments.push(format!(
                "{} -> {} ({})",
                suggestion.source, suggestion.code, suggestion.note
            )),
            _ => {}
        }
    }
    (lines, comments)
}

#[derive(Clone)]
pub(crate) struct IdeaFileTarget {
    pub(crate) path: PathBuf,
    pub(crate) target_tag: String,
    pub(crate) score: usize,
    pub(crate) idea_count: usize,
}

pub(crate) fn scan_idea_file_targets(root: &Path) -> Result<Vec<IdeaFileTarget>, String> {
    let mut targets = Vec::new();
    let ideas_root = root.join("common").join("ideas");
    if !ideas_root.exists() {
        return Ok(targets);
    }
    for file in collect_files(&ideas_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let stem = file
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_string();
        if is_large_shared_idea_file_name(&stem) {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let country_wrappers = named_block_ranges(&text, "country");
        if country_wrappers.is_empty() {
            continue;
        }
        let idea_ids = country_wrappers
            .iter()
            .flat_map(|range| {
                direct_block_ranges(&range.content)
                    .into_iter()
                    .map(|(id, _)| id)
            })
            .collect::<Vec<_>>();
        let idea_count = idea_ids.len();
        for tag in candidate_tags_from_idea_file(&stem, &idea_ids) {
            let score = idea_file_target_score(&stem, &idea_ids, &tag);
            if score > 0 {
                targets.push(IdeaFileTarget {
                    path: file.clone(),
                    target_tag: tag,
                    score,
                    idea_count,
                });
            }
        }
    }
    Ok(targets)
}

pub(crate) fn select_idea_file_target<'a>(
    targets: &'a [IdeaFileTarget],
    target: &str,
) -> Option<&'a IdeaFileTarget> {
    let target = sanitize_identifier_part(target, "TAG").to_ascii_uppercase();
    targets
        .iter()
        .filter(|candidate| candidate.target_tag == target)
        .max_by_key(|candidate| (candidate.score, candidate.idea_count))
}

pub(crate) fn candidate_tags_from_idea_file(stem: &str, idea_ids: &[String]) -> BTreeSet<String> {
    let mut tags = BTreeSet::new();
    for token in stem.split(|ch: char| !ch.is_ascii_alphanumeric()) {
        let token = token.to_ascii_uppercase();
        if looks_like_tag(&token) {
            tags.insert(token);
        }
    }
    for id in idea_ids {
        let Some((prefix, _)) = id.split_once('_') else {
            continue;
        };
        let prefix = prefix.to_ascii_uppercase();
        if looks_like_tag(&prefix) {
            tags.insert(prefix);
        }
    }
    tags
}

pub(crate) fn idea_file_target_score(stem: &str, idea_ids: &[String], tag: &str) -> usize {
    let tag_lower = tag.to_ascii_lowercase();
    let stem_lower = stem.to_ascii_lowercase();
    let stem_tokens = stem_lower
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut score = 0usize;
    if stem_lower == tag_lower {
        score += 100;
    } else if stem_tokens.iter().any(|token| *token == tag_lower) {
        score += 80;
    } else if stem_lower.starts_with(&tag_lower) || stem_lower.ends_with(&tag_lower) {
        score += 50;
    } else if stem_lower.contains(&tag_lower) {
        score += 20;
    }
    let prefix = format!("{}_", tag.to_ascii_uppercase());
    let id_matches = idea_ids
        .iter()
        .filter(|id| id.to_ascii_uppercase().starts_with(&prefix))
        .count();
    score + id_matches.min(12) * 10
}

pub(crate) fn is_large_shared_idea_file_name(stem: &str) -> bool {
    let lower = stem.to_ascii_lowercase();
    lower.contains("minister")
        || lower.contains("advisor")
        || lower.contains("adviser")
        || lower.contains("character")
}

pub(crate) fn concrete_suggestion_lines(suggestions: &[Suggestion], kinds: &[&str]) -> Vec<String> {
    suggestions
        .iter()
        .filter(|suggestion| kinds.contains(&suggestion.kind.as_str()))
        .filter_map(concrete_suggestion_code)
        .collect()
}

pub(crate) fn decision_effect_lines(suggestions: &[Suggestion]) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::new();
    let mut comments = Vec::new();
    for suggestion in suggestions {
        match suggestion.kind.as_str() {
            "country_effect" => {
                if let Some(code) = concrete_suggestion_code(suggestion) {
                    lines.push(code);
                }
            }
            "state_effect_candidate" => {
                if let Some(code) = materialize_state_effect(suggestion) {
                    lines.push(format!(
                        "random_owned_controlled_state = {{\n\tlimit = {{ is_core_of = ROOT }}\n\t{code}\n}}"
                    ));
                } else {
                    comments.push(format!(
                        "{} -> {} ({})",
                        suggestion.source, suggestion.code, suggestion.note
                    ));
                }
            }
            "country_effect_candidate" | "raw_effect" => comments.push(format!(
                "{} -> {} ({})",
                suggestion.source, suggestion.code, suggestion.note
            )),
            _ => {}
        }
    }
    (lines, comments)
}

pub(crate) fn concrete_suggestion_code(suggestion: &Suggestion) -> Option<String> {
    if suggestion.code.contains('<') || suggestion.code.contains('>') {
        None
    } else {
        Some(suggestion.code.clone())
    }
}

pub(crate) fn materialize_state_effect(suggestion: &Suggestion) -> Option<String> {
    let level = parse_int(&suggestion.source).unwrap_or(1);
    let code = suggestion
        .code
        .replace("<number>", &level.max(1).to_string());
    concrete_suggestion_code(&Suggestion::new(
        &suggestion.kind,
        &code,
        &suggestion.source,
        &suggestion.note,
    ))
}

pub(crate) fn append_decisions_to_existing_categories(
    path: &Path,
    appends: &BTreeMap<String, Vec<(String, String)>>,
) -> Result<bool, String> {
    if appends.is_empty() {
        return Ok(false);
    }
    let mut text = read_utf8_lossy(path)?;
    let ranges = direct_block_ranges(&text);
    let mut insertions: Vec<(usize, String)> = Vec::new();
    for (category_id, blocks) in appends {
        let Some((_, range)) = ranges.iter().find(|(id, _)| id == category_id) else {
            return Err(format!(
                "{}: decision category {category_id} was not found for insertion",
                path.display()
            ));
        };
        let mut rendered = String::new();
        for (decision_id, block) in blocks {
            if text.contains(decision_id) {
                continue;
            }
            rendered.push('\n');
            rendered.push_str(block);
        }
        if !rendered.is_empty() {
            insertions.push((range.close, rendered));
        }
    }
    if insertions.is_empty() {
        return Ok(false);
    }
    insertions.sort_by(|a, b| b.0.cmp(&a.0));
    for (close, rendered) in insertions {
        let mut updated = String::new();
        updated.push_str(&text[..close]);
        if !text[..close].ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(&rendered);
        updated.push_str(&text[close..]);
        text = updated;
    }
    fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}

pub(crate) fn append_ideas_to_existing_country_wrapper(
    path: &Path,
    appends: &[(String, String)],
) -> Result<bool, String> {
    if appends.is_empty() {
        return Ok(false);
    }
    let text = read_utf8_lossy(path)?;
    let Some(range) = named_block_ranges(&text, "country").into_iter().next() else {
        return Err(format!(
            "{}: country idea wrapper was not found for insertion",
            path.display()
        ));
    };
    let mut rendered = String::new();
    for (idea_id, block) in appends {
        if text.contains(idea_id) {
            continue;
        }
        rendered.push('\n');
        rendered.push_str(block);
    }
    if rendered.is_empty() {
        return Ok(false);
    }
    let mut updated = String::new();
    let insert_at = insertion_before_closing_indent(&text, range.close);
    updated.push_str(&text[..insert_at]);
    if !text[..insert_at].ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&rendered);
    updated.push_str(&text[insert_at..]);
    fs::write(path, updated).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}

pub(crate) fn insertion_before_closing_indent(text: &str, close: usize) -> usize {
    let mut idx = close;
    while idx > 0 {
        let Some(ch) = text[..idx].chars().next_back() else {
            break;
        };
        if ch == ' ' || ch == '\t' {
            idx -= ch.len_utf8();
        } else {
            break;
        }
    }
    idx
}

pub(crate) fn append_blocks_to_named_wrapper(
    path: &Path,
    wrapper: &str,
    header: &str,
    blocks: &[(String, String)],
) -> Result<bool, String> {
    if blocks.is_empty() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = if path.exists() {
        read_utf8_lossy(path)?
    } else {
        format!("{header}{wrapper} = {{\n}}\n")
    };
    if named_block_ranges(&text, wrapper).is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("\n{wrapper} = {{\n}}\n"));
    }
    let Some(range) = named_block_ranges(&text, wrapper).into_iter().next() else {
        return Err(format!(
            "{}: wrapper {wrapper} was not found for insertion",
            path.display()
        ));
    };
    let mut rendered = String::new();
    for (key, block) in blocks {
        if text.contains(key) {
            continue;
        }
        rendered.push('\n');
        rendered.push_str(block);
    }
    if rendered.is_empty() {
        return Ok(false);
    }
    let insert_at = insertion_before_closing_indent(&text, range.close);
    let mut updated = String::new();
    updated.push_str(&text[..insert_at]);
    if !text[..insert_at].ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&rendered);
    updated.push_str(&text[insert_at..]);
    fs::write(path, updated).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}

pub(crate) fn append_unique_blocks(
    path: &Path,
    header: &str,
    blocks: &[(String, String)],
) -> Result<bool, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = if path.exists() {
        read_utf8_lossy(path)?
    } else {
        header.to_string()
    };
    let mut changed = false;
    for (key, block) in blocks {
        if text.contains(key) {
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
        text.push_str(block);
        changed = true;
    }
    if changed || !path.exists() {
        fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn append_localisation_entries(
    path: &Path,
    entries: &BTreeMap<String, String>,
) -> Result<bool, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let mut text = if path.exists() {
        let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        String::from_utf8_lossy(&bytes)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from("l_simp_chinese:\n")
    };
    if !text
        .lines()
        .any(|line| line.trim_start_matches('\u{feff}').trim() == "l_simp_chinese:")
    {
        text = format!("l_simp_chinese:\n{text}");
    }

    let mut existing = BTreeSet::new();
    collect_localisation_keys(&text, &mut existing);
    let mut changed = false;
    for (key, value) in entries {
        if existing.contains(key) {
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("  {key}:0 \"{}\"\n", localisation_value(value)));
        changed = true;
    }
    if changed || !path.exists() {
        fs::write(path, format!("\u{feff}{text}"))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub(crate) fn localisation_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\r', '\n'], " ")
}

pub(crate) fn indent_lines(text: &str, indent: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        out.push_str(indent);
        out.push_str(line);
        out.push('\n');
    }
    out
}
