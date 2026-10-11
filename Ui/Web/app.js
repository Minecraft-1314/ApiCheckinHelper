const query = new URLSearchParams(window.location.search);
const mockBridge = query.has("mock");
const t = (key, ...args) => window.I18n.t(key, ...args);

const pending = new Map();
const elements = {};
const STATE_TONES = {
  success: "good",
  already: "good",
  unauthorized: "warn",
  challenge: "warn",
  offline: "bad",
  failed: "bad",
  disabled: "idle",
  unsupported: "idle",
  skipped: "idle",
  idle: "idle",
  running: "accent"
};
const STATE_KEYS = {
  success: "state.success",
  already: "state.already",
  unauthorized: "state.unauthorized",
  challenge: "state.challenge",
  offline: "state.offline",
  failed: "state.failed",
  disabled: "state.disabled",
  unsupported: "state.unsupported",
  skipped: "state.skipped",
  idle: "state.idle",
  running: "state.running"
};
const PROBLEM_STATES = new Set(["unauthorized", "challenge", "offline", "failed", "disabled", "unsupported"]);
const GOOD_STATES = new Set(["success", "already"]);
const DEFAULT_SETTINGS = {
  concurrency: 3,
  timeoutMs: 15000,
  notify: true,
  skipManualOnly: true,
  theme: "auto",
  language: "auto"
};
const STAGGER_STEP_MS = 35;
const STAGGER_CAP_MS = 280;
const BRIDGE_TIMEOUT_MS = 180000;
const TOAST_MS = 2600;

let sequence = 0;
function createEmptyState() {
  return {
    sites: [],
    settings: { ...DEFAULT_SETTINGS },
    history: {},
    verify: { total: 0, ok: 0, failed: 0, items: [] },
    theme: "auto",
    language: "auto",
    dataDirectory: "",
    lastError: ""
  };
}

let appState = createEmptyState();
let selectedId = "";
let activeFilter = "all";
let busy = false;
let toastTimer = 0;
let confirmResolver = null;
let promptResolver = null;

const ELEMENT_IDS = [
  "headerSummary", "headerConfigured", "headerAuto", "headerManual",
  "refreshButton", "importButton", "languageButton", "themeButton", "settingsButton",
  "cancelButton", "runAllButton", "progressTrack", "progressFill",
  "searchInput", "clearSearchButton", "addToggleButton", "filterGroup",
  "addPanel", "addHost", "addName", "probeNewButton", "addSiteButton",
  "siteCount", "siteList", "emptyState", "emptyTitle", "emptyCopy",
  "detailEmpty", "detailContent", "detailName", "detailStatus",
  "detailHost", "openSiteButton", "detailTags",
  "tokenGroup", "tokenField",
  "userIdField", "formStatus", "saveCredentialButton", "verifySiteButton",
  "clearCredentialButton", "currentCredential", "resultTime", "resultBody",
  "probeSiteButton", "renameSiteButton", "runSiteButton", "removeSiteButton",
  "footerConfigured", "footerToday", "footerQuota", "footerProblems",
  "footerStatus", "openDataButton", "toast", "toastIcon", "toastText",
  "importOverlay", "importInputs", "addImportRowButton", "importStatus",
  "clearCredentialsButton", "importConfirmButton",
  "settingsOverlay", "settingConcurrency", "settingTimeout", "settingTheme",
  "settingLanguage", "settingNotify", "settingSkipManual", "saveSettingsButton",
  "confirmOverlay", "confirmTitle", "confirmCopy", "confirmCancelButton",
  "confirmAcceptButton", "promptOverlay", "promptTitle", "promptCopy",
  "promptInput", "promptStatus", "promptCancelButton", "promptAcceptButton"
];

document.addEventListener("DOMContentLoaded", initialize);

function initialize() {
  ELEMENT_IDS.forEach(id => {
    elements[id] = document.getElementById(id);
  });
  bindEvents();
  window.I18n.apply(appState.settings?.language || "auto");
  applyTheme(appState.settings?.theme || "auto");
  refreshIcons();
  addImportRow();
  renderSites(false);
  loadState(false);
}

function bindEvents() {
  elements.refreshButton.addEventListener("click", () => loadState(true));
  elements.importButton.addEventListener("click", () => openOverlay("importOverlay"));
  elements.languageButton.addEventListener("click", cycleLanguage);
  elements.themeButton.addEventListener("click", toggleTheme);
  elements.settingsButton.addEventListener("click", openSettings);
  elements.cancelButton.addEventListener("click", cancelOperation);
  elements.runAllButton.addEventListener("click", () => runCheckin(null));

  elements.searchInput.addEventListener("input", () => {
    elements.clearSearchButton.hidden = elements.searchInput.value.length === 0;
    renderSites(false);
  });
  elements.clearSearchButton.addEventListener("click", () => {
    elements.searchInput.value = "";
    elements.clearSearchButton.hidden = true;
    elements.searchInput.focus();
    renderSites(false);
  });

  elements.filterGroup.addEventListener("click", event => {
    const button = event.target.closest("[data-filter]");
    if (!button) return;
    activeFilter = button.dataset.filter;
    syncSegments(elements.filterGroup, button);
    renderSites(false);
  });

  elements.addToggleButton.addEventListener("click", () => {
    elements.addPanel.classList.toggle("hidden");
    if (!elements.addPanel.classList.contains("hidden")) elements.addHost.focus();
  });
  elements.probeNewButton.addEventListener("click", probeNewSite);
  elements.addSiteButton.addEventListener("click", addNewSite);
  elements.addHost.addEventListener("keydown", event => {
    if (event.key === "Enter") addNewSite();
  });

  elements.siteList.addEventListener("click", event => {
    const row = event.target.closest("[data-site-id]");
    if (row) selectSite(row.dataset.siteId);
  });
  elements.siteList.addEventListener("dblclick", event => {
    const row = event.target.closest("[data-site-id]");
    if (!row) return;
    const site = findSite(row.dataset.siteId);
    if (site) call("openExternal", { url: site.bookmark });
  });

  elements.saveCredentialButton.addEventListener("click", saveCredential);
  elements.verifySiteButton.addEventListener("click", () => verifySite(selectedId));
  elements.clearCredentialButton.addEventListener("click", clearCredential);
  elements.openSiteButton.addEventListener("click", openSelectedSite);
  elements.probeSiteButton.addEventListener("click", probeSelectedSite);
  elements.renameSiteButton.addEventListener("click", renameSelectedSite);
  elements.runSiteButton.addEventListener("click", () => runCheckin(selectedId));
  elements.removeSiteButton.addEventListener("click", removeSelectedSite);
  elements.openDataButton.addEventListener("click", () => call("openDataDirectory"));

  elements.addImportRowButton.addEventListener("click", addImportRow);
  elements.importInputs.addEventListener("click", event => {
    const button = event.target.closest("[data-remove-row]");
    if (!button) return;
    const entry = button.closest(".import-entry");
    if (elements.importInputs.querySelectorAll(".import-entry").length <= 1) {
      resetImportEntry(entry);
    } else {
      entry.remove();
    }
    renumberImportRows();
  });
  elements.clearCredentialsButton.addEventListener("click", clearAllCredentials);
  elements.importConfirmButton.addEventListener("click", importCredentials);

  elements.saveSettingsButton.addEventListener("click", saveSettings);
  elements.confirmCancelButton.addEventListener("click", () => resolveConfirm(false));
  elements.confirmAcceptButton.addEventListener("click", () => resolveConfirm(true));
  elements.promptCancelButton.addEventListener("click", () => resolvePrompt(null));
  elements.promptAcceptButton.addEventListener("click", submitPrompt);
  elements.promptInput.addEventListener("keydown", event => {
    if (event.key === "Enter") submitPrompt();
  });

  document.querySelectorAll("[data-close]").forEach(button => {
    button.addEventListener("click", () => closeOverlay(button.dataset.close));
  });
  document.querySelectorAll(".overlay").forEach(overlay => {
    overlay.addEventListener("mousedown", event => {
      if (event.target === overlay) closeOverlay(overlay.id);
    });
  });

  document.addEventListener("keydown", handleShortcut);
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if ((appState.settings?.theme || "auto") === "auto") applyTheme("auto");
  });
  document.addEventListener("i18n:applied", () => {
    const counts = getCounts();
    renderHeader(counts);
    renderSites(false);
    renderDetail();
    renderFooter(counts);
    syncStaticLabels();
  });

  if (window.chrome?.webview) {
    window.chrome.webview.addEventListener("message", handleMessage);
  }
}

function syncStaticLabels() {
  document.querySelectorAll(".import-entry").forEach(entry => {
    entry.querySelectorAll("[data-i18n]").forEach(node => {
      node.textContent = t(node.dataset.i18n);
    });
    entry.querySelectorAll("[data-i18n-placeholder]").forEach(node => {
      node.setAttribute("placeholder", t(node.dataset.i18nPlaceholder));
    });
  });
  renumberImportRows();
}

async function loadState(refresh) {
  try {
    appState = await call("getState", { refresh });
    appState.settings = appState.settings || { ...DEFAULT_SETTINGS };
    selectedId = appState.sites.some(site => site.id === selectedId)
      ? selectedId
      : appState.sites[0]?.id || "";
    window.I18n.apply(appState.settings.language);
    applyTheme(appState.settings.theme);
    renderAll(true);
    setFooterStatus(refresh ? t("app.reloaded") : "");
    if (appState.lastError) showToast(appState.lastError, "warn");
  } catch (error) {
    showToast(error.message || t("msg.loadFailed"), "bad");
  }
}

function renderAll(stagger) {
  const counts = getCounts();
  renderHeader(counts);
  renderSites(stagger);
  renderDetail();
  renderFooter(counts);
  refreshIcons();
}

function renderHeader(counts = getCounts()) {
  elements.headerConfigured.textContent = t("header.configured", counts.configured);
  elements.headerAuto.textContent = t("header.auto", counts.auto);
  elements.headerManual.textContent = t("header.manual", counts.manual);
  elements.headerSummary.textContent = counts.total
    ? t("app.configuredOf", counts.configured, counts.total)
    : t("app.emptyConfig");
}

function renderFooter(counts = getCounts()) {
  const bucket = appState.history?.[localDayKey()] || {};
  elements.footerConfigured.textContent = t("footer.configured", counts.configured, counts.total);
  elements.footerToday.textContent = t("footer.today", bucket.success || 0);
  elements.footerQuota.textContent = t("footer.quota", formatUsd(bucket.quotaUsd || 0));
  elements.footerProblems.textContent = t("footer.problems", counts.problems);
}

function getCounts() {
  let configured = 0;
  let manual = 0;
  let auto = 0;
  let problems = 0;
  for (const site of appState.sites || []) {
    if (!site.configured) continue;
    configured++;
    if (isManual(site)) manual++;
    else auto++;
    if (PROBLEM_STATES.has(getSiteState(site))) problems++;
  }
  return { total: (appState.sites || []).length, configured, manual, auto, problems };
}

function isCheckinEnabled(site) {
  return site.meta?.checkinEnabled ?? site.snapshot?.checkinEnabled ?? true;
}

function isManual(site) {
  return Boolean(site.meta?.turnstile || site.snapshot?.turnstile) || isCheckinEnabled(site) === false;
}

function stateTone(site) {
  return STATE_TONES[getSiteState(site)] || "idle";
}

function stateLabel(site) {
  return t(STATE_KEYS[getSiteState(site)] || "state.idle");
}

function renderSites(stagger) {
  const sites = filteredSites();

  elements.siteList.replaceChildren();
  sites.forEach((site, index) => {
    elements.siteList.append(createSiteRow(site, index, stagger));
  });

  updateSiteListMeta(sites);
}

function filteredSites() {
  const keyword = elements.searchInput.value.trim().toLowerCase();
  return (appState.sites || []).filter(site => matches(site, keyword));
}

function createSiteRow(site, index, stagger) {
  const row = document.createElement("button");
  row.type = "button";
  row.className = `site-row${site.id === selectedId ? " selected" : ""}${site.configured ? "" : " unconfigured"}${stagger ? " animate-in" : ""}`;
  row.dataset.siteId = site.id;
  row.setAttribute("role", "listitem");
  if (stagger) {
    row.style.setProperty("--stagger", `${Math.min(index * STAGGER_STEP_MS, STAGGER_CAP_MS)}ms`);
  }

  const dot = document.createElement("span");
  dot.className = `site-dot dot-${stateTone(site)}`;

  const copy = document.createElement("span");
  copy.className = "site-copy";
  const name = document.createElement("span");
  name.className = "site-name";
  name.textContent = site.name || site.host;
  const host = document.createElement("span");
  host.className = "site-host";
  host.textContent = site.host;
  copy.append(name, host);

  const meta = document.createElement("span");
  meta.className = "site-meta";
  const pill = document.createElement("span");
  pill.className = `status-pill tone-${stateTone(site)}`;
  pill.textContent = stateLabel(site);
  const time = document.createElement("span");
  time.className = "site-time";
  time.textContent = formatRelative(site.result?.at);
  meta.append(pill, time);

  row.append(dot, copy, meta);
  return row;
}

function updateSiteListMeta(sites) {
  elements.siteCount.textContent = `${sites.length} / ${(appState.sites || []).length}`;
  elements.emptyState.classList.toggle("hidden", sites.length > 0);
  if (sites.length === 0) {
    const hasSites = (appState.sites || []).length > 0;
    elements.emptyTitle.textContent = t(hasSites ? "list.emptyFiltered.title" : "list.emptyNone.title");
    elements.emptyCopy.textContent = t(hasSites ? "list.emptyFiltered.copy" : "list.emptyNone.copy");
  }
}

function updateSiteRow(site) {
  const sites = filteredSites();
  const visibleIndex = sites.findIndex(item => item.id === site.id);
  const row = elements.siteList.querySelector(`[data-site-id="${CSS.escape(site.id)}"]`);
  if (visibleIndex >= 0 && row) {
    row.replaceWith(createSiteRow(site, visibleIndex, false));
    return;
  }
  if (visibleIndex >= 0) {
    renderSites(false);
    return;
  }
  if (row) {
    row.remove();
    updateSiteListMeta(sites);
  }
}

function matches(site, keyword) {
  if (keyword && !`${site.name} ${site.host}`.toLowerCase().includes(keyword)) return false;
  if (activeFilter === "ready") return site.configured && !isManual(site);
  if (activeFilter === "problem") return PROBLEM_STATES.has(getSiteState(site));
  if (activeFilter === "good") return GOOD_STATES.has(getSiteState(site));
  if (activeFilter === "todo") return !site.configured;
  return true;
}

function renderDetail() {
  const site = findSite(selectedId);
  elements.detailEmpty.classList.toggle("hidden", Boolean(site));
  elements.detailContent.classList.toggle("hidden", !site);
  if (!site) return;

  elements.detailName.textContent = site.name || site.host;
  elements.detailHost.textContent = site.origin + site.entry;
  elements.detailStatus.className = `status-pill tone-${stateTone(site)}`;
  elements.detailStatus.textContent = stateLabel(site);

  elements.detailTags.replaceChildren();
  const version = site.meta?.version || site.snapshot?.version;
  if (version) addTag(version, "idle");
  if (site.meta?.turnstile || site.snapshot?.turnstile) addTag(t("tag.turnstile"), "warn");
  if (site.meta?.account) {
    const suffix = site.meta.userId ? t("tag.userId", site.meta.userId) : "";
    addTag(t("tag.bound", site.meta.account) + (suffix ? ` ${suffix}` : ""), "good");
  } else if (site.meta?.userId) {
    addTag(t("tag.userId", site.meta.userId), "good");
  }

  elements.tokenField.value = site.credential?.token || "";
  elements.userIdField.value = site.credential?.userId > 0 ? String(site.credential.userId) : "";
  elements.currentCredential.textContent = describeCredential(site.credential);
  renderResult(site);
  refreshIcons();
}

function addTag(text, tone) {
  const tag = document.createElement("span");
  tag.className = `tag-pill tone-${tone}`;
  tag.textContent = text;
  elements.detailTags.append(tag);
}

function syncSegments(group, active) {
  group.querySelectorAll(".segment").forEach(item => {
    item.classList.toggle("active", item === active);
  });
}

function renderResult(site) {
  const result = site.result;
  elements.resultBody.replaceChildren();
  if (!result) {
    elements.resultTime.textContent = t("detail.result.empty");
    const copy = document.createElement("p");
    copy.className = "text-sm text-muted";
    copy.textContent = t("detail.result.hint");
    elements.resultBody.append(copy);
    return;
  }
  elements.resultTime.textContent = formatRelative(result.at);
  const tone = STATE_TONES[result.state] || "bad";
  const row = document.createElement("div");
  row.className = "flex flex-wrap items-center gap-3";
  const pill = document.createElement("span");
  pill.className = `status-pill tone-${tone}`;
  pill.textContent = t(STATE_KEYS[result.state] || "state.failed");
  const message = document.createElement("p");
  message.className = "text-sm text-ink";
  message.textContent = result.state === "success" && result.quota > 0
    ? t("amount.gained", formatQuota(result.quota, site.meta || site.snapshot))
    : result.message || pill.textContent;
  row.append(pill, message);
  elements.resultBody.append(row);
  if (result.httpStatus) {
    const code = document.createElement("p");
    code.className = "mt-3 font-mono text-[11px] text-muted";
    code.textContent = t("http.line", result.httpStatus, result.path ? t("http.path", result.path) : "");
    elements.resultBody.append(code);
  }
}

async function saveCredential() {
  const site = findSite(selectedId);
  if (!site) return;
  const payload = {
    siteId: site.id,
    type: "token",
    token: elements.tokenField.value.trim(),
    userId: positiveInt(elements.userIdField.value)
  };
  const problem = credentialProblem(payload);
  if (problem) return setFormStatus(problem, "warn");

  await runOperation(t("btn.saveCredential"), async () => {
    await call("saveCredential", payload);
    const outcome = await call("verifySite", { siteId: site.id });
    await loadState(false);
    const text = outcome.message || (outcome.ok ? t("msg.savedAndChecked") : t("msg.savedButUnchecked"));
    setFormStatus(text, outcome.ok ? "good" : "bad");
    showToast(outcome.ok ? t("msg.credentialSaved") : t("msg.credentialSavedUnverified", text), outcome.ok ? "good" : "bad");
  });
}

function credentialProblem(payload) {
  return payload.token ? "" : t("msg.tokenRequired");
}

async function verifySite(siteId) {
  if (!findSite(siteId)) return;
  await runOperation(t("btn.verify"), async () => {
    const outcome = await call("verifySite", { siteId });
    await loadState(false);
    const text = outcome.message || (outcome.ok ? t("msg.savedAndChecked") : t("msg.savedButUnchecked"));
    if (siteId === selectedId) setFormStatus(text, outcome.ok ? "good" : "bad");
    showToast(text, outcome.ok ? "good" : "bad");
  });
}

async function clearCredential() {
  const site = findSite(selectedId);
  if (!site) return;
  const accepted = await confirmAction(
    t("msg.clearTitle"),
    t("msg.clearCopy", site.name || site.host),
    t("msg.clearAllAccept")
  );
  if (!accepted) return;
  await runOperation(t("btn.clearCredential"), async () => {
    await call("clearCredential", { siteId: site.id });
    await loadState(false);
    showToast(t("msg.credentialCleared"), "good");
  });
}

async function probeNewSite() {
  const host = elements.addHost.value.trim();
  if (!host) return showToast(t("msg.hostRequired"), "warn");
  await runOperation(t("btn.probe"), async () => {
    setFooterStatus(t("msg.probeRunning"));
    try {
      const result = await call("discoverSite", { host });
      if (!result.ok) {
        showToast(result.message || t("msg.probeFailed"), "bad");
        return;
      }
      if (!elements.addName.value.trim() && result.name) elements.addName.value = result.name;
      const abilities = [];
      abilities.push(result.checkinEnabled ? t("state.success") : t("state.disabled"));
      if (result.turnstile) abilities.push(t("tag.turnstile"));
      if (result.oauth?.length) abilities.push(`OAuth: ${result.oauth.join("/")}`);
      showToast(`${result.name || result.host} · ${abilities.join(" · ")}`, "good");
    } finally {
      setFooterStatus("");
    }
  });
}

async function addNewSite() {
  const host = elements.addHost.value.trim();
  if (!host) return showToast(t("msg.hostRequired"), "warn");
  try {
    const site = await call("addSite", { host, name: elements.addName.value.trim() });
    elements.addHost.value = "";
    elements.addName.value = "";
    elements.addPanel.classList.add("hidden");
    selectedId = site.id;
    await loadState(false);
    showToast(t("msg.added", site.name), "good");
    probeSelectedSite();
  } catch (error) {
    showToast(error.message || t("msg.addFailed"), "bad");
  }
}

async function probeSelectedSite() {
  const site = findSite(selectedId);
  if (!site) return;
  await runOperation(t("btn.probeSite"), async () => {
    const outcome = await call("probeSite", { siteId: site.id });
    await loadState(false);
    showToast(outcome.message || t("state.idle"), outcome.state === "idle" ? "good" : "warn");
  });
}

async function renameSelectedSite() {
  const site = findSite(selectedId);
  if (!site) return;
  const name = await promptAction(t("msg.renameTitle"), t("msg.renameCopy"), site.name, site.host);
  if (name === null) return;
  if (!name.trim()) return showToast(t("msg.nameRequired"), "warn");
  try {
    await call("renameSite", { siteId: site.id, name: name.trim() });
    await loadState(false);
    showToast(t("msg.nameUpdated"), "good");
  } catch (error) {
    showToast(error.message || t("msg.saveFailed"), "bad");
  }
}

async function removeSelectedSite() {
  const site = findSite(selectedId);
  if (!site) return;
  const accepted = await confirmAction(
    t("msg.removeTitle"),
    t("msg.removeCopy", site.name || site.host),
    t("msg.removeAccept")
  );
  if (!accepted) return;
  try {
    await call("removeSite", { siteId: site.id });
    selectedId = "";
    await loadState(false);
    showToast(t("msg.removed"), "good");
  } catch (error) {
    showToast(error.message || t("msg.saveFailed"), "bad");
  }
}

async function runCheckin(siteId) {
  const label = siteId ? t("btn.runSite") : t("btn.runAll");
  await runOperation(label, async () => {
    const summary = await call("runCheckin", { siteId });
    await loadState(false);
    const results = Object.values(summary.results || {});
    const good = results.filter(item => GOOD_STATES.has(item.state)).length;
    const bad = results.filter(item => PROBLEM_STATES.has(item.state)).length;
    if (!summary.total) return showToast(t("run.noneToRun"), "warn");
    const attention = bad ? t("run.needsAttention", bad) : "";
    showToast(t("run.summary", good, attention), bad ? "warn" : "good");
  });
}

async function runOperation(label, action) {
  if (busy) return;
  setBusy(true, label);
  setProgress(0, 1);
  try {
    await action();
  } catch (error) {
    showToast(error?.cancelled ? t("msg.operationStopped") : (error.message || t("run.failed", label)), "bad");
  } finally {
    setBusy(false, "");
  }
}

function setBusy(value, label) {
  busy = value;
  elements.cancelButton.classList.toggle("hidden", !value);
  elements.runAllButton.classList.toggle("hidden", value);
  elements.progressTrack.classList.toggle("visible", value);
  if (!value) setProgress(0, 1);
  const counts = getCounts();
  elements.headerSummary.textContent = value
    ? t("run.doing", label)
    : (counts.total ? t("app.configuredOf", counts.configured, counts.total) : t("app.emptyConfig"));
  document.querySelectorAll("button").forEach(button => {
    if (button === elements.cancelButton) return;
    button.disabled = value;
  });
}

function setProgress(done, total) {
  const ratio = total > 0 ? Math.max(0.02, Math.min(1, done / total)) : 0;
  elements.progressFill.style.transform = `scaleX(${ratio})`;
}

async function cancelOperation() {
  if (!busy) return;
  try {
    await call("cancel");
  } catch (error) {
    showToast(error.message || t("msg.stopFailed"), "bad");
  }
}

function openSettings() {
  const settings = appState.settings || DEFAULT_SETTINGS;
  elements.settingConcurrency.value = String(settings.concurrency || DEFAULT_SETTINGS.concurrency);
  elements.settingTimeout.value = String(Math.round((settings.timeoutMs || DEFAULT_SETTINGS.timeoutMs) / 1000));
  elements.settingTheme.value = settings.theme || "auto";
  elements.settingLanguage.value = normalizeLanguage(settings.language);
  elements.settingNotify.checked = settings.notify !== false;
  elements.settingSkipManual.checked = settings.skipManualOnly !== false;
  openOverlay("settingsOverlay");
}

async function saveSettings() {
  const payload = {
    concurrency: positiveInt(elements.settingConcurrency.value, DEFAULT_SETTINGS.concurrency),
    timeoutMs: positiveInt(elements.settingTimeout.value, 15) * 1000,
    theme: elements.settingTheme.value,
    language: normalizeLanguage(elements.settingLanguage.value),
    notify: elements.settingNotify.checked,
    skipManualOnly: elements.settingSkipManual.checked
  };
  try {
    appState = await call("patchSettings", payload);
    appState.settings = appState.settings || { ...DEFAULT_SETTINGS };
    window.I18n.apply(appState.settings.language);
    applyTheme(appState.settings.theme);
    renderAll(false);
    closeOverlay("settingsOverlay");
    showToast(t("msg.settingsSaved"), "good");
  } catch (error) {
    showToast(error.message || t("msg.settingsFailed"), "bad");
  }
}

async function cycleLanguage() {
  const order = ["auto", "zh-cn", "en-us"];
  const current = normalizeLanguage(appState.settings?.language);
  const next = order[(order.indexOf(current) + 1) % order.length];
  await applyLanguageChoice(next);
}

async function applyLanguageChoice(preference) {
  const previous = normalizeLanguage(appState.settings?.language);
  appState.settings = { ...DEFAULT_SETTINGS, ...(appState.settings || {}), language: preference };
  window.I18n.apply(preference);
  renderAll(false);
  try {
    appState = await call("patchSettings", {
      ...DEFAULT_SETTINGS,
      ...(appState.settings || {}),
      language: preference
    });
    appState.settings = appState.settings || { ...DEFAULT_SETTINGS };
  } catch (error) {
    window.I18n.apply(previous);
    showToast(error.message || t("msg.settingsFailed"), "bad");
  }
}

function normalizeLanguage(value) {
  const raw = String(value || "auto").trim().toLowerCase();
  if (raw.startsWith("zh")) return "zh-cn";
  if (raw.startsWith("en")) return "en-us";
  return "auto";
}

async function toggleTheme() {
  const current = resolvedTheme(appState.settings?.theme || "auto");
  const next = current === "dark" ? "light" : "dark";
  applyTheme(next);
  try {
    appState = await call("setTheme", { theme: next });
    renderAll(false);
  } catch (error) {
    applyTheme(current);
    showToast(error.message || t("msg.themeFailed"), "bad");
  }
}

function applyTheme(preference) {
  const mode = resolvedTheme(preference);
  document.documentElement.classList.toggle("dark", mode === "dark");
  document.documentElement.classList.toggle("light", mode !== "dark");
}

function resolvedTheme(preference) {
  if (preference === "light" || preference === "dark") return preference;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function addImportRow() {
  const entry = document.createElement("section");
  entry.className = "import-entry";
  entry.innerHTML = importRowTemplate();
  elements.importInputs.append(entry);
  renumberImportRows();
  refreshIcons();
}

function importRowTemplate() {
  return `
    <div class="import-entry-header">
      <span class="import-entry-number">1</span>
      <span class="import-entry-title"></span>
      <button class="icon-button" type="button" data-remove-row data-i18n-aria="btn.removeGroup">
        <i data-lucide="x"></i>
      </button>
    </div>
    <div class="import-entry-body">
      <div class="import-entry-grid">
        <label class="import-field">
          <span class="field-label" data-i18n="import.name"></span>
          <input class="text-field" type="text" maxlength="80" data-field="name" data-i18n-placeholder="add.nameHint" autocomplete="off">
        </label>
        <label class="import-field">
          <span class="field-label" data-i18n="import.host"></span>
          <input class="text-field mono" type="text" maxlength="2048" data-field="host" data-i18n-placeholder="add.hostHint" autocomplete="off">
        </label>
      </div>
      <div class="credential-entry-grid">
        <label class="import-field full">
          <span class="field-label" data-i18n="import.credential"></span>
          <input class="text-field mono" type="password" maxlength="4096" data-field="token" placeholder="sk-..." autocomplete="off">
        </label>
      </div>
      <div class="credential-entry-grid">
        <label class="import-field">
          <span class="field-label" data-i18n="import.type"></span>
          <select class="text-field" data-field="type">
            <option value="token" data-i18n="import.typeToken"></option>
          </select>
        </label>
        <label class="import-field">
          <span class="field-label" data-i18n="import.userId"></span>
          <input class="text-field" type="number" min="1" max="1000000000" data-field="userId">
        </label>
      </div>
    </div>`;
}

function resetImportEntry(entry) {
  entry.querySelectorAll("input").forEach(input => {
    input.value = "";
  });
}

function renumberImportRows() {
  [...elements.importInputs.querySelectorAll(".import-entry")].forEach((entry, index) => {
    entry.querySelector(".import-entry-number").textContent = String(index + 1);
    entry.querySelector(".import-entry-title").textContent = t("import.group", index + 1);
  });
}

function collectImportEntries() {
  return [...elements.importInputs.querySelectorAll(".import-entry")].map(entry => ({
    name: entry.querySelector('[data-field="name"]').value.trim(),
    host: entry.querySelector('[data-field="host"]').value.trim(),
    type: entry.querySelector('[data-field="type"]').value.trim(),
    token: entry.querySelector('[data-field="token"]').value.trim(),
    userId: positiveInt(entry.querySelector('[data-field="userId"]').value)
  }));
}

async function importCredentials() {
  const entries = collectImportEntries();
  if (!entries.length) return setImportStatus(t("msg.needOneGroup"), "warn");
  try {
    const result = await call("importCredentials", { entries });
    if (result.errors?.length) {
      setImportStatus(t("import.partial", result.errors.length, result.errors.slice(0, 8).join("\n")), "bad");
    } else {
      setImportStatus(t("import.summary", result.imported), "good");
    }
    if (result.imported > 0) await finishImport();
  } catch (error) {
    setImportStatus(error.message || t("msg.callFailed"), "bad");
  }
}

async function finishImport() {
  await loadState(false);
  await runOperation(t("btn.verify"), async () => {
    const verify = await call("verifyAll");
    await loadState(false);
    setImportStatus(t("import.verified", verify.ok + verify.failed, verify.ok, verify.failed), verify.failed ? "warn" : "good");
  });
}

async function clearAllCredentials() {
  const accepted = await confirmAction(t("msg.clearAllTitle"), t("msg.clearAllCopy"), t("msg.clearAllAccept"));
  if (!accepted) return;
  try {
    await call("clearAllCredentials");
    await loadState(false);
    elements.importInputs.querySelectorAll(".import-entry").forEach(resetImportEntry);
    setImportStatus(t("msg.credentialsCleared"), "good");
  } catch (error) {
    setImportStatus(error.message || t("msg.callFailed"), "bad");
  }
}

function setImportStatus(text, tone) {
  elements.importStatus.textContent = text;
  elements.importStatus.className = `form-status ${tone}`;
}

function openSelectedSite() {
  const site = findSite(selectedId);
  if (site) call("openExternal", { url: site.bookmark });
}

function selectSite(siteId) {
  selectedId = siteId;
  for (const row of elements.siteList.querySelectorAll(".site-row")) {
    row.classList.toggle("selected", row.dataset.siteId === selectedId);
  }
  renderDetail();
}

function findSite(siteId) {
  return (appState.sites || []).find(site => site.id === siteId) || null;
}

function getSiteState(site) {
  if (!site.configured) return "skipped";
  if (isCheckinEnabled(site) === false) return "disabled";
  if (site.result?.state) return site.result.state;
  if (site.meta?.turnstile || site.snapshot?.turnstile) return "challenge";
  return "idle";
}

function describeCredential(credential) {
  if (!credential) return t("cred.unconfigured");
  if (credential.type === "token" && credential.token) return t("cred.summaryToken", maskText(credential.token, 4, 4));
  return t("cred.unconfigured");
}

function maskText(text, head, tail) {
  if (!text) return "";
  if (text.length <= head + tail) return "•".repeat(Math.max(4, text.length));
  const dots = Math.min(16, Math.max(4, text.length - head - tail));
  return `${text.slice(0, head)}${"•".repeat(dots)}${text.slice(-tail)}`;
}

function formatRelative(timestamp) {
  if (!timestamp) return "";
  const diff = Date.now() - Number(timestamp);
  if (diff < 0) return t("relative.soon");
  if (diff < 60000) return t("relative.justNow");
  if (diff < 3600000) return t("relative.minutes", Math.floor(diff / 60000));
  if (diff < 86400000) return t("relative.hours", Math.floor(diff / 3600000));
  if (diff < 604800000) return t("relative.days", Math.floor(diff / 86400000));
  return new Date(Number(timestamp)).toLocaleDateString();
}

function formatUsd(value) {
  const amount = Number(value) || 0;
  if (amount <= 0) return "—";
  if (amount < 1) return `$${amount.toFixed(4)}`;
  if (amount < 100) return `$${amount.toFixed(2)}`;
  return `$${Math.round(amount)}`;
}

function formatQuota(quota, meta) {
  const amount = Number(quota) || 0;
  if (amount <= 0) return "";
  const unit = Number(meta?.quotaPerUnit) > 0 ? Number(meta.quotaPerUnit) : 500000;
  if (meta?.displayInCurrency === false) return t("amount.credits", Math.round(amount).toLocaleString());
  const symbols = { USD: "$", CNY: "¥", EUR: "€", JPY: "¥", CNH: "¥", RUB: "₽" };
  const symbol = symbols[(meta?.displayType || "USD").toUpperCase()] || "";
  const value = amount / unit * (Number(meta?.exchangeRate) > 0 ? Number(meta.exchangeRate) : 1);
  if (value >= 100) return `${symbol}${Math.round(value)}`;
  if (value >= 1) return `${symbol}${value.toFixed(2)}`;
  return `${symbol}${value.toFixed(4)}`;
}

function localDayKey() {
  const date = new Date();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

function positiveInt(value, fallback = 0) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
}

function openOverlay(id) {
  const overlay = document.getElementById(id);
  if (!overlay) return;
  overlay.classList.add("open");
  requestAnimationFrame(() => {
    overlay.querySelector("input, select, button")?.focus();
  });
}

function closeOverlay(id) {
  const overlay = document.getElementById(id);
  overlay?.classList.remove("open");
  if (id === "confirmOverlay") resolveConfirm(false);
  if (id === "promptOverlay") resolvePrompt(null);
}

function confirmAction(title, copy, acceptLabel) {
  elements.confirmTitle.textContent = title;
  elements.confirmCopy.textContent = copy;
  elements.confirmAcceptButton.textContent = acceptLabel || t("btn.accept");
  openOverlay("confirmOverlay");
  return new Promise(resolve => {
    confirmResolver = resolve;
  });
}

function resolveConfirm(value) {
  if (!confirmResolver) return;
  const resolver = confirmResolver;
  confirmResolver = null;
  elements.confirmOverlay.classList.remove("open");
  resolver(value);
}

function promptAction(title, copy, value = "", placeholder = "") {
  elements.promptTitle.textContent = title;
  elements.promptCopy.textContent = copy;
  elements.promptInput.value = value;
  elements.promptInput.placeholder = placeholder;
  elements.promptStatus.className = "form-status hidden";
  openOverlay("promptOverlay");
  return new Promise(resolve => {
    promptResolver = resolve;
  });
}

function submitPrompt() {
  resolvePrompt(elements.promptInput.value);
}

function resolvePrompt(value) {
  if (!promptResolver) return;
  const resolver = promptResolver;
  promptResolver = null;
  elements.promptOverlay.classList.remove("open");
  resolver(value);
}

function setFormStatus(text, tone) {
  elements.formStatus.textContent = text;
  elements.formStatus.className = text ? `form-status ${tone}` : "form-status hidden";
}

function setFooterStatus(text) {
  elements.footerStatus.textContent = text;
}

function showToast(text, tone = "accent") {
  window.clearTimeout(toastTimer);
  elements.toastText.textContent = text;
  const icon = { good: "circle-check", bad: "circle-x", warn: "triangle-alert" }[tone] || "info";
  elements.toastIcon.replaceChildren();
  const node = document.createElement("i");
  node.dataset.lucide = icon;
  elements.toastIcon.append(node);
  elements.toast.className = `toast visible tone-${tone}`;
  refreshIcons();
  toastTimer = window.setTimeout(() => {
    elements.toast.classList.remove("visible");
  }, TOAST_MS);
}

function handleShortcut(event) {
  const ctrl = event.ctrlKey || event.metaKey;
  if (ctrl && event.key.toLowerCase() === "r") {
    event.preventDefault();
    runCheckin(null);
  } else if (event.key === "F5") {
    event.preventDefault();
    loadState(true);
  } else if (ctrl && event.key.toLowerCase() === "f") {
    event.preventDefault();
    elements.searchInput.focus();
  } else if (ctrl && event.key.toLowerCase() === "i") {
    event.preventDefault();
    openOverlay("importOverlay");
  } else if (ctrl && event.key === ",") {
    event.preventDefault();
    openSettings();
  } else if (ctrl && event.key.toLowerCase() === "d") {
    event.preventDefault();
    toggleTheme();
  } else if (event.key === "Escape") {
    const open = [...document.querySelectorAll(".overlay.open")].pop();
    if (open) {
      closeOverlay(open.id);
    } else if (elements.searchInput.value) {
      elements.searchInput.value = "";
      elements.clearSearchButton.hidden = true;
      renderSites(false);
    }
  }
}

function handleMessage(event) {
  const message = event.data;
  if (message?.type === "progress") return handleProgress(message);
  if (message?.type === "status") return setFooterStatus(message.text || "");
  if (!message?.id || !pending.has(message.id)) return;
  const resolver = pending.get(message.id);
  pending.delete(message.id);
  if (message.ok) resolver.resolve(message.result);
  else resolver.reject(new Error(message.error || t("msg.callFailed")));
}

function handleProgress(message) {
  setProgress(message.done || 0, Math.max(1, message.total || 1));
  setFooterStatus(message.label || `${message.done || 0}/${message.total || 0}`);
  if (!message.result?.id) return;
  const site = findSite(message.result.id);
  if (!site) return;
  site.result = message.result;
  updateSiteRow(site);
  const counts = getCounts();
  renderHeader(counts);
  renderFooter(counts);
  if (selectedId === site.id) renderResult(site);
}

function call(action, payload = {}) {
  if (mockBridge) {
    return typeof window.__mockBridge === "function"
      ? window.__mockBridge(action, payload)
      : Promise.reject(new Error("Mock bridge unavailable"));
  }
  if (!window.chrome?.webview) {
    return Promise.reject(new Error("Bridge unavailable"));
  }
  sequence++;
  const id = sequence;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    window.chrome.webview.postMessage({ id, action, payload });
    window.setTimeout(() => {
      if (!pending.has(id)) return;
      pending.delete(id);
      reject(new Error(t("msg.timeout")));
    }, BRIDGE_TIMEOUT_MS);
  });
}

function refreshIcons() {
  if (!window.lucide) return;
  window.lucide.createIcons({
    attrs: {
      "stroke-width": 1.8,
      "aria-hidden": "true"
    }
  });
}
