# Changelog

All notable changes to this project will be documented in this file.

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

[查看完整的版本历史](https://github.com/YOUR_USERNAME/pdf_rename/releases)
