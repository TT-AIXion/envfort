const state = {
  token: "",
  profile: "default",
  profiles: [],
};

const el = {
  session: document.getElementById("session-status"),
  message: document.getElementById("message"),
  profileSelect: document.getElementById("profile-select"),
  profileName: document.getElementById("profile-name"),
  profileCreate: document.getElementById("profile-create"),
  profileDelete: document.getElementById("profile-delete"),
  refreshSecrets: document.getElementById("refresh-secrets"),
  setForm: document.getElementById("set-form"),
  secretKey: document.getElementById("secret-key"),
  secretValue: document.getElementById("secret-value"),
  secretList: document.getElementById("secret-list"),
  envFile: document.getElementById("env-file"),
  envPaste: document.getElementById("env-paste"),
  importFile: document.getElementById("import-file"),
  importPaste: document.getElementById("import-paste"),
};

function setMessage(text, level = "info") {
  el.message.textContent = text;
  el.message.className = level === "error" ? "error" : level === "success" ? "success" : "";
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

async function fetchToken() {
  const res = await fetch("/token", {
    method: "GET",
    cache: "no-store",
  });
  if (!res.ok) {
    throw new Error("token exchange failed");
  }
  state.token = (await res.text()).trim();
  el.session.textContent = "Authenticated";
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
}

function renderSecrets(secrets) {
  el.secretList.innerHTML = "";

  if (!Array.isArray(secrets) || secrets.length === 0) {
    const tr = document.createElement("tr");
    tr.innerHTML = "<td colspan='3'>No keys yet</td>";
    el.secretList.appendChild(tr);
    return;
  }

  for (const item of secrets) {
    const tr = document.createElement("tr");

    const tdKey = document.createElement("td");
    tdKey.textContent = item.key;

    const tdMasked = document.createElement("td");
    tdMasked.textContent = item.masked || "********";

    const tdAction = document.createElement("td");
    const delButton = document.createElement("button");
    delButton.type = "button";
    delButton.textContent = "Delete";
    delButton.className = "danger";
    delButton.addEventListener("click", async () => {
      const ok = window.confirm(`Delete key ${item.key} from profile ${state.profile}?`);
      if (!ok) {
        return;
      }
      try {
        await api(`/api/secrets/${encodeURIComponent(item.key)}?profile=${encodeURIComponent(state.profile)}`, {
          method: "DELETE",
        });
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
  renderSecrets(data.secrets || []);
}

async function handleCreateProfile() {
  const name = el.profileName.value.trim();
  if (!name) {
    setMessage("profile name is required", "error");
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
    setMessage("key and value are required", "error");
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

  setMessage(
    `Imported ${data.imported} entries. ${data.suggestion || ""}`.trim(),
    "success"
  );
  await loadSecrets();
}

function bindEvents() {
  el.profileSelect.addEventListener("change", async () => {
    state.profile = el.profileSelect.value;
    try {
      await loadSecrets();
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.refreshSecrets.addEventListener("click", async () => {
    try {
      await loadSecrets();
      setMessage(`Loaded keys for ${state.profile}`, "info");
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.profileCreate.addEventListener("click", async () => {
    try {
      await handleCreateProfile();
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.profileDelete.addEventListener("click", async () => {
    try {
      await handleDeleteProfile();
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.setForm.addEventListener("submit", async (event) => {
    try {
      await handleSetSecret(event);
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.importFile.addEventListener("click", async () => {
    try {
      const file = el.envFile.files && el.envFile.files[0];
      if (!file) {
        setMessage("select a .env file first", "error");
        return;
      }
      const content = await file.text();
      await handleImportEnv(content);
    } catch (err) {
      setMessage(err.message, "error");
    }
  });

  el.importPaste.addEventListener("click", async () => {
    try {
      await handleImportEnv(el.envPaste.value);
      el.envPaste.value = "";
    } catch (err) {
      setMessage(err.message, "error");
    }
  });
}

async function init() {
  try {
    await fetchToken();
    bindEvents();
    await loadProfiles();
    await loadSecrets();
    setMessage(`Ready on profile ${state.profile}`, "success");
  } catch (err) {
    el.session.textContent = "Failed";
    setMessage(err.message || "failed to initialize ui", "error");
  }
}

void init();
