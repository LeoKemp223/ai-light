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

saveButton.addEventListener("click", saveSettings);
closeButton.addEventListener("click", () => currentWindow?.close());

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
  saveButton.disabled = isBusy;
  closeButton.disabled = isBusy;
  bindSelect.disabled = isBusy;
  portInput.disabled = isBusy;
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
}
