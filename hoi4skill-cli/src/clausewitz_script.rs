//! Clausewitz-style block and assignment parsing shared across HOI4 features.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn direct_blocks_named(text: &str, name: &str) -> Vec<String> {
    direct_child_blocks(text)
        .into_iter()
        .filter_map(|(key, block)| (key == name).then_some(block))
        .collect()
}

pub(crate) fn direct_block_ranges(text: &str) -> Vec<(String, NamedBlockRange)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
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
            let key = &text[start..i];
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
            }
            if j < bytes.len() && bytes[j] == b'{' {
                if let Some((content, close)) = braced_content_at(text, j) {
                    out.push((
                        key.to_string(),
                        NamedBlockRange {
                            close,
                            end: close + 1,
                            content,
                        },
                    ));
                    i = close + 1;
                    continue;
                }
                break;
            }
            continue;
        }
        i += 1;
    }
    out
}

pub(crate) fn direct_child_blocks(text: &str) -> Vec<(String, String)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
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
            let key = &text[start..i];
            let mut j = i;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
            }
            if j < bytes.len() && bytes[j] == b'{' {
                if let Some((content, end)) = braced_content_at(text, j) {
                    out.push((key.to_string(), content));
                    i = end + 1;
                    continue;
                }
                break;
            }
            continue;
        }
        i += 1;
    }
    out
}

pub(crate) fn braced_content_at(text: &str, open_byte: usize) -> Option<(String, usize)> {
    if text.as_bytes().get(open_byte) != Some(&b'{') {
        return None;
    }
    let content_start = open_byte + 1;
    let mut depth = 1i32;
    let mut in_quote = false;
    let mut escape = false;
    for (offset, ch) in text[content_start..].char_indices() {
        if in_quote {
            if ch == '"' && !escape {
                in_quote = false;
            }
            if escape {
                escape = false;
            } else {
                escape = ch == '\\';
            }
            continue;
        }
        if ch == '"' {
            in_quote = true;
            escape = false;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                let end = content_start + offset;
                return Some((text[content_start..end].to_string(), end));
            }
        }
    }
    None
}

pub(crate) fn blocks_named(text: &str, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find(name) {
        let before_ok = idx == 0
            || rest[..idx]
                .chars()
                .last()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after_name = &rest[idx + name.len()..];
        let after_trimmed = after_name.trim_start();
        let after_ok = if let Some(after_eq) = after_trimmed.strip_prefix('=') {
            after_eq.trim_start().starts_with('{')
        } else {
            after_trimmed.starts_with('{')
        };
        if !before_ok || !after_ok {
            rest = after_name;
            continue;
        }
        rest = &rest[idx + name.len()..];
        if let Some(open) = rest.find('{') {
            let content_start = open + 1;
            let mut depth = 1;
            let mut end = None;
            for (i, ch) in rest[content_start..].char_indices() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
            }
            if let Some(end) = end {
                out.push(rest[content_start..content_start + end].to_string());
                rest = &rest[content_start + end + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    out
}

pub(crate) fn direct_block_keys(block: &str) -> Vec<String> {
    let bytes = block.as_bytes();
    let mut keys = Vec::new();
    let mut i = 0usize;
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut escape = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' && !escape {
            in_quote = !in_quote;
            i += 1;
            continue;
        }
        if in_quote {
            escape = ch == '\\' && !escape;
            if ch != '\\' {
                escape = false;
            }
            i += 1;
            continue;
        }
        escape = false;
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
            if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b'{') {
                keys.push(key.to_string());
            }
            continue;
        }
        i += 1;
    }
    keys
}

pub(crate) fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':')
}

pub(crate) fn block_assignment(block: &str, key: &str) -> Option<String> {
    find_assignment_in_text(block, key).map(str::to_string)
}

pub(crate) fn find_assignment_in_text<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut rest = text;
    while let Some(idx) = rest.find(key) {
        let before_ok = idx == 0
            || rest[..idx]
                .chars()
                .last()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after_key = &rest[idx + key.len()..];
        let after_ok = after_key
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '=');
        if before_ok && after_ok {
            let value = after_key.trim_start().strip_prefix('=')?.trim_start();
            return Some(read_assignment_value(value));
        }
        rest = &rest[idx + key.len()..];
    }
    None
}

pub(crate) fn read_assignment_value(value: &str) -> &str {
    let value = value.trim_start();
    if let Some(quoted) = value.strip_prefix('"') {
        if let Some(end) = quoted.find('"') {
            return &quoted[..end];
        }
        return quoted;
    }
    value
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('"')
}

pub(crate) fn resolve_texture(root: &Path, texturefile: &str) -> Option<PathBuf> {
    let rel = texturefile
        .replace('/', "\\")
        .trim_start_matches('\\')
        .to_string();
    let path = root.join(rel);
    path.exists().then_some(path)
}

pub(crate) fn write_icon_preview(
    output: &Path,
    root: &Path,
    rows: &[(Sprite, String, String, String)],
) -> Result<(), String> {
    let mut tsv = String::from("name\ttexturefile\tlocal_path\tstatus\n");
    let mut cards = String::new();
    for (sprite, local, preview, status) in rows {
        tsv.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            sprite.name, sprite.texturefile, local, status
        ));
        let image = if preview.is_empty() {
            "<div class=\"no-preview\">No Preview</div>".to_string()
        } else {
            format!(
                "<img src=\"{}\" alt=\"{}\">",
                html_escape(preview),
                html_escape(&sprite.name)
            )
        };
        cards.push_str(&format!(
            "<article class=\"card\"><div class=\"thumb\">{}</div><h2>{}</h2><p><b>texture</b> {}</p><p><b>status</b> {}</p><p class=\"path\">{}</p></article>\n",
            image,
            html_escape(&sprite.name),
            html_escape(&sprite.texturefile),
            html_escape(status),
            html_escape(local)
        ));
    }
    let html = format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>HOI4 Icon Preview</title>
<style>body{{margin:0;font-family:Segoe UI,Arial,sans-serif;background:#f5f2ec;color:#242424}}header{{padding:18px 24px;background:#243447;color:white}}header h1{{margin:0 0 6px;font-size:22px}}header p{{margin:0;color:#d8e1ea;font-size:13px}}.toolbar{{padding:12px 24px;background:white;border-bottom:1px solid #ddd;position:sticky;top:0}}input{{width:min(560px,100%);padding:8px 10px;font-size:14px;border:1px solid #b8b8b8;border-radius:4px}}.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:12px;padding:16px 24px 28px}}.card{{background:white;border:1px solid #ddd;border-radius:6px;padding:10px;min-width:0}}.thumb{{height:128px;display:grid;place-items:center;background:#202833;border-radius:4px;overflow:hidden}}.thumb img{{max-width:112px;max-height:112px}}.no-preview{{color:#cfd6df;font-size:13px}}h2{{font-size:14px;margin:10px 0 6px;overflow-wrap:anywhere}}p{{margin:4px 0;font-size:12px;line-height:1.35;overflow-wrap:anywhere}}.path{{color:#666}}</style></head>
<body><header><h1>HOI4 Icon Preview</h1><p>Mod: {} | Items: {}</p></header><div class="toolbar"><input id="filter" placeholder="Filter by sprite name or texture path"></div><main class="grid" id="grid">{}</main><script>const input=document.getElementById('filter');const cards=[...document.querySelectorAll('.card')];input.addEventListener('input',()=>{{const q=input.value.toLowerCase();for(const card of cards){{card.style.display=card.innerText.toLowerCase().includes(q)?'':'none';}}}});</script></body></html>"#,
        html_escape(&root.display().to_string()),
        rows.len(),
        cards
    );
    fs::write(output.join("index.html"), html)
        .map_err(|e| format!("write icon preview html: {e}"))?;
    fs::write(output.join("icons.tsv"), tsv).map_err(|e| format!("write icons.tsv: {e}"))?;
    Ok(())
}
