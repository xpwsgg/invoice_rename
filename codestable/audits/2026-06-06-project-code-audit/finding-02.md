---
doc_type: audit-finding
audit: 2026-06-06-project-code-audit
finding_id: "security-02"
nature: security
severity: P2
confidence: medium
suggested_action: cs-refactor
status: open
---

# Finding 02：Tauri 安全边界偏宽

## 速答

应用当前关闭 CSP，并把默认 capability 绑定到所有窗口。现阶段前端不渲染远程内容，也没有发现 `innerHTML` 注入点，所以不是立即可利用漏洞；但这会降低后续引入新窗口、远程资源或更复杂 UI 时的防护余量。

## 关键证据

- `src-tauri/tauri.conf.json:25` — `"security": {`：存在显式安全配置块。
- `src-tauri/tauri.conf.json:26` — `"csp": null`：CSP 被关闭。
- `src-tauri/capabilities/default.json:5` — `"windows": ["*"]`：该 capability 适用于所有窗口，而不是只绑定 `main`。
- `src-tauri/capabilities/default.json:6` — `"permissions": [`：权限集中配置在默认 capability 下。
- `src-tauri/capabilities/default.json:7` — `"core:default"`：默认 core 权限集较宽。
- `src-tauri/capabilities/default.json:8` — `"dialog:default"`：默认 dialog 权限包含打开、保存、消息等权限；当前前端只需要目录选择。

## 影响

如果后续增加新窗口、预览页面、帮助页面或加载远程资源，`windows: ["*"]` 会让这些窗口默认继承当前能力；`csp: null` 也无法提供脚本来源和内联脚本约束。当前代码使用 `textContent` 渲染表格，风险主要是未来演进时的安全债务。

## 修复方向

为 `main` 窗口绑定最小 capability；把 `dialog:default` 收窄到实际需要的 open 权限；配置与当前 Vite/Tauri bundle 兼容的 CSP。

## 建议动作

`cs-refactor`，因为这是安全硬化和权限收敛，不要求改变用户可见行为。
