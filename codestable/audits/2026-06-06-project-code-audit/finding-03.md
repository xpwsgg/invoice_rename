---
doc_type: audit-finding
audit: 2026-06-06-project-code-audit
finding_id: "maintainability-03"
nature: maintainability
severity: P2
confidence: high
suggested_action: cs-refactor
status: open
---

# Finding 03：质量门禁当前不通过

## 速答

README 写明 `cargo clippy -- -D warnings` 为“零告警”，并列出 `cargo fmt --check` 作为 CI 风格检查；但本次实跑二者都失败。功能测试仍通过，这不是立即运行故障，但会让维护者误判当前基线。

## 关键证据

- `README.md:89` — `cargo clippy -- -D warnings` 被描述为 `lint，零告警`。
- `README.md:90` — `cargo fmt --check` 被列为格式化检查。
- `src-tauri/src/commands.rs:29` — 当前格式与 `rustfmt` 输出不一致，`cargo fmt --check` 报告该处需要换行重排。
- `src-tauri/src/pdf_parser.rs:36` — `thread_local!` 初始化触发 clippy `missing_const_for_thread_local`。
- `src-tauri/src/renamer.rs:78` — `(len - i) % 3 == 0` 触发 clippy `manual_is_multiple_of`。
- `src-tauri/src/renamer.rs:119` — `unwrap_or_else(|_| InvoiceInfo { ... })` 触发 clippy `unnecessary_lazy_evaluations`。

## 影响

如果 CI 或发布前检查按 README 执行，会在格式和 lint 阶段失败；如果团队忽略这些命令，README 中的质量承诺会失效。这个问题也会掩盖真正重要的 clippy 告警，因为当前基线已经不是零告警。

## 修复方向

运行 `cargo fmt` 并处理 3 个 clippy 告警；如果某条 lint 在当前 Rust 版本下不适合采用，应显式 `allow` 并写明原因。

## 建议动作

`cs-refactor`，因为修复是低风险代码整理和质量门禁恢复。
