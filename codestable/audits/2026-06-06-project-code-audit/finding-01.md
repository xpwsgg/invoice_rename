---
doc_type: audit-finding
audit: 2026-06-06-project-code-audit
finding_id: "bug-01"
nature: bug
severity: P1
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 01：PDF 解析错误被降级成 UNKNOWN

## 速答

`build_plan()` 把 `extract_invoice_info()` 的所有错误都吞掉并转成 `InvoiceInfo { number: None, total_amount_cents: None }`，导致加密、损坏、PDFium 打不开等系统级解析失败后，文件仍会被复制成 `UNKNOWN-...pdf`，前端只看到“未识别发票号”。

## 关键证据

- `src-tauri/src/renamer.rs:119` — `let info = extract_fn(&path).unwrap_or_else(|_| InvoiceInfo { ... })`：这里没有区分 `Ok(None)` 与 `Err(_)`，所有 PDF 解析错误都被静默降级。
- `src-tauri/src/renamer.rs:124` — `let prefix = invoice.clone().unwrap_or_else(|| "UNKNOWN".to_string())`：降级后的解析错误进入 UNKNOWN 命名路径。
- `src-tauri/src/renamer.rs:233` — `std::fs::copy(&plan.source, &final_target)`：即使解析错误，后续仍会复制源文件到输出目录。
- `src-tauri/src/renamer.rs:241` — `if plan.invoice_number.is_none()`：最终行级提示是“未识别发票号”，不是“PDF 打不开/加密/损坏”。
- `README.md:206` — 文档说明 `PDF 加密 / 损坏` 应 `跳过此文件` 并计为 failed；当前代码会复制它。

## 影响

用户处理加密、损坏、PDFium 无法打开的文件时，输出目录会出现 UNKNOWN 文件。这个文件不是“抽不到号码但内容可用”的普通兜底，而是解析失败文件；二者被混在一起会误导人工补救，也会污染后续重跑时的同名序号。

## 修复方向

让 `build_plan()` 保留行级解析错误状态，或在解析失败时生成不可复制的 failed plan；`execute_plan()` 根据原因输出不同 `note`，并按当前产品决策决定是否复制。

## 建议动作

`cs-issue`，因为这是用户可触发的行为错误，修复需要重新定义解析失败与 UNKNOWN 兜底的边界。
