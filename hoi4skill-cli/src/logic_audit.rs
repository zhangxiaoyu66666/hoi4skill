//! Gameplay-logic reachability checks for large mods.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_logic_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let changed_files = logic_audit_changed_files(&resolved.root, &map)?;
    if map.flags.contains("changed-only") && changed_files.is_empty() {
        return Err("--changed-only requires at least one --changed <path>".to_string());
    }
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let mut report = audit_logic(&resolved.root)?;
    if map.flags.contains("changed-only") {
        report.filter_changed(&changed_files);
    }
    let json = logic_audit_json(&resolved, &report, &changed_files, max_items);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Default)]
struct LogicAuditReport {
    focus_total: usize,
    focus_trees_total: usize,
    focus_roots_total: usize,
    event_total: usize,
    event_refs_total: usize,
    broken_focus_refs: Vec<LogicIssue>,
    cross_tree_focus_refs: Vec<LogicIssue>,
    asymmetric_mutual_exclusions: Vec<LogicIssue>,
    unreachable_focuses: Vec<LogicIssue>,
    empty_focus_trees: Vec<LogicIssue>,
    broken_event_refs: Vec<LogicIssue>,
    event_type_mismatches: Vec<LogicIssue>,
    potential_orphan_events: Vec<LogicIssue>,
    empty_event_options: Vec<LogicIssue>,
    event_cycle_issues: Vec<LogicIssue>,
    event_flag_sets_without_readers: Vec<LogicIssue>,
    event_flag_reads_without_local_sets: Vec<LogicIssue>,
    event_chain_edges: Vec<EventLogicRef>,
    event_entry_events: Vec<String>,
    event_terminal_events: Vec<String>,
    event_isolated_events: Vec<String>,
    event_cycles: Vec<Vec<String>>,
}

#[derive(Clone)]
struct LogicIssue {
    kind: String,
    id: String,
    target: Option<String>,
    files: Vec<String>,
    detail: String,
}

#[derive(Clone)]
struct FocusLogicNode {
    id: String,
    file: String,
    tree_id: String,
    prerequisites: Vec<String>,
    mutually_exclusive: Vec<String>,
    relative_position_id: Option<String>,
}

#[derive(Clone)]
struct EventLogicNode {
    id: String,
    file: String,
    event_type: String,
    is_triggered_only: bool,
    empty_option_indexes: Vec<usize>,
}

#[derive(Clone)]
struct EventLogicRef {
    id: String,
    file: String,
    event_type: String,
    source_event_id: Option<String>,
    context: String,
}

#[derive(Clone)]
struct EventFlagRef {
    flag: String,
    file: String,
    source_event_id: Option<String>,
    action: String,
}

impl LogicAuditReport {
    fn filter_changed(&mut self, changed_files: &[String]) {
        self.broken_focus_refs
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.cross_tree_focus_refs
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.asymmetric_mutual_exclusions
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.unreachable_focuses
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.empty_focus_trees
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.broken_event_refs
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.event_type_mismatches
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.potential_orphan_events
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.empty_event_options
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.event_cycle_issues
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.event_flag_sets_without_readers
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.event_flag_reads_without_local_sets
            .retain(|issue| logic_issue_touches_changed(issue, changed_files));
        self.event_chain_edges.retain(|edge| {
            changed_files
                .iter()
                .any(|changed| logic_file_touches_changed(&edge.file, changed))
        });
    }
}

fn audit_logic(root: &Path) -> Result<LogicAuditReport, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }
    let nodes = collect_focus_logic_nodes(root)?;
    let event_nodes = collect_event_logic_nodes(root)?;
    let event_refs = collect_event_logic_refs(root)?;
    let flag_refs = collect_event_flag_refs(root)?;
    let mut by_id = BTreeMap::<String, Vec<FocusLogicNode>>::new();
    let mut tree_ids = BTreeSet::new();
    for node in &nodes {
        by_id.entry(node.id.clone()).or_default().push(node.clone());
        if !node.tree_id.is_empty() {
            tree_ids.insert(node.tree_id.clone());
        }
    }

    let mut report = LogicAuditReport {
        focus_total: nodes.len(),
        focus_trees_total: tree_ids.len(),
        event_total: event_nodes.len(),
        event_refs_total: event_refs.len(),
        ..LogicAuditReport::default()
    };
    let mut children_by_parent = BTreeMap::<String, BTreeSet<String>>::new();
    let mut roots = BTreeSet::new();
    for node in &nodes {
        if node.prerequisites.is_empty() {
            roots.insert(node.id.clone());
        }
        for target in &node.prerequisites {
            children_by_parent
                .entry(target.clone())
                .or_default()
                .insert(node.id.clone());
            push_focus_ref_issues(
                node,
                target,
                "prerequisite",
                &by_id,
                &mut report.broken_focus_refs,
                &mut report.cross_tree_focus_refs,
            );
        }
        for target in &node.mutually_exclusive {
            push_focus_ref_issues(
                node,
                target,
                "mutually_exclusive",
                &by_id,
                &mut report.broken_focus_refs,
                &mut report.cross_tree_focus_refs,
            );
            if let Some(target_nodes) = by_id.get(target) {
                if !target_nodes.iter().any(|target_node| {
                    target_node
                        .mutually_exclusive
                        .iter()
                        .any(|id| id == &node.id)
                }) {
                    report.asymmetric_mutual_exclusions.push(LogicIssue {
                        kind: "asymmetric_mutual_exclusion".to_string(),
                        id: node.id.clone(),
                        target: Some(target.clone()),
                        files: vec![node.file.clone()],
                        detail: format!(
                            "focus {} declares mutually_exclusive with {}, but the reverse edge was not found",
                            node.id, target
                        ),
                    });
                }
            }
        }
        if let Some(target) = &node.relative_position_id {
            push_focus_ref_issues(
                node,
                target,
                "relative_position_id",
                &by_id,
                &mut report.broken_focus_refs,
                &mut report.cross_tree_focus_refs,
            );
        }
    }
    report.focus_roots_total = roots.len();
    for tree_id in &tree_ids {
        if !nodes.iter().any(|node| &node.tree_id == tree_id) {
            report.empty_focus_trees.push(LogicIssue {
                kind: "empty_focus_tree".to_string(),
                id: tree_id.clone(),
                target: None,
                files: Vec::new(),
                detail: format!("focus tree {tree_id} has no imported focus blocks"),
            });
        }
    }

    let reachable = reachable_focus_ids(&roots, &children_by_parent);
    if !nodes.is_empty() && roots.is_empty() {
        for node in &nodes {
            report.unreachable_focuses.push(LogicIssue {
                kind: "no_focus_root".to_string(),
                id: node.id.clone(),
                target: None,
                files: vec![node.file.clone()],
                detail: "no root focus without prerequisite was found; cycle or over-linked tree suspected"
                    .to_string(),
            });
        }
    } else {
        for node in &nodes {
            if !reachable.contains(&node.id) {
                report.unreachable_focuses.push(LogicIssue {
                    kind: "unreachable_focus".to_string(),
                    id: node.id.clone(),
                    target: None,
                    files: vec![node.file.clone()],
                    detail: format!(
                        "focus {} is not reachable from any focus without prerequisites",
                        node.id
                    ),
                });
            }
        }
    }
    audit_event_logic(&event_nodes, &event_refs, &flag_refs, &mut report);
    Ok(report)
}

fn audit_event_logic(
    event_nodes: &[EventLogicNode],
    event_refs: &[EventLogicRef],
    flag_refs: &[EventFlagRef],
    report: &mut LogicAuditReport,
) {
    let event_ids = event_nodes
        .iter()
        .map(|event| event.id.clone())
        .collect::<BTreeSet<_>>();
    let event_by_id = event_nodes
        .iter()
        .map(|event| (event.id.clone(), event))
        .collect::<BTreeMap<_, _>>();
    let mut incoming = BTreeMap::<String, BTreeSet<String>>::new();
    for event_ref in event_refs {
        if !event_ids.contains(&event_ref.id) {
            report.broken_event_refs.push(LogicIssue {
                kind: "broken_event_reference".to_string(),
                id: event_ref.id.clone(),
                target: Some(event_ref.id.clone()),
                files: vec![event_ref.file.clone()],
                detail: format!(
                    "{} in {} triggers missing event id {}",
                    event_ref.event_type, event_ref.file, event_ref.id
                ),
            });
        } else {
            if let Some(target_event) = event_by_id.get(&event_ref.id) {
                if target_event.event_type != event_ref.event_type {
                    report.event_type_mismatches.push(LogicIssue {
                        kind: "event_type_mismatch".to_string(),
                        id: event_ref
                            .source_event_id
                            .clone()
                            .unwrap_or_else(|| event_ref.id.clone()),
                        target: Some(event_ref.id.clone()),
                        files: vec![event_ref.file.clone(), target_event.file.clone()],
                        detail: format!(
                            "{} in {} triggers {}, but target event {} is defined as {}",
                            event_ref.event_type,
                            event_ref.file,
                            event_ref.id,
                            event_ref.id,
                            target_event.event_type
                        ),
                    });
                }
            }
            incoming
                .entry(event_ref.id.clone())
                .or_default()
                .insert(event_ref.file.clone());
        }
    }
    let mut outgoing = BTreeMap::<String, BTreeSet<String>>::new();
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for event_ref in event_refs {
        let Some(source) = event_ref.source_event_id.as_ref() else {
            continue;
        };
        if !event_ids.contains(&event_ref.id) {
            continue;
        }
        outgoing
            .entry(source.clone())
            .or_default()
            .insert(event_ref.id.clone());
        adjacency
            .entry(source.clone())
            .or_default()
            .insert(event_ref.id.clone());
        report.event_chain_edges.push(event_ref.clone());
    }
    for event in event_nodes {
        for option_index in &event.empty_option_indexes {
            report.empty_event_options.push(LogicIssue {
                kind: "empty_event_option".to_string(),
                id: event.id.clone(),
                target: Some(format!("option_{option_index}")),
                files: vec![event.file.clone()],
                detail: format!(
                    "{} {} option {} has no gameplay effect, tooltip effect, hidden effect, or follow-up event",
                    event.event_type, event.id, option_index
                ),
            });
        }
        if event.is_triggered_only && !incoming.contains_key(&event.id) {
            report.potential_orphan_events.push(LogicIssue {
                kind: "potential_orphan_event".to_string(),
                id: event.id.clone(),
                target: None,
                files: vec![event.file.clone()],
                detail: format!(
                    "{} {} is_triggered_only = yes and no local event trigger reference was found",
                    event.event_type, event.id
                ),
            });
        }
    }
    for event in event_nodes {
        let has_incoming = incoming.contains_key(&event.id);
        let has_outgoing = outgoing.contains_key(&event.id);
        if !has_incoming {
            report.event_entry_events.push(event.id.clone());
        }
        if !has_outgoing {
            report.event_terminal_events.push(event.id.clone());
        }
        if !has_incoming && !has_outgoing {
            report.event_isolated_events.push(event.id.clone());
        }
    }
    let adjacency = adjacency
        .into_iter()
        .map(|(source, targets)| (source, targets.into_iter().collect::<Vec<_>>()))
        .collect::<BTreeMap<_, _>>();
    report.event_cycles = find_event_chain_cycles(&adjacency);
    for cycle in &report.event_cycles {
        let files = cycle
            .iter()
            .filter_map(|id| event_nodes.iter().find(|event| &event.id == id))
            .map(|event| event.file.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        report.event_cycle_issues.push(LogicIssue {
            kind: "event_chain_cycle".to_string(),
            id: cycle.first().cloned().unwrap_or_default(),
            target: cycle.get(1).cloned(),
            files,
            detail: format!(
                "event chain contains a cycle {}; add delay/conditions or break the loop before release",
                cycle.join(" -> ")
            ),
        });
    }
    audit_event_flag_lifecycle(flag_refs, report);
}

fn collect_focus_logic_nodes(root: &Path) -> Result<Vec<FocusLogicNode>, String> {
    let mut nodes = Vec::new();
    for file in txt_files(root, "common/national_focus")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        let trees = direct_blocks_named(&text, "focus_tree");
        if trees.is_empty() {
            collect_focus_logic_blocks(
                &mut nodes,
                &rel,
                "",
                "",
                &direct_blocks_named(&text, "focus"),
            );
            continue;
        }
        for tree in trees {
            let tree_id = block_assignment(&tree, "id").unwrap_or_default();
            let country_tag = direct_blocks_named(&tree, "country")
                .first()
                .and_then(|block| block_assignment(block, "tag"))
                .unwrap_or_default();
            collect_focus_logic_blocks(
                &mut nodes,
                &rel,
                &tree_id,
                &country_tag,
                &direct_blocks_named(&tree, "focus"),
            );
        }
    }
    nodes.sort_by(|a, b| a.tree_id.cmp(&b.tree_id).then(a.id.cmp(&b.id)));
    Ok(nodes)
}

fn collect_focus_logic_blocks(
    nodes: &mut Vec<FocusLogicNode>,
    file: &str,
    tree_id: &str,
    _country_tag: &str,
    blocks: &[String],
) {
    for block in blocks {
        let Some(id) = block_assignment(block, "id") else {
            continue;
        };
        nodes.push(FocusLogicNode {
            id,
            file: file.to_string(),
            tree_id: tree_id.to_string(),
            prerequisites: wrapped_assignment_values(block, "prerequisite", "focus"),
            mutually_exclusive: wrapped_assignment_values(block, "mutually_exclusive", "focus"),
            relative_position_id: block_assignment(block, "relative_position_id"),
        });
    }
}

fn collect_event_logic_nodes(root: &Path) -> Result<Vec<EventLogicNode>, String> {
    let mut nodes = Vec::new();
    for file in txt_files(root, "events")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for event_type in ["country_event", "news_event", "state_event"] {
            for block in direct_blocks_named(&text, event_type) {
                let Some(id) = block_assignment(&block, "id") else {
                    continue;
                };
                nodes.push(EventLogicNode {
                    id,
                    file: rel.clone(),
                    event_type: event_type.to_string(),
                    is_triggered_only: block_assignment(&block, "is_triggered_only")
                        .is_some_and(|value| value.eq_ignore_ascii_case("yes")),
                    empty_option_indexes: empty_event_option_indexes(&block),
                });
            }
        }
    }
    nodes.sort_by(|a, b| a.file.cmp(&b.file).then(a.id.cmp(&b.id)));
    Ok(nodes)
}

fn empty_event_option_indexes(event_block: &str) -> Vec<usize> {
    direct_blocks_named(event_block, "option")
        .iter()
        .enumerate()
        .filter_map(|(idx, option)| (!event_option_has_result(option)).then_some(idx + 1))
        .collect()
}

fn event_option_has_result(option: &str) -> bool {
    for key in direct_block_keys(option) {
        if !is_event_option_meta_key(&key) {
            return true;
        }
        if matches!(key.as_str(), "hidden_effect" | "effect_tooltip" | "tooltip") {
            if direct_blocks_named(option, &key)
                .iter()
                .any(|block| !block.trim().is_empty())
            {
                return true;
            }
        }
    }
    direct_assignment_keys(option)
        .into_iter()
        .any(|key| !is_event_option_meta_key(&key))
}

fn is_event_option_meta_key(key: &str) -> bool {
    matches!(
        key,
        "name"
            | "trigger"
            | "ai_chance"
            | "hidden_effect"
            | "effect_tooltip"
            | "tooltip"
            | "original_recipient_only"
            | "show_as_tooltip"
            | "custom_effect_tooltip"
    )
}

fn direct_assignment_keys(block: &str) -> Vec<String> {
    let bytes = block.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut escape = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_quote {
            if ch == '"' && !escape {
                in_quote = false;
            }
            if escape {
                escape = false;
            } else {
                escape = ch == '\\';
            }
            i += 1;
            continue;
        }
        if ch == '"' {
            in_quote = true;
            escape = false;
            i += 1;
            continue;
        }
        if ch == '{' {
            depth += 1;
            i += 1;
            continue;
        }
        if ch == '}' {
            depth = (depth - 1).max(0);
            i += 1;
            continue;
        }
        if depth == 0 && is_identifier_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_identifier_byte(bytes[i]) {
                i += 1;
            }
            let key = &block[start..i];
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] != b'{' {
                    keys.push(key.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
    keys
}

fn collect_event_logic_refs(root: &Path) -> Result<Vec<EventLogicRef>, String> {
    let mut refs = Vec::new();
    for file in collect_files(root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        if rel.starts_with("events/") {
            collect_event_logic_refs_from_event_file(&mut refs, &rel, &text);
            continue;
        }
        for event_type in ["country_event", "news_event", "state_event"] {
            for block in blocks_named(&text, event_type) {
                let Some(id) = block_assignment(&block, "id") else {
                    continue;
                };
                if looks_like_event_definition_block(&block) {
                    continue;
                }
                refs.push(EventLogicRef {
                    id,
                    file: rel.clone(),
                    event_type: event_type.to_string(),
                    source_event_id: None,
                    context: "external".to_string(),
                });
            }
        }
    }
    refs.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.source_event_id.cmp(&b.source_event_id))
            .then(a.context.cmp(&b.context))
            .then(a.id.cmp(&b.id))
    });
    refs.dedup_by(|a, b| {
        a.id == b.id
            && a.file == b.file
            && a.event_type == b.event_type
            && a.source_event_id == b.source_event_id
            && a.context == b.context
    });
    Ok(refs)
}

fn collect_event_flag_refs(root: &Path) -> Result<Vec<EventFlagRef>, String> {
    let mut refs = Vec::new();
    for file in collect_files(root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        if rel.starts_with("events/") {
            collect_event_flag_refs_from_event_file(&mut refs, &rel, &text);
        }
        collect_event_flag_refs_from_text(&mut refs, &rel, None, "project", &text);
    }
    refs.sort_by(|a, b| {
        a.flag
            .cmp(&b.flag)
            .then(a.action.cmp(&b.action))
            .then(a.file.cmp(&b.file))
            .then(a.source_event_id.cmp(&b.source_event_id))
    });
    refs.dedup_by(|a, b| {
        a.flag == b.flag
            && a.file == b.file
            && a.source_event_id == b.source_event_id
            && a.action == b.action
    });
    Ok(refs)
}

fn collect_event_flag_refs_from_event_file(refs: &mut Vec<EventFlagRef>, file: &str, text: &str) {
    for event_type in ["country_event", "news_event", "state_event"] {
        for block in direct_blocks_named(text, event_type) {
            let Some(source_id) = block_assignment(&block, "id") else {
                continue;
            };
            collect_event_flag_refs_from_text(refs, file, Some(&source_id), "event", &block);
        }
    }
}

fn collect_event_flag_refs_from_text(
    refs: &mut Vec<EventFlagRef>,
    file: &str,
    source_event_id: Option<&str>,
    context: &str,
    text: &str,
) {
    for (key, action) in [
        ("set_country_flag", "set"),
        ("set_global_flag", "set"),
        ("has_country_flag", "read"),
        ("has_global_flag", "read"),
        ("clr_country_flag", "clear"),
        ("clr_global_flag", "clear"),
    ] {
        for flag in assignment_values_in_text(text, key) {
            if !is_event_lifecycle_flag(&flag) {
                continue;
            }
            refs.push(EventFlagRef {
                flag,
                file: file.to_string(),
                source_event_id: source_event_id.map(str::to_string),
                action: format!("{context}:{action}:{key}"),
            });
        }
    }
}

fn is_event_lifecycle_flag(flag: &str) -> bool {
    !flag.is_empty()
        && flag.len() <= 128
        && flag
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
}

fn audit_event_flag_lifecycle(flag_refs: &[EventFlagRef], report: &mut LogicAuditReport) {
    let mut sets = BTreeMap::<String, Vec<&EventFlagRef>>::new();
    let mut consumers = BTreeMap::<String, Vec<&EventFlagRef>>::new();
    let mut event_reads = BTreeMap::<String, Vec<&EventFlagRef>>::new();
    for flag_ref in flag_refs {
        if flag_ref.action.contains(":set:") {
            sets.entry(flag_ref.flag.clone())
                .or_default()
                .push(flag_ref);
        } else if flag_ref.action.contains(":read:") || flag_ref.action.contains(":clear:") {
            consumers
                .entry(flag_ref.flag.clone())
                .or_default()
                .push(flag_ref);
            if flag_ref.source_event_id.is_some() && flag_ref.action.contains(":read:") {
                event_reads
                    .entry(flag_ref.flag.clone())
                    .or_default()
                    .push(flag_ref);
            }
        }
    }
    for (flag, set_refs) in &sets {
        let event_sets = set_refs
            .iter()
            .copied()
            .filter(|flag_ref| flag_ref.source_event_id.is_some())
            .collect::<Vec<_>>();
        if event_sets.is_empty() || consumers.contains_key(flag) {
            continue;
        }
        let files = event_sets
            .iter()
            .map(|flag_ref| flag_ref.file.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let ids = event_sets
            .iter()
            .filter_map(|flag_ref| flag_ref.source_event_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        report.event_flag_sets_without_readers.push(LogicIssue {
            kind: "event_flag_set_without_reader".to_string(),
            id: ids
                .first()
                .cloned()
                .unwrap_or_else(|| "<event>".to_string()),
            target: Some(flag.clone()),
            files,
            detail: format!(
                "event chain sets flag `{flag}`, but no has_* or clr_* reference to that flag was found in the local mod"
            ),
        });
    }
    for (flag, read_refs) in &event_reads {
        if sets.contains_key(flag) {
            continue;
        }
        let files = read_refs
            .iter()
            .map(|flag_ref| flag_ref.file.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let ids = read_refs
            .iter()
            .filter_map(|flag_ref| flag_ref.source_event_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        report.event_flag_reads_without_local_sets.push(LogicIssue {
            kind: "event_flag_read_without_local_set".to_string(),
            id: ids
                .first()
                .cloned()
                .unwrap_or_else(|| "<event>".to_string()),
            target: Some(flag.clone()),
            files,
            detail: format!(
                "event chain reads flag `{flag}`, but no local set_country_flag/set_global_flag writer was found"
            ),
        });
    }
}

fn collect_event_logic_refs_from_event_file(refs: &mut Vec<EventLogicRef>, file: &str, text: &str) {
    for source_type in ["country_event", "news_event", "state_event"] {
        for block in direct_blocks_named(text, source_type) {
            let Some(source_id) = block_assignment(&block, "id") else {
                continue;
            };
            collect_event_logic_refs_from_context(
                refs,
                file,
                Some(&source_id),
                "immediate",
                &direct_blocks_named(&block, "immediate").join("\n"),
            );
            collect_event_logic_refs_from_context(
                refs,
                file,
                Some(&source_id),
                "event hidden_effect",
                &direct_blocks_named(&block, "hidden_effect").join("\n"),
            );
            for after in direct_blocks_named(&block, "after") {
                collect_direct_event_logic_refs_from_context(
                    refs,
                    file,
                    Some(&source_id),
                    "after",
                    &after,
                );
                collect_event_logic_refs_from_context(
                    refs,
                    file,
                    Some(&source_id),
                    "after hidden_effect",
                    &direct_blocks_named(&after, "hidden_effect").join("\n"),
                );
            }
            for option in direct_blocks_named(&block, "option") {
                collect_direct_event_logic_refs_from_context(
                    refs,
                    file,
                    Some(&source_id),
                    "option",
                    &option,
                );
                collect_event_logic_refs_from_context(
                    refs,
                    file,
                    Some(&source_id),
                    "option hidden_effect",
                    &direct_blocks_named(&option, "hidden_effect").join("\n"),
                );
                collect_event_logic_refs_from_context(
                    refs,
                    file,
                    Some(&source_id),
                    "option effect_tooltip",
                    &direct_blocks_named(&option, "effect_tooltip").join("\n"),
                );
            }
        }
    }
}

fn collect_direct_event_logic_refs_from_context(
    refs: &mut Vec<EventLogicRef>,
    file: &str,
    source_event_id: Option<&str>,
    context: &str,
    text: &str,
) {
    if text.trim().is_empty() {
        return;
    }
    for event_type in ["country_event", "news_event", "state_event"] {
        for block in direct_blocks_named(text, event_type) {
            let Some(id) = block_assignment(&block, "id") else {
                continue;
            };
            if looks_like_event_definition_block(&block) {
                continue;
            }
            refs.push(EventLogicRef {
                id,
                file: file.to_string(),
                event_type: event_type.to_string(),
                source_event_id: source_event_id.map(str::to_string),
                context: context.to_string(),
            });
        }
    }
}

fn collect_event_logic_refs_from_context(
    refs: &mut Vec<EventLogicRef>,
    file: &str,
    source_event_id: Option<&str>,
    context: &str,
    text: &str,
) {
    if text.trim().is_empty() {
        return;
    }
    for event_type in ["country_event", "news_event", "state_event"] {
        for block in blocks_named(text, event_type) {
            let Some(id) = block_assignment(&block, "id") else {
                continue;
            };
            if looks_like_event_definition_block(&block) {
                continue;
            }
            refs.push(EventLogicRef {
                id,
                file: file.to_string(),
                event_type: event_type.to_string(),
                source_event_id: source_event_id.map(str::to_string),
                context: context.to_string(),
            });
        }
    }
}

fn looks_like_event_definition_block(block: &str) -> bool {
    block_assignment(block, "title").is_some()
        || block_assignment(block, "desc").is_some()
        || block_assignment(block, "is_triggered_only").is_some()
        || !direct_blocks_named(block, "option").is_empty()
}

fn push_focus_ref_issues(
    node: &FocusLogicNode,
    target: &str,
    relation: &str,
    by_id: &BTreeMap<String, Vec<FocusLogicNode>>,
    broken: &mut Vec<LogicIssue>,
    cross_tree: &mut Vec<LogicIssue>,
) {
    let Some(target_nodes) = by_id.get(target) else {
        broken.push(LogicIssue {
            kind: "broken_focus_reference".to_string(),
            id: node.id.clone(),
            target: Some(target.to_string()),
            files: vec![node.file.clone()],
            detail: format!(
                "focus {} has {} reference to missing focus {}",
                node.id, relation, target
            ),
        });
        return;
    };
    if !node.tree_id.is_empty()
        && target_nodes
            .iter()
            .all(|target_node| target_node.tree_id != node.tree_id)
    {
        cross_tree.push(LogicIssue {
            kind: "cross_tree_focus_reference".to_string(),
            id: node.id.clone(),
            target: Some(target.to_string()),
            files: vec![node.file.clone()],
            detail: format!(
                "focus {} has {} reference to {} outside focus tree {}",
                node.id, relation, target, node.tree_id
            ),
        });
    }
}

fn reachable_focus_ids(
    roots: &BTreeSet<String>,
    children_by_parent: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut queue = roots.iter().cloned().collect::<Vec<_>>();
    while let Some(id) = queue.pop() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        if let Some(children) = children_by_parent.get(&id) {
            queue.extend(children.iter().cloned());
        }
    }
    reachable
}

fn logic_audit_json(
    resolved: &ModRootResolution,
    report: &LogicAuditReport,
    changed_files: &[String],
    max_items: usize,
) -> String {
    let issue_count = report.broken_focus_refs.len()
        + report.cross_tree_focus_refs.len()
        + report.asymmetric_mutual_exclusions.len()
        + report.unreachable_focuses.len()
        + report.empty_focus_trees.len()
        + report.broken_event_refs.len()
        + report.event_type_mismatches.len()
        + report.potential_orphan_events.len()
        + report.empty_event_options.len()
        + report.event_cycle_issues.len()
        + report.event_flag_sets_without_readers.len()
        + report.event_flag_reads_without_local_sets.len();
    format!(
        "{{\n  \"schema\": \"hoi4skill.logic_audit.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"ok\": {},\n  \"focus_total\": {},\n  \"focus_trees_total\": {},\n  \"focus_roots_total\": {},\n  \"event_total\": {},\n  \"event_refs_total\": {},\n  \"event_chain_edge_count\": {},\n  \"event_chain_entry_count\": {},\n  \"event_chain_terminal_count\": {},\n  \"event_chain_isolated_count\": {},\n  \"event_chain_cycle_count\": {},\n  \"issue_count\": {},\n  \"changed_files\": {},\n  \"broken_focus_refs_count\": {},\n  \"cross_tree_focus_refs_count\": {},\n  \"asymmetric_mutual_exclusions_count\": {},\n  \"unreachable_focuses_count\": {},\n  \"empty_focus_trees_count\": {},\n  \"broken_event_refs_count\": {},\n  \"event_type_mismatches_count\": {},\n  \"potential_orphan_events_count\": {},\n  \"empty_event_options_count\": {},\n  \"event_cycle_issues_count\": {},\n  \"event_flag_sets_without_readers_count\": {},\n  \"event_flag_reads_without_local_sets_count\": {},\n  \"broken_focus_refs\": {},\n  \"cross_tree_focus_refs\": {},\n  \"asymmetric_mutual_exclusions\": {},\n  \"unreachable_focuses\": {},\n  \"empty_focus_trees\": {},\n  \"broken_event_refs\": {},\n  \"event_type_mismatches\": {},\n  \"potential_orphan_events\": {},\n  \"empty_event_options\": {},\n  \"event_cycle_issues\": {},\n  \"event_flag_sets_without_readers\": {},\n  \"event_flag_reads_without_local_sets\": {},\n  \"event_chain_edges\": {},\n  \"event_chain_entry_events\": {},\n  \"event_chain_terminal_events\": {},\n  \"event_chain_isolated_events\": {},\n  \"event_chain_cycles\": {},\n  \"suggested_commands\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_bool(issue_count == 0),
        report.focus_total,
        report.focus_trees_total,
        report.focus_roots_total,
        report.event_total,
        report.event_refs_total,
        report.event_chain_edges.len(),
        report.event_entry_events.len(),
        report.event_terminal_events.len(),
        report.event_isolated_events.len(),
        report.event_cycles.len(),
        issue_count,
        json_array(changed_files),
        report.broken_focus_refs.len(),
        report.cross_tree_focus_refs.len(),
        report.asymmetric_mutual_exclusions.len(),
        report.unreachable_focuses.len(),
        report.empty_focus_trees.len(),
        report.broken_event_refs.len(),
        report.event_type_mismatches.len(),
        report.potential_orphan_events.len(),
        report.empty_event_options.len(),
        report.event_cycle_issues.len(),
        report.event_flag_sets_without_readers.len(),
        report.event_flag_reads_without_local_sets.len(),
        logic_issues_json(&report.broken_focus_refs, max_items),
        logic_issues_json(&report.cross_tree_focus_refs, max_items),
        logic_issues_json(&report.asymmetric_mutual_exclusions, max_items),
        logic_issues_json(&report.unreachable_focuses, max_items),
        logic_issues_json(&report.empty_focus_trees, max_items),
        logic_issues_json(&report.broken_event_refs, max_items),
        logic_issues_json(&report.event_type_mismatches, max_items),
        logic_issues_json(&report.potential_orphan_events, max_items),
        logic_issues_json(&report.empty_event_options, max_items),
        logic_issues_json(&report.event_cycle_issues, max_items),
        logic_issues_json(&report.event_flag_sets_without_readers, max_items),
        logic_issues_json(&report.event_flag_reads_without_local_sets, max_items),
        event_logic_refs_json(&report.event_chain_edges, max_items),
        json_array(&report.event_entry_events),
        json_array(&report.event_terminal_events),
        json_array(&report.event_isolated_events),
        event_cycles_json(&report.event_cycles, max_items),
        json_array(&logic_audit_suggested_commands(changed_files))
    )
}

fn logic_issues_json(issues: &[LogicIssue], max_items: usize) -> String {
    format!(
        "[{}]",
        issues
            .iter()
            .take(max_items)
            .map(|issue| {
                format!(
                    "{{\"kind\": {}, \"id\": {}, \"target\": {}, \"files\": {}, \"detail\": {}}}",
                    json_str(&issue.kind),
                    json_str(&issue.id),
                    json_optional_str(issue.target.as_deref()),
                    json_array(&issue.files),
                    json_str(&issue.detail)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn event_logic_refs_json(refs: &[EventLogicRef], max_items: usize) -> String {
    format!(
        "[{}]",
        refs.iter()
            .take(max_items)
            .map(|event_ref| {
                format!(
                    "{{\"source_event_id\": {}, \"target_event_id\": {}, \"event_type\": {}, \"context\": {}, \"file\": {}}}",
                    json_optional_str(event_ref.source_event_id.as_deref()),
                    json_str(&event_ref.id),
                    json_str(&event_ref.event_type),
                    json_str(&event_ref.context),
                    json_str(&event_ref.file)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn event_cycles_json(cycles: &[Vec<String>], max_items: usize) -> String {
    format!(
        "[{}]",
        cycles
            .iter()
            .take(max_items)
            .map(|cycle| json_array(cycle))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn logic_audit_suggested_commands(changed_files: &[String]) -> Vec<String> {
    let mut commands = vec![
        "hoi4skill build-mod-index <mod-root> --output .hoi4skill/mod_index.json".to_string(),
        "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
    ];
    for changed in changed_files.iter().take(4) {
        commands.push(format!(
            "hoi4skill impact <mod-root> --changed {changed} --output .hoi4skill/impact_logic.json"
        ));
    }
    commands
}

fn logic_audit_changed_files(root: &Path, map: &ArgMap) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for key in ["changed", "changed-file"] {
        for raw in repeated_values(map, key) {
            let path = PathBuf::from(raw);
            let rel = if path.is_absolute() {
                relative_slash_path(root, &path)
            } else {
                slash_path(&path)
            };
            if !files.iter().any(|item| item == &rel) {
                files.push(rel);
            }
        }
    }
    Ok(files)
}

fn logic_issue_touches_changed(issue: &LogicIssue, changed_files: &[String]) -> bool {
    issue.files.iter().any(|file| {
        changed_files
            .iter()
            .any(|changed| logic_file_touches_changed(file, changed))
    })
}

fn logic_file_touches_changed(file: &str, changed: &str) -> bool {
    file == changed || file.starts_with(changed) || changed.starts_with(file)
}
