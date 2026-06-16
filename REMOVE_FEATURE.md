# 文件移除功能

## 功能描述

在文件列表中为每个文件添加"移除"操作按钮，允许用户在重命名前从待处理列表中移除不需要的文件。

## 用户场景

### 典型使用场景
1. **混合文件夹**：文件夹中既有发票 PDF，也有其他 PDF 文档
2. **错误文件**：个别 PDF 解析失败或格式不正确
3. **重复文件**：同一发票的不同版本
4. **测试文件**：临时测试用的 PDF 文件

### 使用流程
```
1. 选择文件夹 → 自动扫描显示所有 PDF
2. 查看文件列表和总金额
3. 点击"移除"按钮排除不需要的文件
4. 总金额和统计数据实时更新
5. 填写用户名和 TN
6. 点击"开始重命名" → 只处理保留的文件
```

## 技术实现

### 前端改动

#### 1. HTML 结构（index.html）
```html
<thead>
  <tr>
    <th class="col-idx">#</th>
    <th class="col-name">源文件名</th>
    <th class="col-invoice">发票号</th>
    <th class="col-amount">合计</th>
    <th class="col-status">结果</th>
    <th class="col-action">操作</th>  <!-- 新增列 -->
  </tr>
</thead>
```

#### 2. JavaScript 逻辑（main.js）

**renderScannedFile()** - 渲染扫描文件时添加移除按钮
```javascript
const actionCell = document.createElement("td");
actionCell.className = "cell-action";
const removeBtn = document.createElement("button");
removeBtn.className = "btn-remove";
removeBtn.textContent = "移除";
removeBtn.onclick = () => removeFile(file.index);
actionCell.append(removeBtn);
tr.append(actionCell);
```

**removeFile()** - 移除文件并更新显示
```javascript
function removeFile(fileIndex) {
  // 1. 从 scannedFiles 缓存中移除
  const removedIndex = scannedFiles.findIndex(f => f.index === fileIndex);
  scannedFiles.splice(removedIndex, 1);

  // 2. 从 DOM 中移除对应行
  const row = $resultBody.querySelector(`tr[data-file-index="${fileIndex}"]`);
  row.remove();

  // 3. 重新编号显示（保持连续）
  let displayIndex = 1;
  Array.from($resultBody.querySelectorAll('tr')).forEach((row) => {
    const indexCell = row.querySelector('.cell-idx');
    if (indexCell && !row.querySelector('.table-empty')) {
      indexCell.textContent = String(displayIndex);
      displayIndex++;
    }
  });

  // 4. 更新汇总信息（总金额、文件数量）
  updateScanSummary();

  // 5. 处理空列表情况
  if (scannedFiles.length === 0) {
    // 显示"未找到 PDF 文件"
  }
}
```

**updateScanSummary()** - 更新汇总信息
```javascript
function updateScanSummary() {
  let totalAmountCents = 0;
  let amountRecognized = 0;
  let amountMissing = 0;

  for (const file of scannedFiles) {
    if (file.amountCents !== null && file.amountCents !== undefined) {
      totalAmountCents += file.amountCents;
      amountRecognized++;
    } else {
      amountMissing++;
    }
  }

  $summaryAmount.textContent = formatAmount(totalAmountCents);
  renderStats({
    count: scannedFiles.length,
    success: 0,
    failed: 0,
    missing: amountMissing,
  });
}
```

**runRename()** - 只重命名保留的文件
```javascript
async function runRename() {
  // ...
  
  // 提取要处理的文件名列表
  const fileNames = scannedFiles.map(f => f.sourceName);

  const summary = await invoke("rename_pdfs", {
    sourceDir: $sourceDir.value.trim(),
    userName: $userName.value.trim(),
    trackingNumber: $tracking.value.trim(),
    fileNames: fileNames,  // 只传递保留的文件
    onRow: channel,
  });
  
  // ...
}
```

#### 3. CSS 样式（style.css）

```css
.col-action {
  width: 60px;
  text-align: center;
}

.btn-remove {
  padding: 3px 8px;
  font-size: 11px;
  line-height: 1.4;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--surface);
  color: var(--muted);
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-remove:hover {
  background: var(--error-soft);
  border-color: var(--error);
  color: var(--error);
}

.btn-remove:active {
  transform: scale(0.95);
}

.result-table td.cell-action {
  text-align: center;
}
```

### 后端改动

#### 1. 修改 rename_pdfs 命令（commands.rs）

**新增参数**：
```rust
#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    file_names: Vec<String>,  // 新增：要处理的文件名列表
    on_row: Channel<InvoiceRow>,
) -> Result<RenameSummary, AppError>
```

调用新函数：
```rust
let plans = build_plan_for_files(
    &source, 
    &user_name, 
    &tracking_number, 
    &file_names,  // 传递文件名列表
    extract_invoice_info
)?;
```

#### 2. 新增 build_plan_for_files 函数（renamer.rs）

```rust
pub fn build_plan_for_files<F>(
    source_dir: &Path,
    user_name: &str,
    tracking_number: &str,
    file_names: &[String],  // 只处理这些文件
    extract_fn: F,
) -> Result<Vec<RenamePlan>, AppError>
where
    F: Fn(&Path) -> Result<InvoiceInfo, AppError>,
{
    // 验证参数
    validate_name("用户名", user_name)?;
    validate_name("Tracking Number", tracking_number)?;

    // 只处理传入的文件列表
    for file_name in file_names {
        let path = source_dir.join(file_name);
        
        if !path.is_file() || !is_pdf(&path) {
            continue;
        }

        // 提取发票信息
        let (info, parse_error) = match extract_fn(&path) {
            Ok(info) => (info, None),
            Err(e) => (InvoiceInfo::default(), Some(e.to_string())),
        };

        // 构建重命名计划
        // ...
    }

    Ok(plans)
}
```

**与原 build_plan 的区别**：
- `build_plan()`: 扫描整个文件夹的所有 PDF
- `build_plan_for_files()`: 只处理指定的文件列表

## 用户体验

### 实时反馈
- ✅ 点击"移除"后，该行立即消失
- ✅ 文件序号自动重新编号（保持连续）
- ✅ 总金额立即更新
- ✅ 统计数据实时更新（总张数、未识别金额）

### 数据一致性
- ✅ 移除的文件不参与总金额计算
- ✅ 移除的文件不参与重命名操作
- ✅ 统计数据始终准确反映当前列表

### 边界处理
- ✅ 移除所有文件后显示"未找到 PDF 文件"
- ✅ 重命名时自动跳过已移除的文件
- ✅ 移除后可以重新选择文件夹（清空列表）

## 典型使用示例

### 示例 1：排除非发票文件
```
初始扫描结果：
1. invoice-001.pdf  ¥100.00  [待处理] [移除]
2. receipt-002.pdf  ¥50.00   [待处理] [移除]  ← 这是收据，不是发票
3. invoice-003.pdf  ¥200.00  [待处理] [移除]

点击第 2 行的"移除"按钮：
1. invoice-001.pdf  ¥100.00  [待处理] [移除]
2. invoice-003.pdf  ¥200.00  [待处理] [移除]

总金额：¥300.00（原来 ¥350.00）
总张数：2（原来 3）
```

### 示例 2：排除解析失败的文件
```
初始扫描结果：
1. invoice-001.pdf  ¥100.00  [待处理] [移除]
2. encrypted.pdf    —        [⚠]      [移除]  ← 加密 PDF，无法解析
3. invoice-003.pdf  ¥200.00  [待处理] [移除]

点击第 2 行的"移除"按钮：
1. invoice-001.pdf  ¥100.00  [待处理] [移除]
2. invoice-003.pdf  ¥200.00  [待处理] [移除]

总金额：¥300.00
总张数：2
未识别金额：0（原来 1）
```

### 示例 3：移除所有文件
```
点击所有"移除"按钮后：

┌────────────────────────────────┐
│      未找到 PDF 文件            │
└────────────────────────────────┘

总金额：¥0.00
总张数：0
```

## 技术细节

### 缓存机制
```javascript
let scannedFiles = []; // 全局缓存

// 扫描时填充
scannedFiles = scanResult.files || [];

// 移除时更新
scannedFiles.splice(removedIndex, 1);

// 重命名时使用
const fileNames = scannedFiles.map(f => f.sourceName);
```

### 序号重编机制
```javascript
// 只更新显示序号，不修改原始数据
let displayIndex = 1;
Array.from($resultBody.querySelectorAll('tr')).forEach((row) => {
  const indexCell = row.querySelector('.cell-idx');
  if (indexCell && !row.querySelector('.table-empty')) {
    indexCell.textContent = String(displayIndex);
    displayIndex++;
  }
});
```

### 状态管理

| 状态 | 显示移除按钮 | 说明 |
|------|------------|------|
| 扫描完成 | ✅ 是 | 用户可以移除文件 |
| 重命名中 | ❌ 否 | 不显示移除按钮 |
| 重命名完成 | ❌ 否 | 显示结果，不可移除 |

## 版本信息

- 功能版本: v0.4.2
- 实现日期: 2026-06-16
- 依赖功能: 即时文件扫描 (v0.4.1)

## 测试场景

### 功能测试
- [x] 扫描后每行显示"移除"按钮
- [x] 点击"移除"按钮，该行立即消失
- [x] 移除后序号重新编号（1, 2, 3...）
- [x] 移除后总金额正确更新
- [x] 移除后统计数据正确更新
- [x] 移除所有文件后显示空状态
- [x] 重命名时只处理保留的文件
- [x] 重命名结果不显示移除按钮

### 边界测试
- [x] 单个文件：移除后显示空状态
- [x] 移除部分文件：正确重编序号
- [x] 移除所有文件：显示"未找到 PDF 文件"
- [x] 移除后重新选择文件夹：列表正确刷新

### 交互测试
- [x] 按钮 hover 效果：红色高亮
- [x] 按钮点击效果：缩小动画
- [x] 快速连续点击：不产生错误
- [x] 移除期间总金额动画：平滑更新
