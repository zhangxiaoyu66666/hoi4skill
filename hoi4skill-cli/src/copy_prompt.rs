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
    out.push_str("## Source Mods\n\n");
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

pub(crate) fn idea_category_counts(entries: &[IdeaCopyEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(entry.category.clone()).or_default() += 1;
    }
    counts
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
        out.push_str("机制路由硬规则：即时奖励写国策 `completion_reward`；长期修正必须生成/引用民族精神，用 `add_ideas` 添加，临时状态再用 `remove_ideas` 移除；不要把 `modifier = { ... }` 直接写进国策效果。\n\n");
        out.push_str("国策骨架硬规则：如果要输出国策代码，必须展开成完整 `focus = { ... }` 模板，不准写短国策或半截片段，方便玩家手动调整条件。\n");
        out.push_str(
            "国家筛选块硬规则：必须使用固定结构 `country = { factor = 0 modifier = { add = 10 tag = <TAG> } }`；不要把 `add` 改成 100 或其他数值。\n\n",
        );
        out.push_str("x/y 排布硬规则：如果用户没有给国策树草图，一律套五段模板：y=0 一个开篇国策 x=0；y=1 两到四个展开国策，x 间隔 2；y=2 一个阶段成果 x=0；y=3 两到四个展开国策，x 间隔 2；y=4 一个收尾成果 x=0。不要随机散点。\n\n");
        out.push_str("icon 硬规则：`icon =` 必须填写从目标 MOD、依赖 MOD 或游戏 `interface/*.gfx` 读取到的真实 `GFX_goal*` 国策图标；如果没有图标索引，只能用 `GFX_goal_unknown` 并说明需要先索引，禁止按标题编造 sprite 名。\n\n");
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
        out.push_str("icon = <verified GFX_goal* from interface/*.gfx, or GFX_goal_unknown>\n");
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
