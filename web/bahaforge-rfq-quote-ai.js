/**
 * BahaForge CAD CAM AI Studio — Talaşlı İmalat Gelen Teklifler (RFQ),
 * 2D Teknik Resim AI Fiyat Yorumlama, Malzeme + İşçilik Hesabı ve Resmi Teklif Maili Stüdyosu.
 * %100 Yerel çalışır; canlı studio_cad_server.py (/api/rfq-analyze) varsa 2D PDF/görsel teknik resmi AI ile okur.
 */
(function () {
  "use strict";
  if (window.__BAHAFORGE_RFQ_LOADED__) return;
  window.__BAHAFORGE_RFQ_LOADED__ = true;

  const RFQ_STORE_KEY = "bahaforge_rfq_inbox_v7";
  const ML_MEM_KEY = "bahaforge_local_ml_memory_v1";

  function loadMlMemory() {
    try {
      const raw = localStorage.getItem(ML_MEM_KEY);
      if (raw) {
        const arr = JSON.parse(raw);
        if (Array.isArray(arr)) return arr;
      }
    } catch (_) {}
    return [];
  }

  function saveMlMemory(arr) {
    localStorage.setItem(ML_MEM_KEY, JSON.stringify(arr));
  }

  // Embedded Zero-Cloud Self-Learning Weighted k-NN + Volume-Scaled CNC Pricing AI
  function localMlPredict(sample, mem) {
    if (!mem || !mem.length) {
      return {
        trained: 0,
        confidence: 0,
        stage: "🎓 İlk Eğitim Modu (Hafıza Boş)",
        explanation: "Henüz kayıtlı fiyat hafızası yok. Bu ilk parçanın gerçek fiyat/sürelerini girip 'Kaydet & Yapay Zekayı Eğit' butonuna basın; sonraki PDF'lerde motor otomatik tahmin yapmaya başlayacak.",
        prediction: null,
      };
    }
    const volTarget = sample.shape === "cylindrical"
      ? Math.PI * Math.pow((sample.dimD || 100) / 20, 2) * ((sample.dimZ || 25) / 10)
      : ((sample.dimX || 100) * (sample.dimY || 80) * (sample.dimZ || 25)) / 1000;

    const scored = mem.map(m => {
      let dist = 0;
      if (m.materialKey !== sample.materialKey) dist += 1.6;
      if (m.shape !== sample.shape) dist += 0.9;
      const vRatio = Math.abs(Math.log((Math.max(volTarget, 1)) / Math.max(m.volCm3 || 1, 1)));
      dist += vRatio * 1.1;
      const qRatio = Math.abs(Math.log(Math.max(sample.qty || 1, 1) / Math.max(m.qty || 1, 1)));
      dist += qRatio * 0.25;
      const sim = 1 / (1 + dist);
      return { m, sim, vScale: Math.max(0.45, Math.min(2.6, Math.pow(Math.max(volTarget, 1) / Math.max(m.volCm3 || 1, 1), 0.38))) };
    }).sort((a, b) => b.sim - a.sim).slice(0, 5);

    let wSum = 0, turn = 0, mill = 0, edm = 0, setup = 0, kgP = 0, dMat = 0, tRate = 0, mRate = 0, marg = 0, scrap = 0, unitP = 0;
    scored.forEach(({ m, sim, vScale }) => {
      const w = sim * sim;
      wSum += w;
      turn  += w * (m.turnMin || 0) * vScale;
      mill  += w * (m.millMin || 0) * vScale;
      edm   += w * (m.edmMin || 0) * vScale;
      setup += w * (m.setupMin || 45);
      kgP   += w * (m.kgPrice || (ALLOYS[sample.materialKey]?.priceKg || 150));
      dMat  += w * (m.directMatUnit || 160);
      tRate += w * (m.turnRate || 1650);
      mRate += w * (m.millRate || 2100);
      marg  += w * (m.marginPct || 28);
      scrap += w * (m.scrapPct || 15);
      unitP += w * (m.sellUnit || 0);
    });

    const n = mem.length;
    const topSim = scored[0].sim;
    const conf = Math.min(97, Math.round((Math.min(n, 12) / 12) * 58 + topSim * 40));
    const stage = n < 3
      ? `🌱 Öğreniyor (${n} Parça Hafızada)`
      : n < 10
        ? `📈 Yarı-Otonom Tahmin (${n} Parça)`
        : `⚡ Otonom Fiyat Motoru (${n} Parça)`;

    return {
      trained: n,
      confidence: conf,
      stage,
      matchedPart: `${scored[0].m.partName} (${scored[0].m.partCode}) — %${Math.round(topSim * 100)} benzer`,
      explanation: `Yerel Öğrenen Yapay Zeka (${n} geçmiş işten öğrendi, Güven %${conf}): En yakın referans "${scored[0].m.partName} (${scored[0].m.partCode})". Tezgâh saatleri, malzeme kg fiyatı ve kâr marjı sizin atölyenizin gerçek geçmişine göre otomatik dolduruldu!`,
      prediction: {
        turnMin: Math.round(turn / wSum),
        millMin: Math.round(mill / wSum),
        edmMin: Math.round(edm / wSum),
        setupMin: Math.round(setup / wSum),
        kgPrice: Math.round(kgP / wSum),
        directMatUnit: Math.round(dMat / wSum),
        turnRate: Math.round((tRate / wSum) / 50) * 50,
        millRate: Math.round((mRate / wSum) / 50) * 50,
        marginPct: Math.round(marg / wSum),
        scrapPct: Math.round(scrap / wSum),
        estUnitTl: Math.round(unitP / wSum),
      },
    };
  }

  const ALLOYS = {
    al7075:    { name: "Alüminyum 7075-T6 (Havacılık)", density: 2.81, priceKg: 420 },
    al6061:    { name: "Alüminyum 6061-T6",             density: 2.70, priceKg: 295 },
    st37:      { name: "St37 / S235JR Yapı Çeliği",     density: 7.85, priceKg: 58  },
    steel4140: { name: "4140 Islah Çeliği (42CrMo4)",   density: 7.85, priceKg: 115 },
    ss304:     { name: "AISI 304 Paslanmaz Çelik",      density: 7.93, priceKg: 245 },
    ss316:     { name: "AISI 316L Paslanmaz Çelik",     density: 7.98, priceKg: 315 },
    ti64:      { name: "Titanyum Ti-6Al-4V (Grade 5)",  density: 4.43, priceKg: 2650 },
    brass:     { name: "Pirinç MS58 (CuZn39Pb3)",       density: 8.47, priceKg: 460 },
    pom:       { name: "Delrin / POM Mühendislik Plast.", density: 1.41, priceKg: 260 },
  };

  const DEFAULT_RFQS = [
    {
      id: "RFQ-2026-101",
      date: new Date().toLocaleDateString("tr-TR"),
      customer: "Karadağ Makina / Havacılık Yan Sanayi",
      contact: "Ümit Karadağ",
      email: "info@karadagmakina.net",
      phone: "905534503041",
      partName: "CNC İşlenmiş Montaj Flanşı",
      partCode: "FLN-304-Ø120",
      materialKey: "ss304",
      shape: "cylindrical",
      dimD: 120, dimX: 120, dimY: 120, dimZ: 33,
      scrapPct: 18,
      qty: 50,
      turnMin: 18, turnRate: 1650,
      millMin: 22, millRate: 2100,
      edmMin: 0, edmRate: 1950,
      setupMin: 45, toolPct: 8, outsideUnit: 120, marginPct: 28,
      leadDays: 7,
      tolerance: "ISO 2768-m / Ø40 H7",
      comment: "AISI 304 paslanmazda talaş kırıcı geometrili kaplamalı uç önerilir; 6x Ø10 delik ve Ø40 H7 merkez delik 2 bağlamada işlenmelidir.",
      status: "Fiyatlandırıldı",
    },
    {
      id: "RFQ-2026-102",
      date: new Date().toLocaleDateString("tr-TR"),
      customer: "Eskişehir Savunma & Aktüatör A.Ş.",
      contact: "Mühendislik Satın Alma",
      email: "satinalma@ornek-savunma.com.tr",
      phone: "905320000000",
      partName: "5-Eksen Servo Gövde Bloğu",
      partCode: "SRV-7075-09",
      materialKey: "al7075",
      shape: "prismatic",
      dimD: 0, dimX: 145, dimY: 95, dimZ: 48,
      scrapPct: 22,
      qty: 25,
      turnMin: 0, turnRate: 1650,
      millMin: 42, millRate: 2600,
      edmMin: 0, edmRate: 1950,
      setupMin: 75, toolPct: 7, outsideUnit: 240, marginPct: 32,
      leadDays: 10,
      tolerance: "±0.02 mm • Sert Eloksal",
      comment: "7075-T6 kütükte VoluMill yüksek hızlı kaba boşaltma ile işleme süresi %35 kısaltılabilir; sert eloksal fason bedeli eklendi.",
      status: "Yeni RFQ",
    },
  ];

  function loadRfqs() {
    try {
      const raw = localStorage.getItem(RFQ_STORE_KEY);
      if (raw) {
        const arr = JSON.parse(raw);
        if (Array.isArray(arr) && arr.length) return arr;
      }
    } catch (_) {}
    localStorage.setItem(RFQ_STORE_KEY, JSON.stringify(DEFAULT_RFQS));
    return JSON.parse(JSON.stringify(DEFAULT_RFQS));
  }

  function saveRfqs(arr) {
    localStorage.setItem(RFQ_STORE_KEY, JSON.stringify(arr));
  }

  function fmt(n) {
    return Number(n || 0).toLocaleString("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  function calcQuote(f) {
    const alloy = ALLOYS[f.materialKey] || ALLOYS.steel4140;
    const priceKg = Number(f.kgPrice ?? alloy.priceKg);
    const density = alloy.density; // g/cm3
    const qty = Math.max(1, Number(f.qty) || 1);
    const mode = f.pricingMode || "unit"; // "unit" | "grams" | "billet" | "fason"

    let volCm3 = 0;
    let netKg = 0;
    let grossKg = 0;
    let billetLabel = "";
    let matUnit = 0;

    if (mode === "fason") {
      billetLabel = "Müşteri Malzemeli (Sadece Fason İşçilik)";
      netKg = (Number(f.netGrams) || 0) / 1000.0;
      grossKg = netKg;
      matUnit = 0;
    } else if (mode === "unit") {
      netKg = (Number(f.netGrams) || 0) / 1000.0;
      grossKg = netKg;
      matUnit = Math.max(0, Number(f.directMatUnit) || 0);
      billetLabel = `Parça Başı Değerleme${netKg > 0 ? ` (${(netKg * 1000).toFixed(0)} g/ad)` : ""}`;
    } else if (mode === "grams") {
      netKg = Math.max(0, Number(f.netGrams) || 0) / 1000.0;
      grossKg = netKg * (1 + (Number(f.scrapPct) || 0) / 100.0);
      matUnit = grossKg * priceKg;
      billetLabel = `Net ${(netKg * 1000).toFixed(0)} g → Fireli ${grossKg.toFixed(3)} kg/ad`;
    } else {
      // billet mm -> kg
      if (f.shape === "cylindrical") {
        const rCm = (Number(f.dimD) || 0) / 20.0;
        const lCm = (Number(f.dimZ) || 0) / 10.0;
        volCm3 = Math.PI * rCm * rCm * lCm;
        billetLabel = `Ø${f.dimD} × ${f.dimZ} mm (Silindirik Kütük)`;
      } else {
        volCm3 = ((Number(f.dimX) || 0) * (Number(f.dimY) || 0) * (Number(f.dimZ) || 0)) / 1000.0;
        billetLabel = `${f.dimX} × ${f.dimY} × ${f.dimZ} mm (Prizmatik Kütük)`;
      }
      netKg = (volCm3 * density) / 1000.0;
      grossKg = netKg * (1 + (Number(f.scrapPct) || 0) / 100.0);
      matUnit = grossKg * priceKg;
    }

    const matTotal = matUnit * qty;

    const turnUnit = ((Number(f.turnMin) || 0) / 60.0) * (Number(f.turnRate) || 0);
    const millUnit = ((Number(f.millMin) || 0) / 60.0) * (Number(f.millRate) || 0);
    const edmUnit  = ((Number(f.edmMin)  || 0) / 60.0) * (Number(f.edmRate)  || 0);
    const setupRate = Math.max(Number(f.millRate) || 0, Number(f.turnRate) || 0, 1500);
    const setupTotal = ((Number(f.setupMin) || 0) / 60.0) * setupRate;
    const setupUnit = setupTotal / qty;

    const rawMachUnit = turnUnit + millUnit + edmUnit + setupUnit;
    const toolUnit = rawMachUnit * ((Number(f.toolPct) || 0) / 100.0);
    const outsideUnit = Number(f.outsideUnit) || 0;
    const laborUnit = rawMachUnit + toolUnit + outsideUnit;
    const laborTotal = laborUnit * qty;

    const costUnit = matUnit + laborUnit;
    const sellUnit = costUnit * (1 + (Number(f.marginPct) || 0) / 100.0);
    const subTotal = sellUnit * qty;
    const vat = subTotal * 0.20;
    const grandTotal = subTotal + vat;

    return {
      alloy, billetLabel, priceKg, qty, mode,
      netKg, grossKg,
      matUnit, matTotal,
      turnUnit, millUnit, edmUnit, setupUnit, toolUnit, outsideUnit,
      laborUnit, laborTotal,
      costUnit, sellUnit, subTotal, vat, grandTotal,
    };
  }

  function injectStyles() {
    if (document.getElementById("bf-rfq-styles")) return;
    const s = document.createElement("style");
    s.id = "bf-rfq-styles";
    s.textContent = `
      .bf-rfq-wrap { display: grid; grid-template-columns: 1.08fr 0.92fr; gap: 14px; }
      @media (max-width: 1120px) { .bf-rfq-wrap { grid-template-columns: 1fr; } }
      .bf-dropzone {
        border: 2px dashed rgba(56,189,248,0.45); border-radius: 12px; padding: 16px;
        background: rgba(14,165,233,0.06); text-align: center; cursor: pointer; transition: 0.16s;
      }
      .bf-dropzone:hover, .bf-dropzone.drag { border-color: #38bdf8; background: rgba(14,165,233,0.14); }
      .bf-ai-box {
        margin-top: 10px; padding: 11px 13px; border-radius: 10px;
        background: rgba(16,185,129,0.09); border: 1px solid rgba(16,185,129,0.35);
        font-size: 12px; line-height: 1.5; color: #d1fae5;
      }
      .bf-grid-3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
      .bf-grid-2 { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
      .bf-field { display: flex; flex-direction: column; gap: 4px; }
      .bf-field label { font-size: 11px; color: #94a3b8; font-weight: 700; }
      .bf-field input, .bf-field select, .bf-field textarea {
        background: #060a14; border: 1px solid rgba(255,255,255,0.14); color: #f8fafc;
        border-radius: 8px; padding: 7px 9px; font-size: 12.5px; width: 100%; font-family: inherit;
      }
      .bf-kpi-strip { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; margin-bottom: 12px; }
      .bf-kpi {
        background: rgba(15,23,42,0.88); border: 1px solid rgba(255,255,255,0.10);
        border-radius: 10px; padding: 10px 11px;
      }
      .bf-kpi .k { font-size: 10.5px; color: #94a3b8; font-weight: 700; text-transform: uppercase; }
      .bf-kpi .v { font-size: 16px; font-weight: 900; color: #38bdf8; margin-top: 3px; }
      .bf-kpi.green .v { color: #34d399; }
      .bf-kpi.amber .v { color: #fbbf24; }
      .bf-kpi.violet .v { color: #c084fc; }
      .bf-sec-title {
        font-size: 12px; font-weight: 800; color: #38bdf8; letter-spacing: 0.3px;
        text-transform: uppercase; margin: 12px 0 7px; padding-bottom: 4px;
        border-bottom: 1px solid rgba(56,189,248,0.22);
      }
      .bf-mail-box {
        width: 100%; min-height: 270px; background: #040812; color: #e2e8f0;
        border: 1px solid rgba(56,189,248,0.32); border-radius: 10px; padding: 11px;
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 11.8px; line-height: 1.5;
      }
      .bf-badge { display: inline-block; padding: 3px 8px; border-radius: 999px; font-size: 10.5px; font-weight: 800; }
      .bf-badge.new { background: rgba(56,189,248,0.18); color: #7dd3fc; }
      .bf-badge.quoted { background: rgba(251,191,36,0.18); color: #fde68a; }
      .bf-badge.sent { background: rgba(168,85,247,0.20); color: #e9d5ff; }
      .bf-badge.won { background: rgba(16,185,129,0.20); color: #6ee7b7; }
      .bf-preview-thumb { max-height: 130px; max-width: 100%; border-radius: 8px; border: 1px solid rgba(255,255,255,0.16); margin-top: 8px; display: none; }
    `;
    document.head.appendChild(s);
  }

  function mountRfqStudio() {
    injectStyles();
    const navList = document.querySelector(".nav-list");
    const mainWrap = document.querySelector(".main-content") || document.querySelector("main") || document.getElementById("pane-engine")?.parentElement;
    if (!navList || !mainWrap) return;

    // Add Sidebar Navigation Button as TOP priority tab
    const navBtn = document.createElement("button");
    navBtn.className = "nav-btn";
    navBtn.id = "nav-rfq";
    navBtn.dataset.tab = "rfq";
    navBtn.innerHTML = `<span>📥 Teklif & AI Fiyat (RFQ)</span><span class="nav-kbd" id="bfRfqCountBadge">2</span>`;
    navList.insertBefore(navBtn, navList.firstChild);

    // Create Main Tab Pane
    const pane = document.createElement("section");
    pane.className = "tab-pane";
    pane.id = "pane-rfq";
    pane.innerHTML = `
      <div class="bf-kpi-strip" id="bfKpiStrip"></div>

      <div class="bf-rfq-wrap">
        <!-- SOL KOLON: TEKNİK RESİM AI + MALZEME + İŞÇİLİK -->
        <div class="card">
          <div class="card-title">
            <span>📐 Teknik Resim AI Fiyat Yorumlama • Malzeme &amp; İşçilik Hesabı</span>
            <span style="font-size:11px;color:#34d399;font-weight:800;">⚡ Yerel + OpenCascade AI</span>
          </div>

          <div id="bfMlStatusBar" style="margin-bottom:10px;padding:10px 12px;border-radius:10px;background:linear-gradient(135deg,rgba(99,102,241,0.14),rgba(14,165,233,0.10));border:1px solid rgba(129,140,248,0.40);display:flex;align-items:center;justify-content:space-between;gap:10px;flex-wrap:wrap;">
            <div>
              <div style="font-size:12px;font-weight:900;color:#c7d2fe;" id="bfMlStageText">🧠 Gömülü Yerel Öğrenen Fiyat AI • 🎓 İlk Eğitim Modu (0 Parça)</div>
              <div style="font-size:11px;color:#94a3b8;margin-top:2px;" id="bfMlSubText">Siz PDF yükleyip gerçek fiyat girdikçe motor hafızasını güçlendirir, sonraki PDF'lerde kendi rakamlarını otomatik tahmin eder (%100 yerel, sıfır bulut).</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px;">
              <div style="width:96px;height:8px;background:rgba(255,255,255,0.10);border-radius:999px;overflow:hidden;"><div id="bfMlConfFill" style="height:100%;width:8%;background:linear-gradient(90deg,#f59e0b,#34d399);transition:width 0.3s;"></div></div>
              <span id="bfMlConfPct" style="font-size:11.5px;font-weight:900;color:#6ee7b7;">%0</span>
              <button type="button" class="btn sm" id="bfRunMlNowBtn" title="Mevcut ölçülere göre hafızadan tahmin yap">🧠 Hafızadan Tahmin Et</button>
            </div>
          </div>

          <div class="bf-dropzone" id="bfDropzone">
            <input type="file" id="bfFileInp" accept=".pdf,.png,.jpg,.jpeg,.webp,application/pdf,image/*" style="display:none" />
            <div style="font-size:13.5px;font-weight:800;color:#e0f2fe;">📄 2D Teknik Resim PDF veya Görsel Sürükle-Bırak (veya Tıkla)</div>
            <div style="font-size:11.5px;color:#94a3b8;margin-top:4px;">Yapay zeka ölçüleri, malzemeyi, toleransı ve CNC operasyon sürelerini okuyup fiyat yorumunu otomatik çıkarır</div>
            <div style="margin-top:9px;">
              <button type="button" class="btn primary sm" id="bfPickFileBtn">📂 PDF / Teknik Resim Seç</button>
            </div>
            <img id="bfPreviewImg" class="bf-preview-thumb" alt="Teknik Resim Önizleme" />
          </div>

          <div id="bfAiBox" class="bf-ai-box">
            <strong>🤖 Mühendislik Fiyat Yorumu:</strong>
            <span id="bfAiCommentText">Teknik resim PDF yükleyin veya aşağıdaki ölçüleri girin; malzeme kütük ağırlığı, tezgâh saatleri ve müşteri teklif maili anında hazırlansın.</span>
          </div>

          <div class="bf-sec-title">1. Müşteri &amp; Gelen Teklif (RFQ) Bilgileri</div>
          <div class="bf-grid-3">
            <div class="bf-field"><label>Müşteri / Firma Adı</label><input id="bfCust" value="Karadağ Makina" /></div>
            <div class="bf-field"><label>Yetkili Kişi</label><input id="bfContact" value="Ümit Karadağ" /></div>
            <div class="bf-field"><label>Müşteri E-Posta</label><input id="bfEmail" value="info@karadagmakina.net" /></div>
            <div class="bf-field"><label>WhatsApp / Tel</label><input id="bfPhone" value="905534503041" /></div>
            <div class="bf-field"><label>Parça Adı</label><input id="bfPartName" value="CNC İşlenmiş Montaj Flanşı" /></div>
            <div class="bf-field"><label>Teknik Resim / Parça Kodu</label><input id="bfPartCode" value="FLN-304-Ø120" /></div>
          </div>

          <div class="bf-sec-title">2. Fiyatlandırma Modu &amp; Malzeme Hesabı (Parça Başı / Gram / Kütük)</div>
          <div class="bf-grid-3">
            <div class="bf-field">
              <label>Fiyatlandırma Yöntemi</label>
              <select id="bfPricingMode">
                <option value="unit" selected>🎯 Parça Başı Değerleme (Adet ₺ — Önerilen)</option>
                <option value="grams">⚖️ Gerçek Parça Ağırlığı (Gram → ₺)</option>
                <option value="billet">📐 Kütük Ölçüsünden Hesapla (mm → kg)</option>
                <option value="fason">🛠️ Müşteri Malzemeli / Sadece Fason İşçilik (0 ₺)</option>
              </select>
            </div>
            <div class="bf-field">
              <label>Alaşım / Malzeme</label>
              <select id="bfMatKey">
                ${Object.entries(ALLOYS).map(([k, v]) => `<option value="${k}">${v.name} (${v.density} g/cm³)</option>`).join("")}
              </select>
            </div>
            <div class="bf-field" id="bfWrapDirectMat"><label>Parça Başı Malzeme Bedeli (₺/adet)</label><input id="bfDirectMatUnit" type="number" step="10" value="180" /></div>

            <div class="bf-field" id="bfWrapGrams"><label>Parça Net Ağırlığı (Gram — Örn: 450 g)</label><input id="bfNetGrams" type="number" step="10" value="450" /></div>
            <div class="bf-field" id="bfWrapKgPrice" style="display:none"><label>Kg Birim Fiyatı (₺/kg)</label><input id="bfKgPrice" type="number" step="5" value="245" /></div>
            <div class="bf-field" id="bfWrapShape" style="display:none">
              <label>Kütük Formu</label>
              <select id="bfShape">
                <option value="cylindrical">Silindirik Mil / Boru (ØD × L)</option>
                <option value="prismatic">Prizmatik Blok / Lama (X × Y × Z)</option>
              </select>
            </div>

            <div class="bf-field" id="bfWrapD" style="display:none"><label>Dış Çap ØD (mm)</label><input id="bfDimD" type="number" step="1" value="60" /></div>
            <div class="bf-field" id="bfWrapX" style="display:none"><label>En X (mm)</label><input id="bfDimX" type="number" step="1" value="80" /></div>
            <div class="bf-field" id="bfWrapY" style="display:none"><label>Boy Y (mm)</label><input id="bfDimY" type="number" step="1" value="60" /></div>
            <div class="bf-field" id="bfWrapZ" style="display:none"><label>Uzunluk / Kalınlık Z (mm)</label><input id="bfDimZ" type="number" step="1" value="20" /></div>
            <div class="bf-field" id="bfWrapScrap" style="display:none"><label>Kesim &amp; Talaş Fire Payı (%)</label><input id="bfScrapPct" type="number" step="1" value="15" /></div>
          </div>

          <div class="bf-sec-title">3. İşçilik, Tezgâh Süresi &amp; Kâr Marjı</div>
          <div class="bf-grid-3">
            <div class="bf-field"><label>Sipariş Adedi</label><input id="bfQty" type="number" min="1" value="50" /></div>
            <div class="bf-field"><label>CNC Torna (dk/adet)</label><input id="bfTurnMin" type="number" step="1" value="18" /></div>
            <div class="bf-field"><label>Torna Saat Ücreti (₺/sa)</label><input id="bfTurnRate" type="number" step="50" value="1650" /></div>

            <div class="bf-field"><label>3/5-Eksen Freze (dk/adet)</label><input id="bfMillMin" type="number" step="1" value="22" /></div>
            <div class="bf-field"><label>Freze Saat Ücreti (₺/sa)</label><input id="bfMillRate" type="number" step="50" value="2100" /></div>
            <div class="bf-field"><label>Tel Erozyon / Taşlama (dk)</label><input id="bfEdmMin" type="number" step="1" value="0" /></div>

            <div class="bf-field"><label>Setup / Bağlama (dk/parti)</label><input id="bfSetupMin" type="number" step="5" value="45" /></div>
            <div class="bf-field"><label>Takım / Elmas Uç Payı (%)</label><input id="bfToolPct" type="number" step="1" value="8" /></div>
            <div class="bf-field"><label>Isıl İşlem / Kaplama (₺/ad)</label><input id="bfOutside" type="number" step="10" value="120" /></div>

            <div class="bf-field"><label>Kâr Marjı (%)</label><input id="bfMargin" type="number" step="1" value="28" /></div>
            <div class="bf-field"><label>Termin Süresi (İş Günü)</label><input id="bfLeadDays" type="number" step="1" value="7" /></div>
            <div class="bf-field"><label>Tolerans / Kalite Notu</label><input id="bfTol" value="ISO 2768-m / Ø40 H7" /></div>
          </div>

          <div style="display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;">
            <button class="btn primary" id="bfSaveRfqBtn">🧠 Kaydet &amp; Yerel Yapay Zekayı Eğit (RFQ)</button>
            <button class="btn" id="bfNewRfqBtn">➕ Yeni Boş Teklif</button>
          </div>
        </div>

        <!-- SAĞ KOLON: MALZEME+İŞÇİLİK KIRILIMI + MÜŞTERİYE TEKLİF MAİLİ -->
        <div class="card">
          <div class="card-title">
            <span>✉️ Müşteriye Malzeme + İşçilik Teklif Maili Hazırlayıcı</span>
            <select id="bfMailLang" style="background:#060a14;border:1px solid rgba(255,255,255,0.18);color:#fff;border-radius:7px;padding:4px 8px;font-size:11.5px;">
              <option value="tr">🇹🇷 Türkçe Kurumsal Teklif</option>
              <option value="en">🇬🇧 English Export Quotation</option>
            </select>
          </div>

          <div id="bfBreakdownBox" style="background:rgba(2,6,23,0.75);border:1px solid rgba(255,255,255,0.1);border-radius:10px;padding:10px 12px;margin-bottom:10px;font-size:12px;line-height:1.6;"></div>

          <div class="bf-field" style="margin-bottom:8px;">
            <label>E-Posta Konu Satırı (Subject)</label>
            <input id="bfMailSubject" />
          </div>
          <div class="bf-field" style="margin-bottom:10px;">
            <label>Teklif Maili Gövdesi (Düzenlenebilir)</label>
            <textarea id="bfMailBody" class="bf-mail-box"></textarea>
          </div>

          <div style="display:flex;flex-wrap:wrap;gap:8px;">
            <button class="btn primary" id="bfOpenMailtoBtn">📧 Mail Uygulamasında Aç (Gönder)</button>
            <button class="btn" id="bfCopyMailBtn">📋 Maili Kopyala</button>
            <button class="btn wa" id="bfSendWaBtn">📲 WhatsApp Teklifi Aç</button>
            <button class="btn" id="bfPrintQuoteBtn">🖨️ A4 Teklif PDF</button>
          </div>
        </div>
      </div>

      <!-- ALT PANEL: GELEN TEKLİFLER (RFQ) GELEN KUTUSU TABLOSU -->
      <div class="card" style="margin-top:14px;">
        <div class="card-title">
          <span>📥 Gelen Teklifler (RFQ) &amp; Fiyatlandırma Takip Listesi</span>
          <div style="display:flex;gap:8px;align-items:center;">
            <input id="bfRfqSearch" placeholder="🔍 Müşteri, parça kodu veya malzeme ara..." style="background:#060a14;border:1px solid rgba(255,255,255,0.15);color:#fff;padding:6px 10px;border-radius:8px;font-size:12px;width:240px;" />
            <button class="btn sm" id="bfExportCsvBtn">📊 Excel/CSV İndir</button>
          </div>
        </div>
        <div style="overflow-x:auto;">
          <table>
            <thead>
              <tr>
                <th>Teklif No</th>
                <th>Müşteri &amp; Yetkili</th>
                <th>Parça &amp; Malzeme</th>
                <th>Adet</th>
                <th>Malzeme / İşçilik</th>
                <th>Genel Toplam (KDV Dahil)</th>
                <th>Durum</th>
                <th>İşlem</th>
              </tr>
            </thead>
            <tbody id="bfRfqTbody"></tbody>
          </table>
        </div>
      </div>
    `;
    mainWrap.insertBefore(pane, mainWrap.firstChild);

    let activeRfqId = null;
    let currentComment = DEFAULT_RFQS[0].comment;

    function getFormObj() {
      return {
        id: activeRfqId || ("RFQ-2026-" + Math.floor(100 + Math.random() * 900)),
        date: new Date().toLocaleDateString("tr-TR"),
        customer: document.getElementById("bfCust").value.trim(),
        contact: document.getElementById("bfContact").value.trim(),
        email: document.getElementById("bfEmail").value.trim(),
        phone: document.getElementById("bfPhone").value.trim(),
        partName: document.getElementById("bfPartName").value.trim(),
        partCode: document.getElementById("bfPartCode").value.trim(),
        pricingMode: document.getElementById("bfPricingMode").value,
        directMatUnit: Number(document.getElementById("bfDirectMatUnit").value) || 0,
        netGrams: Number(document.getElementById("bfNetGrams").value) || 0,
        materialKey: document.getElementById("bfMatKey").value,
        kgPrice: Number(document.getElementById("bfKgPrice").value) || 0,
        shape: document.getElementById("bfShape").value,
        dimD: Number(document.getElementById("bfDimD").value) || 0,
        dimX: Number(document.getElementById("bfDimX").value) || 0,
        dimY: Number(document.getElementById("bfDimY").value) || 0,
        dimZ: Number(document.getElementById("bfDimZ").value) || 0,
        scrapPct: Number(document.getElementById("bfScrapPct").value) || 0,
        qty: Math.max(1, Number(document.getElementById("bfQty").value) || 1),
        turnMin: Number(document.getElementById("bfTurnMin").value) || 0,
        turnRate: Number(document.getElementById("bfTurnRate").value) || 0,
        millMin: Number(document.getElementById("bfMillMin").value) || 0,
        millRate: Number(document.getElementById("bfMillRate").value) || 0,
        edmMin: Number(document.getElementById("bfEdmMin").value) || 0,
        edmRate: 1950,
        setupMin: Number(document.getElementById("bfSetupMin").value) || 0,
        toolPct: Number(document.getElementById("bfToolPct").value) || 0,
        outsideUnit: Number(document.getElementById("bfOutside").value) || 0,
        marginPct: Number(document.getElementById("bfMargin").value) || 0,
        leadDays: Number(document.getElementById("bfLeadDays").value) || 7,
        tolerance: document.getElementById("bfTol").value.trim(),
        comment: currentComment,
      };
    }

    function setFormObj(r) {
      activeRfqId = r.id;
      currentComment = r.comment || "";
      document.getElementById("bfCust").value = r.customer || "";
      document.getElementById("bfContact").value = r.contact || "";
      document.getElementById("bfEmail").value = r.email || "";
      document.getElementById("bfPhone").value = r.phone || "";
      document.getElementById("bfPartName").value = r.partName || "";
      document.getElementById("bfPartCode").value = r.partCode || "";
      document.getElementById("bfPricingMode").value = r.pricingMode || "unit";
      document.getElementById("bfDirectMatUnit").value = r.directMatUnit ?? 180;
      document.getElementById("bfNetGrams").value = r.netGrams ?? 450;
      document.getElementById("bfMatKey").value = r.materialKey || "ss304";
      document.getElementById("bfKgPrice").value = r.kgPrice || (ALLOYS[r.materialKey]?.priceKg || 245);
      document.getElementById("bfShape").value = r.shape || "cylindrical";
      document.getElementById("bfDimD").value = r.dimD || 100;
      document.getElementById("bfDimX").value = r.dimX || 100;
      document.getElementById("bfDimY").value = r.dimY || 100;
      document.getElementById("bfDimZ").value = r.dimZ || 30;
      document.getElementById("bfScrapPct").value = r.scrapPct ?? 18;
      document.getElementById("bfQty").value = r.qty || 50;
      document.getElementById("bfTurnMin").value = r.turnMin ?? 0;
      document.getElementById("bfTurnRate").value = r.turnRate || 1650;
      document.getElementById("bfMillMin").value = r.millMin ?? 0;
      document.getElementById("bfMillRate").value = r.millRate || 2100;
      document.getElementById("bfEdmMin").value = r.edmMin ?? 0;
      document.getElementById("bfSetupMin").value = r.setupMin ?? 45;
      document.getElementById("bfToolPct").value = r.toolPct ?? 8;
      document.getElementById("bfOutside").value = r.outsideUnit ?? 0;
      document.getElementById("bfMargin").value = r.marginPct ?? 28;
      document.getElementById("bfLeadDays").value = r.leadDays ?? 7;
      document.getElementById("bfTol").value = r.tolerance || "ISO 2768-m";
      document.getElementById("bfAiCommentText").textContent = currentComment || "Ölçüler yüklendi.";
      toggleShapeFields();
      recalcAll();
    }

    function toggleShapeFields() {
      const mode = document.getElementById("bfPricingMode").value;
      const cyl = document.getElementById("bfShape").value === "cylindrical";
      document.getElementById("bfWrapDirectMat").style.display = mode === "unit" ? "flex" : "none";
      document.getElementById("bfWrapGrams").style.display = (mode === "unit" || mode === "grams") ? "flex" : "none";
      document.getElementById("bfWrapKgPrice").style.display = (mode === "grams" || mode === "billet") ? "flex" : "none";
      document.getElementById("bfWrapShape").style.display = mode === "billet" ? "flex" : "none";
      document.getElementById("bfWrapD").style.display = (mode === "billet" && cyl) ? "flex" : "none";
      document.getElementById("bfWrapX").style.display = (mode === "billet" && !cyl) ? "flex" : "none";
      document.getElementById("bfWrapY").style.display = (mode === "billet" && !cyl) ? "flex" : "none";
      document.getElementById("bfWrapZ").style.display = mode === "billet" ? "flex" : "none";
      document.getElementById("bfWrapScrap").style.display = (mode === "grams" || mode === "billet") ? "flex" : "none";
    }

    function recalcAll() {
      const f = getFormObj();
      const q = calcQuote(f);
      const company = (document.getElementById("orgNameInp")?.value || "BahaTech CNC Talaşlı İmalat").trim();
      const sender = (document.getElementById("orgMgrInp")?.value || "Bahadır Taş").trim();
      const tel = (document.getElementById("orgTelInp")?.value || "+90 543 863 47 70").trim();

      document.getElementById("bfKpiStrip").innerHTML = `
        <div class="bf-kpi"><div class="k">Parça Başı Malzeme</div><div class="v">${fmt(q.matUnit)} ₺/ad</div></div>
        <div class="bf-kpi amber"><div class="k">Parça Başı İşçilik</div><div class="v">${fmt(q.laborUnit)} ₺/ad</div></div>
        <div class="bf-kpi violet"><div class="k">Teklif Birim Fiyatı (KDV Hariç)</div><div class="v">${fmt(q.sellUnit)} ₺/ad</div></div>
        <div class="bf-kpi green"><div class="k">Parti Genel Toplam (${q.qty} ad +KDV)</div><div class="v">${fmt(q.grandTotal)} ₺</div></div>
      `;

      const matDetailHtml = q.mode === "fason"
        ? `<strong>Malzeme:</strong> Müşteri Malzemeli — Sadece Fason CNC İşçilik (0,00 ₺)`
        : q.mode === "unit"
          ? `<strong>Malzeme (Parça Başı):</strong> ${q.alloy.name} • ${q.billetLabel} • <strong>${fmt(q.matUnit)} ₺/ad</strong> (Parti ${q.qty} ad: <strong>${fmt(q.matTotal)} ₺</strong>)`
          : `<strong>Malzeme (Ağırlık Bazlı):</strong> ${q.alloy.name} • ${q.billetLabel} • Brüt <strong>${q.grossKg.toFixed(3)} kg</strong> × ${fmt(q.priceKg)} ₺/kg = <strong>${fmt(q.matUnit)} ₺/ad</strong> (Parti: <strong>${fmt(q.matTotal)} ₺</strong>)`;

      document.getElementById("bfBreakdownBox").innerHTML = `
        <div>${matDetailHtml}</div>
        <div><strong>İşçilik &amp; Tezgâh Kırılımı:</strong> Torna ${fmt(q.turnUnit)} ₺ + Freze ${fmt(q.millUnit)} ₺ + Tel Erozyon ${fmt(q.edmUnit)} ₺ + Setup ${fmt(q.setupUnit)} ₺ + Takım/Uç ${fmt(q.toolUnit)} ₺ + Fason ${fmt(q.outsideUnit)} ₺ = <strong>${fmt(q.laborUnit)} ₺/ad</strong> (Parti: <strong>${fmt(q.laborTotal)} ₺</strong>)</div>
        <div style="margin-top:4px;color:#6ee7b7;"><strong>Teklif Özeti:</strong> Birim Satış: <strong>${fmt(q.sellUnit)} ₺/adet</strong> (KDV Hariç) • Parti Ara Toplam: <strong>${fmt(q.subTotal)} ₺</strong> • %20 KDV: <strong>${fmt(q.vat)} ₺</strong> • <strong>Genel Toplam: ${fmt(q.grandTotal)} ₺</strong></div>
      `;

      const lang = document.getElementById("bfMailLang").value;
      if (lang === "en") {
        document.getElementById("bfMailSubject").value = `CNC Machining Quotation — ${f.partName} (${f.partCode}) • ${f.qty} pcs`;
        document.getElementById("bfMailBody").value =
`Dear ${f.contact || f.customer},

Thank you for your RFQ. Please find below our itemized material + CNC machining quotation based on your technical drawing:

• Part Name / Code: ${f.partName} (${f.partCode})
• Quantity: ${f.qty} pcs
• Material & Billet: ${q.alloy.name} | ${q.billetLabel} | Gross ${q.grossKg.toFixed(2)} kg/pc
• Tolerance / Quality: ${f.tolerance || "ISO 2768-m"}

COST BREAKDOWN (EXCL. VAT):
1) Material Cost (${f.qty} pcs): ${fmt(q.matTotal)} TRY (${fmt(q.matUnit)} TRY/pc)
2) CNC Machining, Setup & Tooling (${f.qty} pcs): ${fmt(q.laborTotal)} TRY (${fmt(q.laborUnit)} TRY/pc)
3) Unit Price (incl. engineering margin): ${fmt(q.sellUnit)} TRY / pc
4) Subtotal: ${fmt(q.subTotal)} TRY
5) VAT (20%): ${fmt(q.vat)} TRY
6) GRAND TOTAL: ${fmt(q.grandTotal)} TRY

• Lead Time: ${f.leadDays} business days after PO confirmation
• Validity: 15 days
• Technical Note: ${currentComment || "Dimensional inspection report included."}

Best regards,
${sender}
${company} | ${tel}`;
      } else {
        document.getElementById("bfMailSubject").value = `Fiyat Teklifimiz — ${f.partName} (${f.partCode}) • ${f.qty} Adet`;
        document.getElementById("bfMailBody").value =
`Sayın ${f.contact || f.customer},

Tarafımıza ilettiğiniz teknik resim doğrultusunda hazırlanan malzeme + CNC talaşlı imalat işçilik fiyat teklifimizi bilgilerinize sunarız:

• Parça Adı / Kodu: ${f.partName} (${f.partCode})
• Sipariş Adedi: ${f.qty} Adet
• Malzeme & Değerleme: ${q.alloy.name} | ${q.billetLabel}
• Tolerans & Kalite: ${f.tolerance || "ISO 2768-m"}

MALZEME + İŞÇİLİK MALİYET KIRILIMI (KDV HARİÇ):
1) Ham Malzeme Bedeli (${f.qty} adet): ${fmt(q.matTotal)} ₺ (Birim: ${fmt(q.matUnit)} ₺)
2) CNC Talaşlı İmalat, Setup & Fason İşçilik (${f.qty} adet): ${fmt(q.laborTotal)} ₺ (Birim: ${fmt(q.laborUnit)} ₺)
3) Teklif Birim Fiyatı (KDV Hariç): ${fmt(q.sellUnit)} ₺ / adet
4) Ara Toplam (KDV Hariç): ${fmt(q.subTotal)} ₺
5) KDV (%20): ${fmt(q.vat)} ₺
6) GENEL TOPLAM (KDV DAHİL): ${fmt(q.grandTotal)} ₺

• Teslim Süresi (Termin): Sipariş onayından itibaren ${f.leadDays} iş günü
• Teklif Geçerlilik Süresi: 15 gün
• Mühendislik Notu: ${currentComment || "Parçalar ölçüm raporu ile teslim edilir."}

Onayınız halinde üretim planına hemen alabiliriz.

Saygılarımızla,
${sender}
${company}
Tel: ${tel}`;
      }
    }

    function renderRfqTable() {
      const list = loadRfqs();
      document.getElementById("bfRfqCountBadge").textContent = String(list.length);
      const q = (document.getElementById("bfRfqSearch").value || "").toLowerCase();
      const filtered = list.filter(r =>
        `${r.id} ${r.customer} ${r.contact} ${r.partName} ${r.partCode} ${r.materialKey}`.toLowerCase().includes(q)
      );
      const tbody = document.getElementById("bfRfqTbody");
      tbody.innerHTML = filtered.map(r => {
        const c = calcQuote(r);
        const cls = r.status === "Onaylandı" ? "won" : r.status === "Mail Gönderildi" ? "sent" : r.status === "Fiyatlandırıldı" ? "quoted" : "new";
        return `
          <tr>
            <td><strong>${r.id}</strong><br><small style="color:#94a3b8">${r.date || ""}</small></td>
            <td><strong>${r.customer}</strong><br><small style="color:#94a3b8">${r.contact || ""} • ${r.email || ""}</small></td>
            <td><strong>${r.partName}</strong> (${r.partCode})<br><small style="color:#94a3b8">${c.alloy.name}</small></td>
            <td><strong>${r.qty}</strong> ad</td>
            <td><small>Malz: ${fmt(c.matTotal)} ₺<br>İşç: ${fmt(c.laborTotal)} ₺</small></td>
            <td><strong style="color:#34d399">${fmt(c.grandTotal)} ₺</strong><br><small style="color:#94a3b8">${fmt(c.sellUnit)} ₺/ad</small></td>
            <td><span class="bf-badge ${cls}">${r.status}</span></td>
            <td>
              <button class="btn sm" data-bf-load="${r.id}">✏️ Yükle</button>
              <button class="btn sm" data-bf-del="${r.id}" style="color:#fca5a5">🗑</button>
            </td>
          </tr>
        `;
      }).join("");

      tbody.querySelectorAll("[data-bf-load]").forEach(btn => {
        btn.onclick = () => {
          const item = loadRfqs().find(x => x.id === btn.dataset.bfLoad);
          if (item) setFormObj(item);
        };
      });
      tbody.querySelectorAll("[data-bf-del]").forEach(btn => {
        btn.onclick = () => {
          const next = loadRfqs().filter(x => x.id !== btn.dataset.bfDel);
          saveRfqs(next);
          renderRfqTable();
        };
      });
    }

    function refreshMlBadge() {
      const mem = loadMlMemory();
      const n = mem.length;
      const conf = n === 0 ? 0 : Math.min(97, Math.round((Math.min(n, 12) / 12) * 65 + 28));
      const stage = n === 0
        ? "🎓 İlk Eğitim Modu (0 Parça — İlk fiyatlarınızı girin)"
        : n < 3
          ? `🌱 Öğrenmeye Başladı (${n} Parça Hafızada)`
          : n < 10
            ? `📈 Yarı-Otonom Tahmin (${n} Parça Hafızada)`
            : `⚡ Otonom Fiyat Motoru (${n} Parça — Minimum İş Yükü)`;
      const elStage = document.getElementById("bfMlStageText");
      const elFill = document.getElementById("bfMlConfFill");
      const elPct = document.getElementById("bfMlConfPct");
      if (elStage) elStage.textContent = `🧠 Gömülü Yerel Öğrenen Fiyat AI • ${stage}`;
      if (elFill) elFill.style.width = `${Math.max(6, conf)}%`;
      if (elPct) elPct.textContent = `%${conf}`;
    }

    async function applySelfLearningPrediction(sampleObj, triggerMsgPrefix) {
      let mem = loadMlMemory();
      // Try Rust/native server persistent ML first, seamlessly fall back to localStorage mirror
      let mlRes = null;
      try {
        const r = await fetch("/api/ml/predict", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(sampleObj),
        });
        if (r.ok) {
          const j = await r.json();
          if (j && j.ok && j.trained > 0 && j.prediction) mlRes = j;
        }
      } catch (_) {}
      if (!mlRes) mlRes = localMlPredict(sampleObj, mem);

      refreshMlBadge();
      if (mlRes && mlRes.prediction && mlRes.trained > 0) {
        const p = mlRes.prediction;
        if (p.directMatUnit != null && p.directMatUnit > 0) document.getElementById("bfDirectMatUnit").value = p.directMatUnit;
        if (p.turnMin != null) document.getElementById("bfTurnMin").value = p.turnMin;
        if (p.millMin != null) document.getElementById("bfMillMin").value = p.millMin;
        if (p.edmMin != null) document.getElementById("bfEdmMin").value = p.edmMin;
        if (p.setupMin != null) document.getElementById("bfSetupMin").value = p.setupMin;
        if (p.kgPrice) document.getElementById("bfKgPrice").value = p.kgPrice;
        if (p.turnRate) document.getElementById("bfTurnRate").value = p.turnRate;
        if (p.millRate) document.getElementById("bfMillRate").value = p.millRate;
        if (p.marginPct) document.getElementById("bfMargin").value = p.marginPct;
        if (p.scrapPct) document.getElementById("bfScrapPct").value = p.scrapPct;
        currentComment = `${triggerMsgPrefix ? triggerMsgPrefix + " | " : ""}${mlRes.explanation}`;
        document.getElementById("bfAiCommentText").textContent = currentComment;
        recalcAll();
        return true;
      }
      return false;
    }

    // Upload & AI Technical Drawing Analysis
    async function analyzeTechnicalDrawing(file) {
      const box = document.getElementById("bfAiCommentText");
      box.innerHTML = `⏳ <strong>${file.name}</strong> okunuyor... Gömülü yerel yapay zeka ölçüleri, malzemeyi ve geçmiş fiyat hafızasını analiz ediyor...`;
      const reader = new FileReader();
      reader.onload = async () => {
        const b64 = reader.result;
        if (file.type.startsWith("image/")) {
          const img = document.getElementById("bfPreviewImg");
          img.src = b64;
          img.style.display = "inline-block";
        }
        let serverComment = "";
        try {
          const resp = await fetch("/api/rfq-analyze", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ filename: file.name, file_b64: b64 }),
          });
          const data = await resp.json();
          if (!resp.ok || !data.ok) throw new Error(data.error || "Sunucu analiz hatası");

          if (data.preview_png_b64) {
            const img = document.getElementById("bfPreviewImg");
            img.src = data.preview_png_b64;
            img.style.display = "inline-block";
          }
          if (data.customer) document.getElementById("bfCust").value = data.customer;
          if (data.part_name) document.getElementById("bfPartName").value = data.part_name;
          if (data.part_code) document.getElementById("bfPartCode").value = data.part_code;
          if (data.qty && data.qty > 0) document.getElementById("bfQty").value = data.qty;
          if (data.material_key && ALLOYS[data.material_key]) {
            document.getElementById("bfMatKey").value = data.material_key;
            document.getElementById("bfKgPrice").value = data.kg_price_tl || ALLOYS[data.material_key].priceKg;
          }
          if (data.shape === "cylindrical" || data.shape === "prismatic") {
            document.getElementById("bfShape").value = data.shape;
          }
          if (data.dim_d_mm) document.getElementById("bfDimD").value = data.dim_d_mm;
          if (data.dim_x_mm) document.getElementById("bfDimX").value = data.dim_x_mm;
          if (data.dim_y_mm) document.getElementById("bfDimY").value = data.dim_y_mm;
          if (data.dim_z_mm) document.getElementById("bfDimZ").value = data.dim_z_mm;
          if (data.net_grams != null) document.getElementById("bfNetGrams").value = data.net_grams;
          if (data.direct_mat_unit_tl != null) document.getElementById("bfDirectMatUnit").value = data.direct_mat_unit_tl;
          if (data.vision_verified) {
            // Vision read the exact title-block billet dimensions & weight -> show Kütük mm->kg or accurate unit breakdown!
            document.getElementById("bfPricingMode").value = "billet";
            document.getElementById("bfScrapPct").value = 8;
          }
          toggleShapeFields();
          if (data.turn_min != null) document.getElementById("bfTurnMin").value = data.turn_min;
          if (data.mill_min != null) document.getElementById("bfMillMin").value = data.mill_min;
          if (data.edm_min != null) document.getElementById("bfEdmMin").value = data.edm_min;
          if (data.setup_min != null) document.getElementById("bfSetupMin").value = data.setup_min;
          if (data.tolerance) document.getElementById("bfTol").value = data.tolerance;
          const massSummary = data.billet_kg
            ? ` [Ham Kütük: ${Number(data.billet_kg).toFixed(2)} kg • Net İşlenmiş: ~${Number(data.net_kg || 0).toFixed(2)} kg]`
            : "";
          serverComment = `[${data.difficulty || "Hassas"}]${massSummary} ${data.engineering_comment || ""}`;
          currentComment = serverComment;
          box.textContent = currentComment;
          recalcAll();
        } catch (err) {
          serverComment = `Yerel mod (${file.name})`;
          currentComment = `Dosya (${file.name}) yüklendi — ilk eğitimde gerçek süre ve fiyatınızı girip 'Kaydet & Eğit'e basın; motor bundan sonraki çizimleri kendi öğrenecek.`;
          box.textContent = currentComment;
          document.getElementById("bfPartCode").value = file.name.replace(/\.[^.]+$/, "").toUpperCase().slice(0, 24);
          recalcAll();
        }
        // Apply self-learning shop memory ON TOP of detected geometry!
        const fNow = getFormObj();
        await applySelfLearningPrediction(fNow, serverComment);
      };
      reader.readAsDataURL(file);
    }

    // Bind events (prevent window default drag navigation so Finder drag-drop works flawlessly in WKWebView!)
    window.addEventListener("dragover", (e) => { e.preventDefault(); }, false);
    window.addEventListener("drop", (e) => { e.preventDefault(); }, false);
    const dz = document.getElementById("bfDropzone");
    const fi = document.getElementById("bfFileInp");
    const pickBtn = document.getElementById("bfPickFileBtn");
    dz.onclick = (e) => { if (e.target !== fi) { fi.value = ""; fi.click(); } };
    if (pickBtn) pickBtn.onclick = (e) => { e.stopPropagation(); fi.value = ""; fi.click(); };
    dz.ondragenter = (e) => { e.preventDefault(); e.stopPropagation(); dz.classList.add("drag"); };
    dz.ondragover = (e) => { e.preventDefault(); e.stopPropagation(); dz.classList.add("drag"); };
    dz.ondragleave = (e) => { e.preventDefault(); e.stopPropagation(); dz.classList.remove("drag"); };
    dz.ondrop = (e) => {
      e.preventDefault(); e.stopPropagation(); dz.classList.remove("drag");
      const file = e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0];
      if (file) analyzeTechnicalDrawing(file);
    };
    fi.onchange = () => { if (fi.files && fi.files[0]) analyzeTechnicalDrawing(fi.files[0]); };

    document.getElementById("bfPricingMode").onchange = () => { toggleShapeFields(); recalcAll(); };
    document.getElementById("bfMatKey").onchange = () => {
      const k = document.getElementById("bfMatKey").value;
      if (ALLOYS[k]) document.getElementById("bfKgPrice").value = ALLOYS[k].priceKg;
      recalcAll();
    };
    document.getElementById("bfShape").onchange = () => { toggleShapeFields(); recalcAll(); };
    document.getElementById("bfMailLang").onchange = recalcAll;
    pane.querySelectorAll("input, select").forEach(el => {
      if (el.id !== "bfRfqSearch" && el.id !== "bfMailSubject") {
        el.addEventListener("input", recalcAll);
      }
    });
    document.getElementById("bfRfqSearch").oninput = renderRfqTable;

    document.getElementById("bfRunMlNowBtn").onclick = async () => {
      const f = getFormObj();
      const applied = await applySelfLearningPrediction(f, "🧠 Manuel Hafıza Tahmini");
      if (!applied) {
        alert("🎓 Henüz yapay zeka hafızasında eğitilmiş parça yok! Önce birkaç parçanın gerçek fiyat/sürelerini girip 'Kaydet & Yerel Yapay Zekayı Eğit' butonuna basın.");
      }
    };

    document.getElementById("bfSaveRfqBtn").onclick = async () => {
      const obj = getFormObj();
      const q = calcQuote(obj);
      obj.status = "Fiyatlandırıldı";
      const list = loadRfqs();
      const idx = list.findIndex(x => x.id === obj.id);
      if (idx >= 0) list[idx] = obj; else list.unshift(obj);
      saveRfqs(list);
      activeRfqId = obj.id;
      renderRfqTable();

      // Train embedded Self-Learning Local ML Memory (both localStorage + Rust persistent disk file!)
      const volCm3 = obj.shape === "cylindrical"
        ? Math.PI * Math.pow((obj.dimD || 100) / 20, 2) * ((obj.dimZ || 25) / 10)
        : ((obj.dimX || 100) * (obj.dimY || 80) * (obj.dimZ || 25)) / 1000;
      const sample = {
        id: obj.id,
        partName: obj.partName,
        partCode: obj.partCode,
        materialKey: obj.materialKey,
        shape: obj.shape,
        dimD: obj.dimD, dimX: obj.dimX, dimY: obj.dimY, dimZ: obj.dimZ,
        volCm3: Number(volCm3.toFixed(2)),
        qty: obj.qty,
        kgPrice: obj.kgPrice,
        directMatUnit: Number(q.matUnit.toFixed(2)),
        netGrams: obj.netGrams,
        scrapPct: obj.scrapPct,
        turnMin: obj.turnMin, turnRate: obj.turnRate,
        millMin: obj.millMin, millRate: obj.millRate,
        edmMin: obj.edmMin, setupMin: obj.setupMin,
        marginPct: obj.marginPct,
        sellUnit: Number(q.sellUnit.toFixed(2)),
      };
      const mem = loadMlMemory().filter(m => m.id !== sample.id);
      mem.push(sample);
      saveMlMemory(mem);
      try {
        await fetch("/api/ml/train", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(sample),
        });
      } catch (_) {}
      refreshMlBadge();
      alert(`✅ ${obj.id} kaydedildi ve Yerel Yapay Zeka bu işten eğitildi! (Hafızadaki toplam parça: ${mem.length})`);
    };

    document.getElementById("bfNewRfqBtn").onclick = () => {
      activeRfqId = null;
      currentComment = "Yeni RFQ kaydı açıldı. Teknik resim yükleyin veya ölçü girin.";
      document.getElementById("bfCust").value = "";
      document.getElementById("bfContact").value = "";
      document.getElementById("bfEmail").value = "";
      document.getElementById("bfPhone").value = "905";
      document.getElementById("bfPartName").value = "";
      document.getElementById("bfPartCode").value = "";
      document.getElementById("bfAiCommentText").textContent = currentComment;
      recalcAll();
    };

    document.getElementById("bfOpenMailtoBtn").onclick = () => {
      const to = document.getElementById("bfEmail").value.trim();
      const subj = encodeURIComponent(document.getElementById("bfMailSubject").value);
      const body = encodeURIComponent(document.getElementById("bfMailBody").value);
      const list = loadRfqs();
      const idx = list.findIndex(x => x.id === activeRfqId);
      if (idx >= 0) { list[idx].status = "Mail Gönderildi"; saveRfqs(list); renderRfqTable(); }
      window.location.href = `mailto:${to}?subject=${subj}&body=${body}`;
    };

    document.getElementById("bfCopyMailBtn").onclick = async () => {
      const full = `Konu: ${document.getElementById("bfMailSubject").value}\n\n${document.getElementById("bfMailBody").value}`;
      try { await navigator.clipboard.writeText(full); } catch (_) {}
      alert("✅ Malzeme + İşçilik Teklif Maili panoya kopyalandı!");
    };

    document.getElementById("bfSendWaBtn").onclick = () => {
      const ph = document.getElementById("bfPhone").value.replace(/\D/g, "");
      const txt = encodeURIComponent(document.getElementById("bfMailBody").value);
      window.open(`https://wa.me/${ph}?text=${txt}`, "_blank");
    };

    document.getElementById("bfPrintQuoteBtn").onclick = () => window.print();

    document.getElementById("bfExportCsvBtn").onclick = () => {
      const rows = [["TeklifNo", "Tarih", "Musteri", "Yetkili", "Eposta", "Parca", "Kod", "Adet", "MalzemeTL", "IscilikTL", "GenelToplamTL", "Durum"]];
      loadRfqs().forEach(r => {
        const c = calcQuote(r);
        rows.push([r.id, r.date, r.customer, r.contact, r.email, r.partName, r.partCode, r.qty, c.matTotal.toFixed(2), c.laborTotal.toFixed(2), c.grandTotal.toFixed(2), r.status]);
      });
      const csv = "\uFEFF" + rows.map(r => r.map(x => `"${String(x ?? "").replace(/"/g, '""')}"`).join(";")).join("\n");
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([csv], { type: "text/csv;charset=utf-8" }));
      a.download = "BahaForge_Gelen_Teklifler_RFQ.csv";
      a.click();
    };

    // Hook into host tab switching cleanly
    navBtn.onclick = () => {
      document.querySelectorAll(".nav-btn").forEach(b => b.classList.remove("active"));
      document.querySelectorAll(".tab-pane").forEach(p => p.classList.remove("active"));
      navBtn.classList.add("active");
      pane.classList.add("active");
    };
    document.querySelectorAll(".nav-btn:not(#nav-rfq)").forEach(b => {
      b.addEventListener("click", () => {
        navBtn.classList.remove("active");
        pane.classList.remove("active");
      });
    });

    // Initialize with first RFQ & activate RFQ studio immediately!
    const initial = loadRfqs();
    if (initial[0]) setFormObj(initial[0]);
    renderRfqTable();
    refreshMlBadge();
    navBtn.click();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", mountRfqStudio);
  } else {
    mountRfqStudio();
  }
})();
