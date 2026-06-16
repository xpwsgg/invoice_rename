# 用户体验优化 - 即时文件扫描

## 优化内容

将文件扫描和金额计算从"点击开始重命名"时执行，改为**选择文件夹后立即执行**。

## 用户体验对比

### 优化前
```
1. 用户选择文件夹
2. 填写用户名和 TN
3. 点击"开始重命名"
4. 开始扫描文件...（等待）
5. 显示文件列表和总金额
6. 执行重命名
```

### 优化后
```
1. 用户选择文件夹
2. 立即显示文件列表和总金额 ✨（无需等待）
3. 填写用户名和 TN（此时已知文件数量和总金额）
4. 点击"开始重命名"
5. 直接执行重命名（复用扫描结果）
```

## 技术实现

### 后端更改（Rust）

#### 1. 新增 `scan_pdfs` 命令
**文件**: `src-tauri/src/commands.rs`

```rust
#[tauri::command]
pub async fn scan_pdfs(source_dir: String) -> Result<ScanSummary, AppError>
```

功能：
- 扫描指定文件夹的所有 PDF 文件
- 提取发票号和金额信息
- 返回文件列表和总金额汇总
- 无需用户名和 TN，只需文件夹路径

#### 2. 新增数据结构

```rust
pub struct ScannedFile {
    pub index: usize,
    pub source_name: String,
    pub invoice_number: Option<String>,
    pub amount_cents: Option<i64>,
    pub amount_display: Option<String>,
    pub parse_error: Option<String>,
}

pub struct ScanSummary {
    pub total: usize,
    pub total_amount_cents: i64,
    pub amount_recognized: usize,
    pub amount_missing: usize,
    pub parse_errors: usize,
    pub files: Vec<ScannedFile>,
}
```

#### 3. 注册命令
**文件**: `src-tauri/src/lib.rs`

```rust
.invoke_handler(tauri::generate_handler![
    commands::scan_pdfs,        // 新增
    commands::rename_pdfs,
    commands::open_folder
])
```

### 前端更改（JavaScript）

#### 1. 选择文件夹后立即扫描
**文件**: `src/main.js`

```javascript
$pickBtn.addEventListener("click", async () => {
  if (running) return;
  const picked = await open({ directory: true, multiple: false });
  if (typeof picked === "string" && picked.length > 0) {
    $sourceDir.value = picked;
    showError("");
    await scanFolder(picked); // 立即扫描
  }
});
```

#### 2. 新增 scanFolder() 函数

```javascript
async function scanFolder(sourceDir) {
  setMode("run");
  resetTable();
  setRunning(true);

  try {
    const scanResult = await invoke("scan_pdfs", { sourceDir });
    scannedFiles = scanResult.files || [];

    // 渲染文件列表
    for (const file of scannedFiles) {
      // 显示文件信息：序号、文件名、发票号、金额、状态
    }

    // 更新总金额
    $summaryAmount.textContent = formatAmount(scanResult.totalAmountCents);
    renderStats({ ... });

  } catch (e) {
    showError(e);
  } finally {
    setRunning(false);
  }
}
```

#### 3. 缓存扫描结果

```javascript
let scannedFiles = []; // 缓存扫描结果，避免重复扫描
```

在 `runRename()` 中复用：
```javascript
if (scannedFiles.length === 0) {
  await scanFolder(sourceDir);
}
```

### 样式更改（CSS）

**文件**: `src/style.css`

新增状态样式：
```css
.status-badge.warn {
  color: var(--warn);
}

.status-badge.pending {
  color: var(--info);
}

.result-table tbody tr.row-warning td {
  background: var(--warn-soft);
}
```

## 用户价值

### 1. 即时反馈
- 选择文件夹后立即看到：
  - 有多少个 PDF 文件
  - 总金额是多少
  - 是否有文件解析失败

### 2. 确认正确性
- 在填写用户名和 TN 之前就知道文件夹是否正确
- 避免填写完才发现选错文件夹

### 3. 提前发现问题
- 解析失败的文件会显示 ⚠ 警告
- 未识别金额的文件会标记
- 用户可以提前决定是否继续

### 4. 性能优化
- 扫描结果被缓存
- "开始重命名"时复用扫描结果
- 避免重复解析 PDF

## 状态显示

| 状态 | 显示 | 含义 |
|------|------|------|
| 待处理 | 灰色"待处理" | 扫描完成，等待重命名 |
| 警告 | 黄色 ⚠ | PDF 解析失败 |
| 成功 | 绿色 ✓ | 重命名成功 |
| 失败 | 红色 ✗ | 重命名失败 |

## 构建和测试

### 构建命令
```bash
# 前端
npm run build

# 后端
cd src-tauri && cargo build --release

# 开发模式
npm run tauri dev
```

### 测试场景
1. **正常场景**: 选择包含多个 PDF 的文件夹，验证立即显示文件列表和总金额
2. **空文件夹**: 选择空文件夹，验证显示"未找到 PDF 文件"
3. **解析失败**: 选择包含加密/损坏 PDF 的文件夹，验证显示警告标记
4. **重复选择**: 多次选择不同文件夹，验证缓存正确更新

## 版本信息

- 优化日期: 2026-06-16
- 版本: v0.4.1+
- 影响范围: 文件扫描流程、用户交互体验
