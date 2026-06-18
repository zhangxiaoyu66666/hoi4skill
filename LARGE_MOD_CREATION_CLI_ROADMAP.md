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
hoi4skill generate-work-package --mod-root "M:\mods\my_large_mod" --package RUS_political_paths --game-root "C:\path\Hearts of Iron IV" --final-check
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
