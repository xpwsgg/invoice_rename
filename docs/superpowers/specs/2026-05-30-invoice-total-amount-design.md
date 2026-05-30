# 发票价税合计统计 + 表格化结果界面 — 设计文档

- 日期：2026-05-30
- 状态：已与用户确认设计方向，待写实现计划
- 关联：在 `2026-05-13-pdf-rename-design.md`（PDF 重命名工具）基础上增量

## 1. 背景与目标

现有工具扫描源文件夹顶层 PDF，提取**发票号码**，按 `<发票号>-<用户名>-<TN>.pdf` 复制到输出子目录，并以流式**文本日志**展示进度。

本次新增能力：**在重命名的同时，提取每张发票的「价税合计（含税总额）」，并在界面上汇总所有发票的总金额。** 同时按用户要求**重构结果展示区**：把纯文本日志区改为**发票列表表格 + 顶部总金额汇总**。

## 2. 已确认的需求决策

| 决策点 | 结论 |
|--------|------|
| 「总计金额」指什么 | 发票最下方「价税合计（小写）¥XXX」即**含税总额** |
| 提取失败如何处理 | **计入已识别部分的总和**，并明确提示「N 张未识别金额、未计入」；不阻塞 |
| 结果展示形态 | 上方表单不变；下方日志区**重构为表格** |
| 表格列 | `序号 / 源文件名 / 发票号 / 合计 / 结果` |
| 顶部汇总 | 表格上方显示**价税合计总和**（醒目）+ 张数统计 |
| 失败行展示 | 红色高亮，失败原因放在「结果」列 |

## 3. 关键技术验证（真实样本）

样本来源：`~/Downloads/ESI报销发票备份/买工具/000-116-539`，共 13 张电子普通发票（中国增值税）。用 `pdftotext`（poppler，流式模式最接近项目所用 pdfium 的 `text.all()`）提取后验证：

- **13/13 全部命中**价税合计金额，与 `-layout` 模式真值一致。
- **价税合计总和 = ¥983.01**（= 98301 分），可作为实现后的端到端回归基准。
- 货币符号统一为**半角 ¥（U+00A5）**。
- 「小写」括号有两种：**半角 `(小写)` 与全角 `（小写）`**，均需兼容。
- 锚点 `(小写) ¥金额` 在流式文本中**紧邻**出现，比「价税合计」标签更鲁棒（有 1 张 `-layout` 提取失败，靠此锚点仍成功）。

**锚点正则（已用样本验证）：**
```
[(（]\s*小写\s*[)）]\s*[¥￥]?\s*([0-9,]+\.[0-9]{2})
```
- 捕获组为金额字符串（可能含千分位逗号），解析时去逗号、按两位小数转「分」。

### ⚠️ 待实现首步消解的风险
以上基于 **poppler**。项目实际用 **pdfium**，文本块输出顺序可能不同。**实现第一步**：将 1～2 张样本 PDF 复制到 `src-tauri/tests/fixtures/`，写一个临时诊断打印 pdfium `text.all()` 全文，确认 `(小写)` 与金额仍相邻；若顺序不同，据实测调整正则后再定稿。

## 4. 架构方案

**方案 A（采用）**：把单一职责的提取函数从「只返回发票号」升级为「一次打开 PDF、一份全文里同时解析号码与金额」，返回结构体 `InvoiceInfo`。复用现有纯文本解析模式（`find_invoice_number_in_text` 已是纯函数，平行新增 `find_total_amount_in_text`）。

否决方案 B（保留原函数 + 新增独立金额函数）：会导致每个 PDF **打开两次、提取两次文本**，而文本提取正是主要开销。

## 5. 后端详细设计

### 5.1 `pdf_parser.rs`
```rust
// 新增：发票字段解析纯函数（易单测）
pub fn find_total_amount_in_text(text: &str) -> Option<i64>; // 返回「分」

// 新增：聚合结构
pub struct InvoiceInfo {
    pub number: Option<String>,
    pub total_amount_cents: Option<i64>,
}

// 升级：一次打开 PDF，全文里同时解析号码 + 金额
pub fn extract_invoice_info(path: &Path) -> Result<InvoiceInfo, AppError>;
// 保留 find_invoice_number_in_text 不变；extract_invoice_number 可保留或由 info 派生
```
- 金额解析：用 §3 正则在全文匹配；命中后去千分位逗号、`圆/元` 容错非必须（锚点已是小写数字），转为分（`元*100 + 角分`，避免浮点：用字符串分割整数/小数部分）。

### 5.2 `renamer.rs`
```rust
pub struct RenamePlan {
    pub source: PathBuf,
    pub target: PathBuf,
    pub invoice_number: Option<String>,
    pub total_amount_cents: Option<i64>,   // 新增
}

pub struct RenameSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub output_dir: Option<String>,
    pub total_amount_cents: i64,           // 新增：已识别金额之和（分）
    pub amount_recognized: usize,          // 新增
    pub amount_missing: usize,             // 新增
}

// 发给前端的「一行结果」
pub struct InvoiceRow {
    pub index: usize,
    pub total: usize,
    pub source_name: String,
    pub invoice_number: Option<String>,
    pub amount_cents: Option<i64>,
    pub amount_display: Option<String>,    // 后端预格式化 "¥93.33"
    pub status: String,                    // "success" | "failed"
    pub note: String,                      // 失败/警示原因
}
// serde 统一 rename_all = "camelCase" 供前端直接消费

// 金额格式化（千分位），单测覆盖
pub fn format_amount(cents: i64) -> String; // 98301 -> "¥983.01"; 1234567 -> "¥12,345.67"
```
- `build_plan` 的 `extract_fn` 签名改为 `Fn(&Path) -> Result<InvoiceInfo, AppError>`，把 `total_amount_cents` 填入 `RenamePlan`。
- 进度回调 trait 由「日志」改为「行」：
```rust
pub trait ProgressSink { fn row(&mut self, row: InvoiceRow); }
```
- `execute_plan(plans, sink)`：每处理完一张发票 `sink.row(...)`；金额统计**基于所有扫描到的发票**（与复制成败无关）：`total_amount_cents += amount`、`amount_recognized/amount_missing` 计数；返回填好新字段的 `RenameSummary`。
- 移除 `LogEntry` / `Logger`（用户要求表格取代日志）。

### 5.3 `commands.rs` / `lib.rs`
- `rename_pdfs` 的 `on_log: Channel<LogEntry>` → `on_row: Channel<InvoiceRow>`；`ChannelLogger` → `ChannelSink`（实现 `ProgressSink`）。
- 调用 `extract_invoice_info`。
- **全局性失败分层**：源目录不存在、输出目录创建失败 → 返回 `Err(AppError)`（前端在表单错误区显示）；**行级失败**（单文件复制失败、未识别发票号、序号耗尽）→ `InvoiceRow.status="failed"` + `note`。

## 6. 前端详细设计（`index.html` / `style.css` / `main.js`）

panel 区 idle 仍显示「使用说明」；run 切换为**汇总条 + 表格**：

```
┌─ 执行结果 ───────────────────────────── [打开文件夹]┐
│  价税合计总和   ¥983.01                              │  醒目大字
│  共 13 张 · 成功 13 · 失败 0 · 未识别金额 0          │  副行统计
│  ████████████████████  13 / 13                       │  进度条（保留）
├──────┬─────────────────┬──────────────┬────────┬────┤
│  #   │ 源文件名         │ 发票号        │ 合计   │结果│
├──────┼─────────────────┼──────────────┼────────┼────┤
│  1   │ 26117….pdf       │ 26117000…5670│ ¥93.33 │ ✓  │
│ ...  │ x.pdf            │ —            │ —      │ ✗  │  失败行红色
└──────┴─────────────────┴──────────────┴────────┴────┘
```

- **数据流**：`channel.onmessage` 收到 `InvoiceRow` → 追加表格行 + 用 `amountCents` 实时累加顶部总和 + 用 `index/total` 更新进度条；`rename_pdfs` 返回 `summary` → 用 `totalAmountCents / amountRecognized / amountMissing` 校准顶部汇总，显示「打开文件夹」。
- `main.js`：移除 `appendLog` / `classifyLevel` / 日志正则（`PROGRESS_RE`/`SUMMARY_RE`），改为 `renderRow(row)` + `updateSummary(...)` + `resetTable()`；新增 JS `formatAmount(cents)` 与后端 `format_amount` 对齐（实时累加用）。
- `index.html`：panel 内 `#logBox` 换为汇总条 + `<table>`（`<thead>` 固定、`<tbody>` 滚动）。
- `style.css`：表格样式、汇总条醒目样式、失败行红色高亮、源文件名列超长省略号。
- 源文件名列过长用 `text-overflow: ellipsis`，`title` 属性悬停看全名；结果列失败时 `title` 显示 `note`。

## 7. 测试策略

- `pdf_parser`（纯函数单测）：半角/全角括号、半角/全角 ¥、千分位、英文 `Total`、无金额→`None`；用真实样本文本片段作为测试输入。
- `renamer`：
  - `format_amount`：`98301→"¥983.01"`、`1234567→"¥12,345.67"`、`0→"¥0.00"`。
  - `execute_plan`：mock `InvoiceInfo` 含/缺金额，验证 `total_amount_cents`、`amount_recognized`、`amount_missing`、逐行 `InvoiceRow`。
  - 更新现有 `ok_extract`/`none_extract` mock 返回 `InvoiceInfo`。
- **端到端回归**（可选 fixture-gated）：若样本 PDF 入 fixtures，断言 13 张总和 = `98301` 分（¥983.01）。

## 8. 实现顺序（建议）

1. **pdfium 文本实测**（消解 §3 风险）：样本入 fixtures，确认 `(小写)` 与金额相邻，定稿正则。
2. `pdf_parser.rs`：`find_total_amount_in_text` + `InvoiceInfo` + `extract_invoice_info` + 单测。
3. `renamer.rs`：结构体新字段、`format_amount`、`ProgressSink`/`InvoiceRow`、`execute_plan` 汇总、改 `build_plan` 签名、改测试 mock。
4. `commands.rs`/`lib.rs`：channel 类型与 sink 改造、全局错误分层。
5. 前端 `index.html`/`style.css`/`main.js`：表格 + 汇总条 + 数据流。
6. 全量 `cargo test` + `npm run build` + 真实样本人工验证总和 = ¥983.01。

## 9. 风险与缓解

| 风险 | 缓解 |
|------|------|
| pdfium 与 poppler 文本顺序不同，锚点失效 | 实现第一步用 pdfium 实测；正则只依赖 `(小写)+金额` 近邻，不依赖跨行布局 |
| 个别发票格式异常（扫描件无文本层、特殊模板） | 失败不阻塞，计入 `amount_missing` 并在表格标注 |
| 浮点累加误差 | 全程用整数「分」累加，仅展示层格式化 |
| 不同发票类型（专票/全电）措辞差异 | 锚点为通用的「(小写)」；后续遇到新格式再扩充正则与 fixture |

## 10. 前置事项（进入实现前需用户确认）

当前 working tree 有 **7 个文件的未提交改动**（commands.rs / lib.rs / renamer.rs / tauri.conf.json / index.html / main.js / style.css），**与本功能无关但覆盖相同文件**。实现前需决定：
- (a) 先把这些改动提交/作为基线，本功能在其上叠加；或
- (b) 其他处理方式。

否则本功能改动会与既有未提交改动混在一起，难以区分与回滚。
