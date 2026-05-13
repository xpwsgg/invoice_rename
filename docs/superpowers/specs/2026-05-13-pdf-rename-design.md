# PDF 发票批量重命名工具 — 设计文档

- **日期**：2026-05-13
- **平台**：macOS
- **状态**：设计已确认，待生成实施计划

---

## 1. 背景与目标

用户每月会从开票方收到一批 PDF 格式的中国增值税电子发票（数电票/电子普票/电子专票）。需要按统一规则批量重命名后，归档到子目录中以便日后检索。

**命名规则**：`{发票号码}-{用户名}-{trackingNumber}.pdf`，示例：`26322000000893295511-Felix-000-115-216.pdf`。

提取目标字段是**发票号码**（数电票上的 20 位号码），不是纳税人识别号。

---

## 2. 需求摘要

### 功能性需求
1. 通过 GUI 选择一个**源文件夹**，扫描该文件夹**顶层**的所有 PDF 文件（不递归子目录）。
2. 对每个 PDF：
   - 提取 20 位发票号码。
   - 提取失败（扫描件、加密、格式特殊）时用 `UNKNOWN` 作为前缀占位符。
   - 仍然**复制**（不移动）原文件到输出目录，保留原文件不变。
3. 输出目录是 `{源文件夹}/{trackingNumber}/`，不存在则创建。
4. 一次执行所有 PDF **共用同一个 trackingNumber**。
5. 文件命名冲突时**自动加序号后缀**（`-1`、`-2`、`-3`…），上限 999。
6. 界面有**实时滚动的执行日志**（不持久化到文件）。

### 非功能性需求
- 中文界面（按钮、标签、日志文案）。
- 不记忆上次输入（用户名 / 文件夹），每次启动空白。
- 一次执行的总耗时应在秒级。

### 显式不做
- 不递归扫描子目录。
- 不做"取消执行"按钮（第一版）。
- 不做 OCR（扫描件 PDF 走 UNKNOWN 流程）。
- 不持久化日志到文件。
- 不做深色模式切换（通过 CSS 媒体查询自动适配系统主题）。

---

## 3. 技术选型

| 维度 | 选择 | 理由 |
|---|---|---|
| 应用框架 | Tauri 2.x | 当前稳定主线版本，插件生态成熟 |
| 前端 | Vanilla HTML/CSS/JS | UI 极简，无需打包器，体积最小 |
| 后端语言 | Rust | Tauri 原生支持，性能/类型安全 |
| PDF 提取 | `pdfium-render` crate | 基于 Chrome 同款 PDFium，对中文字体/CMap 支持最稳 |
| 错误类型 | `thiserror` crate | 标准做法，可派生 `Serialize` |
| 临时目录测试 | `tempfile` crate | 测试隔离 |
| 日志推送 | Tauri 2 `ipc::Channel` | 比 emit/listen 更直接、不需要清理订阅 |

---

## 4. 项目结构

```
pdf_rename/
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # 程序入口
│   │   ├── lib.rs              # tauri::Builder 与 commands 注册
│   │   ├── commands.rs         # 暴露给前端的 #[tauri::command]
│   │   ├── pdf_parser.rs       # PDF 文本提取 + 发票号码正则
│   │   ├── renamer.rs          # 文件复制、命名、冲突处理
│   │   └── error.rs            # AppError + Serialize
│   ├── tests/
│   │   ├── fixtures/           # 真实/脱敏 PDF 样本（按需）
│   │   └── extract_real_invoices.rs
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── icons/                  # 应用图标（占位即可）
├── src/                        # 前端（Vanilla）
│   ├── index.html
│   ├── main.js
│   └── style.css
├── package.json                # 仅承载 tauri-cli 脚本
├── docs/
│   └── superpowers/specs/
│       └── 2026-05-13-pdf-rename-design.md
└── README.md
```

---

## 5. UI 设计

### 5.1 布局

垂直从上到下：
1. 标题：**"PDF 发票批量重命名"**。
2. 表单区：
   - 源文件夹：只读 input + 右侧"选择"按钮。
   - 用户名：text input。
   - Tracking Number：text input。
   - 表单右下角："开始重命名"主按钮。
3. 日志区：等宽字体、自动滚动、占满剩余高度。

### 5.2 交互

- **"选择"按钮**：调用 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })`，选中后回填输入框。
- **"开始重命名"按钮**：
  - 先做前端校验（见 §7.1）。
  - 校验通过后：按钮禁用 + 文案改为 "处理中…"；输入框禁用。
  - 执行完成或失败后恢复。
- **日志区**：
  - 行格式 `HH:MM:SS  LEVEL  消息`，等宽字体（`SF Mono` / `Menlo`）。
  - INFO 普通色、WARN 橙色、ERROR 红色。
  - 新行写入时自动滚到底（第一版不实现"用户上滑时停留"的细粒度行为）。
  - 不持久化，应用关闭即清空；下次执行开始时清空。

### 5.3 视觉风格

- 浅色简洁风（macOS 风格），白底、灰边框、强调色 `#0a84ff`。
- 通过 `@media (prefers-color-scheme: dark)` 自动适配深色系统。
- 默认窗口尺寸 720×640，可调整，日志区高度跟随。

---

## 6. 后端模块设计

### 6.1 `pdf_parser.rs`

**公开 API**
```rust
pub fn extract_invoice_number(pdf_path: &Path) -> Result<Option<String>, ParseError>;
```

| 返回 | 含义 |
|---|---|
| `Ok(Some("26322..."))` | 成功提取到 20 位发票号码 |
| `Ok(None)` | PDF 能打开、能抽文字，但没匹配到 20 位发票号码 |
| `Err(...)` | PDF 打不开 / 加密 / 损坏等系统级错误 |

**流程**
1. 用进程级单例 `Pdfium` 实例（`OnceCell` 包装）打开 PDF。
2. 遍历所有 page，调用 `page.text()?.all()` 合并全文。
3. 在合并文本上跑两层正则：
   - 优先：`(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})`
   - 兜底：`(?<!\d)(\d{20})(?!\d)`
4. 返回第一个命中的捕获组。

**Cargo 配置要点**
- 启用 `pdfium-render` 的 `thread_safe` feature（必需）；其他 feature 按 crate 当前文档选择。
- PDFium dylib 的获取方式有两条路（实施时择一，按 pdfium-render crate 当前文档为准）：
  1. 从官方 release 下载预编译 dylib，放到 `src-tauri/lib/libpdfium.dylib`，运行时通过 `Pdfium::bind_to_library(...)` 加载。
  2. 通过 cargo build script 自动拉取并放到 `src-tauri/lib/`。
- 在 `tauri.conf.json` 的 `bundle.resources` 中包含 `lib/libpdfium.dylib`，使打包后的 .app 自带 PDFium，无需用户额外安装。

### 6.2 `renamer.rs`

**类型与公开 API**
```rust
pub struct RenamePlan {
    pub source: PathBuf,
    pub target: PathBuf,
    pub invoice_number: Option<String>,  // None 表示用 UNKNOWN
}

pub struct RenameSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

// 日志输出抽象：让 renamer 不直接耦合 Tauri Channel，便于测试时用 Vec<LogEntry> 收集。
pub trait Logger {
    fn log(&mut self, entry: LogEntry);
}

pub fn build_plan(
    source_dir: &Path,
    user_name: &str,
    tracking_number: &str,
    extract_fn: impl Fn(&Path) -> Result<Option<String>, ParseError>,
) -> Result<Vec<RenamePlan>, IoError>;

pub fn execute_plan(plans: &[RenamePlan], logger: &mut impl Logger) -> RenameSummary;
```

在 `commands.rs` 里实现一个 `ChannelLogger` 把 `Logger::log` 转成 `Channel<LogEntry>::send`；测试代码可以用 `Vec<LogEntry>` 自行实现 `Logger` 验证日志内容。

**命名与目录**
- 基础名：`{发票号码 or UNKNOWN}-{userName}-{trackingNumber}.pdf`。
- 输出目录：`{源文件夹}/{trackingNumber}/`，不存在则递归创建。
- 冲突时按 `name-1.pdf`、`name-2.pdf`… 顺序找第一个不存在的文件名（上限 999）。

**扫描规则**
- 大小写不敏感识别 `.pdf` 扩展名（`Invoice.PDF` 也算）。
- 仅扫描源文件夹**顶层**，不递归子目录。这也天然跳过了已有的 `{trackingNumber}` 子目录（如重复执行）。

**复制语义**
- `std::fs::copy`，保留原文件。
- 单文件失败（系统错误）不中止整批；UNKNOWN 不算失败。

### 6.3 `commands.rs`

```rust
#[derive(Serialize, Clone)]
pub struct LogEntry {
    pub ts: String,       // "HH:MM:SS"
    pub level: String,    // "info" | "warn" | "error"
    pub message: String,
}

#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    on_log: tauri::ipc::Channel<LogEntry>,
) -> Result<RenameSummary, String>;
```

- 入口做参数校验（路径存在、非空、非法字符）。
- 通过 `on_log.send(...)` 实时推日志。
- 整批跑完返回 `RenameSummary`。

### 6.4 `error.rs`

- 用 `thiserror` 定义 `AppError`（`Io` / `Pdf` / `InvalidInput`），实现 `Serialize`。
- 命令返回 `Result<_, String>`（取 `AppError.to_string()`），前端以 error 级别打印。

---

## 7. 数据流

### 7.1 前端校验

| 情况 | 处理 |
|---|---|
| 源文件夹未选 | 红色提示 "请选择源文件夹"，不发起 invoke |
| 用户名为空 | 红色提示 "请输入用户名" |
| 用户名含 `/ \ : * ? " < > \|` | 红色提示 "用户名不能包含特殊字符" |
| Tracking Number 为空 | 红色提示 "请输入 Tracking Number" |
| Tracking Number 含 `/ \ : * ? " < > \|` | 红色提示 "Tracking Number 不能包含特殊字符" |

注：`-` 合法。

### 7.2 后端校验

| 情况 | 处理 |
|---|---|
| 源文件夹不存在 / 不可读 | `Err`，前端 error 日志 |
| 顶层无 PDF | warn `未找到 PDF 文件`，返回 `Summary { total: 0, ok: 0, fail: 0 }` |
| 输出子目录创建失败 | `Err`，中止整批 |

### 7.3 单文件处理（不中止整批）

| 情况 | 处理 | 统计 |
|---|---|---|
| PDF 打不开 / 加密 / 损坏 | error 日志，**不复制** | failed |
| 找不到发票号码 | warn 日志，前缀用 `UNKNOWN`，**仍复制** | success |
| 目标同名已存在 | 自动加序号 `-1`、`-2`…（≤999），info 日志说明 | success |
| 序号 1..999 都被占用 | error 日志 | failed |
| `fs::copy` 失败（磁盘满 / 权限） | error 日志 | failed |

### 7.4 日志条目示例

| 阶段 | 级别 | 文案 |
|---|---|---|
| 启动 | info | `扫描到 5 个 PDF 文件` |
| 成功 | info | `[1/5] a.pdf → 26322000000893295511-Felix-000-115-216.pdf` |
| 兜底 | warn | `[2/5] b.pdf 未匹配到发票号码，使用 UNKNOWN 占位` |
| 加序号 | info | `[3/5] c.pdf → UNKNOWN-Felix-000-115-216-1.pdf（同名已存在，加序号）` |
| 错误 | error | `[4/5] d.pdf 打开失败：File is encrypted` |
| 汇总 | info | `完成：成功 4，失败 1，耗时 3.2s` |

---

## 8. 测试策略

### 8.1 Rust 单元测试（`cargo test`）

| 模块 | 覆盖项 |
|---|---|
| `pdf_parser` | 真实数电票样本 → 提取出预期号码；无发票号 PDF → `Ok(None)`；损坏文件 → `Err` |
| `renamer::build_plan` | mock `extract_fn` + 临时目录 → `RenamePlan` 数量、`target` 路径、UNKNOWN 处理正确 |
| `renamer::execute_plan` | `tempfile::TempDir` 准备假 PDF → 复制后目标存在、原文件还在、冲突序号正确累加 |
| 校验 | 非法字符的用户名 / tracking 被拒绝 |

### 8.2 集成测试

- 在 `src-tauri/tests/fixtures/` 放 2-3 张脱敏的真实数电票（按隐私要求决定是否进 git）。
- `tests/extract_real_invoices.rs` 跑一次完整流程，断言提取结果。

### 8.3 前端

- 第一版**不写前端自动化测试**（Vanilla 项目收益低、调试成本高）。
- 手动验证清单：
  1. 空表单点击"开始" → 看到红色校验提示。
  2. 选择空目录 → "未找到 PDF 文件"。
  3. 含数电票的目录 → 日志逐条刷出，子目录创建，文件名正确。
  4. 同批跑两次 → 第二次出现加序号日志。
  5. 塞一个加密 PDF → 该项 error，其他正常。

### 8.4 CI（可选）

- 第一版不强制。仅本地 `cargo fmt --check && cargo clippy -D warnings && cargo test`。

---

## 9. 取舍与未来扩展

**取舍**
- **不做取消按钮**：典型批量几秒完成，加取消会让命令变成可中断 task，复杂度收益不匹配。
- **不做 OCR**：扫描件占比预期极低，走 UNKNOWN 流程已可用。
- **不持久化日志**：需求未要求，且会引入文件管理复杂度。
- **不记忆上次输入**：用户明确表示不需要。

**未来可扩展**
- 取消按钮（基于 `tokio::sync::CancellationToken` + Tauri 命令级 channel）。
- OCR 兜底（接入 Tesseract，仅在 PDFium 文本提取为空时启用）。
- 持久化日志到 `{源文件夹}/{trackingNumber}/rename.log`。
- Windows / Linux 跨平台分发。

---

## 10. 验收标准

- macOS 上 `npm run tauri dev` 能启动，UI 三项输入 + 按钮 + 日志区可见且符合 §5。
- 选择一个含数张真实数电票的文件夹，输入用户名和 tracking number，点击"开始重命名"后：
  - 子目录 `{trackingNumber}/` 出现在源文件夹下。
  - 文件名严格符合 `{发票号码}-{用户名}-{trackingNumber}.pdf` 格式。
  - 提取失败的文件以 `UNKNOWN-…` 命名并仍被复制。
  - 同名加序号生效。
  - 日志区按 §7.4 格式实时滚动；运行结束有汇总行。
  - 原 PDF 仍保留在源文件夹中。
- `cargo test` 全绿。
- `npm run tauri build` 产出可分发的 `.app`，双击可运行，PDFium 动态库已正确包含。
