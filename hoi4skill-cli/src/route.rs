//! P4 event-chain, route-guide, and asset-plan reports.

#[allow(unused_imports)]
use crate::*;

struct EventNode {
    id: String,
    file: String,
    outgoing: Vec<String>,
}

struct EventSource {
    kind: String,
    id: String,
    file: String,
    outgoing: Vec<String>,
}

struct EventChainDraftNode {
    key: String,
    title: String,
    event_type: String,
    source_line: usize,
    option_count: usize,
    outgoing_raw: Vec<String>,
    outgoing: Vec<String>,
}

pub(crate) fn cmd_event_chain_graph(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let nodes = scan_event_nodes(&root)?;
    let json = render_event_chain_graph_json(&root, &nodes);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_route_guide(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let start = require_value(&map, "start")?;
    let nodes = scan_event_nodes(&root)?;
    let steps = reachable_event_steps(&nodes, &start, parse_usize_option(&map, "max-steps", 40)?);
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"start\": {},\n  \"step_count\": {},\n  \"steps\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.route_guide.v1"),
        json_bool(!steps.is_empty()),
        json_str(if steps.is_empty() { "start_not_found" } else { "route_ready" }),
        json_str(&root.display().to_string()),
        json_str(&start),
        steps.len(),
        json_array(&steps),
        json_str("route guide is evidence from existing event references; do not invent missing event choices")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && steps.is_empty() {
        return Err(format!(
            "route start `{start}` was not found in indexed events"
        ));
    }
    Ok(())
}

pub(crate) fn cmd_transaction_route_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let text = read_utf8_lossy(&input)?;
    let changed_files = json_string_array_field(&text, "changed_files");
    let systems = route_transaction_systems(&text, &changed_files);
    let has_focus = systems.contains("focus");
    let has_event = systems.contains("event");
    let has_decision = systems.contains("decision");
    let needs_route = has_focus || has_event || has_decision;
    let dependency_graph = json_string_array_field(&text, "dependency_graph");
    let has_declared_route_edge = dependency_graph.iter().any(|edge| {
        edge.contains("focus completion_reward -> event trigger")
            || edge.contains("decision route ->")
            || edge.contains("event")
    });
    let mut blockers = Vec::new();
    if !text.contains("\"schema\": \"hoi4skill.mod_transaction_plan.v1\"") {
        blockers.push("input is not a mod-transaction-plan report".to_string());
    }
    if needs_route && !has_declared_route_edge {
        blockers.push(
            "focus/event/decision transaction lacks a declared route dependency edge".to_string(),
        );
    }
    if needs_route && changed_files.is_empty() {
        blockers.push("route transaction plan needs changed_files evidence".to_string());
    }
    let ok = blockers.is_empty();
    let status = if ok && needs_route {
        "route_plan_ready"
    } else if ok {
        "route_not_required"
    } else {
        "blocked"
    };
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"input\": {},\n  \"needs_route\": {},\n  \"systems\": {},\n  \"changed_files\": {},\n  \"dependency_graph\": {},\n  \"blockers\": {},\n  \"next_commands\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.transaction_route_plan.v1"),
        json_bool(ok),
        json_str(status),
        json_str(&input.display().to_string()),
        json_bool(needs_route),
        json_array(&systems.into_iter().collect::<Vec<_>>()),
        json_array(&changed_files),
        json_array(&dependency_graph),
        json_array(&blockers),
        json_array(&[
            "After writers apply, run event-chain-graph and route-blocker-audit against the generated event ids.".to_string(),
            "Do not treat transaction-route-plan as playable runtime evidence; it is pre-write route topology evidence only.".to_string(),
        ]),
        json_str("A transaction that touches focus/event/decision content must declare route dependencies before runtime-evidence-gate can proceed.")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_event_chain_author_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let text = if let Some(text) = value(&map, "text").or_else(|| value(&map, "request")) {
        text.to_string()
    } else if let Some(input) = value(&map, "input") {
        read_text_document(&normalize_path(input)?)?
    } else {
        return Err("event-chain-author-plan requires --text, --request, or --input".to_string());
    };
    let namespace = value(&map, "namespace")
        .map(str::to_string)
        .or_else(|| event_chain_namespace_from_text(&text));
    let mut nodes = event_chain_draft_nodes(&text);
    let mut blockers = Vec::new();
    let mut questions = Vec::new();
    if namespace.is_none() {
        questions.push("Provide event namespace before writing event ids.".to_string());
    }
    if nodes.is_empty() {
        blockers.push("event-chain author plan found no event nodes".to_string());
    }
    event_chain_resolve_edges(&mut nodes, &mut blockers);
    event_chain_dead_node_blockers(&nodes, &mut blockers);
    event_chain_cycle_blockers(&nodes, &mut blockers);
    let ok = blockers.is_empty();
    let json = render_event_chain_author_plan_json(
        ok,
        namespace.as_deref(),
        &nodes,
        &questions,
        &blockers,
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_trigger_source_graph(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let target = require_value(&map, "event").or_else(|_| require_value(&map, "target-event"))?;
    let nodes = scan_event_nodes(&root)?;
    let sources = scan_event_sources(&root)?;
    let incoming = sources
        .iter()
        .filter(|source| source.outgoing.iter().any(|id| id == &target))
        .collect::<Vec<_>>();
    let target_exists = nodes.iter().any(|node| node.id == target);
    let mut blockers = Vec::new();
    if !target_exists {
        blockers.push(format!(
            "target event `{target}` is not defined in scanned events"
        ));
    }
    if incoming.is_empty() {
        blockers.push(format!(
            "target event `{target}` has no scanned trigger source"
        ));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"target_event\": {},\n  \"target_exists\": {},\n  \"incoming_count\": {},\n  \"incoming_sources\": [{}],\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.trigger_source_graph.v1"),
        json_bool(ok),
        json_str(if ok { "trigger_sources_ready" } else { "blocked" }),
        json_str(&root.display().to_string()),
        json_str(&target),
        json_bool(target_exists),
        incoming.len(),
        render_event_source_refs(&incoming),
        json_array(&blockers),
        json_str("trigger sources come only from scanned event/focus/decision/on_action/scripted_effect calls; no route is invented when no evidence exists")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_on_action_graph(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let sources = scan_on_action_sources(&root)?;
    let rows = sources.iter().collect::<Vec<_>>();
    let ok = !rows.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"on_action_count\": {},\n  \"on_actions\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.on_action_graph.v1"),
        json_bool(ok),
        json_str(if ok { "on_action_graph_ready" } else { "no_on_actions_found" }),
        json_str(&root.display().to_string()),
        rows.len(),
        render_event_source_refs(&rows),
        json_str("on_action graph lists indexed on_action entry points and event calls found under common/on_actions")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err("no on_actions found".to_string());
    }
    Ok(())
}

pub(crate) fn cmd_dead_event_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let nodes = scan_event_nodes(&root)?;
    let sources = scan_event_sources(&root)?;
    let incoming_ids = sources
        .iter()
        .flat_map(|source| source.outgoing.iter().cloned())
        .collect::<BTreeSet<_>>();
    let dead = nodes
        .iter()
        .filter(|node| !incoming_ids.contains(&node.id))
        .map(|node| {
            format!(
                "{{\"id\": {}, \"file\": {}, \"outgoing_count\": {}}}",
                json_str(&node.id),
                json_str(&node.file),
                node.outgoing.len()
            )
        })
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"mod_root\": {},\n  \"event_count\": {},\n  \"dead_event_count\": {},\n  \"dead_events\": [{}],\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.dead_event_audit.v1"),
        json_str(if dead.is_empty() { "no_dead_events_found" } else { "dead_events_reported" }),
        json_str(&root.display().to_string()),
        nodes.len(),
        dead.len(),
        dead.join(", "),
        json_str("dead-event audit reports missing incoming evidence; later authoring must either add a trigger path or mark the event intentionally external")
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_route_blocker_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let target = require_value(&map, "target-event").or_else(|_| require_value(&map, "event"))?;
    let nodes = scan_event_nodes(&root)?;
    let sources = scan_event_sources(&root)?;
    let incoming = sources
        .iter()
        .filter(|source| source.outgoing.iter().any(|id| id == &target))
        .collect::<Vec<_>>();
    let target_node = nodes.iter().find(|node| node.id == target);
    let index = value(&map, "game-root")
        .map(normalize_path)
        .transpose()?
        .map(|game_root| build_game_index_with_mod_paths(&game_root, &[]))
        .transpose()?;
    let unknown_triggers = if let Some(node) = target_node {
        let file = root.join(node.file.replace('/', "\\"));
        let text = read_utf8_lossy(&file)?;
        index
            .as_ref()
            .map(|index| unindexed_trigger_keys(&text, index))
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let mut blockers = Vec::new();
    if target_node.is_none() {
        blockers.push(format!("target event `{target}` is not defined"));
    }
    if incoming.is_empty() {
        blockers.push(format!(
            "target event `{target}` has no incoming route evidence"
        ));
    }
    for trigger in &unknown_triggers {
        blockers.push(format!("trigger `{trigger}` is not indexed"));
    }
    let ok = blockers.is_empty();
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"target_event\": {},\n  \"incoming_count\": {},\n  \"incoming_sources\": [{}],\n  \"unknown_triggers\": {},\n  \"blockers\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.route_blocker_audit.v1"),
        json_bool(ok),
        json_str(if ok { "route_clear" } else { "blocked" }),
        json_str(&root.display().to_string()),
        json_str(&target),
        incoming.len(),
        render_event_source_refs(&incoming),
        json_array(&unknown_triggers),
        json_array(&blockers),
        json_str("route blocker audit reports missing definitions, missing incoming routes, and unindexed trigger keys before AI explains or rewrites a chain")
    );
    write_or_print(&json, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        return Err(blockers.join("; "));
    }
    Ok(())
}

pub(crate) fn cmd_icon_generate_plan(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let id = require_value(&map, "id")?;
    let kind = value(&map, "kind").unwrap_or("focus");
    let prompt = value(&map, "prompt").unwrap_or(&id);
    let sprite = value(&map, "sprite")
        .map(str::to_string)
        .unwrap_or_else(|| format!("GFX_{}_{}", kind, slugify(&id, "icon")));
    let texturefile = value(&map, "texturefile")
        .map(str::to_string)
        .unwrap_or_else(|| format!("gfx/interface/goals/{}.dds", slugify(&id, "icon")));
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"id\": {},\n  \"kind\": {},\n  \"sprite\": {},\n  \"texturefile\": {},\n  \"prompt\": {},\n  \"next_commands\": {}\n}}\n",
        json_str("hoi4skill.icon_generate_plan.v1"),
        json_str("icon_plan_ready"),
        json_str(&id),
        json_str(kind),
        json_str(&sprite),
        json_str(&texturefile),
        json_str(prompt),
        json_array(&[format!("hoi4skill import-generated-icon --sprite {sprite} --texturefile {texturefile} --mod-root <mod> --execute")])
    );
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_import_generated_icon(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let root = required_mod_root_for_route(&map)?;
    let sprite = require_value(&map, "sprite")?;
    let texturefile = require_value(&map, "texturefile")?;
    let gfx_file = value(&map, "gfx-file").unwrap_or("interface/generated_icons.gfx");
    let target = root.join(gfx_file.replace('/', "\\"));
    let code = format!(
        "spriteTypes = {{\n  spriteType = {{\n    name = \"{sprite}\"\n    texturefile = \"{texturefile}\"\n  }}\n}}\n"
    );
    let mut artifacts = Vec::new();
    if map.flags.contains("execute") {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&target, code).map_err(|e| format!("write {}: {e}", target.display()))?;
        artifacts.push(target.display().to_string());
    }
    let json = format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"sprite\": {},\n  \"texturefile\": {},\n  \"gfx_file\": {},\n  \"artifacts\": {},\n  \"rule\": {}\n}}\n",
        json_str("hoi4skill.import_generated_icon.v1"),
        json_str(if map.flags.contains("execute") { "icon_registered" } else { "plan_only" }),
        json_str(&sprite),
        json_str(&texturefile),
        json_str(gfx_file),
        json_array(&artifacts),
        json_str("register the sprite before focus/idea/event code references it")
    );
    write_or_print(&json, value(&map, "output"))
}

fn required_mod_root_for_route(map: &ArgMap) -> Result<PathBuf, String> {
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root".to_string())?;
    Ok(resolve_mod_root(&normalize_path(&input)?)?.root)
}

fn event_chain_namespace_from_text(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        for prefix in ["命名空间：", "命名空间:", "namespace：", "namespace:"] {
            if let Some(value) = trimmed.strip_prefix(prefix) {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(slugify(value, "event_namespace"));
                }
            }
        }
    }
    None
}

fn event_chain_draft_nodes(text: &str) -> Vec<EventChainDraftNode> {
    let mut nodes = Vec::new();
    let mut pending_key: Option<String> = None;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(value) = event_chain_value_after_any(trimmed, &["事件键：", "事件键:"]) {
            pending_key = Some(slugify(value, "event"));
            continue;
        }
        if let Some(title) = event_chain_value_after_any(trimmed, &["事件：", "事件:"]) {
            let key = pending_key
                .take()
                .unwrap_or_else(|| slugify(title, "event"));
            nodes.push(EventChainDraftNode {
                key,
                title: title.to_string(),
                event_type: "country_event".to_string(),
                source_line: index + 1,
                option_count: 0,
                outgoing_raw: Vec::new(),
                outgoing: Vec::new(),
            });
            continue;
        }
        let Some(current) = nodes.last_mut() else {
            continue;
        };
        if let Some(value) =
            event_chain_value_after_any(trimmed, &["类型：", "类型:", "后续类型："])
        {
            current.event_type =
                if value.contains("新闻") || value.eq_ignore_ascii_case("news_event") {
                    "news_event".to_string()
                } else if value.contains("州") || value.eq_ignore_ascii_case("state_event") {
                    "state_event".to_string()
                } else {
                    "country_event".to_string()
                };
        }
        if event_chain_value_after_any(trimmed, &["选项", "option"]).is_some() {
            current.option_count += 1;
        }
        if let Some(value) = event_chain_followup_value(trimmed) {
            current.outgoing_raw.push(value.to_string());
        }
    }
    nodes
}

fn event_chain_value_after_any<'a>(line: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    for prefix in prefixes {
        if let Some(value) = line.strip_prefix(prefix) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn event_chain_followup_value(line: &str) -> Option<&str> {
    if !(line.contains("后续事件") || line.contains("随机后续事件") || line.contains("follow-up"))
    {
        return None;
    }
    line.split_once('：')
        .or_else(|| line.split_once(':'))
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
}

fn event_chain_resolve_edges(nodes: &mut [EventChainDraftNode], blockers: &mut Vec<String>) {
    let mut names = BTreeMap::new();
    for node in nodes.iter() {
        names.insert(node.key.clone(), node.key.clone());
        names.insert(node.title.clone(), node.key.clone());
        let title_slug = slugify(&node.title, "event");
        if title_slug != "event" {
            names.insert(title_slug, node.key.clone());
        }
    }
    for node in nodes.iter_mut() {
        for raw in &node.outgoing_raw {
            let raw_slug = slugify(raw, "event");
            let lookup = names
                .get(raw)
                .or_else(|| {
                    (raw_slug != "event")
                        .then(|| names.get(&raw_slug))
                        .flatten()
                })
                .cloned();
            if let Some(target) = lookup {
                if !node.outgoing.iter().any(|existing| existing == &target) {
                    node.outgoing.push(target);
                }
            } else {
                blockers.push(format!(
                    "event `{}` references missing follow-up event `{}`",
                    node.title, raw
                ));
            }
        }
    }
}

fn event_chain_dead_node_blockers(nodes: &[EventChainDraftNode], blockers: &mut Vec<String>) {
    let incoming = nodes
        .iter()
        .flat_map(|node| node.outgoing.iter().cloned())
        .collect::<BTreeSet<_>>();
    for node in nodes.iter().skip(1) {
        if !incoming.contains(&node.key) {
            blockers.push(format!(
                "event `{}` has no incoming trigger edge",
                node.title
            ));
        }
    }
}

fn event_chain_cycle_blockers(nodes: &[EventChainDraftNode], blockers: &mut Vec<String>) {
    let outgoing = nodes
        .iter()
        .map(|node| (node.key.as_str(), node.outgoing.clone()))
        .collect::<BTreeMap<_, _>>();
    for node in nodes {
        let mut seen = BTreeSet::new();
        let mut stack = node.outgoing.clone();
        while let Some(next) = stack.pop() {
            if next == node.key {
                blockers.push(format!("event chain contains cycle at `{}`", node.title));
                break;
            }
            if !seen.insert(next.clone()) {
                continue;
            }
            if let Some(children) = outgoing.get(next.as_str()) {
                stack.extend(children.iter().cloned());
            }
        }
    }
    blockers.sort();
    blockers.dedup();
}

fn render_event_chain_author_plan_json(
    ok: bool,
    namespace: Option<&str>,
    nodes: &[EventChainDraftNode],
    questions: &[String],
    blockers: &[String],
) -> String {
    let edge_count = nodes.iter().map(|node| node.outgoing.len()).sum::<usize>();
    let mut map = BTreeMap::new();
    map.insert(
        "schema".to_string(),
        json_str("hoi4skill.event_chain_author_plan.v1"),
    );
    map.insert("ok".to_string(), json_bool(ok).to_string());
    map.insert(
        "status".to_string(),
        json_str(if ok {
            "event_chain_author_plan_ready"
        } else {
            "blocked"
        }),
    );
    map.insert("direct_write".to_string(), json_bool(false).to_string());
    map.insert("namespace".to_string(), json_optional_str(namespace));
    map.insert("node_count".to_string(), nodes.len().to_string());
    map.insert("edge_count".to_string(), edge_count.to_string());
    map.insert("nodes".to_string(), event_chain_draft_nodes_json(nodes));
    map.insert(
        "logic_checks".to_string(),
        json_array(&[
            "namespace_required".to_string(),
            "all_followup_events_must_exist".to_string(),
            "non_entry_events_need_incoming_edge".to_string(),
            "cycles_are_blockers_until_user_confirms_loop_design".to_string(),
        ]),
    );
    map.insert("questions".to_string(), json_array(questions));
    map.insert("blocker_count".to_string(), blockers.len().to_string());
    map.insert("blockers".to_string(), json_array(blockers));
    map.insert(
        "next_commands".to_string(),
        json_array(&[
            "hoi4skill apply-event-cards --input event_chain_cards.md --game-root <hoi4> --final-check".to_string(),
            "hoi4skill event-chain-graph --mod-root <target> --output .hoi4skill/event_chain_graph.json".to_string(),
            "hoi4skill route-blocker-audit --mod-root <target> --event <event.id> --require-passed".to_string(),
        ]),
    );
    map.insert(
        "rules".to_string(),
        json_array(&[
            "event-chain-author-plan is plan-only and never writes event files".to_string(),
            "AI prose must be converted into node/edge cards before Clausewitz writers run"
                .to_string(),
            "dead events, missing follow-ups, and unconfirmed cycles block apply".to_string(),
        ]),
    );
    json_raw_object(&map)
}

fn event_chain_draft_nodes_json(nodes: &[EventChainDraftNode]) -> String {
    format!(
        "[{}]",
        nodes
            .iter()
            .map(|node| {
                let mut map = BTreeMap::new();
                map.insert("key".to_string(), json_str(&node.key));
                map.insert("title".to_string(), json_str(&node.title));
                map.insert("event_type".to_string(), json_str(&node.event_type));
                map.insert("source_line".to_string(), node.source_line.to_string());
                map.insert("option_count".to_string(), node.option_count.to_string());
                map.insert("outgoing_raw".to_string(), json_array(&node.outgoing_raw));
                map.insert("outgoing".to_string(), json_array(&node.outgoing));
                json_raw_object(&map)
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn route_transaction_systems(text: &str, changed_files: &[String]) -> BTreeSet<String> {
    let mut systems = BTreeSet::new();
    for system in json_string_values(text, "system") {
        systems.insert(system);
    }
    for file in changed_files {
        let normalized = file.replace('\\', "/").to_ascii_lowercase();
        let system = if normalized.starts_with("common/national_focus/") {
            "focus"
        } else if normalized.starts_with("events/") {
            "event"
        } else if normalized.starts_with("common/decisions/") {
            "decision"
        } else if normalized.starts_with("common/on_actions/") {
            "on_action"
        } else {
            continue;
        };
        systems.insert(system.to_string());
    }
    systems
}

fn json_string_values(text: &str, key: &str) -> Vec<String> {
    let marker = format!("\"{key}\":");
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find(&marker) {
        let value_start = offset + start + marker.len();
        let rest = text[value_start..].trim_start();
        if let Some(rest) = rest.strip_prefix('"') {
            let mut value = String::new();
            let mut escaped = false;
            for ch in rest.chars() {
                if escaped {
                    value.push(match ch {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    });
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    out.push(value);
                    break;
                } else {
                    value.push(ch);
                }
            }
        }
        offset = value_start;
    }
    out.sort();
    out.dedup();
    out
}

fn scan_event_nodes(root: &Path) -> Result<Vec<EventNode>, String> {
    let mut nodes = Vec::new();
    let event_root = root.join("events");
    if !event_root.exists() {
        return Ok(nodes);
    }
    for file in collect_files(&event_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for block in event_definition_blocks(&text) {
            let Some(id) = block_assignment(&block, "id") else {
                continue;
            };
            if id.contains('.') && !nodes.iter().any(|node| node.id == id) {
                nodes.push(EventNode {
                    id,
                    file: relative_slash_path(root, &file),
                    outgoing: extract_event_calls(&block),
                });
            }
        }
    }
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(nodes)
}

fn scan_event_sources(root: &Path) -> Result<Vec<EventSource>, String> {
    let mut sources = Vec::new();
    for node in scan_event_nodes(root)? {
        sources.push(EventSource {
            kind: "event".to_string(),
            id: node.id,
            file: node.file,
            outgoing: node.outgoing,
        });
    }
    sources.extend(scan_generic_event_sources(
        root,
        "common/national_focus",
        "focus",
    )?);
    sources.extend(scan_generic_event_sources(
        root,
        "common/decisions",
        "decision",
    )?);
    sources.extend(scan_generic_event_sources(
        root,
        "common/scripted_effects",
        "scripted_effect",
    )?);
    sources.extend(scan_on_action_sources(root)?);
    sources.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.file.cmp(&b.file))
    });
    Ok(sources)
}

fn scan_generic_event_sources(
    root: &Path,
    relative_dir: &str,
    kind: &str,
) -> Result<Vec<EventSource>, String> {
    let mut sources = Vec::new();
    let dir = root.join(relative_dir.replace('/', "\\"));
    if !dir.exists() {
        return Ok(sources);
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let outgoing = extract_event_calls(&text);
        if outgoing.is_empty() {
            continue;
        }
        let ids = extract_route_source_ids(&text, kind);
        let id = ids
            .first()
            .cloned()
            .unwrap_or_else(|| relative_slash_path(root, &file));
        sources.push(EventSource {
            kind: kind.to_string(),
            id,
            file: relative_slash_path(root, &file),
            outgoing,
        });
    }
    Ok(sources)
}

fn scan_on_action_sources(root: &Path) -> Result<Vec<EventSource>, String> {
    let mut sources = Vec::new();
    let dir = root.join("common").join("on_actions");
    if !dir.exists() {
        return Ok(sources);
    }
    for file in collect_files(&dir)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let names = collect_on_action_names(&text);
        let outgoing = extract_event_calls(&text);
        if names.is_empty() && outgoing.is_empty() {
            continue;
        }
        for name in names {
            sources.push(EventSource {
                kind: "on_action".to_string(),
                id: name,
                file: relative_slash_path(root, &file),
                outgoing: outgoing.clone(),
            });
        }
    }
    Ok(sources)
}

fn event_definition_blocks(text: &str) -> Vec<String> {
    let stripped = strip_comments(text);
    direct_child_blocks(&stripped)
        .into_iter()
        .filter(|(key, _)| matches!(key.as_str(), "country_event" | "news_event" | "state_event"))
        .map(|(_, block)| block)
        .collect()
}

fn extract_event_calls(text: &str) -> Vec<String> {
    let mut calls = Vec::new();
    for marker in ["country_event", "news_event", "state_event"] {
        let needle = format!("{marker} =");
        let mut rest = text;
        while let Some(pos) = rest.find(&needle) {
            rest = &rest[pos + needle.len()..];
            if let Some(id_pos) = rest.find("id") {
                let after_id = &rest[id_pos + 2..];
                if let Some(eq) = after_id.find('=') {
                    let id = take_route_identifier(&after_id[eq + 1..]);
                    if id.contains('.') && !calls.iter().any(|existing| existing == &id) {
                        calls.push(id);
                    }
                }
            }
        }
    }
    calls
}

fn extract_route_source_ids(text: &str, kind: &str) -> Vec<String> {
    let stripped = strip_comments(text);
    let block_names = match kind {
        "focus" => vec!["focus"],
        "decision" => vec!["decision"],
        "scripted_effect" => Vec::new(),
        _ => Vec::new(),
    };
    let mut ids = Vec::new();
    for block_name in block_names {
        for block in blocks_named(&stripped, block_name) {
            if let Some(id) = block_assignment(&block, "id") {
                ids.push(id.trim_matches('"').to_string());
            }
        }
    }
    if kind == "scripted_effect" {
        ids.extend(collect_direct_entries(&stripped));
    }
    ids.sort();
    ids.dedup();
    ids
}

fn collect_on_action_names(text: &str) -> Vec<String> {
    let mut names = collect_direct_entries(&strip_comments(text));
    names.retain(|name| name.starts_with("on_"));
    names.sort();
    names.dedup();
    names
}

fn collect_direct_entries(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(key) = assignment_key(line) else {
            continue;
        };
        if is_route_container_key(key) {
            continue;
        }
        if is_identifier_like(key) {
            out.push(key.to_string());
        }
    }
    out
}

fn take_route_identifier(text: &str) -> String {
    text.trim_start()
        .trim_start_matches('"')
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '.')
        .collect()
}

fn render_event_source_refs(sources: &[&EventSource]) -> String {
    sources
        .iter()
        .map(|source| {
            format!(
                "{{\"kind\": {}, \"id\": {}, \"file\": {}, \"outgoing\": {}}}",
                json_str(&source.kind),
                json_str(&source.id),
                json_str(&source.file),
                json_array(&source.outgoing)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn unindexed_trigger_keys(text: &str, index: &GameIndex) -> Vec<String> {
    if index.triggers.is_empty() {
        return Vec::new();
    }
    let mut unknown = BTreeSet::new();
    for block in blocks_named(&strip_comments(text), "trigger") {
        for line in block.lines() {
            let Some(key) = assignment_key(line) else {
                continue;
            };
            if is_route_container_key(key) || is_common_trigger_operator(key) {
                continue;
            }
            if !index.triggers.contains(key) {
                unknown.insert(key.to_string());
            }
        }
    }
    unknown.into_iter().collect()
}

fn is_route_container_key(key: &str) -> bool {
    matches!(
        key,
        "id" | "title"
            | "desc"
            | "picture"
            | "option"
            | "immediate"
            | "hidden_effect"
            | "trigger"
            | "complete_effect"
            | "completion_reward"
            | "available"
            | "visible"
            | "days"
            | "random_days"
            | "ai_chance"
            | "name"
    )
}

fn is_common_trigger_operator(key: &str) -> bool {
    matches!(
        key,
        "AND" | "OR" | "NOT" | "limit" | "ROOT" | "FROM" | "PREV" | "THIS" | "owner" | "controller"
    )
}

fn reachable_event_steps(nodes: &[EventNode], start: &str, max_steps: usize) -> Vec<String> {
    let by_id = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    if !by_id.contains_key(start) {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut queue = vec![start.to_string()];
    let mut steps = Vec::new();
    while let Some(id) = queue.pop() {
        if !seen.insert(id.clone()) || steps.len() >= max_steps {
            continue;
        }
        if let Some(node) = by_id.get(id.as_str()) {
            steps.push(format!(
                "{} -> [{}] ({})",
                node.id,
                node.outgoing.join(", "),
                node.file
            ));
            for next in &node.outgoing {
                queue.push(next.clone());
            }
        }
    }
    steps
}

fn render_event_chain_graph_json(root: &Path, nodes: &[EventNode]) -> String {
    let edges = nodes.iter().map(|node| node.outgoing.len()).sum::<usize>();
    let rows = nodes
        .iter()
        .map(|node| {
            format!(
                "{{\"id\": {}, \"file\": {}, \"outgoing\": {}}}",
                json_str(&node.id),
                json_str(&node.file),
                json_array(&node.outgoing)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": {},\n  \"ok\": true,\n  \"status\": {},\n  \"mod_root\": {},\n  \"node_count\": {},\n  \"edge_count\": {},\n  \"nodes\": [{}]\n}}\n",
        json_str("hoi4skill.event_chain_graph.v1"),
        json_str("graph_ready"),
        json_str(&root.display().to_string()),
        nodes.len(),
        edges,
        rows
    )
}
