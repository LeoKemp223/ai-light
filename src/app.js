const tauriEvent = window.__TAURI__?.event;
const tauriCore = window.__TAURI__?.core;
const currentWindow =
  window.__TAURI__?.window?.getCurrentWindow?.() ??
  window.__TAURI__?.webviewWindow?.getCurrentWebviewWindow?.();

let lights = [];
const lightElements = new Map();

const container = document.getElementById("lights-container");
const menu = document.getElementById("menu");
const standbyLight = createStandbyLight();
container.appendChild(standbyLight);

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
}

function createStandbyLight() {
  const root = createLightElement({
    label: "AI Light",
    status: "Idle",
    title: "AI Light",
    standby: true,
  });

  root.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    showMenu(event.clientX, event.clientY, [
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
      ["Logs", () => safeInvoke("open_session_logs", { projectId })],
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

  root.append(housing, labelEl);
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

  for (const [label, action] of items) {
    const item = document.createElement("button");
    item.type = "button";
    item.textContent = label;
    item.addEventListener("click", () => {
      hideMenu();
      action();
    });
    menu.appendChild(item);
  }

  menu.hidden = false;
  const { innerWidth, innerHeight } = window;
  const rect = menu.getBoundingClientRect();
  menu.style.left = `${Math.min(x, innerWidth - rect.width - 4)}px`;
  menu.style.top = `${Math.min(y, innerHeight - rect.height - 4)}px`;
}

function hideMenu() {
  menu.hidden = true;
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

refreshLights();
window.setInterval(refreshLights, 1000);
