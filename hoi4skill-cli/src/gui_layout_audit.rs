//! Standalone GUI layout audit for existing HOI4 Mods.
//!
//! This audit complements generated-GUI checks. It reports only bounds that
//! can be proven from explicit numeric rectangles, flags the known building
//! ordering hazard, and can import actual engine clipping diagnostics.

use crate::*;

#[derive(Clone, Debug)]
struct LayoutNode {
    control_type: String,
    name: Option<String>,
    line: usize,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<i32>,
    height: Option<i32>,
    children: Vec<LayoutNode>,
}

#[derive(Clone, Debug)]
struct GuiLayoutFinding {
    classification: &'static str,
    code: &'static str,
    file: String,
    line: Option<usize>,
    object: Option<String>,
    detail: String,
}

pub(crate) fn cmd_gui_layout_audit(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "gui-layout-audit requires a mod root or --mod-root <path>".to_string())?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let runtime_log = value(&map, "runtime-log")
        .or_else(|| value(&map, "error-log"))
        .map(normalize_path)
        .transpose()?;
    let findings = audit_gui_layout(&resolved.root, runtime_log.as_deref())?;
    let confirmed_count = findings
        .iter()
        .filter(|finding| finding.classification == "confirmed_missing")
        .count();
    let runtime_required_count = findings
        .iter()
        .filter(|finding| finding.classification == "runtime_layout_required")
        .count();
    let ok = confirmed_count == 0 && runtime_required_count == 0;
    let report = gui_layout_audit_json(
        &resolved,
        runtime_log.as_deref(),
        &findings,
        confirmed_count,
        runtime_required_count,
        ok,
        max_items,
    );
    write_or_print(&report, value(&map, "output"))?;
    if map.flags.contains("require-passed") && !ok {
        Err(
            "gui-layout-audit found confirmed runtime errors or unresolved layout hazards"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

fn audit_gui_layout(
    root: &Path,
    runtime_log: Option<&Path>,
) -> Result<Vec<GuiLayoutFinding>, String> {
    let mut findings = Vec::new();
    let interface_root = root.join("interface");
    if interface_root.is_dir() {
        for file in collect_files(&interface_root)? {
            if !file
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|extension| extension.eq_ignore_ascii_case("gui"))
            {
                continue;
            }
            let relative = relative_slash_path(root, &file);
            let text = strip_comments(&read_utf8_lossy(&file)?);
            let roots = parse_layout_nodes(&text);
            for node in &roots {
                collect_static_bounds_findings(node, None, &relative, &mut findings);
            }
            collect_dynamic_layout_hazards(&text, &relative, &mut findings);
        }
    }
    collect_building_order_hazards(root, &mut findings)?;
    if let Some(runtime_log) = runtime_log {
        let text = read_utf8_lossy(runtime_log)?;
        collect_runtime_clipping_findings(&text, runtime_log, &mut findings);
    }
    findings.sort_by(|left, right| {
        (
            left.classification,
            left.file.as_str(),
            left.line,
            left.code,
            left.object.as_deref(),
        )
            .cmp(&(
                right.classification,
                right.file.as_str(),
                right.line,
                right.code,
                right.object.as_deref(),
            ))
    });
    findings.dedup_by(|left, right| {
        left.classification == right.classification
            && left.code == right.code
            && left.file == right.file
            && left.line == right.line
            && left.object == right.object
            && left.detail == right.detail
    });
    Ok(findings)
}

fn parse_layout_nodes(text: &str) -> Vec<LayoutNode> {
    let mut roots = Vec::new();
    collect_layout_nodes(text, 1, &mut roots);
    roots
}

fn collect_layout_nodes(text: &str, base_line: usize, output: &mut Vec<LayoutNode>) {
    for (kind, range) in direct_block_ranges(text) {
        let line = base_line
            + text[..range.close.saturating_sub(range.content.len() + 1)]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count();
        let child_base_line = line;
        let mut children = Vec::new();
        collect_layout_nodes(&range.content, child_base_line, &mut children);
        if is_gui_layout_node_kind(&kind) {
            output.push(LayoutNode {
                control_type: kind,
                name: direct_assignment_string(&range.content, "name"),
                line,
                x: direct_pair_i32(&range.content, "position", "x"),
                y: direct_pair_i32(&range.content, "position", "y"),
                width: direct_pair_i32(&range.content, "size", "width")
                    .or_else(|| direct_assignment_i32(&range.content, "maxWidth")),
                height: direct_pair_i32(&range.content, "size", "height")
                    .or_else(|| direct_assignment_i32(&range.content, "maxHeight")),
                children,
            });
        } else {
            output.extend(children);
        }
    }
}

fn is_gui_layout_node_kind(kind: &str) -> bool {
    kind.ends_with("Type") || matches!(kind, "background" | "positionType")
}

fn direct_assignment_string(block: &str, key: &str) -> Option<String> {
    direct_scalar_assignment(block, key)
}

fn direct_assignment_i32(block: &str, key: &str) -> Option<i32> {
    direct_scalar_assignment(block, key)?.parse().ok()
}

fn direct_pair_i32(block: &str, wrapper: &str, key: &str) -> Option<i32> {
    direct_blocks_named(block, wrapper)
        .first()
        .and_then(|pair| block_assignment(pair, key))
        .and_then(|value| value.parse().ok())
}

fn direct_scalar_assignment(block: &str, wanted: &str) -> Option<String> {
    let bytes = block.as_bytes();
    let mut index = 0usize;
    let mut depth = 0i32;
    let mut quoted = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if byte == b'"' && !escaped {
                quoted = false;
            }
            escaped = byte == b'\\' && !escaped;
            if byte != b'\\' {
                escaped = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            quoted = true;
            index += 1;
            continue;
        }
        if byte == b'{' {
            depth += 1;
            index += 1;
            continue;
        }
        if byte == b'}' {
            depth = (depth - 1).max(0);
            index += 1;
            continue;
        }
        if depth == 0 && is_identifier_byte(byte) {
            let start = index;
            while index < bytes.len() && is_identifier_byte(bytes[index]) {
                index += 1;
            }
            let key = &block[start..index];
            let mut cursor = index;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
            if key == wanted && bytes.get(cursor) == Some(&b'=') {
                cursor += 1;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if bytes.get(cursor) == Some(&b'{') {
                    return None;
                }
                return Some(read_assignment_value(&block[cursor..]).to_string());
            }
            continue;
        }
        index += 1;
    }
    None
}

fn collect_static_bounds_findings(
    node: &LayoutNode,
    parent: Option<&LayoutNode>,
    file: &str,
    findings: &mut Vec<GuiLayoutFinding>,
) {
    if let Some(parent) = parent {
        if let Some(detail) = explicit_bounds_violation(node, parent) {
            findings.push(GuiLayoutFinding {
                classification: "confirmed_missing",
                code: "static_child_out_of_bounds",
                file: file.to_string(),
                line: Some(node.line),
                object: node.name.clone(),
                detail,
            });
        }
    }
    for child in &node.children {
        collect_static_bounds_findings(child, Some(node), file, findings);
    }
}

fn explicit_bounds_violation(child: &LayoutNode, parent: &LayoutNode) -> Option<String> {
    let (Some(x), Some(y), Some(width), Some(height), Some(parent_width), Some(parent_height)) = (
        child.x,
        child.y,
        child.width,
        child.height,
        parent.width,
        parent.height,
    ) else {
        return None;
    };
    if width <= 0 || height <= 0 || parent_width <= 0 || parent_height <= 0 {
        return None;
    }
    if x < 0
        || y < 0
        || x.saturating_add(width) > parent_width
        || y.saturating_add(height) > parent_height
    {
        Some(format!(
            "child {} rectangle x={x}, y={y}, width={width}, height={height} exceeds parent {} width={parent_width}, height={parent_height}",
            child
                .name
                .as_deref()
                .unwrap_or(child.control_type.as_str()),
            parent
                .name
                .as_deref()
                .unwrap_or(parent.control_type.as_str())
        ))
    } else {
        None
    }
}

fn collect_building_order_hazards(
    root: &Path,
    findings: &mut Vec<GuiLayoutFinding>,
) -> Result<(), String> {
    let buildings_root = root.join("common").join("buildings");
    if !buildings_root.is_dir() {
        return Ok(());
    }
    let mut shares_slots_entries = Vec::new();
    for file in collect_files(&buildings_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let relative = relative_slash_path(root, &file);
        let text = strip_comments(&read_utf8_lossy(&file)?);
        collect_shares_slots_entries(&text, &relative, &mut shares_slots_entries);
    }
    if shares_slots_entries.is_empty() {
        return Ok(());
    }
    let gui = root.join("interface").join("countryconstructionsview.gui");
    if !gui.is_file() {
        return Ok(());
    }
    let text = read_utf8_lossy(&gui)?;
    let has_fixed_overlay = blocks_named(&text, "containerWindowType")
        .into_iter()
        .any(|block| {
            block_assignment(&block, "name").as_deref() == Some("building_construction_speeds")
                && blocks_named(&block, "instantTextBoxType")
                    .into_iter()
                    .filter(|child| block_assignment(child, "position").is_none())
                    .count()
                    > 1
        })
        || (text.contains("building_construction_speeds") && text.contains("_speed"));
    if has_fixed_overlay {
        findings.push(GuiLayoutFinding {
            classification: "runtime_layout_required",
            code: "building_order_fixed_overlay",
            file: relative_slash_path(root, &gui),
            line: text
                .lines()
                .position(|line| line.contains("building_construction_speeds"))
                .map(|line| line + 1),
            object: Some("building_construction_speeds".to_string()),
            detail: format!(
                "building shares_slots metadata is present while the construction-speed overlay is position-based; verify effective building category order and clipping at runtime ({})",
                shares_slots_entries.into_iter().take(16).collect::<Vec<_>>().join(", ")
            ),
        });
    }
    Ok(())
}

fn collect_shares_slots_entries(text: &str, relative: &str, entries: &mut Vec<String>) {
    collect_shares_slots_entries_with_owner(text, relative, None, entries);
}

fn collect_shares_slots_entries_with_owner(
    text: &str,
    relative: &str,
    owner: Option<&str>,
    entries: &mut Vec<String>,
) {
    for (name, block) in direct_child_blocks(text) {
        let next_owner = if matches!(name.as_str(), "buildings" | "level_cap") {
            owner
        } else {
            Some(name.as_str())
        };
        if let Some(value) = direct_scalar_assignment(&block, "shares_slots") {
            entries.push(format!(
                "{}={value}@{relative}",
                next_owner.unwrap_or(name.as_str())
            ));
        }
        collect_shares_slots_entries_with_owner(&block, relative, next_owner, entries);
    }
}

fn collect_dynamic_layout_hazards(
    text: &str,
    relative: &str,
    findings: &mut Vec<GuiLayoutFinding>,
) {
    collect_dynamic_layout_hazards_in_block(text, relative, findings);
}

fn collect_dynamic_layout_hazards_in_block(
    text: &str,
    relative: &str,
    findings: &mut Vec<GuiLayoutFinding>,
) {
    for (kind, block) in direct_child_blocks(text) {
        let name = direct_assignment_string(&block, "name");
        if kind == "containerWindowType" && name.as_deref() == Some("possible_constructions") {
            let dynamic_size = direct_blocks_named(&block, "size")
                .first()
                .is_some_and(|size| {
                    ["width", "height"].into_iter().any(|key| {
                        block_assignment(size, key).is_some_and(|value| value.contains('%'))
                    })
                });
            if dynamic_size {
                findings.push(GuiLayoutFinding {
                    classification: "runtime_layout_required",
                    code: "dynamic_percent_size_requires_runtime",
                    file: relative.to_string(),
                    line: None,
                    object: name,
                    detail: "possible_constructions uses percentage sizing, so its resolved height and clipping cannot be proven statically; import an engine clipping log or runtime layout measurement".to_string(),
                });
            }
        }
        collect_dynamic_layout_hazards_in_block(&block, relative, findings);
    }
}

fn collect_runtime_clipping_findings(
    text: &str,
    runtime_log: &Path,
    findings: &mut Vec<GuiLayoutFinding>,
) {
    for (index, raw) in text.lines().enumerate() {
        let lower = raw.to_ascii_lowercase();
        if (lower.contains("outside") && lower.contains("clipp"))
            || lower.contains("objects outside")
        {
            findings.push(GuiLayoutFinding {
                classification: "confirmed_missing",
                code: "runtime_object_clipped",
                file: runtime_log.display().to_string(),
                line: Some(index + 1),
                object: runtime_log_object_name(raw),
                detail: raw.trim().to_string(),
            });
        }
    }
}

fn runtime_log_object_name(line: &str) -> Option<String> {
    let quoted = line
        .split('"')
        .nth(1)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    quoted.map(str::to_string).or_else(|| {
        ["possible_constructions", "building_construction_speeds"]
            .into_iter()
            .find(|name| line.contains(name))
            .map(str::to_string)
    })
}

fn gui_layout_audit_json(
    resolved: &ModRootResolution,
    runtime_log: Option<&Path>,
    findings: &[GuiLayoutFinding],
    confirmed_count: usize,
    runtime_required_count: usize,
    ok: bool,
    max_items: usize,
) -> String {
    let parser_gap_count = findings
        .iter()
        .filter(|finding| finding.classification == "parser_gap")
        .count();
    let items = findings
        .iter()
        .take(max_items)
        .map(|finding| {
            format!(
                "{{\"classification\": {}, \"code\": {}, \"file\": {}, \"line\": {}, \"object\": {}, \"detail\": {}}}",
                json_str(finding.classification),
                json_str(finding.code),
                json_str(&finding.file),
                finding.line.map(|line| line.to_string()).unwrap_or_else(|| "null".to_string()),
                json_optional_str(finding.object.as_deref()),
                json_str(&finding.detail),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.gui_layout_audit.v1\",\n  \"ok\": {},\n  \"status\": {},\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"runtime_log\": {},\n  \"confirmed_missing_count\": {},\n  \"parser_gap_count\": {},\n  \"runtime_layout_required_count\": {},\n  \"finding_count\": {},\n  \"findings\": [{}]\n}}\n",
        json_bool(ok),
        json_str(if ok { "passed" } else { "layout_attention_required" }),
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_optional_str(runtime_log.map(|path| path.display().to_string()).as_deref()),
        confirmed_count,
        parser_gap_count,
        runtime_required_count,
        findings.len(),
        items,
    )
}
