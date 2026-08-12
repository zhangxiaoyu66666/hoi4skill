//! Searchable catalog for the complete local HOI4 documentation directory.

use crate::*;
use serde::Serialize;

#[derive(Serialize)]
struct DocumentationCatalog {
    schema: &'static str,
    game_root: String,
    documentation_root: String,
    query: Option<String>,
    documents_total: usize,
    markdown_files_total: usize,
    matches_total: usize,
    documents: Vec<DocumentationDocument>,
    matches: Vec<DocumentationMatch>,
}

#[derive(Serialize)]
struct DocumentationDocument {
    id: String,
    file: String,
    title: String,
    sections: usize,
    bytes: usize,
    role: &'static str,
}

#[derive(Serialize)]
struct DocumentationMatch {
    document: String,
    file: String,
    line: usize,
    heading: String,
    snippet: String,
}

pub(crate) fn cmd_documentation_catalog(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let documentation_root = game_root.join("documentation");
    if !documentation_root.is_dir() {
        return Err(format!(
            "{}: HOI4 documentation directory does not exist",
            documentation_root.display()
        ));
    }
    let query = value(&map, "query")
        .or_else(|| map.positionals.first().map(String::as_str))
        .map(str::trim)
        .filter(|query| !query.is_empty());
    let max_items = parse_usize_option(&map, "max-items", 200)?;
    let catalog = build_documentation_catalog(&game_root, query, max_items)?;
    let json = serde_json::to_string_pretty(&catalog)
        .map_err(|error| format!("serialize documentation catalog: {error}"))?;
    write_or_print(&format!("{json}\n"), value(&map, "output"))
}

fn build_documentation_catalog(
    game_root: &Path,
    query: Option<&str>,
    max_items: usize,
) -> Result<DocumentationCatalog, String> {
    let documentation_root = game_root.join("documentation");
    let mut files = collect_files(&documentation_root)?
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .collect::<Vec<_>>();
    files.sort();
    let mut documents = Vec::new();
    let mut matches = Vec::new();
    let query_lower = query.map(str::to_lowercase);
    for file in files {
        let text = read_utf8_lossy(&file)?;
        let relative = relative_slash_path(game_root, &file);
        let id = file
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("documentation")
            .to_string();
        let title = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
            .unwrap_or(&id)
            .to_string();
        let sections = text
            .lines()
            .filter(|line| line.trim_start().starts_with("## "))
            .count();
        documents.push(DocumentationDocument {
            id: id.clone(),
            file: relative.clone(),
            title,
            sections,
            bytes: text.len(),
            role: documentation_role(&id),
        });
        let Some(query_lower) = query_lower.as_deref() else {
            continue;
        };
        let mut heading = String::new();
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix('#') {
                heading = value.trim_start_matches('#').trim().to_string();
            }
            if matches.len() >= max_items || !trimmed.to_lowercase().contains(query_lower) {
                continue;
            }
            matches.push(DocumentationMatch {
                document: id.clone(),
                file: relative.clone(),
                line: line_idx + 1,
                heading: heading.clone(),
                snippet: documentation_snippet(trimmed),
            });
        }
    }
    Ok(DocumentationCatalog {
        schema: "hoi4skill.documentation_catalog.v1",
        game_root: game_root.display().to_string(),
        documentation_root: documentation_root.display().to_string(),
        query: query.map(str::to_string),
        documents_total: documents.len(),
        markdown_files_total: documents.len(),
        matches_total: matches.len(),
        documents,
        matches,
    })
}

fn documentation_role(id: &str) -> &'static str {
    match id {
        "effects_documentation" | "triggers_documentation" | "modifiers_documentation" => {
            "strict_code_index"
        }
        "loc_formatter_documentation" | "loc_objects_documentation" => "localisation_reference",
        "console_commands_documentation" => "console_reference",
        _ => "script_language_reference",
    }
}

fn documentation_snippet(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    let mut chars = value.chars();
    let snippet = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{snippet}…")
    } else {
        snippet
    }
}
