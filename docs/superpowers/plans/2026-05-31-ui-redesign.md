# 界面重设计实现计划（浅色 + 工具条常驻）

> **For agentic workers:** REQUIRED SUB-SKILL: 用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务执行。步骤用 `- [ ]` 复选框跟踪。

**Goal:** 把 ESI 发票重命名工具的界面从「深浅双主题跟随系统 + 表单/结果垂直堆叠」改为「固定浅色 + 顶部工具条常驻 + 结果区最大化」，解决空间分配、元素细节、整体观感三个痛点。

**Architecture:** 纯前端改动，三文件 `src/index.html` / `src/style.css` / `src/main.js`；**后端零改动**（现有 `InvoiceRow` / `RenameSummary` 字段已够用）。核心约束：**保持所有元素 id 不变**（`sourceDir` / `pickDirBtn` / `userName` / `trackingNumber` / `runBtn` / `formError` / `openFolderBtn` / `summaryBar` / `summaryAmount` / `summaryStats` / `progressTrack` / `progressFill` / `progressText` / `helpView` / `resultTable` / `resultBody`），使 main.js 选择器基本不变，改动集中在布局结构与少量行为。

**Tech Stack:** Vanilla HTML/CSS/JS + Vite 5 + Tauri 2。

**关联设计文档:** `docs/superpowers/specs/2026-05-31-ui-redesign-design.md`

---

## 验证策略（重要：本项目无前端测试框架）

前端为纯视觉/布局重构，无 vitest/jest，不强加无意义 UI 单测。每个任务的验证方式：

1. 执行期间保持 `npm run dev` 运行（端口 1420，`strictPort`）。若 1420 被占用说明已有 dev server，直接复用。
2. 每次改动后浏览器自动热更新，用 chrome-devtools 截图核对（视口设 760×800，与 app 窗口一致）。
3. 核对 run 态需要数据时，用 `evaluate_script` 注入 mock 行（脚本见附录 A）。
4. 全部完成后 Task 7 跑 `npm run build` 做生产构建语法验证，再由用户 `npm run tauri dev` 人工核对真实样本。

**后端不受影响**，但每个改动 commit 前确保 `npm run build` 不会因语法错误失败（Task 内未单独 build 的，靠 dev server 已能加载即说明无致命语法错误）。

---

## File Structure

| 文件 | 职责 | 本次改动 |
|------|------|----------|
| `src/index.html` | DOM 结构 | 标题栏微调；`form`→`toolbar`；`summaryBar` 重构；移除 `clearLogBtn`；helpView 文案 |
| `src/style.css` | 样式 | 固定浅色变量 + 删 dark media query；新增 `.toolbar`；重构 `.summary-bar`；表格/进度条/说明卡微调 |
| `src/main.js` | 行为 | 移除返回说明逻辑；`renderStats` 改 4 统计列；进度条 running 显示/完成隐藏 |

---

## Task 1: 固定浅色基调（删除深色跟随）

**Files:**
- Modify: `src/style.css:1-55`（`:root` 与 `@media (prefers-color-scheme: dark)`）

- [ ] **Step 1: 替换 `:root` 变量并删除整段 dark media query**

把 `src/style.css` 第 1–55 行（从 `:root {` 到 dark media query 的 `}`，即当前 `* {` 之前的全部）整体替换为：

```css
:root {
  --bg: #f4f5f7;
  --fg: #1b1f27;
  --muted: #6b7280;
  --faint: #9ca3af;
  --border: #e5e7eb;
  --hairline: #f1f2f4;
  --surface: #ffffff;
  --surface-elevated: #fafbfc;
  --surface-soft: #f3f4f6;
  --accent: #2f6bff;
  --accent-strong: #2257e0;
  --accent-soft: #eef3ff;
  --accent-ring: rgba(47, 107, 255, 0.18);
  --shadow-card: 0 1px 2px rgba(16, 24, 40, 0.04);
  --shadow-button: 0 1px 2px rgba(47, 107, 255, 0.22);

  --error: #e5484d;
  --error-soft: #fdeced;
  --warn: #d97706;
  --warn-soft: #fdf0e3;
  --success: #16a34a;
  --success-soft: #e8f6ee;
  --info: #6b7280;
  --info-soft: #eef0f2;

  font-family: -apple-system, BlinkMacSystemFont, "SF Pro SC", "SF Pro Text",
    "PingFang SC", "Microsoft YaHei", sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

> 关键：**不保留** `@media (prefers-color-scheme: dark)` 块——这是固定浅色的前提。新增 `--faint`、`--accent-soft`；其余变量名沿用，旧布局会立即套用新浅色值。

- [ ] **Step 2: 截图核对（深色系统下也应为浅色）**

确保 `npm run dev` 运行，chrome-devtools 视口 760×800，导航 `http://localhost:1420` 并 `emulate` colorScheme=dark 后截图——界面应**仍为浅色**（验证 dark media query 已删除）。再切回 light 截图确认无异常。

- [ ] **Step 3: Commit**

```bash
git add src/style.css
git commit -m "style(ui): 固定浅色主题，移除系统深色跟随"
```

---

## Task 2: 顶部工具条（form → toolbar）

**Files:**
- Modify: `src/index.html:16-39`（`<section class="card form">` 整段）
- Modify: `src/style.css`（在 `/* ---------- Form ---------- */` 段之后新增 Toolbar 段）

- [ ] **Step 1: 替换表单 HTML 为工具条**

把 `src/index.html` 第 16–39 行 `<section class="card form"> … </section>` 整段替换为：

```html
      <section class="card toolbar">
        <div class="tb-row">
          <div class="tb-folder">
            <span class="tb-folder-icon">📁</span>
            <input id="sourceDir" type="text" readonly placeholder="点击选择源文件夹" />
            <button id="pickDirBtn" type="button" class="tb-pick">选择</button>
          </div>
          <input id="userName" class="tb-input" type="text" placeholder="用户名，如 Felix" autocomplete="off" />
          <input id="trackingNumber" class="tb-input tb-tn" type="text" placeholder="TN，如 000-116-216" autocomplete="off" />
          <button id="runBtn" type="button" class="primary">开始重命名</button>
        </div>
        <span id="formError" class="error-text" aria-live="polite"></span>
      </section>
```

> id 全部保留（`sourceDir` / `pickDirBtn` / `userName` / `trackingNumber` / `runBtn` / `formError`），main.js 无需改动。`formError` 从右对齐的 `.actions` 移到工具条下方作错误条。

- [ ] **Step 2: 新增 Toolbar 样式**

在 `src/style.css` 的 `/* ---------- Form ---------- */` 段（`.form { … }` 等）之后，`/* ---------- Card surface ---------- */` 之外的位置，新增以下样式（用 `.toolbar` 前缀提高特异性，覆盖通用 `input[type="text"]`）：

```css
/* ---------- Toolbar ---------- */

.toolbar {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
}

.tb-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.toolbar .tb-folder {
  flex: 2.4;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  height: 38px;
  padding: 0 6px 0 11px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-elevated);
}

.tb-folder-icon {
  flex: 0 0 auto;
  font-size: 14px;
  opacity: 0.8;
}

.toolbar .tb-folder input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  padding: 0;
  font-size: 12.5px;
  color: var(--fg);
  height: auto;
}

.toolbar .tb-folder input:focus {
  outline: none;
  box-shadow: none;
  border: none;
}

.toolbar .tb-pick {
  flex: 0 0 auto;
  padding: 5px 12px;
  font-size: 12px;
  font-weight: 550;
  color: var(--accent);
  background: var(--accent-soft);
  border: none;
  border-radius: 6px;
}

.toolbar .tb-pick:hover:not(:disabled) {
  background: var(--accent-ring);
  color: var(--accent-strong);
}

.toolbar .tb-input {
  flex: 1;
  height: 38px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-elevated);
  color: var(--fg);
  font-size: 12.5px;
  padding: 0 11px;
  min-width: 0;
}

.toolbar .tb-input.tb-tn {
  flex: 1.3;
}

.toolbar .error-text {
  text-align: left;
  flex: none;
}

.toolbar .error-text:empty {
  display: none;
}
```

- [ ] **Step 3: 截图核对 idle 工具条**

dev server 热更新后，chrome-devtools 截图（760×800）。核对：标题栏下方是一行工具条（📁 源文件夹格子 + 选择键 / 用户名 / TN / 开始重命名按钮），无错位；下方仍是旧的使用说明（本任务未动）。

- [ ] **Step 4: Commit**

```bash
git add src/index.html src/style.css
git commit -m "feat(ui): 表单改为顶部常驻工具条"
```

---

## Task 3: 移除「返回说明」按钮

**Files:**
- Modify: `src/index.html:41-49`（panel-header）
- Modify: `src/main.js`（移除 `$clearBtn` 声明、事件、setRunning 引用）
- Modify: `src/style.css:298-301`（移除 `#clearLogBtn` 隐藏规则）

- [ ] **Step 1: 从 panel-header 移除 clearLogBtn**

把 `src/index.html` 第 42–49 行 `<header class="panel-header"> … </header>` 替换为：

```html
        <header class="panel-header">
          <h2 class="panel-title panel-title-idle">使用说明</h2>
          <h2 class="panel-title panel-title-run">执行结果</h2>
          <div class="panel-actions">
            <button id="openFolderBtn" class="ghost ghost-accent" type="button" hidden title="在访达中打开输出文件夹">打开文件夹</button>
          </div>
        </header>
```

- [ ] **Step 2: 移除 main.js 中的 $clearBtn**

在 `src/main.js` 删除三处：

1. 第 14 行声明：`const $clearBtn = document.getElementById("clearLogBtn");`
2. 第 50–54 行事件：
```js
$clearBtn.addEventListener("click", () => {
  if (running) return;
  resetTable();
  setMode("idle");
});
```
3. `setRunning` 函数内第 88 行：`$clearBtn.disabled = flag;`

- [ ] **Step 3: 移除 CSS 中失效的 clearLogBtn 规则**

删除 `src/style.css` 第 298–301 行：
```css
.panel[data-mode="idle"] #clearLogBtn {
  visibility: hidden;
  pointer-events: none;
}
```

- [ ] **Step 4: 截图 + 控制台核对无报错**

dev 热更新后，`list_console_messages` 确认无 `$clearBtn is null` 之类报错；截图确认 panel 头部仅剩「使用说明/执行结果」标题与「打开文件夹」按钮。

- [ ] **Step 5: Commit**

```bash
git add src/index.html src/main.js src/style.css
git commit -m "refactor(ui): 移除「返回说明」按钮，工具条常驻替代"
```

---

## Task 4: 汇总区重构（大金额 + 4 统计列）

**Files:**
- Modify: `src/index.html:51-57`（`#summaryBar`）
- Modify: `src/style.css:422-475`（`.summary-bar` 相关段）
- Modify: `src/main.js`（`renderStats`）

- [ ] **Step 1: 简化 summaryBar HTML（统计交给 JS 生成）**

把 `src/index.html` 中 `<div id="summaryBar" …> … </div>` 整段替换为：

```html
        <div id="summaryBar" class="summary-bar">
          <div class="summary-total">
            <span class="summary-total-label">价税合计总和</span>
            <span id="summaryAmount" class="summary-total-value">¥0.00</span>
          </div>
          <div id="summaryStats" class="summary-stats"></div>
        </div>
```

- [ ] **Step 2: 重构 summary-bar 样式**

把 `src/style.css` 的 `/* ---------- Summary bar ---------- */` 整段（`.summary-bar` 到 `.summary-stats .stat-warn`，约第 422–475 行）替换为：

```css
/* ---------- Summary bar ---------- */

.summary-bar {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  background: var(--surface);
  border: 1px solid var(--hairline);
  border-radius: 8px;
}

.summary-total {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.summary-total-label {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--faint);
}

.summary-total-value {
  font-size: 32px;
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1;
  color: var(--fg);
  font-variant-numeric: tabular-nums;
}

.summary-stats {
  display: flex;
  gap: 18px;
  text-align: right;
}

.summary-stat {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.summary-stat .st-num {
  font-size: 18px;
  font-weight: 650;
  line-height: 1.1;
  font-variant-numeric: tabular-nums;
  color: var(--fg);
}

.summary-stat .st-label {
  font-size: 11px;
  color: var(--faint);
}

.summary-stat.is-fail .st-num {
  color: var(--error);
}

.summary-stat.is-warn .st-num {
  color: var(--warn);
}
```

- [ ] **Step 3: 重写 renderStats 生成 4 统计列**

把 `src/main.js` 的 `renderStats` 函数整体替换为：

```js
function renderStats({ count, success, failed, missing }) {
  const cols = [
    { num: count, label: "总张数" },
    { num: success, label: "成功" },
    { num: failed, label: "失败", cls: failed > 0 ? "is-fail" : null },
    { num: missing, label: "未识别金额", cls: missing > 0 ? "is-warn" : null },
  ];
  $summaryStats.replaceChildren();
  for (const c of cols) {
    const col = document.createElement("div");
    col.className = "summary-stat" + (c.cls ? ` ${c.cls}` : "");
    const num = document.createElement("span");
    num.className = "st-num";
    num.textContent = String(c.num);
    const label = document.createElement("span");
    label.className = "st-label";
    label.textContent = c.label;
    col.append(num, label);
    $summaryStats.append(col);
  }
}
```

> `renderStats` 的调用方（`resetTable` 传 `acc`、`renderRow` 传 `acc`、`updateSummary` 传 `{count,success,failed,missing}`）签名不变，无需改动。

- [ ] **Step 4: 注入 mock 数据截图核对汇总**

dev 热更新后，用附录 A 脚本注入 run 态数据，截图核对：左侧大金额 `¥983.01`、右侧 4 列统计（总张数 13 / 成功 12 / 失败 1 红 / 未识别金额 1 橙）对齐美观。

- [ ] **Step 5: Commit**

```bash
git add src/index.html src/style.css src/main.js
git commit -m "feat(ui): 汇总区重构为大金额 + 4 统计列"
```

---

## Task 5: 进度条仅运行时显示

**Files:**
- Modify: `src/main.js`（`updateProgress`、`updateSummary`、`runRename` 进度初始化）
- Modify: `src/style.css:327-333`（`.progress-text` min-width）

- [ ] **Step 1: 进度文案改「处理中 N / 总」**

把 `src/main.js` 的 `updateProgress` 函数替换为：

```js
function updateProgress(cur, total) {
  if (total > 0) {
    $progressTrack.hidden = false;
    $progressFill.style.width = `${((cur / total) * 100).toFixed(1)}%`;
    $progressText.textContent = `处理中 ${cur} / ${total}`;
  }
}
```

- [ ] **Step 2: 完成时隐藏进度条**

在 `src/main.js` 的 `updateSummary` 函数**末尾**（`if (summary.total === 0) { … }` 块之后、函数右括号之前）加一行：

```js
  $progressTrack.hidden = true;
}
```

即完成校准后移除进度条（去掉满进度条冗余）。

- [ ] **Step 3: runRename 进度初始文案对齐**

把 `src/main.js` `runRename` 中这三行：
```js
  $progressTrack.hidden = false;
  $progressFill.style.width = "0%";
  $progressText.textContent = "0 / 0";
```
替换为：
```js
  $progressTrack.hidden = false;
  $progressFill.style.width = "0%";
  $progressText.textContent = "处理中 0 / 0";
```

- [ ] **Step 4: 进度文字加宽以容纳「处理中」**

把 `src/style.css` `.progress-text` 的 `min-width: 56px;` 改为 `min-width: 84px;`。

- [ ] **Step 5: 截图核对 running 与完成两态**

注入 mock（附录 A）但**保留** `progressTrack` 显示并设 `进度条 60%、处理中 8 / 13` → 截图（进度条在）；再调用 `updateSummary`（附录 A 第 2 段）→ 截图（进度条消失，汇总定格）。

- [ ] **Step 6: Commit**

```bash
git add src/main.js src/style.css
git commit -m "feat(ui): 进度条仅运行时显示，完成后隐藏"
```

---

## Task 6: 表格/说明卡浅色细节 + 文案

**Files:**
- Modify: `src/index.html`（helpView 第 3 步文案）
- Modify: `src/style.css`（表格 hover；openFolderBtn 浅蓝实心）

- [ ] **Step 1: 更新使用说明第 3 步文案**

把 `src/index.html` helpView 内：
```html
            <li>点击 <strong>开始重命名</strong>，等待日志结束</li>
```
替换为：
```html
            <li>点击 <strong>开始重命名</strong>，下方实时显示结果与价税合计总和</li>
```

- [ ] **Step 2: 表格行 hover + 失败行 hover 保红**

在 `src/style.css` 的 `.result-table tbody tr:last-child td { border-bottom: none; }` 之后新增：

```css
.result-table tbody tr:hover td {
  background: var(--surface-elevated);
}

.result-table tbody tr.row-failed:hover td {
  background: var(--error-soft);
}
```

- [ ] **Step 3: 「打开文件夹」改浅蓝实心按钮**

把 `src/style.css` 的 `button.ghost-accent` 与其 hover 两段替换为：

```css
button.ghost-accent {
  color: var(--accent);
  background: var(--accent-soft);
  border-color: transparent;
  font-weight: 550;
}

button.ghost-accent:hover:not(:disabled) {
  background: var(--accent-ring);
  color: var(--accent-strong);
}
```

- [ ] **Step 4: 截图核对 idle 与 run**

idle 截图核对说明卡文案与浅色观感；注入 mock（附录 A）截图核对表格 hover、失败行红底、长文件名省略、20 位发票号不挤压金额列、「打开文件夹」浅蓝实心。

- [ ] **Step 5: Commit**

```bash
git add src/index.html src/style.css
git commit -m "style(ui): 表格 hover、说明文案与打开文件夹按钮细节"
```

---

## Task 7: 生产构建 + 人工验证

**Files:** 无（验证任务）

- [ ] **Step 1: 生产构建**

Run: `npm run build`
Expected: `vite build` 成功，无报错（类似 `✓ built in …`）。

- [ ] **Step 2: 后端回归未受影响**

Run: `cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5`
Expected: `test result: ok. 40 passed`（纯前端改动不应影响后端）。

- [ ] **Step 3: 提交构建产物外的收尾（如有）**

若 Task 1–6 已逐一 commit，本步无新增改动；`git status` 应仅余未提交的发票金额功能改动（与本次无关，勿混入）。

- [ ] **Step 4: 交给用户人工验证**

提示用户运行 `npm run tauri dev`，在真实桌面窗口核对：
- 三态：idle 首屏 / running 进度条「处理中 N/总」/ 完成进度条消失
- 真实样本跑一遍，汇总显示 `¥983.01`，统计正确
- 失败行红底 + ✗ hover 看原因；长文件名省略 + hover 看全名
- 空结果（无 PDF）提示；源目录不存在时工具条下方红色错误条
- 深色模式的 Mac 上确认 app 仍为浅色

---

## 附录 A：run 态核对注入脚本（chrome-devtools `evaluate_script`）

**第 1 段——注入结果数据（running 视图）：**

```js
() => {
  const panel = document.querySelector('.panel');
  panel.dataset.mode = 'run';
  const rows = [
    [1,'26117000000567012345.pdf','26117000000567012345','¥93.33','success'],
    [2,'发票_京东商城_20260512094533.pdf','26322000000893295511','¥439.60','success'],
    [3,'滴滴出行行程报销电子发票.pdf','26117000001234567890','¥48.10','success'],
    [4,'scan_0001_无文本层扫描件.pdf',null,null,'failed'],
    [5,'美团商旅_2026_发票.pdf','26117000009876543210','¥12.88','success'],
    [6,'aliyun_invoice_april.pdf','26500000000112233445','¥260.00','success'],
    [7,'高德打车发票.pdf','26117000005544332211','¥18.50','success'],
  ];
  const body = document.getElementById('resultBody');
  body.replaceChildren();
  for (const [i,name,inv,amt,st] of rows) {
    const tr = document.createElement('tr');
    if (st==='failed') tr.className='row-failed';
    const mk=(c,t,ti)=>{const td=document.createElement('td');td.className=c;td.textContent=t;if(ti)td.title=ti;return td;};
    tr.append(mk('cell-idx',String(i)));
    tr.append(mk('cell-name',name,name));
    tr.append(mk('cell-invoice',inv||'—',inv||''));
    tr.append(mk(amt?'cell-amount':'cell-amount missing',amt||'—'));
    const sc=document.createElement('td');sc.className='cell-status';
    const b=document.createElement('span');b.className='status-badge '+(st==='success'?'ok':'fail');b.textContent=st==='success'?'✓':'✗';
    sc.append(b);tr.append(sc);
    body.append(tr);
  }
  // 进度条 running 态
  const pt=document.getElementById('progressTrack');pt.hidden=false;
  document.getElementById('progressFill').style.width='60%';
  document.getElementById('progressText').textContent='处理中 8 / 13';
  // 汇总
  document.getElementById('summaryAmount').textContent='¥983.01';
  const stats=document.getElementById('summaryStats');stats.replaceChildren();
  for (const [n,l,cls] of [[13,'总张数',''],[12,'成功',''],[1,'失败','is-fail'],[1,'未识别金额','is-warn']]) {
    const col=document.createElement('div');col.className='summary-stat'+(cls?' '+cls:'');
    const num=document.createElement('span');num.className='st-num';num.textContent=String(n);
    const lab=document.createElement('span');lab.className='st-label';lab.textContent=l;
    col.append(num,lab);stats.append(col);
  }
  document.getElementById('openFolderBtn').hidden=false;
  return 'injected';
}
```

**第 2 段——模拟完成（隐藏进度条）：**

```js
() => { document.getElementById('progressTrack').hidden = true; return 'done'; }
```

---

## Self-Review

- **Spec coverage**：浅色固定 + 删 dark media query（T1）；工具条 + 文件夹选择键（T2）；移除返回说明（T3）；汇总大金额 + 4 统计（T4）；进度条 running 显示/完成隐藏（T5）；表格 hover / 说明文案 / 打开文件夹浅蓝（T6）；错误条 = `formError` 移工具条下方 + `:empty` 隐藏（T2）；构建 + 人工验证（T7）。全部覆盖。
- **Placeholder scan**：各步均含完整代码，无 TBD/TODO。
- **Type/接口一致性**：`renderStats({count,success,failed,missing})` 与三处调用方（`resetTable`/`renderRow` 传 `acc`，`updateSummary` 传同名字段）一致；id 全保留，main.js 选择器不变；`updateSummary` 末尾新增 `$progressTrack.hidden = true` 与 T5 文案改动不冲突。
