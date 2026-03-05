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
  profileDangerHint: document.getElementById("profile-danger-hint"),
  refreshSecrets: document.getElementById("refresh-secrets"),
  setForm: document.getElementById("set-form"),
  setSubmit: document.getElementById("set-submit"),
  secretKey: document.getElementById("secret-key"),
  secretValue: document.getElementById("secret-value"),
  secretList: document.getElementById("secret-list"),
  keysCount: document.getElementById("keys-count"),
  keysSearch: document.getElementById("keys-search"),
  statProfiles: document.getElementById("stat-profiles"),
  statKeys: document.getElementById("stat-keys"),
  statVisible: document.getElementById("stat-visible"),
  envFile: document.getElementById("env-file"),
  envPaste: document.getElementById("env-paste"),
  importFile: document.getElementById("import-file"),
  importPaste: document.getElementById("import-paste"),
};

const STATUS_PREFIX = {
  info: "Status",
  success: "Success",
  error: "Error",
};

function setSessionState(text, status = "pending") {
  el.session.textContent = text;
  el.session.setAttribute("data-state", status);
}

function setMessage(text, level = "info") {
  const normalized = level === "success" || level === "error" ? level : "info";
  el.message.textContent = `${STATUS_PREFIX[normalized]}: ${text}`;
  el.message.dataset.level = normalized;
}

function readErrorMessage(payload) {
  if (!payload || typeof payload !== "object") {
    return "Request failed";
  }
  if (typeof payload.error === "string" && payload.error.length > 0) {
    return payload.error;
  }
  if (typeof payload.message === "string" && payload.message.length > 0) {
    return payload.message;
  }
  return "Request failed";
}

function toErrorMessage(err) {
  if (err instanceof Error && err.message) {
    return err.message;
  }
  return "Request failed";
}

function updateProfileChip() {
  el.activeProfileChip.textContent = state.profile;
}

function updateProfileDangerState() {
  const isDefault = state.profile === "default";
  el.profileDelete.disabled = isDefault;

  if (el.profileDangerHint) {
    el.profileDangerHint.textContent = isDefault
      ? "The default profile is protected from deletion."
      : `Deleting "${state.profile}" is permanent and cannot be undone.`;
  }
}

function updateStats(visibleCount = state.secrets.length) {
  el.statProfiles.textContent = String(state.profiles.length);
  el.statKeys.textContent = String(state.secrets.length);
  el.statVisible.textContent = String(visibleCount);
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
  if (!target || !target.tagName) {
    return false;
  }
  return target.matches("input, textarea, select, [contenteditable='true']");
}

async function fetchToken() {
  const res = await fetch("/token", {
    method: "GET",
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error("Token exchange failed");
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
    return true;
  } catch {
    el.sessionHealth.textContent = "Unavailable";
    return false;
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
  updateProfileDangerState();
  updateStats();
}

function renderEmptyRow(title, description) {
  const tr = document.createElement("tr");
  tr.className = "empty";

  const td = document.createElement("td");
  td.colSpan = 3;

  const wrapper = document.createElement("div");
  wrapper.className = "empty-state";

  const heading = document.createElement("p");
  heading.className = "empty-title";
  heading.textContent = title;

  const body = document.createElement("p");
  body.className = "empty-description";
  body.textContent = description;

  wrapper.appendChild(heading);
  wrapper.appendChild(body);
  td.appendChild(wrapper);
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
    el.keysCount.textContent = `${filteredCount} shown of ${total} keys`;
  } else {
    el.keysCount.textContent = `${total} keys`;
  }
}

function renderSecrets() {
  el.secretList.innerHTML = "";
  const entries = filteredSecrets();

  updateKeysCount(entries.length);
  updateStats(entries.length);

  if (state.secrets.length === 0) {
    renderEmptyRow(
      "No keys in this profile",
      `Profile "${state.profile}" has no secrets yet. Add a secret from Quick actions or import a .env file.`
    );
    return;
  }

  if (entries.length === 0) {
    renderEmptyRow(
      "No matching keys",
      `No key names matched "${state.filter.trim()}". Adjust the search term and try again.`
    );
    return;
  }

  for (const item of entries) {
    const tr = document.createElement("tr");

    const tdKey = document.createElement("td");
    tdKey.className = "key-cell";
    const keyCode = document.createElement("code");
    keyCode.textContent = item.key;
    tdKey.appendChild(keyCode);

    const tdMasked = document.createElement("td");
    tdMasked.className = "masked-cell";
    tdMasked.textContent = item.masked || "********";

    const tdAction = document.createElement("td");
    tdAction.className = "action-cell";

    const delButton = document.createElement("button");
    delButton.type = "button";
    delButton.textContent = "Delete";
    delButton.className = "btn btn-danger btn-row";
    delButton.addEventListener("click", async () => {
      const confirmed = window.confirm(
        `Delete key "${item.key}" from profile "${state.profile}"? This cannot be undone.`
      );
      if (!confirmed) {
        return;
      }

      try {
        await withButtonBusy(delButton, async () =>
          api(`/api/secrets/${encodeURIComponent(item.key)}?profile=${encodeURIComponent(state.profile)}`, {
            method: "DELETE",
          })
        );
        await loadSecrets();
        setMessage(`Deleted key "${item.key}" from profile "${state.profile}".`, "success");
      } catch (err) {
        setMessage(`Could not delete key "${item.key}": ${toErrorMessage(err)}`, "error");
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
    setMessage("Enter a profile name before creating.", "error");
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
  setMessage(`Created profile "${name}" and switched to it.`, "success");
}

async function handleDeleteProfile() {
  const profile = state.profile;

  if (profile === "default") {
    setMessage("Default profile is protected. Select a different profile to delete.", "error");
    return;
  }

  const confirmed = window.confirm(
    `Delete profile "${profile}" and every key inside it? This action is permanent.`
  );
  if (!confirmed) {
    setMessage(`Profile deletion cancelled for "${profile}".`, "info");
    return;
  }

  const typed = window.prompt(`Type "${profile}" to confirm deletion.`);
  if (typed !== profile) {
    setMessage(`Deletion stopped: confirmation text did not match "${profile}".`, "info");
    return;
  }

  const data = await api(`/api/profiles/${encodeURIComponent(profile)}`, {
    method: "DELETE",
  });

  if (!data.deleted) {
    setMessage(`Profile "${profile}" was not found.`, "error");
    return;
  }

  state.profile = "default";
  await loadProfiles();
  await loadSecrets();
  setMessage(`Deleted profile "${profile}" and switched to "${state.profile}".`, "success");
}

async function handleSetSecret(event) {
  event.preventDefault();
  const key = el.secretKey.value.trim();
  const value = el.secretValue.value;

  if (!key || !value) {
    setMessage("Both key and value are required.", "error");
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
  await loadSecrets();
  setMessage(`Saved key "${key}" to profile "${state.profile}".`, "success");
  el.secretValue.focus();
}

async function handleImportEnv(content) {
  if (!content || !content.trim()) {
    setMessage(".env content is empty. Add at least one KEY=value pair.", "error");
    return;
  }

  const data = await api("/api/import-env", {
    method: "POST",
    body: JSON.stringify({
      profile: state.profile,
      content,
    }),
  });

  await loadSecrets();
  const suggestion = data.suggestion ? ` ${data.suggestion}` : "";
  setMessage(
    `Imported ${data.imported} entr${data.imported === 1 ? "y" : "ies"} into profile "${state.profile}".${suggestion}`,
    "success"
  );
}

function bindEvents() {
  el.profileSelect.addEventListener("change", async () => {
    state.profile = el.profileSelect.value;
    updateProfileChip();
    updateProfileDangerState();

    try {
      await loadSecrets();
      setMessage(`Switched to profile "${state.profile}" and loaded ${state.secrets.length} keys.`, "success");
    } catch (err) {
      setMessage(`Could not load profile "${state.profile}": ${toErrorMessage(err)}`, "error");
    }
  });

  el.refreshSecrets.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.refreshSecrets, async () => loadSecrets());
      setMessage(`Refreshed ${state.secrets.length} keys for profile "${state.profile}".`, "success");
    } catch (err) {
      setMessage(`Refresh failed: ${toErrorMessage(err)}`, "error");
    }
  });

  el.profileCreate.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.profileCreate, () => handleCreateProfile());
    } catch (err) {
      setMessage(`Profile creation failed: ${toErrorMessage(err)}`, "error");
    }
  });

  el.profileDelete.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.profileDelete, () => handleDeleteProfile());
    } catch (err) {
      setMessage(`Profile deletion failed: ${toErrorMessage(err)}`, "error");
    }
  });

  el.setForm.addEventListener("submit", async (event) => {
    try {
      await withButtonBusy(el.setSubmit, () => handleSetSecret(event));
    } catch (err) {
      setMessage(`Could not save secret: ${toErrorMessage(err)}`, "error");
    }
  });

  el.importFile.addEventListener("click", async () => {
    try {
      const file = el.envFile.files && el.envFile.files[0];
      if (!file) {
        setMessage("Select a .env file before importing.", "error");
        return;
      }
      const content = await file.text();
      await withButtonBusy(el.importFile, () => handleImportEnv(content));
    } catch (err) {
      setMessage(`File import failed: ${toErrorMessage(err)}`, "error");
    }
  });

  el.importPaste.addEventListener("click", async () => {
    try {
      await withButtonBusy(el.importPaste, () => handleImportEnv(el.envPaste.value));
      el.envPaste.value = "";
    } catch (err) {
      setMessage(`Paste import failed: ${toErrorMessage(err)}`, "error");
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
    setMessage("Requesting one-time UI token…", "info");
    await fetchToken();

    const healthy = await checkHealth();
    bindEvents();
    await loadProfiles();
    await loadSecrets();

    if (healthy) {
      setMessage(`Dashboard ready on profile "${state.profile}" with ${state.secrets.length} keys loaded.`, "success");
    } else {
      setMessage(
        `Dashboard loaded on profile "${state.profile}", but API health check is unavailable.`,
        "error"
      );
    }
  } catch (err) {
    setSessionState("Failed", "error");
    el.sessionAuth.textContent = "Failed";
    el.sessionHealth.textContent = "Unavailable";
    setMessage(toErrorMessage(err), "error");
  }
}

void init();
