# hoi4skill Rust 技术改进研究

日期：2026-06-12

## 一句话结论

`hoi4skill` 下一阶段不应该追求“Rust 生态全家桶”，而应该采用少量高杠杆技术，把当前手写 CLI/JSON/扫描/解析/错误报告逐步升级成：

```text
Cargo workspace
  -> hoi4skill-core 业务库
  -> thin hoi4skill-cli
  -> typed reports with serde/serde_json
  -> clap typed commands
  -> winnow Clausewitz parser
  -> miette/thiserror diagnostics
  -> ignore-based file walker
  -> snapshot tests
  -> later: redb cache, Tauri workbench, optional LSP
```

最重要的原则：业务核心先稳定成可测试的 library crate；桌面端、AI Agent、CLI 都只能调用 core，不各自拼字符串或直接改文件。

## 当前 Rust 现状

当前 `hoi4skill-cli/Cargo.toml` 只有两个运行依赖：

```toml
calamine = "0.35.0"
zip = { version = "7.2.0", default-features = false, features = ["deflate"] }
```

这带来几个好处：

- 发布包很干净，Windows 用户不用装 Python/PowerShell 运行环境。
- 编译和分发风险低。
- 逻辑都在本仓库里，容易定位。

但现在已经出现典型增长痛点：

- `args.rs` 手写参数解析，命令越来越多后很容易出现 help、默认值、重复参数、路径参数不一致。
- `json.rs` 手写 JSON 字符串，报告结构越多越容易漏转义或难以被前端/Agent 稳定读取。
- 大量函数返回 `Result<T, String>`，错误没有机器可读分类、源文件 span、help 文案。
- `collect_files` 手写递归扫描，缺少统一忽略规则、并发扫描、错误策略和大 MOD 性能控制。
- `clausewitz_script.rs` 已经承担真实解析职责，但仍是手工字符串扫描；随着校验器增强，应该升级成更结构化的 tokenizer/parser。
- 测试数量已经很多，但生成的大段 HOI4 文本、JSON 报告、Markdown context pack 适合 snapshot 测试，而不是到处写长字符串断言。

## P0：立刻值得做的技术

### 1. Cargo workspace + `hoi4skill-core`

推荐先改工程结构，不急着改业务逻辑：

```text
crates/hoi4skill-core/
  src/lib.rs
  src/workspace.rs
  src/parser/
  src/validation/
  src/reports/
  src/generators/

hoi4skill-cli/
  src/main.rs
  src/commands.rs
```

原因：

- Cargo workspace 天然支持多个 package 共用 `Cargo.lock` 和 target 输出目录，也支持 `cargo check --workspace` 这种跨成员命令。
- 桌面端 Tauri、未来 LSP、AI policy check、CLI 都应复用同一份 core。
- 现在的 release profile 已经在 CLI crate 里；拆 workspace 后 profile 应移到 workspace 根，因为 Cargo 只读取 workspace root 的 profile 设置。

迁移方式：

1. 新建根 `Cargo.toml`，声明 workspace。
2. 复制现有 `hoi4skill-cli/src` 到 `crates/hoi4skill-core/src`，先保留模块名。
3. `hoi4skill-cli` 只保留参数解析、调用 core、stdout/stderr。
4. 保持旧命令和输出格式不变，先让 `cargo test --workspace` 通过。

不要一边拆 crate 一边重写解析器，否则回归范围会炸开。

### 2. `serde` + `serde_json`

优先级很高。

适用面：

- `workflow_report.json`
- `mod_knowledge.json`
- `tag_resolution.json`
- `edit_context` 中嵌入的 dry-run JSON
- `error_report.json`
- Tauri command payload
- 未来 AI policy check 的机器可读报告

当前手写 JSON 可以保留一层兼容函数，但新结构应改成：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowReport {
    pub detected: DetectedSections,
    pub plans: Vec<FeaturePlan>,
    pub changed_files: Vec<PathBuf>,
    pub validation: ValidationReport,
    pub next_steps: Vec<String>,
}
```

收益：

- 报告结构可被 CLI、桌面端、Agent 一致消费。
- 不再手写转义。
- 可以为未来 JSON schema / TypeScript 类型生成留接口。
- Tauri 事件 payload 本身也要求可序列化对象。

### 3. `clap` derive

当前 `parse_args` 很轻，但命令数量已经太多。建议迁移到 `clap` derive：

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Validate(ValidateArgs),
    RunWorkflow(RunWorkflowArgs),
    PrepareEditContext(PrepareEditContextArgs),
}
```

收益：

- 自动 help/version。
- 参数类型化：`PathBuf`、`usize`、`bool`、`Vec<PathBuf>`。
- 重复参数、默认值、枚举值有统一规则。
- 可以把当前 `usage.rs` 的手写 help 逐步淘汰。

兼容策略：

- 先迁移 3 个新/高风险命令：`validate`、`run-workflow`、`prepare-edit-context`。
- 旧 alias 如 `edit-context`、`preflight-context` 必须保留。
- 快速 smoke test 不要假设 `--help` 文本完全固定，应测试关键命令能解析。

### 4. `thiserror` + `miette`

推荐组合：

- core 内部错误用 `thiserror` 定义结构化 enum。
- CLI 输出和带源代码片段的诊断用 `miette`。

目标不是让错误“好看”，而是让 AI 和普通用户知道该修哪一行：

```text
common/national_focus/xxx.txt:42: focus abc uses `mutual_exclusion`
help: use `mutually_exclusive = { focus = <id> }`
```

适用面：

- Clausewitz parser 错误。
- localisation BOM/header/key 错误。
- focus 字段近似拼写。
- event namespace 错误。
- scope 错误，如 country scope 里直接写 state effect。

迁移方式：

1. 保留现有 `Reporter` 的 `errors/warnings` 模型。
2. 新增 `DiagnosticItem { severity, path, span, code, message, help }`。
3. CLI 默认仍打印现有简洁列表。
4. 加 `--format json` 或 `--diagnostic rich` 后输出结构化/富诊断。

### 5. `insta` snapshot tests

这是 dev-dependency，风险低、回报高。

特别适合：

- focus tree 输出。
- event cards 输出。
- `prepare-edit-context` Markdown。
- `mod_knowledge` summary。
- `workflow_report.json`。
- `analyze-error-log` 报告。

现在很多测试只断言包含某些字符串。snapshot 可以把完整输出固定下来，未来改模板时会清楚看到差异。

使用策略：

- 先给 5 个黄金样例建 snapshot。
- snapshot 内容要去绝对路径、时间戳、临时目录。
- 不用 `cargo insta review` 也可以通过 `cargo test` 和 `INSTA_UPDATE` 更新。

## P1：核心能力升级技术

### 6. `winnow` Clausewitz parser

当前 `clausewitz_script.rs` 已经能扫 block 和 assignment，但越往后越需要真正的语法层：

- token：identifier、string、number、operator、brace、comment。
- AST：assignment、block、repeated key、source span。
- error recovery：一个坏块不应让整文件全部无法索引。
- formatter：未来 patch preview 可保留原风格。

推荐 `winnow`，原因是它是 parser combinator，既能声明式组合，也不阻止局部手写 imperative 解析；适合 HOI4 这种“接近但不等于常规语言”的格式。

不要一开始追求完整 HOI4 grammar。第一阶段只替换这些能力：

```text
strip_comments
assignment_value
blocks_named
direct_child_blocks
braced_content_at
```

里程碑：

1. `ParsedFile { items, diagnostics }`
2. `Item::Assignment { key, value, span }`
3. `Item::Block { key, children, span }`
4. 用 AST 改写 focus/event 校验。
5. 用 AST 给 `miette` 提供 span。

### 7. `ignore` walker

推荐替换手写 `collect_files`，但要非常谨慎配置。

`ignore` 的价值：

- 快速递归目录遍历。
- 可配置 glob、file type、hidden、ignore 文件。
- 未来可按系统扫描：只扫 `common/**/*.txt`、`events/**/*.txt`、`interface/**/*.gfx`。

注意：扫描 HOI4 MOD 时不能盲目遵守仓库 `.gitignore`，因为用户可能把生成物忽略了，但它们仍是游戏会加载的真实文件。因此建议封装：

```rust
pub struct FileWalkOptions {
    pub respect_gitignore: bool,
    pub include_hidden: bool,
    pub extensions: Vec<&'static str>,
    pub roots: Vec<SystemRoot>,
}
```

默认：

- 对源代码仓库工具：可 respect gitignore。
- 对目标 MOD/game root：不因 `.gitignore` 隐藏游戏会读取的文件。

### 8. `similar` patch preview

桌面工作台和 AI 安全链都需要“写前预览”。`similar` 可做文本 diff：

- 生成前后文件 diff。
- 生成 `PatchPreview` 给 Tauri。
- CLI 可输出 unified-like diff。
- AI policy check 可验证“实际改动是否超出 Scope Contract”。

建议先做纯 Rust patch model：

```rust
pub struct PatchSet {
    pub files: Vec<FilePatch>,
}

pub struct FilePatch {
    pub path: PathBuf,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub hunks: Vec<PatchHunk>,
}
```

然后 writer 先返回 patch，`--apply` 才写入。这样“其他 AI 乱写”会少很多，因为默认路径从直接写文件变成预览/应用两段式。

### 9. `redb` persistent cache

当前 Clausewitz code library 是 `index.tsv + snippets.dat`，这个设计很轻。下一阶段如果要做更快查询、增量索引和桌面常驻缓存，可以引入 `redb`。

适合缓存：

- game index：TAG、sprite、state/province、technology、modifier。
- Clausewitz examples metadata。
- mod workspace scan cache。
- 文件 fingerprint -> parsed AST。
- error.log 历史问题。

为什么不是马上上 SQLite：

- `redb` 是纯 Rust 嵌入式 KV，ACID、MVCC、crash-safe，发布包更简单。
- 当前需求更像 key-value / index cache，不是复杂 SQL 查询。

注意：

- 不要把用户游戏/模组代码原文打包或提交。
- cache 必须可删除、可重建。
- cache key 必须包含 game/mod path、mtime/size/hash、hoi4skill version。

### 10. `tracing`

等 core 拆出来后再加。它适合：

- game index 构建耗时。
- parser 文件级耗时。
- Tauri 任务进度。
- error-log 分析流程。

CLI 默认不用刷满日志。建议：

- `--verbose` 打开 human log。
- `--trace-json` 输出机器可读事件。
- Tauri 后端把 tracing event 映射成 task progress。

## P2：产品化和生态技术

### 11. Tauri v2 typed commands + event progress

已有 `TAURI_RUST_FLUENT_DESKTOP_ROADMAP.md`，这里补充 Rust 技术侧：

- Tauri command 只调用 `hoi4skill-core`。
- command 输入输出都用 `serde` 类型。
- 长任务不要阻塞 UI：建立 task model。
- Rust 侧通过 event/channel 发进度。
- 前端永远不直接写 MOD 文件，只提交 patch apply 请求。

Tauri 文档里事件 payload 需要可序列化并可 clone；这进一步支持 `serde` 作为 core 报告格式。

### 12. Optional LSP：`tower-lsp`

只有在 parser/diagnostics 稳定后才做 LSP。

适合功能：

- `.txt` 中 hover 显示 effect/trigger 文档。
- completion：GFX、focus id、event namespace、modifier。
- diagnostics：大括号、近似字段、scope 错误、本地化缺失。
- code action：把 `mutual_exclusion` 修成 `mutually_exclusive`。

不建议 P0 做，因为 LSP 会引入 async runtime、协议复杂度和编辑器生态适配。先让 CLI/core 诊断稳定。

### 13. `rayon`

暂不列为必需。只有当真实大 MOD 扫描/索引有性能瓶颈时再上。

候选点：

- 多文件解析。
- GFX texture existence check。
- localisation key index。
- Clausewitz library build。

前提：

- parser 和 report 数据结构必须是纯函数/可并发。
- 输出顺序要稳定，避免测试和报告抖动。
- Windows 机械硬盘/杀软环境下，并发读文件可能不一定更快，必须实测。

## 暂时不要用的技术

### 不要先上完整 async/Tokio core

core 当前是文件扫描、解析、生成、校验，主要是同步 IO + CPU 工作。强行 async 会污染 API，让 CLI、测试和 Tauri wrapper 都更复杂。

例外：

- Tauri app 层可以 async。
- LSP 层可以 async。
- core 保持同步函数，外层用任务线程调用。

### 不要先上 tree-sitter

除非已经有可维护的 HOI4 Clausewitz grammar。否则为一门松散脚本语言维护 tree-sitter grammar，成本比 `winnow` AST 高很多。

### 不要把 `redb` 当成源数据

cache 只能是派生物。真相仍然是用户的 game root、mod root、dependency root、input workbook/copy 和 generated report。

### 不要因为桌面端而把业务绑到 Tauri

`hoi4skill-core` 不能依赖 Tauri。否则 CLI、Agent skill 和未来 LSP 都会被桌面壳绑住。

## 推荐引入顺序

### Phase A：结构化报告和 crate 拆分

目标：不改变用户行为，但让代码可以继续长。

1. 根目录 workspace。
2. `crates/hoi4skill-core`。
3. `serde` / `serde_json`。
4. `thiserror`。
5. 迁移 2-3 个报告结构。
6. `cargo test --workspace`。

验收：

- 旧 CLI 命令可运行。
- 现有 JSON 输出字段保持兼容或有明确 version。
- release zip 结构不变。

### Phase B：CLI 和诊断

目标：命令入口变稳，错误能指向文件/位置。

1. `clap` derive 迁移高频命令。
2. 保留旧 alias。
3. `miette` diagnostic item。
4. `validate --format json`。
5. `validate --diagnostic rich`。

验收：

- `validate` 仍能被 Skill 文档中的命令调用。
- 近似字段、scope 错误、localisation 错误有 path/span/help。
- warning-only 仍不被误报成 clean success。

### Phase C：Parser 和 snapshot

目标：减少字符串扫描的隐性 bug。

1. `winnow` tokenizer。
2. AST block/assignment。
3. focus/event 校验迁移到 AST。
4. `insta` snapshots。
5. 黄金样例覆盖 focus/event/decision/idea/context pack。

验收：

- 当前 `cargo test --release` 通过。
- snapshot 清晰展示模板变化。
- parser 错误不会导致整文件索引丢失。

### Phase D：扫描、预览、缓存

目标：支撑大型 MOD 和桌面工作台。

1. `ignore` walker 封装。
2. `similar` patch preview。
3. writer 返回 `PatchSet`，再 apply。
4. `redb` cache 原型。
5. cache rebuild / invalidate。

验收：

- 对 KXC/MDARD 这类大样本扫描稳定。
- 所有写文件命令都能 dry-run patch。
- 删除 cache 后功能完全可恢复。

### Phase E：桌面和 LSP

目标：普通玩家使用，而不是只给命令行用户。

1. Tauri v2 wrapper。
2. task progress events。
3. Fluent UI 工作台。
4. 可视 patch/validation/error-log。
5. parser 稳定后再考虑 `tower-lsp`。

## 依赖建议表

| 技术 | 阶段 | 用途 | 是否推荐 |
| --- | --- | --- | --- |
| Cargo workspace | P0 | 拆 core/cli/desktop | 必须 |
| `serde` | P0 | 共享数据模型 | 必须 |
| `serde_json` | P0 | JSON 报告和 Agent/桌面接口 | 必须 |
| `clap` derive | P0/P1 | 类型化 CLI | 推荐 |
| `thiserror` | P0 | core 错误 enum | 推荐 |
| `miette` | P1 | path/span/help 富诊断 | 推荐 |
| `insta` | P1 | 生成结果 snapshot | 推荐 |
| `winnow` | P1 | Clausewitz tokenizer/parser | 推荐 |
| `ignore` | P1 | 可配置扫描器 | 推荐，但扫描 MOD 时别默认 obey `.gitignore` |
| `similar` | P1 | patch preview / diff | 推荐 |
| `redb` | P1/P2 | 本地 cache/index | 推荐，但只做派生缓存 |
| `tracing` | P2 | 后台任务和性能追踪 | 可选 |
| Tauri v2 | P2 | 桌面工作台 | 推荐，等 core 稳 |
| `tower-lsp` | P2 | 编辑器诊断/补全 | 可选，等 parser 稳 |
| `rayon` | P2 | 并行扫描/索引 | 性能实测后再用 |
| Tokio in core | 暂缓 | async runtime | 不推荐 |
| tree-sitter | 暂缓 | 完整语言 grammar | 暂不推荐 |

## 参考资料

- Cargo workspace: <https://doc.rust-lang.org/cargo/reference/workspaces.html>
- Cargo profiles: <https://doc.rust-lang.org/cargo/reference/profiles.html>
- clap derive: <https://docs.rs/clap/latest/clap/_derive/index.html>
- Serde: <https://serde.rs/>
- serde_json: <https://docs.rs/serde_json>
- thiserror: <https://docs.rs/thiserror>
- miette: <https://docs.rs/miette/latest/miette/>
- winnow: <https://docs.rs/winnow/latest/winnow/>
- ignore: <https://docs.rs/ignore/latest/ignore/>
- insta: <https://docs.rs/insta/latest/insta/>
- similar: <https://docs.rs/similar>
- redb: <https://docs.rs/redb/latest/redb/>
- tracing: <https://docs.rs/tracing>
- Tauri v2 events: <https://v2.tauri.app/develop/calling-frontend/>
- tower-lsp: <https://docs.rs/tower-lsp/latest/tower_lsp/>
