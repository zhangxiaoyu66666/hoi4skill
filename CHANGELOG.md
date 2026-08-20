# 更新日志

本项目遵循语义化版本。除特别说明外，所有版本均为非官方 HOI4 MOD 工具版本，
不包含或分发 Paradox Interactive 的游戏资产。

## 0.30.7 - 2026-08-20

### 检查模型与误报修复

- 验证报告改为结构化诊断，统一输出严重级别、分类、类别、代码、对象、中文消息和原始证据；宿主应用不再依赖英文文本正则猜测错误类型。
- 区分已确认错误、已确认缺失、未解析引用与工具能力缺口；内容完整性缺口在交互界面中按警告展示，同时保留 CLI 严格门禁语义。
- 项目检查将引擎自动生成的国策 `_desc` 本地化键识别为可选提示，不再误报为确定缺失。
- 脚本扫描器识别引号与注释边界，并补齐动态变量数组、临时作用域与国家标签、大小写不敏感的效果/触发器、GFX 数字帧及自定义资源、修正、专精和别名等合法定义来源。

### 性能

- 游戏、依赖 MOD 与目标 MOD 的分层索引缓存改为 Rayon 有界并行加载，并保持按层确定性合并。
- 原版 Idea 修正证据随游戏层缓存复用；即使依赖 MOD 使用 `replace_path`，也无需在每次验证时重新解析原版定义。
- 不含图标和动态占位符的本地化文本走快速路径，减少大项目无效分词。
- 本机 Release 冷缓存检查：Kaiserreich 由 10.641 秒降至 8.979 秒，TNO 由 12.796 秒降至 10.037 秒；各连续三次报告哈希保持一致。

### 本地化术语

- 新增项目级全局本地化术语表；首次确认的译法可按源语言/目标语言持久保存，并自动注入后续翻译批次。
- `translate-localisation --apply` 在写入前统一检查新翻译和已有目标文本；术语不一致会阻止整批写入，避免部分文件污染。
- 新增 `localisation-glossary --check`，可对整个目标语言目录审计术语一致性并输出逐键诊断。

### 验证

- Rust 测试共 999 项通过，并通过 Release 格式、Clippy、测试与构建检查。
- Kaiserreich 与 TNO 连续三次完整报告 SHA-256 分别稳定为 `E9C3268A492112A8D81CD228B3EBC35653CAB7D7B9A463D14745238C3C001DE1` 与 `D91EF6B9C5AF17BE6156CD1EA29211F99F7E17D3671FD61114D5FF4F080FB4A3`。
- 发布包只包含 Skill、文档和 Windows CLI，不包含或分发 Paradox Interactive 游戏及 MOD 资产。

## 0.30.6 - 2026-08-16

- 验证清单先按扩展名过滤，再读取脚本元数据；大型 MOD 不再为纹理、音频等无关资源逐文件查询大小。
- 并行任务按文件字节量和可用 CPU 动态分块，每核保留有限的可窃取任务，兼顾多核吞吐与稳定内存占用。
- 超大脚本、本地化和 GFX 索引在同一有界线程池内继续拆分独立解析阶段；诊断仍按原文件顺序稳定合并。
- 新增 `HOI4SKILL_PROFILE_VALIDATION=1` 阶段计时，供 Release 性能回归定位索引、清单、解析和合并瓶颈。

## 0.30.5 - 2026-08-16

- 验证文件与 Validation 索引改为有界并行处理，并按精确文件元数据复用持久缓存。
- GFX 贴图存在性检查在一次验证中复用同一份 `replace_path` 分层计划，避免每个引用重复构建。
- 相似代码候选按索引实例缓存；新增 `--compact-report` 供交互式诊断跳过昂贵的全库模糊候选，但不跳过任何合法性检查。
- 新增 `--no-index-cache`，供单文件检查直接并行读取索引源，避免小文件被首次缓存写入阻塞。
- TNO、Kaiserreich 与 Millennium Dawn 大项目检查加入 Release 性能验收。

## 0.30.4 - 2026-08-14

- 游戏、依赖 MOD、目标 MOD 的分层扫描现在遵循重复声明的 `replace_path`，并在递归前剪掉被高层屏蔽的低层目录。
- 新增仅供游戏更新适配使用的 `--replace-path-diagnostics`；它可读取被屏蔽文件并输出有限数量的元数据和内容哈希，但绝不会将旧符号放回有效索引。
- 游戏/代码索引、严格校验、GFX 贴图解析、Clausewitz 参考库与地图数据审计/规划统一使用分层可见性规则。

## 0.30.3 - 2026-08-12

### 性能

- 为游戏代码目录和 Clausewitz 参考结果加入带版本控制的二进制缓存，并按用途裁剪
  `GameIndex` 构建范围，减少重复扫描本机游戏与依赖 MOD。
- 为 GFX 审计加入文件清单和增量刷新；未变化的 `.gfx` 文件可复用上次索引结果。
- 严格校验复用单次读取、注释剥离和解析结果，避免同一脚本被多个检查器重复处理。
- 增加 `release-fast` 构建配置，供开发期快速获得接近发布版的性能表现。

### 新功能

- 增加 `documentation-catalog` / `documentation-query`，可直接检索本机
  `Hearts of Iron IV/documentation` 下的 Markdown 文档，并返回文件、标题、行号和摘要。
- 增加 `gui-layout-audit`，检查明确尺寸容器中的静态越界、建筑分类变化与固定坐标控件的
  潜在错位，以及必须由实机确认的动态布局；可导入 `error.log` 中的 clipping 诊断。
- `code-catalog` 与 `check-code-symbol` 现在支持完整的具名 GFX 资源和 `sprite` 类型查询。
- 国策布局输入可保留描述、耗时、可用/跳过/显示条件、有效性开关、开始效果等字段，
  不再统一退化为默认模板值。

### 修复

- `set_technology` 校验会过滤 `popup` 元数据，同时继续报告真正不存在的科技 ID。
- GFX 索引识别所有具名 `*Type` 块，包括 `corneredTileSpriteType`、
  `frameAnimatedSpriteType`、`progressBarType`、`circularProgressBarType`、
  `maskedShieldType` 和 `textSpriteType`，并兼容大小写与 `textureFile` 写法。
- 将“具名 GFX 是否存在”和“拥有 `texturefile` 的贴图块”拆分索引，避免合法 GUI 资源被误报缺失。
- 严格错误分类增加 `confirmed_missing`、`parser_gap` 与 `runtime_layout_required`，
  让真实缺失、解析能力缺口和实机验证要求不再混为一类。

### 验证

- Rust 测试：947 项通过。
- `cargo fmt --check`、`cargo clippy --release -- -D warnings`、
  `cargo test --release` 与 `cargo build --release` 纳入发布前检查。
- 使用本机 HOI4、Millennium Dawn 父 MOD 与 MDBR 子 MOD 执行严格代码索引校验：
  0 errors，0 warnings。
- MDBR GUI 静态审计正确标记建筑固定坐标风险和动态 `possible_constructions`
  运行时验证要求；不把未提供实机日志的布局问题伪装成已确认错误。

### 已知限制

- 百分比尺寸、动态列表高度和引擎运行时 clipping 仍必须结合游戏实机日志或截图确认。
- 缓存以文件元数据和索引模式判定有效性；索引架构升级会自动放弃旧缓存并重建。
