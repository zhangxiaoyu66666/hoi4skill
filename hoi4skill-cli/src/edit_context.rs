//! Pre-edit context packaging for model-assisted HOI4 edits.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_prepare_edit_context(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_input = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("mod");
    let sheet = value(&map, "sheet");
    let tree_id = value(&map, "tree-id");
    let max_items = parse_usize_option(&map, "max-items", 80)?;
    let max_sprites = parse_usize_option(&map, "max-sprites", 400)?;
    let max_context_files = parse_usize_option(&map, "max-context-files", 24)?;
    let dependency_roots = dependency_mod_roots(&map)?;
    let game_root = value(&map, "game-root").map(normalize_path).transpose()?;
    if game_root.is_none() && !dependency_roots.is_empty() {
        return Err("--mod-path requires --game-root during edit-context preparation".to_string());
    }
    let game_index = game_root
        .as_ref()
        .map(|path| build_game_index_with_mod_paths(path, &dependency_roots))
        .transpose()?;
    let requested_library = value(&map, "code-library")
        .map(normalize_path)
        .transpose()?;
    let code_mod_roots = code_mod_roots(&map)?;
    if !code_mod_roots.is_empty() {
        let request = value(&map, "request").ok_or_else(|| {
            "--code-mod-path requires --request with the user's literal authorization".to_string()
        })?;
        enforce_mod_code_request(request, &code_mod_roots)?;
    }
    let code_libraries = game_root
        .as_ref()
        .map(|root| {
            ensure_clausewitz_libraries(root, &code_mod_roots, requested_library.as_deref())
        })
        .transpose()?
        .or_else(|| requested_library.map(|path| vec![path]));
    enforce_tag_request_contract(&map, tag, game_index.as_ref())?;

    let context = prepare_edit_context_markdown(
        &input,
        &mod_input,
        tag,
        prefix,
        sheet,
        tree_id,
        value(&map, "request"),
        &dependency_roots,
        game_index.as_ref(),
        max_items,
        max_sprites,
        max_context_files,
        code_libraries.as_deref(),
    )?;
    write_or_print(&context, value(&map, "output"))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_edit_context_markdown(
    input: &Path,
    mod_input: &Path,
    tag: &str,
    prefix: &str,
    sheet: Option<&str>,
    tree_id: Option<&str>,
    explicit_request: Option<&str>,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
    max_items: usize,
    max_sprites: usize,
    max_context_files: usize,
    code_libraries: Option<&[PathBuf]>,
) -> Result<String, String> {
    let resolved = resolve_mod_root(mod_input)?;
    let mut workflow_input = workflow_input_from_path(input, sheet, tag, prefix)?;
    append_explicit_request(&mut workflow_input, explicit_request);
    let request_text = &workflow_input.text;
    let knowledge_json = mod_knowledge_json(&resolved, max_items, max_sprites, dependency_roots)?;
    let context_validation_options = ValidationOptions {
        strict_code_index: game_index.is_some(),
    };
    let workflow_json = run_workflow_json_with_focus_layout_options(
        request_text,
        workflow_input.focus_layout.as_ref(),
        Some(&resolved.root),
        tag,
        prefix,
        tree_id,
        true,
        game_index,
        context_validation_options,
    )?;
    let markdown_summary = json_string_field(&knowledge_json, "markdown_summary")
        .unwrap_or_else(|| "mod_knowledge markdown_summary was not found".to_string());
    let anti_hallucination_rules =
        json_string_array_field(&knowledge_json, "anti_hallucination_rules");
    let unknown_facts =
        edit_context_unknown_facts(request_text, &knowledge_json, dependency_roots, game_index);
    let blocked = edit_context_blocked_until_verified(&unknown_facts, &workflow_json);
    let write_gate = edit_context_write_gate(
        request_text,
        workflow_input.focus_layout.is_some(),
        &knowledge_json,
        dependency_roots,
        game_index,
        &unknown_facts,
        &workflow_json,
    );
    let scope_contract = requirement_scope_contract(
        request_text,
        workflow_input.focus_layout.is_some(),
        tag,
        prefix,
    );
    let excerpts = edit_context_file_excerpts(&resolved.root, max_context_files)?;

    let mut out = String::new();
    out.push_str("# HOI4 Edit Context Pack\n\n");
    out.push_str("Use this as the first context block before generating or editing files. ");
    out.push_str("Do not write code from memory when a fact is missing here.\n\n");
    out.push_str("## Request\n\n");
    out.push_str(&format!("- input: `{}`\n", input.display()));
    out.push_str(&format!("- mod_root: `{}`\n", resolved.root.display()));
    out.push_str(&format!("- tag: `{tag}`\n- prefix: `{prefix}`\n"));
    if let Some(sheet) = sheet {
        out.push_str(&format!("- sheet: `{sheet}`\n"));
    }
    if let Some(tree_id) = tree_id {
        out.push_str(&format!("- tree_id: `{tree_id}`\n"));
    }
    out.push_str(&format!(
        "- dry_run_validation: `{}`\n",
        if context_validation_options.strict_code_index {
            "strict-code-index"
        } else {
            "local-static-only"
        }
    ));
    if dependency_roots.is_empty() {
        out.push_str("- dependency_mod_roots: none supplied\n");
    } else {
        out.push_str("- dependency_mod_roots:\n");
        for root in dependency_roots {
            out.push_str(&format!("  - `{}`\n", root.display()));
        }
    }
    if let Some(libraries) = code_libraries {
        out.push_str("- clausewitz_code_layers (highest priority first):\n");
        for (index, library) in libraries.iter().enumerate() {
            let kind = if index + 1 == libraries.len() {
                "vanilla_base"
            } else {
                "user_authorized_mod"
            };
            out.push_str(&format!("  - {kind}: `{}`\n", library.display()));
        }
    }
    out.push('\n');
    out.push_str(&markdown_fence(
        "text",
        truncate_chars(request_text, 18_000).as_str(),
    ));

    out.push_str("\n## Requirement Scope Contract\n\n");
    out.push_str("- rule: this section is the complete file-creation boundary; a new mod does not authorize unrelated systems.\n");
    out.push_str(&format!(
        "- authorized_systems: {}\n",
        list_or_none(&scope_contract.authorized_systems, 50)
    ));
    out.push_str(&format!(
        "- minimum_events: {}\n",
        scope_contract
            .minimum_events
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not requested".to_string())
    ));
    out.push_str(&format!(
        "- minimum_national_spirits: {}\n",
        scope_contract
            .minimum_ideas
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not requested".to_string())
    ));
    out.push_str("\n### Planned Files\n\n");
    push_markdown_list(&mut out, &scope_contract.planned_files);
    out.push_str("\n### Forbidden Without Explicit Request\n\n");
    push_markdown_list(&mut out, &scope_contract.forbidden_without_explicit_request);
    out.push_str("\n### Scope Rules\n\n");
    push_markdown_list(&mut out, &scope_contract.rules);

    out.push_str("\n## AI Authoring Contract\n\n");
    out.push_str("- Treat player-facing prose as intent, not as Clausewitz code.\n");
    out.push_str("- Convert shorthand such as `llm：战争正当化 = -10%` with `hoi4skill compile-intent --kind auto --game-root <HOI4 root> --strict-code-index` before any final script output.\n");
    out.push_str("- Use only effects, triggers, modifiers, buildings, resources, sprites, technologies, tags, and IDs that appear in this context pack, local excerpts, or `check-code-symbol`/`code-catalog` results.\n");
    out.push_str("- If `compile-intent`, `check-code-symbol`, dry-run safety, or validation says `ok: false` / `final_code_allowed: false`, stop and fix the structured input; do not handwrite fallback Clausewitz.\n");
    out.push_str("- Final generated files must pass `hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index` before being treated as usable.\n");

    out.push_str("\n## Write Gate\n\n");
    out.push_str(&format!("- status: `{}`\n", write_gate.status));
    out.push_str("- rule: if the status is not `READY_FOR_NARROW_WRITE`, resolve the missing evidence before writing final game script.\n");
    out.push_str("- rule: write only inside the allowed edit surface and only for IDs/paths shown in the dry-run plan or verified local files.\n\n");
    out.push_str("### Verified Evidence\n\n");
    push_markdown_list(&mut out, &write_gate.verified_evidence);
    out.push_str("\n### Allowed Edit Surface\n\n");
    push_markdown_list(&mut out, &write_gate.allowed_edit_surface);
    out.push_str("\n### Missing Evidence To Resolve\n\n");
    push_markdown_list(&mut out, &write_gate.missing_evidence);
    out.push_str("\n### Verification Steps\n\n");
    push_markdown_list(&mut out, &write_gate.verification_steps);
    out.push_str("\n### Stop Conditions\n\n");
    push_markdown_list(&mut out, &write_gate.stop_conditions);

    out.push_str("\n## Knowledge Summary\n\n");
    out.push_str(&markdown_summary);
    if !markdown_summary.ends_with('\n') {
        out.push('\n');
    }

    if let Some(index) = game_index {
        out.push_str("\n## Indexed Game/Dependency Resources\n\n");
        out.push_str(&render_indexed_resource_summary(index, 40));
        out.push_str("\n## Clausewitz Syntax Reference Table\n\n");
        out.push_str(&render_clausewitz_reference_table(Some(index)));
    }

    if let Some(libraries) = code_libraries {
        out.push_str("\n## Retrieved Clausewitz Code Library\n\n");
        out.push_str("- rule: read these verified local examples before producing structured inputs or changing a generator.\n");
        out.push_str("- rule: copy syntax and block ownership only; never copy IDs, country-specific narrative, or unrelated effects.\n");
        out.push_str(&render_retrieved_clausewitz_context(
            libraries,
            request_text,
            &scope_contract.authorized_systems,
        )?);
    }

    out.push_str("\n## Dry Run Plan\n\n");
    out.push_str("This is a non-writing `run-workflow` plan with validation against the target mod root. When a game/dependency index is available, this dry run uses strict code-index validation so the model sees final-gate failures before writing.\n\n");
    out.push_str(&markdown_fence(
        "json",
        truncate_chars(&workflow_json, 60_000).as_str(),
    ));

    out.push_str("\n## Anti-Hallucination Rules\n\n");
    if anti_hallucination_rules.is_empty() {
        out.push_str("- Use only facts from the knowledge summary, local excerpts, explicit user input, or an indexed game/dependency root.\n");
        out.push_str("- Missing facts are unknown; verify them before editing.\n");
    } else {
        for rule in anti_hallucination_rules {
            out.push_str(&format!("- {rule}\n"));
        }
    }

    out.push_str("\n## Unknown Facts\n\n");
    for fact in &unknown_facts {
        out.push_str(&format!("- {fact}\n"));
    }

    out.push_str("\n## Blocked Until Verified\n\n");
    for item in &blocked {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Local File Excerpts\n\n");
    if excerpts.is_empty() {
        out.push_str("- No local content excerpts were selected.\n");
    } else {
        for excerpt in excerpts {
            out.push_str(&format!("### `{}`\n\n", excerpt.path));
            out.push_str(&markdown_fence(
                "text",
                truncate_chars(&excerpt.text, 12_000).as_str(),
            ));
            out.push('\n');
        }
    }

    out.push_str("\n## Safe Next Step\n\n");
    out.push_str("- If every `Blocked Until Verified` item is resolved, rerun `run-workflow` without `--dry-run` or use the narrow `apply-*` command.\n");
    out.push_str("- If any blocked item remains, read/index the missing files first or ask the user for explicit IDs/roots.\n");
    out.push_str("- After writes, run `hoi4skill validate <mod-root> --game-root <HOI4 root> --strict-code-index --request \"<literal user request>\"` and then check HOI4 `error.log` from an in-game launch.\n");
    Ok(out)
}

pub(crate) fn render_indexed_resource_summary(index: &GameIndex, limit: usize) -> String {
    let mut out = String::new();
    out.push_str("- rule: only use these indexed resources or local `interface/*.gfx` evidence; missing resources are unknown.\n");
    out.push_str(&format!(
        "- country_tags: {} total; sample: {}\n",
        index.country_tags.len(),
        sample_btree_strings(&index.country_tags, limit)
    ));
    out.push_str(&format!(
        "- ideologies: {} total; sample: {}\n",
        index.ideologies.len(),
        sample_btree_strings(&index.ideologies, limit)
    ));
    out.push_str(&format!(
        "- focus_goal_sprites: {} total; sample: {}\n",
        index.focus_goal_sprites.len(),
        sample_btree_strings(&index.focus_goal_sprites, limit)
    ));
    out.push_str(&format!(
        "- idea_pictures: {} total; sample: {}\n",
        index.idea_pictures.len(),
        sample_btree_strings(&index.idea_pictures, limit)
    ));
    out.push_str(&format!(
        "- event_pictures: {} total; sample: {}\n",
        index.event_pictures.len(),
        sample_btree_strings(&index.event_pictures, limit)
    ));
    out.push_str(&format!(
        "- decision_icons: {} total; sample: {}\n",
        index.decision_icons.len(),
        sample_btree_strings(&index.decision_icons, limit)
    ));
    out.push_str(&format!(
        "- decision_category_pictures: {} total; sample: {}\n",
        index.decision_category_pictures.len(),
        sample_btree_strings(&index.decision_category_pictures, limit)
    ));
    out.push_str(&format!(
        "- leader_portraits: {} total; sample: {}\n",
        index.leader_portraits.len(),
        sample_btree_strings(&index.leader_portraits, limit)
    ));
    out.push_str(&format!(
        "- effects: {} total; sample: {}\n",
        index.effects.len(),
        sample_btree_strings(&index.effects, limit)
    ));
    out.push_str(&format!(
        "- triggers: {} total; sample: {}\n",
        index.triggers.len(),
        sample_btree_strings(&index.triggers, limit)
    ));
    out.push_str(&format!(
        "- modifiers: {} total; sample: {}\n",
        index.modifiers.len(),
        sample_btree_strings(&index.modifiers, limit)
    ));
    out
}

fn sample_btree_strings(values: &BTreeSet<String>, limit: usize) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone)]
pub(crate) struct EditContextExcerpt {
    pub(crate) path: String,
    pub(crate) text: String,
}

pub(crate) struct EditContextWriteGate {
    pub(crate) status: &'static str,
    pub(crate) verified_evidence: Vec<String>,
    pub(crate) allowed_edit_surface: Vec<String>,
    pub(crate) missing_evidence: Vec<String>,
    pub(crate) verification_steps: Vec<String>,
    pub(crate) stop_conditions: Vec<String>,
}

pub(crate) fn edit_context_file_excerpts(
    root: &Path,
    max_context_files: usize,
) -> Result<Vec<EditContextExcerpt>, String> {
    let files = collect_files(root)?;
    let sample = sample_content_files(root, &files, max_context_files);
    let mut out = Vec::new();
    for rel in sample {
        let path = root.join(rel.replace('/', "\\"));
        if !path.exists() || !path.is_file() {
            continue;
        }
        let Ok(text) = read_utf8_lossy(&path) else {
            continue;
        };
        out.push(EditContextExcerpt { path: rel, text });
    }
    Ok(out)
}

pub(crate) fn edit_context_write_gate(
    request_text: &str,
    supplied_focus_layout: bool,
    knowledge_json: &str,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
    unknown_facts: &[String],
    workflow_json: &str,
) -> EditContextWriteGate {
    let focus_text = extract_focus_layout_text(request_text);
    let feature_text = extract_card_text(request_text, FEATURE_CARD_HEADERS);
    let event_text = extract_card_text(request_text, &["事件"]);
    let feature_cards = parse_cards(&feature_text, FEATURE_CARD_HEADERS);
    let event_cards = parse_cards(&event_text, &["事件"]);
    let has_focus_layout = supplied_focus_layout || !focus_text.trim().is_empty();
    let scope_contract =
        requirement_scope_contract(request_text, has_focus_layout, "TAG", "feature");
    let scope_wants_ideas = scope_contract
        .authorized_systems
        .iter()
        .any(|system| system == "national_spirits");
    let scope_wants_events = scope_contract
        .authorized_systems
        .iter()
        .any(|system| system == "events");
    let detected_sections = usize::from(has_focus_layout)
        + usize::from(scope_wants_ideas || !feature_cards.is_empty())
        + usize::from(scope_wants_events || !event_cards.is_empty());
    let is_submod = knowledge_json.contains("\"kind\": \"submod\"");
    let unknown_descriptor = knowledge_json.contains("\"kind\": \"unknown_no_descriptor\"");

    let mut verified_evidence = Vec::new();
    if unknown_descriptor {
        verified_evidence
            .push("mod root is not confirmed; descriptor.mod was not observed".to_string());
    } else {
        verified_evidence.push(
            "mod root was resolved and mod-knowledge generated descriptor/local-file evidence"
                .to_string(),
        );
    }
    if is_submod {
        if dependency_roots.is_empty() {
            verified_evidence
                .push("target is a submod, but no dependency roots were indexed".to_string());
        } else {
            verified_evidence.push(format!(
                "target is a submod and {} dependency root(s) were supplied",
                dependency_roots.len()
            ));
        }
    } else {
        verified_evidence
            .push("target is not classified as a dependency-backed submod".to_string());
    }
    if game_index.is_some() {
        verified_evidence.push(
            "game/dependency index is available for tags, sprites, leader portraits, states, provinces, technologies, and symbols"
                .to_string(),
        );
    } else {
        verified_evidence.push(
            "no game/dependency index is available; only local mod facts are verified".to_string(),
        );
    }
    verified_evidence.push(format!(
        "request parsed as focus_layout={}, feature_cards={}, event_cards={}",
        has_focus_layout,
        feature_cards.len(),
        event_cards.len()
    ));
    verified_evidence.push(format!(
        "dry-run validation status is {}",
        workflow_validation_status(workflow_json)
    ));
    verified_evidence.push(format!(
        "dry-run safety status is {}",
        workflow_safety_status(workflow_json)
    ));

    let mut allowed_edit_surface = Vec::new();
    if has_focus_layout {
        allowed_edit_surface.push(
            "common/national_focus and localisation/simp_chinese for the target focus tree only"
                .to_string(),
        );
    }
    for card in &feature_cards {
        match feature_card_type(&card.kind).unwrap_or("") {
            "decision" => allowed_edit_surface.push(
                "common/decisions, common/decisions/categories, and localisation for parsed decision cards"
                    .to_string(),
            ),
            "idea" => allowed_edit_surface
                .push("common/ideas and localisation for parsed national-spirit cards".to_string()),
            "technology" => allowed_edit_surface.push(
                "common/technologies and localisation for parsed technology cards, after indexed reference checks"
                    .to_string(),
            ),
            "gui" => allowed_edit_surface.push(
                "common/scripted_guis, interface/*.gui, and localisation for conservative GUI skeletons"
                    .to_string(),
            ),
            "scripted_effect" => allowed_edit_surface
                .push("common/scripted_effects for parsed scripted-effect helper cards".to_string()),
            "scripted_trigger" => allowed_edit_surface.push(
                "common/scripted_triggers for parsed scripted-trigger helper cards".to_string(),
            ),
            "state_effect" => allowed_edit_surface.push(
                "common/scripted_effects state-scope helpers only; no direct history/states writes without plan-history-edit"
                    .to_string(),
            ),
            _ => {}
        }
    }
    if scope_wants_ideas
        && !allowed_edit_surface
            .iter()
            .any(|surface| surface.starts_with("common/ideas"))
    {
        allowed_edit_surface.push(
            "common/ideas and Simplified Chinese localisation for explicitly requested national spirits only"
                .to_string(),
        );
    }
    if scope_wants_events || !event_cards.is_empty() {
        allowed_edit_surface.push(
            "events and Simplified Chinese localisation for explicitly requested events and verified namespaces only"
                .to_string(),
        );
    }
    if allowed_edit_surface.is_empty() {
        allowed_edit_surface.push(
            "no file writes; convert the request into a parseable focus/card/event plan first"
                .to_string(),
        );
    }
    allowed_edit_surface.push(
        "preserve every file and setting outside the dry-run plan and verified local evidence"
            .to_string(),
    );
    allowed_edit_surface.sort();
    allowed_edit_surface.dedup();

    let mut missing_evidence = unknown_facts
        .iter()
        .filter(|fact| !fact.starts_with("no obvious missing high-risk facts"))
        .cloned()
        .collect::<Vec<_>>();
    if workflow_json.contains("\"ok\": false") {
        missing_evidence
            .push("dry-run validation is not clean; review validation errors/warnings".to_string());
    }
    if workflow_json.contains("\"final_code_allowed\": false") {
        missing_evidence.push(
            "dry-run safety blocks final code; map every raw effect/trigger or placeholder through verified CLI output"
                .to_string(),
        );
    }
    if detected_sections == 0 {
        missing_evidence.push(
            "request was not parsed into focus layout, feature cards, or event cards".to_string(),
        );
    }
    if missing_evidence.is_empty() {
        missing_evidence
            .push("none detected by preflight; still treat absent facts as unknown".to_string());
    }
    missing_evidence.sort();
    missing_evidence.dedup();

    let mut verification_steps = edit_context_verification_steps(&missing_evidence);
    if verification_steps.is_empty() {
        verification_steps.push(
            "rerun `run-workflow --dry-run` after any context change and compare the plan before writing"
                .to_string(),
        );
    }

    let mut stop_conditions = vec![
        "stop if a needed tag, state/province ID, technology, modifier, sprite, namespace, file path, or leader syntax is absent from the context pack".to_string(),
        "stop if the dry-run plan does not mention the system you intend to edit".to_string(),
        "stop if validation reports errors or unreviewed warnings".to_string(),
    ];
    if unknown_descriptor {
        stop_conditions
            .push("stop until the real mod root or launcher .mod file is confirmed".to_string());
    }
    if is_submod && dependency_roots.is_empty() {
        stop_conditions.push(
            "stop before using inherited dependency content until --mod-path roots are indexed"
                .to_string(),
        );
    }

    let hard_blocked = unknown_descriptor
        || detected_sections == 0
        || workflow_json.contains("\"status\": \"errors\"")
        || workflow_json.contains("\"final_code_allowed\": false");
    let status = if hard_blocked {
        "BLOCKED"
    } else if missing_evidence
        .iter()
        .any(|fact| !fact.starts_with("none detected by preflight"))
    {
        "VERIFY_FIRST"
    } else {
        "READY_FOR_NARROW_WRITE"
    };

    EditContextWriteGate {
        status,
        verified_evidence,
        allowed_edit_surface,
        missing_evidence,
        verification_steps,
        stop_conditions,
    }
}

pub(crate) fn edit_context_unknown_facts(
    request_text: &str,
    knowledge_json: &str,
    dependency_roots: &[PathBuf],
    game_index: Option<&GameIndex>,
) -> Vec<String> {
    let mut facts = Vec::new();
    let lower = request_text.to_ascii_lowercase();
    let mentions_history = contains_any(
        request_text,
        &[
            "history/states",
            "history/countries",
            "州",
            "省份",
            "province",
            "state id",
            "州ID",
            "STATE_",
            "首都",
            "capital",
            "胜利点",
            "资源",
            "建筑",
        ],
    );
    let mentions_icons = contains_any(request_text, &["图标", "icon", "gfx", "dds", "png"]);
    let mentions_country_or_leader = contains_any(
        request_text,
        &[
            "创建国家",
            "国家tag",
            "国家TAG",
            "领袖",
            "将领",
            "顾问",
            "country_leader",
            "character",
        ],
    );
    let is_submod = knowledge_json.contains("\"kind\": \"submod\"");
    let unknown_descriptor = knowledge_json.contains("\"kind\": \"unknown_no_descriptor\"");
    let no_dependency_roots = is_submod
        && dependency_roots.is_empty()
        && knowledge_json.contains("\"dependency_mod_roots\": []");

    if unknown_descriptor {
        facts.push(
            "target mod root is not confirmed because descriptor.mod was not found".to_string(),
        );
    }
    if no_dependency_roots {
        facts.push("submod dependencies are named but no dependency --mod-path roots were supplied, so inherited tags/sprites/scripts/technologies/state facts remain unknown".to_string());
    }
    if mentions_history {
        facts.push("history/state/province/capital facts require `plan-history-edit`, indexed game/dependency roots, or explicit user-provided IDs before direct history writes".to_string());
    }
    if mentions_icons && game_index.is_none() {
        facts.push("game/dependency icon index was not built; focus icons, idea pictures, decision icons, decision category pictures, event pictures, and leader portraits may use only locally observed registrations, with `GFX_goal_unknown` as the focus fallback".to_string());
    }
    if mentions_country_or_leader && no_dependency_roots {
        facts.push("country/leader syntax for dependency-provided content is unknown until dependency roots are indexed".to_string());
    }
    if (lower.contains("technology") || request_text.contains("科技")) && game_index.is_none() {
        facts.push("technology, category, equipment, and modifier references are not checked against a game index".to_string());
    }
    if facts.is_empty() {
        facts.push("no obvious missing high-risk facts were detected; still treat facts absent from the knowledge summary as unknown".to_string());
    }
    facts.sort();
    facts.dedup();
    facts
}

fn edit_context_verification_steps(missing_evidence: &[String]) -> Vec<String> {
    let mut steps = Vec::new();
    for fact in missing_evidence {
        if fact.contains("history/state/province/capital") {
            steps.push("run `hoi4skill plan-history-edit <mod-root> --text <request> --game-root <hoi4-root> [--mod-path <dependency>]` before direct history writes".to_string());
        } else if fact.contains("submod dependencies") || fact.contains("country/leader syntax") {
            steps.push("rerun `prepare-edit-context` with each dependency launcher/root supplied through `--mod-path`".to_string());
        } else if fact.contains("icon index") {
            steps.push("supply `--game-root` or verify exact focus/idea/decision/event/leader portrait sprite registrations in local/dependency `interface/*.gfx`; ideas register `GFX_idea_*` but idea blocks must omit the `GFX_idea_` prefix".to_string());
        } else if fact.contains("technology") {
            steps.push("supply `--game-root` so technologies, categories, equipment, and modifiers are checked against an index".to_string());
        } else if fact.contains("descriptor.mod") {
            steps.push("rerun against the real mod directory, descriptor.mod, or launcher-side `.mod` file".to_string());
        } else if fact.contains("dry-run validation") {
            steps.push("read the `Dry Run Plan` validation errors/warnings and fix or explicitly accept each warning before writing".to_string());
        } else if fact.contains("dry-run safety blocks") {
            steps.push("use `hoi4skill compile-intent --kind auto --game-root <HOI4 root> --strict-code-index` or `check-code-symbol` to replace every raw effect/trigger or placeholder with verified structured input".to_string());
        } else if fact.contains("not parsed") {
            steps.push("rewrite the input as a focus layout, feature card, or event card, then regenerate the context pack".to_string());
        }
    }
    steps.sort();
    steps.dedup();
    steps
}

pub(crate) fn edit_context_blocked_until_verified(
    unknown_facts: &[String],
    workflow_json: &str,
) -> Vec<String> {
    let mut blocked = Vec::new();
    for fact in unknown_facts {
        if fact.contains("history/state/province/capital") {
            blocked.push("Do not edit `history/states` or `history/countries` directly until `plan-history-edit` says the IDs are known.".to_string());
        } else if fact.contains("submod dependencies") {
            blocked.push("Do not reference inherited dependency tags, sprites, scripted values, technologies, or state/province IDs until dependency roots are indexed.".to_string());
        } else if fact.contains("icon index") {
            blocked.push("Do not invent focus, idea, decision-category, event, or leader portrait sprite names; use verified local/indexed registrations, reference ideas without the `GFX_idea_` prefix, or use `GFX_goal_unknown` for an unresolved focus icon.".to_string());
        } else if fact.contains("technology") {
            blocked.push("Do not use unindexed technology/equipment/category/modifier IDs as confirmed facts.".to_string());
        } else if fact.contains("descriptor.mod") {
            blocked.push(
                "Do not edit until the real mod root or launcher `.mod` file is confirmed."
                    .to_string(),
            );
        }
    }
    if workflow_json.contains("\"validation\": {\"ran\": true, \"ok\": false") {
        blocked.push("Do not report success until dry-run validation errors/warnings are reviewed and resolved.".to_string());
    }
    if workflow_json.contains("\"final_code_allowed\": false") {
        blocked.push("Do not write final Clausewitz until dry-run safety allows final code; unresolved raw effects/triggers and placeholders must be mapped first.".to_string());
    }
    if workflow_json.contains(
        "\"detected\": {\"focus_layout\": false, \"feature_cards\": 0, \"event_cards\": 0}",
    ) {
        blocked.push("Do not write files yet; the request was not parsed into a focus layout, feature card, or event card plan.".to_string());
    }
    if blocked.is_empty() {
        blocked.push("No hard blocker detected; write only the files shown by the plan and preserve unrelated content.".to_string());
    }
    blocked.sort();
    blocked.dedup();
    blocked
}

fn workflow_validation_status(workflow_json: &str) -> &'static str {
    if workflow_json.contains("\"status\": \"errors\"") {
        "errors"
    } else if workflow_json.contains("\"status\": \"warnings\"") {
        "warnings"
    } else if workflow_json.contains("\"status\": \"ok\"") {
        "ok"
    } else if workflow_json.contains("\"ran\": false") {
        "not_run"
    } else {
        "unknown"
    }
}

fn workflow_safety_status(workflow_json: &str) -> &'static str {
    if workflow_json.contains("\"final_code_allowed\": false") {
        "blocked"
    } else if workflow_json.contains("\"final_code_allowed\": true") {
        "allows_final_code"
    } else {
        "unknown"
    }
}

fn push_markdown_list(out: &mut String, items: &[String]) {
    if items.is_empty() {
        out.push_str("- none\n");
    } else {
        for item in items {
            out.push_str(&format!("- {item}\n"));
        }
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| {
        let needle_lower = needle.to_ascii_lowercase();
        lower.contains(&needle_lower) || text.contains(needle)
    })
}

pub(crate) fn json_string_field(json: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let start = json.find(&key)? + key.len();
    let after_colon = json[start..].find(':')? + start + 1;
    let mut offset = after_colon;
    offset = skip_json_whitespace(json, offset)?;
    parse_json_string_at(json, offset).map(|(value, _)| value)
}

pub(crate) fn json_string_array_field(json: &str, field: &str) -> Vec<String> {
    let key = format!("\"{field}\"");
    let Some(start) = json.find(&key).map(|idx| idx + key.len()) else {
        return Vec::new();
    };
    let Some(after_colon) = json[start..].find(':').map(|idx| idx + start + 1) else {
        return Vec::new();
    };
    let mut offset = after_colon;
    let Some(next_offset) = skip_json_whitespace(json, offset) else {
        return Vec::new();
    };
    offset = next_offset;
    if !json[offset..].starts_with('[') {
        return Vec::new();
    }
    offset += 1;
    let mut out = Vec::new();
    loop {
        let Some(next_offset) = skip_json_whitespace(json, offset) else {
            break;
        };
        offset = next_offset;
        if json[offset..].starts_with(']') {
            break;
        }
        let Some((value, next)) = parse_json_string_at(json, offset) else {
            break;
        };
        out.push(value);
        offset = next;
        while json[offset..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == ',')
        {
            offset += json[offset..].chars().next().unwrap().len_utf8();
        }
    }
    out
}

fn parse_json_string_at(json: &str, start: usize) -> Option<(String, usize)> {
    if !json[start..].starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut offset = start + 1;
    while offset < json.len() {
        let ch = json[offset..].chars().next()?;
        offset += ch.len_utf8();
        match ch {
            '"' => return Some((out, offset)),
            '\\' => {
                let esc = json[offset..].chars().next()?;
                offset += esc.len_utf8();
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'b' => out.push('\u{0008}'),
                    'f' => out.push('\u{000c}'),
                    'u' => {
                        let hex = json.get(offset..offset + 4)?;
                        let code = u16::from_str_radix(hex, 16).ok()?;
                        let decoded = char::from_u32(code as u32)?;
                        out.push(decoded);
                        offset += 4;
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    None
}

fn skip_json_whitespace(json: &str, mut offset: usize) -> Option<usize> {
    while offset < json.len() {
        let ch = json[offset..].chars().next()?;
        if !ch.is_whitespace() {
            break;
        }
        offset += ch.len_utf8();
    }
    Some(offset)
}

pub(crate) fn markdown_fence(info: &str, text: &str) -> String {
    format!("````{info}\n{text}\n````\n")
}

pub(crate) fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in text.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("\n... <truncated> ...");
            return out;
        }
        out.push(ch);
    }
    out
}
