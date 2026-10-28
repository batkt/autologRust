mod claude;
mod report;
mod store;

use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, SecondsFormat, TimeZone, Utc};
use report::{markdown, tailan_beltgey, tailan_serverees, ub, Bosgo};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use store::{tasaldal_shalgay, LogStore, State, TailanBichlega};
use tokio::{sync::oneshot, time::sleep};
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

const SERVICE_NAME: &str = "AutologKhyanalt";
const SERVICE_DISPLAY_NAME: &str = "zevTurees serverKhyanaltLog Autolog";
/// Backend-ийн /serverKhyanalt endpoint-ийн .limit(200)
const KHYANALT_LIMIT: usize = 200;
/// models/serverKhyanaltLog.js-д тодорхойлсон төрлүүд (REQUEST_SLOW-оос бусад)
const TURLUUD: [&str; 9] = [
    "CPU",
    "PROCESS_CPU",
    "GATSALT",
    "KHARILTSAGCH",
    "INSTANCE_GATSSAN",
    "BROWSER_GATSALT",
    "DB_UDAAN",
    "WATCHDOG",
    "MONGO_DISK",
];

// ─── Тохиргоо ─────────────────────────────────────────────────────────────────

// Debug derive хийхгүй — token, нууц үг log-д гарахаас сэргийлнэ.
#[derive(Deserialize, Clone)]
struct Config {
    #[serde(default = "default_url")]
    url: String,
    #[serde(default)]
    token: String,
    #[serde(default, rename = "nevtrekhNer")]
    nevtrekh_ner: String,
    #[serde(default, rename = "nuutsUg")]
    nuuts_ug: String,
    #[serde(default, rename = "baiguullagiinId")]
    baiguullagiin_id: String,
    #[serde(default = "default_true")]
    requests_tatakh: bool,
    /// /serverKhyanalt/tailan?bukhHost=1 — нэг DB-д бичдэг бүх серверийн (turees, rently) log
    #[serde(default)]
    bukh_host: bool,
    #[serde(default = "default_requests_limit")]
    requests_limit: usize,
    #[serde(default = "default_tailan_tsag")]
    tailan_tsag: String,
    #[serde(default = "default_udaan_bosgo")]
    udaan_bosgo_ms: f64,
    #[serde(default = "default_request_log_bosgo")]
    request_log_bosgo_ms: f64,
    #[serde(default = "default_khadgalakh_khonog")]
    khadgalakh_khonog: i64,
    #[serde(default = "default_timeout")]
    timeout_secs: u64,
    /// Өдрийн тайланд P1/P2 байвал Claude Code-оор (claude -p) засвар хийлгэх
    #[serde(default)]
    claude_ajilluulakh: bool,
    #[serde(default = "default_claude_exe")]
    claude_exe: String,
    #[serde(default = "default_claude_repos")]
    claude_repos: Vec<String>,
    #[serde(default = "default_claude_budget")]
    claude_budget_usd: f64,
    #[serde(default = "default_claude_timeout")]
    claude_timeout_min: u64,
    #[serde(default)]
    claude_model: String,
}

fn default_claude_exe() -> String {
    "claude".into()
}
fn default_claude_repos() -> Vec<String> {
    ["D:/Work/tureesBack", "D:/Work/turees", "D:/Work/parking-v2"]
        .map(String::from)
        .to_vec()
}
fn default_claude_budget() -> f64 {
    5.0
}
fn default_claude_timeout() -> u64 {
    60
}

fn default_url() -> String {
    "https://turees.zevtabs.mn/api".into()
}
fn default_true() -> bool {
    true
}
fn default_requests_limit() -> usize {
    100
}
fn default_tailan_tsag() -> String {
    "21:00".into()
}
fn default_udaan_bosgo() -> f64 {
    5000.0
}
fn default_request_log_bosgo() -> f64 {
    1000.0
}
fn default_khadgalakh_khonog() -> i64 {
    30
}
fn default_timeout() -> u64 {
    20
}

impl Config {
    fn bosgo(&self) -> Bosgo {
        Bosgo {
            udaan_ms: self.udaan_bosgo_ms,
            request_log_ms: self.request_log_bosgo_ms,
        }
    }

    fn claude(&self) -> claude::ClaudeTokhirgoo {
        claude::ClaudeTokhirgoo {
            exe: self.claude_exe.clone(),
            repos: self.claude_repos.clone(),
            budget_usd: self.claude_budget_usd,
            timeout_min: self.claude_timeout_min.max(1),
            model: self.claude_model.clone(),
        }
    }

    fn tailan_tsag(&self) -> NaiveTime {
        NaiveTime::parse_from_str(&self.tailan_tsag, "%H:%M")
            .unwrap_or_else(|_| NaiveTime::from_hms_opt(21, 0, 0).unwrap())
    }
}

// ─── Log ──────────────────────────────────────────────────────────────────────

type LogFile = Arc<Mutex<Box<dyn Write + Send>>>;

fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn open_log(dir: &Path) -> LogFile {
    let log_path = dir.join("autolog.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("Log файл нээхэд алдаа");
    Arc::new(Mutex::new(Box::new(file)))
}

fn log_write(log: &LogFile, level: &str, msg: &str) {
    let line = format!(
        "[{}] [{}] {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        msg
    );
    print!("{}", line);
    if let Ok(mut f) = log.lock() {
        let _ = f.write_all(line.as_bytes());
    }
}

/// exe-ийн хажуудах config.toml, байхгүй бол одоогийн хавтасных (cargo run).
/// Буцаах хавтас нь log, data, reports хадгалах үндсэн хавтас болно.
fn load_config() -> (Config, PathBuf) {
    let exe_dir = get_exe_dir();
    let candidates = [exe_dir.clone(), PathBuf::from(".")];
    let Some(dir) = candidates.iter().find(|d| d.join("config.toml").is_file()) else {
        eprintln!("config.toml олдсонгүй: {:?}", exe_dir.join("config.toml"));
        std::process::exit(1);
    };
    let config_str = fs::read_to_string(dir.join("config.toml")).unwrap_or_else(|e| {
        eprintln!("config.toml уншихад алдаа: {}", e);
        std::process::exit(1);
    });
    let mut config: Config = toml::from_str(&config_str).unwrap_or_else(|e| {
        eprintln!("config.toml алдаа: {}", e);
        std::process::exit(1);
    });
    config.url = config.url.trim_end_matches('/').to_string();
    (config, dir.clone())
}

// ─── HTTP ─────────────────────────────────────────────────────────────────────

enum ApiAldaa {
    /// 401 — token дууссан эсвэл буруу
    Erkhgui,
    Busad(String),
}

struct Api {
    client: Client,
    config: Config,
    token: Option<String>,
}

fn khariu_tovch(body: &str) -> String {
    body.chars().take(300).collect()
}

impl Api {
    fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("autolog/0.1")
            .build()
            .expect("HTTP client үүсгэхэд алдаа");
        let token = Some(config.token.trim().to_string()).filter(|t| !t.is_empty());
        Self { client, config, token }
    }

    fn nevtrekh_bolomjtoi(&self) -> bool {
        !self.config.nevtrekh_ner.is_empty() && !self.config.nuuts_ug.is_empty()
    }

    async fn nevtrey(&mut self) -> Result<(), String> {
        if !self.nevtrekh_bolomjtoi() {
            return Err("token хүчингүй/байхгүй, config.toml-д nevtrekhNer, nuutsUg тохируулаагүй".into());
        }
        let mut body = json!({
            "nevtrekhNer": self.config.nevtrekh_ner,
            "nuutsUg": self.config.nuuts_ug,
        });
        if !self.config.baiguullagiin_id.is_empty() {
            body["baiguullagiinId"] = json!(self.config.baiguullagiin_id);
        }
        let resp = self
            .client
            .post(format!("{}/ajiltanNevtrey", self.config.url))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Нэвтрэх хүсэлт: {e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!("Нэвтрэх HTTP {}: {}", status.as_u16(), khariu_tovch(&text)));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("Нэвтрэх хариу: {e}"))?;
        if v.get("multipleOrgs").and_then(Value::as_bool) == Some(true) {
            return Err("Олон байгууллагад бүртгэлтэй — config.toml-д baiguullagiinId тохируулна уу".into());
        }
        let token = v
            .get("token")
            .and_then(Value::as_str)
            .ok_or("Нэвтрэх хариунд token алга")?;
        self.token = Some(token.to_string());
        Ok(())
    }

    async fn get_once(&self, zam: &str) -> Result<Value, ApiAldaa> {
        let token = self.token.as_deref().unwrap_or_default();
        let resp = self
            .client
            .get(format!("{}{}", self.config.url, zam))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| ApiAldaa::Busad(e.to_string()))?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(ApiAldaa::Erkhgui);
        }
        let text = resp.text().await.map_err(|e| ApiAldaa::Busad(e.to_string()))?;
        if status == StatusCode::FORBIDDEN {
            return Err(ApiAldaa::Busad("HTTP 403 — Zevtabs CAdmin1 эрх шаардлагатай".into()));
        }
        if !status.is_success() {
            return Err(ApiAldaa::Busad(format!("HTTP {}: {}", status.as_u16(), khariu_tovch(&text))));
        }
        serde_json::from_str(&text).map_err(|e| ApiAldaa::Busad(format!("JSON: {e}")))
    }

    async fn get_json(&mut self, zam: &str) -> Result<Value, String> {
        if self.token.is_none() {
            self.nevtrey().await?;
        }
        match self.get_once(zam).await {
            Ok(v) => Ok(v),
            Err(ApiAldaa::Erkhgui) => {
                self.token = None;
                self.nevtrey().await?;
                self.get_once(zam).await.map_err(|e| match e {
                    ApiAldaa::Erkhgui => "HTTP 401 — шинэ token-оор ч эрхгүй".into(),
                    ApiAldaa::Busad(s) => s,
                })
            }
            Err(ApiAldaa::Busad(s)) => Err(s),
        }
    }
}

// ─── Цуглуулах үндсэн логик ───────────────────────────────────────────────────

struct App {
    api: Api,
    config: Config,
    store: LogStore,
    state: State,
    data_dir: PathBuf,
    reports_dir: PathBuf,
    log: LogFile,
}

impl App {
    fn new(config: Config, base_dir: &Path, log: LogFile) -> Self {
        let data_dir = base_dir.join("data");
        let reports_dir = base_dir.join("reports");
        let _ = fs::create_dir_all(&data_dir);
        let _ = fs::create_dir_all(&reports_dir);
        let store = LogStore::load(data_dir.join("logs.jsonl"));
        let state = State::load(&data_dir.join("state.json"));
        Self {
            api: Api::new(config.clone()),
            config,
            store,
            state,
            data_dir,
            reports_dir,
            log,
        }
    }

    fn state_khadgal(&self) {
        if let Err(e) = self.state.save(&self.data_dir.join("state.json")) {
            log_write(&self.log, "ERROR", &format!("state.json хадгалахад алдаа: {e}"));
        }
    }

    fn tsevelrey(&mut self) {
        let cutoff = Utc::now() - ChronoDuration::days(self.config.khadgalakh_khonog);
        match self.store.prune(cutoff) {
            Ok(n) if n > 0 => log_write(&self.log, "INFO", &format!("{n} хуучин бичлэг устгалаа")),
            Ok(_) => {}
            Err(e) => log_write(&self.log, "ERROR", &format!("Хуучин бичлэг цэвэрлэх: {e}")),
        }
        self.state.tasaldluud.retain(|g| g.tugsgul >= cutoff);
        self.state.tailanguud.retain(|t| t.tugsgul >= cutoff);
    }

    /// Бүх эх сурвалжийг нэг удаа татна. /serverKhyanalt нь хамгийн ихдээ 200
    /// бичлэг буцаадаг тул төрөл бүрээр тусад нь татаж хамрах хүрээг нэмэгдүүлнэ.
    /// Дүүрэн ирсэн багц ekhlel-ийг хамраагүй бол дутуу хэсгийг tasaldal болгоно.
    /// Нэг ч эх сурвалж амжилттай татагдаагүй бол Err.
    async fn tatay(&mut self, ekhlel: DateTime<Utc>) -> Result<Option<Value>, String> {
        self.nevtersen().await?;
        let mut aldaanuud = Vec::new();
        let mut shine = 0;
        let mut amjilttai = 0;
        // (нэр, зам, limit, дутууг шалгах эсэх, нөхөх turul)
        let mut survaljuud = vec![(
            "serverKhyanalt".to_string(),
            "/serverKhyanalt".to_string(),
            KHYANALT_LIMIT,
            false,
            None,
        )];
        for turul in TURLUUD {
            survaljuud.push((
                format!("serverKhyanalt?turul={turul}"),
                format!("/serverKhyanalt?turul={turul}"),
                KHYANALT_LIMIT,
                true,
                None,
            ));
        }
        if self.config.requests_tatakh {
            survaljuud.push((
                "requests".to_string(),
                format!("/serverKhyanalt/requests?limit={}", self.config.requests_limit),
                self.config.requests_limit,
                true,
                // /requests нь turul талбарыг select хийдэггүй
                Some("REQUEST_SLOW"),
            ));
        }

        for (ner, zam, limit, shalgakh, turul) in survaljuud {
            match self.api.get_json(&zam).await {
                Ok(Value::Array(mut items)) => {
                    amjilttai += 1;
                    if let Some(turul) = turul {
                        for it in items.iter_mut().filter(|it| it.get("turul").is_none()) {
                            it["turul"] = json!(turul);
                        }
                    }
                    if shalgakh {
                        if let Some(g) = tasaldal_shalgay(&ner, &items, limit, Some(ekhlel)) {
                            log_write(
                                &self.log,
                                "WARN",
                                &format!(
                                    "{ner}: {} → {} (УБ) хооронд хамрах хүрээ дутуу (limit {limit} хүрсэн)",
                                    report::ub_tsag(&g.ekhlel),
                                    report::ub_tsag(&g.tugsgul)
                                ),
                            );
                            self.state.tasaldluud.push(g);
                        }
                    }
                    match self.store.merge(items) {
                        Ok(n) => shine += n,
                        Err(e) => aldaanuud.push(format!("{zam}: хадгалах {e}")),
                    }
                }
                Ok(_) => aldaanuud.push(format!("{zam}: массив биш хариу")),
                Err(e) => aldaanuud.push(format!("{zam}: {e}")),
            }
        }

        let odoo = self.odoo_tatay().await.map_err(|e| aldaanuud.push(e)).ok();

        self.state.suuliin_tatalt = Some(Utc::now());
        if aldaanuud.is_empty() {
            self.state.suuliin_aldaa = None;
            log_write(
                &self.log,
                "INFO",
                &format!(
                    "Татлаа: local санд шинээр нэмэгдсэн {shine} (давхардлыг хассан), local санд нийт {} бичлэг",
                    self.store.len()
                ),
            );
        } else {
            let msg = aldaanuud.join("; ");
            log_write(&self.log, "ERROR", &format!("Татахад алдаа: {msg} (шинэ {shine})"));
            self.state.suuliin_aldaa = Some(msg.clone());
            if amjilttai == 0 {
                self.state_khadgal();
                return Err(msg);
            }
        }
        self.state_khadgal();
        Ok(odoo)
    }

    async fn nevtersen(&mut self) -> Result<(), String> {
        if self.api.token.is_some() {
            return Ok(());
        }
        self.api.nevtrey().await.map_err(|e| {
            log_write(&self.log, "ERROR", &format!("Нэвтэрч чадсангүй: {e}"));
            self.state.suuliin_aldaa = Some(e.clone());
            self.state_khadgal();
            e
        })
    }

    /// /serverKhyanalt/odoo — тухайн агшны төлөв, data/odoo.json-д хадгална.
    async fn odoo_tatay(&mut self) -> Result<Value, String> {
        let v = self
            .api
            .get_json("/serverKhyanalt/odoo")
            .await
            .map_err(|e| format!("/serverKhyanalt/odoo: {e}"))?;
        let text = serde_json::to_string_pretty(&v).unwrap_or_default();
        fs::write(self.data_dir.join("odoo.json"), text).map_err(|e| format!("odoo.json: {e}"))?;
        Ok(v)
    }

    /// /serverKhyanalt/tailan — цонхны бүх log-ийг сервер талд нэгтгэсэн хариу.
    async fn server_tailan(&mut self, ekhlel: DateTime<Utc>, tugsgul: DateTime<Utc>) -> Result<Value, String> {
        let iso = |t: DateTime<Utc>| t.to_rfc3339_opts(SecondsFormat::Secs, true);
        let mut zam = format!("/serverKhyanalt/tailan?ekhlel={}&tugsgul={}", iso(ekhlel), iso(tugsgul));
        if self.config.bukh_host {
            zam.push_str("&bukhHost=1");
        }
        let v = self.api.get_json(&zam).await?;
        if v.get("bulgemuud").and_then(Value::as_array).is_none() {
            return Err("bulgemuud массив алга".into());
        }
        Ok(v)
    }

    /// ekhlel..tugsgul цонхны өгөгдлийг татаж тайланг reports/<ner>.md, .json болгон бичнэ.
    /// Эхлээд /serverKhyanalt/tailan (limit-гүй), backend-д байхгүй бол хуучин endpoint-ууд.
    async fn tailan_bichiy(&mut self, ner: &str, ognoo: &str, ekhlel: DateTime<Utc>, tugsgul: DateTime<Utc>) -> Option<TailanUr> {
        // Юу ч татаагүй бол хуурамч "0 бичлэг" тайлан гаргахгүй — дараагийн шалгалтаар дахин оролдоно
        self.nevtersen().await.ok()?;
        let tailan = match self.server_tailan(ekhlel, tugsgul).await {
            Ok(v) => {
                let odoo = self.odoo_tatay().await;
                self.state.suuliin_tatalt = Some(Utc::now());
                self.state.suuliin_aldaa = odoo.as_ref().err().cloned();
                if let Err(e) = &odoo {
                    log_write(&self.log, "ERROR", e);
                }
                self.state_khadgal();
                tailan_serverees(&v, ognoo, ekhlel, tugsgul, &self.config.bosgo(), odoo.ok().as_ref())
            }
            Err(e) => {
                log_write(
                    &self.log,
                    "WARN",
                    &format!("/serverKhyanalt/tailan ашиглах боломжгүй ({e}) — хуучин endpoint-уудаар татна"),
                );
                let odoo = self.tatay(ekhlel).await.ok()?;
                tailan_beltgey(
                    self.store.values(),
                    ognoo,
                    ekhlel,
                    tugsgul,
                    &self.config.bosgo(),
                    &self.state.tasaldluud,
                    self.store.ekhnii(),
                    odoo.as_ref(),
                )
            }
        };
        let md = markdown(&tailan);
        let md_path = self.reports_dir.join(format!("{ner}.md"));
        let json_path = self.reports_dir.join(format!("{ner}.json"));
        let bichsen = fs::write(&md_path, &md)
            .and_then(|_| fs::write(&json_path, serde_json::to_string_pretty(&tailan).unwrap_or_default()));
        match bichsen {
            Ok(_) => {
                log_write(&self.log, "INFO", &format!("Тайлан: {} ({} бичлэг)", md_path.display(), tailan.niit));
                Some(TailanUr { niit: tailan.niit, md })
            }
            Err(e) => {
                log_write(&self.log, "ERROR", &format!("Тайлан бичихэд алдаа: {e}"));
                None
            }
        }
    }

    /// Тухайн өдрийн tailan_tsag (УБ цаг) — UTC-ээр.
    fn tailan_tsag_utc(&self, odoo: DateTime<Utc>) -> DateTime<Utc> {
        self.ognoonii_tailan_tsag(odoo.with_timezone(&ub()).date_naive())
    }

    fn ognoonii_tailan_tsag(&self, ognoo: NaiveDate) -> DateTime<Utc> {
        ub().from_local_datetime(&ognoo.and_time(self.config.tailan_tsag()))
            .single()
            .expect("UTC+8 тогтмол offset")
            .with_timezone(&Utc)
    }

    /// Дараагийн тайлан хүртэл хүлээх хугацаа.
    fn daraagiin_khuleelt(&self) -> Duration {
        let odoo = Utc::now();
        let mut daraagiin = self.tailan_tsag_utc(odoo);
        if daraagiin <= odoo {
            daraagiin += ChronoDuration::days(1);
        }
        (daraagiin - odoo).to_std().unwrap_or_default() + Duration::from_secs(5)
    }

    /// Нэг процесс өдөрт: tailan_tsag өнгөрмөгц (1) өмнөх 24 цагийн тайланг нэг
    /// удаа гаргана, (2) claude_ajilluulakh бол тэр тайлангаар Claude-г нэг удаа
    /// ажиллуулна. Аль нэг нь өнөөдөр хийгдсэн бол алгасна.
    async fn tailan_shalgay(&mut self) {
        let odoo = Utc::now();
        let tugsgul = self.tailan_tsag_utc(odoo);
        if odoo < tugsgul {
            return;
        }
        let ognoo = odoo.with_timezone(&ub()).format("%Y-%m-%d").to_string();
        self.ognoogoor(&ognoo, tugsgul, false).await;
    }

    /// `tugsgul` хүртэлх 24 цагийн тайлан, дараа нь Claude. `albadakh` (гараар
    /// `autolog.exe <огноо>`) бол state-д тэмдэглэгдсэн ч хоёуланг нь дахин хийнэ.
    async fn ognoogoor(&mut self, ognoo: &str, tugsgul: DateTime<Utc>, albadakh: bool) {
        if !albadakh && self.state.tailanguud.iter().any(|t| t.ognoo == ognoo) {
            log_write(&self.log, "INFO", &format!("{ognoo}-ий тайлан аль хэдийн гарсан"));
        } else {
            let ekhlel = tugsgul - ChronoDuration::hours(24);
            let Some(ur) = self.tailan_bichiy(ognoo, ognoo, ekhlel, tugsgul).await else {
                return;
            };
            self.state.tailanguud.retain(|t| t.ognoo != ognoo);
            self.state.tailanguud.push(TailanBichlega {
                ognoo: ognoo.to_string(),
                ekhlel,
                tugsgul,
                niit: ur.niit,
                uusgesen: Utc::now(),
            });
            self.tsevelrey();
            self.state_khadgal();
        }

        let shalgasan = self.state.claude_shalgasan.iter().any(|o| o == ognoo);
        if !self.config.claude_ajilluulakh {
            if albadakh {
                log_write(&self.log, "INFO", "claude_ajilluulakh = false — Claude ажиллуулсангүй");
            }
            return;
        }
        if shalgasan && !albadakh {
            return;
        }
        let md = self.reports_dir.join(format!("{ognoo}.md"));
        let json = self.reports_dir.join(format!("{ognoo}.json"));
        // Амжилттай эсэхээс үл хамааран өдөрт нэг л удаа — давтан зардал гаргахгүй.
        // Дахин оролдох бол `autolog.exe <огноо>` эсвэл `autolog.exe claude <огноо>`.
        if !shalgasan {
            self.state.claude_shalgasan.push(ognoo.to_string());
            self.state_khadgal();
        }
        match anhaarakh_too(&json) {
            Some(0) => log_write(&self.log, "INFO", "P1/P2 асуудал алга — Claude ажиллуулсангүй"),
            Some(_) => self.claude_ajilluul(ognoo, &md, &json).await,
            None => log_write(&self.log, "ERROR", &format!("{} уншиж чадсангүй", json.display())),
        }
    }

    async fn claude_ajilluul(&self, ognoo: &str, md: &Path, json: &Path) {
        log_write(&self.log, "INFO", &format!("Claude шалгалт эхэллээ: {}", md.display()));
        let yavts = |m: &str| log_write(&self.log, "CLAUDE", m);
        match claude::ajilluulya(&self.config.claude(), ognoo, md, json, &self.reports_dir, yavts).await {
            Ok(ur) => {
                log_write(
                    &self.log,
                    "INFO",
                    &format!(
                        "Claude дууслаа: {} · зардал {} · өөрчилсөн файл {} · алгассан repo: {}",
                        ur.garalt.display(),
                        ur.zardal_usd.map(|z| format!("${z:.2}")).unwrap_or_else(|| "-".into()),
                        ur.zasvarlasan.len(),
                        if ur.algassan.is_empty() { "-".to_string() } else { ur.algassan.join(", ") }
                    ),
                );
            }
            Err(e) => log_write(&self.log, "ERROR", &format!("Claude: {e}")),
        }
    }
}

struct TailanUr {
    niit: usize,
    md: String,
}

/// Тайлангийн JSON дахь P1 + P2 бүлгийн тоо.
fn anhaarakh_too(json: &Path) -> Option<usize> {
    let v: Value = serde_json::from_str(&fs::read_to_string(json).ok()?).ok()?;
    let bulgemuud = v.get("bulgemuud")?.as_array()?;
    Some(
        bulgemuud
            .iter()
            .filter(|b| b.get("ach").and_then(Value::as_u64).is_some_and(|a| a <= 2))
            .count(),
    )
}

/// `autolog.exe claude [YYYY-MM-DD]` — бэлэн тайлангаар Claude-г гараар ажиллуулна.
async fn run_claude(config: Config, base_dir: PathBuf, log: LogFile, ognoo: Option<String>) {
    let app = App::new(config, &base_dir, log.clone());
    let ognoo = ognoo.unwrap_or_else(|| Utc::now().with_timezone(&ub()).format("%Y-%m-%d").to_string());
    let md = app.reports_dir.join(format!("{ognoo}.md"));
    let json = app.reports_dir.join(format!("{ognoo}.json"));
    if !md.is_file() {
        log_write(&log, "ERROR", &format!("Тайлан олдсонгүй: {}", md.display()));
        return;
    }
    app.claude_ajilluul(&ognoo, &md, &json).await;
}

/// `autolog.exe [dakhin] YYYY-MM-DD` — тухайн өдрийн тайланг (tailan_tsag хүртэлх
/// 24 цаг; өнөөдөр бөгөөд цаг болоогүй бол одоо хүртэл) state-ээс үл хамааран
/// дахин гаргаж, claude_ajilluulakh бол Claude-г дахин ажиллуулна.
async fn run_dakhin(config: Config, base_dir: PathBuf, log: LogFile, ognoo: NaiveDate) {
    let mut app = App::new(config, &base_dir, log.clone());
    let odoo = Utc::now();
    if ognoo > odoo.with_timezone(&ub()).date_naive() {
        log_write(&log, "ERROR", &format!("{ognoo} ирээдүйн огноо байна"));
        return;
    }
    let tugsgul = app.ognoonii_tailan_tsag(ognoo).min(odoo);
    log_write(&log, "INFO", &format!("{ognoo}-ий тайланг гараар дахин гаргаж байна"));
    app.ognoogoor(&ognoo.format("%Y-%m-%d").to_string(), tugsgul, true).await;
}

async fn run_autolog(config: Config, base_dir: PathBuf, log: LogFile, mut shutdown_rx: oneshot::Receiver<()>) {
    let mut app = App::new(config, &base_dir, log.clone());
    app.tsevelrey();
    log_write(
        &log,
        "INFO",
        &format!(
            "Autolog эхэллээ: {} (өдөр бүр {} УБ цагт), local {} бичлэг",
            app.config.url,
            app.config.tailan_tsag,
            app.store.len()
        ),
    );
    loop {
        app.tailan_shalgay().await;
        let khuleelt = app.daraagiin_khuleelt();
        log_write(&log, "INFO", &format!("Дараагийн шалгалт {} минутын дараа", khuleelt.as_secs() / 60));
        // Компьютер унтсан, цаг өөрчлөгдсөн тохиолдолд цагт нэг удаа дахин тооцно
        tokio::select! {
            _ = &mut shutdown_rx => break,
            _ = sleep(khuleelt.min(Duration::from_secs(3600))) => {}
        }
    }
    log_write(&log, "INFO", "Autolog зогслоо");
}

/// `autolog.exe tailan [tsag]` — нэг удаа татаж, сүүлийн N цагийн тайланг гаргана.
async fn run_tailan(config: Config, base_dir: PathBuf, log: LogFile, tsag: i64) {
    let mut app = App::new(config, &base_dir, log);
    let tugsgul = Utc::now();
    let ner = format!("adhoc-{}", tugsgul.with_timezone(&ub()).format("%Y%m%d-%H%M"));
    let ognoo = tugsgul.with_timezone(&ub()).format("%Y-%m-%d").to_string();
    if let Some(ur) = app.tailan_bichiy(&ner, &ognoo, tugsgul - ChronoDuration::hours(tsag), tugsgul).await {
        println!("\n{}", ur.md);
    }
}

fn console_run(f: impl FnOnce(Config, PathBuf, LogFile) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>) {
    let (config, base_dir) = load_config();
    let log = open_log(&base_dir);
    tokio::runtime::Runtime::new().unwrap().block_on(f(config, base_dir, log));
}

// ─── Windows Service ──────────────────────────────────────────────────────────

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<OsString>) {
    if let Err(e) = run_as_service() {
        let log = open_log(&get_exe_dir());
        log_write(&log, "ERROR", &format!("Service алдаатай зогслоо: {:?}", e));
    }
}

fn run_as_service() -> windows_service::Result<()> {
    let (config, base_dir) = load_config();
    let log = open_log(&base_dir);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let shutdown_tx = Arc::new(Mutex::new(Some(shutdown_tx)));

    let status_handle = service_control_handler::register(SERVICE_NAME, {
        let tx = Arc::clone(&shutdown_tx);
        move |event| match event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                if let Ok(mut guard) = tx.lock() {
                    if let Some(sender) = guard.take() {
                        let _ = sender.send(());
                    }
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(run_autolog(config, base_dir, log.clone(), shutdown_rx));

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}

// ─── Service install / uninstall ─────────────────────────────────────────────

fn install_service() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
    let exe_path = std::env::current_exe().unwrap();

    let _svc = manager.create_service(
        &ServiceInfo {
            name: OsString::from(SERVICE_NAME),
            display_name: OsString::from(SERVICE_DISPLAY_NAME),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe_path,
            launch_arguments: vec![],
            dependencies: vec![],
            account_name: None, // LocalSystem
            account_password: None,
        },
        ServiceAccess::empty(),
    )?;

    println!("✓ Service суулгагдлаа: {}", SERVICE_NAME);
    println!("  Эхлүүлэх: sc start {}", SERVICE_NAME);
    println!("  Зогсоох : sc stop  {}", SERVICE_NAME);
    Ok(())
}

fn uninstall_service() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let svc = manager.open_service(SERVICE_NAME, ServiceAccess::DELETE | ServiceAccess::STOP)?;
    svc.delete()?;
    println!("✓ Service устгагдлаа: {}", SERVICE_NAME);
    Ok(())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("install") => {
            install_service().unwrap_or_else(|e| eprintln!("Суулгахад алдаа: {:?}", e));
        }
        Some("uninstall") => {
            uninstall_service().unwrap_or_else(|e| eprintln!("Устгахад алдаа: {:?}", e));
        }
        Some("run") => {
            // Console горим — гараар туршихад
            console_run(|c, d, l| {
                let (_tx, rx) = oneshot::channel::<()>();
                Box::pin(async move {
                    let _tx = _tx;
                    run_autolog(c, d, l, rx).await
                })
            });
        }
        Some("tailan") => {
            let tsag = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(24);
            console_run(move |c, d, l| Box::pin(run_tailan(c, d, l, tsag)));
        }
        Some("claude") => {
            let ognoo = args.get(2).cloned();
            console_run(move |c, d, l| Box::pin(run_claude(c, d, l, ognoo)));
        }
        Some("dakhin") => {
            let odoo = Utc::now().with_timezone(&ub()).date_naive();
            match args.get(2).map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d")) {
                None => console_run(move |c, d, l| Box::pin(run_dakhin(c, d, l, odoo))),
                Some(Ok(ognoo)) => console_run(move |c, d, l| Box::pin(run_dakhin(c, d, l, ognoo))),
                Some(Err(_)) => eprintln!("Огноо YYYY-MM-DD хэлбэртэй байна"),
            }
        }
        Some(a) if NaiveDate::parse_from_str(a, "%Y-%m-%d").is_ok() => {
            let ognoo = NaiveDate::parse_from_str(a, "%Y-%m-%d").unwrap();
            console_run(move |c, d, l| Box::pin(run_dakhin(c, d, l, ognoo)));
        }
        Some("help") | Some("--help") | Some("-h") => {
            println!(
                "Ашиглалт:\n  autolog.exe run          - console горимд ажиллуулна\n  autolog.exe [dakhin] [YYYY-MM-DD] - тухайн өдрийн тайлан (+ Claude)-г дахин гаргана\n  autolog.exe tailan [24]  - нэг удаа татаж, сүүлийн N цагийн тайлан гаргана\n  autolog.exe claude [YYYY-MM-DD] - reports\\<огноо>.md тайлангаар Claude-д засвар хийлгэнэ\n  autolog.exe install      - Windows Service болгож бүртгэнэ (Administrator)\n  autolog.exe uninstall    - Windows Service-г устгана (Administrator)"
            );
        }
        _ => {
            // SCM-ээс дуудагдах үед — service dispatcher эхлүүлнэ
            service_dispatcher::start(SERVICE_NAME, ffi_service_main).unwrap_or_else(|_| {
                // SCM байхгүй (гараар ажиллуулсан) бол console горимд орно
                console_run(|c, d, l| {
                    let (_tx, rx) = oneshot::channel::<()>();
                    Box::pin(async move {
                        let _tx = _tx;
                        run_autolog(c, d, l, rx).await
                    })
                });
            });
        }
    }
}
