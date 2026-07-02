//! P28 ambiguity and user-confirmation gate.
//!
//! This layer does not write Clausewitz. It turns unresolved natural-language
//! references into explicit questions so weak model output cannot silently pick
//! the wrong tag, cosmetic alias, icon, flag, or map target.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
struct AmbiguityCandidate {
    id: String,
    source_kind: String,
    source_file: String,
    risk: String,
}

#[derive(Clone)]
struct AmbiguityQuestion {
    id: String,
    kind: String,
    prompt: String,
    candidates: Vec<AmbiguityCandidate>,
    risk: String,
}

pub(crate) fn cmd_ambiguity_report(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = require_value(&map, "text")?;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let mod_root = value(&map, "mod-root").map(normalize_path).transpose()?;
    let dependency_roots =
        dependency_mod_roots_for_optional_edited_mod(&map, mod_root.as_deref(), true)?;
    let index = build_game_index_with_mod_paths(&game_root, &dependency_roots)?;
    let mut roots = vec![game_root.clone()];
    roots.extend(dependency_roots.iter().cloned());
    if let Some(root) = &mod_root {
        roots.push(root.clone());
    }

    let questions = build_ambiguity_questions(&text, &index, &roots);
    let ok = questions.is_empty();
    let json = ambiguity_report_json(&game_root, mod_root.as_deref(), &text, &questions);
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(questions
            .iter()
            .map(|question| question.prompt.clone())
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_answer_ambiguity(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let answers = normalize_path(&require_value(&map, "answers")?)?;
    let report = read_utf8_lossy(&input)?;
    let answers_text = read_utf8_lossy(&answers)?;
    if !report.contains("\"schema\": \"hoi4skill.ambiguity_report.v1\"") {
        return Err("input is not an ambiguity-report JSON".to_string());
    }
    let question_ids = json_question_ids(&report);
    let unresolved = question_ids
        .iter()
        .filter(|id| !answers_text.contains(&format!("\"{id}\"")))
        .cloned()
        .collect::<Vec<_>>();
    let ok = unresolved.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"answers\": {},\n  \"question_ids\": {},\n  \"unresolved_question_ids\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.resolved_intent.v1"),
        json_bool(ok),
        json_str(if ok { "ambiguity_resolved" } else { "questions_unanswered" }),
        json_str(&input.display().to_string()),
        json_str(&answers.display().to_string()),
        json_array(&question_ids),
        json_array(&unresolved),
        json_str("every ambiguity-report question id must be answered explicitly before any writer receives the intent")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(format!(
            "unanswered ambiguity question id(s): {}",
            unresolved.join(", ")
        ));
    }
    Ok(())
}

pub(crate) fn cmd_ambiguity_gate(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let mut blockers = Vec::new();
    if !text.contains("\"schema\": \"hoi4skill.resolved_intent.v1\"") {
        blockers.push("input is not an answer-ambiguity resolved intent".to_string());
    }
    if !text.contains("\"ok\": true") {
        blockers.push("resolved intent still has unanswered ambiguity questions".to_string());
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.ambiguity_gate.v1"),
        json_bool(ok),
        json_str(if ok { "ambiguity_gate_passed" } else { "blocked" }),
        json_str(&input.display().to_string()),
        json_array(&blockers),
        json_str("ambiguous tag, cosmetic, icon, flag, unit, and map references must be resolved before Rust writers assemble code")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn build_ambiguity_questions(
    text: &str,
    index: &GameIndex,
    roots: &[PathBuf],
) -> Vec<AmbiguityQuestion> {
    let mut questions = Vec::new();
    let placeholders = bracket_placeholders(text);
    for placeholder in &placeholders {
        let normalized = ambiguity_normalize(placeholder);
        if normalized.is_empty()
            || placeholder.starts_with("红色:")
            || placeholder.starts_with("红色：")
        {
            continue;
        }
        if placeholder.contains("图标") || placeholder.to_ascii_lowercase().contains("icon") {
            push_icon_question(&mut questions, placeholder, index);
            continue;
        }
        if placeholder.contains("国旗")
            || placeholder.contains("旗子")
            || placeholder.contains("flag")
        {
            push_flag_question(&mut questions, placeholder, index, roots);
            continue;
        }
        if placeholder.contains("领导人") {
            push_country_alias_question(&mut questions, placeholder, index, true);
            continue;
        }
        push_country_alias_question(&mut questions, placeholder, index, false);
    }

    push_text_alias_questions(&mut questions, text, index);
    push_place_questions(&mut questions, text, index);
    dedup_ambiguity_questions(questions)
}

fn push_icon_question(
    questions: &mut Vec<AmbiguityQuestion>,
    placeholder: &str,
    index: &GameIndex,
) {
    let needle = ambiguity_normalize(placeholder.replace("图标", "").replace("icon", "").trim());
    let mut candidates = Vec::new();
    for (name, sprites) in &index.localisation_icon_names {
        let normalized_name = ambiguity_normalize(name);
        if !needle.is_empty()
            && (normalized_name.contains(&needle) || needle.contains(&normalized_name))
        {
            for sprite in sprites {
                candidates.push(AmbiguityCandidate {
                    id: sprite.clone(),
                    source_kind: "localisation_icon".to_string(),
                    source_file: format!("localisation icon alias `{name}`"),
                    risk: "icon alias may not be the intended visible sprite".to_string(),
                });
            }
        }
    }
    if candidates.is_empty() {
        for sprite in &index.sprites {
            let normalized_sprite = ambiguity_normalize(sprite);
            if !needle.is_empty() && normalized_sprite.contains(&needle) {
                candidates.push(AmbiguityCandidate {
                    id: sprite.clone(),
                    source_kind: "sprite".to_string(),
                    source_file: "interface/*.gfx".to_string(),
                    risk: "sprite name matched text but no localisation alias proved intent"
                        .to_string(),
                });
            }
        }
    }
    if candidates.len() != 1 {
        questions.push(AmbiguityQuestion {
            id: ambiguity_question_id("icon", placeholder),
            kind: "icon".to_string(),
            prompt: format!(
                "Icon placeholder `{placeholder}` needs an indexed sprite choice or user authorization to create/provide an icon."
            ),
            candidates,
            risk: "missing or ambiguous icon must not become an invented GFX_* reference".to_string(),
        });
    }
}

fn push_flag_question(
    questions: &mut Vec<AmbiguityQuestion>,
    placeholder: &str,
    index: &GameIndex,
    roots: &[PathBuf],
) {
    let alias = placeholder
        .replace("国旗", "")
        .replace("旗子", "")
        .replace("flag", "");
    let tags = country_alias_candidates(index, alias.trim());
    let mut candidates = Vec::new();
    for tag in &tags {
        candidates.push(AmbiguityCandidate {
            id: tag.clone(),
            source_kind: "country_or_cosmetic_flag".to_string(),
            source_file: flag_triplet_source(roots, tag),
            risk: if flag_triplet_exists(roots, tag) {
                "flag triplet exists; still confirm this is the intended country/cosmetic flag"
                    .to_string()
            } else {
                "flag triplet is missing in indexed roots".to_string()
            },
        });
    }
    let missing_triplet = tags.iter().any(|tag| !flag_triplet_exists(roots, tag));
    if candidates.len() != 1 || missing_triplet {
        questions.push(AmbiguityQuestion {
            id: ambiguity_question_id("flag", placeholder),
            kind: "flag".to_string(),
            prompt: format!(
                "Flag placeholder `{placeholder}` needs an exact country/cosmetic tag and complete normal/medium/small .tga triplet or asset authorization."
            ),
            candidates,
            risk: "flag placeholders must not compile to guessed tag flags or incomplete assets".to_string(),
        });
    }
}

fn push_country_alias_question(
    questions: &mut Vec<AmbiguityQuestion>,
    placeholder: &str,
    index: &GameIndex,
    leader: bool,
) {
    let alias = placeholder.replace("领导人", "");
    let tags = country_alias_candidates(index, alias.trim());
    if tags.len() != 1 || leader {
        questions.push(AmbiguityQuestion {
            id: ambiguity_question_id(if leader { "leader_alias" } else { "country_alias" }, placeholder),
            kind: if leader { "leader_alias" } else { "country_or_cosmetic_alias" }.to_string(),
            prompt: if leader {
                format!(
                    "`{placeholder}` must resolve against the current country/cosmetic tag state before compiling GetLeader or leader changes."
                )
            } else {
                format!(
                    "`{placeholder}` needs one exact indexed country/cosmetic tag before it can be used in localisation or effects."
                )
            },
            candidates: tags
                .into_iter()
                .map(|tag| AmbiguityCandidate {
                    id: tag,
                    source_kind: "country_or_cosmetic_alias".to_string(),
                    source_file: "localisation country name/DEF/ADJ alias".to_string(),
                    risk: "country name may refer to base tag or cosmetic tag depending on formation state".to_string(),
                })
                .collect(),
            risk: "do not guess tag/cosmetic alias from visible Chinese name".to_string(),
        });
    }
}

fn push_text_alias_questions(
    questions: &mut Vec<AmbiguityQuestion>,
    text: &str,
    index: &GameIndex,
) {
    for (name, tags) in &index.country_name_tags {
        if name.len() < 2 || !text.contains(name) || tags.len() <= 1 {
            continue;
        }
        questions.push(AmbiguityQuestion {
            id: ambiguity_question_id("text_country_alias", name),
            kind: "country_or_cosmetic_alias".to_string(),
            prompt: format!("Visible country name `{name}` matches multiple indexed tags/cosmetic aliases; choose one."),
            candidates: tags
                .iter()
                .map(|tag| AmbiguityCandidate {
                    id: tag.clone(),
                    source_kind: "country_or_cosmetic_alias".to_string(),
                    source_file: "localisation country name/DEF/ADJ alias".to_string(),
                    risk: "same visible name can mean a base country or a cosmetic tag".to_string(),
                })
                .collect(),
            risk: "ambiguous visible country name cannot be auto-selected".to_string(),
        });
    }
}

fn push_place_questions(questions: &mut Vec<AmbiguityQuestion>, text: &str, index: &GameIndex) {
    let mut candidates = Vec::new();
    for (key, id) in &index.state_names {
        let localized = index.localisation_entries.get(key);
        let visible = localized.map(String::as_str).unwrap_or(key);
        if visible.len() >= 2 && text.contains(visible) {
            candidates.push(AmbiguityCandidate {
                id: id.to_string(),
                source_kind: "state".to_string(),
                source_file: format!("localisation key `{key}`"),
                risk: "state match needs province/victory-point/building evidence before OOB or base placement".to_string(),
            });
        }
    }
    if candidates.len() > 1 {
        questions.push(AmbiguityQuestion {
            id: "place_ambiguous".to_string(),
            kind: "place".to_string(),
            prompt: "Place text matches multiple states/provinces; choose exact state/province/victory point target.".to_string(),
            candidates,
            risk: "map edits and OOB deployment need exact state/province IDs".to_string(),
        });
    }
}

fn country_alias_candidates(index: &GameIndex, raw: &str) -> Vec<String> {
    let normalized = ambiguity_normalize(raw);
    if normalized.is_empty() {
        return Vec::new();
    }
    let mut out = BTreeSet::new();
    if index.country_tags.contains(raw) {
        out.insert(raw.to_string());
    }
    for (name, tags) in &index.country_name_tags {
        let normalized_name = ambiguity_normalize(name);
        if normalized_name == normalized
            || normalized_name.contains(&normalized)
            || normalized.contains(&normalized_name)
        {
            out.extend(tags.iter().cloned());
        }
    }
    out.into_iter().collect()
}

fn flag_triplet_exists(roots: &[PathBuf], tag: &str) -> bool {
    ["", "medium/", "small/"].iter().all(|prefix| {
        roots.iter().any(|root| {
            root.join("gfx")
                .join("flags")
                .join(prefix)
                .join(format!("{tag}.tga"))
                .exists()
        })
    })
}

fn flag_triplet_source(roots: &[PathBuf], tag: &str) -> String {
    if flag_triplet_exists(roots, tag) {
        return format!("gfx/flags/{{,medium,small}}/{tag}.tga");
    }
    "missing gfx/flags triplet in indexed roots".to_string()
}

fn bracket_placeholders(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut inside = false;
    for ch in text.chars() {
        match ch {
            '【' if !inside => {
                inside = true;
                current.clear();
            }
            '】' if inside => {
                let value = current.trim().to_string();
                if !value.is_empty() {
                    out.push(value);
                }
                inside = false;
                current.clear();
            }
            _ if inside => current.push(ch),
            _ => {}
        }
    }
    out
}

fn ambiguity_normalize(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric() || ('\u{4e00}'..='\u{9fff}').contains(ch))
        .collect::<String>()
        .to_ascii_lowercase()
}

fn ambiguity_question_id(kind: &str, value: &str) -> String {
    let normalized = ambiguity_normalize(value);
    let suffix = if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized.chars().take(24).collect()
    };
    format!("{kind}_{suffix}")
}

fn dedup_ambiguity_questions(questions: Vec<AmbiguityQuestion>) -> Vec<AmbiguityQuestion> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for question in questions {
        if seen.insert(question.id.clone()) {
            out.push(question);
        }
    }
    out
}

fn ambiguity_report_json(
    game_root: &Path,
    mod_root: Option<&Path>,
    text: &str,
    questions: &[AmbiguityQuestion],
) -> String {
    let ok = questions.is_empty();
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"game_root\": {},\n  \"mod_root\": {},\n  \"text\": {},\n  \"question_count\": {},\n  \"questions\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.ambiguity_report.v1"),
        json_bool(ok),
        json_str(if ok { "no_ambiguity_found" } else { "questions_required" }),
        json_str(&game_root.display().to_string()),
        json_optional_str(mod_root.map(|root| root.display().to_string()).as_deref()),
        json_str(text),
        questions.len(),
        ambiguity_questions_json(questions),
        json_str("unresolved candidates must be answered by the user before any code writer runs")
    )
}

fn ambiguity_questions_json(questions: &[AmbiguityQuestion]) -> String {
    format!(
        "[{}]",
        questions
            .iter()
            .map(ambiguity_question_json)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn ambiguity_question_json(question: &AmbiguityQuestion) -> String {
    format!(
        "{{\"id\": {}, \"kind\": {}, \"prompt\": {}, \"candidates\": {}, \"risk\": {}}}",
        json_str(&question.id),
        json_str(&question.kind),
        json_str(&question.prompt),
        ambiguity_candidates_json(&question.candidates),
        json_str(&question.risk)
    )
}

fn ambiguity_candidates_json(candidates: &[AmbiguityCandidate]) -> String {
    format!(
        "[{}]",
        candidates
            .iter()
            .map(|candidate| {
                format!(
                    "{{\"id\": {}, \"source_kind\": {}, \"source_file\": {}, \"risk\": {}}}",
                    json_str(&candidate.id),
                    json_str(&candidate.source_kind),
                    json_str(&candidate.source_file),
                    json_str(&candidate.risk)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn json_question_ids(report: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = report;
    while let Some(pos) = rest.find("\"id\": \"") {
        let after = &rest[pos + "\"id\": \"".len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let tail = after[end + 1..].trim_start();
        if tail.starts_with(", \"kind\":") {
            out.push(after[..end].to_string());
        }
        rest = &after[end + 1..];
    }
    out
}
