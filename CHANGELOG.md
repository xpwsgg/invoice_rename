# Changelog

All notable changes to this project will be documented in this file.

## [0.4.2] - 2026-06-16

### Added ✨

#### 即时扫描
- 选择文件夹后立即扫描并显示所有 PDF 文件
- 实时显示：序号、文件名、发票号、金额、状态
- 立即计算并显示价税合计总和
- 实时统计：总张数、成功/失败/未识别金额

#### 文件移除
- 每个文件显示"移除"按钮，支持排除不需要的文件
- 点击移除后该文件从列表消失，序号自动重编
- 总金额和统计数据实时更新
- 重命名时只处理保留的文件

### Improved 🚀

- **性能优化**：扫描结果缓存，避免重复解析 PDF
- **用户体验**：
  - 移除按钮悬停红色高亮，点击缩小动画
  - 提前验证文件夹是否正确
  - 在填写信息前就知道文件数量和总金额
- **工作流程优化**：
  - 优化前：选择文件夹 → 填表 → 点击按钮 → 等待扫描 → 看到结果
  - 优化后：选择文件夹 → 立即看到结果 → 移除不需要的 → 填表 → 点击按钮 → 直接重命名

### Technical Details

#### Backend (Rust)
- 新增 `scan_pdfs` 命令：扫描文件夹返回文件列表和金额汇总
- 新增 `build_plan_for_files()` 函数：只处理指定的文件列表
- `rename_pdfs` 命令新增 `file_names: Vec<String>` 参数
- 新增 `ScannedFile` 和 `ScanSummary` 数据结构

#### Frontend (JavaScript)
- 新增 `scanFolder()` / `renderScannedFile()` / `removeFile()` / `updateScanSummary()`
- `runRename()` 优化：传递文件名列表给后端

#### Styling (CSS)
- 新增 `.btn-remove`、`.status-badge.pending`、`.status-badge.warn`、`.row-warning` 样式

### Documentation 📚

- 更新 README.md 功能说明和工作流程
- 新增 OPTIMIZATION_SUMMARY.md：即时扫描功能详细说明
- 新增 REMOVE_FEATURE.md：文件移除功能完整文档

## [0.4.1] - 2026-06-06

### Fixed

- **PDF 解析错误处理** (P1): PDF 解析失败（加密/损坏/无法打开）的文件不再被复制为 UNKNOWN 文件，现在会正确标记为 failed 并显示具体错误信息
- **代码质量门禁** (P2): 修复 cargo fmt 格式问题和 clippy 警告，现在通过零警告检查
- **文件命名冲突** (P2): 同名文件序号后缀从 `_1`、`_2` 改为 `-1`、`-2`，与文档描述一致
- **Tauri 安全配置** (P2): 
  - 收紧 capability 绑定从 `windows: ["*"]` 到 `windows: ["main"]`
  - 收窄 dialog 权限从 `dialog:default` 到 `dialog:allow-open`
  - 配置 CSP 以提高安全性
- **文档更新** (P2): 更新 README.md 以匹配当前代码实现（ProgressSink/ChannelSink 架构，40 个测试）

### Technical Details

- 在 `RenamePlan` 添加 `parse_error` 字段以区分 PDF 解析失败和未找到发票号
- 修复 clippy 警告：`missing_const_for_thread_local`、`manual_is_multiple_of`
- 所有 42 个测试通过
- 构建和 lint 检查全部通过

## [0.4.0] - Previous Release

[查看完整的版本历史](https://github.com/xpwsgg/invoice_rename/releases)
