const tauriEvent = window.__TAURI__?.event;
const tauriCore = window.__TAURI__?.core;

let lights = [];

const container = document.getElementById("lights-container");
const menu = document.getElementById("menu");

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

function render() {
  container.replaceChildren(createStandbyLight());

  for (const light of lights) {
    container.appendChild(createProjectLight(light));
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
      ["Settings", () => safeInvoke("open_settings")],
      ["Pause", () => safeInvoke("pause_monitoring")],
      ["Resume", () => safeInvoke("resume_monitoring")],
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

  if (lightState.status === "Error" || lightState.status === "Done") {
    root.classList.add("is-actionable");
  }

  root.addEventListener("click", () => {
    if (lightState.status === "Error" || lightState.status === "Done") {
      safeInvoke("confirm_light", { projectId: lightState.project_id });
    }
  });

  root.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    showMenu(event.clientX, event.clientY, [
      ["Open", () => safeInvoke("open_project", { projectId: lightState.project_id })],
      ["Copy Path", () => copyProjectPath(lightState.project_id)],
      ["Logs", () => safeInvoke("open_session_logs", { projectId: lightState.project_id })],
      ["Remove", () => safeInvoke("remove_light", { projectId: lightState.project_id })],
    ]);
  });

  return root;
}

function createLightElement({ label, status, title, standby = false }) {
  const root = document.createElement("section");
  root.className = `traffic-light${standby ? " standby" : ""}`;
  root.title = title;
  root.dataset.status = status;

  const housing = document.createElement("div");
  housing.className = "light-housing";
  housing.dataset.tauriDragRegion = "";

  housing.appendChild(createLamp("red", status === "Error"));
  housing.appendChild(createLamp("yellow", status === "Working"));
  housing.appendChild(createLamp("green", status === "Done"));

  const labelEl = document.createElement("div");
  labelEl.className = "light-label";
  labelEl.textContent = label || "unknown";

  root.append(housing, labelEl);
  return root;
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

render();
refreshLights();
window.setInterval(refreshLights, 1000);
