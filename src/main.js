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

function updateProgress(cur, total) {
  if (total > 0) {
    $progressTrack.hidden = false;
    $progressFill.style.width = `${((cur / total) * 100).toFixed(1)}%`;
    $progressText.textContent = `${cur} / ${total}`;
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
  }
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

  resetTable();
  setMode("run");
  $progressTrack.hidden = false;
  $progressFill.style.width = "0%";
  $progressText.textContent = "0 / 0";
  hideOpenFolderBtn();
  setRunning(true);

  const channel = new Channel();
  channel.onmessage = (row) => renderRow(row);

  try {
    const summary = await invoke("rename_pdfs", {
      sourceDir: $sourceDir.value.trim(),
      userName: $userName.value.trim(),
      trackingNumber: $tracking.value.trim(),
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
