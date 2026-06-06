---
doc_type: audit-finding
audit: 2026-06-06-project-code-audit
finding_id: "arch-drift-04"
nature: arch-drift
severity: P2
confidence: high
suggested_action: cs-refactor
status: open
---

# Finding 04：README 架构说明仍描述旧日志模型

## 速答

代码已经从“实时日志面板 + Logger/ChannelLogger”演进为“表格行结果 + ProgressSink/ChannelSink”，但 README 多处仍保留旧模型说明，维护者按文档理解会走错接口。

## 关键证据

- `README.md:27` — 功能列表仍写“实时日志面板（info / warn / error），含进度计数和耗时”；当前 UI 是汇总条、进度条和结果表格。
- `README.md:86` — Rust 单测说明仍写“共 24 条”；本次 `cargo test` 实跑为 40 条。
- `README.md:180` — “关键解耦点”仍开始描述旧实现。
- `README.md:183` — 写 `Logger trait + ChannelLogger`；当前后端类型是 `ProgressSink` 和 `ChannelSink`。
- `src-tauri/src/commands.rs:7` — 当前实现是 `struct ChannelSink`。
- `src-tauri/src/renamer.rs:45` — 当前抽象是 `pub trait ProgressSink`。
- `src/main.js:159` — 当前前端按 `renderRow(row)` 追加表格行，而不是 append 日志。

## 影响

新维护者根据 README 会寻找不存在的 `Logger` / `ChannelLogger`，也会误以为前端有日志级别与耗时输出。这个偏移不会直接破坏运行，但会增加排障和后续改动成本。

## 修复方向

同步 README 的功能列表、命令说明、架构解耦点、测试数量和行级结果数据流；删除旧日志模型描述或改成历史说明。

## 建议动作

`cs-refactor`，因为这是文档与架构说明同步，不改变运行行为。
