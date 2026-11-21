// BahaTech 23-Language Universal Desktop Software Localization Engine
// Automatically localizes ALL 50 standalone BahaTech Desktop OS (.html) apps & Master Hub into the buyer's chosen language!
(function () {
  const LANGS_23 = [
    { code: "tr", flag: "🇹🇷", native: "Türkçe", rtl: false },
    { code: "en", flag: "🇬🇧", native: "English", rtl: false },
    { code: "de", flag: "🇩🇪", native: "Deutsch", rtl: false },
    { code: "pl", flag: "🇵🇱", native: "Polski", rtl: false },
    { code: "ar", flag: "🇸🇦", native: "العربية", rtl: true },
    { code: "ru", flag: "🇷🇺", native: "Русский", rtl: false },
    { code: "fr", flag: "🇫🇷", native: "Français", rtl: false },
    { code: "es", flag: "🇪🇸", native: "Español", rtl: false },
    { code: "pt", flag: "🇧🇷", native: "Português", rtl: false },
    { code: "it", flag: "🇮🇹", native: "Italiano", rtl: false },
    { code: "nl", flag: "🇳🇱", native: "Nederlands", rtl: false },
    { code: "az", flag: "🇦🇿", native: "Azərbaycan", rtl: false },
    { code: "uk", flag: "🇺🇦", native: "Українська", rtl: false },
    { code: "el", flag: "🇬🇷", native: "Ελληνικά", rtl: false },
    { code: "fa", flag: "🇮🇷", native: "فارسی", rtl: true },
    { code: "he", flag: "🇮🇱", native: "עברית", rtl: true },
    { code: "hi", flag: "🇮🇳", native: "हिन्दी", rtl: false },
    { code: "bn", flag: "🇧🇩", native: "বাংলা", rtl: false },
    { code: "zh", flag: "🇨🇳", native: "中文", rtl: false },
    { code: "ja", flag: "🇯🇵", native: "日本語", rtl: false },
    { code: "ko", flag: "🇰🇷", native: "한국어", rtl: false },
    { code: "th", flag: "🇹🇭", native: "ไทย", rtl: false },
    { code: "ka", flag: "🇬🇪", native: "ქართული", rtl: false }
  ];

  const params = new URLSearchParams(window.location.search);
  const initial = window.BHT_LOCKED_DEFAULT_LANG || params.get("lang") || localStorage.getItem("bht_selected_lang_23") || "tr";
  let activeLang = (LANGS_23.find((x) => x.code === initial.toLowerCase()) || LANGS_23[0]).code;

  function getCache(lang) {
    try {
      return JSON.parse(localStorage.getItem("bht_app_i18n_cache_" + lang) || "{}");
    } catch {
      return {};
    }
  }
  function saveCache(lang, map) {
    try {
      localStorage.setItem("bht_app_i18n_cache_" + lang, JSON.stringify(map));
    } catch {}
  }

  async function translateBatchDirect(texts, lang) {
    // Works both on https://bahatechsoftware.com AND inside offline downloaded bespoke .html files!
    try {
      const apiRes = await fetch("/api/i18n-translate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ target: lang, texts })
      });
      if (apiRes.ok) {
        const d = await apiRes.json();
        if (Array.isArray(d.translations) && d.translations.length === texts.length) return d.translations;
      }
    } catch {}
    // Direct GTX fallback for standalone local file:// execution
    const tl = lang === "zh" ? "zh-CN" : lang === "he" ? "iw" : lang;
    const SEP = "\n⚡BHT⚡\n";
    const out = [];
    for (let i = 0; i < texts.length; i += 25) {
      const slice = texts.slice(i, i + 25);
      try {
        const u = "https://translate.googleapis.com/translate_a/single?client=gtx&sl=tr&tl=" + encodeURIComponent(tl) + "&dt=t&q=" + encodeURIComponent(slice.join(SEP));
        const r = await fetch(u);
        const j = await r.json();
        const joined = (j?.[0] || []).map((s) => s?.[0] || "").join("");
        const parts = joined.split(/\s*⚡\s*BHT\s*⚡\s*/);
        if (parts.length === slice.length) out.push(...parts.map((x) => x.trim()));
        else out.push(...slice);
      } catch {
        out.push(...slice);
      }
    }
    return out;
  }

  function collectTranslatableNodes() {
    const items = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, {
      acceptNode(node) {
        const p = node.parentElement;
        if (!p) return NodeFilter.FILTER_REJECT;
        const tag = p.tagName;
        if (["SCRIPT", "STYLE", "NOSCRIPT", "CODE"].includes(tag)) return NodeFilter.FILTER_REJECT;
        if (p.id === "bht-app-lang-select" || p.closest("#bht-app-lang-select")) return NodeFilter.FILTER_REJECT;
        const raw = node.__origTr !== undefined ? node.__origTr : node.nodeValue;
        if (!raw || !/[A-Za-zÇĞİÖŞÜçğıöşü]/.test(raw) || raw.trim().length < 2) return NodeFilter.FILTER_REJECT;
        return NodeFilter.FILTER_ACCEPT;
      }
    });
    let n;
    while ((n = walker.nextNode())) {
      if (n.__origTr === undefined) n.__origTr = n.nodeValue;
      items.push({ kind: "text", node: n, orig: n.__origTr.trim(), lead: n.__origTr.match(/^\s*/)[0], trail: n.__origTr.match(/\s*$/)[0] });
    }
    document.querySelectorAll("input[placeholder], textarea[placeholder], [title]").forEach((el) => {
      if (el.hasAttribute("placeholder")) {
        if (el.__origPh === undefined) el.__origPh = el.getAttribute("placeholder");
        if (el.__origPh && el.__origPh.trim().length > 1) items.push({ kind: "ph", el, orig: el.__origPh.trim() });
      }
      if (el.hasAttribute("title")) {
        if (el.__origTitle === undefined) el.__origTitle = el.getAttribute("title");
        if (el.__origTitle && el.__origTitle.trim().length > 1) items.push({ kind: "title", el, orig: el.__origTitle.trim() });
      }
    });
    return items;
  }

  let translating = false;
  async function applyAppLanguage(lang, persist) {
    const spec = LANGS_23.find((x) => x.code === lang) || LANGS_23[0];
    activeLang = spec.code;
    if (persist) {
      try { localStorage.setItem("bht_selected_lang_23", spec.code); } catch {}
    }
    document.documentElement.lang = spec.code;
    document.documentElement.dir = spec.rtl ? "rtl" : "ltr";
    const sel = document.getElementById("bht-app-lang-select");
    if (sel && sel.value !== spec.code) sel.value = spec.code;

    const items = collectTranslatableNodes();
    if (spec.code === "tr") {
      items.forEach((it) => {
        if (it.kind === "text") it.node.nodeValue = it.lead + it.orig + it.trail;
        else if (it.kind === "ph") it.el.setAttribute("placeholder", it.orig);
        else if (it.kind === "title") it.el.setAttribute("title", it.orig);
      });
      return;
    }

    if (translating) return;
    translating = true;
    try {
      const cache = Object.assign({}, window.BHT_PREBAKED_DICT || {}, getCache(spec.code));
      const missing = Array.from(new Set(items.map((x) => x.orig).filter((t) => !cache[t])));
      if (missing.length > 0) {
        const translated = await translateBatchDirect(missing, spec.code);
        missing.forEach((orig, idx) => {
          if (translated[idx]) cache[orig] = translated[idx];
        });
        saveCache(spec.code, cache);
      }
      items.forEach((it) => {
        const tr = cache[it.orig] || it.orig;
        if (it.kind === "text") it.node.nodeValue = it.lead + tr + it.trail;
        else if (it.kind === "ph") it.el.setAttribute("placeholder", tr);
        else if (it.kind === "title") it.el.setAttribute("title", tr);
      });
    } finally {
      translating = false;
    }
  }

  function mountAppLangBar() {
    const host = document.querySelector(".os-status-pills") || document.querySelector(".topbar .badges") || document.querySelector(".hub-top div:last-child");
    if (host && !document.getElementById("bht-app-lang-select")) {
      const sel = document.createElement("select");
      sel.id = "bht-app-lang-select";
      sel.style.cssText = "background:#0e1629;color:#fde68a;border:1px solid rgba(212,175,106,0.55);border-radius:999px;padding:5px 11px;font-size:11.5px;font-weight:800;cursor:pointer;";
      sel.innerHTML = LANGS_23.map((l) => `<option value="${l.code}">${l.flag} ${l.native} (${l.code.toUpperCase()})</option>`).join("");
      sel.value = activeLang;
      sel.addEventListener("change", () => applyAppLanguage(sel.value, true));
      host.insertBefore(sel, host.firstChild);
    }

    // Re-translate dynamic recalculations or tab switches seamlessly!
    document.addEventListener("click", (e) => {
      if (activeLang !== "tr" && e.target.closest("button")) {
        setTimeout(() => applyAppLanguage(activeLang, false), 60);
      }
    });

    applyAppLanguage(activeLang, false);
  }

  window.applyBahaTechAppLanguage = applyAppLanguage;
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", mountAppLangBar);
  else mountAppLangBar();
})();
