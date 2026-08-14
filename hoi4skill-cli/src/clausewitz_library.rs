//! Local retrieval library built from the user's HOI4 installation and mods.

#[allow(unused_imports)]
use crate::*;
use std::io::{Read, Seek, SeekFrom, Write};

const LIBRARY_VERSION: &str = "1";
const MAX_SNIPPET_CHARS: usize = 24_000;

#[derive(Clone, Debug)]
pub(crate) struct ClausewitzExample {
    pub(crate) system: String,
    pub(crate) symbol: String,
    pub(crate) source: String,
    pub(crate) code: String,
}

#[derive(Clone)]
struct ParsedBlock {
    key: String,
    start: usize,
    open: usize,
    end: usize,
    depth: usize,
}

pub(crate) fn cmd_build_clausewitz_library(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let game_root = normalize_path(&require_value(&map, "game-root")?)?;
    let code_mod_roots = code_mod_roots(&map)?;
    if !code_mod_roots.is_empty() {
        enforce_mod_code_request(&require_value(&map, "request")?, &code_mod_roots)?;
    }
    let output = value(&map, "output")
        .map(normalize_path)
        .transpose()?
        .unwrap_or(default_clausewitz_library_path()?);
    let count = build_empty_clausewitz_library(std::slice::from_ref(&game_root), &output)?;
    println!(
        "Official game code snippet output is disabled; metadata-only library: {}",
        output.display()
    );
    println!("Official examples exported: {count}");
    if !code_mod_roots.is_empty() {
        let overlay = mod_overlay_library_path(&output, &code_mod_roots);
        let count = build_clausewitz_library_with_options(
            &code_mod_roots,
            &overlay,
            layered_scan_options_from_args(&map)?,
        )?;
        println!("User-authorized mod code library: {}", overlay.display());
        println!("Mod examples: {count}");
    }
    Ok(())
}

pub(crate) fn cmd_query_clausewitz_library(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let query = require_value(&map, "query")?;
    let library = value(&map, "library")
        .map(normalize_path)
        .transpose()?
        .unwrap_or(default_clausewitz_library_path()?);
    let max_results = parse_usize_option(&map, "max-results", 6)?;
    let mut libraries = repeated_values(&map, "mod-library")
        .into_iter()
        .map(normalize_path)
        .collect::<Result<Vec<_>, _>>()?;
    libraries.push(library);
    let examples =
        query_clausewitz_libraries(&libraries, &query, value(&map, "system"), max_results)?;
    write_or_print(
        &render_clausewitz_examples_markdown(&query, &examples),
        value(&map, "output"),
    )
}

pub(crate) fn default_clausewitz_library_path() -> Result<PathBuf, String> {
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(local)
            .join("hoi4skill")
            .join("clausewitz-library"));
    }
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        return Ok(PathBuf::from(home)
            .join(".cache")
            .join("hoi4skill")
            .join("clausewitz-library"));
    }
    Ok(env::current_dir()
        .map_err(|e| e.to_string())?
        .join(".hoi4skill")
        .join("clausewitz-library"))
}

pub(crate) fn ensure_clausewitz_libraries(
    game_root: &Path,
    code_mod_roots: &[PathBuf],
    requested_path: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let path = requested_path
        .map(Path::to_path_buf)
        .unwrap_or(default_clausewitz_library_path()?);
    let roots = vec![game_root.to_path_buf()];
    let fingerprint = clausewitz_library_fingerprint(&roots);
    if path.join("manifest.json").is_file()
        && path.join("index.tsv").is_file()
        && path.join("snippets.dat").is_file()
        && {
            let manifest = read_utf8_lossy(&path.join("manifest.json"))?;
            manifest.contains(&format!(
                "\"source_fingerprint\": {}",
                json_str(&fingerprint)
            )) && manifest.contains("\"official_code_output_disabled\": true")
        }
    {
        let mut libraries = Vec::new();
        if !code_mod_roots.is_empty() {
            let overlay = mod_overlay_library_path(&path, code_mod_roots);
            ensure_library_at(code_mod_roots, &overlay)?;
            libraries.push(overlay);
        }
        libraries.push(path);
        return Ok(libraries);
    }
    build_empty_clausewitz_library(&roots, &path)?;
    let mut libraries = Vec::new();
    if !code_mod_roots.is_empty() {
        let overlay = mod_overlay_library_path(&path, code_mod_roots);
        ensure_library_at(code_mod_roots, &overlay)?;
        libraries.push(overlay);
    }
    libraries.push(path);
    Ok(libraries)
}

fn ensure_library_at(roots: &[PathBuf], path: &Path) -> Result<(), String> {
    let fingerprint = clausewitz_library_fingerprint(roots);
    if path.join("manifest.json").is_file()
        && path.join("index.tsv").is_file()
        && path.join("snippets.dat").is_file()
        && read_utf8_lossy(&path.join("manifest.json"))?.contains(&format!(
            "\"source_fingerprint\": {}",
            json_str(&fingerprint)
        ))
    {
        return Ok(());
    }
    build_clausewitz_library(roots, path)?;
    Ok(())
}

pub(crate) fn code_mod_roots(map: &ArgMap) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for raw in repeated_values(map, "code-mod-path") {
        for path in split_path_option(raw) {
            roots.push(resolve_mod_root(&normalize_path(path)?)?.root);
        }
    }
    Ok(dedupe_paths(roots))
}

pub(crate) fn request_explicitly_loads_mod_code(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let names_mod = lower.contains("模组") || lower.contains("mod");
    let requests_code_reference = [
        "加载",
        "读取",
        "参考",
        "仿照",
        "load",
        "reference",
        "use",
        "follow",
    ]
    .iter()
    .any(|verb| lower.contains(verb));
    names_mod && requests_code_reference
}

pub(crate) fn enforce_mod_code_request(request: &str, roots: &[PathBuf]) -> Result<(), String> {
    if roots.is_empty() || request_explicitly_loads_mod_code(request) {
        return Ok(());
    }
    Err("mod code loading is forbidden unless the user's literal request explicitly asks to load, reference, or imitate a specific mod's code; --mod-path alone is dependency evidence, not code-library authorization".to_string())
}

fn mod_overlay_library_path(base: &Path, roots: &[PathBuf]) -> PathBuf {
    let fingerprint = clausewitz_library_fingerprint(roots);
    let hash = fingerprint
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            hash.wrapping_mul(0x100000001b3) ^ u64::from(byte)
        });
    base.parent()
        .unwrap_or(base)
        .join("mod-code-libraries")
        .join(format!("{hash:016x}"))
}

pub(crate) fn build_clausewitz_library(roots: &[PathBuf], output: &Path) -> Result<usize, String> {
    build_clausewitz_library_with_options(roots, output, LayeredScanOptions::effective())
}

pub(crate) fn build_clausewitz_library_with_options(
    roots: &[PathBuf],
    output: &Path,
    scan_options: LayeredScanOptions,
) -> Result<usize, String> {
    if output.exists() {
        let known_library = output.join("manifest.json").is_file()
            && output.join("index.tsv").is_file()
            && output.join("snippets.dat").is_file();
        let empty_directory = output
            .read_dir()
            .map_err(|e| format!("read Clausewitz library output {}: {e}", output.display()))?
            .next()
            .is_none();
        if !known_library && !empty_directory {
            return Err(format!(
                "refusing to replace non-library directory {}; choose an empty output directory",
                output.display()
            ));
        }
        fs::remove_dir_all(output)
            .map_err(|e| format!("replace Clausewitz library {}: {e}", output.display()))?;
    }
    fs::create_dir_all(output).map_err(|e| format!("create {}: {e}", output.display()))?;

    let plan = LayeredSourcePlan::from_roots(roots)?;
    let layered_scan = plan.report(scan_options)?;
    let mut examples = Vec::new();
    for (layer_index, root) in plan.roots().into_iter().enumerate() {
        if !root.is_dir() {
            return Err(format!(
                "{}: indexed root is not a directory",
                root.display()
            ));
        }
        collect_clausewitz_examples_for_layer(&plan, layer_index, &mut examples)?;
    }
    examples.sort_by(|left, right| {
        (&left.system, &left.symbol, &left.source).cmp(&(
            &right.system,
            &right.symbol,
            &right.source,
        ))
    });
    examples.dedup_by(|left, right| {
        left.system == right.system
            && left.symbol == right.symbol
            && left.source == right.source
            && left.code == right.code
    });

    let snippets_path = output.join("snippets.dat");
    fs::create_dir_all(output).map_err(|e| format!("create {}: {e}", output.display()))?;
    let mut snippets = fs::File::create(&snippets_path)
        .map_err(|e| format!("create {}: {e}", snippets_path.display()))?;
    let mut offset = 0u64;
    let mut index = String::from("id\tsystem\tsymbol\tsource\toffset\tlength\tsearch\n");
    for (position, example) in examples.iter().enumerate() {
        let id = format!("{:06}", position + 1);
        let code = truncate_chars(&example.code, MAX_SNIPPET_CHARS);
        let bytes = code.as_bytes();
        snippets
            .write_all(bytes)
            .map_err(|e| format!("write Clausewitz snippet {id}: {e}"))?;
        let search = example_search_terms(example);
        index.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            id,
            tsv_field(&example.system),
            tsv_field(&example.symbol),
            tsv_field(&example.source),
            offset,
            bytes.len(),
            tsv_field(&search)
        ));
        offset += bytes.len() as u64;
    }
    fs::write(output.join("index.tsv"), index)
        .map_err(|e| format!("write Clausewitz library index: {e}"))?;
    let roots_json = roots
        .iter()
        .map(|root| json_str(&root.display().to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        output.join("manifest.json"),
        format!(
            "{{\n  \"format_version\": {},\n  \"source_fingerprint\": {},\n  \"example_count\": {},\n  \"layered_scan\": {},\n  \"roots\": [{}]\n}}\n",
            json_str(LIBRARY_VERSION),
            json_str(&clausewitz_library_fingerprint(roots)),
            examples.len(),
            layered_scan_report_json(&layered_scan),
            roots_json
        ),
    )
    .map_err(|e| format!("write Clausewitz library manifest: {e}"))?;
    Ok(examples.len())
}

fn build_empty_clausewitz_library(roots: &[PathBuf], output: &Path) -> Result<usize, String> {
    if output.exists() {
        let known_library = output.join("manifest.json").is_file()
            && output.join("index.tsv").is_file()
            && output.join("snippets.dat").is_file();
        let empty_directory = output
            .read_dir()
            .map_err(|e| format!("read {}: {e}", output.display()))?
            .next()
            .is_none();
        if !known_library && !empty_directory {
            return Err(format!(
                "refusing to replace non-library directory {}; choose an empty output directory",
                output.display()
            ));
        }
        fs::remove_dir_all(output)
            .map_err(|e| format!("replace Clausewitz library {}: {e}", output.display()))?;
    }
    fs::create_dir_all(output).map_err(|e| format!("create {}: {e}", output.display()))?;
    fs::write(output.join("snippets.dat"), b"")
        .map_err(|e| format!("write empty Clausewitz snippet store: {e}"))?;
    fs::write(
        output.join("index.tsv"),
        "id\tsystem\tsymbol\tsource\toffset\tlength\tsearch\n",
    )
    .map_err(|e| format!("write empty Clausewitz library index: {e}"))?;
    let roots_json = roots
        .iter()
        .map(|root| json_str(&root.display().to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        output.join("manifest.json"),
        format!(
            "{{\n  \"format_version\": {},\n  \"source_fingerprint\": {},\n  \"example_count\": 0,\n  \"official_code_output_disabled\": true,\n  \"roots\": [{}]\n}}\n",
            json_str(LIBRARY_VERSION),
            json_str(&clausewitz_library_fingerprint(roots)),
            roots_json
        ),
    )
    .map_err(|e| format!("write empty Clausewitz library manifest: {e}"))?;
    Ok(0)
}

fn clausewitz_library_fingerprint(roots: &[PathBuf]) -> String {
    let mut parts = Vec::new();
    if let Ok(plan) = LayeredSourcePlan::from_roots(roots) {
        for layer_index in 0..plan.layers().len() {
            parts.push(format!(
                "visibility:{}",
                plan.visibility_fingerprint(layer_index)
            ));
        }
    }
    for root in roots {
        parts.push(root.display().to_string());
        for relative in [
            "hoi4.exe",
            "common/national_focus",
            "events",
            "common/ideas",
            "common/decisions",
            "common/characters",
            "common/scripted_effects",
            "common/scripted_triggers",
            "history/countries",
            "history/states",
            "interface",
        ] {
            let path = root.join(relative);
            let modified = path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            parts.push(format!("{relative}:{modified}"));
        }
    }
    parts.join("|")
}

pub(crate) fn query_clausewitz_library(
    library: &Path,
    query: &str,
    system: Option<&str>,
    max_results: usize,
) -> Result<Vec<ClausewitzExample>, String> {
    let index_path = library.join("index.tsv");
    let index = read_utf8_lossy(&index_path)?;
    let terms = expanded_query_terms(query);
    let requested_system = system.map(normalize_system_name);
    let mut ranked = Vec::new();
    for line in index.lines().skip(1) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() < 7 {
            continue;
        }
        let row_system = fields[1];
        if requested_system
            .as_deref()
            .is_some_and(|requested| !system_matches(requested, row_system))
        {
            continue;
        }
        let haystack = fields[6];
        let score = score_search_text(haystack, &terms)
            + if normalize_search_text(fields[2]) == normalize_search_text(query) {
                20
            } else {
                0
            };
        let length = fields[5].parse::<usize>().unwrap_or(0);
        ranked.push((
            score,
            length,
            fields[0].to_string(),
            row_system.to_string(),
            fields[2].to_string(),
            fields[3].to_string(),
            fields[4].parse::<u64>().unwrap_or(0),
            length,
        ));
    }
    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    ranked.truncate(max_results);

    ranked
        .into_iter()
        .map(|(_, _, _id, system, symbol, source, offset, length)| {
            let code = read_clausewitz_snippet(library, offset, length)?;
            Ok(ClausewitzExample {
                system,
                symbol,
                source,
                code,
            })
        })
        .collect()
}

pub(crate) fn query_clausewitz_libraries(
    libraries: &[PathBuf],
    query: &str,
    system: Option<&str>,
    max_results: usize,
) -> Result<Vec<ClausewitzExample>, String> {
    let mut examples = Vec::new();
    for library in libraries {
        for example in query_clausewitz_library(library, query, system, max_results)? {
            if !examples.iter().any(|existing: &ClausewitzExample| {
                existing.system == example.system
                    && existing.symbol == example.symbol
                    && existing.source == example.source
            }) {
                examples.push(example);
            }
            if examples.len() >= max_results {
                return Ok(examples);
            }
        }
    }
    Ok(examples)
}

fn read_clausewitz_snippet(library: &Path, offset: u64, length: usize) -> Result<String, String> {
    let path = library.join("snippets.dat");
    let mut file = fs::File::open(&path).map_err(|e| format!("open {}: {e}", path.display()))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("seek {}: {e}", path.display()))?;
    let mut bytes = vec![0u8; length];
    file.read_exact(&mut bytes)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

pub(crate) fn render_clausewitz_examples_markdown(
    query: &str,
    examples: &[ClausewitzExample],
) -> String {
    let mut out = String::new();
    out.push_str("# Retrieved HOI4 Clausewitz Examples\n\n");
    out.push_str(&format!("- query: `{}`\n", query.replace('`', "'")));
    out.push_str("- rule: these are read-only examples from indexed local game/mod files; copy structure and verified field names, not IDs or narrative content.\n");
    out.push_str("- rule: generated templates still own final output for focuses, events, decisions, and ideas.\n\n");
    if examples.is_empty() {
        out.push_str("- No matching local code examples were found.\n");
        return out;
    }
    for example in examples {
        out.push_str(&format!(
            "## {} `{}`\n\n- source: `{}`\n\n",
            example.system,
            example.symbol,
            example.source.replace('`', "'")
        ));
        out.push_str(&markdown_fence(
            "hoi4",
            &truncate_chars(&example.code, 12_000),
        ));
        out.push('\n');
    }
    out
}

pub(crate) fn render_retrieved_clausewitz_context(
    libraries: &[PathBuf],
    query: &str,
    authorized_systems: &[String],
) -> Result<String, String> {
    let mut systems = authorized_systems
        .iter()
        .filter_map(|system| match system.as_str() {
            "national_focus" => Some("focus"),
            "events" => Some("event"),
            "national_spirits" => Some("idea"),
            "decisions" => Some("decision"),
            "characters" => Some("character"),
            "country_history" => Some("country_history"),
            "state_history" => Some("state_history"),
            _ => None,
        })
        .collect::<Vec<_>>();
    systems.sort();
    systems.dedup();
    if systems.is_empty() {
        systems.extend(["focus", "event", "idea", "decision"]);
    }

    let mut examples = Vec::new();
    for system in systems {
        for example in query_clausewitz_libraries(libraries, query, Some(system), 2)? {
            if !examples.iter().any(|existing: &ClausewitzExample| {
                existing.system == example.system
                    && existing.symbol == example.symbol
                    && existing.source == example.source
            }) {
                examples.push(example);
            }
        }
    }
    if examples.is_empty() {
        return Ok(
            "- No matching examples were found; stop instead of inventing Clausewitz syntax.\n"
                .to_string(),
        );
    }

    let mut out = String::new();
    for example in examples {
        out.push_str(&format!(
            "\n### {} `{}`\n\n- source: `{}`\n\n",
            example.system,
            example.symbol,
            example.source.replace('`', "'")
        ));
        out.push_str(&markdown_fence("hoi4", &example.code));
    }
    Ok(out)
}

fn collect_clausewitz_examples_for_layer(
    plan: &LayeredSourcePlan,
    layer_index: usize,
    out: &mut Vec<ClausewitzExample>,
) -> Result<(), String> {
    let root = &plan.layers()[layer_index].root;
    for (relative, system) in [
        ("common/national_focus", "focus"),
        ("events", "event"),
        ("common/ideas", "idea"),
        ("common/decisions", "decision"),
        ("common/decision_categories", "decision_category"),
        ("common/characters", "character"),
        ("common/scripted_effects", "scripted_effect"),
        ("common/scripted_triggers", "scripted_trigger"),
        ("history/countries", "country_history"),
        ("history/states", "state_history"),
        ("interface", "gfx"),
    ] {
        for file in plan.collect_files(layer_index, relative)? {
            let extension = file
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "txt" | "gfx") {
                continue;
            }
            let text = read_utf8_lossy(&file)?;
            let root_label = root
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("indexed-root");
            let source = format!("{root_label}/{}", relative_slash_path(root, &file));
            collect_file_examples(system, &source, &text, out);
        }
    }
    Ok(())
}

fn collect_file_examples(system: &str, source: &str, text: &str, out: &mut Vec<ClausewitzExample>) {
    let blocks = scan_clausewitz_blocks(text);
    match system {
        "focus" => {
            for block in blocks.iter().filter(|block| block.key == "focus") {
                if let Some(id) = block_assignment(block_content(text, block), "id") {
                    push_block_example(out, "focus", &id, source, text, block);
                }
            }
            if let Some(tree) = blocks
                .iter()
                .find(|block| block.key == "focus_tree" && block.depth == 0)
            {
                let content = block_content(text, tree);
                let id =
                    block_assignment(content, "id").unwrap_or_else(|| "focus_tree".to_string());
                let header_end = scan_clausewitz_blocks(content)
                    .into_iter()
                    .find(|child| child.key == "focus")
                    .map(|child| tree.open + 1 + child.start)
                    .unwrap_or(tree.end);
                let header = format!("{}{}\n}}", &text[tree.start..header_end], "");
                out.push(ClausewitzExample {
                    system: "focus_tree".to_string(),
                    symbol: id,
                    source: source.to_string(),
                    code: header,
                });
            }
        }
        "event" => {
            for block in blocks.iter().filter(|block| {
                block.depth == 0
                    && matches!(
                        block.key.as_str(),
                        "country_event"
                            | "news_event"
                            | "state_event"
                            | "unit_leader_event"
                            | "operative_leader_event"
                    )
            }) {
                if let Some(id) = block_assignment(block_content(text, block), "id") {
                    push_block_example(out, &block.key, &id, source, text, block);
                }
            }
        }
        "idea" => {
            for block in blocks
                .iter()
                .filter(|block| block.depth == 2 && !known_idea_field(&block.key))
            {
                let kind = if block_content(text, block).contains("traits =") {
                    "advisor"
                } else {
                    "idea"
                };
                push_block_example(out, kind, &block.key, source, text, block);
            }
        }
        "decision" => {
            for block in blocks
                .iter()
                .filter(|block| block.depth == 1 && !known_decision_field(&block.key))
            {
                push_block_example(out, "decision", &block.key, source, text, block);
            }
        }
        "decision_category" => {
            for block in blocks.iter().filter(|block| block.depth == 0) {
                push_block_example(out, "decision_category", &block.key, source, text, block);
            }
        }
        "character" => {
            for block in blocks.iter().filter(|block| block.depth == 1) {
                push_block_example(out, "character", &block.key, source, text, block);
            }
        }
        "scripted_effect" | "scripted_trigger" => {
            for block in blocks.iter().filter(|block| block.depth == 0) {
                push_block_example(out, system, &block.key, source, text, block);
            }
        }
        "state_history" => {
            if let Some(block) = blocks
                .iter()
                .find(|block| block.depth == 0 && block.key == "state")
            {
                let symbol = block_assignment(block_content(text, block), "id")
                    .unwrap_or_else(|| source.to_string());
                push_block_example(out, system, &symbol, source, text, block);
            }
        }
        "country_history" => {
            if text.chars().count() <= MAX_SNIPPET_CHARS {
                out.push(ClausewitzExample {
                    system: system.to_string(),
                    symbol: Path::new(source)
                        .file_stem()
                        .and_then(OsStr::to_str)
                        .unwrap_or(source)
                        .to_string(),
                    source: source.to_string(),
                    code: text.to_string(),
                });
            }
        }
        "gfx" => {
            for block in blocks.iter().filter(|block| block.key == "spriteType") {
                if let Some(name) = block_assignment(block_content(text, block), "name") {
                    push_block_example(out, "gfx_sprite", &name, source, text, block);
                }
            }
        }
        _ => {}
    }
}

fn push_block_example(
    out: &mut Vec<ClausewitzExample>,
    system: &str,
    symbol: &str,
    source: &str,
    text: &str,
    block: &ParsedBlock,
) {
    let code = text[block.start..block.end].trim().to_string();
    if code.chars().count() > MAX_SNIPPET_CHARS || code.lines().count() < 2 {
        return;
    }
    out.push(ClausewitzExample {
        system: system.to_string(),
        symbol: symbol.trim_matches('"').to_string(),
        source: source.to_string(),
        code,
    });
}

fn block_content<'a>(text: &'a str, block: &ParsedBlock) -> &'a str {
    &text[block.open + 1..block.end - 1]
}

fn scan_clausewitz_blocks(text: &str) -> Vec<ParsedBlock> {
    let bytes = text.as_bytes();
    let mut blocks = Vec::new();
    let mut i = 0usize;
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut escape = false;
    let mut in_comment = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
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
        if ch == '#' {
            in_comment = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            in_quote = true;
            i += 1;
            continue;
        }
        if ch == '}' {
            depth = depth.saturating_sub(1);
            i += 1;
            continue;
        }
        if ch == '{' {
            depth += 1;
            i += 1;
            continue;
        }
        if is_identifier_byte(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_identifier_byte(bytes[i]) {
                i += 1;
            }
            let key = text[start..i].to_string();
            let mut cursor = i;
            while cursor < bytes.len() && (bytes[cursor] as char).is_whitespace() {
                cursor += 1;
            }
            if bytes.get(cursor) != Some(&b'=') {
                continue;
            }
            cursor += 1;
            while cursor < bytes.len() && (bytes[cursor] as char).is_whitespace() {
                cursor += 1;
            }
            if bytes.get(cursor) != Some(&b'{') {
                continue;
            }
            if let Some(end) = clausewitz_block_end(text, cursor) {
                blocks.push(ParsedBlock {
                    key,
                    start,
                    open: cursor,
                    end,
                    depth,
                });
            }
            continue;
        }
        i += 1;
    }
    blocks
}

fn clausewitz_block_end(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 1usize;
    let mut i = open + 1;
    let mut in_quote = false;
    let mut escape = false;
    let mut in_comment = false;
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
        } else if in_quote {
            if ch == '"' && !escape {
                in_quote = false;
            }
            if escape {
                escape = false;
            } else {
                escape = ch == '\\';
            }
        } else if ch == '#' {
            in_comment = true;
        } else if ch == '"' {
            in_quote = true;
        } else if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i + 1);
            }
        }
        i += 1;
    }
    None
}

fn known_idea_field(key: &str) -> bool {
    matches!(
        key,
        "allowed"
            | "visible"
            | "available"
            | "modifier"
            | "research_bonus"
            | "equipment_bonus"
            | "on_add"
            | "on_remove"
            | "cancel"
            | "do_effect"
    )
}

fn known_decision_field(key: &str) -> bool {
    matches!(
        key,
        "allowed"
            | "visible"
            | "available"
            | "target_root_trigger"
            | "target_trigger"
            | "custom_cost_trigger"
            | "ai_will_do"
            | "on_map_mode"
            | "scripted_gui"
            | "modifier"
    )
}

fn normalize_system_name(system: &str) -> String {
    match system.trim().to_ascii_lowercase().as_str() {
        "national_focus" | "focuses" | "国策" => "focus".to_string(),
        "events" | "事件" => "event".to_string(),
        "ideas" | "national_spirit" | "national_spirits" | "民族精神" => "idea".to_string(),
        "decisions" | "决议" => "decision".to_string(),
        "characters" | "leader" | "leaders" | "人物" | "领导人" => "character".to_string(),
        "history" | "历史" => "country_history".to_string(),
        other => other.to_string(),
    }
}

fn system_matches(requested: &str, actual: &str) -> bool {
    requested == actual
        || (requested == "event" && actual.ends_with("_event"))
        || (requested == "focus" && actual == "focus_tree")
        || (requested == "history" && matches!(actual, "country_history" | "state_history"))
}

fn example_search_terms(example: &ClausewitzExample) -> String {
    let mut terms = normalize_search_text(&format!(
        "{} {} {}",
        example.system, example.symbol, example.source
    ))
    .split_whitespace()
    .map(str::to_string)
    .collect::<Vec<_>>();
    let mut identifier = String::new();
    for ch in example.code.chars() {
        if ch.is_alphanumeric() || ch == '_' || !ch.is_ascii() {
            identifier.push(ch.to_ascii_lowercase());
        } else if !identifier.is_empty() {
            terms.push(std::mem::take(&mut identifier));
        }
        if terms.len() >= 100 {
            break;
        }
    }
    if !identifier.is_empty() && terms.len() < 100 {
        terms.push(identifier);
    }
    terms.sort();
    terms.dedup();
    terms.join(" ")
}

fn expanded_query_terms(query: &str) -> Vec<String> {
    let mut terms = normalize_search_text(query)
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let lower = query.to_ascii_lowercase();
    for (needles, additions) in [
        (
            &["共产", "社会主义", "苏维埃", "communist", "socialist"][..],
            &["communist", "communism", "socialist", "socialism", "soviet"][..],
        ),
        (
            &["民主", "democratic", "democracy"][..],
            &["democratic", "democracy", "parliament", "election"][..],
        ),
        (
            &["法西斯", "fascist", "fascism"][..],
            &["fascist", "fascism", "nationalist"][..],
        ),
        (
            &["君主", "monarch", "king", "皇帝"][..],
            &["monarch", "monarchy", "king", "royal", "emperor"][..],
        ),
        (
            &["工人", "worker", "劳工"][..],
            &["worker", "workers", "labour", "labor", "proletarian"][..],
        ),
        (
            &["革命", "revolution", "起义"][..],
            &["revolution", "revolutionary", "uprising", "rebellion"][..],
        ),
        (
            &["游击", "guerrilla", "partisan"][..],
            &["guerrilla", "partisan", "resistance"][..],
        ),
    ] {
        if needles.iter().any(|needle| lower.contains(needle)) {
            terms.extend(additions.iter().map(|term| term.to_string()));
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

fn normalize_search_text(text: &str) -> String {
    text.chars()
        .map(|ch| {
            if ch.is_alphanumeric() || !ch.is_ascii() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn score_search_text(haystack: &str, terms: &[String]) -> usize {
    terms
        .iter()
        .map(|term| {
            if haystack.split_whitespace().any(|token| token == term) {
                4
            } else if haystack.contains(term) {
                1
            } else {
                0
            }
        })
        .sum()
}

fn tsv_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}
