# hoi4skill

**hoi4skill** 是一个中文优先的 **Hearts of Iron IV（钢铁雄心 IV）MOD 辅助创作工具**。

它的目标很直接：让用户用中文、Word、Excel、Markdown 或一句话描述 MOD 想法，由 AI 负责规划与解释，由 Rust CLI 读取本机游戏 / 目标 MOD / 父 MOD 证据，组装 HOI4 文件并在发布前做严格校验。AI 不直接手写未经验证的游戏代码，所有 effect、trigger、modifier、sprite、tag、technology、scope/container 等引用都必须来自本机索引；查不到就报错并生成修复上下文。

> 本项目为非官方 MOD 工具，不隶属于 Paradox Interactive，不包含、不分发任何 Hearts of Iron IV 游戏资产。

---

## 1. 功能

本仓库目前主要包含两个部分：

- `hoi4-mod-maker/`：可安装的 Skill 包与参考文档。
- `hoi4skill-cli/`：Rust 编写的 CLI 后端，不依赖 Python 或 PowerShell 运行环境。

### 当前已支持

- **一句话 / 文档生成 MOD 计划**：`author-compiler-plan` 和 `run-workflow` 可读取内联文本、Markdown、纯文本、CSV/TSV、Word、Excel 和图片资产，把需求拆成国策、事件、决议、民族精神、动态修正、history/OOB、GUI、地图、资源等受控 lane。
- **本地知识库与增量刷新**：读取用户本机 HOI4、目标 MOD、父 MOD / 依赖 MOD 文件，建立符号、模板、风格、事件链、地图、GUI、资源索引；文件变化后走增量刷新，不把官方或第三方源码内置进仓库。
- **严格代码索引与 AI 保险**：`validate --strict-code-index`、`semantic-repair-search`、`validate-repair-context`、`weak-ai-regression-suite` 会把不存在的 effect / trigger / modifier / sprite / tag / technology / scope typo 变成硬错误，并返回本地相关代码候选给 AI 修复。
- **作用域 / 容器分类**：区分国家、州、省份、MIO、角色、科技、装备、GUI、地图等容器，防止把地区修正、MIO 代码、民族精神、动态修正混在一起；已知的共用语法由索引和作用域契约确认。
- **国策、事件链、决议、民族精神**：支持从中文卡片 / 表格 / 草图生成或扩展内容；事件链可检查触发来源、后续事件、死事件、循环、分支合流和路线阻断。
- **动态修正与 scripted helper**：可把“某效果 + 数值”编译为 scripted_effects 里的可复用动态修正协议，并阻止 AI 把动态修正当普通民族精神乱写。
- **history / OOB / start-date 场景**：支持国家历史、州历史、科技、外交战争、领袖、OOB、陆空海单位分类、师模板和省份查询的组合计划，避免从地名或记忆乱猜 state / province id。
- **地图数据计划**：按低风险州 / 省份编辑、中风险补给 / 战略区域、高风险拓扑拆分 map 改动，并要求拓扑、运行日志和发布 gate 证据。
- **资源与 interface/GFX 注册**：支持 jpg / jpeg / png / webp / tga 国旗三尺寸导入，GUI asset、国策图标、民族精神图标、决议图标、事件图、头像等 sprite / `interface/*.gfx` 注册和 `gfx-audit`。
- **GUI 后端工作流**：可从父 MOD 学习 GUI 风格模板，规划 standalone、决议附 GUI、topbar 挂载、map window 等后端结构，并用 `gui-output-audit`、`gui-runtime-*` 收集运行和截图证据；可视化拖拽编辑器仍属于未来前端。
- **本地化与文案安全**：支持颜色、图标、旗帜、leader、country / cosmetic tag 占位符解析，检查 `£icon`、`[TAG.GetName]`、颜色控制符和用户原文是否丢失；支持多语言本地化翻译 prompt / 写回查漏。
- **一键导出与发布 gate**：`runtime-release-gate` 和 `export-mod` 在导出到 HOI4 launcher 目录前要求严格校验、运行日志、文本对齐、资源、地图 / GUI 可选证据、manifest 和 rollback 记录。

### 构建方式

```text
cd hoi4skill-cli
cargo build --release
```

编译后的二进制文件通常位于：

```text
hoi4skill-cli/target/release/hoi4skill.exe
```

快速检查：

```text
hoi4skill-cli/target/release/hoi4skill.exe --help
hoi4skill-cli/target/release/hoi4skill.exe doctor-skill-install --fix
hoi4skill-cli/target/release/hoi4skill.exe clausewitz-reference --game-root "C:\path\Hearts of Iron IV"
hoi4skill-cli/target/release/hoi4skill.exe build-clausewitz-library --game-root "C:\path\Hearts of Iron IV"
hoi4skill-cli/target/release/hoi4skill.exe query-clausewitz-library --system focus --query "socialist workers revolution"
```

### 示例命令

```text
hoi4skill scaffold --name "My HOI4 Mod" --output "M:\path\my_mod" --launcher-file
hoi4skill mod-knowledge "M:\path\existing_mod" --mod-path "M:\path\dependency.mod" --output mod_knowledge.json
hoi4skill clausewitz-reference --game-root "C:\path\Hearts of Iron IV" --output clausewitz_reference.md
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV"
hoi4skill build-clausewitz-library --game-root "C:\path\Hearts of Iron IV" --code-mod-path "M:\path\requested_mod" --request "加载 requested_mod 的模组代码作为参考"
hoi4skill query-clausewitz-library --system event --query "uprising country event"
hoi4skill prepare-edit-context --input "M:\path\copy.txt" --mod-root "M:\path\existing_mod" --tag SOV --prefix sov_nep --output edit_context.md
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\existing_mod" --tag SOV --prefix sov_nep --output workflow_report.json
hoi4skill run-workflow --input "M:\path\copy.txt" --mod-root "M:\path\existing_mod" --tag SOV --prefix sov_nep --game-root "C:\path\Hearts of Iron IV" --final-check --output workflow_report.json
hoi4skill author-compiler-plan --text "国策效果：添加民族精神 伟大的中国共产党 效果：政治点数+5%" --mod-root "M:\path\existing_mod" --game-root "C:\path\Hearts of Iron IV" --output author_plan.json
hoi4skill asset-import-plan --mod-root "M:\path\existing_mod" --kind flag --tag PRC --file flag.png --require-passed --output flag_plan.json
hoi4skill scenario-compiler-plan --text "中国共产党拥有1936年科技，王明在台上，江西归PRC，开局与CHI战争" --mod-root "M:\path\existing_mod" --game-root "C:\path\Hearts of Iron IV" --output scenario_plan.json
hoi4skill gui-request-workflow --mod-root "M:\path\existing_mod" --style-mod "M:\path\parent_mod" --game-root "C:\path\Hearts of Iron IV" --text "做一个决议附GUI显示工业计划" --output-dir .hoi4skill/gui_request_workflow
hoi4skill check-text-alignment --mod-root "M:\path\existing_mod" --input "M:\path\copy.txt" --expect-title "国策标题"
hoi4skill validate "M:\path\existing_mod" --game-root "C:\path\Hearts of Iron IV" --strict-code-index --text-source "M:\path\copy.txt" --expect-title "事件标题"
hoi4skill plan-history-edit "M:\path\existing_mod" --text "edit history/states owner for state_id 64" --state-id 64 --game-root "C:\path\Hearts of Iron IV" --output history_plan.json
hoi4skill parse-focus-excel --input "M:\path\focus_tree.xlsx" --tag SOV --prefix sov_excel --sheet FocusTree --output focus_review.md
hoi4skill translate-localisation --mod-root "M:\path\existing_mod" --from english --to simp_chinese --format prompt --output loc_translate_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\existing_mod" --from french --to german --format prompt --output loc_fr_to_de_prompt.md
hoi4skill translate-localisation --mod-root "M:\path\existing_mod" --from french --to german --translated-input translated_l_german.yml --apply --report loc_apply_report.json
hoi4skill localisation-glossary --mod-root "M:\path\existing_mod" --from simp_chinese --to english --set "人民委员会=People's Commissariat"
hoi4skill localisation-glossary --mod-root "M:\path\existing_mod" --from simp_chinese --to english --check --output glossary_check.json
hoi4skill validate "M:\path\existing_mod" --game-root "C:\path\Hearts of Iron IV" --request "literal user request"
```

### 适合谁使用

- 想做 HOI4 MOD，但不想手搓每一个大括号的人。
- 想用 AI 辅助写 MOD，但又担心 AI 把文件结构写炸的人。
- 想把中文设定、国策构想、事件草稿、开局历史、地图调整、GUI 需求变成结构化 MOD 文件的人。
- 想做 KR / KX / TNO / OWB 这类大型或父 MOD 子 MOD，但又需要本地证据、严格校验、分包协作和发布 gate 的团队。

### 安装到 AI 编程工具

本仓库的 `hoi4-mod-maker/` 是标准 `SKILL.md` 技能包，可用于 Codex、OpenCode、Claude Code 以及其他兼容 Agent Skills 的工具。

最省事的方式是下载 GitHub Releases 里的 `hoi4skill-agent-skill-v*.zip`，按 [INSTALL_AGENT_SKILL.md](INSTALL_AGENT_SKILL.md) 解压到对应目录。

也可以直接从源码路径安装：

```text
https://github.com/zhangxiaoyu66666/hoi4skill/tree/main/hoi4-mod-maker
```

---

## 2. 未来可能会做什么

下面是一些可能的发展方向，不代表一定会做，也不保证时间表。开源项目画饼可以，烙饼要命，先把能用的部分做好。

### 可能开发图形化前端

未来可能会开发一个更适合普通玩家使用的前端界面，让用户不必直接面对命令行。

设想中的前端可能包括：

- MOD 项目管理面板；
- 国策树可视化编辑器；
- 事件链 / 决议 / 民族精神卡片编辑器；
- `error.log` 可视化诊断；
- GFX 图标注册与预览；
- 一键校验、一键生成、一键导出；
- 与 AI 工具联动，把自然语言需求转成可检查的 MOD 改动计划。

### 可能适配更多 Paradox Interactive 游戏

hoi4skill 当前优先服务于 Hearts of Iron IV，但长期目标不一定只局限于 HOI4。

未来如果条件允许，可能会逐步探索适配更多 P 社游戏的 MOD 工作流，例如：

- Europa Universalis 系列；
- Crusader Kings 系列；
- Victoria 系列；
- Stellaris；
- 其他使用相近脚本结构和 MOD 组织方式的 Paradox 游戏。

理想状态下，它不只是一个 HOI4 工具，而是一个面向 **P 社游戏 MOD 创作的中文优先工具链**。

### 可能增强 AI 工作流

未来可能会继续强化：

- 从一句话生成 MOD 原型；
- 从设计文档生成国策、事件、决议；
- 从已有 MOD 中学习命名风格和文件布局；
- 自动生成本地化文本；
- 自动分析报错并给出修复建议；
- 为 Codex / ChatGPT / 啊拼等工具提供更稳定的后端能力。

---

## 3. 赞助与交流

hoi4skill 目前主要由个人维护。如果这个项目帮你少踩一次 HOI4 MOD 的坑，或者让你少和大括号搏斗半小时，欢迎赞助支持，由于项目主要贡献者是一名尝试自力更生的二级残疾人（https://github.com/zhangxiaoyu66666），如果您想项目可以走得更远，有能力的情况下建议赞助。

赞助会主要用于：

- 继续开发 hoi4skill；
- 测试更多 HOI4 MOD 场景；
- 完善文档和示例；
- 未来可能的图形化前端开发；
- 适配更多 P 社游戏；
- 维持开源项目的长期更新。

### 赞助二维码

![赞助二维码](assets/qr/alipay.jpg)

### QQ 交流群

![QQ 交流群](assets/qr/qq-group.jpg)

也欢迎通过 GitHub Issues 提交问题、建议和使用反馈和点击小星星。

---

## 4. 许可证

hoi4skill 采用 **GNU General Public License v3.0 only（GPL-3.0-only）** 发布。

详情请查看仓库中的 [LICENSE](LICENSE)。

### 发布与使用注意

请不要把本地生成文件、游戏文件或第三方资产误传到公开仓库中：

Hearts of Iron IV、Paradox Interactive 及相关名称属于其各自权利人。本项目只是非官方 MOD 辅助工具，不包含游戏资产，也不代表 Paradox Interactive 官方立场。
