# ESI 发票重命名工具

一个 macOS + Windows 桌面工具，用于批量整理 PDF 电子发票：自动识别 PDF 中的 20 位发票号码，按 `{发票号}-{用户名}-{Tracking}.pdf` 格式重命名，并复制到以 Tracking Number 命名的子目录中。

底层用 [Tauri 2](https://tauri.app/) + Rust + [PDFium](https://github.com/bblanchon/pdfium-binaries)，前端是零依赖的原生 JS + Vite。

## 下载安装

直接从 [GitHub Releases](https://github.com/xpwsgg/invoice_rename/releases/latest) 拿对应平台的产物：

| 平台 | 文件 | 用法 |
|---|---|---|
| macOS（Intel / Apple Silicon） | `ESI-Invoice-Rename.dmg` | 双击挂载，把 `ESI Invoice Rename.app` 拖到 `Applications` |
| Windows（x64） | `ESI-Invoice-Rename.exe` | **单文件便携版**，无需安装。`pdfium.dll` 已嵌入 exe，首次启动会自动展开到 `%TEMP%\esi_invoice_rename\` |

打开后一次只允许运行一个实例，重复双击图标会把已有窗口提到前台。

## 功能

### 核心功能
- **即时扫描**：选择文件夹后立即显示所有 PDF 文件列表和总金额（v0.4.2+）
- **智能识别**：自动识别发票号码和价税合计金额
  - 发票号码：优先匹配带字段名的 `发票号码: / Invoice Number: / Invoice No`，退化到独立的 20 位数字片段
  - 价税合计：优先匹配中文大写金额锚点，回退为文档中最大的 ¥ 金额
  - 都匹配不到时使用 `UNKNOWN` 占位，文件依然会被复制
- **文件过滤**：每个文件都有"移除"按钮，可在重命名前排除不需要的文件（v0.4.2+）
  - 移除后总金额和统计数据实时更新
  - 重命名时只处理保留的文件
- **批量重命名**：输出路径 `{源目录}/{TrackingNumber}/{发票号}-{用户名}-{TrackingNumber}.pdf`
- **智能冲突处理**：同名文件冲突时自动加序号 `-1`、`-2` … 直到 `-999`
- **实时反馈**：
  - 汇总条显示价税合计总和、总张数、成功/失败/未识别金额统计
  - 进度条实时显示处理进度
  - 结果表格逐行追加处理结果
- **安全操作**：源文件只读取，不移动、不修改、不删除（用 `copy` 而非 `rename`）
- **智能记忆**：用户名自动记忆并回填；源目录和 Tracking Number 不记忆

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

## 环境要求（开发）

- macOS 或 Windows
- [Node.js](https://nodejs.org/) ≥ 18
- [Rust](https://www.rust-lang.org/) stable toolchain
- [Tauri 2 系统依赖](https://tauri.app/start/prerequisites/)

## 首次准备

```bash
# 1. 安装前端依赖
npm install

# 2. 下载 PDFium 动态库到 src-tauri/lib/
#    macOS:
./scripts/fetch-pdfium.sh
#    Windows (PowerShell):
./scripts/fetch-pdfium.ps1
```

两个脚本都从 [pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) 拉取最新预编译库，按当前架构挑对应包：

- macOS：`pdfium-mac-arm64.tgz` 或 `pdfium-mac-x64.tgz` → `src-tauri/lib/libpdfium.dylib`
- Windows：`pdfium-win-x64.tgz` 或 `pdfium-win-arm64.tgz` → `src-tauri/lib/pdfium.dll`

> Windows 上 `pdfium.dll` 不仅 dev 时需要，**`cargo build` 也会通过 `include_bytes!` 把它嵌入 .exe**，所以编译前必须先跑 fetch 脚本。

## 启动命令速查

| 场景 | 命令 | 说明 |
|---|---|---|
| 桌面应用开发（推荐） | `npm run tauri dev` | 由 Tauri CLI 拉起 Vite dev server（`localhost:1420`），再用 debug 版二进制开窗。修改前端热更新；修改 Rust 触发重编译。首次启动会编译大量 crate，约 1–2 分钟。 |
| 仅前端 | `npm run dev` | 纯 Vite，无 Tauri IPC，调样式/排版用，`invoke()` 会报错。 |
| 前端构建 | `npm run build` | 把 `src/` 打成静态资源到 `dist/`（供 Tauri 打包消费）。 |
| 前端构建产物预览 | `npm run preview` | 用 Vite 起本地 server 浏览 `dist/`。 |
| 桌面应用打包 | `npm run tauri build` | release 编译 + 打包。macOS 产出 `.app` 与 `.dmg`（`src-tauri/target/release/bundle/`）；Windows 产出 `pdf_rename.exe`（`src-tauri/target/release/`，含嵌入的 PDFium）。 |
| 重新生成图标 | `npm run tauri icon path/to/source.png` | 从 1024×1024 PNG 源图生成全部平台的图标到 `src-tauri/icons/`。 |
| Rust 单测 | `cd src-tauri && cargo test` | 跑所有 Rust 单元测试（`error` / `pdf_parser` / `renamer`），共 40 条。 |
| 单模块测试 | `cd src-tauri && cargo test renamer::tests` | 按模块过滤。 |
| 快速类型检查 | `cd src-tauri && cargo check` | 不产出二进制，验证类型与编译。 |
| 静态分析 | `cd src-tauri && cargo clippy -- -D warnings` | lint，零告警。 |
| 格式化检查 | `cd src-tauri && cargo fmt --check` | CI 风格。 |

**端到端 PDF 解析测试（可选）**：把一份真实数电票 PDF 命名为 `src-tauri/tests/fixtures/sample.pdf`，`cargo test` 会自动调用 PDFium 验证抽号流程；fixture 不存在时该用例自动跳过。

## 打包后行为

`tauri.conf.json` 的 `bundle.resources` 用 `lib/*` 通配，每个平台都把当前架构的 PDFium 装进 bundle。运行时 `pdf_parser::locate_lib_dir` 按以下顺序定位库：

**Windows（特殊路径）**：

1. 把编译期 `include_bytes!` 嵌入的 `pdfium.dll` 写出到 `%TEMP%\esi_invoice_rename\pdfium.dll`（按文件大小判断是否需要重写，升级 exe 时会自动覆盖）
2. 加载这个 dll

**macOS / 通用 fallback**：

1. 开发模式：`src-tauri/lib/libpdfium.dylib`
2. exe 同级目录（Windows portable 备选路径）
3. exe 同级 `lib/` 子目录
4. macOS bundle：`<.app>/Contents/Resources/lib/libpdfium.dylib`
5. 都找不到时退回系统 PDFium（一般不会命中）

## 项目结构

```
.
├── README.md
├── .github/
│   └── workflows/
│       └── build.yml             # macOS .dmg + Windows .exe 矩阵构建
├── src/                          # 前端（vanilla JS + Vite）
│   ├── index.html                # ESI 发票重命名工具 UI
│   ├── main.js                   # 表单校验、IPC 调用、日志渲染、用户名持久化
│   └── style.css                 # 跟随系统的浅/深色样式
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── lib.rs                # tauri::Builder，注册 single-instance + dialog 插件
│   │   ├── commands.rs           # rename_pdfs 命令 + Channel 日志
│   │   ├── pdf_parser.rs         # PDFium 抽取 + 跨平台 lib 定位 + Windows 嵌入 dll
│   │   ├── renamer.rs            # 计划构建 / 执行 / 冲突处理
│   │   └── error.rs              # 统一错误类型
│   ├── lib/                      # PDFium 动态库（脚本下载，gitignore）
│   ├── icons/                    # 全平台图标（macOS .icns / Windows .ico / PNG / iOS / Android）
│   ├── capabilities/
│   │   └── default.json          # Tauri v2 权限：core:default + dialog:default
│   ├── tauri.conf.json
│   └── Cargo.toml
├── scripts/
│   ├── fetch-pdfium.sh           # macOS 拉 libpdfium.dylib
│   └── fetch-pdfium.ps1          # Windows 拉 pdfium.dll
├── docs/superpowers/             # 原始设计文档与实施计划
└── vite.config.js
```

## 工作流程概览

```
 UI 选目录（立即扫描显示文件列表和总金额）
        │
        ▼  invoke("scan_pdfs", { sourceDir })
 扫描所有 PDF ─▶ pdfium 抽取文本 ─▶ 正则匹配发票号和金额
        │
        ▼  前端显示
 文件列表（每行有"移除"按钮）+ 价税合计总和 + 统计
        │
        ▼  用户操作
 移除不需要的文件 → 总金额实时更新
        │
        ▼  填写用户名和 TN，点击"开始重命名"
 invoke("rename_pdfs", { sourceDir, userName, trackingNumber, fileNames, onRow })
        │
        ▼  spawn_blocking
 build_plan_for_files ─▶ 只处理保留的文件 ─▶ 构建重命名计划
        │
        ▼
 execute_plan ─▶ 创建输出目录 ─▶ 逐个 copy ─▶ 同名加序号
        │
        ▼  Channel<InvoiceRow>
 前端实时追加结果，结束后返回 summary
```

## 设计思路

### 技术选型为什么是这一套

- **Tauri 2 而不是 Electron**：原生窗口 + Rust 后端，体积小、启动快，PDF 解析这种 CPU 密集工作交给 Rust 也更合适。
- **PDFium（`pdfium-render` crate）而不是 `lopdf` / `pdf-extract` 等纯 Rust 实现**：电子发票 PDF 用了大量自定义 CMap 和子集化中文字体，纯 Rust 实现的文本抽取经常乱码或丢字；PDFium 是 Chrome 同款引擎，对中文最稳。代价是要带一个动态库，因此用脚本拉 + bundle 打包。
- **Vanilla JS + Vite 而不是 React/Vue**：UI 只有一个表单 + 日志面板，没必要上框架。Vite 的存在只是为了把 ESM 形式的 `@tauri-apps/api` 打成浏览器能直接 import 的产物。
- **Tauri 2 `ipc::Channel<T>` 而不是 `emit` / `listen` 事件**：日志是单向、强时序、生命周期跟一次 invoke 绑定的流，Channel 的语义更贴近，前端也不用手动取消订阅。

### 模块边界

```
commands.rs      ← 参数校验 + 把 Channel 包成 ProgressSink + spawn_blocking
   │
   ├─ scan_pdfs           ← 扫描文件夹，返回文件列表和金额汇总（v0.4.2+）
   ├─ rename_pdfs         ← 接收文件名列表，只处理指定文件（v0.4.2+）
   ├─ pdf_parser.rs       ← PDFium 调用 + 正则匹配发票号和金额
   ├─ renamer.rs          ← build_plan / build_plan_for_files / execute_plan
   └─ error.rs            ← AppError 统一前后端错误形态
```

关键解耦点：

- **即时扫描与重命名分离**（v0.4.2+）：`scan_pdfs` 只扫描不重命名，`rename_pdfs` 接收文件名列表按需处理
- `build_plan` / `build_plan_for_files`：前者扫描整个文件夹，后者只处理指定文件列表
- `build_plan_for_files` 把"如何从 PDF 抽号"作为参数注入，测试时传 mock 闭包
- `ProgressSink` trait + `ChannelSink`：`renamer` 不依赖 Tauri，测试时用 `VecSink` 收集进度
- PDFium 实例用 `thread_local!` + `RefCell` 持有，避开全局 `Mutex` 开销

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

### 单实例锁

通过 `tauri-plugin-single-instance` 注册回调：第二次启动 .app / .exe 时不会再开新窗口，而是把已有窗口 `show + unminimize + set_focus` 提到前台。回调实现见 `src-tauri/src/lib.rs`。

### Windows 单文件便携版：编译期内嵌 PDFium

Windows 产物是孤零零一个 `.exe`，没有外挂 dll。实现方式：

```rust
#[cfg(windows)]
const EMBEDDED_PDFIUM_DLL: &[u8] = include_bytes!("../lib/pdfium.dll");
```

- 编译时 `include_bytes!` 把 7.2 MB 的 dll 直接编入 exe，最终 exe 约 11 MB
- 启动时 `ensure_embedded_pdfium()` 把 dll 写到 `%TEMP%\esi_invoice_rename\pdfium.dll`
- 用文件大小做指纹比较：已存在且大小一致就复用，否则重写——升级 exe 时自动覆盖
- macOS 通过 `#[cfg(windows)]` 隔离，不会拖累 .app 体积，也不需要 `pdfium.dll` 存在

### 用户名持久化（其它字段刻意不记）

- 用户名用 `localStorage["pdfRename.userName"]` 存，**只在通过校验且真正点击"开始重命名"时写入**——避免试错的临时输入污染记忆
- 启动时模块加载阶段读出回填到输入框
- 源目录和 Tracking Number 没记：源目录每次可能不同，Tracking Number 几乎一票一号，记反而碍事

### Tauri v2 capability

`src-tauri/capabilities/default.json` 显式授权 `core:default` 与 `dialog:default`。Tauri 2 是强权限模型——少了这个文件，`@tauri-apps/plugin-dialog` 的 `open()` 会被静默拒绝（按"选择目录"按钮没反应是典型症状），自定义 `#[tauri::command]` 也调不通。

### 显式不做

- 不递归子目录（避免把已有的 `{Tracking}/` 重复处理）
- 不做"取消"按钮（典型批量秒级完成）
- 不做 OCR
- 不持久化日志
- 不记忆源目录和 Tracking Number（用户名是例外，见上）

详细的取舍理由见 [设计文档 §9](docs/superpowers/specs/2026-05-13-pdf-rename-design.md)。

## CI / Release 流程

`.github/workflows/build.yml` 的矩阵：

| Runner | PDFium 拉取 | 产出 | Release Asset |
|---|---|---|---|
| `macos-latest` | `scripts/fetch-pdfium.sh` | `bundle/dmg/*.dmg` → 重命名为 `ESI-Invoice-Rename.dmg` | `ESI-Invoice-Rename.dmg` |
| `windows-latest` | `scripts/fetch-pdfium.ps1` | `target/release/pdf_rename.exe` → 重命名为 `ESI-Invoice-Rename.exe` | `ESI-Invoice-Rename.exe` |

触发方式：

- **打 tag**：`git tag v0.1.1 && git push origin v0.1.1` → 矩阵构建 + 自动建 GitHub Release（带 contents:write 权限的 `softprops/action-gh-release`）
- **手动**：GitHub 仓库 → Actions → build → Run workflow（不会创建 release，只产 artifacts）

## 设计与实施文档

| 文档 | 内容 |
|---|---|
| [`docs/superpowers/specs/2026-05-13-pdf-rename-design.md`](docs/superpowers/specs/2026-05-13-pdf-rename-design.md) | 完整设计：背景、需求、技术选型、模块 API、数据流、测试策略、取舍 |
| [`docs/superpowers/plans/2026-05-13-pdf-rename.md`](docs/superpowers/plans/2026-05-13-pdf-rename.md) | 实施计划：12 个 Task 的逐步落地步骤、每步对应的 commit message |

## 已知限制

- 仅扫描源目录的**顶层** PDF，不递归子目录
- 加密 / 损坏的 PDF 会作为单文件失败记入日志，不影响其他文件
- 同一发票号在同一 Tracking 下最多容纳 1000 份副本（基础名 + `-1`…`-999`）
- 暂不提供 Linux 构建（PDFium Linux 版未集成、`cfg(linux)` 嵌入逻辑未写）
- macOS .dmg 与 Windows .exe 都未做代码签名 / 公证：macOS 首次打开会弹"无法验证开发者"，需右键 → 打开放行；Windows SmartScreen 也可能拦一次

## License

私有项目，未指定开源协议。
