//! P62 one-shot/document author compiler.
//!
//! This command is deliberately plan-only. It turns text/docx/xlsx/csv input
//! into lane-scoped operations that later transaction/apply commands can review.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
struct AuthorCompilerLane {
    lane: &'static str,
    reason: &'static str,
    changed_file: &'static str,
    writer: &'static str,
    text: String,
}

struct AuthorCompilerInput {
    source_kind: String,
    source_ref: String,
    text: String,
    source_items: Vec<AuthorCompilerSourceItem>,
}

struct AuthorCompilerSourceItem {
    kind: String,
    reference: String,
    locator: String,
    lane_hint: String,
}

pub(crate) fn cmd_author_compiler_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let target_root = resolve_mod_root(&mod_root)?.root;
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let parent_roots = repeated_values(&map, "mod-path")
        .into_iter()
        .map(|path| resolve_mod_root(&normalize_path(path)?).map(|resolved| resolved.root))
        .collect::<Result<Vec<_>, String>>()?;

    let input = author_compiler_input(&map)?;
    let lanes = author_compiler_lanes(&input.text);
    let mut changed_files = lanes
        .iter()
        .map(|lane| lane.changed_file.to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    changed_files.sort();

    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if !target_root.is_dir() {
        blockers.push(format!(
            "mod root `{}` does not exist",
            target_root.display()
        ));
    }
    if !game_root.is_dir() {
        blockers.push(format!(
            "game root `{}` does not exist",
            game_root.display()
        ));
    }
    for item in &input.source_items {
        if item.kind == "image_asset" {
            let path = PathBuf::from(&item.reference);
            if !path.is_file() {
                blockers.push(format!("image asset `{}` does not exist", item.reference));
            }
        }
    }
    if input.text.trim().is_empty() {
        blockers.push("author compiler input is empty".to_string());
    }
    if author_compiler_raw_clausewitz(&input.text) {
        blockers.push(
            "raw Clausewitz block detected; submit structured intent, not final code".to_string(),
        );
    }
    if lanes.is_empty() {
        questions.push(
            "No lane was confidently detected; specify focus/event/idea/decision/gui/history/oob/map/asset/localisation."
                .to_string(),
        );
    }
    if input.text.contains('【') || input.text.contains('】') {
        questions.push(
            "Resolve bracket placeholders such as icons, flags, cosmetic tags, leaders, and colour spans before apply."
                .to_string(),
        );
    }

    let ok = blockers.is_empty();
    let report = author_compiler_json(
        ok,
        &target_root,
        &game_root,
        &parent_roots,
        &input,
        &lanes,
        &changed_files,
        &questions,
        &blockers,
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

fn author_compiler_input(map: &ArgMap) -> Result<AuthorCompilerInput, String> {
    let mut source_items = Vec::new();
    let mut text_parts = Vec::new();
    if let Some(text) = value(map, "text").or_else(|| value(map, "request")) {
        source_items.push(AuthorCompilerSourceItem {
            kind: "plain_text".to_string(),
            reference: "inline_text".to_string(),
            locator: "inline_text".to_string(),
            lane_hint: "auto".to_string(),
        });
        text_parts.push(text.to_string());
    }

    let input_values = repeated_values(map, "input")
        .into_iter()
        .chain(repeated_values(map, "file"))
        .collect::<Vec<_>>();
    for input_value in input_values {
        let input = normalize_path(&input_value)?;
        let source_kind = author_compiler_source_kind(&input);
        let text = read_text_document(&input)?;
        source_items.push(AuthorCompilerSourceItem {
            kind: source_kind,
            reference: input.display().to_string(),
            locator: author_compiler_source_locator(&input),
            lane_hint: "auto".to_string(),
        });
        text_parts.push(text);
    }

    for image_value in repeated_values(map, "image") {
        let image = normalize_path(&image_value)?;
        let extension = image
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        source_items.push(AuthorCompilerSourceItem {
            kind: "image_asset".to_string(),
            reference: image.display().to_string(),
            locator: format!("image_extension:{extension}"),
            lane_hint: "asset".to_string(),
        });
        text_parts.push(format!("图片素材：{}", image.display()));
    }

    if source_items.is_empty() {
        return Err("expected --text, --request, --input, --file, or --image".to_string());
    }

    let source_kind = if source_items.len() == 1 {
        source_items[0].kind.clone()
    } else {
        "multi_source".to_string()
    };
    let source_ref = source_items
        .iter()
        .map(|item| item.reference.clone())
        .collect::<Vec<_>>()
        .join("; ");
    Ok(AuthorCompilerInput {
        source_kind,
        source_ref,
        text: text_parts.join("\n"),
        source_items,
    })
}

fn author_compiler_source_kind(path: &Path) -> String {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "docx" => "docx_word".to_string(),
        "xlsx" | "xls" | "xlsm" | "xlsb" | "ods" => "xlsx_excel".to_string(),
        "csv" | "tsv" => "csv_table".to_string(),
        "md" | "markdown" => "markdown".to_string(),
        _ => "plain_text_file".to_string(),
    }
}

fn author_compiler_source_locator(path: &Path) -> String {
    match author_compiler_source_kind(path).as_str() {
        "xlsx_excel" => "workbook/sheet/cell extraction".to_string(),
        "csv_table" => "table row/column extraction".to_string(),
        "docx_word" => "word paragraph extraction".to_string(),
        "markdown" => "markdown line extraction".to_string(),
        _ => "text line extraction".to_string(),
    }
}

fn author_compiler_lanes(text: &str) -> Vec<AuthorCompilerLane> {
    let mut lanes = Vec::new();
    for (lane, reason, changed_file, writer, keywords) in author_compiler_lane_specs() {
        let mut snippets = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let lower = trimmed.to_ascii_lowercase();
            if keywords.iter().any(|keyword| {
                trimmed.contains(keyword) || lower.contains(&keyword.to_ascii_lowercase())
            }) {
                snippets.push(format!("line {}: {}", idx + 1, trimmed));
            }
        }
        if !snippets.is_empty() {
            lanes.push(AuthorCompilerLane {
                lane,
                reason,
                changed_file,
                writer,
                text: snippets.join("\n"),
            });
        }
    }
    lanes
}

fn author_compiler_lane_specs() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Vec<&'static str>,
)> {
    vec![
        (
            "focus",
            "focus_keyword",
            "common/national_focus/generated_focus.txt",
            "focus writer",
            vec!["国策", "focus", "completion_reward"],
        ),
        (
            "event",
            "event_keyword",
            "events/generated_events.txt",
            "event writer",
            vec!["事件", "event", "option", "新闻"],
        ),
        (
            "idea",
            "idea_keyword",
            "common/ideas/generated_ideas.txt",
            "idea writer",
            vec!["民族精神", "national spirit", "idea", "顾问", "替换为"],
        ),
        (
            "dynamic_modifier",
            "dynamic_modifier_keyword",
            "common/dynamic_modifiers/generated_dynamic_modifiers.txt",
            "dynamic modifier writer",
            vec![
                "动态修正",
                "dynamic modifier",
                "set_temp_variable",
                "change_",
            ],
        ),
        (
            "decision",
            "decision_keyword",
            "common/decisions/generated_decisions.txt",
            "decision writer",
            vec!["决议", "decision", "任务", "mission"],
        ),
        (
            "gui",
            "gui_keyword",
            "common/scripted_guis/generated_gui.txt",
            "gui workflow",
            vec!["gui", "界面", "按钮", "scripted_gui", "挂载"],
        ),
        (
            "history",
            "history_keyword",
            "history/countries/generated_history.txt",
            "history transaction",
            vec!["开局", "history", "领导人", "执政党", "科技", "首都"],
        ),
        (
            "oob",
            "oob_keyword",
            "history/units/generated_oob.txt",
            "oob transaction",
            vec!["oob", "部署", "师", "联队", "舰队", "location"],
        ),
        (
            "map",
            "map_keyword",
            "history/states/generated_state_changes.txt",
            "map/state transaction",
            vec![
                "地图",
                "州",
                "省份",
                "铁路",
                "补给",
                "胜利点",
                "资源",
                "人口",
            ],
        ),
        (
            "asset",
            "asset_keyword",
            "interface/generated_assets.gfx",
            "asset registry",
            vec![
                "图标",
                "国旗",
                "旗子",
                "头像",
                "图片",
                "icon",
                "flag",
                "sprite",
                "gfx",
                "图片素材",
            ],
        ),
        (
            "localisation",
            "localisation_keyword",
            "localisation/simp_chinese/generated_l_simp_chinese.yml",
            "localisation writer",
            vec![
                "本地化",
                "文案",
                "翻译",
                "【",
                "】",
                "§",
                "$",
                "[ROOT",
                "[Root",
            ],
        ),
        (
            "gameplay_guide",
            "guide_keyword",
            ".hoi4skill/gameplay_route_guide.md",
            "route guide",
            vec!["攻略", "路线", "怎么玩", "控制台", "research all"],
        ),
    ]
}

fn author_compiler_raw_clausewitz(text: &str) -> bool {
    [
        "completion_reward = {",
        "option = {",
        "modifier = {",
        "immediate = {",
        "hidden_effect = {",
        "set_temp_variable = {",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn author_compiler_json(
    ok: bool,
    mod_root: &Path,
    game_root: &Path,
    parent_roots: &[PathBuf],
    input: &AuthorCompilerInput,
    lanes: &[AuthorCompilerLane],
    changed_files: &[String],
    questions: &[String],
    blockers: &[String],
) -> String {
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.author_compiler_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "author_compiler_plan_ready"
        } else {
            "author_compiler_blocked"
        }),
    );
    map.insert("direct_write".to_string(), json_bool(false).to_string());
    map.insert(
        "p101_contract".to_string(),
        json_str("unified_author_compiler_entry"),
    );
    map.insert(
        "mod_root".to_string(),
        json_str(&mod_root.display().to_string()),
    );
    map.insert(
        "game_root".to_string(),
        json_str(&game_root.display().to_string()),
    );
    map.insert(
        "parent_mod_roots".to_string(),
        json_array(
            &parent_roots
                .iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>(),
        ),
    );
    map.insert("source_kind".to_string(), json_str(&input.source_kind));
    map.insert("source_ref".to_string(), json_str(&input.source_ref));
    map.insert(
        "source_items".to_string(),
        author_compiler_source_items_json(&input.source_items),
    );
    map.insert(
        "input_modes".to_string(),
        author_compiler_input_modes_json(&input.source_items),
    );
    map.insert(
        "input_preview".to_string(),
        json_str(&text_preview(&input.text)),
    );
    map.insert("lane_count".to_string(), lanes.len().to_string());
    map.insert("lanes".to_string(), author_compiler_lanes_json(lanes));
    map.insert("changed_files".to_string(), json_array(changed_files));
    map.insert(
        "operations".to_string(),
        author_compiler_operations_json(lanes),
    );
    map.insert(
        "transaction_graph".to_string(),
        author_compiler_transaction_graph_json(lanes),
    );
    map.insert("questions".to_string(), json_array(questions));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "rules".to_string(),
        json_array(&[
            "author-compiler-plan is plan-only and never writes Clausewitz files".to_string(),
            "AI may only provide structured intent, candidates, questions, and repair suggestions".to_string(),
            "Rust writers assemble final HOI4 files from local indexed evidence".to_string(),
            "each downstream writer must consume only its own lane text".to_string(),
            "all symbols, scopes, placeholders, and assets must pass local index gates before apply".to_string(),
        ]),
    );
    map.insert(
        "next_commands".to_string(),
        json_array(&[
            "hoi4skill mod-transaction-plan --mod-root <target> --game-root <hoi4> --plan author_compiler_plan.json --require-passed --output .hoi4skill/transaction.json".to_string(),
            "hoi4skill stale-plan-gate --input .hoi4skill/transaction.json --knowledge .hoi4skill/kb.json --require-passed".to_string(),
            "hoi4skill mod-transaction-apply --input .hoi4skill/transaction.json --execute --final-check --atomic --require-passed".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn author_compiler_source_items_json(items: &[AuthorCompilerSourceItem]) -> String {
    format!(
        "[{}]",
        items
            .iter()
            .map(|item| {
                let mut map = BTreeMap::new();
                map.insert("kind".to_string(), json_str(&item.kind));
                map.insert("reference".to_string(), json_str(&item.reference));
                map.insert("locator".to_string(), json_str(&item.locator));
                map.insert("lane_hint".to_string(), json_str(&item.lane_hint));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn author_compiler_input_modes_json(items: &[AuthorCompilerSourceItem]) -> String {
    let mut modes = items
        .iter()
        .map(|item| item.kind.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    modes.sort();
    json_array(&modes)
}

fn author_compiler_lanes_json(lanes: &[AuthorCompilerLane]) -> String {
    format!(
        "[{}]",
        lanes
            .iter()
            .map(|lane| {
                let mut map = BTreeMap::new();
                map.insert("lane".to_string(), json_str(lane.lane));
                map.insert("reason".to_string(), json_str(lane.reason));
                map.insert("writer".to_string(), json_str(lane.writer));
                map.insert("changed_file".to_string(), json_str(lane.changed_file));
                map.insert("text".to_string(), json_str(&lane.text));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn author_compiler_operations_json(lanes: &[AuthorCompilerLane]) -> String {
    format!(
        "[{}]",
        lanes
            .iter()
            .map(|lane| {
                let mut map = BTreeMap::new();
                map.insert("system".to_string(), json_str(lane.lane));
                map.insert("operation".to_string(), json_str("plan_lane_transaction"));
                map.insert("writer".to_string(), json_str(lane.writer));
                map.insert("changed_file".to_string(), json_str(lane.changed_file));
                map.insert("source".to_string(), json_str("author_compiler_lane_text"));
                map.insert(
                    "risk".to_string(),
                    json_str(author_compiler_lane_risk(lane.lane)),
                );
                map.insert(
                    "evidence_required".to_string(),
                    json_str(author_compiler_lane_evidence(lane.lane)),
                );
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn author_compiler_transaction_graph_json(lanes: &[AuthorCompilerLane]) -> String {
    let nodes = lanes
        .iter()
        .map(|lane| {
            let mut map = BTreeMap::new();
            map.insert("id".to_string(), json_str(lane.lane));
            map.insert("writer".to_string(), json_str(lane.writer));
            map.insert("changed_file".to_string(), json_str(lane.changed_file));
            json_raw_object(&map)
        })
        .collect::<Vec<_>>();
    let edges = author_compiler_transaction_edges(lanes)
        .into_iter()
        .map(|(from, to, reason)| {
            let mut map = BTreeMap::new();
            map.insert("from".to_string(), json_str(from));
            map.insert("to".to_string(), json_str(to));
            map.insert("reason".to_string(), json_str(reason));
            json_raw_object(&map)
        })
        .collect::<Vec<_>>();
    let mut map = BTreeMap::new();
    map.insert("nodes".to_string(), format!("[{}]", nodes.join(", ")));
    map.insert("edges".to_string(), format!("[{}]", edges.join(", ")));
    map.insert(
        "gate".to_string(),
        json_str("mod-transaction-plan must verify dependencies before apply"),
    );
    json_raw_object(&map)
}

fn author_compiler_transaction_edges(
    lanes: &[AuthorCompilerLane],
) -> Vec<(&'static str, &'static str, &'static str)> {
    let lane_set = lanes.iter().map(|lane| lane.lane).collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    for lane in &["focus", "event", "decision", "idea", "gui"] {
        if lane_set.contains(lane) && lane_set.contains("localisation") {
            edges.push((
                *lane,
                "localisation",
                "visible text must align with user source",
            ));
        }
        if lane_set.contains(lane) && lane_set.contains("asset") {
            edges.push((*lane, "asset", "referenced sprites and pictures must exist"));
        }
    }
    if lane_set.contains("history") && lane_set.contains("oob") {
        edges.push((
            "history",
            "oob",
            "OOB locations depend on start-date state ownership",
        ));
    }
    if lane_set.contains("map") && lane_set.contains("oob") {
        edges.push((
            "map",
            "oob",
            "OOB province selection depends on map/state evidence",
        ));
    }
    edges
}

fn author_compiler_lane_risk(lane: &str) -> &'static str {
    match lane {
        "map" | "history" | "oob" | "gui" => "high",
        "asset" | "dynamic_modifier" | "common_high_value" => "medium",
        _ => "normal",
    }
}

fn author_compiler_lane_evidence(lane: &str) -> &'static str {
    match lane {
        "focus" => "focus id, reward effects, prerequisites, icon, localisation",
        "event" => "namespace/id, trigger source, options, follow-up events, localisation",
        "idea" => "idea id, modifier registry, picture/sprite, localisation, container type",
        "dynamic_modifier" => "dynamic modifier id, scripted_effect helper, variable parameters",
        "decision" => "decision category, visible/available/complete/remove effects",
        "gui" => "scripted_gui, interface gui/gfx, mount point, scripted effects/triggers",
        "history" => "tag, character, politics, technology, state/province, diplomacy evidence",
        "oob" => "unit taxonomy, division/air/naval kind, province/base evidence",
        "map" => "state/province ids, resources/buildings/vp/network/topology evidence",
        "asset" => "source image, target format, sprite id, path, size",
        "localisation" => "key, language, token preservation, placeholder resolution",
        "gameplay_guide" => "real focus/event/decision ids and route graph evidence",
        _ => "local game/parent/target index evidence",
    }
}

fn text_preview(text: &str) -> String {
    let mut out = text.lines().take(8).collect::<Vec<_>>().join("\n");
    if out.chars().count() > 500 {
        out = out.chars().take(500).collect::<String>();
        out.push_str("...");
    }
    out
}
