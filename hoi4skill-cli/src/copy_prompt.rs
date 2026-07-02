//! Prompt builders that learn focus and national-spirit copywriting style from local mods.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_focus_copy_prompt(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let options = FocusCopyPromptOptions::from_args(&map)?;
    let mut roots = map
        .positionals
        .iter()
        .map(|root| normalize_path(root))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(root) = value(&map, "mod-root") {
        roots.push(normalize_path(root)?);
    }
    if roots.is_empty() {
        return Err("missing mod root; pass one or more mod paths".to_string());
    }

    let mut entries = Vec::new();
    for root in &roots {
        entries.extend(scan_focus_copy_entries(root)?);
    }
    let markdown = render_focus_copy_prompt(&roots, &entries, &options);
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_idea_copy_prompt(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let options = FocusCopyPromptOptions::from_args(&map)?;
    let mut roots = map
        .positionals
        .iter()
        .map(|root| normalize_path(root))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(root) = value(&map, "mod-root") {
        roots.push(normalize_path(root)?);
    }
    if roots.is_empty() {
        return Err("missing mod root; pass one or more mod paths".to_string());
    }

    let mut resolved_roots = Vec::new();
    let mut entries = Vec::new();
    let all_categories = map.flags.contains("all-categories");
    for root in &roots {
        let resolved = resolve_mod_root(root)?;
        entries.extend(scan_idea_copy_entries(&resolved.root, all_categories)?);
        resolved_roots.push(resolved.root);
    }
    let markdown = render_idea_copy_prompt(&resolved_roots, &entries, &options, all_categories);
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_event_copy_prompt(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let options = FocusCopyPromptOptions::from_args(&map)?;
    let language = value(&map, "language").unwrap_or("all");
    validate_copy_prompt_language(language)?;
    let template = value(&map, "template")
        .or_else(|| value(&map, "event-template"))
        .unwrap_or("auto");
    validate_event_copy_prompt_template(template)?;
    let mut roots = map
        .positionals
        .iter()
        .map(|root| normalize_path(root))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(root) = value(&map, "mod-root") {
        roots.push(normalize_path(root)?);
    }
    if roots.is_empty() {
        return Err("missing mod root; pass one or more mod paths".to_string());
    }

    let mut resolved_roots = Vec::new();
    let mut entries = Vec::new();
    for root in &roots {
        let resolved = resolve_mod_root(root)?;
        entries.extend(scan_event_copy_entries_with_language(
            &resolved.root,
            language,
        )?);
        resolved_roots.push(resolved.root);
    }
    let markdown =
        render_event_copy_prompt(&resolved_roots, &entries, &options, language, template);
    write_or_print(&markdown, value(&map, "output"))
}

pub(crate) fn cmd_event_style_profile(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let language = value(&map, "language").unwrap_or("all");
    validate_copy_prompt_language(language)?;
    let template = value(&map, "template")
        .or_else(|| value(&map, "event-template"))
        .unwrap_or("auto");
    validate_event_copy_prompt_template(template)?;
    let format = value(&map, "format").unwrap_or("markdown");
    if !matches!(format, "markdown" | "md" | "json") {
        return Err(format!(
            "unsupported event-style-profile format `{format}`; expected markdown or json"
        ));
    }
    let mut roots = map
        .positionals
        .iter()
        .map(|root| normalize_path(root))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(root) = value(&map, "mod-root") {
        roots.push(normalize_path(root)?);
    }
    if roots.is_empty() {
        return Err("missing mod root; pass one or more mod paths".to_string());
    }

    let mut resolved_roots = Vec::new();
    let mut entries = Vec::new();
    for root in &roots {
        let resolved = resolve_mod_root(root)?;
        entries.extend(scan_event_copy_entries_with_language(
            &resolved.root,
            language,
        )?);
        resolved_roots.push(resolved.root);
    }
    let output = render_event_style_profile(&resolved_roots, &entries, language, template, format);
    write_or_print(&output, value(&map, "output"))
}

pub(crate) fn cmd_work_package_style_context(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let language = value(&map, "language").unwrap_or("all");
    validate_copy_prompt_language(language)?;
    let template = value(&map, "template")
        .or_else(|| value(&map, "event-template"))
        .unwrap_or("auto");
    validate_event_copy_prompt_template(template)?;
    let mut roots = map
        .positionals
        .iter()
        .map(|root| normalize_path(root))
        .collect::<Result<Vec<_>, _>>()?;
    for root in repeated_values(&map, "style-mod") {
        roots.push(normalize_path(root)?);
    }
    for root in repeated_values(&map, "style-root") {
        roots.push(normalize_path(root)?);
    }
    if roots.is_empty() {
        return Err("missing style mod root; pass one or more paths or --style-mod".to_string());
    }

    let mut resolved_roots = Vec::new();
    let mut focus_entries = Vec::new();
    let mut idea_entries = Vec::new();
    let mut event_entries = Vec::new();
    for root in &roots {
        let resolved = resolve_mod_root(root)?;
        focus_entries.extend(scan_focus_copy_entries(&resolved.root)?);
        idea_entries.extend(scan_idea_copy_entries(&resolved.root, false)?);
        event_entries.extend(scan_event_copy_entries_with_language(
            &resolved.root,
            language,
        )?);
        resolved_roots.push(resolved.root);
    }
    let markdown = render_work_package_style_context(
        &resolved_roots,
        &focus_entries,
        &idea_entries,
        &event_entries,
        language,
        template,
    );
    write_or_print(&markdown, value(&map, "output"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusCopyPromptStyle {
    Full,
    Compact,
}

pub(crate) struct FocusCopyPromptOptions {
    pub(crate) title_examples: usize,
    pub(crate) sample_keys: usize,
    pub(crate) style: FocusCopyPromptStyle,
}

impl FocusCopyPromptOptions {
    pub(crate) fn from_args(map: &ArgMap) -> Result<Self, String> {
        let style = match value(map, "style").unwrap_or("full") {
            "full" => FocusCopyPromptStyle::Full,
            "compact" => FocusCopyPromptStyle::Compact,
            other => return Err(format!("unsupported focus-copy-prompt style: {other}")),
        };
        Ok(Self {
            title_examples: parse_usize_option(map, "title-examples", 16)?,
            sample_keys: parse_usize_option(map, "sample-keys", 8)?,
            style,
        })
    }
}

#[derive(Clone)]
pub(crate) struct FocusCopyEntry {
    pub(crate) mod_name: String,
    pub(crate) file: String,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
}

#[derive(Clone)]
pub(crate) struct IdeaCopyEntry {
    pub(crate) mod_name: String,
    pub(crate) file: String,
    pub(crate) category: String,
    pub(crate) id: String,
    pub(crate) picture: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
}

#[derive(Clone)]
pub(crate) struct EventCopyEntry {
    pub(crate) mod_name: String,
    pub(crate) file: String,
    pub(crate) event_type: String,
    pub(crate) id: String,
    pub(crate) title: Option<String>,
    pub(crate) desc: Option<String>,
    pub(crate) picture: Option<String>,
    pub(crate) option_names: Vec<String>,
}

pub(crate) fn scan_focus_copy_entries(root: &Path) -> Result<Vec<FocusCopyEntry>, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let mod_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("mod")
        .to_string();
    let localisation = collect_focus_localisation_map(root)?;
    let focus_root = root.join("common").join("national_focus");
    let mut entries = Vec::new();
    if !focus_root.exists() {
        return Ok(entries);
    }

    for file in collect_files(&focus_root)? {
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        let file_name = file
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("focus.txt")
            .to_string();
        for block in blocks_named(&text, "focus") {
            if let Some(id) = block_assignment(&block, "id") {
                let title = localisation.get(&id).cloned();
                let desc = localisation.get(&format!("{id}_desc")).cloned();
                if title.is_some() || desc.is_some() {
                    entries.push(FocusCopyEntry {
                        mod_name: mod_name.clone(),
                        file: file_name.clone(),
                        id,
                        title,
                        desc,
                    });
                }
            }
        }
    }
    Ok(entries)
}

pub(crate) fn scan_idea_copy_entries(
    root: &Path,
    all_categories: bool,
) -> Result<Vec<IdeaCopyEntry>, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let mod_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("mod")
        .to_string();
    let localisation = collect_focus_localisation_map(root)?;
    let ideas = import_ideas(root, &localisation)?;
    Ok(ideas
        .into_iter()
        .filter(|idea| all_categories || is_national_spirit_category(&idea.category))
        .filter(|idea| idea.title.is_some() || idea.desc.is_some())
        .map(|idea| IdeaCopyEntry {
            mod_name: mod_name.clone(),
            file: idea.file,
            category: idea.category,
            id: idea.id,
            picture: idea.picture,
            title: idea.title,
            desc: idea.desc,
        })
        .collect())
}

#[allow(dead_code)]
pub(crate) fn scan_event_copy_entries(root: &Path) -> Result<Vec<EventCopyEntry>, String> {
    scan_event_copy_entries_with_language(root, "simp_chinese")
}

pub(crate) fn scan_event_copy_entries_with_language(
    root: &Path,
    language: &str,
) -> Result<Vec<EventCopyEntry>, String> {
    if !root.exists() {
        return Err(format!("{}: mod root does not exist", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("{}: mod root is not a directory", root.display()));
    }

    let mod_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("mod")
        .to_string();
    let localisation = collect_copy_localisation_map(root, language)?;
    Ok(import_events(root, &localisation)?
        .into_iter()
        .filter(|event| {
            event.title.is_some()
                || event.desc.is_some()
                || event.options.iter().any(|option| option.name.is_some())
        })
        .map(|event| EventCopyEntry {
            mod_name: mod_name.clone(),
            file: event.file,
            event_type: event.event_type,
            id: event.id,
            title: event.title,
            desc: event.desc,
            picture: event.picture,
            option_names: event
                .options
                .into_iter()
                .filter_map(|option| option.name)
                .collect(),
        })
        .collect())
}

pub(crate) fn validate_copy_prompt_language(language: &str) -> Result<(), String> {
    if language == "all"
        || (!language.is_empty()
            && language
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
    {
        Ok(())
    } else {
        Err(format!(
            "unsupported localisation language filter `{language}`; use simp_chinese, english, or all"
        ))
    }
}

pub(crate) fn validate_event_copy_prompt_template(template: &str) -> Result<(), String> {
    if event_copy_template_specs()
        .iter()
        .any(|spec| spec.id == template)
    {
        return Ok(());
    }
    let supported = event_copy_template_specs()
        .iter()
        .map(|spec| spec.id)
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "unsupported event-copy-prompt template `{template}`; expected one of: {supported}"
    ))
}

pub(crate) struct EventCopyTemplateSpec {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) use_when: &'static str,
    pub(crate) structure: &'static str,
}

pub(crate) fn event_copy_template_specs() -> &'static [EventCopyTemplateSpec] {
    &[
        EventCopyTemplateSpec {
            id: "auto",
            title: "Auto-select from event intent",
            use_when: "the user has not chosen a fixed form",
            structure: "infer the best skeleton from event type, scene source, conflict, and requested tone",
        },
        EventCopyTemplateSpec {
            id: "historical_report",
            title: "Historical report",
            use_when: "institutional change, cabinet crisis, congress, reform, military order, or diplomatic shift",
            structure: "source or document -> named actors -> institutional stakes -> decision pressure",
        },
        EventCopyTemplateSpec {
            id: "political_drama",
            title: "Political drama",
            use_when: "faction struggle, betrayal, ideological split, coup rumor, purge, street confrontation, or party meeting",
            structure: "tense scene -> rival positions -> escalation -> player stance",
        },
        EventCopyTemplateSpec {
            id: "weird_route",
            title: "Weird-route serious tone",
            use_when: "strange alternate-history premise that must still read like an in-universe state document",
            structure: "absurd premise treated as official fact -> rationalising actors -> consequences -> unnerving choice",
        },
        EventCopyTemplateSpec {
            id: "diplomatic_report",
            title: "Diplomatic report",
            use_when: "embassy cable, ultimatum, recognition dispute, conference, treaty, border crisis, or foreign mission",
            structure: "telegram or communique -> competing readings -> negotiated ambiguity -> response",
        },
        EventCopyTemplateSpec {
            id: "revolutionary_scene",
            title: "Revolutionary scene",
            use_when: "strike, uprising, militia mobilisation, committee vote, land reform, speech, arrest, or mass campaign",
            structure: "street or meeting detail -> slogans and organisers -> class or party stakes -> mobilisation order",
        },
        EventCopyTemplateSpec {
            id: "news_bulletin",
            title: "News bulletin",
            use_when: "news_event, public announcement, foreign headline, victory, disaster, assassination, or proclamation",
            structure: "headline fact -> immediate public reaction -> international or domestic meaning -> closing judgement",
        },
        EventCopyTemplateSpec {
            id: "internal_meeting",
            title: "Internal meeting",
            use_when: "closed-door debate, committee dispute, planning session, interrogation, or emergency cabinet meeting",
            structure: "room and participants -> argument lines -> unresolved risk -> chair's decision",
        },
    ]
}

pub(crate) fn push_event_builtin_templates(out: &mut String, selected_template: &str) {
    out.push_str("\n## Built-in Event Templates\n\n");
    out.push_str(
        "Use the selected built-in template as the structural skeleton, then adapt its voice using the user-specified mod samples in this prompt. Samples are for style learning only; do not copy their text.\n\n",
    );
    out.push_str(&format!("Selected template: `{selected_template}`\n\n"));
    for spec in event_copy_template_specs() {
        out.push_str(&format!(
            "- `{}`: {}. Use when {}; skeleton: {}.\n",
            spec.id, spec.title, spec.use_when, spec.structure
        ));
    }
}

pub(crate) fn collect_copy_localisation_map(
    root: &Path,
    language: &str,
) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    let loc_root = root.join("localisation");
    if !loc_root.exists() {
        return Ok(map);
    }
    let underscore_suffix = format!("_l_{language}.yml");
    let spaced_suffix = format!(" l_{language}.yml");
    for file in collect_files(&loc_root)? {
        let norm = slash_path(&file);
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "yml" | "yaml") {
            continue;
        }
        if language != "all"
            && !norm.ends_with(&underscore_suffix)
            && !norm.ends_with(&spaced_suffix)
        {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        collect_localisation_map(&text, &mut map);
    }
    Ok(map)
}

pub(crate) fn is_national_spirit_category(category: &str) -> bool {
    matches!(category, "country" | "hidden_ideas")
}

pub(crate) fn collect_focus_localisation_map(
    root: &Path,
) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    let loc_root = root.join("localisation");
    if !loc_root.exists() {
        return Ok(map);
    }
    for file in collect_files(&loc_root)? {
        let norm = slash_path(&file);
        let ext = file
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "yml" | "yaml") || !norm.ends_with("simp_chinese.yml") {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        collect_localisation_map(&text, &mut map);
    }
    Ok(map)
}

pub(crate) fn collect_localisation_map(text: &str, map: &mut BTreeMap<String, String>) {
    for line in text.lines() {
        if let Some((key, value)) = parse_localisation_line(line) {
            map.insert(key, value);
        }
    }
}

pub(crate) fn parse_localisation_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start_matches('\u{feff}').trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    if trimmed.starts_with("l_") && trimmed.ends_with(':') {
        return None;
    }
    let colon = trimmed.find(':')?;
    let key = trimmed[..colon].trim();
    if key.is_empty() {
        return None;
    }
    let mut rest = trimmed[colon + 1..].trim_start();
    while rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        rest = &rest[1..];
    }
    rest = rest.trim_start();
    let value = parse_quoted_value(rest)?;
    Some((key.to_string(), value))
}

pub(crate) fn parse_quoted_value(value: &str) -> Option<String> {
    let quoted = value.strip_prefix('"')?;
    let mut out = String::new();
    let mut escape = false;
    for ch in quoted.chars() {
        if ch == '"' && !escape {
            return Some(out);
        }
        if escape {
            out.push(match ch {
                'n' => '\n',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escape = false;
        } else if ch == '\\' {
            escape = true;
        } else {
            out.push(ch);
        }
    }
    None
}

pub(crate) fn render_focus_copy_prompt(
    roots: &[PathBuf],
    entries: &[FocusCopyEntry],
    options: &FocusCopyPromptOptions,
) -> String {
    let with_desc = entries.iter().filter(|entry| entry.desc.is_some()).count();
    let desc_lengths = entries
        .iter()
        .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
        .collect::<Vec<_>>();
    let avg_len = average_usize(&desc_lengths);
    let median_len = median_usize(desc_lengths.clone());
    let title_examples = focus_title_examples(entries, options.title_examples);
    let mod_stats = focus_copy_mod_stats(entries);

    let mut out = String::new();
    out.push_str("# HOI4 Chinese Focus Copywriting Prompt\n\n");
    out.push_str("This prompt was generated from local national focus localisation.\n\n");
    out.push_str("## Style Sample Mods\n\n");
    for root in roots {
        out.push_str(&format!("- `{}`\n", root.display()));
    }
    out.push_str("\n## Learned Statistics\n\n");
    out.push_str(&format!(
        "- Matched focus localisation entries: {}\n",
        entries.len()
    ));
    out.push_str(&format!("- Entries with descriptions: {with_desc}\n"));
    out.push_str(&format!(
        "- Average description length: {avg_len:.1} Chinese characters\n"
    ));
    out.push_str(&format!(
        "- Median description length: {} Chinese characters\n",
        median_len.unwrap_or(0)
    ));
    for stat in mod_stats {
        out.push_str(&format!(
            "- `{}`: matched {}, with descriptions {}, average length {:.1}\n",
            stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
        ));
    }

    if !title_examples.is_empty() {
        out.push_str("\n## Title Examples\n\n");
        out.push_str("Use these as title-shape references, not text to copy:\n\n");
        for title in title_examples {
            out.push_str(&format!("- `{title}`\n"));
        }
    }

    let sample_rows = focus_sample_rows(entries, options.sample_keys);
    if !sample_rows.is_empty() {
        out.push_str("\n## Sample Focus Keys\n\n");
        out.push_str(
            "These identify the learned sample shape without copying existing descriptions:\n\n",
        );
        for row in sample_rows {
            out.push_str(&format!(
                "- `{}` in `{}` -> `{}` (desc chars: {})\n",
                row.id, row.file, row.title, row.desc_len
            ));
        }
    }

    if options.style == FocusCopyPromptStyle::Full {
        push_focus_copy_style_guide(&mut out);
    }

    out.push_str("\n## Prompt\n\n");
    out.push_str(
        r#"```text
你是钢铁雄心4中文国策文案作者。请按我的本地 mod 文案风格，为下面的国策写中文标题与描述。

风格要求：
- 写成 HOI4 架空历史国策文案，不要写成现代说明书。
- 标题短促有力，像政策名、政治口号、路线名、运动名或人物路线标签。
- 描述采用“历史矛盾/现实困境 -> 阶级或制度解释 -> 政策必要性 -> 行动或历史方向”的结构。
- 语气要像政治史论、路线斗争、革命宣言或国家建设文件，允许有讽刺，但不要网文腔。
- 必须以本国、本路线或本利益集团的内部第一视角写；可以使用“我们”、党、政府、军队、共和国等自我称谓。
- 不要直接列游戏效果，不要说“该国策将给予...”，除非用户明确要求机制说明。
- 不要抄已有文案，保持同类节奏与词汇质感即可。
- 默认描述 100-180 字；如果是路线终点、党代会、宪法、主席/领袖国策，可写 220-360 字。

硬性禁止：
- 不准输出“具体效果待补充”“描述”“TODO”“占位”“暂无”等占位符。
- 不准使用“先做可校验 demo”“保守脚本骨架”“之后补回文案/路线叙事”作为交付策略或自我解释。
- 不准把可编译、可校验的骨架当成完成品；生成前必须先抽取路线叙事，最终输出必须包含完成态标题、描述、本地化和脚本。
- 不准输出第三方视角、百科视角、历史学者旁白或“他们/该国/该政权将...”式外部评价。
- 不准只改标题不写描述；描述必须是完成态、可直接放进本地化文件的风格化国策文案。
- 不准把国策描述写成纯机制说明或“完成后获得...”的效果列表。
- 不准生成与已有国策重复的国策ID；如目标 mod 已有相同 focus id，必须改为安全后缀并同步更新 `{focus_id}_desc`。
- 不准使用 `sov_nep_l_simp_chinese.yml` 这类 prefix 本地化文件名；国家内容必须写入 `<TAG>_l_simp_chinese.yml`。
- 不准在 `l_simp_chinese:` 下生成 `<prefix>_mod_name`、`chinaprc_1979_mod_name` 或任何 `*_mod_name`；mod 名称只写在 `descriptor.mod` 和外层 `.mod` 文件。

输入：
国家/势力：{国家或势力}
时间线背景：{世界线背景}
所属路线：{政治路线或分支}
国策ID：{focus_id}
国策作用：{这个国策在剧情/机制上的作用}
前置矛盾：{上一阶段的问题或争论}
希望语气：{historical_policy / ideological_debate / revolutionary_mobilisation / strange_route}
关键词：{必须出现或可参考的词}
长度：{短/中/长，可省略}

输出：
1. 标题
2. 描述
3. 若需要写入本地化，给出：
   {focus_id}:0 "标题"
   {focus_id}_desc:0 "描述"
```
"#,
    );
    out
}

pub(crate) fn push_focus_copy_style_guide(out: &mut String) {
    out.push_str("\n## Learned Style Guide\n\n");
    out.push_str(
        "Write like a Chinese HOI4 alternate-history mod, not like a product summary.\n\n",
    );
    out.push_str("Core structure:\n\n");
    out.push_str("1. Start from a historical wound, institutional contradiction, class conflict, military crisis, or factional debate.\n");
    out.push_str("2. Interpret it through the regime's ideology or political line.\n");
    out.push_str("3. State why the new policy is necessary.\n");
    out.push_str("4. End with action, consolidation, rupture, or a new historical direction.\n\n");
    out.push_str("Perspective rules:\n\n");
    out.push_str("- Write from inside the target country, regime, route, army, party, faction, or interest group.\n");
    out.push_str("- Avoid third-party observer, encyclopedia, or historian narration; the text should sound like an in-universe political force justifying itself.\n\n");
    out.push_str("Title rules:\n\n");
    out.push_str("- Prefer short titles: slogans, policy phrases, doctrine names, movement names, or person-route labels.\n");
    out.push_str("- Avoid bland titles such as `发展经济`, `加强军队`, or `改善工业` unless the user intentionally wants plain placeholder copy.\n");
    out.push_str("- Do not explain raw effects in the title.\n\n");
    out.push_str("Length rules:\n\n");
    out.push_str(
        "- Minor industry, army, research, or administrative focus: 60-110 Chinese characters.\n",
    );
    out.push_str("- Normal political focus: 90-160 Chinese characters.\n");
    out.push_str("- Ideological branch focus: 140-260 Chinese characters.\n");
    out.push_str("- Route climax, chairman focus, constitutional focus, or civil-war settlement: 220-360 Chinese characters.\n\n");
    out.push_str("Useful cadence:\n\n");
    out.push_str("- `自...以来，...便...`\n");
    out.push_str("- `毫无疑问，...`\n");
    out.push_str("- `然而，...`\n");
    out.push_str("- `或许...，但...`\n");
    out.push_str("- `因此，...`\n");
    out.push_str("- `我们必须...`\n");
    out.push_str("- `这不是...，而是...`\n");
    out.push_str("- `只有...，才能...`\n\n");
    out.push_str("Vocabulary texture:\n\n");
    out.push_str("- 党、国家、共和国、人民、群众、工人阶级、农民、干部、官僚、先锋队\n");
    out.push_str("- 帝国主义、反动派、军阀、买办、寡头、资产阶级、封建残余、特权阶层\n");
    out.push_str("- 新民主主义、社会主义民主、苏维埃民主、人民民主专政、无产阶级专政\n");
    out.push_str("- 整顿、重组、改造、清算、巩固、统一、动员、普及、确立、推进\n\n");
    out.push_str("Tone modes:\n\n");
    out.push_str(
        "- `historical_policy`: sober state-building, administration, diplomacy, army reform.\n",
    );
    out.push_str("- `ideological_debate`: faction choices, doctrine focuses, party congresses.\n");
    out.push_str("- `revolutionary_mobilisation`: war, uprising, liberation, anti-imperialism.\n");
    out.push_str(
        "- `strange_route`: absurd or experimental paths, still written as in-universe policy.\n\n",
    );
    out.push_str("Mechanic routing rules:\n\n");
    out.push_str("- 国策是行动节点；即时奖励可以写入 `completion_reward`。\n");
    out.push_str("- 长期修正不属于国策树本体：要生成或引用民族精神，由国策 `completion_reward` 使用 `add_ideas` 添加。\n");
    out.push_str(
        "- 如果长期效果只是阶段性状态，必须说明结束国策、事件或决议用 `remove_ideas` 移除。\n",
    );
    out.push_str("- 不要把 `modifier = { ... }` 直接写进国策 `completion_reward`。\n\n");
    out.push_str("Quality checklist:\n\n");
    out.push_str("- The title fits a focus tooltip and is not a full sentence.\n");
    out.push_str("- The description does not mention raw game effects.\n");
    out.push_str("- The first sentence creates historical or political pressure.\n");
    out.push_str("- The middle sentence explains why the chosen line is necessary.\n");
    out.push_str("- The last sentence gives direction, not just summary.\n");
    out.push_str("- The text sounds like an in-universe faction justifying itself.\n");
}

pub(crate) fn render_idea_copy_prompt(
    roots: &[PathBuf],
    entries: &[IdeaCopyEntry],
    options: &FocusCopyPromptOptions,
    all_categories: bool,
) -> String {
    let with_desc = entries.iter().filter(|entry| entry.desc.is_some()).count();
    let desc_lengths = entries
        .iter()
        .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
        .collect::<Vec<_>>();
    let avg_len = average_usize(&desc_lengths);
    let median_len = median_usize(desc_lengths.clone());
    let title_examples = idea_title_examples(entries, options.title_examples);
    let mod_stats = idea_copy_mod_stats(entries);
    let category_counts = idea_category_counts(entries);

    let mut out = String::new();
    out.push_str("# HOI4 Chinese National Spirit Copywriting Prompt\n\n");
    out.push_str("This prompt was generated from local `common/ideas` localisation.\n\n");
    out.push_str("## Source Mods\n\n");
    for root in roots {
        out.push_str(&format!("- `{}`\n", root.display()));
    }
    out.push_str("\n## Learned Statistics\n\n");
    out.push_str(&format!(
        "- Matched idea localisation entries: {}\n",
        entries.len()
    ));
    out.push_str(&format!("- Entries with descriptions: {with_desc}\n"));
    out.push_str(&format!(
        "- Average description length: {avg_len:.1} Chinese characters\n"
    ));
    out.push_str(&format!(
        "- Median description length: {} Chinese characters\n",
        median_len.unwrap_or(0)
    ));
    out.push_str(&format!(
        "- Category filter: {}\n",
        if all_categories {
            "all idea categories"
        } else {
            "national spirits only (`country`, `hidden_ideas`)"
        }
    ));
    for stat in mod_stats {
        out.push_str(&format!(
            "- `{}`: matched {}, with descriptions {}, average length {:.1}\n",
            stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
        ));
    }
    if !category_counts.is_empty() {
        out.push_str("- Categories:");
        for (category, count) in category_counts {
            out.push_str(&format!(" `{category}`={count}"));
        }
        out.push('\n');
    }

    if !title_examples.is_empty() {
        out.push_str("\n## Title Examples\n\n");
        out.push_str("Use these as national-spirit title-shape references, not text to copy:\n\n");
        for title in title_examples {
            out.push_str(&format!("- `{title}`\n"));
        }
    }

    let sample_rows = idea_sample_rows(entries, options.sample_keys);
    if !sample_rows.is_empty() {
        out.push_str("\n## Sample Idea Keys\n\n");
        out.push_str(
            "These identify the learned sample shape without copying existing descriptions:\n\n",
        );
        for row in sample_rows {
            let picture = row.picture.unwrap_or_else(|| "<none>".to_string());
            out.push_str(&format!(
                "- `{}` in `{}` [{}] -> `{}` (picture: `{}`, desc chars: {})\n",
                row.id, row.file, row.category, row.title, picture, row.desc_len
            ));
        }
    }

    if options.style == FocusCopyPromptStyle::Full {
        push_idea_copy_style_guide(&mut out);
    }

    out.push_str("\n## Prompt\n\n");
    out.push_str(
        r#"```text
你是钢铁雄心4中文民族精神文案作者。请按我的本地 mod 文案风格，为下面的民族精神写中文名称与描述。

重要区分：
- 国策是“行动、路线推进、政策选择”。
- 民族精神是“国家长期状态、制度结构、社会矛盾、动员结果、改革后果或历史包袱”。
- 不要把民族精神写成“我们将要做什么”的国策描述；要写成“这个国家现在处于什么状态，以及这种状态为什么存在”。
- 当国策需要长期修正效果时，民族精神承载 `modifier`，国策只用 `add_ideas` 添加；若是临时状态，必须给出结束国策/事件/决议里的 `remove_ideas` 边界。

风格要求：
- 写成 HOI4 架空历史 MOD 的状态说明，不要写成现代产品说明或数值说明。
- 名称像一种制度、社会状态、政治病灶、动员氛围、改革成果或历史遗产。
- 描述采用“状态来源/历史根源 -> 当前表现 -> 对国家或社会的影响”的结构。
- 可以有政治史论和意识形态判断，但语气应比国策更凝练、更像 tooltip 里的国家状态说明。
- 不要直接列游戏效果，不要说“该民族精神给予...”，除非用户明确要求机制说明。
- 不要抄已有文案，保持同类节奏与词汇质感即可。
- 默认描述 60-140 字；严重危机、路线遗产或核心制度可写 140-220 字。

硬性禁止：
- 不准输出“正在影响国家”“描述”“TODO”“占位”“暂无”等泛泛占位文本。
- 不准把民族精神写成国策行动、未来政策承诺、任务目标或“我们必须...”式推进口号。
- 不准只写机制，不准直接罗列稳定度、战争支持、工厂、政治点等数值效果。
- 民族精神ID必须以 `_idea` 结尾；若输入不是 `_idea`，输出时必须规范化。
- 不准使用 `sov_nep_l_simp_chinese.yml` 这类 prefix 本地化文件名；国家内容必须写入 `<TAG>_l_simp_chinese.yml` 的“民族精神”分区。
- 不准在 `l_simp_chinese:` 下生成 `<prefix>_mod_name`、`chinaprc_1979_mod_name` 或任何 `*_mod_name`；mod 名称只写在 `descriptor.mod` 和外层 `.mod` 文件。

输入：
国家/势力：{国家或势力}
时间线背景：{世界线背景}
民族精神ID：{idea_id，必须以 _idea 结尾，例如 FER_fragmented_railway_authority_idea}
民族精神性质：{正面 / 负面 / 混合 / 隐藏 / 临时}
来源：{开局状态 / 国策获得 / 事件获得 / 决议获得 / 路线结果}
状态含义：{它代表的制度、社会矛盾、改革成果或历史包袱}
机制含义：{稳定度、战争支持、工厂、军队、政治点等效果的自然语言解释}
希望语气：{structural_crisis / reform_momentum / wartime_mobilisation / ideological_legacy / recovery_state}
关键词：{必须出现或可参考的词}
长度：{短/中/长，可省略}

输出：
1. 名称
2. 描述
3. 本地化（放在该国家本地化文件的“民族精神”分区，不要混到国策分区）：
   {idea_id}:0 "名称"
   {idea_id}_desc:0 "描述"
```
"#,
    );
    out
}

pub(crate) fn push_idea_copy_style_guide(out: &mut String) {
    out.push_str("\n## Learned National Spirit Style Guide\n\n");
    out.push_str("Write the idea as a persistent condition, not a focus action.\n\n");
    out.push_str("Core structure:\n\n");
    out.push_str(
        "1. Name the historical, institutional, military, or social root of the condition.\n",
    );
    out.push_str(
        "2. Explain how it currently appears inside the state, army, economy, party, or society.\n",
    );
    out.push_str("3. End with the pressure, limitation, momentum, or legacy it creates.\n\n");
    out.push_str("Title rules:\n\n");
    out.push_str("- Prefer noun phrases: `低落的士气`, `土地改革`, `新经济政策复兴`, `军队双轨制`, `官僚主义阴影`.\n");
    out.push_str(
        "- Avoid action titles that sound like focuses, such as `推进工业化` or `重启改革`.\n",
    );
    out.push_str("- Positive ideas can be achievements or mobilised states; negative ideas can be wounds, contradictions, shortages, corruption, fear, fragmentation, or exhaustion.\n\n");
    out.push_str("Description rules:\n\n");
    out.push_str("- Do not promise future policy steps unless the idea is explicitly temporary or transitional.\n");
    out.push_str("- Do not list raw modifiers; translate them into social or political meaning.\n");
    out.push_str("- Mention the affected institution if relevant: army, factories, cadres, peasants, workers, local committees, foreign capital, railways, security organs.\n");
    out.push_str("- If the idea is hidden, describe its structural meaning quietly; do not call it hidden.\n\n");
    out.push_str("Tone modes:\n\n");
    out.push_str("- `structural_crisis`: inherited wound, paralysis, corruption, shortage, fractured authority.\n");
    out.push_str(
        "- `reform_momentum`: cautious recovery, administrative repair, productive mobilisation.\n",
    );
    out.push_str(
        "- `wartime_mobilisation`: society under arms, front-line pressure, military discipline.\n",
    );
    out.push_str(
        "- `ideological_legacy`: doctrine, party line, revolutionary memory, legitimacy claim.\n",
    );
    out.push_str(
        "- `recovery_state`: a damaged country beginning to breathe again, but not yet healed.\n\n",
    );
    out.push_str("Quality checklist:\n\n");
    out.push_str("- The name sounds like a state or condition, not a mission.\n");
    out.push_str("- The description explains why the condition exists.\n");
    out.push_str("- The description implies gameplay effects without spelling out raw numbers.\n");
    out.push_str("- The wording is compact enough for a national-spirit tooltip.\n");
    out.push_str("- The text does not reuse the focus prompt's action-oriented ending.\n");
}

pub(crate) fn render_event_copy_prompt(
    roots: &[PathBuf],
    entries: &[EventCopyEntry],
    options: &FocusCopyPromptOptions,
    language: &str,
    template: &str,
) -> String {
    let with_desc = entries.iter().filter(|entry| entry.desc.is_some()).count();
    let desc_lengths = entries
        .iter()
        .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
        .collect::<Vec<_>>();
    let avg_len = average_usize(&desc_lengths);
    let median_len = median_usize(desc_lengths.clone());
    let title_examples = event_title_examples(entries, options.title_examples);
    let option_examples = event_option_examples(entries, options.title_examples);
    let sample_rows = event_sample_rows(entries, options.sample_keys);
    let mod_stats = event_copy_mod_stats(entries);
    let event_type_counts = event_type_counts(entries);
    let style_profile = event_copy_style_profile(entries);

    let mut out = String::new();
    out.push_str("# HOI4 Chinese Event Copywriting Prompt\n\n");
    out.push_str(
        "This prompt was generated from local event scripts and selected localisation files.\n\n",
    );
    out.push_str("## Style Sample Mods\n\n");
    for root in roots {
        out.push_str(&format!("- `{}`\n", root.display()));
    }
    out.push_str("\n## Learned Statistics\n\n");
    out.push_str(&format!("- Localisation language filter: `{language}`\n"));
    out.push_str(&format!("- Built-in template: `{template}`\n"));
    out.push_str(&format!(
        "- Matched event localisation entries: {}\n",
        entries.len()
    ));
    out.push_str(&format!("- Entries with descriptions: {with_desc}\n"));
    out.push_str(&format!(
        "- Average event description length: {avg_len:.1} characters\n"
    ));
    out.push_str(&format!(
        "- Median event description length: {} characters\n",
        median_len.unwrap_or(0)
    ));
    if !event_type_counts.is_empty() {
        out.push_str("- Event types:");
        for (event_type, count) in event_type_counts {
            out.push_str(&format!(" `{event_type}`={count}"));
        }
        out.push('\n');
    }
    for stat in mod_stats {
        out.push_str(&format!(
            "- `{}`: matched {}, with descriptions {}, average length {:.1}\n",
            stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
        ));
    }
    push_event_copy_style_profile(&mut out, &style_profile);

    if !title_examples.is_empty() {
        out.push_str("\n## Title Examples\n\n");
        out.push_str("Use these as event title-shape references, not text to copy:\n\n");
        for title in title_examples {
            out.push_str(&format!("- `{title}`\n"));
        }
    }
    if !option_examples.is_empty() {
        out.push_str("\n## Option Text Examples\n\n");
        out.push_str("Use these as option-label shape references, not text to copy:\n\n");
        for option in option_examples {
            out.push_str(&format!("- `{option}`\n"));
        }
    }
    if !sample_rows.is_empty() {
        out.push_str("\n## Sample Event Keys\n\n");
        out.push_str(
            "These identify the learned sample shape without copying existing descriptions:\n\n",
        );
        for row in sample_rows {
            let picture = row.picture.unwrap_or_else(|| "<none>".to_string());
            out.push_str(&format!(
                "- `{}` in `{}` [{}] -> `{}` (picture: `{}`, options: {}, desc chars: {})\n",
                row.id, row.file, row.event_type, row.title, picture, row.options, row.desc_len
            ));
        }
    }

    push_event_builtin_templates(&mut out, template);

    if options.style == FocusCopyPromptStyle::Full {
        push_event_copy_style_guide(&mut out);
    }

    out.push_str("\n## Prompt\n\n");
    out.push_str(
r#"```text
你是钢铁雄心4中文事件文案作者。请先按 CLI 内置事件模板组织结构，再参考用户指定 mod 的文案统计、标题形态、选项形态和样例 key 调整语气。
重要：样例只用于风格学习，不得复制原文；用户指定 mod 不是固定世界观，除非输入明确要求引用其设定。
风格画像里的长度、按钮数、场景来源和语气标记是约束信号；优先贴近这些统计，不要逐句模仿样例。

事件文案定位：
- 事件不是国策说明，也不是民族精神状态说明；事件是“某个具体时刻发生了什么，以及这个时刻为什么迫使玩家选择”。
- 描述必须写成一个可发生的场景、通报、会议、报纸、演讲、电报、审讯、街头见闻或外交照会。
- 事件可以短，但不能空；如果是路线关键节点、政变、党代会、统一宣告、外交危机，可以写成长段叙事。
- 选项文字要短，像玩家对此事的态度、命令或历史评价，不要写成机制按钮。

风格要求：
- 标题像新闻标题、会议议题、政治事件、人物行动或危机名称。
- 描述默认采用“具体场景/消息来源 -> 参与者与冲突 -> 政治含义 -> 玩家即将采取的立场”的结构；如果选择了内置模板，以模板 skeleton 为准。
- 可以使用对话、电报、公告、报纸口吻、内部报告、演讲摘录，但要服务于事件冲突。
- 可以按用户指定的风格样本调整叙事密度、讽刺程度、对白长度、制度细节和情绪强度；不要把样本 mod 名称当成必须复制的写法。
- 不要直接列游戏效果，不要说“获得政治点/稳定度”，效果只写进 `效果A/B` 字段或脚本。
- 事件链优先使用结构化字段 `后续事件A/B`、`后续类型A/B`、`后续作用域A/B`、`延迟A/B`、`延迟小时A/B`、`随机延迟天数A/B`、`随机延迟小时A/B`，不要把下一事件藏在描述里。
- 大型事件链必须给关键事件填写 `事件键`，它是稳定编辑锚点，不是 HOI4 event id；后续改标题/描述/选项时保持同一个 `事件键`，CLI 会复用原事件 ID。
- 同一批事件卡里的 `事件键` 必须唯一；不确定就留空，不要复制粘贴同一个键。
- `后续事件A/B` 可以填同批事件标题、同批事件的 `事件键`，或已验证事件 ID；未确定时留给 CLI 报错，不要编造 ID。
- 同一个选项要触发多条后续事件时，用 `后续事件A2/A3`、`后续类型A2/A3`、`后续作用域A2/A3`、`延迟A2/A3`；不要写成新的 `选项A2`。
- 同一个选项要按概率触发后续事件时，用 `随机后续事件A/A2/A3` 和 `随机后续权重A/A2/A3`；CLI 会生成 `random_list`，不要手写 `random_list`。
- 后续事件只在某条件满足时触发，用 `后续条件A/A2` 或 `随机后续条件A/A2`；CLI 会生成 `if = { limit = { ... } ... }`，不要手写 `if/limit`。
- `后续类型A/B` 支持国家事件、新闻事件、州事件/地区事件；州事件默认 `trigger_for = controller`，需要其他对象时填写 `后续触发对象A/B`。
- 需要把后续事件发给其他作用域时填写 `后续作用域A/B`，只能用有原版/本地证据的 `ROOT/FROM/PREV/THIS` 或已索引 TAG；不确定就留空。
- 只想在按钮里展示效果、不想执行时，写 `效果预览A/B`；真正执行的机制仍写 `效果A/B` 或 `隐藏效果A/B`。
- 默认描述 120-260 字；路线关键事件 260-600 字；新闻事件 120-220 字；短通知 60-120 字。

硬性禁止：
- 不准输出“TODO”“描述待补”“占位”“暂无”“这将触发后续事件”等占位文本。
- 不准用现代产品说明、百科总结、作者旁白或代码解释代替事件叙事。
- 不准把选项写成“获得奖励”“增加稳定度”“触发事件”。
- 不准捏造不存在的图片、effect、trigger、tag、事件命名空间；不确定时必须要求 `check-code-symbol` 或索引证据。
- 不准让两个事件使用同一个 `事件键`；这会被 CLI 当作 malformed event cards 拒绝。
- 显式图片必须来自 `check-code-symbol --kind event_picture` 或本地索引；拼写不确定时不要写入。
- 事件 ID 必须使用已确认 namespace；不得和已有事件 ID 冲突。
- 最终生成必须走 `apply-event-cards --game-root ... --mod-path ... --final-check` 或 `validate --strict-code-index`。

输入：
国家/势力：{国家或势力}
时间线背景：{世界线背景}
事件ID或命名空间：{namespace 或 event_id}
事件键：{稳定编辑锚点，例如 rectification_opening；可省略，但事件链关键节点建议填写}
事件类型：{country_event / news_event / state_event}
事件图片：{已索引图片ID，可省略}
触发条件：{自然语言或已验证触发}
事件作用：{这件事在剧情/路线/机制上的作用}
场景来源：{会议 / 电报 / 报纸 / 演讲 / 审讯 / 外交照会 / 街头 / 前线报告}
主要人物/组织：{必须出现的人物或组织}
冲突：{各方争执或危机}
内置模板：{auto / historical_report / political_drama / weird_route / diplomatic_report / revolutionary_scene / news_bulletin / internal_meeting}
希望语气：{稳健历史叙事 / 政治戏剧 / 奇路线正经写法 / 外交报告 / 革命动员 / 新闻简报 / 内部会议}
选项：{玩家可选立场或按钮含义}
效果：{每个选项的机制意图，供 CLI 转代码}

输出为事件卡：
事件：{标题提示}
事件键：{稳定编辑锚点；同一事件改标题时必须保持不变，可选但推荐}
类型：{国家事件/新闻事件/地区事件}
目标：{TAG}
命名空间：{namespace}
图片：{GFX_report_event_xxx 或其他已索引事件图}
触发：{触发条件}
标题：{本地化标题}
描述：{完成态事件描述}
选项A：{短按钮文案}
效果A：{机制意图}
效果预览A：{只显示不执行的效果意图，可选}
后续事件A：{同批事件标题、事件键或已验证 event_id，可选}
后续类型A：{国家事件/新闻事件/州事件，可选；省略时由目标事件类型推断}
后续作用域A：{ROOT/FROM/PREV/THIS/已索引TAG，可选；省略为当前作用域}
后续条件A：{自然语言或已验证 trigger，可选；满足时才触发该后续事件}
后续触发对象A：{州事件可选，默认 controller；可填 owner/controller 等本地证据允许的值}
延迟A：{延迟天数，可选}
延迟小时A：{延迟小时数，可选；和延迟A二选一}
随机延迟天数A：{额外随机天数，可选}
随机延迟小时A：{额外随机小时数，可选}
后续事件A2：{同一选项触发的第二条后续事件，可填标题、事件键或已验证 event_id，可选}
后续类型A2：{国家事件/新闻事件/州事件，可选}
后续作用域A2：{ROOT/FROM/PREV/THIS/已索引TAG，可选}
后续条件A2：{第二条后续事件的触发条件，可选}
延迟A2：{第二条后续事件延迟天数，可选}
随机后续事件A：{同一选项的随机分支目标之一，可填标题、事件键或已验证 event_id，可选}
随机后续权重A：{正整数权重，默认 100}
随机后续条件A：{该随机分支的触发条件，可选}
随机后续事件A2：{同一选项的随机分支目标之二，可填标题、事件键或已验证 event_id，可选}
随机后续权重A2：{正整数权重，默认 100}
随机后续条件A2：{该随机分支的触发条件，可选}
选项B：{短按钮文案，可选}
效果B：{机制意图，可选}
效果预览B：{只显示不执行的效果意图，可选}
后续事件B：{同批事件标题、事件键或已验证 event_id，可选}
后续类型B：{国家事件/新闻事件/州事件，可选；省略时由目标事件类型推断}
后续作用域B：{ROOT/FROM/PREV/THIS/已索引TAG，可选；省略为当前作用域}
后续触发对象B：{州事件可选，默认 controller；可填 owner/controller 等本地证据允许的值}
延迟B：{延迟天数，可选}
延迟小时B：{延迟小时数，可选；和延迟B二选一}
随机延迟天数B：{额外随机天数，可选}
随机延迟小时B：{额外随机小时数，可选}
```
"#,
    );
    out
}

pub(crate) fn render_event_style_profile(
    roots: &[PathBuf],
    entries: &[EventCopyEntry],
    language: &str,
    template: &str,
    format: &str,
) -> String {
    let profile = event_copy_style_profile(entries);
    let event_type_counts = event_type_counts(entries);
    let mod_stats = event_copy_mod_stats(entries);
    if format == "json" {
        return event_style_profile_json(
            roots,
            entries,
            &profile,
            &event_type_counts,
            &mod_stats,
            language,
            template,
        );
    }
    let mut out = String::new();
    out.push_str("# HOI4 Event Style Profile\n\n");
    out.push_str("This is a compact style profile inferred from local event scripts and localisation. It is safe to pass to an AI as style context because it records structure and statistics, not full copied event prose.\n\n");
    out.push_str("- schema: `hoi4skill.event_style_profile.v1`\n");
    out.push_str(&format!("- language: `{language}`\n"));
    out.push_str(&format!("- selected_template: `{template}`\n"));
    out.push_str(&format!("- matched_events: `{}`\n", entries.len()));
    out.push_str("\n## Source Mods\n\n");
    for root in roots {
        out.push_str(&format!("- `{}`\n", root.display()));
    }
    out.push_str("\n## Statistics\n\n");
    out.push_str(&format!(
        "- description_lengths: short_0_120=`{}`, standard_121_260=`{}`, long_261_plus=`{}`\n",
        profile.short_desc, profile.standard_desc, profile.long_desc
    ));
    out.push_str(&format!(
        "- title_shape: matched=`{}`, avg_chars=`{:.1}`\n",
        profile.title_count, profile.avg_title_len
    ));
    out.push_str(&format!(
        "- option_shape: matched=`{}`, avg_chars=`{:.1}`, median_chars=`{}`, avg_options_per_event=`{:.1}`\n",
        profile.option_count,
        profile.avg_option_len,
        profile.median_option_len.unwrap_or(0),
        profile.avg_options_per_event
    ));
    out.push_str(&format!(
        "- voice_markers: quoted_descs=`{}`, paragraph_descs=`{}`, strong_punctuation_descs=`{}`\n",
        profile.quoted_desc, profile.paragraph_desc, profile.strong_punctuation_desc
    ));
    if !event_type_counts.is_empty() {
        out.push_str("- event_types:");
        for (event_type, count) in &event_type_counts {
            out.push_str(&format!(" `{event_type}`={count}"));
        }
        out.push('\n');
    }
    if !profile.scene_cues.is_empty() {
        out.push_str("- scene_cues:");
        for (cue, count) in &profile.scene_cues {
            out.push_str(&format!(" `{cue}`={count}"));
        }
        out.push('\n');
    }
    if !mod_stats.is_empty() {
        out.push_str("\n## Per-Mod Coverage\n\n");
        for stat in &mod_stats {
            out.push_str(&format!(
                "- `{}`: matched `{}`, descriptions `{}`, avg_desc_chars `{:.1}`\n",
                stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
            ));
        }
    }
    out.push_str("\n## Built-In Template Contract\n\n");
    for spec in event_copy_template_specs() {
        if spec.id == template || template == "auto" {
            out.push_str(&format!(
                "- `{}`: use when {}; skeleton: {}.\n",
                spec.id, spec.use_when, spec.structure
            ));
        }
    }
    out.push_str("\n## AI Rules\n\n");
    for rule in event_style_profile_rules() {
        out.push_str(&format!("- {rule}\n"));
    }
    out.push_str("\n## Recommended Commands\n\n");
    out.push_str("- `hoi4skill event-copy-prompt <style-mod> --template <template> --output event_prompt.md`\n");
    out.push_str("- `hoi4skill apply-event-cards --input events.txt --mod-root <mod> --tag <TAG> --prefix <prefix> --game-root <HOI4 root> --final-check`\n");
    out.push_str("- `hoi4skill validate <mod> --game-root <HOI4 root> --strict-code-index`\n");
    out
}

pub(crate) fn event_style_profile_json(
    roots: &[PathBuf],
    entries: &[EventCopyEntry],
    profile: &EventCopyStyleProfile,
    event_type_counts: &BTreeMap<String, usize>,
    mod_stats: &[FocusCopyModStat],
    language: &str,
    template: &str,
) -> String {
    let roots_json = roots
        .iter()
        .map(|root| root.display().to_string())
        .collect::<Vec<_>>();
    let event_types = event_type_counts
        .iter()
        .map(|(event_type, count)| {
            format!(
                "{{\"event_type\": {}, \"count\": {}}}",
                json_str(event_type),
                count
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let scene_cues = profile
        .scene_cues
        .iter()
        .map(|(cue, count)| format!("{{\"cue\": {}, \"count\": {}}}", json_str(cue), count))
        .collect::<Vec<_>>()
        .join(", ");
    let mod_stats_json = mod_stats
        .iter()
        .map(|stat| {
            format!(
                "{{\"mod_name\": {}, \"matched\": {}, \"with_desc\": {}, \"avg_desc_len\": {:.1}}}",
                json_str(&stat.mod_name),
                stat.matched,
                stat.with_desc,
                stat.avg_desc_len
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let templates = event_copy_template_specs()
        .iter()
        .filter(|spec| template == "auto" || spec.id == template)
        .map(|spec| {
            format!(
                "{{\"id\": {}, \"use_when\": {}, \"structure\": {}}}",
                json_str(spec.id),
                json_str(spec.use_when),
                json_str(spec.structure)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"schema\": \"hoi4skill.event_style_profile.v1\",\n  \"language\": {},\n  \"selected_template\": {},\n  \"source_mods\": {},\n  \"matched_events\": {},\n  \"description_lengths\": {{\"short_0_120\": {}, \"standard_121_260\": {}, \"long_261_plus\": {}}},\n  \"title_shape\": {{\"matched\": {}, \"avg_chars\": {:.1}}},\n  \"option_shape\": {{\"matched\": {}, \"avg_chars\": {:.1}, \"median_chars\": {}, \"avg_options_per_event\": {:.1}, \"median_options_per_event\": {}}},\n  \"voice_markers\": {{\"quoted_descs\": {}, \"paragraph_descs\": {}, \"strong_punctuation_descs\": {}}},\n  \"event_types\": [{}],\n  \"scene_cues\": [{}],\n  \"mod_stats\": [{}],\n  \"built_in_templates\": [{}],\n  \"rules\": {},\n  \"anti_copy_rule\": {}\n}}\n",
        json_str(language),
        json_str(template),
        json_array(&roots_json),
        entries.len(),
        profile.short_desc,
        profile.standard_desc,
        profile.long_desc,
        profile.title_count,
        profile.avg_title_len,
        profile.option_count,
        profile.avg_option_len,
        profile.median_option_len.unwrap_or(0),
        profile.avg_options_per_event,
        profile.median_options_per_event.unwrap_or(0),
        profile.quoted_desc,
        profile.paragraph_desc,
        profile.strong_punctuation_desc,
        event_types,
        scene_cues,
        mod_stats_json,
        templates,
        json_array(&event_style_profile_rules()),
        json_str("This profile is for structural style transfer only; do not copy localised source prose, titles, option text, or worldbuilding unless the user explicitly asks for that content.")
    )
}

pub(crate) fn event_style_profile_rules() -> Vec<String> {
    vec![
        "Use built-in templates as the event skeleton; use the profile only to tune density, cadence, scene source, and option length.".to_string(),
        "Do not copy event descriptions, titles, option text, named worldbuilding, or unique jokes from the sampled mod.".to_string(),
        "If the user's requested style conflicts with strict validation, validation wins.".to_string(),
        "Generated event cards must still pass apply-event-cards --final-check or validate --strict-code-index.".to_string(),
    ]
}

pub(crate) fn render_work_package_style_context(
    roots: &[PathBuf],
    focus_entries: &[FocusCopyEntry],
    idea_entries: &[IdeaCopyEntry],
    event_entries: &[EventCopyEntry],
    language: &str,
    template: &str,
) -> String {
    let focus_desc_lengths = focus_entries
        .iter()
        .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
        .collect::<Vec<_>>();
    let idea_desc_lengths = idea_entries
        .iter()
        .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
        .collect::<Vec<_>>();
    let event_profile = event_copy_style_profile(event_entries);
    let event_type_counts = event_type_counts(event_entries);
    let focus_titles = focus_title_examples(focus_entries, 12);
    let idea_titles = idea_title_examples(idea_entries, 12);
    let mut out = String::new();
    out.push_str("# HOI4 Work Package Style Context\n\n");
    out.push_str("- schema: `hoi4skill.work_package_style_context.v1`\n");
    out.push_str(&format!("- language: `{language}`\n"));
    out.push_str(&format!("- selected_event_template: `{template}`\n"));
    out.push_str("- rule: use this as structural style context only; do not copy source prose, titles, option text, named jokes, or worldbuilding.\n");
    out.push_str("- rule: user intent, game code index, placeholder resolution, text alignment, and final validation override style.\n");
    out.push_str("- rule: AI fills intent/layout/card inputs; Rust `hoi4skill` writers emit final Clausewitz and localisation.\n");

    out.push_str("\n## Source Mods\n\n");
    for root in roots {
        out.push_str(&format!("- `{}`\n", root.display()));
    }

    out.push_str("\n## National Focus Style Profile\n\n");
    out.push_str(&format!("- matched_focuses: `{}`\n", focus_entries.len()));
    out.push_str(&format!(
        "- descriptions: matched=`{}`, avg_chars=`{:.1}`, median_chars=`{}`\n",
        focus_desc_lengths.len(),
        average_usize(&focus_desc_lengths),
        median_usize(focus_desc_lengths.clone()).unwrap_or(0)
    ));
    if !focus_titles.is_empty() {
        out.push_str("- title_shapes:");
        for title in focus_titles {
            out.push_str(&format!(" `{title}`"));
        }
        out.push('\n');
    }
    for stat in focus_copy_mod_stats(focus_entries) {
        out.push_str(&format!(
            "- `{}`: matched `{}`, descriptions `{}`, avg_desc_chars `{:.1}`\n",
            stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
        ));
    }

    out.push_str("\n## National Spirit Style Profile\n\n");
    out.push_str(&format!("- matched_ideas: `{}`\n", idea_entries.len()));
    out.push_str(&format!(
        "- descriptions: matched=`{}`, avg_chars=`{:.1}`, median_chars=`{}`\n",
        idea_desc_lengths.len(),
        average_usize(&idea_desc_lengths),
        median_usize(idea_desc_lengths.clone()).unwrap_or(0)
    ));
    let category_counts = idea_category_counts(idea_entries);
    if !category_counts.is_empty() {
        out.push_str("- categories:");
        for (category, count) in category_counts {
            out.push_str(&format!(" `{category}`={count}"));
        }
        out.push('\n');
    }
    if !idea_titles.is_empty() {
        out.push_str("- title_shapes:");
        for title in idea_titles {
            out.push_str(&format!(" `{title}`"));
        }
        out.push('\n');
    }
    for stat in idea_copy_mod_stats(idea_entries) {
        out.push_str(&format!(
            "- `{}`: matched `{}`, descriptions `{}`, avg_desc_chars `{:.1}`\n",
            stat.mod_name, stat.matched, stat.with_desc, stat.avg_desc_len
        ));
    }

    out.push_str("\n## Event Style Profile\n\n");
    out.push_str(&format!("- matched_events: `{}`\n", event_entries.len()));
    out.push_str(&format!(
        "- description_lengths: short_0_120=`{}`, standard_121_260=`{}`, long_261_plus=`{}`\n",
        event_profile.short_desc, event_profile.standard_desc, event_profile.long_desc
    ));
    out.push_str(&format!(
        "- title_shape: matched=`{}`, avg_chars=`{:.1}`\n",
        event_profile.title_count, event_profile.avg_title_len
    ));
    out.push_str(&format!(
        "- option_shape: matched=`{}`, avg_chars=`{:.1}`, median_chars=`{}`, avg_options_per_event=`{:.1}`\n",
        event_profile.option_count,
        event_profile.avg_option_len,
        event_profile.median_option_len.unwrap_or(0),
        event_profile.avg_options_per_event
    ));
    out.push_str(&format!(
        "- voice_markers: quoted_descs=`{}`, paragraph_descs=`{}`, strong_punctuation_descs=`{}`\n",
        event_profile.quoted_desc,
        event_profile.paragraph_desc,
        event_profile.strong_punctuation_desc
    ));
    if !event_type_counts.is_empty() {
        out.push_str("- event_types:");
        for (event_type, count) in event_type_counts {
            out.push_str(&format!(" `{event_type}`={count}"));
        }
        out.push('\n');
    }
    if !event_profile.scene_cues.is_empty() {
        out.push_str("- scene_cues:");
        for (cue, count) in &event_profile.scene_cues {
            out.push_str(&format!(" `{cue}`={count}"));
        }
        out.push('\n');
    }

    out.push_str("\n## Built-In Event Template Contract\n\n");
    for spec in event_copy_template_specs() {
        if spec.id == template || template == "auto" {
            out.push_str(&format!(
                "- `{}`: use when {}; skeleton: {}.\n",
                spec.id, spec.use_when, spec.structure
            ));
        }
    }

    out.push_str("\n## AI Authoring Rules\n\n");
    out.push_str("- Fill `focus_layout.txt`, `event_cards.txt`, `feature_cards.txt`, `intent.txt`, and `localisation_placeholders.txt` using this profile for cadence and density only.\n");
    out.push_str("- Do not introduce raw Clausewitz syntax that is absent from the local code index; use natural-language intent and let CLI writers compile it.\n");
    out.push_str("- Keep national spirits as state/condition text; keep dynamic modifiers in the dynamic modifier protocol, not as ideas.\n");
    out.push_str("- Preserve all HOI4 localisation tokens, colour markers, icons, scripted localisation, and user-visible requested text.\n");
    out.push_str("- If an icon, tag, cosmetic tag, leader token, event picture, effect, trigger, or modifier cannot be resolved, ask the user instead of inventing it.\n");
    out
}

pub(crate) fn push_event_copy_style_guide(out: &mut String) {
    out.push_str("\n## Learned Event Style Guide\n\n");
    out.push_str("Write events as moments, not as generic summaries.\n\n");
    out.push_str("Core structure:\n\n");
    out.push_str("1. Anchor the event in a concrete source: meeting room, telegram, newspaper, speech, street, front line, court, party office, embassy, prison, or radio broadcast.\n");
    out.push_str("2. Name the actors and their immediate conflict.\n");
    out.push_str("3. Explain the political stakes in one or two sentences.\n");
    out.push_str("4. End near a decision, order, revelation, rupture, or public reaction.\n\n");
    out.push_str("Title rules:\n\n");
    out.push_str("- Prefer event nouns: `来自巴黎的电报`, `反革命政变`, `第一届全国人民代表大会`, `宋庆龄抵达上海`.\n");
    out.push_str("- News events can use declarative headlines; country events can use scene or document titles.\n");
    out.push_str("- Avoid vague titles such as `重大事件`, `新的决定`, `政治变化`.\n\n");
    out.push_str("Description modes:\n\n");
    out.push_str("- `historical_report`: sober alternate-history exposition with institutional and diplomatic context.\n");
    out.push_str("- `political_drama`: stronger ideological conflict, speeches, factions, betrayals, street action, and named personalities.\n");
    out.push_str("- `weird_route`: absurd or uncanny premise written as if official actors take it seriously.\n");
    out.push_str("- `diplomatic_report`: telegrams, embassy language, communiques, and negotiated ambiguity.\n");
    out.push_str("- `revolutionary_scene`: crowds, committees, strikes, militias, speeches, banners, arrests, and mobilisation.\n\n");
    out.push_str("Option-label rules:\n\n");
    out.push_str(
        "- Keep option labels short and expressive; one line, usually 4-18 Chinese characters.\n",
    );
    out.push_str("- Express stance, command, irony, or historical judgement.\n");
    out.push_str("- Do not put raw effects in option labels.\n\n");
    out.push_str("Quality checklist:\n\n");
    out.push_str("- The first sentence answers where the information comes from.\n");
    out.push_str("- The middle names people, institutions, factions, or social groups.\n");
    out.push_str("- The ending creates pressure for the option.\n");
    out.push_str("- The description does not mention raw game effects.\n");
    out.push_str(
        "- The event picture is indexed or deliberately omitted for semantic selection.\n",
    );
    out.push_str("- The event can be converted by `apply-event-cards --final-check` without unresolved AI mapping comments.\n");
}

#[derive(Default)]
pub(crate) struct FocusCopyModStat {
    pub(crate) mod_name: String,
    pub(crate) matched: usize,
    pub(crate) with_desc: usize,
    pub(crate) avg_desc_len: f64,
}

pub(crate) fn focus_copy_mod_stats(entries: &[FocusCopyEntry]) -> Vec<FocusCopyModStat> {
    let mut grouped: BTreeMap<String, Vec<&FocusCopyEntry>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.mod_name.clone())
            .or_default()
            .push(entry);
    }
    grouped
        .into_iter()
        .map(|(mod_name, group)| {
            let lengths = group
                .iter()
                .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
                .collect::<Vec<_>>();
            FocusCopyModStat {
                mod_name,
                matched: group.len(),
                with_desc: lengths.len(),
                avg_desc_len: average_usize(&lengths),
            }
        })
        .collect()
}

pub(crate) fn idea_copy_mod_stats(entries: &[IdeaCopyEntry]) -> Vec<FocusCopyModStat> {
    let mut grouped: BTreeMap<String, Vec<&IdeaCopyEntry>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.mod_name.clone())
            .or_default()
            .push(entry);
    }
    grouped
        .into_iter()
        .map(|(mod_name, group)| {
            let lengths = group
                .iter()
                .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
                .collect::<Vec<_>>();
            FocusCopyModStat {
                mod_name,
                matched: group.len(),
                with_desc: lengths.len(),
                avg_desc_len: average_usize(&lengths),
            }
        })
        .collect()
}

pub(crate) fn event_copy_mod_stats(entries: &[EventCopyEntry]) -> Vec<FocusCopyModStat> {
    let mut grouped: BTreeMap<String, Vec<&EventCopyEntry>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.mod_name.clone())
            .or_default()
            .push(entry);
    }
    grouped
        .into_iter()
        .map(|(mod_name, group)| {
            let lengths = group
                .iter()
                .filter_map(|entry| entry.desc.as_ref().map(|desc| desc.chars().count()))
                .collect::<Vec<_>>();
            FocusCopyModStat {
                mod_name,
                matched: group.len(),
                with_desc: lengths.len(),
                avg_desc_len: average_usize(&lengths),
            }
        })
        .collect()
}

pub(crate) fn idea_category_counts(entries: &[IdeaCopyEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry.category.clone()).or_default() += 1;
    }
    counts
}

pub(crate) fn event_type_counts(entries: &[EventCopyEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry.event_type.clone()).or_default() += 1;
    }
    counts
}

#[derive(Default)]
pub(crate) struct EventCopyStyleProfile {
    pub(crate) total: usize,
    pub(crate) title_count: usize,
    pub(crate) avg_title_len: f64,
    pub(crate) option_count: usize,
    pub(crate) avg_option_len: f64,
    pub(crate) median_option_len: Option<usize>,
    pub(crate) avg_options_per_event: f64,
    pub(crate) median_options_per_event: Option<usize>,
    pub(crate) short_desc: usize,
    pub(crate) standard_desc: usize,
    pub(crate) long_desc: usize,
    pub(crate) quoted_desc: usize,
    pub(crate) paragraph_desc: usize,
    pub(crate) strong_punctuation_desc: usize,
    pub(crate) scene_cues: BTreeMap<&'static str, usize>,
}

pub(crate) fn event_copy_style_profile(entries: &[EventCopyEntry]) -> EventCopyStyleProfile {
    let title_lengths = entries
        .iter()
        .filter_map(|entry| entry.title.as_ref().map(|title| title.chars().count()))
        .collect::<Vec<_>>();
    let option_lengths = entries
        .iter()
        .flat_map(|entry| {
            entry
                .option_names
                .iter()
                .map(|option| option.chars().count())
        })
        .collect::<Vec<_>>();
    let options_per_event = entries
        .iter()
        .map(|entry| entry.option_names.len())
        .collect::<Vec<_>>();
    let mut profile = EventCopyStyleProfile {
        total: entries.len(),
        title_count: title_lengths.len(),
        avg_title_len: average_usize(&title_lengths),
        option_count: option_lengths.len(),
        avg_option_len: average_usize(&option_lengths),
        median_option_len: median_usize(option_lengths.clone()),
        avg_options_per_event: average_usize(&options_per_event),
        median_options_per_event: median_usize(options_per_event),
        ..Default::default()
    };
    for entry in entries {
        let Some(desc) = entry.desc.as_ref() else {
            continue;
        };
        match desc.chars().count() {
            0..=120 => profile.short_desc += 1,
            121..=260 => profile.standard_desc += 1,
            _ => profile.long_desc += 1,
        }
        if contains_event_quote_marker(desc) {
            profile.quoted_desc += 1;
        }
        if desc.contains('\n') {
            profile.paragraph_desc += 1;
        }
        if desc.contains('！') || desc.contains('!') || desc.contains('？') || desc.contains('?')
        {
            profile.strong_punctuation_desc += 1;
        }
        let scene_text = format!("{} {}", entry.title.as_deref().unwrap_or_default(), desc);
        for cue in event_scene_cues(&scene_text) {
            *profile.scene_cues.entry(cue).or_default() += 1;
        }
    }
    profile
}

pub(crate) fn push_event_copy_style_profile(out: &mut String, profile: &EventCopyStyleProfile) {
    if profile.total == 0 {
        return;
    }
    out.push_str("\n## Learned Event Style Profile\n\n");
    out.push_str("Machine-readable constraints inferred from the selected mod samples. Use them to tune density and cadence; do not copy sample wording.\n\n");
    out.push_str("```text\n");
    out.push_str(&format!(
        "desc_length_profile: short_0_120={} standard_121_260={} long_261_plus={}\n",
        profile.short_desc, profile.standard_desc, profile.long_desc
    ));
    out.push_str(&format!(
        "title_shape: matched={} avg_chars={:.1}\n",
        profile.title_count, profile.avg_title_len
    ));
    out.push_str(&format!(
        "option_shape: matched={} avg_chars={:.1} median_chars={} avg_options_per_event={:.1} median_options_per_event={}\n",
        profile.option_count,
        profile.avg_option_len,
        profile.median_option_len.unwrap_or(0),
        profile.avg_options_per_event,
        profile.median_options_per_event.unwrap_or(0)
    ));
    out.push_str(&format!(
        "voice_markers: quoted_descs={} paragraph_descs={} strong_punctuation_descs={}\n",
        profile.quoted_desc, profile.paragraph_desc, profile.strong_punctuation_desc
    ));
    if !profile.scene_cues.is_empty() {
        out.push_str("scene_cues:");
        for (cue, count) in &profile.scene_cues {
            out.push_str(&format!(" `{cue}`={count}"));
        }
        out.push('\n');
    }
    out.push_str("```\n");
}

pub(crate) fn contains_event_quote_marker(text: &str) -> bool {
    text.contains('"')
        || text.contains('“')
        || text.contains('”')
        || text.contains('「')
        || text.contains('」')
}

pub(crate) fn event_scene_cues(text: &str) -> Vec<&'static str> {
    const CUES: &[(&str, &[&str])] = &[
        (
            "meeting",
            &["会议", "委员会", "代表", "会场", "主席", "内阁", "议会"],
        ),
        (
            "report",
            &["报告", "通报", "文件", "档案", "命令", "办公室"],
        ),
        ("telegram", &["电报", "照会", "公报", "通讯", "广播"]),
        ("newspaper", &["报纸", "新闻", "头条", "记者"]),
        ("street", &["街头", "广场", "群众", "人群", "罢工", "游行"]),
        ("speech", &["演讲", "讲话", "宣告", "口号", "广播声"]),
        (
            "diplomacy",
            &["大使", "使馆", "外交", "条约", "边境", "巴黎"],
        ),
        ("front", &["前线", "军队", "士兵", "战场"]),
    ];
    CUES.iter()
        .filter_map(|(cue, needles)| {
            needles
                .iter()
                .any(|needle| text.contains(needle))
                .then_some(*cue)
        })
        .collect()
}

pub(crate) fn focus_title_examples(entries: &[FocusCopyEntry], limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut titles = Vec::new();
    for entry in entries {
        let Some(title) = entry.title.as_ref() else {
            continue;
        };
        let len = title.chars().count();
        if !(2..=12).contains(&len) {
            continue;
        }
        if seen.insert(title.clone()) {
            titles.push(title.clone());
        }
        if titles.len() >= limit {
            break;
        }
    }
    titles
}

pub(crate) fn idea_title_examples(entries: &[IdeaCopyEntry], limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut titles = Vec::new();
    for entry in entries {
        let Some(title) = entry.title.as_ref() else {
            continue;
        };
        let len = title.chars().count();
        if !(2..=16).contains(&len) {
            continue;
        }
        if seen.insert(title.clone()) {
            titles.push(title.clone());
        }
        if titles.len() >= limit {
            break;
        }
    }
    titles
}

pub(crate) fn event_title_examples(entries: &[EventCopyEntry], limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut titles = Vec::new();
    for entry in entries {
        let Some(title) = entry.title.as_ref() else {
            continue;
        };
        let len = title.chars().count();
        if !(2..=24).contains(&len) {
            continue;
        }
        if seen.insert(title.clone()) {
            titles.push(title.clone());
        }
        if titles.len() >= limit {
            break;
        }
    }
    titles
}

pub(crate) fn event_option_examples(entries: &[EventCopyEntry], limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut options = Vec::new();
    for option in entries.iter().flat_map(|entry| entry.option_names.iter()) {
        let len = option.chars().count();
        if !(2..=24).contains(&len) {
            continue;
        }
        if seen.insert(option.clone()) {
            options.push(option.clone());
        }
        if options.len() >= limit {
            break;
        }
    }
    options
}

pub(crate) struct FocusCopySampleRow {
    pub(crate) id: String,
    pub(crate) file: String,
    pub(crate) title: String,
    pub(crate) desc_len: usize,
}

pub(crate) struct IdeaCopySampleRow {
    pub(crate) id: String,
    pub(crate) file: String,
    pub(crate) category: String,
    pub(crate) picture: Option<String>,
    pub(crate) title: String,
    pub(crate) desc_len: usize,
}

pub(crate) struct EventCopySampleRow {
    pub(crate) id: String,
    pub(crate) file: String,
    pub(crate) event_type: String,
    pub(crate) picture: Option<String>,
    pub(crate) title: String,
    pub(crate) options: usize,
    pub(crate) desc_len: usize,
}

pub(crate) fn focus_sample_rows(
    entries: &[FocusCopyEntry],
    limit: usize,
) -> Vec<FocusCopySampleRow> {
    entries
        .iter()
        .filter_map(|entry| {
            let title = entry.title.as_ref()?;
            let desc = entry.desc.as_ref()?;
            Some(FocusCopySampleRow {
                id: entry.id.clone(),
                file: entry.file.clone(),
                title: title.clone(),
                desc_len: desc.chars().count(),
            })
        })
        .take(limit)
        .collect()
}

pub(crate) fn idea_sample_rows(entries: &[IdeaCopyEntry], limit: usize) -> Vec<IdeaCopySampleRow> {
    entries
        .iter()
        .filter_map(|entry| {
            let title = entry.title.as_ref()?;
            let desc = entry.desc.as_ref()?;
            Some(IdeaCopySampleRow {
                id: entry.id.clone(),
                file: entry.file.clone(),
                category: entry.category.clone(),
                picture: entry.picture.clone(),
                title: title.clone(),
                desc_len: desc.chars().count(),
            })
        })
        .take(limit)
        .collect()
}

pub(crate) fn event_sample_rows(
    entries: &[EventCopyEntry],
    limit: usize,
) -> Vec<EventCopySampleRow> {
    entries
        .iter()
        .filter_map(|entry| {
            let title = entry.title.as_ref()?;
            let desc = entry.desc.as_ref()?;
            Some(EventCopySampleRow {
                id: entry.id.clone(),
                file: entry.file.clone(),
                event_type: entry.event_type.clone(),
                picture: entry.picture.clone(),
                title: title.clone(),
                options: entry.option_names.len(),
                desc_len: desc.chars().count(),
            })
        })
        .take(limit)
        .collect()
}

pub(crate) fn average_usize(values: &[usize]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<usize>() as f64 / values.len() as f64
    }
}

pub(crate) fn median_usize(mut values: Vec<usize>) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

pub(crate) fn cmd_parse_focus_copy_cards(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = require_value(&map, "input")?;
    let text = read_utf8_lossy(&normalize_path(&input)?)?;
    let cards = parse_focus_copy_cards(&text);
    let markdown = render_focus_copy_card_prompts(&cards);
    write_or_print(&markdown, value(&map, "output"))
}

#[derive(Clone)]
pub(crate) struct FocusCopyCard {
    pub(crate) title_hint: String,
    pub(crate) focus_id: String,
    pub(crate) country: String,
    pub(crate) timeline: String,
    pub(crate) route: String,
    pub(crate) purpose: String,
    pub(crate) conflict: String,
    pub(crate) tone: String,
    pub(crate) keywords: String,
    pub(crate) length: String,
}

pub(crate) fn parse_focus_copy_cards(text: &str) -> Vec<FocusCopyCard> {
    parse_cards(text, &["国策", "标题", "focus", "Focus"])
        .into_iter()
        .enumerate()
        .map(|(idx, card)| focus_copy_card_from_card(idx, card))
        .collect()
}

pub(crate) fn focus_copy_card_from_card(idx: usize, card: Card) -> FocusCopyCard {
    let fallback_id = format!("TAG_focus_copy_{}", idx + 1);
    FocusCopyCard {
        title_hint: card.title.clone(),
        focus_id: first_field(&card.fields, &["国策ID", "ID", "id", "focus_id"])
            .unwrap_or(fallback_id),
        country: first_field(
            &card.fields,
            &["国家", "势力", "国家/势力", "country", "tag"],
        )
        .unwrap_or_else(|| "待填写".to_string()),
        timeline: first_field(&card.fields, &["时间线背景", "背景", "世界线", "timeline"])
            .unwrap_or_else(|| "待填写".to_string()),
        route: first_field(&card.fields, &["所属路线", "路线", "分支", "route"])
            .unwrap_or_else(|| "待填写".to_string()),
        purpose: first_field(
            &card.fields,
            &["国策作用", "作用", "剧情作用", "机制作用", "purpose"],
        )
        .or_else(|| first_field(&card.fields, &["描述", "desc"]))
        .unwrap_or_else(|| "待填写".to_string()),
        conflict: first_field(&card.fields, &["前置矛盾", "矛盾", "前情", "conflict"])
            .unwrap_or_else(|| "待填写".to_string()),
        tone: first_field(&card.fields, &["希望语气", "语气", "tone"])
            .unwrap_or_else(|| infer_focus_copy_tone(&card)),
        keywords: first_field(&card.fields, &["关键词", "key words", "keywords"])
            .unwrap_or_else(|| "待填写".to_string()),
        length: first_field(&card.fields, &["长度", "length"]).unwrap_or_else(|| "中".to_string()),
    }
}

pub(crate) fn first_field(fields: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| fields.get(*key))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(crate) fn infer_focus_copy_tone(card: &Card) -> String {
    let text = format!(
        "{} {}",
        card.title,
        card.fields.values().cloned().collect::<Vec<_>>().join(" ")
    );
    if contains_any(&text, &["战争", "起义", "解放", "动员", "前线", "人民军队"]) {
        "revolutionary_mobilisation".to_string()
    } else if contains_any(&text, &["路线", "主义", "党代会", "理论", "批判", "民主"])
    {
        "ideological_debate".to_string()
    } else if contains_any(
        &text,
        &["奇怪", "荒诞", "实验", "超现实", "动物", "佛", "外星"],
    ) {
        "strange_route".to_string()
    } else {
        "historical_policy".to_string()
    }
}

pub(crate) fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

pub(crate) fn render_focus_copy_card_prompts(cards: &[FocusCopyCard]) -> String {
    let mut out = String::new();
    out.push_str("# Focus Copywriting Batch\n\n");
    out.push_str(
        "Use each block as a prompt for Chinese HOI4 focus skeleton, title, and description writing.\n\n",
    );
    for (idx, card) in cards.iter().enumerate() {
        out.push_str(&format!("## {}. {}\n\n", idx + 1, card.title_hint));
        out.push_str("```text\n");
        out.push_str("你是钢铁雄心4中文国策文案作者。请按我的本地 mod 文案风格，为下面的国策写完整国策骨架、中文标题与描述。\n\n");
        out.push_str("风格要求：\n");
        out.push_str("- 写成 HOI4 架空历史国策文案，不要写成现代说明书。\n");
        out.push_str("- 标题短促有力，像政策名、政治口号、路线名、运动名或人物路线标签。\n");
        out.push_str("- 描述采用“历史矛盾/现实困境 -> 阶级或制度解释 -> 政策必要性 -> 行动或历史方向”的结构。\n");
        out.push_str("- 必须以本国、本路线或本利益集团的内部第一视角写，不准写成第三方观察者、百科条目或历史学者旁白。\n");
        out.push_str("- 不要直接列游戏效果，不要说“该国策将给予...”，除非用户明确要求机制说明。\n");
        out.push_str("- 不要抄已有文案，保持同类节奏与词汇质感即可。\n\n");
        out.push_str("交付硬规则：不准以“先做可校验 demo”“保守脚本骨架”“之后补回文案/路线叙事”为由跳过国策文案工作流；可编译骨架不是完成品，必须先抽取路线叙事，再输出完成态标题、描述、本地化和脚本。\n\n");
        out.push_str("本地化硬规则：不准生成 `<prefix>_mod_name`、`chinaprc_1979_mod_name` 或任何 `*_mod_name`；mod 名称只写在 `descriptor.mod` 和外层 `.mod` 文件。\n\n");
        out.push_str("机制路由硬规则：即时奖励写国策 `completion_reward`；长期修正中的固定修正生成/引用民族精神，用 `add_ideas` 添加，临时状态再用 `remove_ideas` 移除；变量驱动动态修正必须引用已验证的 `common/dynamic_modifiers` ID 和 `common/scripted_effects` 快捷效果，按 `custom_effect_tooltip = <动态修正>_tt`、`set_temp_variable = { temp_<动态修正> = <数值> }`、`change_<动态修正><序号> = yes` 组装，不准凭空编 `change_*` 或变量名；不要把 `modifier = { ... }` 直接写进国策效果。\n\n");
        out.push_str("国策骨架硬规则：如果要输出国策代码，必须展开成完整 `focus = { ... }` 模板，不准写短国策或半截片段，方便玩家手动调整条件。\n");
        out.push_str(
            "国家筛选块硬规则：必须使用固定结构 `country = { factor = 0 modifier = { add = 10 tag = <TAG> } }`；不要把 `add` 改成 100 或其他数值。\n\n",
        );
        out.push_str("x/y 排布硬规则：如果用户没有给国策树草图，一律套五段模板：y=0 一个开篇国策 x=0；y=1 两到四个展开国策，x 间隔 2；y=2 一个阶段成果 x=0；y=3 两到四个展开国策，x 间隔 2；y=4 一个收尾成果 x=0。不要随机散点。\n\n");
        out.push_str("icon 硬规则：`icon =` 必须填写从目标 MOD、依赖 MOD 或游戏 `interface/goals*.gfx` 读取到的真实国策图标 sprite（如 `GFX_goal...` 或 `GFX_focus...`），并按国策含义选择；如果没有图标索引，只能用 `GFX_goal_unknown` 并说明需要先索引，禁止按标题编造 sprite 名。\n\n");
        out.push_str("互斥字段硬规则：只能写 `mutually_exclusive = { focus = <id> }`。禁止 `mutual_exclusion`、`mutual_exclusive`、`mutually_exclusion` 等近似拼写。\n\n");
        out.push_str("字段拼写硬规则：所有国策字段必须使用 HOI4 精确字段名，不准复数化、缩写、翻译或凭印象拼写。重点检查 `prerequisite`、`relative_position_id`、`completion_reward`、`ai_will_do`、`cancel_if_invalid`、`continue_if_invalid`、`available_if_capitulated`。\n\n");
        out.push_str("事件字段同样必须精确：命名空间只能用顶层 `add_namespace`，事件结构使用 `is_triggered_only`、`fire_only_once`、`mean_time_to_happen`、`immediate`、`option`，禁止近似拼写。\n\n");
        out.push_str("输入：\n");
        out.push_str(&format!("国家/势力：{}\n", card.country));
        out.push_str(&format!("时间线背景：{}\n", card.timeline));
        out.push_str(&format!("所属路线：{}\n", card.route));
        out.push_str(&format!("国策ID：{}\n", card.focus_id));
        out.push_str(&format!("标题提示：{}\n", card.title_hint));
        out.push_str(&format!("国策作用：{}\n", card.purpose));
        out.push_str(&format!("前置矛盾：{}\n", card.conflict));
        out.push_str(&format!("希望语气：{}\n", card.tone));
        out.push_str(&format!("关键词：{}\n", card.keywords));
        out.push_str(&format!("长度：{}\n\n", card.length));
        out.push_str("输出：\n");
        out.push_str("1. 完整国策骨架，必须按模板展开，不准输出短国策：\n");
        out.push_str("```text\n");
        out.push_str("focus = {\n");
        out.push_str("id = \n");
        out.push_str(
            "icon = <verified focus icon sprite from interface/goals*.gfx, or GFX_goal_unknown>\n",
        );
        out.push_str("x = <use default template or user sketch>\n");
        out.push_str("y = <use default template or user sketch>\n");
        out.push_str("prerequisite = {focus = }\n");
        out.push_str("relative_position_id =  #基于某个国策位置的相对位置\n");
        out.push_str("cost = <required 1..10; only exceed 10 when the user explicitly asked for a longer focus>\n");
        out.push_str("\t\tai_will_do = {\n");
        out.push_str("\t\t\tfactor = 100\n");
        out.push_str("\t\t}\n\n");
        out.push_str("\t\tavailable = {\n\n\t\t}\n\n");
        out.push_str("\t\tbypass = {\n\t\t}\n\n");
        out.push_str("\t\tcancel_if_invalid = yes\n");
        out.push_str("\t\tcontinue_if_invalid = no\n");
        out.push_str("\t\tavailable_if_capitulated = no\n\n");
        out.push_str("\t\tcompletion_reward = {\n\n\t\t}\n");
        out.push_str("}\n");
        out.push_str("```\n");
        out.push_str("2. 标题\n");
        out.push_str("3. 描述\n");
        out.push_str("4. 本地化：\n");
        out.push_str(&format!("   {}:0 \"标题\"\n", card.focus_id));
        out.push_str(&format!("   {}_desc:0 \"描述\"\n", card.focus_id));
        out.push_str("```\n\n");
    }
    out
}
