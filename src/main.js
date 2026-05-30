import { invoke, Channel } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const FORBIDDEN = /[\/\\:*?"<>|]/;
const USER_NAME_KEY = "pdfRename.userName";
const PROGRESS_RE = /\[(\d+)\/(\d+)\]/;
const SUMMARY_RE = /^完成：成功\s+(\d+)，失败\s+(\d+)/;

const $sourceDir = document.getElementById("sourceDir");
const $userName = document.getElementById("userName");
const $tracking = document.getElementById("trackingNumber");
const $pickBtn = document.getElementById("pickDirBtn");
const $runBtn = document.getElementById("runBtn");
const $err = document.getElementById("formError");
const $log = document.getElementById("logBox");
const $panel = document.querySelector(".panel");
const $clearBtn = document.getElementById("clearLogBtn");
const $openFolderBtn = document.getElementById("openFolderBtn");
const $progressTrack = document.getElementById("progressTrack");
const $progressFill = document.getElementById("progressFill");
const $progressText = document.getElementById("progressText");

const savedUserName = localStorage.getItem(USER_NAME_KEY);
if (savedUserName) {
  $userName.value = savedUserName;
}

let running = false;
let lastOutputDir = null;

$pickBtn.addEventListener("click", async () => {
  if (running) return;
  const picked = await open({ directory: true, multiple: false });
  if (typeof picked === "string" && picked.length > 0) {
    $sourceDir.value = picked;
    showError("");
  }
});

$runBtn.addEventListener("click", runRename);

$clearBtn.addEventListener("click", () => {
  if (running) return;
  clearLog();
  setMode("idle");
});

$openFolderBtn.addEventListener("click", async () => {
  if (!lastOutputDir) return;
  try {
    await invoke("open_folder", { path: lastOutputDir });
  } catch (e) {
    appendLog({
      ts: new Date().toTimeString().slice(0, 8),
      level: "error",
      message: typeof e === "string" ? e : JSON.stringify(e),
    });
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
  $clearBtn.disabled = flag;
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

function clearLog() {
  $log.replaceChildren();
}

function classifyLevel(entry) {
  const lvl = (entry.level || "info").toLowerCase();
  if (lvl === "info" && SUMMARY_RE.test(entry.message)) {
    const m = entry.message.match(SUMMARY_RE);
    if (m) return +m[2] === 0 ? "success" : "summary-fail";
  }
  return lvl;
}

function appendLog(entry) {
  const level = classifyLevel(entry);

  const line = document.createElement("div");
  line.className = `log-line ${level}`;

  const lvlBadge = document.createElement("span");
  lvlBadge.className = "log-level";
  lvlBadge.textContent =
    level === "success" ? "DONE" : level === "summary-fail" ? "FAIL" : level.toUpperCase();

  const ts = document.createElement("span");
  ts.className = "log-ts";
  ts.textContent = entry.ts || "";

  const msg = document.createElement("span");
  msg.className = "log-msg";
  msg.textContent = entry.message;

  line.append(lvlBadge, ts, msg);
  $log.appendChild(line);
  $log.scrollTop = $log.scrollHeight;

  const m = entry.message.match(PROGRESS_RE);
  if (m) {
    const cur = +m[1];
    const total = +m[2];
    if (total > 0) {
      $progressTrack.hidden = false;
      $progressFill.style.width = `${((cur / total) * 100).toFixed(1)}%`;
      $progressText.textContent = `${cur} / ${total}`;
    }
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

  clearLog();
  setMode("run");
  $progressTrack.hidden = false;
  $progressFill.style.width = "0%";
  $progressText.textContent = "0 / 0";
  hideOpenFolderBtn();
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
    if (summary && summary.outputDir) {
      showOpenFolderBtn(summary.outputDir);
    }
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
