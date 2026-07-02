# hoi4skill 大型 MOD 制造能力路线

## 核心结论

`hoi4skill` 如果要具备“制造大型 MOD”的能力，不能只停留在“生成一个国策树、写一组事件、校验一次语法”。大型 MOD 的本质是一条生产线：

```text
世界观蓝图 -> 项目结构 -> 内容分包 -> 批量生成 -> 引用追踪 -> 增量校验 -> 试玩日志 -> 回归修复 -> 版本发布
```

所以 CLI 后续最需要增强的是：

- 把大型 MOD 拆成可管理的国家、区域、系统、版本任务；
- 为 AI 和人类提供稳定的项目级上下文；
- 批量生成内容时自动分配 id、命名空间、本地化和资产引用；
- 在写入前预测影响范围，在写入后做严格增量校验；
- 用本地游戏代码、依赖 MOD、error.log 和项目索引形成闭环。

KR、TNO、OWB、KX 这类项目可以作为压力测试样本，但目标不是“适配某一个大型 MOD”，而是让 CLI 有能力支撑任何同等级规模的 HOI4 MOD 创作。

## 当前半自动验收命令

现在应把“能在用户指挥下半自动生产”和“最终可发布”分成两道门：

```text
hoi4skill large-mod-semi-auto-gate --mod-root "<large mod>" --require-ready --output .hoi4skill/semiauto_gate.json --markdown-output .hoi4skill/semiauto_gate.md
hoi4skill large-mod-production-gate --mod-root "<large mod>" --require-semiauto-ready --output .hoi4skill/production_gate.json
```

`large-mod-semi-auto-gate` 只验收半自动生产入口是否成立：

- 用户自然语言是否已经通过 `author-work-packages --text` 自动拆成国策、事件、决议、民族精神、动态修正、本地化六类包；
- `.hoi4skill/author_queue.json` 是否来自文本自动分发，且没有 blocked 或 failed repair 包；
- `.hoi4skill/execution_queue.json` 是否覆盖当前蓝图包数量；
- 是否存在 AI 输出保险或 validation repair context；
- 如果用户请求了动态修正，是否存在动态修正包的 AI 输出保险，防止模型把动态修正当民族精神写。

`large-mod-production-gate` 继续负责最终生产/发布前的严审：能力审计、内容审计、批量验收、严格 code index 校验、文本对齐、GFX/本地化/逻辑审计、包上下文、所有权/依赖图和发布门禁。

## 大型 MOD 和小型 MOD 的区别

小型 MOD 通常只需要：

- 创建骨架；
- 写少量 focus、event、decision、idea；
- 注册少量图标；
- 跑一次 `validate`；
- 根据 error.log 修一轮。

大型 MOD 还需要：

- 多国家、多区域、多系统并行开发；
- 大量事件链和国策树之间的互相触发；
- 数千个 flag、variable、scripted_effect、scripted_trigger；
- 大量本地化 key 和多语言同步；
- 大量 GFX 资产和 spriteType 注册；
- 版本升级时的兼容和迁移；
- 基线错误和新增错误分离；
- 面向 AI 的最小上下文切片；
- 面向团队协作的 ID、命名空间、任务包和回归报告。

因此，CLI 的设计目标应该从“内容写入器”升级为“大型 MOD 工程后端”。

## 代码安全目标：AI 不直接编造 Clausewitz 语法

大型 MOD 生产线必须把“AI 写错语法”当成系统性风险处理，而不是靠人工 review 兜底。目标规则：

- AI 只输出意图、文案、结构化卡片、布局和可解释需求；
- 最终 Clausewitz 代码由 Rust writer 根据本地索引和映射表组装；
- effect、trigger、modifier、building、resource、equipment、technology、sprite、state/province 等引用必须来自 `code-catalog`、目标 MOD、依赖 MOD 或本地 HOI4 game index；
- 任何不在索引里的 effect/trigger/modifier/resource ID 都不能降级为“看起来像代码”的输出；
- 未解析映射、TODO 代码标记、`<idea id for ...>` / `<event id for ...>` / `<number>` 占位符在 `--strict-code-index` / `--final-check` 下直接报错；
- 如果索引缺少 effects/triggers/modifiers 文档，严格模式必须阻止验收，而不是允许 AI 猜测。

对应命令目标：

```text
hoi4skill code-catalog --game-root "<HOI4 root>" --mod-path "<dependency mod>" --output code_catalog.json
hoi4skill validate "<mod>" --game-root "<HOI4 root>" --mod-path "<dependency mod>" --strict-code-index
hoi4skill run-workflow --input cards.txt --mod-root "<mod>" --game-root "<HOI4 root>" --final-check
```

## 作者输入层：人话文案先编译，不直接落地

大型 MOD 的高频输入不是完整 Clausewitz，而是类似：

```text
事件文案：【红色：【中共旗子】【中华人民共和国】领导人【中共领导人】】宣布【中华民国国旗】训政开始。
中国国策：没收资本家
效果：
增加民族精神：
资本家的反抗：
稳定度 -10%
```

CLI 应该把这类输入视为“作者意图”，不是最终代码。第一道闸门是把作者占位符、颜色、本地化控制符、国家/cosmetic tag 别名、GFX 图标、效果同义词解析成机器可检查的计划：

```text
hoi4skill author-placeholder-plan --text "【国民党图标】【中华民国国旗】..." --game-root "<HOI4 root>" --mod-path "<dependency mod>" --output placeholder_plan.json
hoi4skill compile-intent --text "战争正当化 = -10%" --game-root "<HOI4 root>" --strict-code-index --require-final-code --output .hoi4skill/intent.json
hoi4skill apply-focus-intent --input focus.txt --text "增加民族精神：资本家的反抗：稳定度 -10%" --mod-root "<mod>" --tag CHI --prefix chi --game-root "<HOI4 root>" --final-check
```

这层必须坚持：

- `【中华民国国旗】`、`【中华人民共和国领导人】` 只能解析到唯一 indexed 国家或 cosmetic tag；歧义会进入 `country_questions`，不能猜；
- `【国民党图标】` 只能解析到已注册 sprite 或本地化图标别名，例如 `GFX_kmt_party_icon:0 "国民党"`；
- `【红色：...】` 只编译为合法 HOI4 颜色 token，内部占位符继续递归解析；
- 查不到图标、国家、cosmetic tag 或 scripted localisation 时，输出 `questions`，并按类型拆到 `asset_questions` / `country_questions`，要求用户说明，而不是编造；
- 最终本地化只能写 `compiled_text`，不能把原始 `【...】` 占位符落到 `.yml`；
- 同义词、英文近义词和自然语言效果先由模型归一化为结构化 intent，再由 CLI 校验和组装代码，避免靠硬编码词表无限膨胀。
- `# TODO:`、`TODO raw HOI4 block`、`<idea id for ...>`、`<event id for ...>` 等生成残留只能存在于草稿或上下文包；`--strict-code-index` / `--final-check` 必须直接报错。
- `compile-intent` 可以作为草案翻译器；但任何进入写入、CI 或发布流程的 intent 都必须加 `--require-final-code` / `--fail-on-draft`，并带 `--game-root --strict-code-index`。否则 JSON 只能显示候选映射，`safety.final_code_allowed=false`、`patch_plan.can_apply=false`，不能当最终 Clausewitz。
- `generate-work-package --dry-run` 生成的 `.hoi4skill/plan_*.json` 也是草案证据，不是发布凭证；其中 `code_authoring_contract.final_code_allowed=false` 会被 `large-mod-release-gate` 收集并阻断发布，直到对应工作包完成写入、边界检查、验证、交接和回归证据。
- `run-work-package --mod-root ... --package ...` 是单包总控入口：它生成 `.hoi4skill/work_package_runs/<package>/run.json` 和 authoring pack，把国策、事件链、决议、民族精神、动态修正、本地化拆成六条 content lane，并列出 evidence / writer / gate 命令。这个 run manifest 仍然声明 `final_code_allowed=false`，AI 只能交 intent、layout、cards 和本地化；Clausewitz 必须由 Rust writer 产生，并且必须过边界、文本对齐、strict-code-index、logic/loc/gfx、handoff、merge 和 release gate。
- `compile-intent` 和 `dynamic_modifier_change_plan` 会输出 `effect_strategies`：民族精神替换必须是 `replace_national_spirit_with_swap_ideas`，新增民族精神必须显式分成 `create_national_spirit_definition` + `add_existing_or_generated_national_spirit`，动态修正必须是 `dynamic_modifier_scripted_effect_protocol`。弱模型修复时应优先看这个字段，而不是从自然语言 note 猜策略。
- `validate --strict-code-index` / `--final-check` 也会交叉检查最终代码：如果 `add_ideas`、`remove_ideas`、`has_idea` 或 `swap_ideas` 里引用的是 indexed dynamic modifier，直接报错，防止弱模型绕过 intent 层把动态修正当民族精神使用。
- `validate-repair-context` 会把这类错误归入 `dynamic_modifier_misuse`，提取误用的 dynamic modifier ID，并给出 `check-code-symbol --kind dynamic_modifier` 与 `dynamic_modifier_scripted_effect_protocol` 修复提示，避免弱模型把它当成普通缺失 idea。
- `.hoi4skill/ai_context_contract.json`、`context_contract.json`、`placeholder_plan.json`、`author_placeholder_plan.json`、`gfx_register*.json`、`gfx_registration*.json`、`gfx_report.json`、`intent.json`、`compile_intent.json`、`dynamic_modifier_change_plan.json`、`ai_repair_context.json`、`validation_repair_context.json`、`repair_context.json` 会被 `large-mod-release-gate` 自动收集；只要 `final_code_allowed_by_context=false`、`final_code_allowed=false`、`can_apply=false`、`status=blocked`、`status=needs_repair`、或任意 `questions` / `blocked_until_verified` / `blockers` / `errors` / `repair_items` 数组非空，发布门禁必须阻断。
- 发布前还必须生成干净的 `.hoi4skill/ai_context_contract.json`、`.hoi4skill/text_alignment.json`、`.hoi4skill/dependency_graph.json`、`validate --game-root <HOI4 root> --strict-code-index --output .hoi4skill/validation.json` 产生的最终验证报告、真实启动游戏后的 `.hoi4skill/error_log_report.json`、`.hoi4skill/merge_gate.json` 和 `.hoi4skill/playtest_gate.json`；如果 `descriptor.mod` 声明了依赖，还必须生成 `.hoi4skill/mod_dependencies.json` / `dependency_resolution.json` / `resolved_dependencies.json`，且每个依赖都是 `status=resolved`，并生成 `.hoi4skill/mod_knowledge.json` / `knowledge_base.json` 作为子 mod 事实依据。干净 AI 上下文合同必须是 `schema=hoi4skill.ai_context_contract.v1`、`write_gate_status=READY_FOR_NARROW_WRITE`、`strict_code_index=true`、`final_code_allowed_by_context=true`、`allowed_edit_surface` 非空、`verification_steps` 非空，并且没有 `unknown_facts` / `blocked_until_verified`。干净文本对齐报告必须是 `schema=hoi4skill.text_alignment.v1`、`ok=true`、`missing_count=0`，确保用户原文标题/文案/提示没有被 AI 漏写。缺 AI 上下文合同、合同字段不满足、缺文本对齐报告、文本缺失、缺依赖图、`validation.json` 缺 `schema=hoi4skill.validation_report.v1` / `strict_code_index=true` / `game_root`、缺 runtime error log 报告、缺 merge gate、缺依赖解析、缺 mod knowledge、依赖 `missing/ambiguous/unresolved`、缺失 playtest、`playtest_complete=false`、缺包 playtest 报告、passed playtest 缺 `validation_report` / `error_log_report` 或 `needs_review` 都会阻断 `large-mod-release-gate` 和 release bundle。
- `.hoi4skill/repair_bundle*` 目录下的 `repair_bundle.json`、`ai_repair_context.json` / `validation_repair_context.json` / `repair_context.json` 以及其中的 validation / audit 报告也会被发布门和 release bundle 收集；`status=repair_prompt_ready`、`status=needs_repair`、`validation_ok=false` 或 `repair_items` 非空说明还有 AI 修复轮未完成，必须阻断发布。
- 任何被发布门收集的报告如果是 `status=skipped` / `status=draft` / `status=needs_input`，或仍有 `questions`、`asset_questions`、`country_questions`、`skipped_assets`、`unknown_facts`、`blocked_until_verified`，也必须当作未完成证据阻断发布。
- 发布门扫描报告的所有同名字段，而不是只信顶层摘要；即使报告顶部写 `ok=true` / `blocking_count=0`，嵌套项里出现 `ok=false`、失败 gate 布尔值、非零问题/警告计数、非空 `warnings` 数组，或带任意普通 JSON 空白格式的嵌套 `status=blocked/skipped/draft/needs_input/needs_review/warnings/errors/failed/question_required` 也会阻断。
- 发布门还会把空文件、非 JSON 对象、括号未闭合、字符串未闭合、缺逗号/缺冒号、尾随逗号、顶层对象后追加垃圾或第二段 JSON 这类 malformed report 标记为 `needs_review`，不能用坏报告绕过验收。
- 发布门和 release bundle 收集到的 JSON 报告必须带 `schema`，schema 必须是 `hoi4skill.*`，并且要匹配文件名/报告类型；`{}`、无 schema、外部假 schema，或 `loc_audit.json` 填 `hoi4skill.gfx_audit.v1`、`ci_plan.json` 填 `hoi4skill.large_mod_playtest_plan.v1` 这类错配 schema 都不能作为发布证据。

这层已经可以作为“智障 AI 保险丝”：AI 可以把用户文案转成占位符/intent，但 CLI 负责查本地索引、编译、提问和拒绝不确定项。

## 第一层能力：大型 MOD 蓝图生成

建议新增：

```text
hoi4skill plan-large-mod --text "架空一战后世界，德国胜利，东亚多军阀线" --output mod_blueprint.yml
hoi4skill plan-large-mod --input design_doc.md --output mod_blueprint.yml
```

输出不是直接写游戏文件，而是生成大型 MOD 蓝图：

- MOD 名称、缩写、默认语言；
- 世界观摘要；
- 主要国家和可玩国家；
- 区域分组；
- 开局年份、关键时间线；
- 核心玩法系统；
- 国家内容优先级；
- 事件链规划；
- 国策树规模规划；
- 决议系统规划；
- 资产需求；
- 本地化语言；
- 版本里程碑。

这个蓝图应该成为后续批量生成的源头，避免 AI 每次都从零猜项目结构。

## 第二层能力：项目结构和开发分包

建议新增：

```text
hoi4skill init-large-mod --blueprint mod_blueprint.yml --output "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV"
hoi4skill split-work-packages --mod-root "M:\mods\my_large_mod" --blueprint mod_blueprint.yml --output work_packages
```

CLI 应该生成：

- 标准目录结构；
- descriptor；
- 项目配置文件；
- 命名空间规则；
- id 分配规则；
- 国家/区域/系统工作包；
- 每个工作包允许写入的文件范围；
- 每个工作包需要的本地化、GFX、脚本模块；
- 初始校验基线。

大型 MOD 不能只靠一个总 prompt 推进。它需要像软件项目一样拆任务。

## 第三层能力：项目级索引缓存

建议新增：

```text
hoi4skill build-mod-index "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV" --output ".hoi4skill\mod_index.json"
hoi4skill update-mod-index "M:\mods\my_large_mod" --changed-only
```

索引对象至少包括：

- 国家 tag；
- focus tree、focus id、prerequisite、mutually_exclusive、available、bypass、completion_reward；
- event namespace、event id、trigger、option、immediate、after；
- decision category、decision id；
- idea、trait、character、advisor、MIO；
- scripted_effect、scripted_trigger、on_action；
- flag、variable、global_flag、country_flag；
- spriteType、texturefile、event picture、focus icon；
- localisation key；
- state、province、strategic region；
- technology、equipment、unit、template；
- 文件归属的国家、区域、系统、版本阶段。

大型 MOD 不能每次都全量扫描。索引必须支持缓存和增量更新。

## 第四层能力：工作包上下文抽取

建议新增：

```text
hoi4skill feature-context --mod-root "M:\mods\my_large_mod" --tag RUS --output RUS_context.md
hoi4skill feature-context --mod-root "M:\mods\my_large_mod" --region east_asia --output east_asia_context.md
hoi4skill feature-context --mod-root "M:\mods\my_large_mod" --system economic_crisis --output economic_crisis_context.md
```

它应该自动收集：

- 相关设计蓝图；
- 相关国家和区域；
- 相关 focus、event、decision、idea；
- 相关 scripted_effect / scripted_trigger；
- 相关 localisation；
- 相关 GFX；
- 相关 history；
- 当前已分配但未完成的 id；
- 已知依赖和阻塞项；
- 本次任务允许写入和禁止写入的文件范围。

这是 AI 写大型 MOD 的关键能力：给模型的不是整个仓库，而是“刚好够用、带边界、带证据”的上下文。

## 第五层能力：批量内容生成

现有 CLI 已支持国策、事件、特性卡片等写入。大型 MOD 还需要上层编排：

```text
hoi4skill generate-work-package --mod-root "M:\mods\my_large_mod" --package RUS_political_paths --blueprint mod_blueprint.yml --dry-run
hoi4skill run-work-package --mod-root "M:\mods\my_large_mod" --package RUS_political_paths --game-root "C:\path\Hearts of Iron IV" --request "用户原始需求" --output-dir ".hoi4skill\work_package_runs\RUS_political_paths"
```

一个工作包可以包含：

- 一棵或多棵国策树；
- 一组事件链；
- 一组决议；
- 一组民族精神；
- 一组角色或顾问；
- 本地化；
- GFX 注册需求；
- scripted_effect / scripted_trigger；
- 测试场景和校验命令。

CLI 的职责不是替代 AI 写所有文案，而是把 AI 输出变成结构化卡片，再由 Rust writer 写入 Clausewitz 文件，并强制通过本地证据终检。

## 第六层能力：ID 和命名空间分配器

建议新增：

```text
hoi4skill reserve-id --mod-root "M:\mods\my_large_mod" --kind event --namespace rus --count 20
hoi4skill reserve-id --mod-root "M:\mods\my_large_mod" --kind focus --tag RUS --prefix RUS --count 40
hoi4skill check-namespace --mod-root "M:\mods\my_large_mod" --namespace rus
```

它应该能：

- 自动查重 event id；
- 自动查重 focus id；
- 自动查重 idea、decision、character id；
- 自动查重 localisation key；
- 按项目规则推荐前缀；
- 生成一批保留 id；
- 标记 id 归属工作包；
- 输出给 AI 的允许使用 id 列表。

这能解决大型 MOD 最常见的协作灾难：ID 撞车和命名风格失控。

## 第七层能力：符号查询和引用追踪

建议新增：

```text
hoi4skill query-symbol --mod-root "M:\mods\my_large_mod" --symbol RUS_duma_election
hoi4skill query-symbol --mod-root "M:\mods\my_large_mod" --symbol rus.120
hoi4skill query-symbol --mod-root "M:\mods\my_large_mod" --symbol GFX_goal_rus_industry
```

输出应该回答：

- 这个符号在哪里定义；
- 它被哪些文件引用；
- 它属于哪个国家、区域、系统、工作包；
- 它对应哪些 localisation key；
- 它对应哪些 GFX 资源；
- 是否有重复定义；
- 是否是孤儿定义；
- 是否引用了不存在的对象；
- 修改它可能影响哪些事件链、国策树或决议系统。

大型 MOD 的维护效率，取决于能不能快速回答“这个东西到底牵着谁”。

## 第八层能力：改动影响范围分析

建议新增：

```text
hoi4skill impact --mod-root "M:\mods\my_large_mod" --changed "common/national_focus/RUS.txt"
hoi4skill impact --mod-root "M:\mods\my_large_mod" --symbol RUS_duma_election
hoi4skill impact --mod-root "M:\mods\my_large_mod" --git-diff
```

输出内容：

- 直接受影响文件；
- 间接受影响文件；
- 受影响国家 tag；
- 受影响 event namespace；
- 受影响 focus tree；
- 受影响 localisation；
- 受影响 GFX；
- 需要重新校验的最小文件集合；
- 需要试玩验证的场景；
- 推荐运行的校验命令。

这能把“大型 MOD 全量风险”压缩成本次改动的真实风险面。

## 第九层能力：增量校验和基线对比

建议增强现有 `validate`：

```text
hoi4skill validate "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV" --strict-code-index --output baseline.json
hoi4skill validate "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV" --strict-code-index --baseline baseline.json
hoi4skill validate "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV" --strict-code-index --changed-only
hoi4skill validate "M:\mods\my_large_mod" --game-root "C:\path\Hearts of Iron IV" --strict-code-index --since-git HEAD
```

必须区分：

- 原本就存在的问题；
- 本次新增的问题；
- 本次修复的问题；
- 本次触碰文件里的旧问题；
- 本次未触碰区域的问题。

大型 MOD 不能因为历史警告太多就失去校验价值。CLI 的终检应该聚焦“本次改动是否引入新错误”。

## 第十层能力：本地化生产线

建议新增或增强：

```text
hoi4skill loc-audit --mod-root "M:\mods\my_large_mod"
hoi4skill loc-audit --mod-root "M:\mods\my_large_mod" --changed-only
hoi4skill loc-sync-report --mod-root "M:\mods\my_large_mod" --from english --to simp_chinese
hoi4skill loc-generate-pack --work-package RUS_political_paths --language simp_chinese
```

需要支持：

- 从内容卡片生成 loc key；
- 检查脚本引用了但本地化不存在；
- 检查本地化存在但脚本不再引用；
- 检查标题和描述是否成对存在；
- 检查多语言 key 是否同步；
- 检查玩家可见文本是否遗漏用户提供内容；
- 检查重复 key；
- 检查 YAML 编码、BOM、缩进和换行问题。

大型 MOD 的文本量很大，本地化必须成为一等工作流。

## 第十一层能力：GFX 和资产生产线

建议新增：

```text
hoi4skill gfx-audit --mod-root "M:\mods\my_large_mod"
hoi4skill gfx-audit --mod-root "M:\mods\my_large_mod" --changed-only
hoi4skill find-icon --mod-root "M:\mods\my_large_mod" --query "russia industry"
hoi4skill asset-pack-plan --work-package RUS_political_paths --output asset_requirements.md
```

需要检查和管理：

- spriteType 指向不存在的 texturefile；
- 图片文件存在但没有被任何 spriteType 引用；
- spriteType 存在但没有脚本引用；
- 路径大小写不一致；
- 图片格式异常；
- 图片尺寸不符合常见用途；
- focus icon、event picture、idea icon 候选搜索；
- 资产需求清单；
- 缺失资产占位策略。

CLI 不需要亲自画图，但要能管理资产需求、注册、引用和校验。

## 第十二层能力：逻辑可达性检查

建议增强静态分析：

- focus prerequisite 是否断链；
- mutually_exclusive 是否引用不存在的 focus；
- focus tree 是否有孤岛；
- available / bypass 是否引用不存在的 flag、idea、tag、state；
- event 是否永远不可触发；
- event option 是否触发不存在的 event；
- event option 是否只有按钮文本、AI 概率或触发条件，却没有任何效果、隐藏效果、提示效果或后续事件；
- hidden event 是否没有入口；
- decision 是否永远不可见；
- mission timeout / complete / remove_effect 是否引用不一致；
- scripted_effect 和 scripted_trigger 是否存在死引用；
- on_action 是否能进入预期事件链。

这类检查比语法校验更接近“这个大型 MOD 是否真的能玩到内容”。

## 第十三层能力：试玩和 error.log 回归

建议增强现有 `analyze-error-log`：

```text
hoi4skill analyze-error-log --input error.log --mod-root "M:\mods\my_large_mod" --baseline old_error_report.json
hoi4skill analyze-error-log --input error.log --mod-root "M:\mods\my_large_mod" --changed-only
hoi4skill large-mod-fix-queue --mod-root "M:\mods\my_large_mod" --report .hoi4skill/error_log_report.json --output .hoi4skill/fix_queue.json
hoi4skill large-mod-regression-plan --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_plan.json
hoi4skill large-mod-regression-gate --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_gate.json
hoi4skill large-mod-regression-brief --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_brief.md
hoi4skill playtest-report --mod-root "M:\mods\my_large_mod" --tag RUS --from-log error.log --output playtest_report.md
```

目标：

- 只报告新增错误；
- 将 error.log 行映射到具体符号；
- 将错误归类为语法、引用、GFX、本地化、历史文件、地图、AI strategy 等；
- 给出最小修复上下文；
- 给出下一步应运行的 `hoi4skill` 命令；
- 将错误和审计失败归入工作包级修复队列；
- 为修复后的包生成最小回归验证计划；
- 用回归 gate 确认修复项已经重新校验、重扫日志、补齐试玩证据；
- 给制作/测试人员输出可读的回归阻塞摘要；
- 记录每个工作包的试玩状态。

大型 MOD 需要持续回归，而不是只在发布前临时救火。

## 推荐的大型 MOD 制造流程

理想命令链：

```text
hoi4skill detect-hoi4-path
hoi4skill plan-large-mod --input design_doc.md --output mod_blueprint.yml
hoi4skill init-large-mod --blueprint mod_blueprint.yml --output "M:\mods\my_large_mod" --game-root "<HOI4 root>"
hoi4skill split-work-packages --mod-root "M:\mods\my_large_mod" --blueprint mod_blueprint.yml --output work_packages
hoi4skill build-mod-index "M:\mods\my_large_mod" --game-root "<HOI4 root>"
hoi4skill feature-context --mod-root "M:\mods\my_large_mod" --tag RUS --output RUS_context.md
hoi4skill reserve-id --mod-root "M:\mods\my_large_mod" --kind event --namespace rus --count 20
hoi4skill generate-work-package --mod-root "M:\mods\my_large_mod" --package RUS_political_paths --dry-run
hoi4skill generate-work-package --mod-root "M:\mods\my_large_mod" --package RUS_political_paths --game-root "<HOI4 root>" --final-check
hoi4skill validate "M:\mods\my_large_mod" --game-root "<HOI4 root>" --strict-code-index --changed-only
hoi4skill analyze-error-log --input error.log --mod-root "M:\mods\my_large_mod" --changed-only
hoi4skill large-mod-fix-queue --mod-root "M:\mods\my_large_mod" --output .hoi4skill/fix_queue.json
hoi4skill large-mod-regression-plan --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_plan.json
hoi4skill large-mod-regression-gate --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_gate.json
hoi4skill large-mod-regression-brief --mod-root "M:\mods\my_large_mod" --output .hoi4skill/regression_brief.md
```

这个流程的关键是：先蓝图，后分包；先上下文，后生成；先 dry-run，后写入；先增量校验，后试玩回归。

## 最小可行版本

如果只做第一版，建议先实现：

```text
hoi4skill plan-large-mod
hoi4skill init-large-mod
hoi4skill split-work-packages
hoi4skill build-mod-index
hoi4skill feature-context
hoi4skill reserve-id
hoi4skill validate --baseline
hoi4skill validate --changed-only
```

这八个能力能把 `hoi4skill` 从“安全生成小块 HOI4 内容”推进到“可以组织大型 MOD 生产”的阶段。

## 还应该补的关键能力

按“能否在用户指挥下稳定生成 KX/KR 级别内容”的标准，下一批最值得补的是：

1. 任务包上下文合同

   每次给 AI 的上下文都应该有机器可读边界：允许写哪些文件、允许新增哪些 id、必须保留哪些已有符号、需要引用哪些依赖 MOD。没有合同，AI 很容易把一个事件链改成跨系统乱写。

2. 写前 patch 预览和越界审计

   writer 默认先输出 patch plan 和 diff 摘要，只有 `--apply` 才写入。审计必须检查“实际改动是否超出本次任务”，例如用户只要事件链，AI 却新建国家 history 或科技树，就直接阻断。
   当前已落地的基础闸门是 `check-work-package-boundary --from-git --fail-on-violation` / `identify-work-packages --from-git` / `split-changed-work-packages --from-git`：它们直接读取 git 工作树实际改动，不依赖 AI 自报 changed files；在硬闸门模式下发现越界文件会非零退出。

3. 事件链语义检查

   现在可以看事件链图，并已由 `logic-audit` 检查断链、孤儿事件、空选项、循环和事件 flag 生命周期：事件里设置但没人读/清理的 flag、事件里读取但本地没人设置的 flag 都会进入 `issue_count`，从而阻断 release gate。后续还需要继续加强互斥选项、重复触发、缺失 follow-up、本地化 tone 是否一致。大型 MOD 事件最容易坏在这里。

4. 国策树路线互斥和奖励平衡检查

   不只看语法，还要看国策树有没有死路、互斥路线是否闭合、前置是否合理、奖励是否过强、动态修正是否被当民族精神、`swap_ideas` 是否用于替换民族精神。
   当前 intent 编译报告已经把这些路径机器化为 `effect_strategies`，可用于 code review、repair bundle 和 release gate 之前的人工/自动检查。

5. scripted_effect/scripted_trigger 复用库

   常见模式应该先进入项目 helper 库：动态修正变更、政治路线切换、经济改革、战争准备、事件链 flag 设置。用户说“建造速度 +10%”时，CLI 应判断是动态修正、民族精神还是一次性效果，并按上下文选择。
   当前最终验证也会阻断 `add_ideas = <dynamic_modifier>`、`has_idea = <dynamic_modifier>`、`swap_ideas` 内引用 dynamic modifier 的写法，要求改走 dynamic modifier helper 协议。

6. 本地化占位符和控制符终检

   所有 `.yml` 写入前都要检查 `§` 颜色闭合、`£` 图标存在、`[TAG.GetName]` tag 存在、`$VAR$` 是否保留、作者占位符是否已编译。查不到就问用户，不允许进入最终文件。
   当前已落地：`validate --strict-code-index` / `--final-check` 会阻断未编译作者占位符、未闭合颜色、未注册 `£icon`、未索引 `[TAG.GetName]` 作用域，以及 `[中华人民共和国.GetLeader]` 这类未编译中文 scope。`validate-repair-context` 会把这些错误归入 `localisation_token_mapping`，并输出应询问用户的图标/tag/cosmetic tag 映射问题。

7. 资产需求队列

   当文案或国策需要图标、事件图、头像但索引没有时，CLI 应生成 `asset_questions` 和 `asset_todo`，包括建议尺寸、目标目录、sprite 命名、是否已有近似图标。不能用假 `GFX_goal_xxx` 顶上。
   当前 `register-gfx-icons` 产物带 `hoi4skill.gfx_registration.v1` schema；放入 `.hoi4skill/gfx_register*.json`、`.hoi4skill/gfx_registration*.json` 或 `.hoi4skill/gfx_report.json` 后会被发布门和 release bundle 自动收集。`assets_skipped > 0` 或 `skipped_assets` 非空会阻断发布，要求先询问用户语义英文文件名、确认已有图标，或补齐真实资产。

8. 可试玩切片生成

   大型 MOD 不能等全部完成才测试。CLI 应能按国家/路线生成最小试玩切片：启动国策、触发事件、关键决议、预期日志、回归清单。这样每个工作包都能独立验收。

9. 错误修复上下文自动喂回

   `validate-repair-context` 已经是雏形，后续要让 error.log、validate、logic-audit、loc-audit、gfx-audit 统一汇成“给 AI 的修复包”：错误、相关代码、允许改动、禁止猜测、推荐命令。

   当前 `prepare-edit-context` 已经内置 `hoi4skill.ai_context_contract.v1` 和 `hoi4skill.edit_context_repair_insurance.v1`：前者把 `Write Gate`、`final_code_allowed_by_context`、允许写入面、缺失证据、验证步骤、未知事实和 blocked-until-verified 项变成机器可读 JSON，避免弱模型只读自然语言段落后误写；使用 `--output` 时还会默认写出 `.hoi4skill/ai_context_contract.json`，让 `large-mod-release-gate` 直接消费这份上下文红灯；后者从 dry-run 的 `safety.blockers`、`validation.errors`、`validation.warnings` 抽取问题，在有 strict code index 时附带 `related_indexed_code`，并给出 `compile-intent`、`check-code-symbol`、`validate-repair-context`、最终 `validate --strict-code-index` 命令。这样弱模型写错后，修复轮可以直接拿同一个上下文包，不会脱离写入边界和语义检索。

   当前也新增了修复提示组装命令：

   ```text
   hoi4skill ai-repair-prompt --edit-context edit_context.md --repair-context ai_repair_context.json --failed-patch failed.patch --output repair_prompt.md
   ```

   它把写入门禁、repair insurance、validate-repair-context、失败补丁和用户原文合成一份 `hoi4skill.ai_repair_prompt.v1`，强制 AI 输出 `repair_summary`、`changed_files`、`patch_plan`、`questions`、`validation_commands`，防止修复轮变成重新发明一遍。

   如果失败材料来自 `validate --output validation.json`、`analyze-error-log --output error_log_report.json` 或原始 HOI4 `error.log`，先打包：

   ```text
   hoi4skill validate "<mod>" --game-root "<HOI4 root>" --mod-path "<dependency>" --strict-code-index --output validation.json
   hoi4skill repair-failed-output --input validation.json --output failed_output.md
   hoi4skill ai-repair-prompt --edit-context edit_context.md --repair-context ai_repair_context.json --failed-patch failed_output.md --output repair_prompt.md
   ```

   这把“错误日志/验证报告”也接进同一条弱模型修复流水线。`validation.json` 会自带 `mod_root`、`game_root`、`dependency_mods` 和 `changed_files`，`error_log_report.json` 会自带 `mod_root` 和 `changed_files`，所以它们被单独交给另一个 AI 或后续命令时，`repair-failed-output` 仍能恢复完整依赖、保留 changed-only 修复边界，并生成带 `--mod-path` 的 `validate-repair-context` 建议命令。

   当前还可以一键跑验证修复包：

   ```text
   hoi4skill ai-repair-bundle "<mod>" --game-root "<HOI4 root>" --edit-context edit_context.md --output-dir .hoi4skill/repair_bundle
   hoi4skill ai-repair-bundle "<submod>" --game-root "<HOI4 root>" --edit-context edit_context.md --auto-mod-paths --launcher-dir "%USERPROFILE%\Documents\Paradox Interactive\Hearts of Iron IV\mod" --output-dir .hoi4skill/repair_bundle
   hoi4skill ai-repair-bundle "<mod>" --game-root "<HOI4 root>" --input request.txt --tag CHI --prefix chi_demo --output-dir .hoi4skill/repair_bundle
   hoi4skill ai-repair-bundle "<mod>" --game-root "<HOI4 root>" --input request.txt --tag CHI --prefix chi_demo --error-log "%USERPROFILE%\Documents\Paradox Interactive\Hearts of Iron IV\logs\error.log" --output-dir .hoi4skill/repair_bundle
   hoi4skill ai-repair-bundle "<mod>" --game-root "<HOI4 root>" --input request.txt --tag CHI --prefix chi_demo --package country_chi --from-git --output-dir .hoi4skill/repair_bundle
   ```

   这个命令会生成或复用 `edit_context.md`，再生成 `validation.json`、`ai_repair_context.json`、`failed_output.md`、`repair_prompt.md` 和 `repair_bundle.json`。独立 `.hoi4skill/ai_repair_context.json` 或 `.hoi4skill/repair_bundle*/ai_repair_context.json` 会被发布门、release bundle 和 release brief 自动收集；`status=needs_repair`、`effective_errors>0` 或 `repair_items` 非空都会阻断发布。如果提供 `--logic-audit`，它会生成 `logic_audit.json`、`logic_audit_failed_output.md`，把断链、孤儿事件、危险事件环和空事件选项合并进同一个 failed output。如果提供 `--loc-audit`，它会生成 `loc_audit.json`、`loc_audit_failed_output.md`，把缺失本地化、孤儿 key、重复 key、颜色/token 问题合并进同一个 failed output。如果提供 `--gfx-audit`，它会生成 `gfx_audit.json`、`gfx_audit_failed_output.md`，把缺失 sprite、缺失贴图、孤儿 sprite、未注册图片合并进同一个 failed output。如果提供 `--error-log` 或 `--error-log-report`，它还会生成 `error_log_report.json`、`error_log_failed_output.md`，并把静态验证和游戏运行日志合并到同一个 failed output。如果提供 `--package`，它还会运行工作包边界闸门，生成 `boundary.json`、`boundary_failed_output.md`，把越界写入也放进 repair prompt。这样“弱 AI 写坏 -> CLI 检查 -> CLI 打包错误 -> AI 只修复错误项”的循环可以覆盖启动测试后的错误和越界改文件。
   对 KXC 这类依赖 KR/KX 的子 mod，修复包应优先传 `--auto-mod-paths --launcher-dir <HOI4 用户 mod 目录>`，让 CLI 从 `descriptor.mod` 的 `dependencies` 和 launcher `.mod` 文件自动解析依赖根；解析出的依赖会继续传入 strict validation、`validate-repair-context` 和 repair prompt。如果自动解析缺失或歧义，再要求用户用 `--mod-path <dependency>` 指定精确根目录，不能让 AI 把依赖符号误报成不存在。

10. 项目风格模板和用户指定风格学习

    内置基础模板负责结构和最低质量；用户可以指定本地 MOD/文档作为风格参考，但输出必须是风格摘要、词汇禁忌、句式倾向和示例结构，不能硬复制原文。这个风格摘要再被事件、国策、本地化 writer 使用。

    当前事件侧已落地第一版：

    ```text
    hoi4skill event-style-profile "M:\path\style_mod" --template political_drama --language simp_chinese --format json --output event_style_profile.json
    ```

    输出 schema 为 `hoi4skill.event_style_profile.v1`，只包含长度分布、按钮节奏、事件类型比例、场景 cue、内置模板契约和 anti-copy 规则，不输出原事件全文。它适合放进“给 AI 的上下文”里，让 AI 学结构和密度，再由 `apply-event-cards --final-check` 写入和验证。

## 最重要的设计原则

大型 MOD 制造不能依赖一次性神奇生成。CLI 应该坚持以下原则：

- 所有生成都来自结构化蓝图或工作包；
- 所有写入都经过 Rust writer；
- 所有涉及游戏语法的输出都查本地 HOI4 代码和依赖 MOD；
- 所有玩家可见文本都做本地化和文本对齐检查；
- 所有改动都能追踪影响范围；
- 所有校验都能区分基线问题和新增问题；
- AI 只能拿到当前任务需要的上下文和允许写入边界；
- 不凭空创建国家、地图、历史、科技、GUI 等高风险系统，除非用户请求或蓝图明确授权。

最终目标是让 `hoi4skill` 成为大型 HOI4 MOD 的工程化后端：AI 负责构思和草稿，CLI 负责结构、写入、索引、校验、回归和边界。
