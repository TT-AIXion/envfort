const state = {
  token: "",
  profile: "default",
  profiles: [],
  secrets: [],
  filter: "",
};

const el = {
  session: document.getElementById("session-status"),
  sessionAuth: document.getElementById("session-auth"),
  sessionHealth: document.getElementById("session-health"),
  activeProfileChip: document.getElementById("active-profile-chip"),
  message: document.getElementById("message"),
  profileSelect: document.getElementById("profile-select"),
  profileName: document.getElementById("profile-name"),
  profileCreate: document.getElementById("profile-create"),
  profileDelete: document.getElementById("profile-delete"),
  refreshSecrets: document.getElementById("refresh-secrets"),
  setForm: document.getElementById("set-form"),
  setSubmit: document.getElementById("set-submit"),
  secretKey: document.getElementById("secret-key"),
  secretValue: document.getElementById("secret-value"),
  secretList: document.getElementById("secret-list"),
  keysCount: document.getElementById("keys-count"),
  keysSearch: document.getElementById("keys-search"),
  envFile: document.getElementById("env-file"),
  envPaste: document.getElementById("env-paste"),
  importFile: document.getElementById("import-file"),
  importPaste: document.getElementById("import-paste"),
};

function setSessionState(text, status = "pending") {
  el.session.textContent = text;
  el.session.setAttribute("data-state", status);
}

function setMessage(text, level = "info") {
  el.message.textContent = text;
  el.message.className = `status${level === "success" ? " status-success" : level === "error" ? " status-error" : ""}`;
}

function readErrorMessage(payload) {
  if (!payload || typeof payload !== "object") {
    return "request failed";
  }
  if (typeof payload.error === "string" && payload.error.length > 0) {
    return payload.error;
  }
  return "request failed";
}

function updateProfileChip() {
  el.activeProfileChip.textContent = `Profile: ${state.profile}`;
}

function setButtonBusy(button, busy) {
  button.disabled = busy;
  button.setAttribute("aria-busy", String(busy));
}

async function withButtonBusy(button, fn) {
  setButtonBusy(button, true);
  try {
    return await fn();
  } finally {
    setButtonBusy(button, false);
  }
}

function isTypingContext(target) {
  if (!target) {
    return false;
  }
  const tagName = target.tagName;
  if (!tagName) {
    return false;
  }
  return (
    tagName === "INPUT" ||
    tagName === "TEXTAREA" ||
    tagName === "SELECT" ||
    target.isContentEditable
  );
}

async function fetchToken() {
  const res = await fetch("/token", {
    method: "GET",
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error("token exchange failed");
  }
  state.token = (await res.text()).trim();
  setSessionState("Authenticated", "success");
  el.sessionAuth.textContent = "Authenticated";
}

async function api(path, options = {}) {
  const headers = new Headers(options.headers || {});
  headers.set("Authorization", `Bearer ${state.token}`);
  if (options.body && !headers.has("Content-Type")) {
    headers.set("Content-Type", "application/json");
  }

  const res = await fetch(path, {
    ...options,
    headers,
    cache: "no-store",
  });

  let payload = null;
  const contentType = res.headers.get("content-type") || "";
  if (contentType.includes("application/json")) {
    payload = await res.json();
  } else {
    const text = await res.text();
    if (text) {
      payload = { error: text };
    }
  }

  if (!res.ok) {
    throw new Error(readErrorMessage(payload));
  }
  return payload;
}

async function checkHealth() {
  try {
    await api("/api/health");
    el.sessionHealth.textContent = "Healthy";
  } catch {
    el.sessionHealth.textContent = "Unavailable";
  }
}

function renderProfileOptions() {
  el.profileSelect.innerHTML = "";
  for (const profile of state.profiles) {
    const option = document.createElement("option");
    option.value = profile;
    option.textContent = profile;
    option.selected = profile === state.profile;
    el.profileSelect.appendChild(option);
  }
}

async function loadProfiles() {
  const data = await api("/api/profiles");
  state.profiles = Array.isArray(data.profiles) ? data.profiles : [];
  if (state.profiles.length === 0) {
    state.profiles = ["default"];
  }
  if (!state.profiles.includes(state.profile)) {
    state.profile = state.profiles[0];
  }
  renderProfileOptions();
  updateProfileChip();
}

function renderEmptyRow(message) {
  const tr = document.createElement("tr");
  tr.className = "empty";
  const td = document.createElement("td");
  td.colSpan = 3;
  td.textContent = message;
  tr.appendChild(td);
  el.secretList.appendChild(tr);
}

function filteredSecrets() {
  const query = state.filter.trim().toLowerCase();
  if (!query) {
    return [...state.secrets];
  }
  return state.secrets.filter((item) => item.key.toLowerCase().includes(query));
}

function updateKeysCount(filteredCount) {
  const total = state.secrets.length;
  if (state.filter.trim()) {
    el.keysCount.textContent = `${filteredCount}/${total} keys`;
    return;
  }
  el.keysCount.textContent = `${total} keys`;
}

function renderSecrets() {
  el.secretList.innerHTML = "";
  const entries = filteredSecrets();
  updateKeysCount(entries.length);

  if (state.secrets.length === 0) {
    renderEmptyRow("No keys yet. Add a secret or import a .env file.");
    return;
  }

  if (entries.length === 0) {
    renderEmptyRow("No keys matched your search.");
    return;
  }

  for (const item of entries) {
    const tr = document.createElement("tr");

    const tdKey = document.createElement("td");
    tdKey.textContent = item.key;

    const tdMasked = document.createElement("td");
    tdMasked.textContent = item.masked || "********";

    const tdAction = document.createElement("td");
    const delButton = document.createElement("button");
    delButton.type = "button";
    delButton.textContent = "Delete";
    delButton.className = "btn btn-danger";
    delButton.addEventListener("click", async () => {
      const ok = window.confirm(`Delete key ${item.key} from profile ${state.profile}?`);
      if (!ok) {
        return;
      }
      try {
        await withButtonBusy(delButton, async () =>
          api(`/api/secrets/${encodeURIComponent(item.key)}?profile=${encodeURIComponent(state.profile)}`, {
            method: "DELETE",
          })
        );
        setMessage(`Deleted ${item.key}`, "success");
        await loadSecrets();
      } catch (err) {
        setMessage(err.message, "error");
      }
    });
    tdAction.appendChild(delButton);

    tr.appendChild(tdKey);
    tr.appendChild(tdMasked);
    tr.appendChild(tdAction);
    el.secretList.appendChild(tr);
  }
}

async function loadSecrets() {
  const data = await api(`/api/secrets?profile=${encodeURIComponent(state.profile)}`);
  state.secrets = Array.isArray(data.secrets) ? data.secrets : [];
  renderSecrets();
}

async function handleCreateProfile() {
  const name = el.profileName.value.trim();
  if (!name) {
    setMessage("Profile name is required", "error");
    return;
  }

  await api("/api/profiles", {
    method: "POST",
    body: JSON.stringify({ name }),
  });
  el.profileName.value = "";
  state.profile = name;
  await loadProfiles();
  await loadSecrets();
  setMessage(`Profile ${name} created`, "success");
}

async function handleDeleteProfile() {
  const profile = state.profile;
  const ok = window.confirm(`Delete profile ${profile} and all keys?`);
  if (!ok) {
    return;
  }

  const data = await api(`/api/profiles/${encodeURIComponent(profile)}`, {
    method: "DELETE",
  });
  if (!data.deleted) {
    setMessage(`Profile ${profile} not found`, "info");
    return;
  }

  state.profile = "default";
  await loadProfiles();
  await loadSecrets();
  setMessage(`Profile ${profile} deleted`, "success");
}

async function handleSetSecret(event) {
  event.preventDefault();
  const key = el.secretKey.value.trim();
  const value = el.secretValue.value;

  if (!key || !value) {
    setMessage("Key and value are required", "error");
    return;
  }

  await api("/api/secrets", {
    method: "POST",
    body: JSON.stringify({
      profile: state.profile,
      key,
      value,
    }),
  });

  el.secretValue.value = "";
  setMessage(`Saved ${key}`, "success");
  await loadSecrets();
}

async function handleImportEnv(content) {
  if (!content || !content.trim()) {
    setMessage(".env content is empty", "error");
    return;
  }

  const data = await api("/api/import-env", {
    method: "POST",
    body: JSON.stringify({
      profile: state.profile,
      content,
    }),
  });

  setMessage(`Imported ${data.imported} entries. ${data.suggestion || ""}`.trim(), "success");
  await loadSecrets();
}

function bindEvents() {
  el.profileSelect.addEventListener("change", async () => {
    state.profile = el.profileSelect.value;
    updateProfileChip();
    try {
      await loadSecrets();
      setMessage(`Loaded profile ${state.profile}`, "info");
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.refreshSecrets.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.refreshSecrets, () => loadSecrets());
      setMessage(`Loaded keys for ${state.profile}`, "info");
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.profileCreate.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.profileCreate, () => handleCreateProfile());
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.profileDelete.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.profileDelete, () => handleDeleteProfile());
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.setForm.addEventListener("submit", async (event) => {
    try {
      await withButtonBusy(el.setSubmit, () => handleSetSecret(event));
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.importFile.addEventListener("click", async () => {
    try {
      const file = el.envFile.files && el.envFile.files[0];
      if (!file) {
        setMessage("Select a .env file first", "error");
        return;
      }
      const content = await file.text();
      await withButtonBusy(el.importFile, () => handleImportEnv(content));
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.importPaste.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.importPaste, () => handleImportEnv(el.envPaste.value));
      el.envPaste.value = "";
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.keysSearch.addEventListener("input", () => {
    state.filter = el.keysSearch.value;
    renderSecrets();
  });

  document.addEventListener("keydown", (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter" && document.activeElement === el.secretValue) {
      event.preventDefault();
      el.setForm.requestSubmit();
      return;
    }

    if (isTypingContext(document.activeElement)) {
      return;
    }

    if (event.key === "/") {
      event.preventDefault();
      el.keysSearch.focus();
      el.keysSearch.select();
      return;
    }

    if (event.key.toLowerCase() === "n") {
      event.preventDefault();
      el.secretKey.focus();
    }
  });
}

async function init() {
  try {
    await fetchToken();
    await checkHealth();
    bindEvents();
    await loadProfiles();
    await loadSecrets();
    setMessage(`Ready on profile ${state.profile}`, "success");
  } catch (err) {
    setSessionState("Failed", "error");
    el.sessionAuth.textContent = "Failed";
    el.sessionHealth.textContent = "Unavailable";
    setMessage(err.message || "Failed to initialize UI", "error");
  }
}

void init();
