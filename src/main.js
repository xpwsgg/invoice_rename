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
const $log = document.getElementById("logBox");

const savedUserName = localStorage.getItem(USER_NAME_KEY);
if (savedUserName) {
  $userName.value = savedUserName;
}

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
  localStorage.setItem(USER_NAME_KEY, $userName.value.trim());
  clearLog();
  setRunning(true);

  const channel = new Channel();
  channel.onmessage = (msg) => appendLog(msg);

  try {
    await invoke("rename_pdfs", {
      sourceDir: $sourceDir.value.trim(),
      userName: $userName.value.trim(),
      trackingNumber: $tracking.value.trim(),
      onLog: channel,
    });
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
