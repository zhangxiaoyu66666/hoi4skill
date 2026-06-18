//! Project-level mod symbol index.
//!
//! This is the first layer for large-mod production: it records where stable
//! symbols are defined so later commands can query and analyze impact.

#[allow(unused_imports)]
use crate::*;

#[derive(Clone, Debug)]
struct ModSymbol {
    kind: String,
    id: String,
    file: String,
    owner: Option<String>,
    parent: Option<String>,
    title: Option<String>,
    extra: BTreeMap<String, String>,
}

#[derive(Default)]
struct ModIndex {
    symbols: Vec<ModSymbol>,
    files_total: usize,
    localisation_keys_total: usize,
    sprite_total: usize,
}

pub(crate) fn cmd_build_mod_index(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let max_symbols = parse_usize_option(&map, "max-symbols", 20000)?;
    let index = build_mod_index(&resolved.root)?;
    let json = mod_index_json(&resolved, &index, max_symbols);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_query_symbol(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let query = require_value(&map, "symbol")?;
    let kind_filter = value(&map, "kind");
    let contains = map.flags.contains("contains");
    let max_results = parse_usize_option(&map, "max-results", 50)?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let index = build_mod_index(&resolved.root)?;
    let results = query_mod_symbols(&index.symbols, &query, kind_filter, contains, max_results);
    let json = query_symbol_json(&resolved, &query, kind_filter, contains, &index, &results);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_impact(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let symbol = value(&map, "symbol").map(str::to_string);
    let changed = value(&map, "changed").map(str::to_string);
    if symbol.is_none() && changed.is_none() {
        return Err("missing --symbol or --changed".to_string());
    }
    let max_symbols = parse_usize_option(&map, "max-symbols", 80)?;
    let max_references = parse_usize_option(&map, "max-references", 200)?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let index = build_mod_index(&resolved.root)?;
    let report = build_impact_report(
        &resolved.root,
        &index,
        symbol.as_deref(),
        changed.as_deref(),
        max_symbols,
        max_references,
    )?;
    let json = impact_report_json(&resolved, &report);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_reserve_id(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let kind = require_value(&map, "kind")?;
    let count = parse_usize_option(&map, "count", 1)?;
    if count == 0 {
        return Err("--count must be at least 1".to_string());
    }
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let index = build_mod_index(&resolved.root)?;
    let reservation = reserve_ids(
        &index,
        &kind,
        value(&map, "namespace"),
        value(&map, "prefix"),
        value(&map, "tag"),
        count,
    )?;
    let json = reserve_id_json(&resolved, &reservation);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_check_namespace(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let namespace = value(&map, "namespace").map(normalize_namespace);
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let index = build_mod_index(&resolved.root)?;
    let report = check_event_namespaces(&resolved.root, &index, namespace.as_deref())?;
    let json = namespace_check_json(&resolved, namespace.as_deref(), &report);
    write_or_print(&json, value(&map, "output"))
}

pub(crate) fn cmd_feature_context(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = map
        .positionals
        .first()
        .cloned()
        .or_else(|| value(&map, "mod-root").map(str::to_string))
        .ok_or_else(|| "missing mod root or launcher .mod file".to_string())?;
    let tag = value(&map, "tag").map(str::to_string);
    let system = value(&map, "system").map(str::to_string);
    if tag.is_none() && system.is_none() {
        return Err("missing --tag or --system".to_string());
    }
    let max_symbols = parse_usize_option(&map, "max-symbols", 120)?;
    let max_references = parse_usize_option(&map, "max-references", 120)?;
    let resolved = resolve_mod_root(&normalize_path(&input)?)?;
    let index = build_mod_index(&resolved.root)?;
    let context = build_feature_context(
        &resolved.root,
        &index,
        tag.as_deref(),
        system.as_deref(),
        max_symbols,
        max_references,
    )?;
    let markdown = feature_context_markdown(&resolved, &context);
    write_or_print(&markdown, value(&map, "output"))
}

fn build_mod_index(root: &Path) -> Result<ModIndex, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let files_total = collect_files(root)?.len();
    let localisation = collect_focus_localisation_map(root)?;
    let mut symbols = Vec::new();
    collect_imported_content_symbols(root, &localisation, &mut symbols)?;
    collect_localisation_symbols(root, &mut symbols)?;
    collect_gfx_symbols(root, &mut symbols)?;
    collect_country_tag_symbols(root, &mut symbols)?;
    collect_scripted_symbols(root, &mut symbols)?;
    symbols.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.id.cmp(&b.id))
            .then(a.file.cmp(&b.file))
    });

    let localisation_keys_total = symbols
        .iter()
        .filter(|symbol| symbol.kind == "localisation")
        .count();
    let sprite_total = symbols
        .iter()
        .filter(|symbol| symbol.kind == "sprite")
        .count();

    Ok(ModIndex {
        symbols,
        files_total,
        localisation_keys_total,
        sprite_total,
    })
}

struct IdReservation {
    kind: String,
    namespace: Option<String>,
    prefix: Option<String>,
    count: usize,
    ids: Vec<String>,
    collisions_skipped: usize,
    existing_event_max: Option<i64>,
    notes: Vec<String>,
}

struct NamespaceCheckReport {
    ok: bool,
    namespaces: Vec<NamespaceCheck>,
    duplicate_event_ids: Vec<DuplicateEventId>,
    warnings: Vec<String>,
    suggested_commands: Vec<String>,
}

struct NamespaceCheck {
    namespace: String,
    declared_files: Vec<String>,
    event_files: Vec<String>,
    event_count: usize,
    max_id: Option<i64>,
    next_id: Option<i64>,
    warnings: Vec<String>,
}

struct DuplicateEventId {
    id: String,
    files: Vec<String>,
}

struct FeatureContext<'a> {
    tag: Option<String>,
    system: Option<String>,
    symbols: Vec<&'a ModSymbol>,
    references: Vec<ImpactReference>,
    files: Vec<String>,
    allowed_paths: Vec<String>,
    suggested_commands: Vec<String>,
}

fn check_event_namespaces(
    root: &Path,
    index: &ModIndex,
    namespace_filter: Option<&str>,
) -> Result<NamespaceCheckReport, String> {
    let declared_by_file = collect_declared_event_namespaces_by_file(root)?;
    let mut declared_by_namespace = BTreeMap::<String, BTreeSet<String>>::new();
    for (file, namespaces) in &declared_by_file {
        for namespace in namespaces {
            declared_by_namespace
                .entry(namespace.clone())
                .or_default()
                .insert(file.clone());
        }
    }

    let mut events_by_namespace = BTreeMap::<String, Vec<(&ModSymbol, Option<i64>)>>::new();
    let mut files_by_event_id = BTreeMap::<String, BTreeSet<String>>::new();
    for symbol in index.symbols.iter().filter(|symbol| symbol.kind == "event") {
        if let Some((namespace, number)) = event_id_namespace_number(&symbol.id) {
            if namespace_filter.is_none_or(|filter| filter == namespace) {
                events_by_namespace
                    .entry(namespace)
                    .or_default()
                    .push((symbol, Some(number)));
            }
        } else if namespace_filter.is_none() {
            events_by_namespace
                .entry("<invalid-event-id>".to_string())
                .or_default()
                .push((symbol, None));
        }
        files_by_event_id
            .entry(symbol.id.clone())
            .or_default()
            .insert(symbol.file.clone());
    }

    if let Some(filter) = namespace_filter {
        declared_by_namespace.entry(filter.to_string()).or_default();
        events_by_namespace.entry(filter.to_string()).or_default();
    }

    let mut namespace_names = BTreeSet::new();
    namespace_names.extend(declared_by_namespace.keys().cloned());
    namespace_names.extend(events_by_namespace.keys().cloned());

    let event_id_max = active_event_id_max();
    let mut checks = Vec::new();
    let mut warnings = Vec::new();
    for namespace in namespace_names {
        if namespace_filter.is_some_and(|filter| filter != namespace) {
            continue;
        }
        let declared_files = declared_by_namespace
            .get(&namespace)
            .map(|files| files.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let events = events_by_namespace
            .get(&namespace)
            .cloned()
            .unwrap_or_default();
        let mut event_files = events
            .iter()
            .map(|(symbol, _)| symbol.file.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        event_files.sort();
        let max_id = events.iter().filter_map(|(_, number)| *number).max();
        let next_id = max_id.map(|number| number + 1);
        let mut namespace_warnings = Vec::new();
        if declared_files.is_empty() && !events.is_empty() {
            namespace_warnings.push(format!(
                "namespace {namespace} has event ids but no add_namespace declaration"
            ));
        }
        if declared_files.len() > 1 {
            namespace_warnings.push(format!(
                "namespace {namespace} is declared in multiple files"
            ));
        }
        for (symbol, _) in &events {
            let file_namespaces = declared_by_file.get(&symbol.file);
            if file_namespaces.is_none_or(|namespaces| !namespaces.contains(&namespace)) {
                namespace_warnings.push(format!(
                    "event id {} appears in {} without add_namespace = {} in the same file",
                    symbol.id, symbol.file, namespace
                ));
            }
        }
        if max_id.is_some_and(|number| number >= event_id_max) {
            namespace_warnings.push(format!(
                "namespace {namespace} has reached or exceeded event id limit {event_id_max}"
            ));
        }
        warnings.extend(namespace_warnings.iter().cloned());
        checks.push(NamespaceCheck {
            namespace,
            declared_files,
            event_files,
            event_count: events.len(),
            max_id,
            next_id,
            warnings: namespace_warnings,
        });
    }

    let duplicate_event_ids = files_by_event_id
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(id, files)| DuplicateEventId {
            id,
            files: files.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    for duplicate in &duplicate_event_ids {
        warnings.push(format!(
            "event id {} is defined in multiple files",
            duplicate.id
        ));
    }

    let mut suggested_commands = vec![
        "hoi4skill build-mod-index <mod-root> --output .hoi4skill/mod_index.json".to_string(),
        "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
    ];
    if let Some(namespace) = namespace_filter {
        suggested_commands.push(format!(
            "hoi4skill reserve-id <mod-root> --kind event --namespace {namespace} --count 20"
        ));
    }
    Ok(NamespaceCheckReport {
        ok: warnings.is_empty(),
        namespaces: checks,
        duplicate_event_ids,
        warnings,
        suggested_commands,
    })
}

fn collect_declared_event_namespaces_by_file(
    root: &Path,
) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
    let mut out = BTreeMap::new();
    for file in txt_files(root, "events")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for line in text.lines() {
            let Some(namespace) = assignment_value(line.trim(), "add_namespace") else {
                continue;
            };
            out.entry(rel.clone())
                .or_insert_with(BTreeSet::new)
                .insert(namespace.to_string());
        }
    }
    Ok(out)
}

fn build_feature_context<'a>(
    root: &Path,
    index: &'a ModIndex,
    tag: Option<&str>,
    system: Option<&str>,
    max_symbols: usize,
    max_references: usize,
) -> Result<FeatureContext<'a>, String> {
    let mut symbols = index
        .symbols
        .iter()
        .filter(|symbol| {
            tag.is_some_and(|tag| symbol_matches_tag(symbol, tag))
                || system.is_some_and(|system| symbol_matches_system(symbol, system))
        })
        .collect::<Vec<_>>();
    symbols = dedupe_symbol_refs(symbols);
    symbols.truncate(max_symbols);

    let ids = symbols
        .iter()
        .map(|symbol| symbol.id.clone())
        .collect::<Vec<_>>();
    let references = collect_symbol_references(root, &ids, max_references)?;
    let mut files = BTreeSet::new();
    for symbol in &symbols {
        files.insert(symbol.file.clone());
    }
    for reference in &references {
        files.insert(reference.file.clone());
    }
    let files = files.into_iter().collect::<Vec<_>>();
    let allowed_paths = feature_allowed_paths(tag, system);
    let suggested_commands = feature_suggested_commands(tag, system);
    Ok(FeatureContext {
        tag: tag.map(str::to_string),
        system: system.map(str::to_string),
        symbols,
        references,
        files,
        allowed_paths,
        suggested_commands,
    })
}

fn reserve_ids(
    index: &ModIndex,
    kind: &str,
    namespace: Option<&str>,
    prefix: Option<&str>,
    tag: Option<&str>,
    count: usize,
) -> Result<IdReservation, String> {
    let existing = index
        .symbols
        .iter()
        .map(|symbol| symbol.id.clone())
        .collect::<BTreeSet<_>>();
    match kind {
        "event" => reserve_event_ids(index, namespace, prefix, count, &existing),
        "focus" | "idea" | "decision" | "decision_category" | "scripted_effect"
        | "scripted_trigger" | "on_action" => {
            reserve_prefixed_ids(kind, prefix.or(tag), count, &existing)
        }
        _ => Err(format!(
            "unsupported --kind {kind}; expected event, focus, idea, decision, decision_category, scripted_effect, scripted_trigger, or on_action"
        )),
    }
}

fn reserve_event_ids(
    index: &ModIndex,
    namespace: Option<&str>,
    prefix: Option<&str>,
    count: usize,
    existing: &BTreeSet<String>,
) -> Result<IdReservation, String> {
    let namespace = namespace
        .or(prefix)
        .map(normalize_namespace)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "event id reservation requires --namespace".to_string())?;
    let existing_event_max = index
        .symbols
        .iter()
        .filter(|symbol| symbol.kind == "event")
        .filter_map(|symbol| event_id_namespace_number(&symbol.id))
        .filter(|(event_namespace, _)| event_namespace == &namespace)
        .map(|(_, number)| number)
        .max();
    let mut ids = Vec::new();
    let mut collisions_skipped = 0;
    let mut next = existing_event_max.unwrap_or(0) + 1;
    while ids.len() < count {
        let id = format!("{namespace}.{next}");
        if existing.contains(&id) {
            collisions_skipped += 1;
        } else {
            ids.push(id);
        }
        next += 1;
    }
    Ok(IdReservation {
        kind: "event".to_string(),
        namespace: Some(namespace),
        prefix: None,
        count,
        ids,
        collisions_skipped,
        existing_event_max,
        notes: vec![
            "Event IDs are suggestions only; add_namespace must exist in the target event file before use.".to_string(),
            "Run hoi4skill validate with --strict-code-index after writing events.".to_string(),
        ],
    })
}

fn reserve_prefixed_ids(
    kind: &str,
    prefix: Option<&str>,
    count: usize,
    existing: &BTreeSet<String>,
) -> Result<IdReservation, String> {
    let prefix = prefix
        .map(normalize_id_prefix)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{kind} id reservation requires --prefix or --tag"))?;
    let stem = match kind {
        "focus" => "focus",
        "idea" => "idea",
        "decision" => "decision",
        "decision_category" => "category",
        "scripted_effect" => "effect",
        "scripted_trigger" => "trigger",
        "on_action" => "on_action",
        _ => kind,
    };
    let mut ids = Vec::new();
    let mut collisions_skipped = 0;
    let mut next = 1usize;
    while ids.len() < count {
        let id = format!("{prefix}_{stem}_{next:03}");
        if existing.contains(&id) || ids.contains(&id) {
            collisions_skipped += 1;
        } else {
            ids.push(id);
        }
        next += 1;
    }
    Ok(IdReservation {
        kind: kind.to_string(),
        namespace: None,
        prefix: Some(prefix),
        count,
        ids,
        collisions_skipped,
        existing_event_max: None,
        notes: vec![
            "IDs are suggestions only; keep them inside the work package allowed edit surface.".to_string(),
            "Run hoi4skill build-mod-index again before writing if other contributors changed the mod.".to_string(),
        ],
    })
}

fn symbol_matches_tag(symbol: &ModSymbol, tag: &str) -> bool {
    let tag_upper = tag.to_ascii_uppercase();
    let tag_lower = tag.to_ascii_lowercase();
    symbol.owner.as_deref().is_some_and(|owner| {
        owner.eq_ignore_ascii_case(&tag_upper) || owner.eq_ignore_ascii_case(&tag_lower)
    }) || symbol.id.starts_with(&format!("{tag_upper}_"))
        || symbol.id.starts_with(&format!("{tag_lower}_"))
        || symbol.file.to_ascii_lowercase().contains(&tag_lower)
}

fn symbol_matches_system(symbol: &ModSymbol, system: &str) -> bool {
    let needle = normalize_search_text(system);
    [
        Some(symbol.id.as_str()),
        Some(symbol.file.as_str()),
        symbol.owner.as_deref(),
        symbol.parent.as_deref(),
        symbol.title.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| normalize_search_text(value).contains(&needle))
}

fn normalize_search_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn feature_allowed_paths(tag: Option<&str>, system: Option<&str>) -> Vec<String> {
    if tag.is_some() {
        vec![
            "common/national_focus".to_string(),
            "events".to_string(),
            "common/ideas".to_string(),
            "common/decisions".to_string(),
            "common/scripted_effects".to_string(),
            "common/scripted_triggers".to_string(),
            "interface".to_string(),
            "gfx/interface".to_string(),
            "localisation".to_string(),
        ]
    } else if system.is_some() {
        vec![
            "common/scripted_effects".to_string(),
            "common/scripted_triggers".to_string(),
            "common/on_actions".to_string(),
            "common/decisions".to_string(),
            "events".to_string(),
            "localisation".to_string(),
        ]
    } else {
        Vec::new()
    }
}

fn feature_suggested_commands(tag: Option<&str>, system: Option<&str>) -> Vec<String> {
    let mut commands = vec![
        "hoi4skill build-mod-index <mod-root> --output mod_index.json".to_string(),
        "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
    ];
    if let Some(tag) = tag {
        commands.push(format!(
            "hoi4skill reserve-id <mod-root> --kind focus --tag {tag} --count 10"
        ));
        commands.push(format!(
            "hoi4skill impact <mod-root> --changed common/national_focus/{tag}.txt"
        ));
    }
    if let Some(system) = system {
        commands.push(format!(
            "hoi4skill query-symbol <mod-root> --symbol {system} --contains"
        ));
        commands.push(format!(
            "hoi4skill impact <mod-root> --symbol <{system}_symbol>"
        ));
    }
    commands
}

struct ImpactReport<'a> {
    query_symbol: Option<String>,
    changed_file: Option<String>,
    seed_symbols: Vec<&'a ModSymbol>,
    related_symbols: Vec<&'a ModSymbol>,
    references: Vec<ImpactReference>,
    affected_files: Vec<String>,
    validation_steps: Vec<String>,
}

struct ImpactReference {
    symbol: String,
    file: String,
    matches: usize,
    relation: String,
}

fn build_impact_report<'a>(
    root: &Path,
    index: &'a ModIndex,
    symbol: Option<&str>,
    changed: Option<&str>,
    max_symbols: usize,
    max_references: usize,
) -> Result<ImpactReport<'a>, String> {
    let changed_file = changed
        .map(|raw| normalize_changed_file(root, raw))
        .transpose()?;
    let mut seed_symbols = Vec::<&ModSymbol>::new();
    if let Some(symbol) = symbol {
        seed_symbols.extend(
            index
                .symbols
                .iter()
                .filter(|candidate| candidate.id == symbol)
                .take(max_symbols),
        );
    }
    if let Some(changed_file) = &changed_file {
        seed_symbols.extend(
            index
                .symbols
                .iter()
                .filter(|candidate| &candidate.file == changed_file)
                .take(max_symbols),
        );
    }
    seed_symbols = dedupe_symbol_refs(seed_symbols);
    seed_symbols.truncate(max_symbols);

    let mut related_symbols = Vec::new();
    for seed in &seed_symbols {
        related_symbols.extend(index.symbols.iter().filter(|candidate| {
            candidate.id != seed.id
                && (candidate.file == seed.file
                    || same_optional(&candidate.owner, &seed.owner)
                    || same_optional(&candidate.parent, &seed.parent))
        }));
    }
    related_symbols = dedupe_symbol_refs(related_symbols);
    related_symbols.truncate(max_symbols);

    let symbol_ids = seed_symbols
        .iter()
        .map(|symbol| symbol.id.clone())
        .collect::<Vec<_>>();
    let references = collect_symbol_references(root, &symbol_ids, max_references)?;
    let mut affected_files = BTreeSet::new();
    for seed in &seed_symbols {
        affected_files.insert(seed.file.clone());
    }
    for related in &related_symbols {
        affected_files.insert(related.file.clone());
    }
    for reference in &references {
        affected_files.insert(reference.file.clone());
    }
    if let Some(changed_file) = &changed_file {
        affected_files.insert(changed_file.clone());
    }
    let affected_files = affected_files.into_iter().collect::<Vec<_>>();
    let validation_steps = impact_validation_steps(&affected_files);
    Ok(ImpactReport {
        query_symbol: symbol.map(str::to_string),
        changed_file,
        seed_symbols,
        related_symbols,
        references,
        affected_files,
        validation_steps,
    })
}

fn query_mod_symbols<'a>(
    symbols: &'a [ModSymbol],
    query: &str,
    kind_filter: Option<&str>,
    contains: bool,
    max_results: usize,
) -> Vec<&'a ModSymbol> {
    let query_lower = query.to_ascii_lowercase();
    symbols
        .iter()
        .filter(|symbol| {
            kind_filter.is_none_or(|kind| symbol.kind == kind)
                && if contains {
                    symbol.id.to_ascii_lowercase().contains(&query_lower)
                        || symbol
                            .title
                            .as_deref()
                            .is_some_and(|title| title.to_ascii_lowercase().contains(&query_lower))
                } else {
                    symbol.id == query
                }
        })
        .take(max_results)
        .collect()
}

fn collect_symbol_references(
    root: &Path,
    symbols: &[String],
    max_references: usize,
) -> Result<Vec<ImpactReference>, String> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let mut references = Vec::new();
    for file in collect_files(root)? {
        if !is_text_index_file(&file) {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let rel = rel_slash(root, &file);
        for symbol in symbols {
            let matches = text.matches(symbol).count();
            if matches == 0 {
                continue;
            }
            references.push(ImpactReference {
                symbol: symbol.clone(),
                file: rel.clone(),
                matches,
                relation: "text_reference".to_string(),
            });
            if references.len() >= max_references {
                return Ok(references);
            }
        }
    }
    Ok(references)
}

fn collect_imported_content_symbols(
    root: &Path,
    localisation: &BTreeMap<String, String>,
    out: &mut Vec<ModSymbol>,
) -> Result<(), String> {
    for focus in import_focuses(root, localisation)? {
        let mut extra = BTreeMap::new();
        if let Some(icon) = focus.icon {
            extra.insert("icon".to_string(), icon);
        }
        if !focus.tree_id.is_empty() {
            extra.insert("tree_id".to_string(), focus.tree_id.clone());
        }
        out.push(ModSymbol {
            kind: "focus".to_string(),
            id: focus.id,
            file: focus.file,
            owner: none_if_empty(focus.country_tag),
            parent: none_if_empty(focus.tree_id),
            title: focus.title,
            extra,
        });
    }
    for event in import_events(root, localisation)? {
        let mut extra = BTreeMap::new();
        extra.insert("event_type".to_string(), event.event_type);
        if let Some(number) = event.number {
            extra.insert("number".to_string(), number.to_string());
        }
        if let Some(picture) = event.picture {
            extra.insert("picture".to_string(), picture);
        }
        out.push(ModSymbol {
            kind: "event".to_string(),
            id: event.id,
            file: event.file,
            owner: event.namespace.clone(),
            parent: event.namespace,
            title: event.title,
            extra,
        });
    }
    for idea in import_ideas(root, localisation)? {
        let mut extra = BTreeMap::new();
        if let Some(picture) = idea.picture {
            extra.insert("picture".to_string(), picture);
        }
        out.push(ModSymbol {
            kind: "idea".to_string(),
            id: idea.id,
            file: idea.file,
            owner: Some(idea.category.clone()),
            parent: Some(idea.category),
            title: idea.title,
            extra,
        });
    }
    for category in import_decision_categories(root, localisation)? {
        let mut extra = BTreeMap::new();
        if let Some(icon) = category.icon {
            extra.insert("icon".to_string(), icon);
        }
        out.push(ModSymbol {
            kind: "decision_category".to_string(),
            id: category.id,
            file: category.file,
            owner: None,
            parent: None,
            title: category.title,
            extra,
        });
    }
    for decision in import_decisions(root, localisation)? {
        let mut extra = BTreeMap::new();
        if let Some(icon) = decision.icon {
            extra.insert("icon".to_string(), icon);
        }
        if let Some(cost) = decision.cost {
            extra.insert("cost".to_string(), cost.to_string());
        }
        out.push(ModSymbol {
            kind: "decision".to_string(),
            id: decision.id,
            file: decision.file,
            owner: Some(decision.category.clone()),
            parent: Some(decision.category),
            title: decision.title,
            extra,
        });
    }
    Ok(())
}

fn collect_localisation_symbols(root: &Path, out: &mut Vec<ModSymbol>) -> Result<(), String> {
    let loc_root = root.join("localisation");
    if !loc_root.exists() {
        return Ok(());
    }
    for file in collect_files(&loc_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut keys = BTreeSet::new();
        collect_localisation_keys(&text, &mut keys);
        let rel = rel_slash(root, &file);
        let language = localisation_language_from_path(&rel);
        for key in keys {
            out.push(ModSymbol {
                kind: "localisation".to_string(),
                id: key,
                file: rel.clone(),
                owner: language.clone(),
                parent: language.clone(),
                title: None,
                extra: BTreeMap::new(),
            });
        }
    }
    Ok(())
}

fn collect_gfx_symbols(root: &Path, out: &mut Vec<ModSymbol>) -> Result<(), String> {
    let interface_root = root.join("interface");
    if !interface_root.exists() {
        return Ok(());
    }
    for file in collect_files(&interface_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "gfx" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let mut sprites = BTreeSet::new();
        collect_sprite_names(&text, &mut sprites);
        let rel = rel_slash(root, &file);
        for sprite in sprites {
            out.push(ModSymbol {
                kind: "sprite".to_string(),
                id: sprite,
                file: rel.clone(),
                owner: None,
                parent: None,
                title: None,
                extra: BTreeMap::new(),
            });
        }
    }
    Ok(())
}

fn collect_country_tag_symbols(root: &Path, out: &mut Vec<ModSymbol>) -> Result<(), String> {
    for file in txt_files(root, "common/country_tags")? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for line in text.lines() {
            let Some(tag) = assignment_key(line) else {
                continue;
            };
            if !looks_like_tag(tag) {
                continue;
            }
            out.push(ModSymbol {
                kind: "country_tag".to_string(),
                id: tag.to_string(),
                file: rel.clone(),
                owner: None,
                parent: None,
                title: None,
                extra: BTreeMap::new(),
            });
        }
    }
    Ok(())
}

fn collect_scripted_symbols(root: &Path, out: &mut Vec<ModSymbol>) -> Result<(), String> {
    collect_direct_definition_symbols(root, "common/scripted_effects", "scripted_effect", out)?;
    collect_direct_definition_symbols(root, "common/scripted_triggers", "scripted_trigger", out)?;
    collect_direct_definition_symbols(root, "common/on_actions", "on_action", out)?;
    Ok(())
}

fn collect_direct_definition_symbols(
    root: &Path,
    rel_dir: &str,
    kind: &str,
    out: &mut Vec<ModSymbol>,
) -> Result<(), String> {
    for file in txt_files(root, rel_dir)? {
        let text = strip_comments(&read_utf8_lossy(&file)?);
        let rel = rel_slash(root, &file);
        for (id, _) in direct_child_blocks(&text) {
            if !is_identifier_like(&id) || is_import_definition_field(&id) {
                continue;
            }
            out.push(ModSymbol {
                kind: kind.to_string(),
                id,
                file: rel.clone(),
                owner: None,
                parent: None,
                title: None,
                extra: BTreeMap::new(),
            });
        }
    }
    Ok(())
}

fn mod_index_json(resolved: &ModRootResolution, index: &ModIndex, max_symbols: usize) -> String {
    let returned = index.symbols.len().min(max_symbols);
    let mut by_kind = BTreeMap::<String, i64>::new();
    for symbol in &index.symbols {
        *by_kind.entry(symbol.kind.clone()).or_default() += 1;
    }
    let symbols = index
        .symbols
        .iter()
        .take(max_symbols)
        .map(symbol_json)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.mod_index.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"files_total\": {},\n  \"symbols_total\": {},\n  \"symbols_returned\": {},\n  \"symbols_truncated\": {},\n  \"localisation_keys_total\": {},\n  \"sprite_total\": {},\n  \"by_kind\": {},\n  \"symbols\": [{}]\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        index.files_total,
        index.symbols.len(),
        returned,
        json_bool(index.symbols.len() > max_symbols),
        index.localisation_keys_total,
        index.sprite_total,
        json_i64_object(&by_kind),
        symbols
    )
}

fn query_symbol_json(
    resolved: &ModRootResolution,
    query: &str,
    kind_filter: Option<&str>,
    contains: bool,
    index: &ModIndex,
    results: &[&ModSymbol],
) -> String {
    let mut by_kind = BTreeMap::<String, i64>::new();
    for symbol in results {
        *by_kind.entry(symbol.kind.clone()).or_default() += 1;
    }
    let definitions = results
        .iter()
        .map(|symbol| symbol_json(symbol))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.query_symbol.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"query\": {},\n  \"kind_filter\": {},\n  \"contains\": {},\n  \"matches\": {},\n  \"indexed_symbols_total\": {},\n  \"by_kind\": {},\n  \"definitions\": [{}]\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_str(query),
        json_optional_str(kind_filter),
        json_bool(contains),
        results.len(),
        index.symbols.len(),
        json_i64_object(&by_kind),
        definitions
    )
}

fn impact_report_json(resolved: &ModRootResolution, report: &ImpactReport<'_>) -> String {
    let seed_symbols = report
        .seed_symbols
        .iter()
        .map(|symbol| symbol_json(symbol))
        .collect::<Vec<_>>()
        .join(", ");
    let related_symbols = report
        .related_symbols
        .iter()
        .map(|symbol| symbol_json(symbol))
        .collect::<Vec<_>>()
        .join(", ");
    let references = report
        .references
        .iter()
        .map(impact_reference_json)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.impact.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"query_symbol\": {},\n  \"changed_file\": {},\n  \"seed_symbol_count\": {},\n  \"related_symbol_count\": {},\n  \"reference_count\": {},\n  \"affected_files\": {},\n  \"seed_symbols\": [{}],\n  \"related_symbols\": [{}],\n  \"references\": [{}],\n  \"validation_steps\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_optional_str(report.query_symbol.as_deref()),
        json_optional_str(report.changed_file.as_deref()),
        report.seed_symbols.len(),
        report.related_symbols.len(),
        report.references.len(),
        json_array(&report.affected_files),
        seed_symbols,
        related_symbols,
        references,
        json_array(&report.validation_steps)
    )
}

fn reserve_id_json(resolved: &ModRootResolution, reservation: &IdReservation) -> String {
    format!(
        "{{\n  \"schema\": \"hoi4skill.reserve_id.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"kind\": {},\n  \"namespace\": {},\n  \"prefix\": {},\n  \"count\": {},\n  \"ids\": {},\n  \"collisions_skipped\": {},\n  \"existing_event_max\": {},\n  \"notes\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_str(&reservation.kind),
        json_optional_str(reservation.namespace.as_deref()),
        json_optional_str(reservation.prefix.as_deref()),
        reservation.count,
        json_array(&reservation.ids),
        reservation.collisions_skipped,
        json_optional_i64(reservation.existing_event_max),
        json_array(&reservation.notes)
    )
}

fn namespace_check_json(
    resolved: &ModRootResolution,
    namespace_filter: Option<&str>,
    report: &NamespaceCheckReport,
) -> String {
    let namespaces = report
        .namespaces
        .iter()
        .map(namespace_check_item_json)
        .collect::<Vec<_>>()
        .join(", ");
    let duplicate_event_ids = report
        .duplicate_event_ids
        .iter()
        .map(duplicate_event_id_json)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.namespace_check.v1\",\n  \"mod_root\": {},\n  \"input\": {},\n  \"input_kind\": {},\n  \"namespace_filter\": {},\n  \"ok\": {},\n  \"namespace_count\": {},\n  \"duplicate_event_id_count\": {},\n  \"warning_count\": {},\n  \"namespaces\": [{}],\n  \"duplicate_event_ids\": [{}],\n  \"warnings\": {},\n  \"suggested_commands\": {}\n}}\n",
        json_str(&resolved.root.display().to_string()),
        json_str(&resolved.input.display().to_string()),
        json_str(&resolved.input_kind),
        json_optional_str(namespace_filter),
        json_bool(report.ok),
        report.namespaces.len(),
        report.duplicate_event_ids.len(),
        report.warnings.len(),
        namespaces,
        duplicate_event_ids,
        json_array(&report.warnings),
        json_array(&report.suggested_commands)
    )
}

fn namespace_check_item_json(item: &NamespaceCheck) -> String {
    format!(
        "{{\"namespace\": {}, \"declared_files\": {}, \"event_files\": {}, \"event_count\": {}, \"max_id\": {}, \"next_id\": {}, \"warnings\": {}}}",
        json_str(&item.namespace),
        json_array(&item.declared_files),
        json_array(&item.event_files),
        item.event_count,
        json_optional_i64(item.max_id),
        json_optional_i64(item.next_id),
        json_array(&item.warnings)
    )
}

fn duplicate_event_id_json(item: &DuplicateEventId) -> String {
    format!(
        "{{\"id\": {}, \"files\": {}}}",
        json_str(&item.id),
        json_array(&item.files)
    )
}

fn feature_context_markdown(resolved: &ModRootResolution, context: &FeatureContext<'_>) -> String {
    let mut out = String::new();
    out.push_str("# HOI4 Feature Context\n\n");
    out.push_str(&format!("- mod_root: `{}`\n", resolved.root.display()));
    if let Some(tag) = &context.tag {
        out.push_str(&format!("- tag: `{tag}`\n"));
    }
    if let Some(system) = &context.system {
        out.push_str(&format!("- system: `{system}`\n"));
    }
    out.push_str(&format!("- symbols: `{}`\n", context.symbols.len()));
    out.push_str(&format!("- references: `{}`\n", context.references.len()));
    out.push('\n');

    out.push_str("## Allowed Edit Surface\n\n");
    for path in &context.allowed_paths {
        out.push_str(&format!("- `{path}`\n"));
    }
    out.push_str("\n## Relevant Symbols\n\n");
    if context.symbols.is_empty() {
        out.push_str("- No indexed symbols matched this context.\n");
    } else {
        for symbol in &context.symbols {
            out.push_str(&format!(
                "- `{}` `{}` in `{}`",
                symbol.kind, symbol.id, symbol.file
            ));
            if let Some(owner) = &symbol.owner {
                out.push_str(&format!(" owner=`{owner}`"));
            }
            if let Some(title) = &symbol.title {
                out.push_str(&format!(" title={}", json_str(title)));
            }
            out.push('\n');
        }
    }

    out.push_str("\n## Referenced Files\n\n");
    if context.files.is_empty() {
        out.push_str("- No files found.\n");
    } else {
        for file in &context.files {
            out.push_str(&format!("- `{file}`\n"));
        }
    }

    out.push_str("\n## Text References\n\n");
    if context.references.is_empty() {
        out.push_str("- No text references found for matched symbols.\n");
    } else {
        for reference in &context.references {
            out.push_str(&format!(
                "- `{}` in `{}` matches `{}` time(s)\n",
                reference.symbol, reference.file, reference.matches
            ));
        }
    }

    out.push_str("\n## Suggested Commands\n\n");
    for command in &context.suggested_commands {
        out.push_str(&format!("- `{command}`\n"));
    }

    out.push_str("\n## Stop Conditions\n\n");
    out.push_str("- Do not create country tags, country history, state history, map data, GUI, technologies, or extra systems unless the literal request or blueprint authorizes them.\n");
    out.push_str("- Before writing gameplay script, run local game/dependency evidence commands and final validation.\n");
    out.push_str("- Missing user-provided player-visible text is unfinished work; run text alignment checks when applicable.\n");
    out
}

fn impact_reference_json(reference: &ImpactReference) -> String {
    format!(
        "{{\"symbol\": {}, \"file\": {}, \"matches\": {}, \"relation\": {}}}",
        json_str(&reference.symbol),
        json_str(&reference.file),
        reference.matches,
        json_str(&reference.relation)
    )
}

fn symbol_json(symbol: &ModSymbol) -> String {
    format!(
        "{{\"kind\": {}, \"id\": {}, \"file\": {}, \"owner\": {}, \"parent\": {}, \"title\": {}, \"extra\": {}}}",
        json_str(&symbol.kind),
        json_str(&symbol.id),
        json_str(&symbol.file),
        json_optional_str(symbol.owner.as_deref()),
        json_optional_str(symbol.parent.as_deref()),
        json_optional_str(symbol.title.as_deref()),
        json_object(&symbol.extra)
    )
}

fn normalize_changed_file(root: &Path, raw: &str) -> Result<String, String> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(relative_slash_path(root, &path))
    } else {
        Ok(slash_path(&path))
    }
}

fn dedupe_symbol_refs(symbols: Vec<&ModSymbol>) -> Vec<&ModSymbol> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for symbol in symbols {
        let key = format!("{}\0{}\0{}", symbol.kind, symbol.id, symbol.file);
        if seen.insert(key) {
            out.push(symbol);
        }
    }
    out
}

fn same_optional(left: &Option<String>, right: &Option<String>) -> bool {
    left.is_some() && left == right
}

fn is_text_index_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "txt" | "yml" | "yaml" | "gfx" | "gui" | "csv"
    )
}

fn impact_validation_steps(affected_files: &[String]) -> Vec<String> {
    let mut steps = vec![
        "hoi4skill build-mod-index <mod-root> --output mod_index.json".to_string(),
        "hoi4skill validate <mod-root> --changed-only --strict-code-index".to_string(),
    ];
    if affected_files
        .iter()
        .any(|file| file.starts_with("events/"))
    {
        steps.push(
            "launch HOI4 once and run hoi4skill analyze-error-log --changed-only".to_string(),
        );
    }
    if affected_files
        .iter()
        .any(|file| file.starts_with("interface/") || file.starts_with("gfx/"))
    {
        steps.push(
            "run hoi4skill gfx-audit when available or validate sprite references".to_string(),
        );
    }
    if affected_files
        .iter()
        .any(|file| file.starts_with("localisation/"))
    {
        steps.push(
            "run hoi4skill validate --text-source when user-provided player text exists"
                .to_string(),
        );
    }
    steps
}

fn normalize_namespace(value: &str) -> String {
    slugify(value, "")
}

fn normalize_id_prefix(value: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_us = false;
        } else if !last_us {
            out.push('_');
            last_us = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn none_if_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn localisation_language_from_path(rel: &str) -> Option<String> {
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.len() >= 2 && parts[0] == "localisation" {
        Some(parts[1].to_string())
    } else {
        None
    }
}
