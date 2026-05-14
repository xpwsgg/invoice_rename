# PDF 发票批量重命名

一个轻量的 macOS 桌面工具，用于批量整理 PDF 电子发票：自动识别 PDF 中的 20 位发票号码，按 `{发票号}-{用户名}-{Tracking}.pdf` 格式重命名，并复制到以 Tracking Number 命名的子目录中。

底层用 [Tauri 2](https://tauri.app/) + Rust + [PDFium](https://github.com/bblanchon/pdfium-binaries)，前端是零依赖的原生 JS + Vite。

## 功能

- 选源文件夹 → 一键扫描顶层所有 `.pdf` / `.PDF`
- 自动识别发票号码：
  - 优先匹配带字段名的 `发票号码: / Invoice Number: / Invoice No`
  - 退化到独立的 20 位数字片段
  - 都匹配不到时使用 `UNKNOWN` 占位，文件依然会被复制
- 输出路径：`{源目录}/{TrackingNumber}/{发票号}-{用户名}-{TrackingNumber}.pdf`
- 同名文件冲突时自动加序号 `-1`、`-2` … 直到 `-999`
- 实时日志面板（info / warn / error），含进度计数和耗时
- 源文件只读取，不移动、不修改、不删除（用 `copy` 而非 `rename`）

## 文件名规则

```
{20 位发票号 或 UNKNOWN}-{用户名}-{TrackingNumber}.pdf
```

例：

```
源文件：发票_001.pdf
用户名：Felix
Tracking：000-115-216

→ /源目录/000-115-216/26322000000893295511-Felix-000-115-216.pdf
```

用户名和 Tracking Number 不允许包含 `/ \ : * ? " < > |`。

## 环境要求

- macOS（目前只打包了 `libpdfium.dylib`，Apple Silicon 和 Intel 都支持）
- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/) stable toolchain
- [Tauri 2](https://tauri.app/start/prerequisites/) 系统依赖

## 首次准备

```bash
# 1. 安装前端依赖
npm install

# 2. 下载 PDFium 动态库到 src-tauri/lib/
./scripts/fetch-pdfium.sh
```

`fetch-pdfium.sh` 会根据当前 macOS 架构（arm64 / x86_64）从 [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) 拉取对应的预编译库。

## 启动命令速查

| 场景 | 命令 | 说明 |
|---|---|---|
| 桌面应用开发（推荐） | `npm run tauri dev` | 由 Tauri CLI 拉起 Vite dev server（`localhost:1420`），再用 debug 版二进制开窗。修改前端热更新；修改 Rust 触发重编译。首次启动会编译大量 crate，约 1–2 分钟。 |
| 仅前端 | `npm run dev` | 纯 Vite，无 Tauri IPC，调样式/排版用，`invoke()` 会报错。 |
| 前端构建 | `npm run build` | 把 `src/` 打成静态资源到 `dist/`（供 Tauri 打包消费）。 |
| 前端构建产物预览 | `npm run preview` | 用 Vite 起本地 server 浏览 `dist/`。 |
| 桌面应用打包 | `npm run tauri build` | release 编译 + 打 `.app` Bundle，产物在 `src-tauri/target/release/bundle/macos/`，已包含 `libpdfium.dylib`。 |
| Rust 单测 | `cd src-tauri && cargo test` | 跑所有 Rust 单元测试（`error` / `pdf_parser` / `renamer`）。 |
| 单模块测试 | `cd src-tauri && cargo test renamer::tests` | 按模块过滤。 |
| 快速类型检查 | `cd src-tauri && cargo check` | 不产出二进制，验证类型与编译。 |
| 静态分析 | `cd src-tauri && cargo clippy -- -D warnings` | lint，零告警。 |
| 格式化检查 | `cd src-tauri && cargo fmt --check` | CI 风格。 |

**端到端 PDF 解析测试（可选）**：把一份真实数电票 PDF 命名为 `src-tauri/tests/fixtures/sample.pdf`，`cargo test` 会自动调用 PDFium 验证抽号流程；fixture 不存在时该用例自动跳过。

## 打包后行为

`tauri.conf.json` 的 `bundle.resources` 已包含 `lib/libpdfium.dylib`，打出的 `.app` 自带 PDFium。运行时 `pdf_parser::locate_lib_dir` 按以下顺序定位库：

1. 开发模式：`src-tauri/lib/libpdfium.dylib`
2. 打包后：`<.app>/Contents/Resources/lib/libpdfium.dylib`（或可执行文件同级 `lib/`）
3. 都找不到时退回系统 PDFium（一般不会命中）

## 项目结构

```
.
├── src/                   # 前端（vanilla JS + Vite）
│   ├── index.html
│   ├── main.js            # 表单校验、IPC 调用、日志渲染
│   └── style.css          # 跟随系统的浅/深色样式
├── src-tauri/             # Rust 后端
│   ├── src/
│   │   ├── lib.rs         # Tauri builder & 插件注册
│   │   ├── commands.rs    # rename_pdfs 命令 + Channel 日志
│   │   ├── pdf_parser.rs  # PDFium 文本抽取 + 发票号正则
│   │   ├── renamer.rs     # 计划构建 / 执行 / 冲突处理
│   │   └── error.rs       # 统一错误类型
│   ├── lib/               # PDFium 动态库（由脚本下载）
│   ├── icons/             # 应用图标
│   └── tauri.conf.json
├── scripts/
│   └── fetch-pdfium.sh
└── vite.config.js
```

## 工作流程概览

```
 UI 选目录 + 填两个字段
        │
        ▼
 invoke("rename_pdfs", { sourceDir, userName, trackingNumber, onLog })
        │
        ▼  spawn_blocking
 build_plan ─▶ 枚举顶层 PDF ─▶ pdfium 抽取文本 ─▶ 正则匹配发票号
        │
        ▼
 execute_plan ─▶ 创建输出目录 ─▶ 逐个 copy ─▶ 同名加序号
        │
        ▼  Channel<LogEntry>
 前端实时追加日志，结束后返回 summary
```

## 设计思路

### 技术选型为什么是这一套

- **Tauri 2 而不是 Electron**：原生窗口 + Rust 后端，体积小、启动快，PDF 解析这种 CPU 密集工作交给 Rust 也更合适。
- **PDFium（`pdfium-render` crate）而不是 `lopdf` / `pdf-extract` 等纯 Rust 实现**：电子发票 PDF 用了大量自定义 CMap 和子集化中文字体，纯 Rust 实现的文本抽取经常乱码或丢字；PDFium 是 Chrome 同款引擎，对中文最稳。代价是要带一个动态库，因此用脚本拉 + bundle 打包。
- **Vanilla JS + Vite 而不是 React/Vue**：UI 只有一个表单 + 日志面板，没必要上框架。Vite 的存在只是为了把 ESM 形式的 `@tauri-apps/api` 打成浏览器能直接 import 的产物。
- **Tauri 2 `ipc::Channel<T>` 而不是 `emit` / `listen` 事件**：日志是单向、强时序、生命周期跟一次 invoke 绑定的流，Channel 的语义更贴近，前端也不用手动取消订阅。

### 模块边界

```
commands.rs      ← 只做参数校验 + 把 Channel 包成 Logger + spawn_blocking
   │
   ├─ pdf_parser.rs    ← PDFium 调用 + 正则匹配，纯函数 find_invoice_number_in_text 单独可测
   ├─ renamer.rs       ← build_plan / execute_plan，与 PDF 解析解耦（通过传入 extract_fn 闭包）
   └─ error.rs         ← AppError 统一前后端错误形态，实现 Serialize 直通 IPC
```

关键解耦点：

- `build_plan` 把"如何从 PDF 抽号"作为参数注入，测试时传 mock 闭包，无需准备真实 PDF。
- `Logger` trait + `ChannelLogger` 的薄封装：`renamer` 不依赖 Tauri，测试时用 `VecLogger` 收集日志做断言。
- PDFium 实例用 `thread_local!` + `RefCell` 持有，避开全局 `Mutex` 的开销；执行在 `spawn_blocking` 线程池里。

### 抽号策略：双层正则 + UNKNOWN 兜底

```
优先：(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})
兜底：(?:^|[^\d])(\d{20})(?:[^\d]|$)
```

- 优先匹配带"发票号码"等字段名的，避免被同页其它 20 位数字误伤。
- 兜底匹配独立 20 位串，应对部分版式只用纯数字的情形。
- 都失败时不阻塞流程：用 `UNKNOWN-{用户名}-{Tracking}.pdf` 命名继续复制，让用户能事后人工识别。OCR 兜底刻意没做（扫描件占比低，收益不抵复杂度）。

### 复制 + 加序号，而不是移动 + 报错

- `std::fs::copy` 而非 `rename`：源文件始终保留，整个流程**幂等且可重跑**，跑错了直接删除子目录重试。
- 冲突时按 `{stem}-1.pdf`、`{stem}-2.pdf` … 找第一个空位（上限 999）：发票号同号但内容不同（PDF 被重复发了不同版本）时不会互相覆盖。

### 单文件失败不中止整批

| 情况 | 处理 | 计入 |
|---|---|---|
| PDF 加密 / 损坏 | error 日志，跳过此文件 | failed |
| 抽不到号 | warn 日志，用 UNKNOWN，照常复制 | success |
| 目标已存在 | 自动加序号，info 日志注明 | success |
| 序号 1–999 都被占 | error 日志，跳过 | failed |
| `fs::copy` 失败 | error 日志，跳过 | failed |

只有"建不出输出目录"这一种顶层失败才会中止整批。

### 显式不做（第一版）

- 不递归子目录（避免把已有的 `{Tracking}/` 重复处理）
- 不做"取消"按钮（典型批量秒级完成）
- 不做 OCR
- 不持久化日志、不记忆上次输入

详细的取舍理由见 [设计文档 §9](docs/superpowers/specs/2026-05-13-pdf-rename-design.md)。

## 设计与实施文档

| 文档 | 内容 |
|---|---|
| [`docs/superpowers/specs/2026-05-13-pdf-rename-design.md`](docs/superpowers/specs/2026-05-13-pdf-rename-design.md) | 完整设计：背景、需求、技术选型、模块 API、数据流、测试策略、取舍 |
| [`docs/superpowers/plans/2026-05-13-pdf-rename.md`](docs/superpowers/plans/2026-05-13-pdf-rename.md) | 实施计划：12 个 Task 的逐步落地步骤、每步对应的 commit message |

## 已知限制

- 仅扫描源目录的**顶层** PDF，不递归子目录
- 加密 / 损坏的 PDF 会作为单文件失败记入日志，不影响其他文件
- 同一发票号在同一 Tracking 下最多容纳 1000 份副本（基础名 + `-1`…`-999`）
- 暂未提供 Windows / Linux 构建脚本（PDFium 库需自行准备）

## License

私有项目，未指定开源协议。
