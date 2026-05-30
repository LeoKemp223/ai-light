const invoke = window.__TAURI__?.core?.invoke;
const currentWindow =
  window.__TAURI__?.window?.getCurrentWindow?.() ??
  window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow?.();

const allowButton = document.getElementById("allow");
const laterButton = document.getElementById("later");
const viewButton = document.getElementById("view");
const statusEl = document.getElementById("status");
const previewEl = document.getElementById("preview");

allowButton.addEventListener("click", installHooks);
laterButton.addEventListener("click", closeDialog);
viewButton.addEventListener("click", showPreview);

async function installHooks() {
  setStatus("Installing...");
  setBusy(true);

  try {
    await invoke("install_hooks_command");
    setStatus("Hooks installed.");
    window.setTimeout(closeDialog, 700);
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }
}

async function showPreview() {
  setBusy(true);

  try {
    const preview = await invoke("preview_hook_config_command");
    previewEl.textContent = preview;
    previewEl.hidden = false;
    setStatus("");
  } catch (error) {
    setStatus(String(error), true);
  } finally {
    setBusy(false);
  }
}

function closeDialog() {
  currentWindow?.close();
}

function setBusy(isBusy) {
  allowButton.disabled = isBusy;
  laterButton.disabled = isBusy;
  viewButton.disabled = isBusy;
}

function setStatus(message, isError = false) {
  statusEl.textContent = message;
  statusEl.classList.toggle("error", isError);
}
