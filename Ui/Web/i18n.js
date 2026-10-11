(function () {
  const TABLE = {
    "zh-cn": {
      "html.lang": "zh-CN",
      "app.title": "API 签到助手",
      "app.loading": "正在读取配置",
      "app.reloaded": "配置已重新读取",
      "app.emptyConfig": "还没有站点",
      "app.configuredOf": "{0}/{1} 个站点已配置",

      "header.configured": "{0} 个已配置",
      "header.auto": "{0} 个可自动",
      "header.manual": "{0} 个需手动",

      "btn.refresh": "重新读取",
      "btn.import": "新增站点",
      "btn.theme": "切换主题",
      "btn.settings": "设置",
      "btn.stop": "停止",
      "btn.runAll": "全部签到",
      "btn.add": "添加站点",
      "btn.addSite": "添加",
      "btn.probe": "探测",
      "btn.save": "保存设置",
      "btn.cancel": "取消",
      "btn.confirm": "确定",
      "btn.accept": "确定",
      "btn.close": "关闭",
      "btn.addGroup": "再添加一个站点",
      "btn.importNow": "新增并体检",
      "btn.clearAll": "清空全部令牌",
      "btn.saveCredential": "保存访问令牌",
      "btn.verify": "体检",
      "btn.clearCredential": "清空",
      "btn.probeSite": "检测站点",
      "btn.rename": "改名",
      "btn.runSite": "单独签到",
      "btn.remove": "移除",
      "btn.dataDir": "数据目录",
      "btn.search": "搜索站点",
      "btn.clearSearch": "清空搜索",
      "btn.openSite": "打开站点",
      "btn.removeGroup": "移除这个站点",

      "search.placeholder": "搜索站点",
      "filter.all": "全部",
      "filter.ready": "可签",
      "filter.problem": "待处理",
      "filter.good": "已完成",
      "filter.todo": "待配置",

      "add.host": "站点地址",
      "add.hostHint": "api.example.com",
      "add.name": "显示名称",
      "add.nameHint": "可留空",

      "list.sites": "站点",
      "list.emptyNone.title": "还没有站点",
      "list.emptyNone.copy": "添加地址后即可配置凭据并开始签到。",
      "list.emptyFiltered.title": "没有符合条件的站点",
      "list.emptyFiltered.copy": "换一个筛选条件或清空搜索。",

      "detail.empty.title": "选择一个站点查看详情",
      "detail.empty.copy": "也可以在左侧添加新的站点。",
      "detail.credential.title": "访问令牌",
      "detail.credential.copy": "保存访问令牌后，程序会自动体检。",
      "detail.result.title": "最近结果",
      "detail.result.empty": "还没有签到记录",
      "detail.result.hint": "完成第一次签到后，结果会显示在这里。",
      "detail.action.title": "操作",
      "detail.action.copy": "检测、打开、改名或移除当前站点。",

      "cred.token": "访问令牌",

      "field.token.label": "系统访问令牌",
      "field.token.hint": "在站点的个人设置中生成，通常以 sk- 开头。",
      "field.userId.label": "用户 ID",
      "field.userId.hint": "仅老版本站点需要。",
      "field.userId.auto": "自动读取",

      "tag.turnstile": "需人机验证",
      "tag.bound": "已绑定 {0}",
      "tag.userId": "用户 ID {0}",

      "footer.configured": "{0} / {1} 已配置",
      "footer.today": "{0} 今日签到",
      "footer.quota": "今日获得 {0}",
      "footer.problems": "{0} 需处理",

      "import.title": "新增站点",
      "import.copy": "填写新的站点地址和访问令牌，保存后自动体检。",
      "import.group": "新增站点 {0}",
      "import.name": "站名",
      "import.host": "站点地址",
      "import.type": "凭据类型",
      "import.typeToken": "访问令牌",
      "import.credential": "凭据内容",
      "import.userId": "用户 ID",

      "settings.title": "设置",
      "settings.copy": "并发、超时与外观。",
      "settings.concurrency": "并发数",
      "settings.concurrencyHint": "同时发请求的站点数量。",
      "settings.timeout": "请求超时（秒）",
      "settings.timeoutHint": "单个站点的最长等待时间。",
      "settings.theme": "主题",
      "settings.themeAuto": "跟随系统",
      "settings.themeLight": "浅色",
      "settings.themeDark": "深色",
      "settings.language": "界面语言",
      "settings.languageAuto": "跟随系统",
      "settings.notify": "结束提示",
      "settings.notifyHint": "签到完成后显示结果",
      "settings.skipManual": "跳过人机验证站点",
      "settings.skipManualHint": "需要 Turnstile 的站点只标记，不自动提交",

      "confirm.title": "确认操作",
      "prompt.title": "输入",

      "state.success": "签到成功",
      "state.already": "今日已签",
      "state.unauthorized": "需要登录",
      "state.challenge": "需手动",
      "state.offline": "站点不可达",
      "state.failed": "签到失败",
      "state.disabled": "未开放签到",
      "state.unsupported": "接口不支持",
      "state.skipped": "待配置",
      "state.idle": "待签到",
      "state.running": "进行中",

      "cred.unconfigured": "未配置",
      "cred.summaryToken": "令牌 {0}",

      "msg.hostRequired": "请先填写站点地址",
      "msg.nameRequired": "名称不能为空",
      "msg.nameUpdated": "站点名称已更新",
      "msg.added": "已添加 {0}",
      "msg.removed": "站点已移除",
      "msg.addFailed": "添加失败",
      "msg.probeRunning": "正在探测站点…",
      "msg.probeFailed": "探测失败",
      "msg.credentialSaved": "访问令牌已保存并通过体检",
      "msg.credentialSavedUnverified": "{0}，但体检未通过",
      "msg.credentialCleared": "访问令牌已清空",
      "msg.credentialsCleared": "已清空全部访问令牌。",
      "msg.clearAllTitle": "清空全部访问令牌",
      "msg.clearAllCopy": "会清掉所有站点的访问令牌，站点清单保留。",
      "msg.clearAllAccept": "清空",
      "msg.clearTitle": "清空访问令牌",
      "msg.clearCopy": "确定清空「{0}」的访问令牌吗？站点本身会保留。",
      "msg.renameTitle": "修改名称",
      "msg.renameCopy": "新的显示名称",
      "msg.removeTitle": "移除站点",
      "msg.removeCopy": "确定移除「{0}」吗？该站点的凭据记录会一起删除。",
      "msg.removeAccept": "移除",
      "msg.saveFailed": "保存失败",
      "msg.settingsSaved": "设置已保存",
      "msg.settingsFailed": "保存设置失败",
      "msg.themeFailed": "切换主题失败",
      "msg.savedAndChecked": "访问令牌可用",
      "msg.savedButUnchecked": "访问令牌不可用",
      "msg.operationStopped": "操作已停止",
      "msg.stopFailed": "停止失败",
      "msg.loadFailed": "读取配置失败",
      "msg.callFailed": "操作失败",
      "msg.timeout": "操作超时",

      "msg.tokenRequired": "请填写访问令牌。",
      "msg.needOneGroup": "先填写至少一个站点。",

      "run.summary": "签到完成：{0} 个成功{1}",
      "run.needsAttention": "，{0} 个要处理",
      "run.noneToRun": "没有可签到的站点",
      "run.failed": "{0}失败",
      "run.doing": "{0}中…",

      "import.summary": "已新增 {0} 个站点。",
      "import.partial": "{0} 个站点未新增：\n{1}",
      "import.verified": "已新增 {0} 个站点，体检完成：{1} 个可用，{2} 个需重配。",

      "relative.justNow": "刚刚",
      "relative.soon": "即将",
      "relative.minutes": "{0} 分钟前",
      "relative.hours": "{0} 小时前",
      "relative.days": "{0} 天前",

      "amount.credits": "{0} 额度",
      "amount.gained": "获得 {0}",
      "http.line": "HTTP {0}{1}",
      "http.path": " · {0}"
    },

    "en-us": {
      "html.lang": "en",
      "app.title": "API Check-in Helper",
      "app.loading": "Loading settings",
      "app.reloaded": "Settings reloaded",
      "app.emptyConfig": "No sites yet",
      "app.configuredOf": "{0}/{1} sites configured",

      "header.configured": "{0} configured",
      "header.auto": "{0} automatable",
      "header.manual": "{0} manual only",

      "btn.refresh": "Reload",
      "btn.import": "Add site",
      "btn.theme": "Toggle theme",
      "btn.settings": "Settings",
      "btn.stop": "Stop",
      "btn.runAll": "Run all",
      "btn.add": "Add site",
      "btn.addSite": "Add",
      "btn.probe": "Probe",
      "btn.save": "Save settings",
      "btn.cancel": "Cancel",
      "btn.confirm": "Confirm",
      "btn.accept": "Confirm",
      "btn.close": "Close",
      "btn.addGroup": "Add another site",
      "btn.importNow": "Add and verify",
      "btn.clearAll": "Clear all tokens",
      "btn.saveCredential": "Save access token",
      "btn.verify": "Verify",
      "btn.clearCredential": "Clear",
      "btn.probeSite": "Probe site",
      "btn.rename": "Rename",
      "btn.runSite": "Check in",
      "btn.remove": "Remove",
      "btn.dataDir": "Data folder",
      "btn.search": "Search sites",
      "btn.clearSearch": "Clear search",
      "btn.openSite": "Open site",
      "btn.removeGroup": "Remove this site",

      "search.placeholder": "Search sites",
      "filter.all": "All",
      "filter.ready": "Ready",
      "filter.problem": "Issues",
      "filter.good": "Done",
      "filter.todo": "Unset",

      "add.host": "Site address",
      "add.hostHint": "api.example.com",
      "add.name": "Display name",
      "add.nameHint": "Optional",

      "list.sites": "Sites",
      "list.emptyNone.title": "No sites yet",
      "list.emptyNone.copy": "Add an address to configure credentials and start checking in.",
      "list.emptyFiltered.title": "No matching sites",
      "list.emptyFiltered.copy": "Change the filter or clear the search.",

      "detail.empty.title": "Select a site to see details",
      "detail.empty.copy": "You can also add a new site on the left.",
      "detail.credential.title": "Access token",
      "detail.credential.copy": "Save the access token and the app verifies it for you.",
      "detail.result.title": "Latest result",
      "detail.result.empty": "No check-in recorded yet",
      "detail.result.hint": "Results appear here after the first check-in.",
      "detail.action.title": "Actions",
      "detail.action.copy": "Probe, open, rename or remove the current site.",

      "cred.token": "Access token",

      "field.token.label": "System access token",
      "field.token.hint": "Generated in the site's own settings, usually starts with sk-.",
      "field.userId.label": "User ID",
      "field.userId.hint": "Only needed on older sites.",
      "field.userId.auto": "auto",

      "tag.turnstile": "Human verification",
      "tag.bound": "Linked to {0}",
      "tag.userId": "User ID {0}",

      "footer.configured": "{0} / {1} configured",
      "footer.today": "{0} today",
      "footer.quota": "Gained {0} today",
      "footer.problems": "{0} need work",

      "import.title": "Add site",
      "import.copy": "Enter a new site address and access token; verification runs after saving.",
      "import.group": "New site {0}",
      "import.name": "Site name",
      "import.host": "Site address",
      "import.type": "Credential type",
      "import.typeToken": "Access token",
      "import.credential": "Credential content",
      "import.userId": "User ID",

      "settings.title": "Settings",
      "settings.copy": "Concurrency, timeout and appearance.",
      "settings.concurrency": "Concurrency",
      "settings.concurrencyHint": "How many sites are contacted at once.",
      "settings.timeout": "Request timeout (seconds)",
      "settings.timeoutHint": "Longest wait for a single site.",
      "settings.theme": "Theme",
      "settings.themeAuto": "Follow system",
      "settings.themeLight": "Light",
      "settings.themeDark": "Dark",
      "settings.language": "Language",
      "settings.languageAuto": "Follow system",
      "settings.notify": "Completion notice",
      "settings.notifyHint": "Show results after a run finishes",
      "settings.skipManual": "Skip human-verification sites",
      "settings.skipManualHint": "Sites behind Turnstile are flagged, never submitted",

      "confirm.title": "Confirm",
      "prompt.title": "Input",

      "state.success": "Checked in",
      "state.already": "Already done",
      "state.unauthorized": "Sign in needed",
      "state.challenge": "Manual only",
      "state.offline": "Unreachable",
      "state.failed": "Failed",
      "state.disabled": "Disabled",
      "state.unsupported": "Unsupported",
      "state.skipped": "Not set up",
      "state.idle": "Ready",
      "state.running": "Running",

      "cred.unconfigured": "Not configured",
      "cred.summaryToken": "Token {0}",

      "msg.hostRequired": "Enter a site address first",
      "msg.nameRequired": "Name cannot be empty",
      "msg.nameUpdated": "Site name updated",
      "msg.added": "Added {0}",
      "msg.removed": "Site removed",
      "msg.addFailed": "Could not add the site",
      "msg.probeRunning": "Probing site…",
      "msg.probeFailed": "Probe failed",
      "msg.credentialSaved": "Access token saved and verified",
      "msg.credentialSavedUnverified": "{0}, but verification failed",
      "msg.credentialCleared": "Access token cleared",
      "msg.credentialsCleared": "All access tokens cleared.",
      "msg.clearAllTitle": "Clear all access tokens",
      "msg.clearAllCopy": "This removes every access token. The site list is kept.",
      "msg.clearAllAccept": "Clear",
      "msg.clearTitle": "Clear access token",
      "msg.clearCopy": "Clear the access token for “{0}”? The site itself is kept.",
      "msg.renameTitle": "Rename",
      "msg.renameCopy": "New display name",
      "msg.removeTitle": "Remove site",
      "msg.removeCopy": "Remove “{0}”? Its stored credential is deleted too.",
      "msg.removeAccept": "Remove",
      "msg.saveFailed": "Save failed",
      "msg.settingsSaved": "Settings saved",
      "msg.settingsFailed": "Could not save settings",
      "msg.themeFailed": "Could not switch theme",
      "msg.savedAndChecked": "Access token works",
      "msg.savedButUnchecked": "Access token rejected",
      "msg.operationStopped": "Operation stopped",
      "msg.stopFailed": "Could not stop",
      "msg.loadFailed": "Could not read the configuration",
      "msg.callFailed": "Operation failed",
      "msg.timeout": "Operation timed out",

      "msg.tokenRequired": "Enter the access token.",
      "msg.needOneGroup": "Enter at least one site.",

      "run.summary": "Check-in finished: {0} succeeded{1}",
      "run.needsAttention": ", {0} need work",
      "run.noneToRun": "No site is ready to check in",
      "run.failed": "{0} failed",
      "run.doing": "{0}…",

      "import.summary": "Added {0} sites.",
      "import.partial": "{0} sites were not added:\n{1}",
      "import.verified": "Added {0} sites; verification: {1} usable, {2} to reconfigure.",

      "relative.justNow": "just now",
      "relative.soon": "soon",
      "relative.minutes": "{0} min ago",
      "relative.hours": "{0} h ago",
      "relative.days": "{0} d ago",

      "amount.credits": "{0} credits",
      "amount.gained": "gained {0}",
      "http.line": "HTTP {0}{1}",
      "http.path": " · {0}"
    }
  };

  const SUPPORTED = ["zh-cn", "en-us"];
  const FALLBACK = "zh-cn";
  let current = FALLBACK;

  function normalize(preference) {
    const value = String(preference || "auto").trim().toLowerCase();
    if (value.startsWith("zh")) return "zh-cn";
    if (value.startsWith("en")) return "en-us";
    return "auto";
  }

  function resolve(preference) {
    const direct = normalize(preference);
    if (direct !== "auto") return direct;
    const detected = normalize(navigator.language || "");
    return detected === "auto" ? FALLBACK : detected;
  }

  function t(key, ...args) {
    const table = TABLE[current] || TABLE["zh-cn"];
    let text = table[key];
    if (text === undefined) text = (TABLE["zh-cn"] || {})[key];
    if (text === undefined) return key;
    return text.replace(/\{(\d+)\}/g, (match, index) =>
      args[index] === undefined ? match : String(args[index])
    );
  }

  function apply(preference) {
    current = resolve(preference);
    document.documentElement.lang = t("html.lang");
    document.title = t("app.title");
    for (const node of document.querySelectorAll("[data-i18n]")) {
      node.textContent = t(node.dataset.i18n);
    }
    for (const node of document.querySelectorAll("[data-i18n-aria]")) {
      node.setAttribute("aria-label", t(node.dataset.i18nAria));
    }
    for (const node of document.querySelectorAll("[data-i18n-title]")) {
      node.setAttribute("title", t(node.dataset.i18nTitle));
    }
    for (const node of document.querySelectorAll("[data-i18n-placeholder]")) {
      node.setAttribute("placeholder", t(node.dataset.i18nPlaceholder));
    }
    document.dispatchEvent(new CustomEvent("i18n:applied", { detail: { language: current } }));
  }

  window.I18n = {
    t,
    apply,
    current: () => current,
    supported: SUPPORTED
  };
})();
