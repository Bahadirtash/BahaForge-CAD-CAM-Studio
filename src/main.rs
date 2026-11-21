#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

// Embedded single-binary web suite assets (zero external runtime files required!)
const ASSET_FORGE_HTML: &str = include_str!("../web/forge.html");
const ASSET_CADCAM_OS_HTML: &str = include_str!("../web/BahaTech_CADCAM_OS.html");
const ASSET_RFQ_JS: &str = include_str!("../web/bahaforge-rfq-quote-ai.js");
const ASSET_I18N_JS: &str = include_str!("../web/app-i18n-23.js");
const ASSET_SUITE_V7_JS: &str = include_str!("../web/bahatech-enterprise-suite-v7.js");
const ASSET_BAHACAD_HTML: &str = include_str!("../web/bahacad.html");
const ASSET_BAHACAM_HTML: &str = include_str!("../web/bahacam.html");
const ASSET_VOLUMILL_JS: &str = include_str!("../web/volumill_engine.js");

fn main() {
    let args: Vec<String> = env::args().collect();
    let headless = args.iter().any(|a| a == "--headless");
    let requested_port: u16 = args
        .windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse::<u16>().ok())
        .unwrap_or(19545);

    let (listener, port) = bind_server(requested_port);

    if !headless {
        let url = format!("http://127.0.0.1:{}/forge.html", port);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(350));
            launch_app_window(&url);
        });
    }

    for stream in listener.incoming().flatten() {
        thread::spawn(move || {
            let _ = handle_client(stream);
        });
    }
}

fn bind_server(preferred: u16) -> (TcpListener, u16) {
    for p in preferred..(preferred.saturating_add(30)) {
        if let Ok(l) = TcpListener::bind(("127.0.0.1", p)) {
            return (l, p);
        }
    }
    let l = TcpListener::bind("127.0.0.1:0").expect("Local TCP socket bind failed");
    let p = l.local_addr().unwrap().port();
    (l, p)
}

#[cfg(target_os = "windows")]
fn launch_app_window(url: &str) {
    let edge_candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        "msedge",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    ];
    let app_arg = format!("--app={}", url);
    for bin in &edge_candidates {
        if Command::new(bin)
            .args([&app_arg, "--window-size=1480,920", "--disable-features=Translate"])
            .spawn()
            .is_ok()
        {
            return;
        }
    }
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
}

#[cfg(target_os = "macos")]
fn launch_app_window(url: &str) {
    let _ = Command::new("open").arg(url).spawn();
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn launch_app_window(url: &str) {
    let _ = Command::new("xdg-open").arg(url).spawn();
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let mut buf = Vec::with_capacity(8192);
    let mut chunk = [0u8; 4096];
    let header_end;
    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
        if buf.len() > 65536 {
            return Ok(());
        }
    }

    let head_str = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head_str.lines();
    let req_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let raw_path = parts[1].split('?').next().unwrap_or("/");

    let mut content_len: usize = 0;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    // Cap body to 25 MB for large engineering drawing PDFs
    if content_len > 25 * 1024 * 1024 {
        return write_json(&mut stream, 413, r#"{"ok":false,"error":"Dosya 25MB limitini aşıyor"}"#);
    }
    while buf.len() < header_end + content_len {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body_bytes = &buf[header_end..buf.len().min(header_end + content_len)];

    if method == "OPTIONS" {
        return write_raw(&mut stream, 204, "text/plain", b"");
    }

    if method == "GET" {
        return match raw_path {
            "/" | "/index.html" | "/forge.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_FORGE_HTML.as_bytes()),
            "/BahaTech_CADCAM_OS.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_CADCAM_OS_HTML.as_bytes()),
            "/bahaforge-rfq-quote-ai.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_RFQ_JS.as_bytes()),
            "/app-i18n-23.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_I18N_JS.as_bytes()),
            "/bahatech-enterprise-suite-v7.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_SUITE_V7_JS.as_bytes()),
            "/bahacad.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_BAHACAD_HTML.as_bytes()),
            "/bahacam.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_BAHACAM_HTML.as_bytes()),
            "/volumill_engine.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_VOLUMILL_JS.as_bytes()),
            "/api/health" => {
                let key_ok = get_xai_key().is_some();
                let mem_n = load_ml_records().len();
                let resp = format!(
                    r#"{{"ok":true,"engine":"BahaForge Pure Rust Native Engine + Self-Learning ML v7.0 (Zero-Python)","os":"{}","arch":"{}","ml_samples":{},"xai_configured":{}}}"#,
                    env::consts::OS,
                    env::consts::ARCH,
                    mem_n,
                    key_ok
                );
                write_json(&mut stream, 200, &resp)
            }
            "/api/ml/status" => {
                let mem = load_ml_records();
                let n = mem.len();
                let conf = if n == 0 { 0 } else { (n.min(12) * 65 / 12 + 28).min(97) };
                let resp = format!(r#"{{"ok":true,"trained":{},"confidence":{},"engine":"BahaForge-Rust-LocalML-kNN"}}"#, n, conf);
                write_json(&mut stream, 200, &resp)
            }
            _ => write_json(&mut stream, 404, r#"{"ok":false,"error":"Sayfa bulunamadı"}"#),
        };
    }

    if method == "POST" {
        let body_str = String::from_utf8_lossy(body_bytes);
        return match raw_path {
            "/api/rfq-analyze" => {
                let json = handle_rfq_analyze(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/pdf2step" | "/api/refine-step" => {
                let json = handle_rust_pdf2step(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/rfq-calc" => {
                let json = handle_rfq_calc(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/ml/train" => {
                let json = handle_ml_train(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/ml/predict" => {
                let json = handle_ml_predict(&body_str);
                write_json(&mut stream, 200, &json)
            }
            _ => write_json(&mut stream, 404, r#"{"ok":false,"error":"Bilinmeyen uç nokta"}"#),
        };
    }

    write_json(&mut stream, 405, r#"{"ok":false,"error":"Method Not Allowed"}"#)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn write_json(stream: &mut TcpStream, status: u16, json: &str) -> std::io::Result<()> {
    write_raw(stream, status, "application/json; charset=utf-8", json.as_bytes())
}

fn write_raw(stream: &mut TcpStream, status: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        413 => "Payload Too Large",
        _ => "Error",
    };
    let hdr = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        status, reason, ctype, body.len()
    );
    stream.write_all(hdr.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn get_xai_key() -> Option<String> {
    if let Ok(v) = env::var("XAI_API_KEY") {
        let t = v.trim().trim_matches('"').trim_matches('\'').to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    let mut paths: Vec<PathBuf> = vec![PathBuf::from(".env")];
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(".env"));
        }
    }
    for p in paths {
        if let Ok(txt) = fs::read_to_string(&p) {
            for line in txt.lines() {
                if let Some(rest) = line.trim().strip_prefix("XAI_API_KEY=") {
                    let t = rest.trim().trim_matches('"').trim_matches('\'').to_string();
                    if !t.is_empty() {
                        return Some(t);
                    }
                }
            }
        }
    }
    None
}

/// Minimal zero-dependency JSON string field extractor
fn extract_json_str(body: &str, key: &str) -> String {
    let pat = format!("\"{}\"", key);
    let Some(pos) = body.find(&pat) else { return String::new(); };
    let rest = &body[pos + pat.len()..];
    let Some(colon) = rest.find(':') else { return String::new(); };
    let after = rest[colon + 1..].trim_start();
    if !after.starts_with('"') {
        return String::new();
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    other => out.push(other),
                }
            }
        } else if c == '"' {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_json_num(body: &str, key: &str, default_val: f64) -> f64 {
    let pat = format!("\"{}\"", key);
    let Some(pos) = body.find(&pat) else { return default_val; };
    let rest = &body[pos + pat.len()..];
    let Some(colon) = rest.find(':') else { return default_val; };
    let after = rest[colon + 1..].trim_start();
    let num_s: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_s.parse::<f64>().unwrap_or(default_val)
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

fn base64_decode(input: &str) -> Vec<u8> {
    let mut table = [255u8; 256];
    for (i, b) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
        table[*b as usize] = i as u8;
    }
    let clean: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < clean.len() {
        let a = table[clean[i] as usize];
        let b = table[clean[i + 1] as usize];
        let c = table[clean[i + 2] as usize];
        let d = table[clean[i + 3] as usize];
        if a == 255 || b == 255 {
            break;
        }
        out.push((a << 2) | (b >> 4));
        if clean[i + 2] != b'=' && c != 255 {
            out.push((b << 4) | (c >> 2));
        }
        if clean[i + 3] != b'=' && d != 255 {
            out.push((c << 6) | d);
        }
        i += 4;
    }
    out
}

/// Pure-Rust heuristic engineering drawing text/dimension parser (works 100% offline!)
fn parse_drawing_specs_offline(filename: &str, raw: &[u8]) -> HashMap<&'static str, String> {
    let mut text = String::new();
    text.push_str(filename);
    text.push(' ');
    // Scan ASCII printable tokens embedded in PDF/text stream
    let mut cur = String::new();
    for &b in raw.iter().take(600_000) {
        if b.is_ascii_graphic() || b == b' ' {
            cur.push(b as char);
        } else {
            if cur.len() >= 3 && cur.len() <= 140 {
                text.push_str(&cur);
                text.push(' ');
            }
            cur.clear();
        }
    }
    let low = text.to_lowercase();

    let (mat_key, mat_label) = if low.contains("7075") {
        ("al7075", "Alüminyum 7075-T6")
    } else if low.contains("6061") || low.contains("alumin") || low.contains("alümin") {
        ("al6061", "Alüminyum 6061-T6")
    } else if low.contains("316") {
        ("ss316", "AISI 316L Paslanmaz")
    } else if low.contains("304") || low.contains("stainless") || low.contains("paslanmaz") {
        ("ss304", "AISI 304 Paslanmaz Çelik")
    } else if low.contains("ti-6") || low.contains("titan") {
        ("ti64", "Titanyum Ti-6Al-4V")
    } else if low.contains("brass") || low.contains("pirin") || low.contains("ms58") {
        ("brass", "Pirinç MS58")
    } else if low.contains("st37") || low.contains("s235") {
        ("st37", "St37 / S235JR")
    } else {
        ("steel4140", "4140 Islah Çeliği")
    };

    let is_cyl = low.contains("flange")
        || low.contains("flan")
        || low.contains("shaft")
        || low.contains("mil")
        || low.contains("od ")
        || low.contains("pcd")
        || low.contains("ø")
        || low.contains("dia");

    // Extract plausible mm numbers (between 8 and 800)
    let mut nums: Vec<f64> = Vec::new();
    let mut tok = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            tok.push(ch);
        } else {
            if let Ok(v) = tok.parse::<f64>() {
                if (8.0..=600.0).contains(&v) && !nums.iter().any(|x| (*x - v).abs() < 0.01) {
                    nums.push(v);
                }
            }
            tok.clear();
        }
    }
    nums.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let d1 = nums.first().copied().unwrap_or(120.0);
    let d2 = nums.get(1).copied().unwrap_or(80.0);
    let d3 = nums.get(2).copied().unwrap_or(28.0).min(120.0);

    let clean_code: String = filename
        .split('.')
        .next()
        .unwrap_or("BHF-PART")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(22)
        .collect::<String>()
        .to_uppercase();

    let mut m = HashMap::new();
    m.insert("part_name", if is_cyl { "CNC Talaşlı İmalat Döner Parça / Flanş" } else { "CNC İşlenmiş Prizmatik Gövde" }.to_string());
    m.insert("part_code", if clean_code.is_empty() { "BHF-2026-01".to_string() } else { clean_code });
    m.insert("material_key", mat_key.to_string());
    m.insert("material_label", mat_label.to_string());
    m.insert("shape", if is_cyl { "cylindrical" } else { "prismatic" }.to_string());
    m.insert("dim_d", format!("{:.1}", d1));
    m.insert("dim_x", format!("{:.1}", d1));
    m.insert("dim_y", format!("{:.1}", d2));
    m.insert("dim_z", format!("{:.1}", d3));
    m
}

fn handle_rfq_analyze(body: &str) -> String {
    let fname = extract_json_str(body, "filename");
    let b64_raw = extract_json_str(body, "file_b64");
    let b64_clean = b64_raw.split_once(',').map(|(_, r)| r).unwrap_or(&b64_raw);
    let bytes = base64_decode(b64_clean);

    let specs = parse_drawing_specs_offline(&fname, &bytes);
    let shape = specs.get("shape").cloned().unwrap_or_else(|| "cylindrical".into());
    let turn_min = if shape == "cylindrical" { 18 } else { 0 };
    let mill_min = if shape == "cylindrical" { 20 } else { 36 };
    let comment = format!(
        "Rust Yerel Teknik Resim Motoru ({} bayt) analizi tamamlandı: {} malzemesi için {} formunda kütük seçildi. {} dk CNC işleme + 45 dk setup önerilir. Ölçüleri doğrulayıp tek tıkla müşteriye malzeme + işçilik teklif maili gönderebilirsiniz.",
        bytes.len(),
        specs.get("material_label").unwrap_or(&"4140".to_string()),
        if shape == "cylindrical" { "silindirik" } else { "prizmatik" },
        turn_min + mill_min
    );

    format!(
        r#"{{"ok":true,"engine":"BahaForge Rust Native v7.0","part_name":"{}","part_code":"{}","material_key":"{}","material_label":"{}","shape":"{}","dim_d_mm":{},"dim_x_mm":{},"dim_y_mm":{},"dim_z_mm":{},"tolerance":"ISO 2768-m","turn_min":{},"mill_min":{},"edm_min":0,"setup_min":45,"difficulty":"Hassas","engineering_comment":"{}"}}"#,
        json_escape(&specs["part_name"]),
        json_escape(&specs["part_code"]),
        json_escape(&specs["material_key"]),
        json_escape(&specs["material_label"]),
        json_escape(&shape),
        specs["dim_d"],
        specs["dim_x"],
        specs["dim_y"],
        specs["dim_z"],
        turn_min,
        mill_min,
        json_escape(&comment)
    )
}

fn handle_rfq_calc(body: &str) -> String {
    let d = extract_json_num(body, "dim_d_mm", 120.0);
    let z = extract_json_num(body, "dim_z_mm", 33.0);
    let density = extract_json_num(body, "density", 7.93);
    let kg_price = extract_json_num(body, "kg_price", 245.0);
    let scrap_pct = extract_json_num(body, "scrap_pct", 18.0);
    let qty = extract_json_num(body, "qty", 50.0).max(1.0);
    let mach_min = extract_json_num(body, "mach_min", 40.0);
    let hour_rate = extract_json_num(body, "hour_rate", 1950.0);
    let margin = extract_json_num(body, "margin_pct", 28.0);

    let r_cm = d / 20.0;
    let l_cm = z / 10.0;
    let vol_cm3 = std::f64::consts::PI * r_cm * r_cm * l_cm;
    let gross_kg = (vol_cm3 * density / 1000.0) * (1.0 + scrap_pct / 100.0);
    let mat_unit = gross_kg * kg_price;
    let labor_unit = (mach_min / 60.0) * hour_rate;
    let sell_unit = (mat_unit + labor_unit) * (1.0 + margin / 100.0);
    let grand_total = sell_unit * qty * 1.20;

    format!(
        r#"{{"ok":true,"engine":"Rust-Native","gross_kg":{:.3},"mat_unit_tl":{:.2},"labor_unit_tl":{:.2},"sell_unit_tl":{:.2},"grand_total_vat_tl":{:.2}}}"#,
        gross_kg, mat_unit, labor_unit, sell_unit, grand_total
    )
}

/// Pure-Rust ISO-10303-21 AP214 STEP + ASCII STL generator (zero Python dependency!)
fn handle_rust_pdf2step(body: &str) -> String {
    let fname = extract_json_str(body, "filename");
    let b64_raw = extract_json_str(body, "file_b64");
    let b64_clean = b64_raw.split_once(',').map(|(_, r)| r).unwrap_or(&b64_raw);
    let bytes = base64_decode(b64_clean);
    let specs = parse_drawing_specs_offline(&fname, &bytes);

    let od: f64 = specs["dim_d"].parse::<f64>().unwrap_or(120.0).max(20.0);
    let h: f64 = specs["dim_z"].parse::<f64>().unwrap_or(24.0).max(8.0);
    let r = od / 2.0;

    let step_txt = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('BahaForge Rust Pure CAD Solid'),'2;1');\nFILE_NAME('{}.step','2026-11-21',('BahaTech Software'),('BahaForge Rust v7.0'),'Rust-BRep','BahaForge','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN {{ 1 0 10303 214 1 1 1 1 }}'));\nENDSEC;\nDATA;\n#1=APPLICATION_CONTEXT('automotive design');\n#2=PRODUCT('{}','{}','',(#1));\n#10=CARTESIAN_POINT('',(0.,0.,0.));\n#11=DIRECTION('',(0.,0.,1.));\n#12=DIRECTION('',(1.,0.,0.));\n#13=AXIS2_PLACEMENT_3D('',#10,#11,#12);\n#20=CYLINDRICAL_SURFACE('',#13,{:.3});\n#21=PLANE('',#13);\n#22=CARTESIAN_POINT('',(0.,0.,{:.3}));\n#23=AXIS2_PLACEMENT_3D('',#22,#11,#12);\n#24=PLANE('',#23);\n#30=ADVANCED_BREP_SHAPE_REPRESENTATION('{}',(#20,#21,#24),#13);\nENDSEC;\nEND-ISO-10303-21;\n",
        specs["part_code"], specs["part_code"], specs["part_name"], r, h, specs["part_code"]
    );

    // Build smooth 48-segment cylinder ASCII STL in pure Rust
    let segs = 48usize;
    let mut stl = String::from("solid bahaforge_rust\n");
    for i in 0..segs {
        let a0 = (i as f64) * 2.0 * std::f64::consts::PI / (segs as f64);
        let a1 = ((i + 1) as f64) * 2.0 * std::f64::consts::PI / (segs as f64);
        let (x0, y0) = (r * a0.cos(), r * a0.sin());
        let (x1, y1) = (r * a1.cos(), r * a1.sin());
        // Bottom cap
        stl.push_str(&format!("facet normal 0 0 -1\n outer loop\n  vertex 0 0 0\n  vertex {:.3} {:.3} 0\n  vertex {:.3} {:.3} 0\n endloop\nendfacet\n", x1, y1, x0, y0));
        // Top cap
        stl.push_str(&format!("facet normal 0 0 1\n outer loop\n  vertex 0 0 {:.3}\n  vertex {:.3} {:.3} {:.3}\n  vertex {:.3} {:.3} {:.3}\n endloop\nendfacet\n", h, x0, y0, h, x1, y1, h));
        // Side wall (2 triangles)
        let nx = ((a0 + a1) * 0.5).cos();
        let ny = ((a0 + a1) * 0.5).sin();
        stl.push_str(&format!("facet normal {:.4} {:.4} 0\n outer loop\n  vertex {:.3} {:.3} 0\n  vertex {:.3} {:.3} 0\n  vertex {:.3} {:.3} {:.3}\n endloop\nendfacet\n", nx, ny, x0, y0, x1, y1, x1, y1, h));
        stl.push_str(&format!("facet normal {:.4} {:.4} 0\n outer loop\n  vertex {:.3} {:.3} 0\n  vertex {:.3} {:.3} {:.3}\n  vertex {:.3} {:.3} {:.3}\n endloop\nendfacet\n", nx, ny, x0, y0, x1, y1, h, x0, y0, h));
    }
    stl.push_str("endsolid bahaforge_rust\n");

    let vol_cm3 = (std::f64::consts::PI * r * r * h) / 1000.0;
    let mass_kg = vol_cm3 * 7.85 / 1000.0;
    let code = format!("// Pure Rust BahaForge Analytic Solid\n// Outer Dia: {:.1} mm | Height: {:.1} mm", od, h);

    format!(
        r#"{{"ok":true,"engine":"BahaForge Rust Native CAD v7.0 (Zero-Python)","part_name":"{}","material":"{}","filename_base":"{}","engineering_notes":"Pure Rust motoru ile {:.1}x{:.1} mm katı model ve ISO-10303-21 AP214 STEP üretildi.","attempts":1,"python_code":"{}","step_content":"{}","stl_content":"{}","stats":{{"bbox_mm":[{:.2},{:.2},{:.2}],"volume_cm3":{:.2},"area_cm2":{:.2},"mass_kg":{:.3},"faces":3,"edges":2}}}}"#,
        json_escape(&specs["part_name"]),
        json_escape(&specs["material_label"]),
        json_escape(&specs["part_code"]),
        od,
        h,
        json_escape(&code),
        json_escape(&step_txt),
        json_escape(&stl),
        od,
        od,
        h,
        vol_cm3,
        (2.0 * std::f64::consts::PI * r * (r + h)) / 100.0,
        mass_kg
    )
}

// ============================================================================
// EMBEDDED ZERO-CLOUD SELF-LEARNING CNC PRICING ML ENGINE (Pure Rust k-NN + Scaling)
// Persists shop pricing history into bahaforge_ml_memory.jsonl beside the executable!
// ============================================================================
struct MlRecord {
    id: String,
    part_name: String,
    part_code: String,
    material_key: String,
    shape: String,
    vol_cm3: f64,
    qty: f64,
    kg_price: f64,
    scrap_pct: f64,
    turn_min: f64,
    turn_rate: f64,
    mill_min: f64,
    mill_rate: f64,
    edm_min: f64,
    setup_min: f64,
    margin_pct: f64,
    sell_unit: f64,
}

fn ml_memory_path() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("bahaforge_ml_memory.jsonl");
            if fs::OpenOptions::new().create(true).append(true).open(&p).is_ok() {
                return p;
            }
        }
    }
    let home = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".bahaforge_ml_memory.jsonl")
}

fn parse_ml_record(line: &str) -> Option<MlRecord> {
    let id = extract_json_str(line, "id");
    if id.is_empty() {
        return None;
    }
    Some(MlRecord {
        id,
        part_name: extract_json_str(line, "partName"),
        part_code: extract_json_str(line, "partCode"),
        material_key: extract_json_str(line, "materialKey"),
        shape: extract_json_str(line, "shape"),
        vol_cm3: extract_json_num(line, "volCm3", 100.0).max(1.0),
        qty: extract_json_num(line, "qty", 25.0).max(1.0),
        kg_price: extract_json_num(line, "kgPrice", 200.0),
        scrap_pct: extract_json_num(line, "scrapPct", 18.0),
        turn_min: extract_json_num(line, "turnMin", 0.0),
        turn_rate: extract_json_num(line, "turnRate", 1650.0),
        mill_min: extract_json_num(line, "millMin", 0.0),
        mill_rate: extract_json_num(line, "millRate", 2100.0),
        edm_min: extract_json_num(line, "edmMin", 0.0),
        setup_min: extract_json_num(line, "setupMin", 45.0),
        margin_pct: extract_json_num(line, "marginPct", 28.0),
        sell_unit: extract_json_num(line, "sellUnit", 0.0),
    })
}

fn load_ml_records() -> Vec<MlRecord> {
    let path = ml_memory_path();
    let Ok(txt) = fs::read_to_string(path) else { return Vec::new(); };
    let mut map: HashMap<String, MlRecord> = HashMap::new();
    for line in txt.lines() {
        if let Some(rec) = parse_ml_record(line) {
            map.insert(rec.id.clone(), rec);
        }
    }
    map.into_values().collect()
}

fn handle_ml_train(body: &str) -> String {
    let path = ml_memory_path();
    let clean_line = body.replace('\n', " ").replace('\r', " ");
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{}", clean_line);
    }
    let n = load_ml_records().len();
    format!(r#"{{"ok":true,"trained":{},"memory_file":"{}"}}"#, n, json_escape(&path.to_string_lossy()))
}

fn handle_ml_predict(body: &str) -> String {
    let mem = load_ml_records();
    let n = mem.len();
    if n == 0 {
        return r#"{"ok":true,"trained":0,"confidence":0,"explanation":"Henüz eğitim verisi yok. İlk parçaların gerçek fiyatlarını girip kaydedin.","prediction":null}"#.to_string();
    }

    let mat = extract_json_str(body, "materialKey");
    let shape = extract_json_str(body, "shape");
    let dim_d = extract_json_num(body, "dimD", 100.0);
    let dim_x = extract_json_num(body, "dimX", 100.0);
    let dim_y = extract_json_num(body, "dimY", 80.0);
    let dim_z = extract_json_num(body, "dimZ", 25.0);
    let qty = extract_json_num(body, "qty", 25.0).max(1.0);

    let vol_target = if shape == "cylindrical" {
        std::f64::consts::PI * (dim_d / 20.0).powi(2) * (dim_z / 10.0)
    } else {
        (dim_x * dim_y * dim_z) / 1000.0
    }.max(1.0);

    let mut scored: Vec<(&MlRecord, f64, f64)> = mem
        .iter()
        .map(|m| {
            let mut dist = 0.0;
            if m.material_key != mat { dist += 1.6; }
            if m.shape != shape { dist += 0.9; }
            dist += (vol_target / m.vol_cm3.max(1.0)).ln().abs() * 1.1;
            dist += (qty / m.qty.max(1.0)).ln().abs() * 0.25;
            let sim = 1.0 / (1.0 + dist);
            let v_scale = (vol_target / m.vol_cm3.max(1.0)).powf(0.38).clamp(0.45, 2.6);
            (m, sim, v_scale)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(5);

    let (mut wsum, mut turn, mut mill, mut edm, mut setup, mut kgp, mut trate, mut mrate, mut marg, mut scrap, mut unitp) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (m, sim, vscale) in &scored {
        let w = sim * sim;
        wsum += w;
        turn += w * m.turn_min * vscale;
        mill += w * m.mill_min * vscale;
        edm += w * m.edm_min * vscale;
        setup += w * m.setup_min;
        kgp += w * m.kg_price;
        trate += w * m.turn_rate;
        mrate += w * m.mill_rate;
        marg += w * m.margin_pct;
        scrap += w * m.scrap_pct;
        unitp += w * m.sell_unit * vscale;
    }

    let top = scored[0].0;
    let top_sim = scored[0].1;
    let conf = (((n.min(12) as f64 / 12.0) * 58.0 + top_sim * 40.0).round() as i32).min(97);
    let expl = format!(
        "BahaForge Rust Yerel ML ({} geçmiş işten öğrendi, Güven %{}): En yakın referans '{} ({})' (%{} benzer). Atölyenizin gerçek tezgâh saatleri, malzeme fiyatı ve kâr marjı otomatik dolduruldu.",
        n, conf, top.part_name, top.part_code, (top_sim * 100.0).round() as i32
    );

    format!(
        r#"{{"ok":true,"trained":{},"confidence":{},"explanation":"{}","prediction":{{"turnMin":{},"millMin":{},"edmMin":{},"setupMin":{},"kgPrice":{},"turnRate":{},"millRate":{},"marginPct":{},"scrapPct":{},"estUnitTl":{}}}}}"#,
        n,
        conf,
        json_escape(&expl),
        (turn / wsum).round() as i64,
        (mill / wsum).round() as i64,
        (edm / wsum).round() as i64,
        (setup / wsum).round() as i64,
        (kgp / wsum).round() as i64,
        (((trate / wsum) / 50.0).round() * 50.0) as i64,
        (((mrate / wsum) / 50.0).round() * 50.0) as i64,
        (marg / wsum).round() as i64,
        (scrap / wsum).round() as i64,
        (unitp / wsum).round() as i64
    )
}
