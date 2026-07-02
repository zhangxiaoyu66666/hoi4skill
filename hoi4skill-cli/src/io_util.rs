//! Shared filesystem and path helpers used by the CLI commands.
//!
//! This module is deliberately small: command code decides what to read or
//! write, while these helpers keep path normalization and recursive traversal
//! behavior consistent.

use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader};
use zip::ZipArchive;

pub(crate) fn write_if_missing(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(true)
}

pub(crate) fn relative_slash_path(root: &Path, path: &Path) -> String {
    slash_path(path.strip_prefix(root).unwrap_or(path))
}

pub(crate) fn write_or_print(text: &str, output: Option<&str>) -> Result<(), String> {
    if let Some(path) = output {
        let path = normalize_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
        println!("{}", path.display());
    } else {
        print!("{text}");
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn normalize_path(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let p = if p.is_absolute() {
        p
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(p)
    };
    Ok(p)
}

pub(crate) fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    collect_files_inner(root, &mut out)?;
    Ok(out)
}

fn collect_files_inner(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| format!("read dir {}: {e}", root.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_inner(&path, out)?;
        } else if path.is_file() {
            out.push(path);
        }
    }
    Ok(())
}

pub(crate) fn read_utf8_lossy(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_start_matches('\u{feff}')
        .to_string())
}

pub(crate) fn read_text_document(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "docx" {
        return read_docx_text(path);
    }
    if matches!(extension.as_str(), "xlsx" | "xls" | "xlsm" | "xlsb" | "ods") {
        return read_spreadsheet_text(path);
    }
    read_utf8_lossy(path)
}

pub(crate) fn read_spreadsheet_text(path: &Path) -> Result<String, String> {
    let mut workbook =
        open_workbook_auto(path).map_err(|e| format!("open workbook {}: {e}", path.display()))?;
    let mut out = String::new();
    for sheet in workbook.sheet_names().to_vec() {
        let range = workbook
            .worksheet_range(&sheet)
            .map_err(|e| format!("read worksheet `{sheet}` from {}: {e}", path.display()))?;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("# sheet: {sheet}\n"));
        for row in range.rows() {
            let mut cells = row.iter().map(spreadsheet_cell_text).collect::<Vec<_>>();
            while cells.last().is_some_and(|cell| cell.is_empty()) {
                cells.pop();
            }
            if cells.iter().all(|cell| cell.is_empty()) {
                continue;
            }
            out.push_str(&cells.join("\t"));
            out.push('\n');
        }
    }
    Ok(out.trim().to_string())
}

fn spreadsheet_cell_text(cell: &Data) -> String {
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
        Data::Empty => return String::new(),
    };
    text.trim().to_string()
}

pub(crate) fn read_docx_text(path: &Path) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| format!("open docx {}: {e}", path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("open docx {} as zip: {e}", path.display()))?;
    let mut document = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("read word/document.xml from {}: {e}", path.display()))?;
    let mut xml = String::new();
    document
        .read_to_string(&mut xml)
        .map_err(|e| format!("read docx xml {}: {e}", path.display()))?;
    Ok(extract_docx_document_text(&xml))
}

pub(crate) fn extract_docx_document_text(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    let mut table_depth = 0usize;
    let mut in_table_cell = false;
    while let Some(start) = rest.find('<') {
        let text_before_tag = &rest[..start];
        if !text_before_tag.trim().is_empty() {
            out.push_str(&decode_xml_text(text_before_tag));
        }
        rest = &rest[start..];
        let Some(tag_end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..=tag_end];
        rest = &rest[tag_end + 1..];
        if tag.starts_with("<w:tab") {
            out.push('\t');
        } else if tag.starts_with("<w:br") {
            out.push('\n');
        } else if tag.starts_with("<w:tbl") {
            table_depth += 1;
        } else if tag.starts_with("</w:tbl") {
            table_depth = table_depth.saturating_sub(1);
            push_docx_row_separator(&mut out);
        } else if tag.starts_with("<w:tc") {
            in_table_cell = table_depth > 0;
        } else if tag.starts_with("</w:tc") {
            in_table_cell = false;
            push_docx_cell_separator(&mut out);
        } else if tag.starts_with("</w:tr") {
            push_docx_row_separator(&mut out);
        } else if tag.starts_with("</w:p") {
            if in_table_cell {
                push_docx_cell_paragraph_separator(&mut out);
            } else {
                out.push('\n');
            }
        } else if is_docx_text_tag(tag) {
            let Some(text_end) = rest.find("</w:t>") else {
                break;
            };
            out.push_str(&decode_xml_text(&rest[..text_end]));
            rest = &rest[text_end + "</w:t>".len()..];
        }
    }
    out.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn is_docx_text_tag(tag: &str) -> bool {
    tag.starts_with("<w:t>") || tag.starts_with("<w:t ")
}

fn push_docx_cell_paragraph_separator(out: &mut String) {
    if !out.ends_with(['\n', '\t', ' ']) {
        out.push(' ');
    }
}

fn push_docx_cell_separator(out: &mut String) {
    trim_docx_cell_tail(out);
    out.push('\t');
}

fn push_docx_row_separator(out: &mut String) {
    trim_docx_row_tail(out);
    if !out.ends_with('\n') {
        out.push('\n');
    }
}

fn trim_docx_cell_tail(out: &mut String) {
    while out.ends_with([' ', '\n']) {
        out.pop();
    }
}

fn trim_docx_row_tail(out: &mut String) {
    while out.ends_with([' ', '\t', '\n']) {
        out.pop();
    }
}

pub(crate) fn decode_xml_text(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
