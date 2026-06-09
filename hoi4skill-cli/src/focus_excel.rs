//! Excel-drawn national-focus tree import.
//!
//! The importer treats the worksheet as a visual grid: every non-empty,
//! non-connector cell is a focus node, and blank rows/columns preserve spacing.
//! Excel columns are expanded to HOI4 `x` steps of 2 so adjacent columns do not
//! visually overlap in the national-focus UI.
//! It intentionally reads only cell values so AI-authored `.xlsx` sketches can
//! become normal HOI4 focus skeletons without relying on spreadsheet styling.

use calamine::{open_workbook_auto, Data, Reader};
use std::io::Read;
use zip::ZipArchive;

#[allow(unused_imports)]
use crate::*;

#[derive(Clone)]
pub(crate) struct ExcelFocusCell {
    pub(crate) row: usize,
    pub(crate) column: usize,
    pub(crate) title: String,
    pub(crate) id_hint: Option<String>,
    pub(crate) icon: Option<String>,
    pub(crate) completion_reward: Vec<String>,
}

pub(crate) struct ExcelFocusImport {
    pub(crate) cells: Vec<ExcelFocusCell>,
    pub(crate) mutual_markers: Vec<(usize, usize)>,
}

pub(crate) fn cmd_parse_focus_excel(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("focus");
    let sheet = value(&map, "sheet");
    let format = value(&map, "format").unwrap_or("focus-tree");
    let mut layout = read_focus_excel_layout(&input, sheet, tag, prefix)?;
    if let Some(tree_id) = value(&map, "tree-id") {
        layout.tree_id = tree_id.to_string();
    }

    let output = match normalise_focus_excel_format(format).as_str() {
        "json" => focus_excel_layout_json(&layout, &input, sheet, tag, prefix),
        "focus-tree" => render_focus_tree(&layout, tag),
        other => {
            return Err(format!(
                "unknown --format `{other}`; use focus-tree or json"
            ))
        }
    };
    write_or_print(&output, value(&map, "output"))
}

pub(crate) fn cmd_apply_focus_excel(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = normalize_path(&require_value(&map, "input")?)?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("focus");
    let sheet = value(&map, "sheet");
    let mut layout = read_focus_excel_layout(&input, sheet, tag, prefix)?;
    if let Some(tree_id) = value(&map, "tree-id") {
        layout.tree_id = tree_id.to_string();
    }

    let changed = apply_focus_layout_to_mod(&mod_root, &layout, tag, prefix)?;
    println!(
        "Applied Excel focus layout: {} focuses",
        layout.focuses.len()
    );
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

pub(crate) fn read_focus_excel_layout(
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> Result<FocusLayout, String> {
    let mut workbook =
        open_workbook_auto(input).map_err(|e| format!("open workbook {}: {e}", input.display()))?;
    let sheet_name = resolve_excel_sheet_name(&workbook, sheet)?;
    let range = workbook.worksheet_range(&sheet_name).map_err(|e| {
        format!(
            "read worksheet `{sheet_name}` from {}: {e}",
            input.display()
        )
    })?;
    let drawing_texts = read_excel_drawing_texts(input, &sheet_name)?;
    let imported = collect_excel_focus_cells_with_drawings(&range, &drawing_texts)?;
    focus_layout_from_excel_cells(imported, tag, prefix)
}

pub(crate) fn resolve_excel_sheet_name<R>(
    workbook: &calamine::Sheets<R>,
    requested: Option<&str>,
) -> Result<String, String>
where
    R: std::io::Read + std::io::Seek,
{
    let names = workbook.sheet_names();
    if names.is_empty() {
        return Err("workbook has no worksheets".to_string());
    }
    if let Some(requested) = requested {
        if names.iter().any(|name| name == requested) {
            return Ok(requested.to_string());
        }
        return Err(format!(
            "worksheet `{requested}` not found; available sheets: {}",
            names.join(", ")
        ));
    }
    Ok(names[0].clone())
}

pub(crate) fn collect_excel_focus_cells_with_drawings(
    range: &calamine::Range<Data>,
    drawing_texts: &[ExcelDrawingText],
) -> Result<ExcelFocusImport, String> {
    let mut raw_cells = BTreeMap::new();
    let (range_start_row, range_start_column) = range.start().unwrap_or((0, 0));
    for (row_index, row) in range.rows().enumerate() {
        for (column_index, cell) in row.iter().enumerate() {
            let Some(text) = excel_cell_text(cell) else {
                continue;
            };
            raw_cells.insert(
                (
                    range_start_row as usize + row_index,
                    range_start_column as usize + column_index,
                ),
                text,
            );
        }
    }
    for drawing in drawing_texts {
        raw_cells
            .entry((drawing.row, drawing.column))
            .and_modify(|cell| *cell = merge_excel_cell_and_drawing(cell, &drawing.text))
            .or_insert_with(|| drawing.text.clone());
    }

    let mut cells = Vec::new();
    let mut mutual_markers = Vec::new();
    for ((row_index, column_index), text) in raw_cells {
        if is_excel_mutual_marker(&text) {
            mutual_markers.push((row_index, column_index));
            continue;
        }
        if is_excel_focus_connector(&text) {
            continue;
        }
        cells.push(parse_excel_focus_cell(row_index, column_index, &text)?);
    }
    if cells.is_empty() {
        return Err("worksheet did not contain any focus cells".to_string());
    }
    Ok(ExcelFocusImport {
        cells,
        mutual_markers,
    })
}

pub(crate) fn is_excel_mutual_marker(text: &str) -> bool {
    let value = text.trim();
    matches!(value, "互斥" | "相互排斥")
        || matches!(
            value.to_ascii_lowercase().as_str(),
            "exclusive" | "mutually exclusive" | "mutual exclusion"
        )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExcelDrawingText {
    pub(crate) row: usize,
    pub(crate) column: usize,
    pub(crate) text: String,
}

pub(crate) fn merge_excel_cell_and_drawing(cell: &str, drawing: &str) -> String {
    let cell = cell.trim();
    let drawing = drawing.trim();
    if cell.is_empty() {
        return drawing.to_string();
    }
    if drawing.is_empty() || cell == drawing {
        return cell.to_string();
    }
    if looks_like_excel_focus_id(cell) {
        format!("{drawing}\nID: {cell}")
    } else {
        format!("{cell}\n{drawing}")
    }
}

pub(crate) fn looks_like_excel_focus_id(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        && (value.contains('_') || value.chars().any(|ch| ch.is_ascii_uppercase()))
}

pub(crate) fn read_excel_drawing_texts(
    input: &Path,
    sheet_name: &str,
) -> Result<Vec<ExcelDrawingText>, String> {
    let extension = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "xlsx" | "xlsm" | "xltx" | "xltm") {
        return Ok(Vec::new());
    }

    let file = fs::File::open(input).map_err(|e| format!("open {}: {e}", input.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("read OOXML archive {}: {e}", input.display()))?;
    let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml")?;
    let workbook_rels = read_zip_text(&mut archive, "xl/_rels/workbook.xml.rels")?;
    let sheet_rid = workbook_sheet_relationship_id(&workbook_xml, sheet_name)
        .ok_or_else(|| format!("worksheet `{sheet_name}` is missing from xl/workbook.xml"))?;
    let sheet_target = relationship_target(&workbook_rels, &sheet_rid)
        .ok_or_else(|| format!("worksheet `{sheet_name}` has no workbook relationship"))?;
    let sheet_path = resolve_ooxml_target("xl/workbook.xml", &sheet_target);
    let sheet_xml = read_zip_text(&mut archive, &sheet_path)?;
    let Some(drawing_rid) = worksheet_drawing_relationship_id(&sheet_xml) else {
        return Ok(Vec::new());
    };
    let sheet_rels_path = relationship_part_path(&sheet_path);
    let sheet_rels = read_zip_text(&mut archive, &sheet_rels_path)?;
    let drawing_target = relationship_target(&sheet_rels, &drawing_rid)
        .ok_or_else(|| format!("worksheet `{sheet_name}` drawing relationship is unresolved"))?;
    let drawing_path = resolve_ooxml_target(&sheet_path, &drawing_target);
    let drawing_xml = read_zip_text(&mut archive, &drawing_path)?;
    Ok(parse_drawing_anchor_texts(&drawing_xml))
}

pub(crate) fn read_zip_text<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<String, String> {
    let mut entry = archive
        .by_name(path)
        .map_err(|e| format!("read OOXML part `{path}`: {e}"))?;
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .map_err(|e| format!("decode OOXML part `{path}`: {e}"))?;
    Ok(text)
}

pub(crate) fn workbook_sheet_relationship_id(xml: &str, sheet_name: &str) -> Option<String> {
    xml_opening_tags(xml, "sheet").find_map(|tag| {
        (xml_attribute(tag, "name").as_deref() == Some(sheet_name))
            .then(|| xml_attribute(tag, "r:id"))
            .flatten()
    })
}

pub(crate) fn worksheet_drawing_relationship_id(xml: &str) -> Option<String> {
    xml_opening_tags(xml, "drawing").find_map(|tag| xml_attribute(tag, "r:id"))
}

pub(crate) fn relationship_target(xml: &str, relationship_id: &str) -> Option<String> {
    xml_opening_tags(xml, "Relationship").find_map(|tag| {
        (xml_attribute(tag, "Id").as_deref() == Some(relationship_id))
            .then(|| xml_attribute(tag, "Target"))
            .flatten()
    })
}

pub(crate) fn xml_opening_tags<'a>(
    xml: &'a str,
    local_name: &'a str,
) -> impl Iterator<Item = &'a str> {
    xml.match_indices('<').filter_map(move |(start, _)| {
        let rest = &xml[start..];
        let end = rest.find('>')?;
        let tag = &rest[..=end];
        let name_end = tag
            .find(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '/'))
            .unwrap_or(tag.len());
        let qualified = tag[1..name_end].trim();
        let actual_local = qualified.rsplit(':').next().unwrap_or(qualified);
        (actual_local == local_name).then_some(tag)
    })
}

pub(crate) fn xml_attribute(tag: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!("{name}={quote}");
        if let Some(attribute_start) = tag.find(&needle) {
            let start = attribute_start + needle.len();
            if let Some(relative_end) = tag[start..].find(quote) {
                let end = start + relative_end;
                return Some(decode_xml_text(&tag[start..end]));
            }
        }
    }
    None
}

pub(crate) fn relationship_part_path(part_path: &str) -> String {
    let (directory, file_name) = part_path.rsplit_once('/').unwrap_or(("", part_path));
    if directory.is_empty() {
        format!("_rels/{file_name}.rels")
    } else {
        format!("{directory}/_rels/{file_name}.rels")
    }
}

pub(crate) fn resolve_ooxml_target(source_part: &str, target: &str) -> String {
    let base = source_part
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    let combined = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else if base.is_empty() {
        target.to_string()
    } else {
        format!("{base}/{target}")
    };
    let mut parts = Vec::new();
    for part in combined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value),
        }
    }
    parts.join("/")
}

pub(crate) fn parse_drawing_anchor_texts(xml: &str) -> Vec<ExcelDrawingText> {
    let mut out = Vec::new();
    for anchor_name in ["twoCellAnchor", "oneCellAnchor", "absoluteAnchor"] {
        for anchor in xml_element_bodies(xml, anchor_name) {
            let Some(from) = xml_element_bodies(anchor, "from").next() else {
                continue;
            };
            let Some(column) = xml_first_text(from, "col").and_then(|value| value.parse().ok())
            else {
                continue;
            };
            let Some(row) = xml_first_text(from, "row").and_then(|value| value.parse().ok()) else {
                continue;
            };
            let text = xml_text_values(anchor, "t")
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                out.push(ExcelDrawingText { row, column, text });
            }
        }
    }
    out
}

pub(crate) fn xml_element_bodies<'a>(
    xml: &'a str,
    local_name: &'a str,
) -> impl Iterator<Item = &'a str> {
    let mut bodies = Vec::new();
    let mut offset = 0usize;
    while let Some(relative_start) = xml[offset..].find('<') {
        let start = offset + relative_start;
        let Some(open_end_relative) = xml[start..].find('>') else {
            break;
        };
        let open_end = start + open_end_relative;
        let open_tag = &xml[start..=open_end];
        let qualified = open_tag[1..]
            .split(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '/'))
            .next()
            .unwrap_or("");
        if qualified.rsplit(':').next().unwrap_or(qualified) != local_name {
            offset = open_end + 1;
            continue;
        }
        let close_tag = format!("</{qualified}>");
        let Some(close_relative) = xml[open_end + 1..].find(&close_tag) else {
            offset = open_end + 1;
            continue;
        };
        let close = open_end + 1 + close_relative;
        bodies.push(&xml[open_end + 1..close]);
        offset = close + close_tag.len();
    }
    bodies.into_iter()
}

pub(crate) fn xml_first_text(xml: &str, local_name: &str) -> Option<String> {
    xml_text_values(xml, local_name).into_iter().next()
}

pub(crate) fn xml_text_values(xml: &str, local_name: &str) -> Vec<String> {
    xml_element_bodies(xml, local_name)
        .map(decode_xml_text)
        .collect()
}

pub(crate) fn decode_xml_text(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub(crate) fn focus_layout_from_excel_cells(
    imported: ExcelFocusImport,
    tag: &str,
    prefix: &str,
) -> Result<FocusLayout, String> {
    let ExcelFocusImport {
        cells,
        mutual_markers,
    } = imported;
    if cells.is_empty() {
        return Err("worksheet did not contain any focus cells".to_string());
    }
    let tag_part = sanitize_identifier_part(tag, "TAG").to_ascii_uppercase();
    let min_row = cells.iter().map(|cell| cell.row).min().unwrap_or(0);
    let min_col = cells.iter().map(|cell| cell.column).min().unwrap_or(0);
    let mut sorted = cells;
    sorted.sort_by_key(|cell| (cell.row, cell.column));

    let mut used = BTreeSet::new();
    let mut focuses = Vec::new();
    for (index, cell) in sorted.iter().enumerate() {
        let fallback = format!("focus_{}_{}", cell.row - min_row, cell.column - min_col);
        let mut id = focus_identifier(&tag_part, &cell.title, cell.id_hint.as_deref(), &fallback);
        let base = id.clone();
        let mut n = 2;
        while used.contains(&id) {
            id = format!("{base}_{n}");
            n += 1;
        }
        used.insert(id.clone());
        focuses.push(FocusNode {
            title: cell.title.clone(),
            id,
            icon: cell.icon.clone(),
            x: ((cell.column - min_col) as i32) * 2,
            y: (cell.row - min_row) as i32,
            relative_position_id: None,
            relative_x: None,
            relative_y: None,
            row: cell.row - min_row,
            column: cell.column - min_col,
            prerequisite: Vec::new(),
            mutually_exclusive: Vec::new(),
            completion_reward: cell.completion_reward.clone(),
        });
        if index > 10_000 {
            return Err(
                "worksheet contains too many focus cells; split the tree into smaller sheets"
                    .to_string(),
            );
        }
    }

    ensure_focus_row_x_spacing(&mut focuses, 2);
    infer_excel_focus_parents(&mut focuses);
    let mutuals = apply_excel_mutual_markers(&mut focuses, &mutual_markers, min_row, min_col);
    let rows = excel_focus_rows(&focuses);
    Ok(FocusLayout {
        tree_id: format!(
            "{}_{}_focus_tree",
            sanitize_identifier_part(prefix, "focus"),
            tag_part
        ),
        rows,
        focuses,
        mutuals,
    })
}

pub(crate) fn apply_excel_mutual_markers(
    focuses: &mut [FocusNode],
    markers: &[(usize, usize)],
    min_row: usize,
    min_col: usize,
) -> Vec<(String, String, usize)> {
    let mut mutuals = Vec::new();
    for (absolute_row, absolute_column) in markers {
        let Some(row) = absolute_row.checked_sub(min_row) else {
            continue;
        };
        let marker_column = absolute_column.saturating_sub(min_col);
        let left = focuses
            .iter()
            .filter(|focus| focus.row == row && focus.column < marker_column)
            .max_by_key(|focus| focus.column)
            .map(|focus| focus.id.clone());
        let right = focuses
            .iter()
            .filter(|focus| focus.row == row && focus.column > marker_column)
            .min_by_key(|focus| focus.column)
            .map(|focus| focus.id.clone());
        let (Some(left), Some(right)) = (left, right) else {
            continue;
        };
        link_mutual(focuses, &left, &right);
        mutuals.push((left, right, row));
    }
    mutuals
}

pub(crate) fn infer_excel_focus_parents(focuses: &mut [FocusNode]) {
    for idx in 0..focuses.len() {
        let current_row = focuses[idx].row;
        if current_row == 0 {
            continue;
        }
        let parent = focuses
            .iter()
            .enumerate()
            .filter(|(candidate_idx, candidate)| {
                *candidate_idx != idx && candidate.row < current_row
            })
            .max_by_key(|(_, candidate)| candidate.row)
            .map(|(_, candidate)| candidate.row)
            .and_then(|nearest_row| {
                focuses
                    .iter()
                    .filter(|candidate| candidate.row == nearest_row)
                    .min_by_key(|candidate| {
                        (
                            (candidate.x - focuses[idx].x).abs(),
                            candidate.y,
                            candidate.x,
                        )
                    })
                    .map(|candidate| {
                        (
                            candidate.id.clone(),
                            focuses[idx].x - candidate.x,
                            focuses[idx].y - candidate.y,
                        )
                    })
            });
        if let Some((parent_id, dx, dy)) = parent {
            focuses[idx].prerequisite.push(parent_id.clone());
            focuses[idx].relative_position_id = Some(parent_id);
            focuses[idx].relative_x = Some(dx);
            focuses[idx].relative_y = Some(dy);
        }
    }
}

pub(crate) fn excel_focus_rows(focuses: &[FocusNode]) -> Vec<FocusRow> {
    let mut by_row: BTreeMap<usize, Vec<&FocusNode>> = BTreeMap::new();
    for focus in focuses {
        by_row.entry(focus.row).or_default().push(focus);
    }
    by_row
        .into_iter()
        .map(|(y, mut row_focuses)| {
            row_focuses.sort_by_key(|focus| focus.column);
            FocusRow {
                y,
                tokens: row_focuses
                    .iter()
                    .map(|focus| focus.title.clone())
                    .collect(),
                focus_ids: row_focuses.iter().map(|focus| focus.id.clone()).collect(),
            }
        })
        .collect()
}

pub(crate) fn parse_excel_focus_cell(
    row: usize,
    column: usize,
    raw: &str,
) -> Result<ExcelFocusCell, String> {
    let mut title = String::new();
    let mut id_hint = None;
    let mut icon = None;
    let mut reward = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(value) = field_value(line, &["id", "focus_id", "focus id", "国策id", "国策ID"])
        {
            id_hint = Some(sanitize_identifier_part(&value, ""));
            continue;
        }
        if let Some(value) = field_value(line, &["icon", "图标", "gfx"]) {
            if value.starts_with("GFX_") {
                icon = Some(value);
            } else {
                icon = Some(format!("GFX_{value}"));
            }
            continue;
        }
        if let Some(value) = field_value(line, &["completion_reward", "reward", "效果", "国策效果"])
        {
            reward.extend(focus_reward_lines_from_effects(&value));
            continue;
        }
        if let Some(value) = field_value(line, &["title", "name", "国策", "标题", "名称"]) {
            if title.is_empty() {
                title = value;
            }
            continue;
        }
        if title.is_empty() {
            title = line.to_string();
        }
    }

    if title.is_empty() {
        return Err(format!(
            "focus cell at row {}, column {} has metadata but no title",
            row + 1,
            column + 1
        ));
    }
    let (parsed_title, parsed_id_hint) = parse_focus_token(&title);
    if id_hint.is_none() {
        id_hint = parsed_id_hint;
    }
    Ok(ExcelFocusCell {
        row,
        column,
        title: parsed_title,
        id_hint,
        icon,
        completion_reward: reward,
    })
}

pub(crate) fn field_value(line: &str, keys: &[&str]) -> Option<String> {
    let (left, right) = split_excel_field(line)?;
    let left_key = normalise_field_key(left);
    keys.iter()
        .any(|key| normalise_field_key(key) == left_key)
        .then(|| right.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn split_excel_field(line: &str) -> Option<(&str, &str)> {
    for sep in ['：', ':', '='] {
        if let Some((left, right)) = line.split_once(sep) {
            return Some((left.trim(), right.trim()));
        }
    }
    None
}

pub(crate) fn normalise_field_key(value: &str) -> String {
    value
        .trim()
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '_' && *ch != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn excel_cell_text(cell: &Data) -> Option<String> {
    let text = match cell {
        Data::String(value) => value.clone(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::Error(value) => value.to_string(),
        Data::Empty => return None,
    };
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

pub(crate) fn is_excel_focus_connector(text: &str) -> bool {
    let value = text.trim();
    if value.is_empty() {
        return true;
    }
    let lowered = value.to_ascii_lowercase();
    let header_words = [
        "focus tree",
        "focus",
        "title",
        "name",
        "id",
        "国策树",
        "国策",
        "标题",
        "名称",
    ];
    if header_words.iter().any(|word| lowered == *word) {
        return true;
    }
    if is_excel_mutual_marker(value) {
        return true;
    }
    value.chars().all(|ch| {
        matches!(
            ch,
            '-' | '─'
                | '━'
                | '—'
                | '–'
                | '|'
                | '│'
                | '┃'
                | '+'
                | '＋'
                | '┌'
                | '┐'
                | '└'
                | '┘'
                | '├'
                | '┤'
                | '┬'
                | '┴'
                | '┼'
                | '↑'
                | '↓'
                | '←'
                | '→'
                | '↔'
                | '<'
                | '>'
                | '^'
                | 'v'
                | 'V'
                | ' '
        )
    })
}

pub(crate) fn normalise_focus_excel_format(format: &str) -> String {
    match format.trim().to_ascii_lowercase().as_str() {
        "txt" | "script" | "hoi4" | "focus" | "focus_tree" | "focus-tree" => {
            "focus-tree".to_string()
        }
        "json" | "report" => "json".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn focus_excel_layout_json(
    layout: &FocusLayout,
    input: &Path,
    sheet: Option<&str>,
    tag: &str,
    prefix: &str,
) -> String {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"schema\": \"hoi4skill.focus_excel.v1\",\n");
    out.push_str(&format!(
        "  \"source\": {},\n",
        json_str(&input.display().to_string())
    ));
    out.push_str(&format!("  \"sheet\": {},\n", json_optional_str(sheet)));
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tree_id\": {},\n", json_str(&layout.tree_id)));
    out.push_str("  \"focuses\": [\n");
    for (i, focus) in layout.focuses.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"title\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"relative_position_id\": {}, \"relative_x\": {}, \"relative_y\": {}, \"prerequisite\": {}, \"mutually_exclusive\": {}}}",
            json_str(&focus.title),
            json_str(&focus.id),
            json_optional_str(focus.icon.as_deref()),
            focus.x,
            focus.y,
            json_optional_str(focus.relative_position_id.as_deref()),
            json_optional_i64(focus.relative_x.map(i64::from)),
            json_optional_i64(focus.relative_y.map(i64::from)),
            json_array(&focus.prerequisite),
            json_array(&focus.mutually_exclusive)
        ));
    }
    out.push_str("\n  ],\n");
    out.push_str("  \"mutually_exclusive\": [\n");
    for (i, (left, right, row)) in layout.mutuals.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"left\": {}, \"right\": {}, \"row\": {row}}}",
            json_str(left),
            json_str(right)
        ));
    }
    out.push_str("\n  ],\n");
    out.push_str(&format!(
        "  \"focus_tree\": {}\n",
        json_str(&render_focus_tree(layout, tag))
    ));
    out.push_str("}\n");
    out
}
