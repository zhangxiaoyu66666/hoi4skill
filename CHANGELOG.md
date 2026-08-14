# 更新日志

本项目遵循语义化版本。除特别说明外，所有版本均为非官方 HOI4 MOD 工具版本，
不包含或分发 Paradox Interactive 的游戏资产。

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
