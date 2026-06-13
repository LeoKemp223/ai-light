const invoke = window.__TAURI__?.core?.invoke;
const currentWindow =
  window.__TAURI__?.window?.getCurrentWindow?.() ??
  window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow?.();

const bindSelect = document.getElementById("http-bind");
const portInput = document.getElementById("http-port");
const configPath = document.getElementById("config-path");
const runtimePort = document.getElementById("runtime-port");
const statusEl = document.getElementById("status");
const saveButton = document.getElementById("save");
const closeButton = document.getElementById("close");

const claudeStatus = document.getElementById("claude-status");
const opencodeStatus = document.getElementById("opencode-status");
const reasonixStatus = document.getElementById("reasonix-status");

saveButton.addEventListener("click", saveSettings);
closeButton.addEventListener("click", () => currentWindow?.close());

document.getElementById("install-claude").addEventListener("click", () => installIntegration("claude"));
document.getElementById("remove-claude").addEventListener("click", () => removeIntegration("claude"));
document.getElementById("install-opencode").addEventListener("click", () => installIntegration("opencode"));
document.getElementById("remove-opencode").addEventListener("click", () => removeIntegration("opencode"));
document.getElementById("install-reasonix").addEventListener("click", () => installIntegration("reasonix"));
document.getElementById("remove-reasonix").addEventListener("click", () => removeIntegration("reasonix"));

loadSettings();

async function loadSettings() {
  setBusy(true);

  try {
    const config = await invoke("get_app_config");
    ensureBindOption(config.httpBind);
    bindSelect.value = config.httpBind;
    portInput.value = config.httpPort ?? "";
    configPath.textContent = config.configPath;
    runtimePort.textContent = config.runtimePort ? String(config.runtimePort) : "Not running";
    setStatus("");
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }

  refreshIntegrationStatus("claude", "check_hooks");
  refreshIntegrationStatus("opencode", "check_opencode");
  refreshIntegrationStatus("reasonix", "check_reasonix");
}

async function refreshIntegrationStatus(name, checkCommand) {
  const el = document.getElementById(`${name}-status`);
  try {
    const installed = await invoke(checkCommand);
    el.textContent = installed ? "✓ Installed" : "— Not installed";
    el.className = "integration-status " + (installed ? "installed" : "");
  } catch {
    el.textContent = "? Unknown";
  }
}

async function saveSettings() {
  const httpPort = parsePort();
  if (httpPort === false) return;

  setBusy(true);

  try {
    await invoke("save_app_config_command", {
      update: {
        httpBind: bindSelect.value,
        httpPort,
      },
    });
    setStatus("Saved. Restart AI Light to apply.");
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }
}

async function installIntegration(tool) {
  setBusy(true);

  const commands = {
    claude: { install: "install_hooks_command", remove: "remove_hooks_command", label: "Claude" },
    opencode: { install: "install_opencode_command", remove: "remove_opencode_command", label: "OpenCode" },
    reasonix: { install: "install_reasonix_command", remove: "remove_reasonix_command", label: "Reasonix" },
  };

  const cmd = commands[tool];
  const checkCommand = { claude: "check_hooks", opencode: "check_opencode", reasonix: "check_reasonix" }[tool];

  try {
    await invoke(cmd.install);
    setStatus(`${cmd.label} integration installed.`);
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
    refreshIntegrationStatus(tool, checkCommand);
  }
}

async function removeIntegration(tool) {
  const labels = { claude: "Claude Code", opencode: "OpenCode", reasonix: "Reasonix" };
  const confirmed = confirm(
    `Remove AI Light ${labels[tool]} integration?`,
  );
  if (!confirmed) return;

  setBusy(true);

  const commands = {
    claude: { remove: "remove_hooks_command" },
    opencode: { remove: "remove_opencode_command" },
    reasonix: { remove: "remove_reasonix_command" },
  };

  const cmd = commands[tool];
  const checkCommand = { claude: "check_hooks", opencode: "check_opencode", reasonix: "check_reasonix" }[tool];

  try {
    await invoke(cmd.remove);
    setStatus(`${labels[tool]} integration removed.`);
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
    refreshIntegrationStatus(tool, checkCommand);
  }
}

function parsePort() {
  const value = portInput.value.trim();
  if (!value) return null;

  const port = Number(value);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    setStatus("Port must be blank or between 1 and 65535.", true);
    portInput.focus();
    return false;
  }

  return port;
}

function ensureBindOption(value) {
  if ([...bindSelect.options].some((option) => option.value === value)) {
    return;
  }

  const option = document.createElement("option");
  option.value = value;
  option.textContent = value;
  bindSelect.appendChild(option);
}

function setBusy(isBusy) {
  const buttons = document.querySelectorAll("button");
  for (const btn of buttons) {
    btn.disabled = isBusy;
  }
  bindSelect.disabled = isBusy;
  portInput.disabled = isBusy;
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
}
