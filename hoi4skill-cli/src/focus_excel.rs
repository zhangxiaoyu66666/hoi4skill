//! Excel-drawn national-focus tree import.
//!
//! The importer treats the worksheet as a visual grid: every non-empty,
//! non-connector cell is a focus node, and blank rows/columns preserve spacing.
//! Excel columns are expanded to HOI4 `x` steps of 2 so adjacent columns do not
//! visually overlap in the national-focus UI.
//! It intentionally reads only cell values so AI-authored `.xlsx` sketches can
//! become normal HOI4 focus skeletons without relying on spreadsheet styling.

use calamine::{open_workbook_auto, Data, Reader};

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
    let cells = collect_excel_focus_cells(&range)?;
    focus_layout_from_excel_cells(cells, tag, prefix)
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

pub(crate) fn collect_excel_focus_cells(
    range: &calamine::Range<Data>,
) -> Result<Vec<ExcelFocusCell>, String> {
    let mut cells = Vec::new();
    for (row_index, row) in range.rows().enumerate() {
        for (column_index, cell) in row.iter().enumerate() {
            let Some(text) = excel_cell_text(cell) else {
                continue;
            };
            if is_excel_focus_connector(&text) {
                continue;
            }
            cells.push(parse_excel_focus_cell(row_index, column_index, &text)?);
        }
    }
    if cells.is_empty() {
        return Err("worksheet did not contain any focus cells".to_string());
    }
    Ok(cells)
}

pub(crate) fn focus_layout_from_excel_cells(
    cells: Vec<ExcelFocusCell>,
    tag: &str,
    prefix: &str,
) -> Result<FocusLayout, String> {
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
    let rows = excel_focus_rows(&focuses);
    Ok(FocusLayout {
        tree_id: format!(
            "{}_{}_focus_tree",
            sanitize_identifier_part(prefix, "focus"),
            tag_part
        ),
        rows,
        focuses,
        mutuals: Vec::new(),
    })
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
    if value.contains("互斥") || lowered.contains("exclusive") {
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
            "{{\"title\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"relative_position_id\": {}, \"relative_x\": {}, \"relative_y\": {}, \"prerequisite\": {}}}",
            json_str(&focus.title),
            json_str(&focus.id),
            json_optional_str(focus.icon.as_deref()),
            focus.x,
            focus.y,
            json_optional_str(focus.relative_position_id.as_deref()),
            json_optional_i64(focus.relative_x.map(i64::from)),
            json_optional_i64(focus.relative_y.map(i64::from)),
            json_array(&focus.prerequisite)
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
