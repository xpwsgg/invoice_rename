---
doc_type: audit-index
audit: 2026-06-06-project-code-audit
scope: src/, src-tauri/src/, Tauri/Vite/Cargo 配置、PDFium 获取脚本、README 与现有设计文档
created: 2026-06-06
status: active
total_findings: 5
---

# project-code-audit 审计报告

## 范围

本次审计覆盖项目真实源码与交付配置：`src/` 前端、`src-tauri/src/` Rust 后端、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`package.json`、`vite.config.js`、`scripts/fetch-pdfium.*`、`README.md` 以及 `docs/superpowers/specs/` 中的现有设计文档。排除 `node_modules/`、`dist/`、图标资源和 `target/` 构建产物。

## 总评

项目核心业务链路清晰，Rust 单元测试覆盖了发票号解析、金额解析、重命名计划、冲突处理和汇总统计，`cargo test` 本次实跑 40 条全部通过，`npm run build` 也通过。主要风险集中在 1 个会影响真实文件处理语义的 bug、1 个桌面应用安全硬化缺口、1 个当前质量门禁不一致问题，以及 2 个用户/维护文档与代码现状不一致的问题。未发现 SQL/命令拼接注入、前端 `innerHTML` 注入或 npm 生产依赖漏洞。

## 发现清单

| # | 性质 | 严重度 | 置信度 | 标题 | 文件 |
|---|---|---|---|---|---|
| 1 | bug | P1 | high | PDF 解析错误被降级成 UNKNOWN，损坏/加密文件会被复制并误报为未识别发票号 | [finding-01.md](finding-01.md) |
| 2 | security | P2 | medium | Tauri CSP 关闭且 capability 绑定所有窗口，安全边界偏宽 | [finding-02.md](finding-02.md) |
| 3 | maintainability | P2 | high | 文档声明的 lint/format 质量门禁当前不通过 | [finding-03.md](finding-03.md) |
| 4 | arch-drift | P2 | high | README 架构说明仍描述旧日志模型，与当前表格/ProgressSink 实现不一致 | [finding-04.md](finding-04.md) |
| 5 | bug | P2 | high | 同名文件序号后缀文档写 `-1`，代码实际生成 `_1` | [finding-05.md](finding-05.md) |

## 按维度分布

| 性质 | P0 | P1 | P2 | 合计 |
|---|---:|---:|---:|---:|
| bug | 0 | 1 | 1 | 2 |
| security | 0 | 0 | 1 | 1 |
| performance | 0 | 0 | 0 | 0 |
| maintainability | 0 | 0 | 1 | 1 |
| arch-drift | 0 | 0 | 1 | 1 |
| **合计** | **0** | **1** | **4** | **5** |

## 验证记录

- `npm run build`：通过，Vite 生产构建成功。
- `cd src-tauri && cargo test`：通过，40 个 Rust 测试全部通过。
- `cd src-tauri && cargo check`：通过。
- `cd src-tauri && cargo fmt --check`：失败，`commands.rs` 与 `pdf_parser.rs` 有格式差异。
- `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings`：失败，3 个 clippy warning 被 `-D warnings` 提升为错误。
- `npm audit --omit=dev --json`：生产依赖漏洞为 0。

## 下一步建议

- **P1 本迭代修**：Finding 1。解析失败和“可解析但未找到发票号”应分层处理，否则加密/损坏 PDF 会被复制为 UNKNOWN，用户后续补救成本高。
- **P2 可顺手修**：Finding 3 和 Finding 5。一个会让 CI/本地质量检查口径失真，一个会让输出文件名与用户文档不一致。
- **P2 安全硬化/文档同步**：Finding 2 和 Finding 4。当前没有发现可直接利用链路，但建议在后续发布前收紧 CSP/capability，并刷新 README 的现状说明。
