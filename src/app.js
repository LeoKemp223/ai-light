const tauriEvent = window.__TAURI__?.event;
const tauriCore = window.__TAURI__?.core;
const currentWindow =
  window.__TAURI__?.window?.getCurrentWindow?.() ??
  window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow?.();

let lights = [];
const lightElements = new Map();
let lastWindowSize = { width: 0, height: 0 };
let resizeFrame = 0;
const WINDOW_GUTTER_X = 8;
const WINDOW_GUTTER_Y = 26;
const WINDOW_PAINT_OVERFLOW_X_PER_LIGHT = 16;
const MENU_EDGE_GUTTER = 12;

const container = document.getElementById("lights-container");
const menu = document.getElementById("menu");
const appHandle = createAppHandle();
container.appendChild(appHandle);

tauriEvent?.listen("state-changed", (event) => {
  lights = Array.isArray(event.payload) ? event.payload : [];
  render();
});

document.addEventListener("click", (event) => {
  if (!menu.contains(event.target)) {
    hideMenu();
  }
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    hideMenu();
  }
});

let isDragging = false;
let dragStart = null;

document.addEventListener("pointerdown", async (event) => {
  if (!shouldStartDrag(event)) return;

  try {
    await currentWindow?.startDragging?.();
    return;
  } catch {}

  isDragging = true;
  const pos = await currentWindow?.outerPosition();
  dragStart = {
    mouseX: event.screenX,
    mouseY: event.screenY,
    winX: pos?.x ?? 0,
    winY: pos?.y ?? 0,
  };
  try {
    event.target.setPointerCapture(event.pointerId);
  } catch {}
});

document.addEventListener("pointermove", async (event) => {
  if (!isDragging || !dragStart || !currentWindow) return;

  const dx = event.screenX - dragStart.mouseX;
  const dy = event.screenY - dragStart.mouseY;

  try {
    const PhysicalPosition = window.__TAURI__?.dpi?.PhysicalPosition;
    if (PhysicalPosition) {
      await currentWindow.setPosition(
        new PhysicalPosition(dragStart.winX + dx, dragStart.winY + dy),
      );
    }
  } catch {}
});

document.addEventListener("pointerup", () => {
  isDragging = false;
  dragStart = null;
});

function render() {
  const visibleProjectIds = new Set(lights.map((light) => light.project_id));
  appHandle.hidden = lights.length > 0;

  for (const light of lights) {
    let element = lightElements.get(light.project_id);
    if (!element) {
      element = createProjectLight(light);
      lightElements.set(light.project_id, element);
      container.appendChild(element);
    }

    updateProjectLight(element, light);
  }

  for (const [projectId, element] of lightElements) {
    if (!visibleProjectIds.has(projectId)) {
      element.remove();
      lightElements.delete(projectId);
    }
  }

  scheduleWindowResize();
}

function createAppHandle() {
  const root = document.createElement("section");
  root.className = "app-handle";
  root.title = "AI Light";
  root.textContent = "AI";

  root.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    showMenu(event.clientX, event.clientY, [
      ["Settings", () => safeInvoke("open_settings")],
      ["Quit", () => safeInvoke("quit_app")],
    ]);
  });

  return root;
}

function createProjectLight(lightState) {
  const root = createLightElement({
    label: lightState.project_label,
    status: lightState.status,
    title: tooltipFor(lightState),
  });
  root.dataset.projectId = lightState.project_id;

  root.addEventListener("click", () => {
    const projectId = root.dataset.projectId;
    const status = root.dataset.status;
    if (status === "Error" || status === "Done") {
      safeInvoke("confirm_light", { projectId });
    }
  });

  root.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    const projectId = root.dataset.projectId;
    showMenu(event.clientX, event.clientY, [
      ["Open", () => safeInvoke("open_project", { projectId })],
      ["Copy Path", () => copyProjectPath(projectId)],
      ["Settings", () => safeInvoke("open_settings")],
      ["Remove", () => safeInvoke("remove_light", { projectId })],
    ]);
  });

  updateProjectLight(root, lightState);
  return root;
}

function createLightElement({ label, status, title, standby = false }) {
  const root = document.createElement("section");
  root.className = `traffic-light${standby ? " standby" : ""}`;
  root.title = title;
  root.dataset.status = status;

  const housing = document.createElement("div");
  housing.className = "light-housing";

  housing.appendChild(createLamp("red", status === "Error"));
  housing.appendChild(createLamp("yellow", status === "Working"));
  housing.appendChild(createLamp("green", status === "Done"));

  const labelEl = document.createElement("div");
  labelEl.className = "light-label";
  labelEl.textContent = label || "unknown";

  root.append(labelEl, housing);
  return root;
}

function updateProjectLight(root, lightState) {
  root.dataset.projectId = lightState.project_id;
  root.dataset.status = lightState.status;
  root.title = tooltipFor(lightState);
  root.classList.toggle(
    "is-actionable",
    lightState.status === "Error" || lightState.status === "Done",
  );

  const label = root.querySelector(".light-label");
  if (label) {
    label.textContent = lightState.project_label || "unknown";
  }

  root.querySelector(".lamp.red")?.classList.toggle("on", lightState.status === "Error");
  root
    .querySelector(".lamp.yellow")
    ?.classList.toggle("on", lightState.status === "Working");
  root.querySelector(".lamp.green")?.classList.toggle("on", lightState.status === "Done");
}

function createLamp(color, isOn) {
  const lamp = document.createElement("div");
  lamp.className = `lamp ${color}${isOn ? " on" : ""}`;
  return lamp;
}

function tooltipFor(lightState) {
  const parts = [
    lightState.project_label || lightState.project_id,
    lightState.status || "Idle",
  ];

  if (lightState.last_tool_call) {
    parts.push(lightState.last_tool_call);
  }

  return parts.join("\n");
}

function showMenu(x, y, items) {
  menu.replaceChildren();

  for (const [label, action, className] of [["Close", hideMenu, "menu-close"], ...items]) {
    const item = document.createElement("button");
    item.type = "button";
    item.textContent = label;
    if (className) {
      item.classList.add(className);
    }
    item.addEventListener("click", () => {
      hideMenu();
      action();
    });
    menu.appendChild(item);
  }

  menu.hidden = false;
  const { innerWidth, innerHeight } = window;
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(
    MENU_EDGE_GUTTER,
    Math.min(x, innerWidth - rect.width - MENU_EDGE_GUTTER),
  )}px`;
  menu.style.top = `${Math.max(
    MENU_EDGE_GUTTER,
    Math.min(y, innerHeight - rect.height - MENU_EDGE_GUTTER),
  )}px`;
  scheduleWindowResize();
}

function hideMenu() {
  menu.hidden = true;
  scheduleWindowResize();
}

function scheduleWindowResize() {
  if (resizeFrame) {
    cancelAnimationFrame(resizeFrame);
  }

  resizeFrame = requestAnimationFrame(resizeWindowToContent);
}

async function resizeWindowToContent() {
  resizeFrame = 0;
  if (!currentWindow) return;

  const bodyStyle = getComputedStyle(document.body);
  const paddingX =
    parseFloat(bodyStyle.paddingLeft) + parseFloat(bodyStyle.paddingRight);
  const paddingY =
    parseFloat(bodyStyle.paddingTop) + parseFloat(bodyStyle.paddingBottom);

  const contentSize = measureVisibleContent();
  let width = Math.ceil(
    contentSize.width +
      paddingX +
      WINDOW_GUTTER_X +
      contentSize.count * WINDOW_PAINT_OVERFLOW_X_PER_LIGHT,
  );
  let height = Math.ceil(contentSize.height + paddingY + WINDOW_GUTTER_Y);

  if (!menu.hidden) {
    const menuRect = menu.getBoundingClientRect();
    width = Math.max(width, Math.ceil(menuRect.right + MENU_EDGE_GUTTER));
    height = Math.max(height, Math.ceil(menuRect.bottom + MENU_EDGE_GUTTER));
  }

  width = Math.max(72, width);
  height = Math.max(76, height);

  if (lastWindowSize.width === width && lastWindowSize.height === height) {
    return;
  }

  try {
    await tauriCore?.invoke("resize_main_window", { width, height });
    lastWindowSize = { width, height };
    return;
  } catch (error) {
    console.debug("resizeWindowToContent", error);
  }

  const LogicalSize = window.__TAURI__?.dpi?.LogicalSize;
  if (!LogicalSize) return;

  try {
    await currentWindow.setSize(new LogicalSize(width, height));
    lastWindowSize = { width, height };
  } catch (error) {
    console.debug("resizeWindowToContent fallback", error);
  }
}

function measureVisibleContent() {
  const children = [...container.children].filter((child) => !child.hidden);
  if (children.length === 0) {
    return { width: 0, height: 0, count: 0 };
  }

  const containerStyle = getComputedStyle(container);
  const gap = parseFloat(containerStyle.columnGap || containerStyle.gap) || 0;
  const width =
    children.reduce((sum, child) => sum + child.offsetWidth, 0) +
    gap * Math.max(0, children.length - 1);
  const height = Math.max(...children.map((child) => child.offsetHeight));

  return { width, height, count: children.length };
}

function shouldStartDrag(event) {
  if (event.button !== 0 || !menu.hidden) {
    return false;
  }

  if (event.target.closest(".menu, button")) {
    return false;
  }

  return Boolean(event.target.closest("#lights-container, .traffic-light"));
}

async function safeInvoke(command, payload) {
  try {
    return await tauriCore?.invoke(command, payload);
  } catch (error) {
    console.debug(command, error);
    return undefined;
  }
}

async function refreshLights() {
  const nextLights = await safeInvoke("get_lights");
  if (Array.isArray(nextLights)) {
    lights = nextLights;
    render();
  }
}

async function copyProjectPath(projectId) {
  const path = await safeInvoke("copy_path", { projectId });
  if (path && navigator.clipboard) {
    await navigator.clipboard.writeText(path);
  }
}

async function showDiagnostics() {
  const diagnostics = await safeInvoke("get_diagnostics");
  if (!diagnostics) return;

  const text = [
    "AI Light Diagnostics",
    "",
    `Config: ${diagnostics.config_dir}`,
    `Runtime: ${diagnostics.runtime_path}`,
    `Lock: ${diagnostics.lock_path}`,
    `Log: ${diagnostics.log_path}`,
    `Claude settings: ${diagnostics.claude_settings_path}`,
    `Hook binary: ${diagnostics.hook_binary_path}`,
    `Codex sessions: ${diagnostics.codex_sessions_path}`,
    "",
    `Hooks installed: ${diagnostics.hooks_installed}`,
    `Hook binary exists: ${diagnostics.hook_binary_exists}`,
    `Runtime exists: ${diagnostics.runtime_exists}`,
    `Light count: ${diagnostics.light_count}`,
    "",
    "Recent log:",
    diagnostics.recent_log || "(empty)",
  ].join("\n");

  if (navigator.clipboard) {
    await navigator.clipboard.writeText(text);
  }
  alert(text);
}

refreshLights();
scheduleWindowResize();
window.setInterval(refreshLights, 1000);
