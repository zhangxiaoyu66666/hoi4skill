//! Plain-text national-focus tree layout parsing and application.

#[allow(unused_imports)]
use crate::*;

pub(crate) fn cmd_parse_focus_layout(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = require_value(&map, "input")?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("focus");
    let text = read_utf8_lossy(&normalize_path(&input)?)?;
    let json = parse_focus_layout_json(&text, tag, prefix);
    write_or_print(&json, value(&map, "output"))
}

#[derive(Clone)]
pub(crate) struct FocusNode {
    pub(crate) title: String,
    pub(crate) id: String,
    pub(crate) icon: Option<String>,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) relative_position_id: Option<String>,
    pub(crate) relative_x: Option<i32>,
    pub(crate) relative_y: Option<i32>,
    pub(crate) row: usize,
    pub(crate) column: usize,
    pub(crate) prerequisite: Vec<String>,
    pub(crate) mutually_exclusive: Vec<String>,
    pub(crate) completion_reward: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct FocusRow {
    pub(crate) y: usize,
    pub(crate) tokens: Vec<String>,
    pub(crate) focus_ids: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct FocusLayout {
    pub(crate) tree_id: String,
    pub(crate) rows: Vec<FocusRow>,
    pub(crate) focuses: Vec<FocusNode>,
    pub(crate) mutuals: Vec<(String, String, usize)>,
}

pub(crate) fn parse_focus_layout(text: &str, tag: &str, prefix: &str) -> FocusLayout {
    let mut rows: Vec<(usize, Vec<String>, Vec<String>)> = Vec::new();
    let mut focuses: Vec<FocusNode> = Vec::new();
    let mut mutuals: Vec<(String, String, usize)> = Vec::new();
    let mut used = BTreeSet::new();
    let mut row_index = 0usize;
    let tag_part = sanitize_identifier_part(tag, "TAG").to_ascii_uppercase();

    for line in text.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let tokens = split_focus_line(line);
        let focus_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| !is_mutual_token(t))
            .cloned()
            .collect();
        let count = focus_tokens.len().max(1);
        let mut row_focus_ids = Vec::new();
        let mut col = 0usize;
        for token in &tokens {
            if is_mutual_token(token) {
                continue;
            }
            let (title, id_hint) = parse_focus_token(token);
            let fallback = format!("focus_{row_index}_{col}");
            let mut id = focus_identifier(&tag_part, &title, id_hint.as_deref(), &fallback);
            let base = id.clone();
            let mut n = 2;
            while used.contains(&id) {
                id = format!("{base}_{n}");
                n += 1;
            }
            used.insert(id.clone());
            let x = (col as i32 * 2) - (count as i32 - 1);
            focuses.push(FocusNode {
                title,
                id: id.clone(),
                icon: None,
                x,
                y: row_index as i32,
                relative_position_id: None,
                relative_x: None,
                relative_y: None,
                row: row_index,
                column: col,
                prerequisite: Vec::new(),
                mutually_exclusive: Vec::new(),
                completion_reward: Vec::new(),
            });
            row_focus_ids.push(id);
            col += 1;
        }
        for (i, token) in tokens.iter().enumerate() {
            if !is_mutual_token(token) {
                continue;
            }
            let left_count = tokens[..i].iter().filter(|t| !is_mutual_token(t)).count();
            if left_count > 0 && left_count < row_focus_ids.len() {
                let left = row_focus_ids[left_count - 1].clone();
                let right = row_focus_ids[left_count].clone();
                link_mutual(&mut focuses, &left, &right);
                mutuals.push((left, right, row_index));
            }
        }
        rows.push((row_index, tokens, row_focus_ids));
        row_index += 1;
    }

    ensure_focus_row_x_spacing(&mut focuses, 2);
    for idx in 0..focuses.len() {
        if focuses[idx].row == 0 {
            continue;
        }
        let prev_row = focuses[idx].row - 1;
        let parent_id = focuses
            .iter()
            .filter(|f| f.row == prev_row)
            .min_by_key(|f| ((f.x - focuses[idx].x).abs(), f.x))
            .map(|f| f.id.clone());
        if let Some(parent_id) = parent_id {
            focuses[idx].prerequisite.push(parent_id);
        }
    }

    FocusLayout {
        tree_id: format!(
            "{}_{}_focus_tree",
            sanitize_identifier_part(prefix, "focus"),
            tag_part
        ),
        rows: rows
            .into_iter()
            .map(|(y, tokens, focus_ids)| FocusRow {
                y,
                tokens,
                focus_ids,
            })
            .collect(),
        focuses,
        mutuals,
    }
}

pub(crate) fn parse_focus_layout_with_rewards(text: &str, tag: &str, prefix: &str) -> FocusLayout {
    let mut layout = parse_focus_layout(text, tag, prefix);
    let reward_lines = focus_layout_reward_lines(text);
    if !reward_lines.is_empty() {
        if let Some(first) = layout.focuses.first_mut() {
            first.completion_reward = reward_lines;
        }
    }
    layout
}

pub(crate) fn focus_layout_reward_lines(text: &str) -> Vec<String> {
    for line in text.lines() {
        let trimmed = line.trim_start();
        let Some(comment) = trimmed.strip_prefix('#') else {
            continue;
        };
        let comment = comment.trim_start();
        let effects = comment
            .strip_prefix("completion_reward:")
            .or_else(|| comment.strip_prefix("completion_reward："))
            .or_else(|| comment.strip_prefix("国策效果:"))
            .or_else(|| comment.strip_prefix("国策效果："))
            .map(str::trim);
        if let Some(effects) = effects {
            return focus_reward_lines_from_effects(effects);
        }
    }
    Vec::new()
}

pub(crate) fn focus_reward_lines_from_effects(effects: &str) -> Vec<String> {
    let suggestions = suggest_common("focus", effects, None, None, None, None);
    let (lines, comments) = decision_effect_lines(&suggestions);
    let mut out = lines;
    out.extend(comments.into_iter().map(|comment| format!("# {comment}")));
    if out.is_empty() {
        vec!["add_political_power = 50".to_string()]
    } else {
        out
    }
}

pub(crate) fn fallback_focus_description(focus: &FocusNode) -> String {
    let title = focus.title.trim();
    let (pressure, direction) = if title.contains("宪法")
        || title.contains("法制")
        || title.contains("制度")
    {
        (
            "旧秩序留下的合法性裂缝仍在国家机器内部回响。",
            "新的制度安排必须把革命、行政与社会动员重新编织在一起，使国家权威拥有可以被群众承认的形式。",
        )
    } else if title.contains("工业")
        || title.contains("工厂")
        || title.contains("五年计划")
        || title.contains("生产")
        || title.contains("铁路")
    {
        (
            "分散的工厂、铁路与计划机关无法独自承担时代的重压。",
            "围绕生产与运输体系的整顿将把资源调度、地方干部和劳动群众纳入同一条国家建设路线。",
        )
    } else if title.contains("农业")
        || title.contains("农民")
        || title.contains("土地")
        || title.contains("乡村")
    {
        (
            "乡村问题从来不是单纯的粮食账本，而是国家与群众关系的根基。",
            "新的政策必须回应农民的现实利益，同时把乡村重新纳入国家能够组织和保护的政治秩序。",
        )
    } else if title.contains("军")
        || title.contains("舰队")
        || title.contains("防务")
        || title.contains("护路")
    {
        (
            "军队的旧习、派系与物资短缺仍在削弱国家意志。",
            "只有把纪律、指挥和后方支援重新统一起来，武装力量才能真正成为政策路线的可靠支柱。",
        )
    } else if title.contains("经济")
        || title.contains("市场")
        || title.contains("贸易")
        || title.contains("投资")
        || title.contains("奈普曼")
    {
        (
            "经济恢复带来的活力与不安正在同时扩散。",
            "国家必须为市场、工人和基层机关划出新的边界，让有限的活力服务于更长远的政治目标。",
        )
    } else if title.contains("党")
        || title.contains("委员会")
        || title.contains("国家")
        || title.contains("强化")
    {
        (
            "路线分歧与行政惰性正在消耗国家的行动能力。",
            "党和国家机关必须重新确认自己的组织原则，把分散的命令、干部和群众期待收束为清晰的政治方向。",
        )
    } else {
        (
            "现实的压力已经越过了旧办法能够承受的边界。",
            "围绕这一政策的推进将重新整理国家资源、地方力量与社会期待，为下一阶段的路线选择奠定基础。",
        )
    };
    format!("{pressure}{title}不应只是纸面上的口号，而应成为国家重新组织自身的契机。{direction}")
}

pub(crate) fn fallback_decision_description(card: &Card) -> String {
    let title = card.title.trim();
    if let Some(effect) = card
        .fields
        .get("效果")
        .filter(|value| !value.trim().is_empty())
    {
        let effect = prose_effect_hint(effect);
        format!("围绕{title}的专项措施将由有关机关推动。{effect}这项决议意在把零散的行政命令转化为可以持续执行的国家行动。")
    } else {
        format!("围绕{title}的专项措施将由有关机关推动。它不是临时口号，而是把地方执行、资源调配与政治目标连接起来的行政安排。")
    }
}

pub(crate) fn fallback_idea_description(card: &Card) -> String {
    let title = card.title.trim();
    let origin = card
        .fields
        .get("来源")
        .or_else(|| card.fields.get("获得"))
        .map(String::as_str)
        .unwrap_or("长期的制度变动");
    if let Some(effect) = card
        .fields
        .get("效果")
        .filter(|value| !value.trim().is_empty())
    {
        let effect = prose_effect_hint(effect);
        format!("{title}源自{origin}，已经沉入国家机器与社会关系之中。{effect}它表现为一种持续存在的政治状态，而不是一次新的政策动员。")
    } else {
        format!("{title}源自{origin}，已经沉入国家机器与社会关系之中。它塑造着干部、群众与各级机关面对危机时的判断，也限制着国家调整路线的空间。")
    }
}

pub(crate) fn prose_effect_hint(effect: &str) -> String {
    let mut hints = Vec::new();
    if effect.contains("稳定") {
        hints.push("社会秩序与基层服从因此出现新的变化");
    }
    if effect.contains("战争支持") || effect.contains("动员") {
        hints.push("战争动员与公共情绪也被重新牵引");
    }
    if effect.contains("军工") || effect.contains("民工") || effect.contains("工厂") {
        hints.push("生产体系和地方资源调配受到直接影响");
    }
    if effect.contains("政治点") || effect.contains("权力") {
        hints.push("中央机关获得了更大的政策周转空间");
    }
    if effect.contains("消费品") || effect.contains("经济") || effect.contains("市场") {
        hints.push("经济生活与国家管制之间的平衡被重新调整");
    }
    if effect.contains("海军")
        || effect.contains("陆军")
        || effect.contains("空军")
        || effect.contains("经验")
    {
        hints.push("军队的训练、组织和专业化程度随之改变");
    }
    if hints.is_empty() {
        "这会改变国家内部若干关键制度的运转方式。".to_string()
    } else {
        format!("{}。", hints.join("，"))
    }
}

pub(crate) fn parse_focus_layout_json(text: &str, tag: &str, prefix: &str) -> String {
    let layout = parse_focus_layout_with_rewards(text, tag, prefix);

    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"tag\": {},\n", json_str(tag)));
    out.push_str(&format!("  \"prefix\": {},\n", json_str(prefix)));
    out.push_str(&format!("  \"tree_id\": {},\n", json_str(&layout.tree_id)));
    out.push_str("  \"rows\": [\n");
    for (i, row) in layout.rows.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"y\": {}, \"tokens\": {}, \"focuses\": {}}}",
            row.y,
            json_array(&row.tokens),
            json_array(&row.focus_ids)
        ));
    }
    out.push_str("\n  ],\n  \"focuses\": [\n");
    for (i, f) in layout.focuses.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"title\": {}, \"id\": {}, \"icon\": {}, \"x\": {}, \"y\": {}, \"relative_position_id\": {}, \"row\": {}, \"column\": {}, \"prerequisite\": {}, \"mutually_exclusive\": {}}}",
            json_str(&f.title),
            json_str(&f.id),
            json_optional_str(f.icon.as_deref()),
            f.x,
            f.y,
            json_optional_str(f.relative_position_id.as_deref()),
            f.row,
            f.column,
            json_array(&f.prerequisite),
            json_array(&f.mutually_exclusive)
        ));
    }
    out.push_str("\n  ],\n  \"mutually_exclusive\": [\n");
    for (i, (left, right, row)) in layout.mutuals.iter().enumerate() {
        comma(&mut out, i, "    ");
        out.push_str(&format!(
            "{{\"left\": {}, \"right\": {}, \"row\": {}}}",
            json_str(left),
            json_str(right),
            row
        ));
    }
    out.push_str("\n  ]\n}\n");
    out
}

pub(crate) fn cmd_apply_focus_layout(args: &[String]) -> Result<(), String> {
    let map = parse_args(args);
    let input = require_value(&map, "input")?;
    let mod_root = normalize_path(&require_value(&map, "mod-root")?)?;
    let tag = value(&map, "tag").unwrap_or("TAG");
    let prefix = value(&map, "prefix").unwrap_or("focus");
    let tree_id = value(&map, "tree-id");
    let text = read_utf8_lossy(&normalize_path(&input)?)?;
    let mut layout = parse_focus_layout_with_rewards(&text, tag, prefix);
    if let Some(tree_id) = tree_id {
        layout.tree_id = tree_id.to_string();
    }
    let changed = apply_focus_layout_to_mod(&mod_root, &layout, tag, prefix)?;

    println!("Applied focus layout: {} focuses", layout.focuses.len());
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

pub(crate) fn apply_focus_layout_to_mod(
    mod_root: &Path,
    layout: &FocusLayout,
    tag: &str,
    prefix: &str,
) -> Result<Vec<PathBuf>, String> {
    let mut layout = layout.clone();
    let existing_target = find_country_focus_tree_target(mod_root, tag)?;
    let localisation = collect_focus_localisation_map(mod_root)?;
    let (focus_path, focus_changed) = if let Some(target) = existing_target {
        offset_layout_y(&mut layout, target.max_y + 1);
        let existing_ids = focus_ids_to_avoid(mod_root, Some(&target), &layout, &localisation)?;
        dedupe_layout_focus_ids(&mut layout, &existing_ids);
        append_focus_layout_to_existing_tree(&target, &layout, &localisation)?
    } else {
        let existing_ids = focus_ids_to_avoid(mod_root, None, &layout, &localisation)?;
        dedupe_layout_focus_ids(&mut layout, &existing_ids);
        let focus_path = mod_root
            .join("common")
            .join("national_focus")
            .join(format!("{prefix}_{tag}_focus.txt"));
        let focus_block = render_focus_tree(&layout, tag);
        let focus_changed = append_unique_blocks(
            &focus_path,
            "# Generated national focus tree by hoi4skill\n",
            &[(layout.tree_id.clone(), focus_block)],
        )?;
        (focus_path, focus_changed)
    };

    let loc_entries = layout
        .focuses
        .iter()
        .flat_map(|focus| {
            [
                (focus.id.clone(), focus.title.clone()),
                (
                    format!("{}_desc", focus.id),
                    fallback_focus_description(focus),
                ),
            ]
        })
        .collect::<BTreeMap<_, _>>();
    let loc_path = target_localisation_path(mod_root, tag);
    let loc_changed = append_localisation_entries(&loc_path, &loc_entries)?;

    let mut changed = Vec::new();
    if focus_changed {
        changed.push(focus_path);
    }
    if loc_changed {
        changed.push(loc_path);
    }
    Ok(changed)
}

#[derive(Clone)]
pub(crate) struct FocusTreeInsertionTarget {
    pub(crate) path: PathBuf,
    pub(crate) range: NamedBlockRange,
    pub(crate) existing_ids: BTreeSet<String>,
    pub(crate) max_y: i32,
    pub(crate) focus_count: usize,
}

#[derive(Clone)]
pub(crate) struct NamedBlockRange {
    pub(crate) close: usize,
    pub(crate) end: usize,
    pub(crate) content: String,
}

pub(crate) fn find_country_focus_tree_target(
    mod_root: &Path,
    tag: &str,
) -> Result<Option<FocusTreeInsertionTarget>, String> {
    let focus_root = mod_root.join("common").join("national_focus");
    if !focus_root.exists() {
        return Ok(None);
    }
    let wanted = sanitize_identifier_part(tag, "TAG").to_ascii_uppercase();
    let mut best: Option<FocusTreeInsertionTarget> = None;
    for file in collect_files(&focus_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = read_utf8_lossy(&file)?;
        for range in named_block_ranges(&text, "focus_tree") {
            let country_tag = focus_tree_country_tag(&range.content);
            if country_tag.as_deref() != Some(wanted.as_str()) {
                continue;
            }
            let existing_ids = focus_tree_existing_ids(&range.content);
            let max_y = focus_tree_max_y(&range.content);
            let focus_count = existing_ids.len();
            let candidate = FocusTreeInsertionTarget {
                path: file.clone(),
                range,
                existing_ids,
                max_y,
                focus_count,
            };
            if best
                .as_ref()
                .is_none_or(|current| candidate.focus_count > current.focus_count)
            {
                best = Some(candidate);
            }
        }
    }
    Ok(best)
}

pub(crate) fn append_focus_layout_to_existing_tree(
    target: &FocusTreeInsertionTarget,
    layout: &FocusLayout,
    localisation: &BTreeMap<String, String>,
) -> Result<(PathBuf, bool), String> {
    let append_focuses = layout
        .focuses
        .iter()
        .filter(|focus| !focus_matches_existing_title(target, focus, localisation))
        .collect::<Vec<_>>();

    let text = read_utf8_lossy(&target.path)?;
    if target.range.close >= text.len() || target.range.end > text.len() {
        return Err(format!(
            "{}: focus tree insertion range is out of date",
            target.path.display()
        ));
    }

    let mut rendered = String::new();
    for focus in append_focuses {
        rendered.push('\n');
        rendered.push_str(&render_focus_block(focus));
    }
    if rendered.is_empty() {
        return Ok((target.path.clone(), false));
    }
    let mut updated = String::new();
    updated.push_str(&text[..target.range.close]);
    if !text[..target.range.close].ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&rendered);
    updated.push_str(&text[target.range.close..]);
    fs::write(&target.path, updated)
        .map_err(|e| format!("write {}: {e}", target.path.display()))?;
    Ok((target.path.clone(), true))
}

pub(crate) fn focus_ids_to_avoid(
    mod_root: &Path,
    target: Option<&FocusTreeInsertionTarget>,
    layout: &FocusLayout,
    localisation: &BTreeMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let mut ids = collect_existing_focus_ids(mod_root)?;
    if let Some(target) = target {
        for focus in &layout.focuses {
            if focus_matches_existing_title(target, focus, localisation) {
                ids.remove(&focus.id);
            }
            for existing_id in &target.existing_ids {
                if localisation
                    .get(existing_id)
                    .is_some_and(|title| title == &focus.title)
                {
                    ids.remove(existing_id);
                }
            }
        }
    }
    Ok(ids)
}

pub(crate) fn focus_matches_existing_title(
    target: &FocusTreeInsertionTarget,
    focus: &FocusNode,
    localisation: &BTreeMap<String, String>,
) -> bool {
    target.existing_ids.contains(&focus.id)
        && localisation
            .get(&focus.id)
            .is_some_and(|title| title == &focus.title)
}

pub(crate) fn collect_existing_focus_ids(mod_root: &Path) -> Result<BTreeSet<String>, String> {
    let focus_root = mod_root.join("common").join("national_focus");
    let mut ids = BTreeSet::new();
    if !focus_root.exists() {
        return Ok(ids);
    }
    for file in collect_files(&focus_root)? {
        if file.extension().and_then(OsStr::to_str).unwrap_or("") != "txt" {
            continue;
        }
        let text = strip_comments(&read_utf8_lossy(&file)?);
        ids.extend(focus_tree_existing_ids(&text));
    }
    Ok(ids)
}

pub(crate) fn offset_layout_y(layout: &mut FocusLayout, offset: i32) {
    if offset <= 0 {
        return;
    }
    for focus in &mut layout.focuses {
        focus.y += offset;
    }
    for row in &mut layout.rows {
        row.y += offset as usize;
    }
    for (_, _, row) in &mut layout.mutuals {
        *row += offset as usize;
    }
}

pub(crate) fn ensure_focus_row_x_spacing(focuses: &mut [FocusNode], min_gap: i32) {
    if min_gap <= 0 {
        return;
    }
    let mut rows: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (idx, focus) in focuses.iter().enumerate() {
        rows.entry(focus.row).or_default().push(idx);
    }
    for indexes in rows.values_mut() {
        indexes.sort_by_key(|idx| (focuses[*idx].x, focuses[*idx].column));
        let mut previous_x: Option<i32> = None;
        for idx in indexes {
            if let Some(previous_x) = previous_x {
                let wanted = previous_x + min_gap;
                if focuses[*idx].x < wanted {
                    focuses[*idx].x = wanted;
                }
            }
            previous_x = Some(focuses[*idx].x);
        }
    }
}

pub(crate) fn dedupe_layout_focus_ids(layout: &mut FocusLayout, existing_ids: &BTreeSet<String>) {
    let mut used = existing_ids.clone();
    let mut id_map = BTreeMap::new();
    for focus in &mut layout.focuses {
        let original = focus.id.clone();
        let mut id = original.clone();
        let mut n = 2;
        while used.contains(&id) {
            id = format!("{original}_{n}");
            n += 1;
        }
        used.insert(id.clone());
        if id != original {
            id_map.insert(original, id.clone());
            focus.id = id;
        }
    }
    if id_map.is_empty() {
        return;
    }
    for focus in &mut layout.focuses {
        if let Some(relative_id) = &mut focus.relative_position_id {
            if let Some(new_id) = id_map.get(relative_id) {
                *relative_id = new_id.clone();
            }
        }
        for value in focus
            .prerequisite
            .iter_mut()
            .chain(focus.mutually_exclusive.iter_mut())
        {
            if let Some(mapped) = id_map.get(value) {
                *value = mapped.clone();
            }
        }
    }
    for row in &mut layout.rows {
        for id in &mut row.focus_ids {
            if let Some(mapped) = id_map.get(id) {
                *id = mapped.clone();
            }
        }
    }
    for (left, right, _) in &mut layout.mutuals {
        if let Some(mapped) = id_map.get(left) {
            *left = mapped.clone();
        }
        if let Some(mapped) = id_map.get(right) {
            *right = mapped.clone();
        }
    }
}

pub(crate) fn focus_tree_country_tag(tree: &str) -> Option<String> {
    blocks_named(tree, "country")
        .first()
        .and_then(|block| block_assignment(block, "tag"))
}

pub(crate) fn focus_tree_existing_ids(tree: &str) -> BTreeSet<String> {
    blocks_named(tree, "focus")
        .into_iter()
        .filter_map(|block| block_assignment(&block, "id"))
        .collect()
}

pub(crate) fn focus_tree_max_y(tree: &str) -> i32 {
    blocks_named(tree, "focus")
        .into_iter()
        .filter_map(|block| block_assignment(&block, "y"))
        .filter_map(|value| value.parse::<i32>().ok())
        .max()
        .unwrap_or(-1)
}

pub(crate) fn named_block_ranges(text: &str, name: &str) -> Vec<NamedBlockRange> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut offset = 0usize;
    while let Some(idx) = rest.find(name) {
        let start = offset + idx;
        let before_ok = start == 0
            || text[..start]
                .chars()
                .last()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        let after_name_start = start + name.len();
        let after_trimmed = text[after_name_start..].trim_start();
        let after_ok = after_trimmed
            .chars()
            .next()
            .is_some_and(|c| c == '=' || c == '{');
        if !before_ok || !after_ok {
            let next = idx + name.len();
            rest = &rest[next..];
            offset += next;
            continue;
        }
        let Some(open_rel) = text[after_name_start..].find('{') else {
            break;
        };
        let open = after_name_start + open_rel;
        let Some((content, close)) = braced_content_at(text, open) else {
            break;
        };
        out.push(NamedBlockRange {
            close,
            end: close + 1,
            content,
        });
        rest = &text[close + 1..];
        offset = close + 1;
    }
    out
}

pub(crate) fn render_focus_tree(layout: &FocusLayout, tag: &str) -> String {
    let mut out = String::new();
    out.push_str("focus_tree = {\n");
    out.push_str(&format!("\tid = {}\n", layout.tree_id));
    out.push_str("\tcountry = {\n\t\tfactor = 0\n\t\tmodifier = {\n\t\t\tadd = 10\n");
    out.push_str(&format!("\t\t\ttag = {tag}\n"));
    out.push_str("\t\t}\n\t}\n");
    for focus in &layout.focuses {
        out.push('\n');
        out.push_str(&render_focus_block(focus));
    }
    out.push_str("}\n");
    out
}

pub(crate) fn render_focus_block(focus: &FocusNode) -> String {
    let mut out = String::new();
    out.push_str("\tfocus = {\n");
    out.push_str(&format!("\t\tid = {}\n", focus.id));
    let icon = focus
        .icon
        .as_deref()
        .unwrap_or_else(|| choose_focus_icon(&focus.title));
    out.push_str(&format!("\t\ticon = {icon}\n"));
    out.push_str(&format!(
        "\t\tx = {}\n",
        focus.relative_x.unwrap_or(focus.x)
    ));
    out.push_str(&format!(
        "\t\ty = {}\n",
        focus.relative_y.unwrap_or(focus.y)
    ));
    for parent in &focus.prerequisite {
        out.push_str(&format!("\t\tprerequisite = {{ focus = {parent} }}\n"));
    }
    for other in &focus.mutually_exclusive {
        out.push_str(&format!("\t\tmutually_exclusive = {{ focus = {other} }}\n"));
    }
    if let Some(relative_id) = &focus.relative_position_id {
        out.push_str(&format!("\t\trelative_position_id = {relative_id}\n"));
    } else {
        out.push_str("\t\t# relative_position_id = <focus id for relative placement>\n");
    }
    out.push_str("\t\tcost = 2.5\n");
    out.push_str("\t\tai_will_do = {\n\t\t\tfactor = 10\n\t\t}\n\n");
    out.push_str("\t\tavailable = {\n\t\t}\n\n");
    out.push_str("\t\tbypass = {\n\t\t}\n");
    out.push_str("\t\tcancel_if_invalid = yes\n");
    out.push_str("\t\tcontinue_if_invalid = no\n");
    out.push_str("\t\tavailable_if_capitulated = no\n\n");
    out.push_str("\t\tcompletion_reward = {\n");
    if focus.completion_reward.is_empty() {
        out.push_str("\t\t\tadd_political_power = 50\n");
    } else {
        for line in &focus.completion_reward {
            out.push_str(&indent_lines(line, "\t\t\t"));
        }
    }
    out.push_str("\t\t}\n");
    out.push_str("\t}\n");
    out
}

pub(crate) fn choose_focus_icon(title: &str) -> &'static str {
    if title.contains("海军") || title.contains("舰") {
        "GFX_goal_generic_navy_doctrines_tactics"
    } else if title.contains("军") || title.contains("战争") || title.contains("武装") {
        "GFX_goal_generic_army_doctrines"
    } else if title.contains("工业")
        || title.contains("工厂")
        || title.contains("五年")
        || title.contains("建设")
    {
        "GFX_goal_generic_construct_civ_factory"
    } else if title.contains("农业") || title.contains("农民") {
        "GFX_goal_generic_consumer_goods"
    } else {
        "GFX_goal_generic_political_reform"
    }
}

pub(crate) fn link_mutual(focuses: &mut [FocusNode], left: &str, right: &str) {
    for f in focuses {
        if f.id == left && !f.mutually_exclusive.iter().any(|x| x == right) {
            f.mutually_exclusive.push(right.to_string());
        }
        if f.id == right && !f.mutually_exclusive.iter().any(|x| x == left) {
            f.mutually_exclusive.push(left.to_string());
        }
    }
}

pub(crate) fn parse_focus_token(token: &str) -> (String, Option<String>) {
    let token = token.trim();
    if let Some((title, hint)) = token.rsplit_once('|') {
        let title = title.trim();
        let hint = sanitize_identifier_part(hint.trim(), "");
        if !title.is_empty() && !hint.is_empty() {
            return (title.to_string(), Some(hint));
        }
    }
    if let Some((title, hint)) = bracket_id_hint(token, '[', ']') {
        return (title, Some(hint));
    }
    if let Some((title, hint)) = paren_id_hint(token) {
        return (title, Some(hint));
    }
    (token.to_string(), None)
}

pub(crate) fn bracket_id_hint(token: &str, open: char, close: char) -> Option<(String, String)> {
    let trimmed = token.trim();
    let close_idx = trimmed.rfind(close)?;
    if close_idx + close.len_utf8() != trimmed.len() {
        return None;
    }
    let open_idx = trimmed[..close_idx].rfind(open)?;
    let title = trimmed[..open_idx].trim();
    let hint = trimmed[open_idx + open.len_utf8()..close_idx].trim();
    let hint = sanitize_identifier_part(hint, "");
    (!title.is_empty() && !hint.is_empty()).then(|| (title.to_string(), hint))
}

pub(crate) fn paren_id_hint(token: &str) -> Option<(String, String)> {
    let (title, raw_hint) = bracket_id_hint(token, '(', ')')?;
    let hint = raw_hint
        .strip_prefix("id_")
        .or_else(|| raw_hint.strip_prefix("id"))
        .unwrap_or(&raw_hint)
        .trim_start_matches('_');
    let hint = sanitize_identifier_part(hint, "");
    (!hint.is_empty()).then_some((title, hint))
}

pub(crate) fn focus_identifier(
    tag: &str,
    title: &str,
    id_hint: Option<&str>,
    fallback: &str,
) -> String {
    let fragment = id_hint
        .map(|hint| sanitize_identifier_part(hint, ""))
        .filter(|hint| !hint.is_empty())
        .or_else(|| english_focus_fragment(title))
        .unwrap_or_else(|| sanitize_identifier_part(fallback, "focus"));
    format!(
        "{}_{}",
        sanitize_identifier_part(tag, "TAG").to_ascii_uppercase(),
        fragment
    )
}

pub(crate) fn english_focus_fragment(title: &str) -> Option<String> {
    let mut out = Vec::new();
    let mut rest = title.trim();
    while !rest.is_empty() {
        let mut matched = false;
        for (cn, en) in focus_phrase_dictionary() {
            if let Some(next) = rest.strip_prefix(cn) {
                push_identifier_words(&mut out, en);
                rest = next;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }
        let mut chars = rest.chars();
        let Some(ch) = chars.next() else {
            break;
        };
        let ch_len = ch.len_utf8();
        if ch.is_ascii_alphanumeric() {
            let mut ascii = String::new();
            ascii.push(ch);
            rest = &rest[ch_len..];
            while let Some(next) = rest.chars().next() {
                if next.is_ascii_alphanumeric() {
                    ascii.push(next);
                    rest = &rest[next.len_utf8()..];
                } else {
                    break;
                }
            }
            push_identifier_words(&mut out, &ascii);
        } else if let Some(word) = focus_char_word(ch) {
            push_identifier_words(&mut out, word);
            rest = &rest[ch_len..];
        } else {
            rest = &rest[ch_len..];
        }
    }
    let joined = out.join("_");
    let sanitized = sanitize_identifier_part(&joined, "");
    (!sanitized.is_empty()).then_some(sanitized)
}

pub(crate) fn push_identifier_words(out: &mut Vec<String>, text: &str) {
    for part in text.split('_') {
        let part = sanitize_identifier_part(part, "");
        if !part.is_empty() && out.last().is_none_or(|last| last != &part) {
            out.push(part);
        }
    }
}

pub(crate) fn focus_phrase_dictionary() -> &'static [(&'static str, &'static str)] {
    &[
        ("新经济政策", "new_economic_policy"),
        ("第一个五年计划", "first_five_year_plan"),
        ("西伯利亚干线", "siberian_mainline"),
        ("远东铁路", "far_east_railway"),
        ("远东", "far_east"),
        ("铁路委员会", "railway_committee"),
        ("委员会", "committee"),
        ("重启", "reopen"),
        ("西伯利亚", "siberian"),
        ("干线", "mainline"),
        ("开放", "open"),
        ("太平洋", "pacific"),
        ("贸易", "trade"),
        ("整编", "reorganize"),
        ("护路军", "railway_guard"),
        ("工业移民", "industrial_migration"),
        ("移民计划", "migration_plan"),
        ("工业", "industry"),
        ("移民", "migration"),
        ("计划", "plan"),
        ("港口", "port"),
        ("自由区", "free_zone"),
        ("统一", "unify"),
        ("边疆", "frontier"),
        ("税制", "tax_system"),
        ("设立", "establish"),
        ("工厂", "factory"),
        ("招揽", "invite"),
        ("海外", "overseas"),
        ("资本", "capital"),
        ("东方", "eastern"),
        ("缓冲国", "buffer_state"),
        ("红色", "red"),
        ("铁路", "railway"),
        ("同盟", "alliance"),
        ("宪章", "charter"),
        ("斯大林", "stalin"),
        ("宪法", "constitution"),
        ("五年", "five_year"),
        ("快速", "rapid"),
        ("强化", "strengthen"),
        ("国家", "state"),
        ("继续", "continue"),
        ("奈普曼", "nepmen"),
        ("入党", "join_party"),
        ("发财", "prosper"),
        ("农民", "peasants"),
    ]
}

pub(crate) fn focus_char_word(ch: char) -> Option<&'static str> {
    match ch {
        '军' => Some("army"),
        '海' => Some("sea"),
        '空' => Some("air"),
        '党' => Some("party"),
        '国' => Some("country"),
        '工' => Some("industry"),
        '农' => Some("farm"),
        '资' => Some("capital"),
        '社' => Some("society"),
        '民' => Some("people"),
        '革' => Some("reform"),
        '命' => Some("revolution"),
        '改' => Some("reform"),
        '建' => Some("build"),
        '设' => Some("build"),
        _ => None,
    }
}

pub(crate) fn sanitize_identifier_part(value: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut last_us = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_us = false;
        } else if !out.is_empty() && !last_us {
            out.push('_');
            last_us = true;
        }
    }
    let mut out = out.trim_matches('_').to_string();
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert_str(0, "f_");
    }
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

pub(crate) fn split_focus_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut ws = String::new();
    for ch in line.chars() {
        if ch == '\t' {
            if !cur.trim().is_empty() {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            ws.clear();
        } else if ch.is_whitespace() {
            ws.push(ch);
        } else {
            if ws.chars().count() >= 2 {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                    cur.clear();
                }
            } else {
                cur.push_str(&ws);
            }
            ws.clear();
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

pub(crate) fn is_mutual_token(s: &str) -> bool {
    matches!(s.trim(), "互斥" | "x" | "X")
}
