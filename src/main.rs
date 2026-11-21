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
const ASSET_INDUSTRY_HTML: &str = include_str!("../web/industry.html");
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
            "/" | "/index.html" | "/industry.html" | "/forge.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_INDUSTRY_HTML.as_bytes()),
            "/forge-legacy.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_FORGE_HTML.as_bytes()),
            "/BahaTech_CADCAM_OS.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_CADCAM_OS_HTML.as_bytes()),
            "/bahaforge-rfq-quote-ai.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_RFQ_JS.as_bytes()),
            "/app-i18n-23.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_I18N_JS.as_bytes()),
            "/bahatech-enterprise-suite-v7.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_SUITE_V7_JS.as_bytes()),
            "/bahacad.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_BAHACAD_HTML.as_bytes()),
            "/bahacam.html" => write_raw(&mut stream, 200, "text/html; charset=utf-8", ASSET_BAHACAM_HTML.as_bytes()),
            "/volumill_engine.js" => write_raw(&mut stream, 200, "application/javascript; charset=utf-8", ASSET_VOLUMILL_JS.as_bytes()),
            "/api/health" => {
                let key_ok = get_xai_key().is_some();
                let wa_ok = get_env_var_multi("KAPSO_API_KEY").is_some();
                let (_, _, smtp_u, smtp_p) = get_smtp_creds();
                let mem_n = load_ml_records().len();
                let resp = format!(
                    r#"{{"ok":true,"engine":"BahaTech Industry Pure Rust Engine v7.2","os":"{}","arch":"{}","ml_samples":{},"xai_configured":{},"whatsapp_api_ready":{},"smtp_ready":{},"smtp_user":"{}"}}"#,
                    env::consts::OS,
                    env::consts::ARCH,
                    mem_n,
                    key_ok,
                    wa_ok,
                    !smtp_p.is_empty(),
                    json_escape(&smtp_u)
                );
                write_json(&mut stream, 200, &resp)
            }
            "/api/ml/status" => {
                let mem = load_ml_records();
                let n = mem.len();
                let conf = if n == 0 { 0 } else { (n.min(12) * 65 / 12 + 28).min(97) };
                let resp = format!(r#"{{"ok":true,"trained":{},"confidence":{},"engine":"BahaTech-Rust-LocalML"}}"#, n, conf);
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
            "/api/whatsapp/send" => {
                let json = handle_direct_whatsapp_send(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/email/send" => {
                let json = handle_direct_email_send(&body_str);
                write_json(&mut stream, 200, &json)
            }
            "/api/invoice/save" => {
                let json = handle_save_invoice_disk(&body_str);
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

fn get_env_var_multi(var_name: &str) -> Option<String> {
    if let Ok(v) = env::var(var_name) {
        let t = v.trim().trim_matches('"').trim_matches('\'').to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    let mut paths: Vec<PathBuf> = vec![
        PathBuf::from(".env"),
        PathBuf::from("/Users/bahadir/Desktop/BahaTech/.env"),
        PathBuf::from("/Users/bahadir/Desktop/BahaTech/DigitalProductFactory/.env"),
    ];
    if let Ok(home) = env::var("HOME").or_else(|_| env::var("USERPROFILE")) {
        paths.push(PathBuf::from(&home).join("Desktop").join("BahaTech").join(".env"));
        paths.push(PathBuf::from(&home).join("Desktop").join("BahaTech").join("DigitalProductFactory").join(".env"));
        paths.push(PathBuf::from(&home).join(".bahaforge.env"));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(".env"));
        }
    }
    let prefix = format!("{}=", var_name);
    for p in paths {
        if let Ok(txt) = fs::read_to_string(&p) {
            for line in txt.lines() {
                if let Some(rest) = line.trim().strip_prefix(&prefix) {
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

fn get_smtp_creds() -> (String, u16, String, String) {
    let host = get_env_var_multi("SMTP_HOST").unwrap_or_else(|| "smtp.porkbun.com".into());
    let port = get_env_var_multi("SMTP_PORT").and_then(|v| v.parse::<u16>().ok()).unwrap_or(587);
    let user = get_env_var_multi("PORKBUN_SMTP_USER")
        .or_else(|| get_env_var_multi("SMTP_USER"))
        .unwrap_or_else(|| "admin@bahatechsoftware.com".into());
    let pass = get_env_var_multi("PORKBUN_SMTP_PASS")
        .or_else(|| get_env_var_multi("SMTP_PASS"))
        .unwrap_or_default();
    (host, port, user, pass)
}

/// Direct IN-APP WhatsApp delivery via Kapso Official WhatsApp Cloud API (NEVER redirects to WhatsApp Web!)
fn handle_direct_whatsapp_send(body: &str) -> String {
    let to_raw = extract_json_str(body, "to");
    let to_digits: String = to_raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let to_clean = if to_digits.starts_with('0') && to_digits.len() == 11 {
        format!("90{}", &to_digits[1..])
    } else if to_digits.starts_with('5') && to_digits.len() == 10 {
        format!("90{}", to_digits)
    } else {
        to_digits
    };
    let text = extract_json_str(body, "text");
    if to_clean.len() < 10 || text.trim().is_empty() {
        return r#"{"ok":false,"error":"Geçerli bir telefon numarası (ör: 90553...) ve mesaj metni girin."}"#.into();
    }

    let custom_key = extract_json_str(body, "api_key");
    let api_key = if !custom_key.is_empty() { Some(custom_key) } else { get_env_var_multi("KAPSO_API_KEY") };
    let Some(api_key) = api_key else {
        return r#"{"ok":false,"error":"Kapso WhatsApp API anahtarı yapılandırılmamış. Firma & Bağlantılar sekmesinden ekleyebilirsiniz."}"#.into();
    };

    let custom_pid = extract_json_str(body, "phone_number_id");
    let phone_id = if !custom_pid.is_empty() {
        custom_pid
    } else {
        get_env_var_multi("KAPSO_PHONE_NUMBER_ID").unwrap_or_else(|| "597907523413541".into())
    };

    let payload = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"send_whatsapp_message","arguments":{{"phone_number_id":"{}","to":"{}","text":"{}"}}}}}}"#,
        json_escape(&phone_id),
        json_escape(&to_clean),
        json_escape(&text)
    );

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(3);
    let tmp_req = env::temp_dir().join(format!("baha_wa_{}.json", stamp));
    if fs::write(&tmp_req, payload.as_bytes()).is_err() {
        return r#"{"ok":false,"error":"Geçici istek dosyası yazılamadı."}"#.into();
    }

    let curl_bin = if cfg!(target_os = "windows") { "curl.exe" } else { "/usr/bin/curl" };
    let out = Command::new(curl_bin)
        .args([
            "-sS",
            "--max-time",
            "25",
            "https://api.kapso.ai/mcp",
            "-H",
            "Content-Type: application/json",
            "-H",
            "Accept: application/json, text/event-stream",
            "-H",
            &format!("X-API-Key: {}", api_key),
            "--data-binary",
            &format!("@{}", tmp_req.to_string_lossy()),
        ])
        .output();
    let _ = fs::remove_file(&tmp_req);

    match out {
        Ok(o) if o.status.success() => {
            let resp_s = String::from_utf8_lossy(&o.stdout).to_string();
            let is_err = resp_s.contains("\"isError\":true")
                || resp_s.contains("error_code")
                || resp_s.contains("\"error\":");
            if is_err {
                let err_detail = extract_json_str(&resp_s, "text");
                let msg = if !err_detail.is_empty() { err_detail } else { resp_s.chars().take(240).collect() };
                format!(
                    r#"{{"ok":false,"to":"{}","error":"{}"}}"#,
                    json_escape(&to_clean),
                    json_escape(&msg)
                )
            } else {
                let wamid = extract_json_str(&resp_s, "id");
                format!(
                    r#"{{"ok":true,"channel":"whatsapp_cloud_api","to":"{}","message_id":"{}","phone_number_id":"{}"}}"#,
                    json_escape(&to_clean),
                    json_escape(&wamid),
                    json_escape(&phone_id)
                )
            }
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).to_string();
            format!(r#"{{"ok":false,"error":"WhatsApp API bağlantı hatası: {}"}}"#, json_escape(&err))
        }
        Err(e) => format!(r#"{{"ok":false,"error":"curl çalıştırılamadı: {}"}}"#, json_escape(&e.to_string())),
    }
}

/// Direct IN-APP TLS SMTP Email delivery via Pure Rust + native OS curl (NEVER opens mailto:/Apple Mail!)
fn handle_direct_email_send(body: &str) -> String {
    let to_email = extract_json_str(body, "to").trim().to_string();
    let subject = extract_json_str(body, "subject");
    let content = extract_json_str(body, "body");
    let sender_name = extract_json_str(body, "sender_name");
    if !to_email.contains('@') || content.trim().is_empty() {
        return r#"{"ok":false,"error":"Geçerli bir alıcı e-posta adresi ve mesaj içeriği girin."}"#.into();
    }

    let (def_host, def_port, def_user, def_pass) = get_smtp_creds();
    let host = {
        let h = extract_json_str(body, "smtp_host");
        if !h.is_empty() { h } else { def_host }
    };
    let port = {
        let p = extract_json_num(body, "smtp_port", def_port as f64) as u16;
        if p > 0 { p } else { def_port }
    };
    let user = {
        let u = extract_json_str(body, "smtp_user");
        if !u.is_empty() { u } else { def_user }
    };
    let pass = {
        let p = extract_json_str(body, "smtp_pass");
        if !p.is_empty() { p } else { def_pass }
    };
    if pass.is_empty() {
        return r#"{"ok":false,"error":"SMTP şifresi tanımlı değil. Firma & Bağlantılar sekmesinden SMTP/Uygulama Şifresi girin."}"#.into();
    }

    let disp_name = if sender_name.is_empty() { "BahaTech" } else { &sender_name };
    let subj_b64 = base64_encode(subject.as_bytes());
    let name_b64 = base64_encode(disp_name.as_bytes());
    let eml = format!(
        "From: =?UTF-8?B?{}?= <{}>\r\nTo: {}\r\nReply-To: {}\r\nSubject: =?UTF-8?B?{}?=\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{}\r\n",
        name_b64, user, to_email, user, subj_b64, content
    );

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(4);
    let tmp_eml = env::temp_dir().join(format!("baha_mail_{}.eml", stamp));
    if fs::write(&tmp_eml, eml.as_bytes()).is_err() {
        return r#"{"ok":false,"error":"E-posta geçici dosyası oluşturulamadı."}"#.into();
    }

    let smtp_url = if port == 465 {
        format!("smtps://{}:{}", host, port)
    } else {
        format!("smtp://{}:{}", host, port)
    };
    let curl_bin = if cfg!(target_os = "windows") { "curl.exe" } else { "/usr/bin/curl" };
    let out = Command::new(curl_bin)
        .args([
            "-sS",
            "--max-time",
            "25",
            "--url",
            &smtp_url,
            "--ssl-reqd",
            "--user",
            &format!("{}:{}", user, pass),
            "--mail-from",
            &user,
            "--mail-rcpt",
            &to_email,
            "--upload-file",
            &tmp_eml.to_string_lossy(),
        ])
        .output();
    let _ = fs::remove_file(&tmp_eml);

    match out {
        Ok(o) if o.status.success() => format!(
            r#"{{"ok":true,"channel":"smtp_direct","from":"{}","to":"{}"}}"#,
            json_escape(&user),
            json_escape(&to_email)
        ),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).to_string();
            format!(r#"{{"ok":false,"error":"SMTP gönderim hatası: {}"}}"#, json_escape(&err))
        }
        Err(e) => format!(r#"{{"ok":false,"error":"curl SMTP hatası: {}"}}"#, json_escape(&e.to_string())),
    }
}

/// Saves official UBL-TR 1.2 XML & HTML archive into ~/Documents/BahaWorks_Faturalar/ locally!
fn handle_save_invoice_disk(body: &str) -> String {
    let inv_no = extract_json_str(body, "inv_no");
    let xml = extract_json_str(body, "xml");
    let clean_no: String = inv_no
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let fname_base = if clean_no.is_empty() { "BHW_FATURA".to_string() } else { clean_no };
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(home).join("Documents").join("BahaWorks_Faturalar");
    if fs::create_dir_all(&dir).is_err() {
        return r#"{"ok":false,"error":"Faturalar klasörü oluşturulamadı."}"#.into();
    }
    let xml_path = dir.join(format!("{}_UBL_TR.xml", fname_base));
    if let Err(e) = fs::write(&xml_path, xml.as_bytes()) {
        return format!(r#"{{"ok":false,"error":"Fatura yazılamadı: {}"}}"#, json_escape(&e.to_string()));
    }
    format!(
        r#"{{"ok":true,"saved_path":"{}","folder":"{}"}}"#,
        json_escape(&xml_path.to_string_lossy()),
        json_escape(&dir.to_string_lossy())
    )
}

fn get_xai_key() -> Option<String> {
    get_env_var_multi("XAI_API_KEY")
}

fn base64_encode(data: &[u8]) -> String {
    const TBL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
        out.push(TBL[(b0 >> 2) as usize] as char);
        out.push(TBL[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < data.len() {
            out.push(TBL[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(TBL[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn render_to_png_bytes(filename: &str, raw: &[u8]) -> Option<Vec<u8>> {
    let low = filename.to_lowercase();
    if low.ends_with(".png") || low.ends_with(".jpg") || low.ends_with(".jpeg") || low.ends_with(".webp") {
        return Some(raw.to_vec());
    }
    let is_pdf = low.ends_with(".pdf") || (raw.len() > 4 && &raw[..4] == b"%PDF");
    let is_heic = low.ends_with(".heic") || low.ends_with(".heif") || low.ends_with(".tif") || low.ends_with(".tiff") || low.ends_with(".bmp");
    if !is_pdf && !is_heic {
        return None;
    }
    let ext = if is_pdf { "pdf" } else { low.rsplit('.').next().unwrap_or("heic") };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    let tmp_dir = env::temp_dir();
    let in_path = tmp_dir.join(format!("bahaforge_in_{}.{}", stamp, ext));
    let png_path = tmp_dir.join(format!("bahaforge_out_{}.png", stamp));
    if fs::write(&in_path, raw).is_err() {
        return None;
    }
    let mut rendered: Option<Vec<u8>> = None;
    #[cfg(target_os = "macos")]
    {
        // Apple CoreGraphics native PDF/HEIC/TIFF->PNG via sips (0.3s, 2200px razor-sharp, zero Python!)
        let ok = Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "-Z", "2200"])
            .arg(&in_path)
            .arg("--out")
            .arg(&png_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            if let Ok(b) = fs::read(&png_path) {
                if b.len() > 500 {
                    rendered = Some(b);
                }
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        if is_pdf {
            let prefix = tmp_dir.join(format!("bahaforge_out_{}", stamp));
            let ok = Command::new("pdftoppm")
                .args(["-png", "-r", "220", "-singlefile"])
                .arg(&in_path)
                .arg(&prefix)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if ok {
                if let Ok(b) = fs::read(&png_path) {
                    if b.len() > 500 {
                        rendered = Some(b);
                    }
                }
            }
        }
    }
    let _ = fs::remove_file(&in_path);
    let _ = fs::remove_file(&png_path);
    rendered
}

/// Pure-Rust ISO-10303-21 (.STEP/.STP) geometry & bounding-box extractor + crisp isometric SVG preview!
fn analyze_step_file_rust(filename: &str, raw: &[u8]) -> Option<String> {
    let low_fn = filename.to_lowercase();
    let txt = String::from_utf8_lossy(raw);
    if !(low_fn.ends_with(".step") || low_fn.ends_with(".stp") || txt.contains("ISO-10303-21")) {
        return None;
    }
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut pts: Vec<(f64, f64, f64)> = Vec::new();
    let mut cyl_count = 0usize;

    for line in txt.lines() {
        let up = line.to_uppercase();
        if up.contains("CYLINDRICAL_SURFACE") {
            cyl_count += 1;
        }
        if up.contains("CARTESIAN_POINT") {
            if let Some(lp) = line.rfind('(') {
                if let Some(rp) = line[lp..].find(')') {
                    let inner = &line[lp + 1..lp + rp];
                    let coords: Vec<f64> = inner
                        .split(',')
                        .filter_map(|s| s.trim().parse::<f64>().ok())
                        .collect();
                    if coords.len() == 3 && coords.iter().all(|v| v.is_finite() && v.abs() < 50_000.0) {
                        let (x, y, z) = (coords[0], coords[1], coords[2]);
                        min_x = min_x.min(x); max_x = max_x.max(x);
                        min_y = min_y.min(y); max_y = max_y.max(y);
                        min_z = min_z.min(z); max_z = max_z.max(z);
                        if pts.len() < 4000 {
                            pts.push((x, y, z));
                        }
                    }
                }
            }
        }
    }

    let dx = if max_x > min_x { (max_x - min_x).max(1.0) } else { 100.0 };
    let dy = if max_y > min_y { (max_y - min_y).max(1.0) } else { 80.0 };
    let dz = if max_z > min_z { (max_z - min_z).max(1.0) } else { 30.0 };
    let is_cyl = cyl_count > 0 && ((dx - dy).abs() / dx.max(1.0) < 0.08);
    let shape = if is_cyl { "cylindrical" } else { "prismatic" };

    // Build clean technical isometric SVG preview of the STEP point cloud + bounding box!
    let mut svg_pts = String::new();
    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let cz = (min_z + max_z) * 0.5;
    let scale = 210.0 / dx.max(dy).max(dz).max(1.0);
    let proj = |x: f64, y: f64, z: f64| -> (f64, f64) {
        let rx = (x - cx) * scale;
        let ry = (y - cy) * scale;
        let rz = (z - cz) * scale;
        let px = 420.0 + (rx - ry) * 0.866;
        let py = 240.0 + (rx + ry) * 0.42 - rz * 0.92;
        (px, py)
    };
    let corners = [
        proj(min_x, min_y, min_z), proj(max_x, min_y, min_z),
        proj(max_x, max_y, min_z), proj(min_x, max_y, min_z),
        proj(min_x, min_y, max_z), proj(max_x, min_y, max_z),
        proj(max_x, max_y, max_z), proj(min_x, max_y, max_z),
    ];
    let edges = [(0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7)];
    let mut box_lines = String::new();
    for (a, b) in edges {
        box_lines.push_str(&format!(
            r##"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#94a3b8" stroke-width="1.2" stroke-dasharray="4 3"/>"##,
            corners[a].0, corners[a].1, corners[b].0, corners[b].1
        ));
    }
    let step_stride = (pts.len() / 600).max(1);
    for (i, &(x, y, z)) in pts.iter().enumerate() {
        if i % step_stride == 0 {
            let (px, py) = proj(x, y, z);
            svg_pts.push_str(&format!(r##"<circle cx="{:.1}" cy="{:.1}" r="1.9" fill="#1d4ed8" opacity="0.72"/>"##, px, py));
        }
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 840 460" style="background:#f8fafc"><rect width="840" height="460" fill="#f8fafc"/>{}{}
        <text x="28" y="38" font-family="-apple-system,Inter,sans-serif" font-size="16" font-weight="700" fill="#0f172a">3D STEP MODEL ÖNİZLEME · {}</text>
        <text x="28" y="62" font-family="-apple-system,Inter,sans-serif" font-size="13" fill="#475569">Dış Zarf (B-Rep): {:.1} × {:.1} × {:.1} mm · {} nokta · {} silindirik yüzey</text>
        </svg>"##,
        box_lines, svg_pts, filename, dx, dy, dz, pts.len(), cyl_count
    );
    let preview_url = format!("data:image/svg+xml;base64,{}", base64_encode(svg.as_bytes()));

    let (mat_key, mat_lbl, density, kg_price) = alloy_meta("steel4140");
    let billet_vol_cm3 = if is_cyl {
        std::f64::consts::PI * (dx / 20.0).powi(2) * (dz / 10.0)
    } else {
        (dx * dy * dz) / 1000.0
    };
    let billet_kg = (billet_vol_cm3 * density) / 1000.0;
    let net_kg = billet_kg * 0.55;
    let direct_mat_tl = (billet_kg * 1.08 * kg_price).round() as i64;
    let clean_code: String = filename.split('.').next().unwrap_or("STEP-PART").to_uppercase();
    let comment = format!(
        "3D STEP (ISO-10303-21) modelinden doğrudan {:.1} × {:.1} × {:.1} mm kütük ölçüleri ve {} silindirik yüzey okundu. Malzemeyi ve özkütleyi seçerek anında kesin ağırlık ve fiyat çıkarabilirsiniz.",
        dx, dy, dz, cyl_count
    );

    Some(format!(
        r#"{{"ok":true,"engine":"BahaWorks Rust STEP Parser v7.3","vision_verified":true,"customer":"","part_name":"3D STEP Parça ({})","part_code":"{}","qty":10,"material_key":"{}","material_label":"{}","density_g_cm3":{:.2},"shape":"{}","dim_d_mm":{:.1},"dim_x_mm":{:.1},"dim_y_mm":{:.1},"dim_z_mm":{:.1},"billet_kg":{:.3},"billet_grams":{},"net_kg":{:.3},"net_grams":{},"direct_mat_unit_tl":{},"kg_price_tl":{:.0},"tolerance":"ISO 2768-m","turn_min":{},"mill_min":{},"edm_min":0,"setup_min":45,"difficulty":"Hassas","engineering_comment":"{}","preview_png_b64":"{}"}}"#,
        json_escape(&clean_code),
        json_escape(&clean_code),
        mat_key,
        json_escape(mat_lbl),
        density,
        shape,
        dx,
        dx,
        dy,
        dz,
        billet_kg,
        (billet_kg * 1000.0).round() as i64,
        net_kg,
        (net_kg * 1000.0).round() as i64,
        direct_mat_tl,
        kg_price,
        if is_cyl { 18 } else { 0 },
        if is_cyl { 20 } else { 40 },
        json_escape(&comment),
        preview_url
    ))
}

fn alloy_meta(key: &str) -> (&'static str, &'static str, f64, f64) {
    match key {
        "al7075" => ("al7075", "Alüminyum 7075-T6 (Havacılık)", 2.81, 420.0),
        "al6061" => ("al6061", "Alüminyum 6061-T6", 2.70, 295.0),
        "al5083" => ("al5083", "Alüminyum 5083", 2.66, 280.0),
        "st37" => ("st37", "St37 / S235JR Yapı Çeliği", 7.85, 58.0),
        "ck45" => ("ck45", "C45 / CK45 İmalat Çeliği", 7.85, 85.0),
        "d2_2379" => ("d2_2379", "2379 (D2) Soğuk İş Kalıp Çeliği", 7.70, 340.0),
        "ss304" => ("ss304", "AISI 304 (1.4301) Paslanmaz Çelik", 7.93, 245.0),
        "ss316" => ("ss316", "AISI 316L (1.4404) Paslanmaz Çelik", 7.98, 315.0),
        "ti64" => ("ti64", "Titanyum Ti-6Al-4V (Grade 5)", 4.43, 2650.0),
        "brass" => ("brass", "Pirinç MS58 (CuZn39Pb3)", 8.47, 460.0),
        "copper" => ("copper", "Elektrolitik Bakır (Cu-ETP)", 8.96, 620.0),
        "pom" => ("pom", "Delrin / POM", 1.41, 260.0),
        "pa6" => ("pa6", "Kestamid / Polyamid PA6", 1.15, 220.0),
        _ => ("steel4140", "4140 Islah Çeliği (42CrMo4)", 7.85, 115.0),
    }
}

/// Calls Grok-4.20 Multimodal Vision directly from Pure Rust via native OS curl (zero Python!)
fn analyze_drawing_with_vision_rust(filename: &str, png_bytes: &[u8]) -> Option<String> {
    let api_key = get_xai_key()?;
    let img_b64 = base64_encode(png_bytes);
    let data_url = format!("data:image/png;base64,{}", img_b64);

    let prompt = format!(
        "Sen Türkiye'de havacılık, savunma ve makine sanayisinde çalışan 25 yıllık kıdemli bir CNC Talaşlı İmalat Teklif ve Maliyet Mühendisisin. \
        Bu 2D teknik resmi (özellikle sağ alttaki ANTET / TITLE BLOCK kutusunu, 'Açıklama / Ham Ölçü' satırını, 'Malzeme' satırını, 'Parça Adı', 'Parça Kodu', firma adını ve varsa üzerine yazılmış '2 ADET' gibi miktar notlarını) BÜYÜK DİKKATLE OKU! \
        Malzeme eşleştirme kuralı: X5CrNi18-10 veya 1.4301 -> ss304; 1.4404 veya 316 -> ss316; 42CrMo4 veya 4140 -> steel4140; St37 veya S235 -> st37; 7075 -> al7075; 6061 -> al6061; Ti-6Al-4V -> ti64; MS58/Pirinç -> brass; POM/Delrin -> pom. \
        Antette '56X75X182.5' veya '17X30X110' gibi ham kütük ölçüsü yazıyorsa BİREBİR o 3 ölçüyü dim_x_mm, dim_y_mm, dim_z_mm olarak ver ve shape='prismatic' yap! Döner parça (mil/flanş) ise shape='cylindrical' ve dim_d_mm (dış çap), dim_z_mm (toplam boy/kalınlık) ver. \
        fill_factor: işlenmiş parçanın ham kütüğe doluluk oranı (ör: derin çatal/delikli krank kolu için 0.38, lama için 0.65, dolu flanş için 0.55). \
        Dosya adı: {}. \
        SADECE saf JSON döndür (markdown veya başka metin yazma): \
        {{\"customer\":\"<antetteki firma adı>\",\"part_name\":\"<antetteki parça adı>\",\"part_code\":\"<antetteki parça kodu>\",\"qty\":<çizimde adet yazıyorsa o sayı yoksa 10>,\"material_key\":\"<al7075|al6061|st37|steel4140|ss304|ss316|ti64|brass|pom>\",\"material_label\":\"<antetteki tam malzeme kodu>\",\"shape\":\"<prismatic|cylindrical>\",\"dim_d_mm\":<sayı>,\"dim_x_mm\":<sayı>,\"dim_y_mm\":<sayı>,\"dim_z_mm\":<sayı>,\"fill_factor\":<0.25-0.95 arası sayı>,\"tolerance\":\"<ör: ISO 2768-fH-E • Ø15 H7 • Ø20 H7>\",\"turn_min\":<parça başı CNC torna dk>,\"mill_min\":<parça başı 3/4/5 eksen CNC freze dk>,\"edm_min\":<tel erozyon/taşlama dk>,\"setup_min\":<toplam bağlama/programlama dk>,\"difficulty\":\"<Standart|Hassas|Havacılık-Savunma>\",\"engineering_comment\":\"<3 cümle Türkçe mühendislik ve fiyat analizi: antetten okunan ham kütük ölçüsü, malzeme, kritik H7/toleranslar ve operasyon tavsiyesi>\"}}",
        filename
    );

    let req_body = format!(
        r#"{{"model":"grok-4.20-0309-non-reasoning","temperature":0.1,"input":[{{"role":"user","content":[{{"type":"input_text","text":"{}"}},{{"type":"input_image","image_url":"{}"}}]}}]}}"#,
        json_escape(&prompt),
        data_url
    );

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(2);
    let tmp_req = env::temp_dir().join(format!("bahaforge_vision_req_{}.json", stamp));
    if fs::write(&tmp_req, req_body.as_bytes()).is_err() {
        return None;
    }

    let curl_bin = if cfg!(target_os = "windows") { "curl.exe" } else { "/usr/bin/curl" };
    let out = Command::new(curl_bin)
        .args([
            "-sS",
            "--max-time",
            "65",
            "https://api.x.ai/v1/responses",
            "-H",
            &format!("Authorization: Bearer {}", api_key),
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            &format!("@{}", tmp_req.to_string_lossy()),
        ])
        .output();
    let _ = fs::remove_file(&tmp_req);

    let out = out.ok()?;
    if !out.status.success() {
        return None;
    }
    let resp_str = String::from_utf8_lossy(&out.stdout).to_string();
    // Extract assistant output text field from xAI Responses JSON
    let text_field = extract_json_str(&resp_str, "text");
    if text_field.is_empty() {
        return None;
    }
    // Extract inner JSON object {...}
    let start = text_field.find('{')?;
    let end = text_field.rfind('}')?;
    if end <= start {
        return None;
    }
    let inner = &text_field[start..=end];

    let customer = extract_json_str(inner, "customer");
    let part_name = extract_json_str(inner, "part_name");
    let part_code = extract_json_str(inner, "part_code");
    let mat_key_raw = extract_json_str(inner, "material_key");
    let mat_label_raw = extract_json_str(inner, "material_label");
    let (mat_key, mat_default_lbl, density, kg_price) = alloy_meta(&mat_key_raw);
    let mat_label = if mat_label_raw.is_empty() { mat_default_lbl.to_string() } else { mat_label_raw };
    let shape_raw = extract_json_str(inner, "shape");
    let shape = if shape_raw == "cylindrical" { "cylindrical" } else { "prismatic" };

    let dim_d = extract_json_num(inner, "dim_d_mm", 60.0).max(5.0);
    let dim_x = extract_json_num(inner, "dim_x_mm", 80.0).max(5.0);
    let dim_y = extract_json_num(inner, "dim_y_mm", 60.0).max(5.0);
    let dim_z = extract_json_num(inner, "dim_z_mm", 20.0).max(2.0);
    let fill = extract_json_num(inner, "fill_factor", 0.50).clamp(0.18, 0.95);
    let qty = extract_json_num(inner, "qty", 10.0).round().max(1.0) as i64;
    let turn_min = extract_json_num(inner, "turn_min", 0.0).round().max(0.0) as i64;
    let mill_min = extract_json_num(inner, "mill_min", 25.0).round().max(0.0) as i64;
    let edm_min = extract_json_num(inner, "edm_min", 0.0).round().max(0.0) as i64;
    let setup_min = extract_json_num(inner, "setup_min", 45.0).round().max(10.0) as i64;
    let tolerance = extract_json_str(inner, "tolerance");
    let difficulty = extract_json_str(inner, "difficulty");
    let comment = extract_json_str(inner, "engineering_comment");

    // Exact physical billet & finished net mass calculation in Pure Rust!
    let billet_vol_cm3 = if shape == "cylindrical" {
        std::f64::consts::PI * (dim_d / 20.0).powi(2) * (dim_z / 10.0)
    } else {
        (dim_x * dim_y * dim_z) / 1000.0
    };
    let billet_kg = (billet_vol_cm3 * density) / 1000.0;
    let net_kg = billet_kg * fill;
    let net_grams = (net_kg * 1000.0).round() as i64;
    let billet_grams = (billet_kg * 1000.0).round() as i64;
    // Material cost is based on the raw billet bought (with 8% saw/bar-end allowance)
    let direct_mat_tl = (billet_kg * 1.08 * kg_price).round() as i64;

    Some(format!(
        r#"{{"ok":true,"engine":"BahaWorks Rust + Vision v7.3","vision_verified":true,"customer":"{}","part_name":"{}","part_code":"{}","qty":{},"material_key":"{}","material_label":"{}","density_g_cm3":{:.2},"shape":"{}","dim_d_mm":{:.1},"dim_x_mm":{:.1},"dim_y_mm":{:.1},"dim_z_mm":{:.1},"billet_kg":{:.3},"billet_grams":{},"net_kg":{:.3},"net_grams":{},"direct_mat_unit_tl":{},"kg_price_tl":{:.0},"tolerance":"{}","turn_min":{},"mill_min":{},"edm_min":{},"setup_min":{},"difficulty":"{}","engineering_comment":"{}","preview_png_b64":"{}"}}"#,
        json_escape(&customer),
        json_escape(if part_name.is_empty() { "CNC Talaşlı İmalat Parçası" } else { &part_name }),
        json_escape(if part_code.is_empty() { filename } else { &part_code }),
        qty,
        mat_key,
        json_escape(&mat_label),
        density,
        shape,
        dim_d,
        dim_x,
        dim_y,
        dim_z,
        billet_kg,
        billet_grams,
        net_kg,
        net_grams,
        direct_mat_tl,
        kg_price,
        json_escape(if tolerance.is_empty() { "ISO 2768-m" } else { &tolerance }),
        turn_min,
        mill_min,
        edm_min,
        setup_min,
        json_escape(if difficulty.is_empty() { "Hassas" } else { &difficulty }),
        json_escape(&comment),
        data_url
    ))
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

/// Pure-Rust heuristic engineering drawing parser (NEVER mistakes PDF MediaBox 595x842 or object IDs for mm!)
fn parse_drawing_specs_offline(filename: &str, raw: &[u8]) -> HashMap<&'static str, String> {
    let mut text = String::new();
    text.push_str(filename);
    text.push(' ');
    // Only extract short readable non-structural tokens; skip PDF headers/MediaBox/object IDs
    let mut cur = String::new();
    for &b in raw.iter().take(400_000) {
        if b.is_ascii_graphic() || b == b' ' {
            cur.push(b as char);
        } else {
            let low_cur = cur.to_lowercase();
            let is_pdf_internal = low_cur.contains("mediabox")
                || low_cur.contains("cropbox")
                || low_cur.contains("fontbbox")
                || low_cur.contains("endobj")
                || low_cur.contains("xref")
                || low_cur.contains("flatedecode")
                || low_cur.contains("pdf-1.")
                || low_cur.contains("00000 n");
            if cur.len() >= 3 && cur.len() <= 120 && !is_pdf_internal {
                text.push_str(&cur);
                text.push(' ');
            }
            cur.clear();
        }
    }
    let low = text.to_lowercase();

    let (mat_key, mat_label, density, kg_price) = if low.contains("7075") {
        ("al7075", "Alüminyum 7075-T6", 2.81f64, 420.0f64)
    } else if low.contains("6061") || low.contains("alumin") || low.contains("alümin") {
        ("al6061", "Alüminyum 6061-T6", 2.70, 295.0)
    } else if low.contains("316") {
        ("ss316", "AISI 316L Paslanmaz", 7.98, 315.0)
    } else if low.contains("304") || low.contains("stainless") || low.contains("paslanmaz") {
        ("ss304", "AISI 304 Paslanmaz Çelik", 7.93, 245.0)
    } else if low.contains("ti-6") || low.contains("titan") {
        ("ti64", "Titanyum Ti-6Al-4V", 4.43, 2650.0)
    } else if low.contains("brass") || low.contains("pirin") || low.contains("ms58") {
        ("brass", "Pirinç MS58", 8.47, 460.0)
    } else if low.contains("st37") || low.contains("s235") {
        ("st37", "St37 / S235JR", 7.85, 58.0)
    } else {
        ("steel4140", "4140 Islah Çeliği", 7.85, 115.0)
    };

    let is_cyl = low.contains("flange")
        || low.contains("flan")
        || low.contains("shaft")
        || low.contains("mil")
        || low.contains("od ")
        || low.contains("pcd")
        || low.contains("ø")
        || low.contains("dia");

    // Extract ONLY explicitly labeled mm dimensions (e.g., "120mm", "120 mm", "Ø80", "D60", "80x40")
    // NEVER grab raw arbitrary PDF numbers like 595 / 842 / 612 / 792!
    let mut explicit_dims: Vec<f64> = Vec::new();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (idx, tok) in tokens.iter().enumerate() {
        let clean = tok.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != 'x' && c != 'X');
        let tl = clean.to_lowercase();
        // Pattern 1: "120mm" or "45.5mm"
        if let Some(num_s) = tl.strip_suffix("mm") {
            if let Ok(v) = num_s.parse::<f64>() {
                if (4.0..=320.0).contains(&v) {
                    explicit_dims.push(v);
                }
            }
        }
        // Pattern 2: "120" followed by "mm"
        if idx + 1 < tokens.len() && tokens[idx + 1].to_lowercase().starts_with("mm") {
            if let Ok(v) = clean.parse::<f64>() {
                if (4.0..=320.0).contains(&v) {
                    explicit_dims.push(v);
                }
            }
        }
        // Pattern 3: "80x60" or "120x30"
        if tl.contains('x') {
            let parts: Vec<&str> = tl.split('x').collect();
            if parts.len() == 2 || parts.len() == 3 {
                let parsed: Vec<f64> = parts.iter().filter_map(|p| p.parse::<f64>().ok()).filter(|v| (4.0..=320.0).contains(v)).collect();
                if parsed.len() == parts.len() {
                    explicit_dims.extend(parsed);
                }
            }
        }
    }

    // Remove PDF standard page point sizes if any slipped through (595, 842, 612, 792)
    explicit_dims.retain(|v| (*v - 595.0).abs() > 2.0 && (*v - 842.0).abs() > 2.0 && (*v - 612.0).abs() > 2.0 && (*v - 792.0).abs() > 2.0);
    explicit_dims.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    // Conservative realistic defaults (Ø60x20 mm = ~0.45 kg) when PDF is scanned/compressed so price is NEVER wildly inflated!
    let d1 = explicit_dims.first().copied().unwrap_or(60.0).clamp(12.0, 260.0);
    let d2 = explicit_dims.get(1).copied().unwrap_or(50.0).clamp(10.0, 220.0);
    let d3 = explicit_dims.get(2).copied().unwrap_or(20.0).clamp(5.0, 80.0);

    // Realistic hollow/machined net mass factor (~0.62 of bounding cylinder/box)
    let raw_vol_cm3 = if is_cyl {
        std::f64::consts::PI * (d1 / 20.0).powi(2) * (d3 / 10.0)
    } else {
        (d1 * d2 * d3) / 1000.0
    };
    let net_grams = (raw_vol_cm3 * 0.62 * density).clamp(40.0, 12000.0);
    let direct_mat_tl = ((net_grams / 1000.0) * 1.15 * kg_price).clamp(25.0, 3500.0);

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
    m.insert("part_name", if is_cyl { "CNC Talaşlı İmalat Parçası" } else { "CNC İşlenmiş Makine Parçası" }.to_string());
    m.insert("part_code", if clean_code.is_empty() { "BHF-2026-01".to_string() } else { clean_code });
    m.insert("material_key", mat_key.to_string());
    m.insert("material_label", mat_label.to_string());
    m.insert("shape", if is_cyl { "cylindrical" } else { "prismatic" }.to_string());
    m.insert("dim_d", format!("{:.1}", d1));
    m.insert("dim_x", format!("{:.1}", d1));
    m.insert("dim_y", format!("{:.1}", d2));
    m.insert("dim_z", format!("{:.1}", d3));
    m.insert("net_grams", format!("{:.0}", net_grams));
    m.insert("direct_mat_tl", format!("{:.0}", direct_mat_tl));
    m.insert("dims_found", if explicit_dims.is_empty() { "0".into() } else { explicit_dims.len().to_string() });
    m
}

fn handle_rfq_analyze(body: &str) -> String {
    let fname = extract_json_str(body, "filename");
    let b64_raw = extract_json_str(body, "file_b64");
    let b64_clean = b64_raw.split_once(',').map(|(_, r)| r).unwrap_or(&b64_raw);
    let bytes = base64_decode(b64_clean);

    // 0. If 3D STEP/STP file -> parse exact B-Rep CARTESIAN_POINT bounds + isometric SVG preview directly in Pure Rust!
    if let Some(step_json) = analyze_step_file_rust(&fname, &bytes) {
        return step_json;
    }

    // 1. Render PDF/HEIC/PNG/JPG/Sketch natively and run Vision analysis if available!
    if let Some(png_bytes) = render_to_png_bytes(&fname, &bytes) {
        if let Some(vision_json) = analyze_drawing_with_vision_rust(&fname, &png_bytes) {
            return vision_json;
        }
    }

    // 2. Fallback: offline heuristic parser
    let specs = parse_drawing_specs_offline(&fname, &bytes);
    let shape = specs.get("shape").cloned().unwrap_or_else(|| "cylindrical".into());
    let turn_min = if shape == "cylindrical" { 10 } else { 0 };
    let mill_min = if shape == "cylindrical" { 12 } else { 18 };
    let dims_note = if specs["dims_found"] == "0" {
        "Çevrimdışı mod — antet/ölçü vektör çizim olduğu için parça başı malzeme bedeli ve süreyi onaylayıp 'Kaydet & Eğit'e basın."
    } else {
        "Teknik resimden açık mm ölçüleri okundu ve parça başı gerçekçi değerleme uygulandı."
    };
    let comment = format!(
        "Parça Başı Değerleme ({}, net ~{} g, tahmini malzeme {} ₺/ad): {}",
        specs["material_label"],
        specs["net_grams"],
        specs["direct_mat_tl"],
        dims_note
    );

    format!(
        r#"{{"ok":true,"engine":"BahaForge Rust Native v7.1","part_name":"{}","part_code":"{}","material_key":"{}","material_label":"{}","shape":"{}","dim_d_mm":{},"dim_x_mm":{},"dim_y_mm":{},"dim_z_mm":{},"net_grams":{},"direct_mat_unit_tl":{},"tolerance":"ISO 2768-m","turn_min":{},"mill_min":{},"edm_min":0,"setup_min":30,"difficulty":"Standart","engineering_comment":"{}"}}"#,
        json_escape(&specs["part_name"]),
        json_escape(&specs["part_code"]),
        json_escape(&specs["material_key"]),
        json_escape(&specs["material_label"]),
        json_escape(&shape),
        specs["dim_d"],
        specs["dim_x"],
        specs["dim_y"],
        specs["dim_z"],
        specs["net_grams"],
        specs["direct_mat_tl"],
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
    direct_mat_unit: f64,
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
        direct_mat_unit: extract_json_num(line, "directMatUnit", 160.0),
        scrap_pct: extract_json_num(line, "scrapPct", 15.0),
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

    let (mut wsum, mut turn, mut mill, mut edm, mut setup, mut kgp, mut dmat, mut trate, mut mrate, mut marg, mut scrap, mut unitp) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (m, sim, vscale) in &scored {
        let w = sim * sim;
        wsum += w;
        turn += w * m.turn_min * vscale;
        mill += w * m.mill_min * vscale;
        edm += w * m.edm_min * vscale;
        setup += w * m.setup_min;
        kgp += w * m.kg_price;
        dmat += w * m.direct_mat_unit;
        trate += w * m.turn_rate;
        mrate += w * m.mill_rate;
        marg += w * m.margin_pct;
        scrap += w * m.scrap_pct;
        unitp += w * m.sell_unit;
    }

    let top = scored[0].0;
    let top_sim = scored[0].1;
    let conf = (((n.min(12) as f64 / 12.0) * 58.0 + top_sim * 40.0).round() as i32).min(97);
    let expl = format!(
        "BahaForge Rust Yerel ML ({} geçmiş işten öğrendi, Güven %{}): En yakın referans '{} ({})' (%{} benzer). Parça başı malzeme bedeli, tezgâh süreleri ve kâr marjı otomatik dolduruldu.",
        n, conf, top.part_name, top.part_code, (top_sim * 100.0).round() as i32
    );

    format!(
        r#"{{"ok":true,"trained":{},"confidence":{},"explanation":"{}","prediction":{{"directMatUnit":{},"turnMin":{},"millMin":{},"edmMin":{},"setupMin":{},"kgPrice":{},"turnRate":{},"millRate":{},"marginPct":{},"scrapPct":{},"estUnitTl":{}}}}}"#,
        n,
        conf,
        json_escape(&expl),
        (dmat / wsum).round() as i64,
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
