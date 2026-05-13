# PDF 发票批量重命名工具 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `docs/superpowers/specs/2026-05-13-pdf-rename-design.md` 描述的设计落地为一个能在 macOS 上运行的 Tauri 2 桌面应用。

**Architecture:** Tauri 2 应用，前端用 Vanilla HTML/CSS/JS（无打包器），后端用 Rust。Rust 端通过 `pdfium-render` crate（绑定预编译 PDFium 动态库）提取数电票 PDF 中的 20 位发票号码，按 `{发票号码}-{用户名}-{trackingNumber}.pdf` 复制到源文件夹下以 trackingNumber 命名的子目录中。日志通过 Tauri 2 `ipc::Channel` 实时推送到前端。

**Tech Stack:** Tauri 2.x · Rust（pdfium-render, regex, thiserror, serde, chrono, tempfile）· Vanilla JS · `@tauri-apps/plugin-dialog`

---

## 文件清单

| 文件 | 职责 |
|---|---|
| `package.json` | 承载 tauri-cli 与 dialog 插件的 npm 脚本 |
| `src-tauri/Cargo.toml` | Rust 依赖与 crate 元信息 |
| `src-tauri/tauri.conf.json` | 应用配置（窗口、bundle、resources） |
| `src-tauri/build.rs` | Tauri 构建脚本 |
| `src-tauri/src/main.rs` | 二进制入口 |
| `src-tauri/src/lib.rs` | `tauri::Builder` 配置、命令注册 |
| `src-tauri/src/error.rs` | `AppError`/`ParseError`/`IoError` |
| `src-tauri/src/pdf_parser.rs` | PDF 文本提取 + 发票号码正则 |
| `src-tauri/src/renamer.rs` | 文件复制、命名、冲突处理、`Logger` trait |
| `src-tauri/src/commands.rs` | `#[tauri::command] rename_pdfs` 与 `ChannelLogger` |
| `src-tauri/lib/libpdfium.dylib` | 预编译 PDFium 动态库（不入 git） |
| `src-tauri/icons/*` | 应用图标（占位） |
| `src/index.html` | UI 结构 |
| `src/style.css` | 样式（macOS 风格 + 深色适配） |
| `src/main.js` | 前端逻辑（选择目录、校验、调用命令、日志渲染） |

---

## Task 1：创建项目根级配置文件

**Files:**
- Create: `package.json`
- Create: `src-tauri/Cargo.toml`

- [ ] **Step 1: 写 `package.json`**

```json
{
  "name": "pdf_rename",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  }
}
```

- [ ] **Step 2: 写 `src-tauri/Cargo.toml`**

```toml
[package]
name = "pdf_rename"
version = "0.1.0"
edition = "2021"

[lib]
name = "pdf_rename_lib"
crate-type = ["lib", "cdylib", "staticlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
regex = "1"
once_cell = "1"
chrono = { version = "0.4", default-features = false, features = ["clock"] }
pdfium-render = { version = "0.8", features = ["thread_safe"] }

[dev-dependencies]
tempfile = "3"

[profile.release]
codegen-units = 1
lto = true
opt-level = "s"
panic = "abort"
strip = true
```

- [ ] **Step 3: 运行 `npm install` 拉取前端依赖**

Run: `cd /Users/xiao/Documents/code/pdf_rename && npm install`
Expected: 在 `node_modules/` 中能看到 `@tauri-apps/cli` 和 `@tauri-apps/plugin-dialog`。

- [ ] **Step 4: Commit**

```bash
git add package.json src-tauri/Cargo.toml
git commit -m "chore: add tauri/cargo project manifests"
```

---

## Task 2：创建 Tauri 后端入口骨架

**Files:**
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/icons/icon.png`（占位图，先 1x1 透明 PNG）

- [ ] **Step 1: 写 `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 2: 写 `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "PDF Rename",
  "version": "0.1.0",
  "identifier": "com.felix.pdfrename",
  "build": {
    "frontendDist": "../src"
  },
  "app": {
    "windows": [
      {
        "title": "PDF 发票批量重命名",
        "width": 720,
        "height": 640,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "app",
    "icon": [
      "icons/icon.png"
    ],
    "resources": [
      "lib/libpdfium.dylib"
    ],
    "category": "Utility"
  }
}
```

- [ ] **Step 3: 写 `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pdf_rename_lib::run();
}
```

- [ ] **Step 4: 写最小化的 `src-tauri/src/lib.rs`（先只起空窗口）**

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 准备占位图标**

Run: `mkdir -p src-tauri/icons && printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x06\x00\x00\x00\x1f\x15\xc4\x89\x00\x00\x00\rIDATx\x9cc\xf8\x0f\x00\x00\x01\x01\x00\x05\xfe\x02\xfe\xa1Yz\xc6\x00\x00\x00\x00IEND\xaeB\x60\x82' > src-tauri/icons/icon.png`

- [ ] **Step 6: 创建空 `src/index.html` 占位（详细 UI 在 Task 12 完成）**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <title>PDF 发票批量重命名</title>
  </head>
  <body>
    <h1>启动中…</h1>
  </body>
</html>
```

- [ ] **Step 7: 验证 cargo check 通过**

Run: `cd src-tauri && cargo check`
Expected: 编译成功，可能首次会下载大量 crate（耐心等待）。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/ src/index.html
git commit -m "feat: scaffold tauri 2 application shell"
```

---

## Task 3：下载并集成 PDFium 动态库

**Files:**
- Create: `src-tauri/lib/libpdfium.dylib`（由脚本下载，不入 git）
- Modify: `src-tauri/src/lib.rs`（暂时不动，仅用于运行验证）
- Create: `scripts/fetch-pdfium.sh`（一次性脚本，供未来重新拉取）

- [ ] **Step 1: 写 `scripts/fetch-pdfium.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

ARCH=$(uname -m)
case "$ARCH" in
  arm64)   PKG="pdfium-mac-arm64.tgz" ;;
  x86_64)  PKG="pdfium-mac-x64.tgz"   ;;
  *) echo "Unsupported arch: $ARCH" >&2; exit 1 ;;
esac

OUT_DIR="src-tauri/lib"
mkdir -p "$OUT_DIR"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/$PKG"
echo "Downloading $URL"
curl -fL "$URL" -o "$TMP/pdfium.tgz"
tar -xzf "$TMP/pdfium.tgz" -C "$TMP"

cp "$TMP/lib/libpdfium.dylib" "$OUT_DIR/libpdfium.dylib"
echo "PDFium installed at $OUT_DIR/libpdfium.dylib"
```

- [ ] **Step 2: 给脚本可执行权限并运行**

Run:
```
chmod +x scripts/fetch-pdfium.sh
./scripts/fetch-pdfium.sh
```
Expected: 控制台输出 `PDFium installed at src-tauri/lib/libpdfium.dylib`，文件存在。

- [ ] **Step 3: 验证 dylib 存在**

Run: `ls -lh src-tauri/lib/libpdfium.dylib`
Expected: 文件大小约 8-12 MB。

- [ ] **Step 4: Commit 脚本**

```bash
git add scripts/fetch-pdfium.sh
git commit -m "build: add pdfium dylib fetch script"
```

---

## Task 4：实现 `error.rs`

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/lib.rs`（声明 `mod error;`）

- [ ] **Step 1: 写 `src-tauri/src/error.rs`**

```rust
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(String),
    Pdf(String),
    InvalidInput(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(msg) => write!(f, "IO 错误：{}", msg),
            AppError::Pdf(msg) => write!(f, "PDF 解析错误：{}", msg),
            AppError::InvalidInput(msg) => write!(f, "无效输入：{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_io_error() {
        let err = AppError::Io("permission denied".into());
        assert_eq!(err.to_string(), "IO 错误：permission denied");
    }

    #[test]
    fn display_pdf_error() {
        let err = AppError::Pdf("encrypted".into());
        assert_eq!(err.to_string(), "PDF 解析错误：encrypted");
    }

    #[test]
    fn display_invalid_input_error() {
        let err = AppError::InvalidInput("user_name empty".into());
        assert_eq!(err.to_string(), "无效输入：user_name empty");
    }

    #[test]
    fn serialize_to_string() {
        let err = AppError::Io("disk full".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"IO 错误：disk full\"");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }
}
```

- [ ] **Step 2: 在 `lib.rs` 顶部声明模块**

修改 `src-tauri/src/lib.rs`，在 `pub fn run()` 之前添加：

```rust
mod error;
```

- [ ] **Step 3: 运行测试**

Run: `cd src-tauri && cargo test error::tests -- --nocapture`
Expected: 5 个测试全部通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(error): add AppError with Serialize"
```

---

## Task 5：`pdf_parser.rs` — 正则提取（纯函数 TDD）

**Files:**
- Create: `src-tauri/src/pdf_parser.rs`
- Modify: `src-tauri/src/lib.rs`（声明 `mod pdf_parser;`）

- [ ] **Step 1: 写测试用例 + 函数签名（先让它编译失败）**

`src-tauri/src/pdf_parser.rs`：

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static RE_LABELED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})").unwrap()
});

static RE_BARE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[^\d])(\d{20})(?:[^\d]|$)").unwrap()
});

/// 在已抽取的 PDF 全文中寻找 20 位发票号码。
/// 优先匹配带"发票号码"等字段名的，找不到时回退到"独立 20 位数字"。
pub fn find_invoice_number_in_text(text: &str) -> Option<String> {
    if let Some(caps) = RE_LABELED.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    if let Some(caps) = RE_BARE.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labeled_chinese() {
        let text = "发票代码: 011002000111\n发票号码: 26322000000893295511\n开票日期";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_chinese_with_colon_variant() {
        let text = "发票号码：26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_english_invoice_number() {
        let text = "Invoice Number: 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_english_invoice_no() {
        let text = "Invoice No 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn bare_fallback() {
        let text = "杂项文本 26322000000893295511 杂项";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn rejects_21_digit_run() {
        // 22 位数字串里没有任何"独立"的 20 位段
        let text = "1234567890123456789012";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn returns_none_when_no_match() {
        let text = "这只是一段普通文本，没有任何 20 位数字";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn prefers_labeled_over_bare() {
        // 文本里既有裸的 20 位数字，也有带"发票号码"标签的；应取后者
        let text = "11111111111111111111\n发票号码: 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }
}
```

- [ ] **Step 2: 在 `lib.rs` 声明模块**

修改 `src-tauri/src/lib.rs`，在 `mod error;` 后添加：

```rust
mod pdf_parser;
```

- [ ] **Step 3: 运行测试**

Run: `cd src-tauri && cargo test pdf_parser -- --nocapture`
Expected: 8 个测试全部通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/pdf_parser.rs src-tauri/src/lib.rs
git commit -m "feat(pdf_parser): extract 20-digit invoice number from text"
```

---

## Task 6：`pdf_parser.rs` — 接入 PDFium

**Files:**
- Modify: `src-tauri/src/pdf_parser.rs`

- [ ] **Step 1: 在 `pdf_parser.rs` 顶部增加 PDFium 单例与公开函数**

在文件顶部 `use` 之后插入：

```rust
use crate::error::AppError;
use once_cell::sync::OnceCell;
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static PDFIUM: OnceCell<Mutex<Pdfium>> = OnceCell::new();

fn pdfium() -> Result<std::sync::MutexGuard<'static, Pdfium>, AppError> {
    let cell = PDFIUM.get_or_try_init(|| -> Result<Mutex<Pdfium>, AppError> {
        // 1. 优先使用与可执行文件同目录的 lib/libpdfium.dylib（dev 模式下指向项目 src-tauri/lib）
        // 2. 找不到时退回系统库
        let local_dir = locate_lib_dir();
        let bindings = match local_dir
            .as_ref()
            .and_then(|p| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(p)).ok())
        {
            Some(b) => b,
            None => Pdfium::bind_to_system_library()
                .map_err(|e| AppError::Pdf(format!("加载 PDFium 库失败：{e}")))?,
        };
        Ok(Mutex::new(Pdfium::new(bindings)))
    })?;
    cell.lock()
        .map_err(|_| AppError::Pdf("PDFium 互斥锁被毒化".into()))
}

fn locate_lib_dir() -> Option<PathBuf> {
    // dev 模式：相对项目根的 src-tauri/lib
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");
    if dev.join("libpdfium.dylib").exists() {
        return Some(dev);
    }
    // 打包后：.app/Contents/Resources/lib（或可执行文件同级）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for cand in [parent.join("lib"), parent.join("../Resources/lib")] {
                if cand.join("libpdfium.dylib").exists() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// 打开 PDF 并尝试提取发票号码。
/// - Ok(Some) : 成功匹配到 20 位号码
/// - Ok(None) : PDF 能打开但无法匹配
/// - Err     : PDF 打不开 / 加密 / 损坏
pub fn extract_invoice_number(pdf_path: &Path) -> Result<Option<String>, AppError> {
    let pdfium = pdfium()?;
    let document = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| AppError::Pdf(format!("打开 PDF 失败：{e}")))?;

    let mut buf = String::new();
    for page in document.pages().iter() {
        let text = page
            .text()
            .map_err(|e| AppError::Pdf(format!("提取文本失败：{e}")))?;
        buf.push_str(&text.all());
        buf.push('\n');
    }
    Ok(find_invoice_number_in_text(&buf))
}
```

- [ ] **Step 2: 增加可选的集成测试（依赖样本文件，无样本时跳过）**

在 `pdf_parser.rs` 的 `#[cfg(test)] mod tests` 末尾追加：

```rust
    #[test]
    fn extract_from_sample_pdf_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.pdf");
        if !path.exists() {
            eprintln!("跳过：未找到 {}", path.display());
            return;
        }
        let result = extract_invoice_number(&path).expect("PDF 解析不应失败");
        assert!(result.is_some(), "应该能从样本 PDF 提取到发票号码");
        let num = result.unwrap();
        assert_eq!(num.len(), 20);
        assert!(num.chars().all(|c| c.is_ascii_digit()));
    }
```

- [ ] **Step 3: 运行测试（PDFium 单例首次初始化可能需要 1-2 秒）**

Run: `cd src-tauri && cargo test pdf_parser -- --nocapture`
Expected: 原有 8 个测试通过；如未放置 fixture，最后一个测试输出 "跳过：…"。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/pdf_parser.rs
git commit -m "feat(pdf_parser): integrate pdfium for text extraction"
```

---

## Task 7：`renamer.rs` — `build_plan`（TDD）

**Files:**
- Create: `src-tauri/src/renamer.rs`
- Modify: `src-tauri/src/lib.rs`（声明 `mod renamer;`）

- [ ] **Step 1: 写 `src-tauri/src/renamer.rs` 第一版（类型 + build_plan + 测试）**

```rust
use crate::error::AppError;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlan {
    pub source: PathBuf,
    pub target: PathBuf,
    pub invoice_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RenameSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: String,
    pub level: String,
    pub message: String,
}

pub trait Logger {
    fn log(&mut self, entry: LogEntry);
}

/// 文件名中不允许出现的字符（跨平台保守集合）
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

pub fn validate_name(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::InvalidInput(format!("{field} 不能为空")));
    }
    if value.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(AppError::InvalidInput(format!(
            "{field} 不能包含特殊字符 / \\ : * ? \" < > |"
        )));
    }
    Ok(())
}

/// 扫描源目录顶层 PDF 文件，结合提取函数构造重命名计划。
pub fn build_plan<F>(
    source_dir: &Path,
    user_name: &str,
    tracking_number: &str,
    extract_fn: F,
) -> Result<Vec<RenamePlan>, AppError>
where
    F: Fn(&Path) -> Result<Option<String>, AppError>,
{
    validate_name("用户名", user_name)?;
    validate_name("Tracking Number", tracking_number)?;

    if !source_dir.is_dir() {
        return Err(AppError::Io(format!(
            "源文件夹不存在或不是目录：{}",
            source_dir.display()
        )));
    }

    let output_dir = source_dir.join(tracking_number);

    let mut plans = Vec::new();
    for entry in std::fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !is_pdf(&path) {
            continue;
        }

        let invoice = extract_fn(&path).unwrap_or(None);
        let prefix = invoice.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        let filename = format!("{prefix}-{user_name}-{tracking_number}.pdf");
        let target = output_dir.join(filename);

        plans.push(RenamePlan {
            source: path,
            target,
            invoice_number: invoice,
        });
    }

    // 按源文件名排序，使日志顺序稳定
    plans.sort_by(|a, b| a.source.file_name().cmp(&b.source.file_name()));
    Ok(plans)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        File::create(&p).expect("touch file");
        p
    }

    fn ok_extract(_: &Path) -> Result<Option<String>, AppError> {
        Ok(Some("26322000000893295511".to_string()))
    }

    fn none_extract(_: &Path) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    #[test]
    fn rejects_empty_user_name() {
        let dir = TempDir::new().unwrap();
        let err = build_plan(dir.path(), "", "TN", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn rejects_forbidden_chars_in_tracking() {
        let dir = TempDir::new().unwrap();
        let err = build_plan(dir.path(), "Felix", "abc/def", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn rejects_missing_source_dir() {
        let bogus = PathBuf::from("/tmp/__definitely_not_existing_dir__");
        let err = build_plan(&bogus, "Felix", "TN", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
    }

    #[test]
    fn collects_only_top_level_pdfs_case_insensitive() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");
        touch(dir.path(), "b.PDF");
        touch(dir.path(), "c.txt");
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        touch(&dir.path().join("sub"), "ignored.pdf");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let names: Vec<_> = plans
            .iter()
            .map(|p| p.source.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.pdf", "b.PDF"]);
    }

    #[test]
    fn target_path_uses_invoice_number_user_and_tracking() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");

        let plans = build_plan(dir.path(), "Felix", "000-115-216", ok_extract).unwrap();
        assert_eq!(plans.len(), 1);
        let p = &plans[0];
        assert_eq!(p.invoice_number.as_deref(), Some("26322000000893295511"));
        assert_eq!(
            p.target,
            dir.path()
                .join("000-115-216")
                .join("26322000000893295511-Felix-000-115-216.pdf")
        );
    }

    #[test]
    fn unknown_prefix_when_extract_returns_none() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");
        let plans = build_plan(dir.path(), "Felix", "TN", none_extract).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].invoice_number, None);
        assert_eq!(
            plans[0]
                .target
                .file_name()
                .unwrap()
                .to_string_lossy(),
            "UNKNOWN-Felix-TN.pdf"
        );
    }
}
```

- [ ] **Step 2: 在 `lib.rs` 声明模块**

修改 `src-tauri/src/lib.rs`，在 `mod pdf_parser;` 后添加：

```rust
mod renamer;
```

- [ ] **Step 3: 运行测试**

Run: `cd src-tauri && cargo test renamer::tests -- --nocapture`
Expected: 6 个测试全部通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/renamer.rs src-tauri/src/lib.rs
git commit -m "feat(renamer): build_plan with validation and unknown fallback"
```

---

## Task 8：`renamer.rs` — `execute_plan` + 冲突加序号

**Files:**
- Modify: `src-tauri/src/renamer.rs`

- [ ] **Step 1: 在 `renamer.rs` 末尾（`#[cfg(test)]` 之前）追加 `execute_plan` 与辅助函数**

```rust
const MAX_DEDUPE: u32 = 999;

fn timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn make_log(level: &str, message: impl Into<String>) -> LogEntry {
    LogEntry {
        ts: timestamp(),
        level: level.to_string(),
        message: message.into(),
    }
}

/// 在 `target` 已存在时尝试 `name-1.pdf`、`name-2.pdf`… 直到上限 999。
/// 返回最终使用的路径以及"是否加了序号"，找不到空位时返回 None。
fn resolve_target(target: &Path) -> Option<(PathBuf, u32)> {
    if !target.exists() {
        return Some((target.to_path_buf(), 0));
    }
    let parent = target.parent()?;
    let stem = target.file_stem()?.to_str()?;
    let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
    for n in 1..=MAX_DEDUPE {
        let candidate_name = if ext.is_empty() {
            format!("{stem}-{n}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return Some((candidate, n));
        }
    }
    None
}

pub fn execute_plan<L: Logger>(plans: &[RenamePlan], logger: &mut L) -> RenameSummary {
    let total = plans.len();
    let mut success = 0usize;
    let mut failed = 0usize;

    if total == 0 {
        logger.log(make_log("warn", "未找到 PDF 文件"));
        return RenameSummary {
            total,
            success,
            failed,
        };
    }

    logger.log(make_log("info", format!("扫描到 {total} 个 PDF 文件")));

    // 输出目录由第一个 plan 的 target.parent 决定
    if let Some(first) = plans.first() {
        if let Some(out_dir) = first.target.parent() {
            if let Err(e) = std::fs::create_dir_all(out_dir) {
                logger.log(make_log(
                    "error",
                    format!("创建输出目录失败：{} ({e})", out_dir.display()),
                ));
                return RenameSummary {
                    total,
                    success,
                    failed: total,
                };
            }
        }
    }

    let started = std::time::Instant::now();

    for (idx, plan) in plans.iter().enumerate() {
        let i = idx + 1;
        let src_name = plan
            .source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // 提取阶段的日志
        if plan.invoice_number.is_none() {
            logger.log(make_log(
                "warn",
                format!("[{i}/{total}] {src_name} 未匹配到发票号码，使用 UNKNOWN 占位"),
            ));
        }

        match resolve_target(&plan.target) {
            None => {
                logger.log(make_log(
                    "error",
                    format!("[{i}/{total}] {src_name} 跳过：同名文件序号已耗尽（>{MAX_DEDUPE}）"),
                ));
                failed += 1;
            }
            Some((final_target, dedupe_n)) => {
                match std::fs::copy(&plan.source, &final_target) {
                    Ok(_) => {
                        let target_name = final_target
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let suffix_note = if dedupe_n > 0 {
                            "（同名已存在，加序号）"
                        } else {
                            ""
                        };
                        logger.log(make_log(
                            "info",
                            format!("[{i}/{total}] {src_name} → {target_name}{suffix_note}"),
                        ));
                        success += 1;
                    }
                    Err(e) => {
                        logger.log(make_log(
                            "error",
                            format!("[{i}/{total}] {src_name} 复制失败：{e}"),
                        ));
                        failed += 1;
                    }
                }
            }
        }
    }

    let secs = started.elapsed().as_secs_f32();
    logger.log(make_log(
        "info",
        format!("完成：成功 {success}，失败 {failed}，耗时 {secs:.1}s"),
    ));

    RenameSummary {
        total,
        success,
        failed,
    }
}
```

- [ ] **Step 2: 在 `#[cfg(test)] mod tests` 末尾追加 execute_plan 测试**

```rust
    #[derive(Default)]
    struct VecLogger {
        entries: Vec<LogEntry>,
    }
    impl Logger for VecLogger {
        fn log(&mut self, entry: LogEntry) {
            self.entries.push(entry);
        }
    }

    fn make_pdf(dir: &Path, name: &str, payload: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, payload).unwrap();
        p
    }

    #[test]
    fn execute_copies_files_to_output_subdir() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        make_pdf(dir.path(), "b.pdf", "BBB");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let mut logger = VecLogger::default();
        let summary = execute_plan(&plans, &mut logger);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.success, 2);
        assert_eq!(summary.failed, 0);

        let out = dir.path().join("TN");
        assert!(out.is_dir());
        let copied: Vec<_> = std::fs::read_dir(&out).unwrap().count() as usize == 2 {
            // 校验复制后内容仍然存在
            Vec::new()
        } else {
            panic!("应有 2 个文件");
        };
        let _ = copied;
        // 原文件仍在源目录
        assert!(dir.path().join("a.pdf").exists());
        assert!(dir.path().join("b.pdf").exists());
    }

    #[test]
    fn execute_adds_sequence_suffix_on_conflict() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "first");
        make_pdf(dir.path(), "b.pdf", "second"); // 两张 PDF 因 mock 提取出同样号码

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let mut logger = VecLogger::default();
        let summary = execute_plan(&plans, &mut logger);

        assert_eq!(summary.success, 2);
        let out = dir.path().join("TN");
        let base = out.join("26322000000893295511-Felix-TN.pdf");
        let seq = out.join("26322000000893295511-Felix-TN-1.pdf");
        assert!(base.exists());
        assert!(seq.exists());
    }

    #[test]
    fn execute_empty_plan_logs_warn() {
        let plans: Vec<RenamePlan> = Vec::new();
        let mut logger = VecLogger::default();
        let s = execute_plan(&plans, &mut logger);
        assert_eq!(s.total, 0);
        assert_eq!(s.success, 0);
        assert_eq!(s.failed, 0);
        assert!(logger.entries.iter().any(|e| e.message.contains("未找到 PDF")));
    }

    #[test]
    fn execute_unknown_logs_warn_but_still_copies() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        let plans = build_plan(dir.path(), "Felix", "TN", none_extract).unwrap();
        let mut logger = VecLogger::default();
        let s = execute_plan(&plans, &mut logger);
        assert_eq!(s.success, 1);
        assert!(logger.entries.iter().any(|e| e.level == "warn" && e.message.contains("UNKNOWN")));
        assert!(dir.path().join("TN").join("UNKNOWN-Felix-TN.pdf").exists());
    }
```

> **注意**：步骤 2 中第一个测试里的 `copied` 用法可能在某些 lint 模式下报警告——其目的只是断言"目录里有 2 个条目"。如果 cargo 给出 unused 变量警告，把第一个测试简化为直接断言：
>
> ```rust
> let count = std::fs::read_dir(&out).unwrap().count();
> assert_eq!(count, 2);
> ```

- [ ] **Step 3: 修正测试代码**

把 Step 2 中第一个测试的 `copied` 块替换为：

```rust
        let count = std::fs::read_dir(&out).unwrap().count();
        assert_eq!(count, 2);
```

- [ ] **Step 4: 运行测试**

Run: `cd src-tauri && cargo test renamer::tests -- --nocapture`
Expected: 10 个 renamer 测试全部通过。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/renamer.rs
git commit -m "feat(renamer): execute_plan with conflict suffix and logger"
```

---

## Task 9：`commands.rs` — `rename_pdfs` 命令 + `ChannelLogger`

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`（声明 + 注册命令）

- [ ] **Step 1: 写 `src-tauri/src/commands.rs`**

```rust
use crate::error::AppError;
use crate::pdf_parser::extract_invoice_number;
use crate::renamer::{build_plan, execute_plan, LogEntry, Logger, RenameSummary};
use std::path::PathBuf;
use tauri::ipc::Channel;

struct ChannelLogger {
    channel: Channel<LogEntry>,
}

impl Logger for ChannelLogger {
    fn log(&mut self, entry: LogEntry) {
        // Channel::send 失败仅在前端已关闭通道时发生；此时静默丢弃即可
        let _ = self.channel.send(entry);
    }
}

#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    on_log: Channel<LogEntry>,
) -> Result<RenameSummary, AppError> {
    let source = PathBuf::from(&source_dir);
    if !source.is_dir() {
        return Err(AppError::Io(format!("源文件夹不存在：{source_dir}")));
    }

    // 在阻塞线程池里跑 IO + PDF 解析，避免阻塞 Tauri 主线程
    let summary = tauri::async_runtime::spawn_blocking(move || -> Result<RenameSummary, AppError> {
        let plans = build_plan(&source, &user_name, &tracking_number, |p| {
            extract_invoice_number(p)
        })?;
        let mut logger = ChannelLogger { channel: on_log };
        Ok(execute_plan(&plans, &mut logger))
    })
    .await
    .map_err(|e| AppError::Io(format!("任务调度失败：{e}")))??;

    Ok(summary)
}
```

- [ ] **Step 2: 更新 `src-tauri/src/lib.rs`**

完整内容：

```rust
mod error;
mod pdf_parser;
mod renamer;
mod commands;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![commands::rename_pdfs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 验证 cargo check + test 通过**

Run: `cd src-tauri && cargo check && cargo test`
Expected: 编译通过，所有单元测试通过。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): expose rename_pdfs with channel logger"
```

---

## Task 10：前端 UI — HTML 与 CSS

**Files:**
- Modify: `src/index.html`
- Create: `src/style.css`

- [ ] **Step 1: 写 `src/index.html`**

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1.0" />
    <title>PDF 发票批量重命名</title>
    <link rel="stylesheet" href="style.css" />
  </head>
  <body>
    <main class="app">
      <h1>PDF 发票批量重命名</h1>

      <section class="form">
        <label class="field">
          <span>源文件夹</span>
          <div class="row">
            <input id="sourceDir" type="text" readonly placeholder="点击右侧选择文件夹" />
            <button id="pickDirBtn" type="button">选择</button>
          </div>
        </label>

        <label class="field">
          <span>用户名</span>
          <input id="userName" type="text" placeholder="如：Felix" autocomplete="off" />
        </label>

        <label class="field">
          <span>Tracking Number</span>
          <input id="trackingNumber" type="text" placeholder="如：000-115-216" autocomplete="off" />
        </label>

        <div class="actions">
          <span id="formError" class="error-text" aria-live="polite"></span>
          <button id="runBtn" type="button" class="primary">开始重命名</button>
        </div>
      </section>

      <section class="log">
        <h2>执行日志</h2>
        <div id="logBox" class="log-box" aria-live="polite"></div>
      </section>
    </main>

    <script type="module" src="main.js"></script>
  </body>
</html>
```

- [ ] **Step 2: 写 `src/style.css`**

```css
:root {
  --bg: #ffffff;
  --fg: #1d1d1f;
  --muted: #6e6e73;
  --border: #d2d2d7;
  --accent: #0a84ff;
  --error: #d70015;
  --warn: #c46a00;
  --log-bg: #f5f5f7;
  --log-fg: #1d1d1f;
  font-family: -apple-system, "SF Pro Text", "PingFang SC", "Microsoft YaHei", sans-serif;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg: #1c1c1e;
    --fg: #f5f5f7;
    --muted: #98989d;
    --border: #3a3a3c;
    --accent: #0a84ff;
    --error: #ff453a;
    --warn: #ff9f0a;
    --log-bg: #2c2c2e;
    --log-fg: #f5f5f7;
  }
}

* { box-sizing: border-box; }

html, body {
  margin: 0;
  height: 100%;
  background: var(--bg);
  color: var(--fg);
}

.app {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 20px 24px;
  gap: 16px;
}

h1 {
  font-size: 18px;
  margin: 0;
}

h2 {
  font-size: 14px;
  margin: 0 0 8px;
  color: var(--muted);
  font-weight: 500;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: 10px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 13px;
}

.field > span {
  color: var(--muted);
}

.row {
  display: flex;
  gap: 8px;
}

input[type="text"] {
  flex: 1;
  font-size: 13px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--fg);
}

input[type="text"]:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(10, 132, 255, 0.25);
}

input[readonly] {
  background: var(--log-bg);
  cursor: default;
}

button {
  font-size: 13px;
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--fg);
  cursor: pointer;
}

button:hover:not(:disabled) {
  border-color: var(--accent);
}

button.primary {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

button.primary:hover:not(:disabled) {
  filter: brightness(0.95);
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 4px;
}

.error-text {
  color: var(--error);
  font-size: 12px;
  flex: 1;
  text-align: right;
}

.log {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.log-box {
  flex: 1;
  overflow-y: auto;
  background: var(--log-bg);
  color: var(--log-fg);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px;
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}

.log-line { display: block; }
.log-line.info  { color: var(--log-fg); }
.log-line.warn  { color: var(--warn); }
.log-line.error { color: var(--error); }
.log-meta { color: var(--muted); margin-right: 6px; }
```

- [ ] **Step 3: 跑一次空 dev 启动验证 UI 渲染**

Run: `npm run tauri dev`（首次启动会编译 Rust，约 30-90 秒；编译完成后弹出窗口确认 UI 可见后 Ctrl+C 关闭）
Expected: 弹出窗口，看到标题、三个输入框、"选择"和"开始重命名"按钮、空的日志区。

- [ ] **Step 4: Commit**

```bash
git add src/index.html src/style.css
git commit -m "feat(ui): static html and css for main view"
```

---

## Task 11：前端 — JS 逻辑（选择目录、校验、调用命令、日志渲染）

**Files:**
- Create: `src/main.js`

- [ ] **Step 1: 写 `src/main.js`**

```javascript
import { invoke, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const FORBIDDEN = /[\/\\:*?"<>|]/;

const $sourceDir = document.getElementById("sourceDir");
const $userName = document.getElementById("userName");
const $tracking = document.getElementById("trackingNumber");
const $pickBtn = document.getElementById("pickDirBtn");
const $runBtn = document.getElementById("runBtn");
const $err = document.getElementById("formError");
const $log = document.getElementById("logBox");

let running = false;

$pickBtn.addEventListener("click", async () => {
  if (running) return;
  const picked = await open({ directory: true, multiple: false });
  if (typeof picked === "string" && picked.length > 0) {
    $sourceDir.value = picked;
    showError("");
  }
});

$runBtn.addEventListener("click", runRename);

function showError(msg) {
  $err.textContent = msg || "";
}

function validate() {
  const sourceDir = $sourceDir.value.trim();
  const userName = $userName.value.trim();
  const tracking = $tracking.value.trim();

  if (!sourceDir) return "请选择源文件夹";
  if (!userName) return "请输入用户名";
  if (FORBIDDEN.test(userName)) return "用户名不能包含特殊字符 / \\ : * ? \" < > |";
  if (!tracking) return "请输入 Tracking Number";
  if (FORBIDDEN.test(tracking)) return "Tracking Number 不能包含特殊字符 / \\ : * ? \" < > |";
  return null;
}

function setRunning(flag) {
  running = flag;
  $runBtn.disabled = flag;
  $pickBtn.disabled = flag;
  $userName.disabled = flag;
  $tracking.disabled = flag;
  $runBtn.textContent = flag ? "处理中…" : "开始重命名";
}

function clearLog() {
  $log.replaceChildren();
}

function appendLog(entry) {
  const line = document.createElement("span");
  line.className = `log-line ${entry.level || "info"}`;
  const meta = document.createElement("span");
  meta.className = "log-meta";
  meta.textContent = `[${entry.ts}] ${(entry.level || "info").toUpperCase()}`;
  line.appendChild(meta);
  line.appendChild(document.createTextNode(entry.message));
  line.appendChild(document.createTextNode("\n"));
  $log.appendChild(line);
  $log.scrollTop = $log.scrollHeight;
}

async function runRename() {
  if (running) return;
  const err = validate();
  if (err) {
    showError(err);
    return;
  }
  showError("");
  clearLog();
  setRunning(true);

  const channel = new Channel();
  channel.onmessage = (msg) => appendLog(msg);

  try {
    const summary = await invoke("rename_pdfs", {
      sourceDir: $sourceDir.value.trim(),
      userName: $userName.value.trim(),
      trackingNumber: $tracking.value.trim(),
      onLog: channel,
    });
    // execute_plan 内部已经写过"完成：..."这一行；此处不重复
    void summary;
  } catch (e) {
    appendLog({
      ts: new Date().toTimeString().slice(0, 8),
      level: "error",
      message: typeof e === "string" ? e : JSON.stringify(e),
    });
  } finally {
    setRunning(false);
  }
}
```

- [ ] **Step 2: 重新启动 dev 验证流程**

Run: `npm run tauri dev`（运行后用真实文件夹手动测试一次：选目录 → 输入用户名 → 输入 tracking → 点开始）
Expected:
- 空表单点"开始重命名" → 红色提示具体错误。
- 选好目录且 3 项填写 → 日志区出现实时滚动条目。
- 子目录 `{源}/{trackingNumber}/` 被创建，里面是重命名后的 PDF。
- 原 PDF 仍在源目录。
- 完成后按钮恢复。

- [ ] **Step 3: Commit**

```bash
git add src/main.js
git commit -m "feat(ui): wire dialog, validation, channel logging"
```

---

## Task 12：手动回归 + 生成发布版

**Files:**（无新增/修改）

- [ ] **Step 1: 手动回归**

参照设计文档 §8.3 的清单逐条手动验证：
1. 空表单提交 → 提示。
2. 选空目录 → 日志 warn "未找到 PDF 文件"。
3. 含数电票的目录 → 日志正确滚出、文件名正确。
4. 同批跑两次 → 第二次出现加序号日志。
5. 塞一个加密 PDF（可临时用 `qpdf --encrypt ...` 制造一个）→ 该项 error，其他正常。

回归过程中遇到与设计冲突的行为应记录到 `docs/superpowers/specs/2026-05-13-pdf-rename-design.md` 末尾，再修正代码。

- [ ] **Step 2: 生产打包**

Run: `npm run tauri build`
Expected: `src-tauri/target/release/bundle/macos/PDF Rename.app` 生成，可双击启动。

- [ ] **Step 3: 验证打包后的 .app 行为**

双击 `PDF Rename.app`，重复 Step 1 的关键路径（选目录 → 跑一次 → 验证子目录与文件名）。Expected: 不依赖 dev 模式仍能正常运行；PDFium 已在 `Resources/lib/libpdfium.dylib`。

- [ ] **Step 4: Commit 任何修正**

```bash
git add -A
git commit -m "chore: post-validation tweaks" || echo "nothing to commit"
```

---

## Self-Review 备忘

实施过程中如发现以下任一情况，必须停下来核对：
1. PDFium 加载失败（dylib 路径找不到）：核对 `locate_lib_dir` 实际返回值与 dylib 实际放置位置。
2. 中文文本提取乱码：可能 `text.all()` 没生效，检查 pdfium-render 版本与 API。
3. `Channel<LogEntry>` API 不存在：核对当前 `@tauri-apps/api` 与 `tauri::ipc::Channel` 版本是否一致；如版本差异，使用 `app.emit("rename-log", payload)` + 前端 `listen` 作为替代实现，但前后端都要改。
4. 任何步骤中的 cargo/clippy 警告：保持 zero-warning 标准；非本任务作用域的警告先记录到 `docs/` 而不要顺手改。

