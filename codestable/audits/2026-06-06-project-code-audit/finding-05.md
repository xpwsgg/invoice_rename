---
doc_type: audit-finding
audit: 2026-06-06-project-code-audit
finding_id: "bug-05"
nature: bug
severity: P2
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 05：同名序号后缀与文档不一致

## 速答

用户文档和设计文档都说明冲突时生成 `-1`、`-2` 后缀；当前代码实际生成 `_1`、`_2`。这会造成输出文件名与用户预期不一致。

## 关键证据

- `README.md:26` — 文档写“同名文件冲突时自动加序号 `-1`、`-2` … 直到 `-999`”。
- `README.md:200` — 架构说明写“按 `{stem}-1.pdf`、`{stem}-2.pdf` … 找第一个空位”。
- `docs/superpowers/specs/2026-05-13-pdf-rename-design.md:29` — 需求写“`-1`、`-2`、`-3`…”。
- `src-tauri/src/renamer.rs:166` — 无扩展名时实际生成 `format!("{stem}_{n}")`。
- `src-tauri/src/renamer.rs:168` — 有扩展名时实际生成 `format!("{stem}_{n}.{ext}")`。
- `src-tauri/src/renamer.rs:444` — 单测断言冲突文件为 `26322000000893295511-Felix-TN_1.pdf`，说明代码现状稳定为下划线。

## 影响

冲突文件仍会安全保留，不会覆盖源文件；影响主要是用户根据文档或既有归档规则查找 `-1` 后缀时找不到。若外部脚本或人工流程依赖文档命名，也会出现轻微兼容问题。

## 修复方向

在产品层拍板使用 `-n` 还是 `_n`；若沿用文档，则调整 `resolve_target()` 和测试；若沿用代码，则同步 README 与设计说明。

## 建议动作

`cs-issue`，因为这是用户可见输出格式与需求不一致，需要明确产品口径后修正。
