# BahaForge CAD CAM AI Studio v7.0 (100% Pure Rust — Zero Python!)

**BahaForge CAD CAM AI Studio v7.0**; CNC torna, 3/5-eksen CNC freze ve tel erozyon talaşlı imalat atölyeleri için **%100 Saf Rust (`std`)** ile sıfır harici bağımlılıkla derlenen masaüstü işletim sistemidir.

## Temel Özellikler
- **📥 Gelen Teklifler (RFQ) & Teklif Takip Kutusu:** Müşteri, parça kodu, malzeme, adet, termin, durum ve CSV/Excel dışa aktarım.
- **🧠 Gömülü Yerel Öğrenen Fiyat Yapay Zekası (`BahaForge-Rust-LocalML`):** İnternetsiz, sıfır bulut! Program indirildiğinde içinde kurulu gelir. İlk birkaç PDF'te kullanıcının girdiği gerçek tezgâh süresi, alış malzeme fiyatı ve kâr marjını yerel `bahaforge_ml_memory.jsonl` hafızasına kaydeder; sonraki PDF teknik resimlerinde hacim/malzeme/adet benzerliğine göre (ağırlıklı k-NN + hacim ölçekleme) kendi fiyat ve süre tahminlerini otomatik doldurur.
- **⚖️ Hassas Malzeme & İşçilik Hesabı:** 9 endüstriyel alaşım (7075, 6061, St37, 4140, AISI 304/316L, Ti-6Al-4V, Pirinç, POM), silindirik/prizmatik kütük, fire %, CNC torna/freze/tel erozyon, setup, takım aşınma, dış fason, kâr marjı & KDV.
- **✉️ Müşteriye Teklif Maili & WhatsApp Stüdyosu:** Türkçe kurumsal ve İngilizce ihracat (EN) kırılımlı teklif mailini tek tıkla üretir (`mailto:`, kopyala, WhatsApp, A4 PDF).
- **📐 2D PDF → 3D STEP & ⚙️ BahaCAM VoluMill CNC:** Tek tek tıkla 3D inceleme ve yüksek hızlı kaba boşaltma G-Code üretimi.
- **🔐 %100 Yerel • 0 ₺ Aylık Kira • Sıfır Python / Sıfır Node.js:** Tek çalıştırılabilir `BahaForge_CAD_CAM_Studio.exe` (~1.2 MB).

## Derleme (Pure Rust)
```bash
# Windows 64-bit .exe
cargo build --release --target x86_64-pc-windows-gnullvm

# macOS Apple Silicon
cargo build --release
```

© 2026 **BahaTech Software LLC** — [bahatechsoftware.com](https://bahatechsoftware.com/products/cadcam-os.html)
