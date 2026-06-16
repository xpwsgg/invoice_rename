import { invoke, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const FORBIDDEN = /[\/\\:*?"<>|]/;
const USER_NAME_KEY = "pdfRename.userName";

const $sourceDir = document.getElementById("sourceDir");
const $userName = document.getElementById("userName");
const $tracking = document.getElementById("trackingNumber");
const $pickBtn = document.getElementById("pickDirBtn");
const $runBtn = document.getElementById("runBtn");
const $err = document.getElementById("formError");
const $panel = document.querySelector(".panel");
const $openFolderBtn = document.getElementById("openFolderBtn");
const $progressTrack = document.getElementById("progressTrack");
const $progressFill = document.getElementById("progressFill");
const $progressText = document.getElementById("progressText");
const $resultTable = document.getElementById("resultTable");
const $resultBody = document.getElementById("resultBody");
const $summaryAmount = document.getElementById("summaryAmount");
const $summaryStats = document.getElementById("summaryStats");

const savedUserName = localStorage.getItem(USER_NAME_KEY);
if (savedUserName) {
  $userName.value = savedUserName;
}

let running = false;
let lastOutputDir = null;
let scannedFiles = []; // 缓存扫描结果

// 实时累加器：边收行边更新顶部汇总，结束后由后端 summary 校准。
let acc = freshAcc();

function freshAcc() {
  return { totalCents: 0, count: 0, success: 0, failed: 0, missing: 0 };
}

$pickBtn.addEventListener("click", async () => {
  if (running) return;
  const picked = await open({ directory: true, multiple: false });
  if (typeof picked === "string" && picked.length > 0) {
    $sourceDir.value = picked;
    showError("");
    await scanFolder(picked); // 立即扫描并显示
  }
});

$runBtn.addEventListener("click", runRename);

$openFolderBtn.addEventListener("click", async () => {
  if (!lastOutputDir) return;
  try {
    await invoke("open_folder", { path: lastOutputDir });
  } catch (e) {
    showError(typeof e === "string" ? e : JSON.stringify(e));
  }
});

function showError(msg) {
  $err.textContent = msg || "";
}

function validate() {
  const sourceDir = $sourceDir.value.trim();
  const userName = $userName.value.trim();
  const tracking = $tracking.value.trim();

  if (!sourceDir) return "请选择源文件夹";
  if (!userName) return "请输入用户名";
  if (FORBIDDEN.test(userName)) return '用户名不能包含特殊字符 / \\ : * ? " < > |';
  if (!tracking) return "请输入 Tracking Number";
  if (FORBIDDEN.test(tracking)) return 'Tracking Number 不能包含特殊字符 / \\ : * ? " < > |';
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

function setMode(mode) {
  $panel.dataset.mode = mode;
  if (mode === "idle") {
    $progressTrack.hidden = true;
    $progressTrack.classList.remove("is-done");
    $progressFill.style.width = "0%";
    $progressText.textContent = "0 / 0";
    hideOpenFolderBtn();
  }
}

function showOpenFolderBtn(outputDir) {
  lastOutputDir = outputDir;
  $openFolderBtn.hidden = false;
}

function hideOpenFolderBtn() {
  lastOutputDir = null;
  $openFolderBtn.hidden = true;
}

/// 把「分」格式化为人民币显示，与后端 format_amount 对齐：98301 -> "¥983.01"。
function formatAmount(cents) {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(cents);
  const yuan = Math.floor(abs / 100).toLocaleString("en-US");
  const frac = String(abs % 100).padStart(2, "0");
  return `¥${sign}${yuan}.${frac}`;
}

function resetTable() {
  $resultBody.replaceChildren();
  acc = freshAcc();
  $summaryAmount.textContent = "¥0.00";
  renderStats(acc);
}

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

/// 扫描文件夹：显示文件列表和总金额
async function scanFolder(sourceDir) {
  setMode("run");
  resetTable();
  $progressTrack.hidden = true;
  setRunning(true);

  try {
    const scanResult = await invoke("scan_pdfs", { sourceDir });
    scannedFiles = scanResult.files || [];

    // 显示扫描结果
    if (scanResult.total === 0) {
      const tr = document.createElement("tr");
      const td = document.createElement("td");
      td.colSpan = 6; // 增加到 6 列（包含操作列）
      td.className = "table-empty";
      td.textContent = "未找到 PDF 文件";
      tr.append(td);
      $resultBody.replaceChildren(tr);
    } else {
      // 渲染每个文件
      for (const file of scannedFiles) {
        renderScannedFile(file);
      }
    }

    // 更新汇总信息
    updateScanSummary();

  } catch (e) {
    showError(typeof e === "string" ? e : JSON.stringify(e));
    setMode("idle");
  } finally {
    setRunning(false);
  }
}

/// 渲染单个扫描的文件
function renderScannedFile(file) {
  const tr = document.createElement("tr");
  tr.dataset.fileIndex = file.index;
  if (file.parseError) tr.className = "row-warning";

  tr.append(makeCell("cell-idx", String(file.index)));
  tr.append(makeCell("cell-name", file.sourceName, file.sourceName));
  tr.append(makeCell("cell-invoice", file.invoiceNumber || "—", file.invoiceNumber || ""));

  const amountMissing = file.amountCents === null || file.amountCents === undefined;
  tr.append(
    makeCell(amountMissing ? "cell-amount missing" : "cell-amount", file.amountDisplay || "—")
  );

  const statusCell = document.createElement("td");
  statusCell.className = "cell-status";
  const badge = document.createElement("span");
  badge.className = "status-badge pending";
  badge.textContent = "待处理";
  if (file.parseError) {
    badge.className = "status-badge warn";
    badge.textContent = "⚠";
    badge.title = file.parseError;
  }
  statusCell.append(badge);
  tr.append(statusCell);

  // 添加操作列
  const actionCell = document.createElement("td");
  actionCell.className = "cell-action";
  const removeBtn = document.createElement("button");
  removeBtn.className = "btn-remove";
  removeBtn.textContent = "移除";
  removeBtn.title = "从列表中移除此文件";
  removeBtn.onclick = () => removeFile(file.index);
  actionCell.append(removeBtn);
  tr.append(actionCell);

  $resultBody.append(tr);
}

/// 从列表中移除文件
function removeFile(fileIndex) {
  // 从缓存中移除
  const removedIndex = scannedFiles.findIndex(f => f.index === fileIndex);
  if (removedIndex === -1) return;

  scannedFiles.splice(removedIndex, 1);

  // 从 DOM 中移除
  const row = $resultBody.querySelector(`tr[data-file-index="${fileIndex}"]`);
  if (row) {
    row.remove();
  }

  // 重新编号显示（但保持原始 sourceName 不变）
  let displayIndex = 1;
  Array.from($resultBody.querySelectorAll('tr')).forEach((row) => {
    const indexCell = row.querySelector('.cell-idx');
    if (indexCell && !row.querySelector('.table-empty')) {
      indexCell.textContent = String(displayIndex);
      displayIndex++;
    }
  });

  // 更新汇总
  updateScanSummary();

  // 如果没有文件了，显示空状态
  if (scannedFiles.length === 0) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 6;
    td.className = "table-empty";
    td.textContent = "未找到 PDF 文件";
    tr.append(td);
    $resultBody.replaceChildren(tr);
  }
}

/// 更新扫描汇总信息
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

function updateProgress(cur, total) {
  if (total > 0) {
    $progressTrack.hidden = false;
    $progressFill.style.width = `${((cur / total) * 100).toFixed(1)}%`;
    $progressText.textContent = `处理中 ${cur} / ${total}`;
  }
}

function makeCell(cls, text, title) {
  const td = document.createElement("td");
  td.className = cls;
  td.textContent = text;
  if (title) td.title = title;
  return td;
}

function renderRow(row) {
  const tr = document.createElement("tr");
  if (row.status === "failed") tr.className = "row-failed";

  tr.append(makeCell("cell-idx", String(row.index)));
  tr.append(makeCell("cell-name", row.sourceName, row.sourceName));
  tr.append(makeCell("cell-invoice", row.invoiceNumber || "—", row.invoiceNumber || ""));

  const amountMissing = row.amountCents === null || row.amountCents === undefined;
  tr.append(
    makeCell(amountMissing ? "cell-amount missing" : "cell-amount", row.amountDisplay || "—")
  );

  const statusCell = document.createElement("td");
  statusCell.className = "cell-status";
  const badge = document.createElement("span");
  badge.className = `status-badge ${row.status === "success" ? "ok" : "fail"}`;
  badge.textContent = row.status === "success" ? "✓" : "✗";
  if (row.note) badge.title = row.note;
  statusCell.append(badge);
  tr.append(statusCell);

  // 重命名结果不显示移除按钮
  const actionCell = document.createElement("td");
  actionCell.className = "cell-action";
  tr.append(actionCell);

  $resultBody.append(tr);
  $resultTable.scrollTop = $resultTable.scrollHeight;

  // 实时累加
  acc.count += 1;
  if (row.status === "success") acc.success += 1;
  else acc.failed += 1;
  if (amountMissing) {
    acc.missing += 1;
  } else {
    acc.totalCents += row.amountCents;
  }
  $summaryAmount.textContent = formatAmount(acc.totalCents);
  renderStats(acc);
  updateProgress(row.index, row.total);
}

function updateSummary(summary) {
  // 用后端权威汇总校准，避免实时累加的边界误差。
  $summaryAmount.textContent = formatAmount(summary.totalAmountCents);
  renderStats({
    count: summary.total,
    success: summary.success,
    failed: summary.failed,
    missing: summary.amountMissing,
  });

  if (summary.total === 0) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 5;
    td.className = "table-empty";
    td.textContent = "未找到 PDF 文件";
    tr.append(td);
    $resultBody.replaceChildren(tr);
    $progressTrack.hidden = true;
    return;
  }
  // 完成态：进度条满格变绿 + 「✓ 处理完成」，给出明确的完成时刻反馈。
  $progressTrack.hidden = false;
  $progressTrack.classList.add("is-done");
  $progressFill.style.width = "100%";
  $progressText.textContent = `✓ 处理完成 ${summary.total} / ${summary.total}`;
}

async function runRename() {
  if (running) return;
  const err = validate();
  if (err) {
    showError(err);
    return;
  }
  showError("");
  localStorage.setItem(USER_NAME_KEY, $userName.value.trim());

  // 如果还没有扫描过，先扫描
  if (scannedFiles.length === 0) {
    const sourceDir = $sourceDir.value.trim();
    await scanFolder(sourceDir);
    if (scannedFiles.length === 0) {
      return; // 扫描失败或无文件
    }
  }

  resetTable();
  setMode("run");
  $progressTrack.hidden = false;
  $progressTrack.classList.remove("is-done");
  $progressFill.style.width = "0%";
  $progressText.textContent = "处理中 0 / 0";
  hideOpenFolderBtn();
  setRunning(true);

  const channel = new Channel();
  channel.onmessage = (row) => renderRow(row);

  try {
    // 提取要处理的文件名列表
    const fileNames = scannedFiles.map(f => f.sourceName);

    const summary = await invoke("rename_pdfs", {
      sourceDir: $sourceDir.value.trim(),
      userName: $userName.value.trim(),
      trackingNumber: $tracking.value.trim(),
      fileNames: fileNames,
      onRow: channel,
    });
    if (summary) {
      updateSummary(summary);
      if (summary.outputDir) {
        showOpenFolderBtn(summary.outputDir);
      }
    }
  } catch (e) {
    showError(typeof e === "string" ? e : JSON.stringify(e));
    setMode("idle");
  } finally {
    setRunning(false);
  }
}
