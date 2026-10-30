// Өдрийн тайлан: turul / request.route / request.method / request.status-аар
// бүлэглэж тоо, дундаж/max khugatsaaMs, сүүлд гарсан цагийг гаргана.
// Тайланд зөвхөн нэгтгэсэн утга орно — tailbar, байгууллагын нэр, IP орохгүй.

use crate::store::{created_at, Tasaldal};
use chrono::{DateTime, FixedOffset, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

pub fn ub() -> FixedOffset {
    // Asia/Ulaanbaatar — 2017 оноос хойш зуны цаг хэрэглэдэггүй, тогтмол UTC+8
    FixedOffset::east_opt(8 * 3600).unwrap()
}

pub fn ub_tsag(dt: &DateTime<Utc>) -> String {
    dt.with_timezone(&ub()).format("%Y-%m-%d %H:%M:%S").to_string()
}

pub struct Bosgo {
    pub udaan_ms: f64,
    pub request_log_ms: f64,
}

#[derive(Serialize)]
pub struct Bulgem {
    /// 1 — яаралтай, 2 — анхаарах, 3 — мэдээллийн
    pub ach: u8,
    pub shaltgaan: String,
    pub turul: String,
    pub method: String,
    pub route: String,
    pub status: Option<i64>,
    pub host: String,
    pub too: usize,
    pub dundaj_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub event_loop_max_ms: Option<f64>,
    pub tasarsan_too: usize,
    pub suuliin: String,
    /// aldaaBarigch-ийн тэмдэглэсэн алдааны нэр (Error, MongoServerError, …)
    pub aldaa_ner: Option<String>,
    /// Алдаа гарсан repo доторх мөр (controller/ajiltan.js:120)
    pub bairshil: Option<String>,
    /// BROWSER_GATSALT-ийн frontend хуудас (query-гүй)
    pub khuudas: Option<String>,
    /// BROWSER_GATSALT-ийн klient.turul — CRASH бол tab унасан, бусад нь хөлдсөн
    pub klient_turul: Option<String>,
    /// Гацалтын үеийн бүх instance-ийн heartbeat-ийн дээд утга
    pub server_baidal: Option<ServerBaidal>,
    /// VALIDATION (4xx) | SYSTEM (5xx)
    pub angilal: Option<String>,
    /// Минутын хязгаараас болж мөр нь хадгалагдаагүй ч REQUEST_STAT-аар тоологдсон
    pub daalgasan_too: usize,
    /// Хүсэлт бүрийн гадаад API (QPay, eBarimt, SMS…)-д зарцуулсан хугацааны дундаж
    pub dundaj_gadaad_ms: Option<f64>,
    pub max_gadaad_ms: Option<f64>,
    /// Хүсэлт бүрийн Mongo query-үүдийн нийлбэр хугацааны дундаж
    pub dundaj_db_ms: Option<f64>,
    pub gadaad_hostuud: Vec<String>,
    pub jishee: Jishee,
}

/// Бүлгийн жишээ — Claude эх кодоос мөшгих эхлэл
#[derive(Serialize, Default)]
pub struct Jishee {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub request_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stack: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skriptuud: Vec<Value>,
    /// Browser гацахаас өмнөх API дуудлагууд (сүүлийнх нь эхэнд), url нь zam_zagvar-аар
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suuliin_api: Vec<Value>,
}

/// BROWSER_GATSALT бичигдэх агшны серверийн байдал (busadInstance)
#[derive(Serialize, Default, Clone, Copy, PartialEq, Debug)]
pub struct ServerBaidal {
    pub process_cpu_max: Option<f64>,
    pub system_cpu_max: Option<f64>,
    pub event_loop_max_ms: Option<f64>,
}

impl ServerBaidal {
    fn nemey(&mut self, o: ServerBaidal) {
        let m = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        self.process_cpu_max = m(self.process_cpu_max, o.process_cpu_max);
        self.system_cpu_max = m(self.system_cpu_max, o.system_cpu_max);
        self.event_loop_max_ms = m(self.event_loop_max_ms, o.event_loop_max_ms);
    }

    /// Серверийн талд ачаалал/хөлдөлт байгаагүй — гацалт browser-ийнх
    fn kheviin(&self) -> bool {
        self.process_cpu_max.is_none_or(|c| c < 80.0)
            && self.system_cpu_max.is_none_or(|c| c < 80.0)
            && self.event_loop_max_ms.is_none_or(|e| e < 500.0)
    }
}

#[derive(Serialize)]
pub struct Tailan {
    pub ognoo: String,
    pub ekhlel: String,
    pub tugsgul: String,
    pub niit: usize,
    pub turluud: BTreeMap<String, usize>,
    pub bulgemuud: Vec<Bulgem>,
    pub tasaldluud: Vec<Tasaldal>,
    /// /serverKhyanalt/odoo — тайлан гаргах агшны instance-ийн санах ой дахь төлөв
    pub odoo: Option<OdooTovch>,
    /// /serverKhyanalt/pm2Aldaa — pm2 error log (console.error)-ийн нэгтгэл
    pub pm2: Option<Pm2Aldaa>,
    pub temdeglel: Vec<String>,
}

#[derive(Serialize)]
pub struct Pm2Aldaa {
    pub niit: usize,
    pub bulgemuud: Vec<Pm2Bulgem>,
    /// Log файл хэт том тул цонхны эхэн хэсэг уншигдаагүй
    pub dutuu: bool,
    pub khyazgaar_khursen: bool,
    /// Сервер log хавтсыг уншиж чадаагүй
    pub aldaa: Option<String>,
}

#[derive(Serialize)]
pub struct Pm2Bulgem {
    pub too: usize,
    /// ID, тоо, хэрэглэгчийн утгыг хассан message (бүлгийн түлхүүр)
    pub tulkhuur: String,
    pub bairshil: Option<String>,
    pub stack: Vec<String>,
    pub instance: Vec<String>,
    pub ekhnii: String,
    pub suuliin: String,
}

/// GET /serverKhyanalt/pm2Aldaa-ийн хариу. murnuud (түүхий мөр)-ийг авахгүй —
/// тайланд зөвхөн сервер талд цэвэрлэсэн бүлгийн түлхүүр орно.
pub fn pm2_unshiy(v: &Value) -> Pm2Aldaa {
    let tsag = |b: &Value, k: &str| {
        b.get(k)
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| ub_tsag(&d.with_timezone(&Utc)))
            .unwrap_or_default()
    };
    let bulgemuud = v
        .get("bulgemuud")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|b| Pm2Bulgem {
            too: b.get("too").and_then(Value::as_u64).unwrap_or(0) as usize,
            tulkhuur: b.get("tulkhuur").and_then(Value::as_str).unwrap_or("").to_string(),
            bairshil: b.get("bairshil").and_then(Value::as_str).map(str::to_string),
            stack: str_jagsaalt(b.get("stack")).collect(),
            instance: str_jagsaalt(b.get("instance")).collect(),
            ekhnii: tsag(b, "ekhnii"),
            suuliin: tsag(b, "suuliin"),
        })
        .collect();
    Pm2Aldaa {
        niit: v.get("niit").and_then(Value::as_u64).unwrap_or(0) as usize,
        bulgemuud,
        dutuu: v
            .get("failuud")
            .and_then(Value::as_array)
            .is_some_and(|f| f.iter().any(|f| f.get("dutuu").and_then(Value::as_bool) == Some(true))),
        khyazgaar_khursen: v.get("khyazgaarKhursen").and_then(Value::as_bool) == Some(true),
        aldaa: v.get("aldaa").and_then(Value::as_str).map(str::to_string),
    }
}

#[derive(Serialize)]
pub struct OdooTovch {
    pub instance: String,
    pub idevkhtei_too: usize,
    pub request_log_daalgasan_too: Option<i64>,
    pub busad_instance_too: usize,
    /// Цонхонд эхэлсэн удаан хүсэлтүүд method + url загвараар (query, ID-гүй)
    pub udaan: Vec<UdaanBulgem>,
}

#[derive(Serialize)]
pub struct UdaanBulgem {
    pub method: String,
    pub zam: String,
    pub too: usize,
    pub max_ms: f64,
    pub status_5xx: usize,
}

fn ipv4(seg: &str) -> Option<[u8; 4]> {
    let mut d = [0u8; 4];
    let mut khesguud = seg.split(':').next()?.split('.');
    for b in &mut d {
        *b = khesguud.next()?.parse().ok()?;
    }
    khesguud.next().is_none().then_some(d)
}

/// URL-аас query, IP болон ID / улсын дугаар / цаг шиг сегментийг хасна
/// (customer мэдээлэл тайланд орохгүй). Дотоод сүлжээний IP нь `:dotoodIp` —
/// асуудал LAN төхөөрөмж (камер, самбар)-тэй холбоотой эсэх нь харагдана.
fn zam_zagvar(url: &str) -> String {
    let zam = url.split(['?', '#']).next().unwrap_or("");
    zam.split('/')
        .map(|seg| {
            if let Some(ip) = ipv4(seg) {
                let dotood = matches!(ip, [10, ..] | [127, ..] | [192, 168, ..])
                    || (ip[0] == 172 && (16..=31).contains(&ip[1]));
                return if dotood { ":dotoodIp" } else { ":ip" };
            }
            let id_shig = !seg.is_empty()
                && (seg.chars().all(|c| c.is_ascii_digit())
                    || seg.chars().filter(char::is_ascii_digit).count() >= 3
                    || (seg.len() >= 16 && seg.chars().all(|c| c.is_ascii_hexdigit() || c == '-')));
            if id_shig { ":id" } else { seg }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// suuliinApi-ийн мөрийг тайланд орох талбаруудаар, url-ийг загвар болгон
fn api_tsevelrey(a: &Value) -> Value {
    serde_json::json!({
        "method": a.get("method"),
        "url": zam_zagvar(a.get("url").and_then(Value::as_str).unwrap_or("")),
        "requestId": a.get("requestId"),
        "khugatsaaMs": a.get("khugatsaaMs"),
        "status": a.get("status"),
        "duusaagui": a.get("duusaagui").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn api_jagsaalt(v: Option<&Value>) -> Vec<Value> {
    v.and_then(Value::as_array).into_iter().flatten().take(10).map(api_tsevelrey).collect()
}

/// Түүхий log-ийн busadInstance (instance → heartbeat, хуучин log-д массив)
fn busad_instance_baidal(v: Option<&Value>) -> Option<ServerBaidal> {
    let hb: Vec<&Value> = match v? {
        Value::Object(m) => m.values().collect(),
        Value::Array(a) => a.iter().collect(),
        _ => return None,
    };
    let max = |k: &str| hb.iter().filter_map(|h| h.get(k).and_then(Value::as_f64)).reduce(f64::max);
    let b = ServerBaidal {
        process_cpu_max: max("processCpu"),
        system_cpu_max: max("systemCpu"),
        event_loop_max_ms: max("eventLoopMaxMs"),
    };
    (b != ServerBaidal::default()).then_some(b)
}

fn odoo_tovch(v: &Value, ekhlel: DateTime<Utc>, tugsgul: DateTime<Utc>) -> OdooTovch {
    let arr_len = |k: &str| v.get(k).and_then(Value::as_array).map_or(0, Vec::len);
    let mut udaan: HashMap<(String, String), UdaanBulgem> = HashMap::new();
    for u in v.get("udaanKhuseltuud").and_then(Value::as_array).into_iter().flatten() {
        let ekhlesen = u
            .get("ekhlesen")
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc));
        if !ekhlesen.is_some_and(|t| t >= ekhlel && t < tugsgul) {
            continue;
        }
        let method = u.get("method").and_then(Value::as_str).unwrap_or("-").to_string();
        let zam = zam_zagvar(u.get("url").and_then(Value::as_str).unwrap_or(""));
        let ms = u.get("khugatsaaMs").and_then(Value::as_f64).unwrap_or(0.0);
        let b = udaan.entry((method.clone(), zam.clone())).or_insert(UdaanBulgem {
            method,
            zam,
            too: 0,
            max_ms: 0.0,
            status_5xx: 0,
        });
        b.too += 1;
        b.max_ms = b.max_ms.max(ms);
        if u.get("status").and_then(Value::as_i64).is_some_and(|s| s >= 500) {
            b.status_5xx += 1;
        }
    }
    let mut udaan: Vec<UdaanBulgem> = udaan.into_values().collect();
    udaan.sort_by(|a, b| b.too.cmp(&a.too).then(b.max_ms.total_cmp(&a.max_ms)));
    OdooTovch {
        instance: v.get("instance").map(|i| i.to_string().trim_matches('"').to_string()).unwrap_or_default(),
        idevkhtei_too: arr_len("idevkhteiKhuseltuud"),
        request_log_daalgasan_too: v.get("requestLogDaalgasanToo").and_then(Value::as_i64),
        busad_instance_too: arr_len("busadInstance"),
        udaan,
    }
}

#[derive(Default)]
struct Acc {
    too: usize,
    sum: f64,
    n: usize,
    max: Option<f64>,
    el_max: Option<f64>,
    tasarsan: usize,
    suuliin: Option<DateTime<Utc>>,
    daalgasan: usize,
    angilal: Option<String>,
    jishee: Jishee,
    gadaad_sum: f64,
    gadaad_n: usize,
    gadaad_max: Option<f64>,
    db_sum: f64,
    db_n: usize,
    hostuud: Vec<String>,
    server: Option<ServerBaidal>,
}

impl Acc {
    fn gadaad_nemey(&mut self, ms: f64, n: usize) {
        self.gadaad_sum += ms * n as f64;
        self.gadaad_n += n;
    }
    fn db_nemey(&mut self, ms: f64, n: usize) {
        self.db_sum += ms * n as f64;
        self.db_n += n;
    }
    fn server_nemey(&mut self, b: Option<ServerBaidal>) {
        if let Some(b) = b {
            self.server.get_or_insert_with(ServerBaidal::default).nemey(b);
        }
    }
    fn host_nemey(&mut self, host: String) {
        if self.hostuud.len() < 5 && !self.hostuud.contains(&host) {
            self.hostuud.push(host);
        }
    }
}

const JISHEE_TOO: usize = 3;

fn jishee_nemey<T: PartialEq>(jagsaalt: &mut Vec<T>, utga: T) {
    if jagsaalt.len() < JISHEE_TOO && !jagsaalt.contains(&utga) {
        jagsaalt.push(utga);
    }
}

fn str_jagsaalt(v: Option<&Value>) -> impl Iterator<Item = String> + '_ {
    v.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
}

/// `/khuudas?id=1` → `/khuudas`
fn query_gui(s: &str) -> String {
    s.split('?').next().unwrap_or("").to_string()
}

fn fmax(a: Option<f64>, b: f64) -> Option<f64> {
    Some(a.map_or(b, |a| a.max(b)))
}

fn angilay(turul: &str, status: Option<i64>, max: Option<f64>, tasarsan: usize, b: &Bosgo) -> (u8, String) {
    if status.is_some_and(|s| s >= 500) {
        return (1, "HTTP 5xx алдаа".into());
    }
    match turul {
        "GATSALT" | "INSTANCE_GATSSAN" | "WATCHDOG" => (1, "Серверийн гацалт / instance".into()),
        "BROWSER_GATSALT" => (2, "Frontend гацалт".into()),
        "DB_UDAAN" | "MONGO_DISK" => (2, "Баазын bottleneck".into()),
        "CPU" | "PROCESS_CPU" => (2, "CPU ачаалал".into()),
        "REQUEST_SLOW" => {
            if max.is_some_and(|m| m >= b.udaan_ms) {
                (2, format!("Жинхэнэ удаан (max ≥ {:.0}ms)", b.udaan_ms))
            } else if tasarsan > 0 {
                (2, "Хүсэлт тасарсан".into())
            } else if status.is_some_and(|s| s >= 400) {
                (3, "HTTP 4xx".into())
            } else {
                (
                    3,
                    format!(
                        "REQUEST_SLOW шошготой ч {:.0}ms-ээс хурдан (бичих босго {:.0}ms)",
                        b.udaan_ms, b.request_log_ms
                    ),
                )
            }
        }
        _ => (3, "Бусад".into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn tailan_beltgey<'a>(
    records: impl IntoIterator<Item = &'a Value>,
    ognoo: &str,
    ekhlel: DateTime<Utc>,
    tugsgul: DateTime<Utc>,
    bosgo: &Bosgo,
    tasaldluud: &[Tasaldal],
    ekhnii_bichlegt: Option<DateTime<Utc>>,
    odoo: Option<&Value>,
) -> Tailan {
    let mut bulgem: HashMap<Tulkhuur, Acc> = HashMap::new();
    let mut turluud = BTreeMap::new();
    let mut niit = 0;

    for v in records {
        let Some(t) = created_at(v) else { continue };
        if t < ekhlel || t >= tugsgul {
            continue;
        }
        niit += 1;
        let turul = v.get("turul").and_then(Value::as_str).unwrap_or("?").to_string();
        *turluud.entry(turul.clone()).or_insert(0) += 1;

        let req = v.get("request");
        let s = |k: &str| {
            req.and_then(|r| r.get(k))
                .and_then(Value::as_str)
                .unwrap_or("-")
                .to_string()
        };
        let status = req.and_then(|r| r.get("status")).and_then(Value::as_i64);
        let host = v.get("host").and_then(Value::as_str).unwrap_or("-").to_string();
        let aldaa = req.and_then(|r| r.get("aldaa"));
        let a_str = |k: &str| aldaa.and_then(|a| a.get(k)).and_then(Value::as_str).map(str::to_string);
        let khuudas = v.pointer("/klient/khuudas").and_then(Value::as_str).map(query_gui);
        let klient_turul = v.pointer("/klient/turul").and_then(Value::as_str).map(str::to_string);
        let acc = bulgem
            .entry((turul, s("method"), s("route"), status, host, a_str("ner"), a_str("bairshil"), khuudas, klient_turul))
            .or_default();
        // Хамгийн сүүлийн гацалтынх үлдэнэ
        let api = api_jagsaalt(v.pointer("/klient/suuliinApi"));
        if !api.is_empty() && acc.suuliin.is_none_or(|s| t >= s) {
            acc.jishee.suuliin_api = api;
        }
        acc.server_nemey(busad_instance_baidal(v.get("busadInstance")));
        acc.too += 1;
        if acc.angilal.is_none() {
            acc.angilal = a_str("angilal");
        }
        if let Some(m) = a_str("message") {
            jishee_nemey(&mut acc.jishee.messages, m);
        }
        if let Some(id) = req.and_then(|r| r.get("requestId")).and_then(Value::as_str) {
            jishee_nemey(&mut acc.jishee.request_ids, id.to_string());
        }
        if acc.jishee.stack.is_empty() {
            acc.jishee.stack = str_jagsaalt(aldaa.and_then(|a| a.get("stack"))).collect();
        }
        if let Some(sk) = v.pointer("/klient/urtFrame/0/skriptuud/0") {
            jishee_nemey(&mut acc.jishee.skriptuud, sk.clone());
        }
        if let Some(gadaad) = req.and_then(|r| r.get("gadaad")).and_then(Value::as_array) {
            let ms: f64 = gadaad.iter().filter_map(|g| g.get("ms").and_then(Value::as_f64)).sum();
            acc.gadaad_nemey(ms, 1);
            acc.gadaad_max = fmax(acc.gadaad_max, ms);
            for h in gadaad.iter().filter_map(|g| g.get("host").and_then(Value::as_str)) {
                acc.host_nemey(h.to_string());
            }
        }
        if let Some(ms) = req.and_then(|r| r.pointer("/db/niitMs")).and_then(Value::as_f64) {
            acc.db_nemey(ms, 1);
        }
        if let Some(ms) = req.and_then(|r| r.get("khugatsaaMs")).and_then(Value::as_f64) {
            acc.sum += ms;
            acc.n += 1;
            acc.max = fmax(acc.max, ms);
        }
        if let Some(el) = v.get("eventLoopMaxMs").and_then(Value::as_f64) {
            acc.el_max = fmax(acc.el_max, el);
        }
        if req.and_then(|r| r.get("tasarsan")).and_then(Value::as_bool) == Some(true) {
            acc.tasarsan += 1;
        }
        acc.suuliin = acc.suuliin.max(Some(t));
    }

    let mut temdeglel = vec![
        "Эх сурвалж (нөөц арга): /serverKhyanalt (төрөл бүрээр, тус бүр сүүлийн 200), /serverKhyanalt/requests (REQUEST_SLOW, сүүлийн 24ц, ≤100). Backend-д /serverKhyanalt/tailan байхгүй үед ашиглана. Зөвхөн API-д хариулсан серверийн (host) бичлэг орно.".to_string(),
    ];
    if ekhnii_bichlegt.is_some_and(|t| t > ekhlel) {
        temdeglel.push(format!(
            "Local сан дахь хамгийн эртний бичлэг {} — цонхны эхлэлийг бүрэн хамраагүй байж магадгүй.",
            ub_tsag(&ekhnii_bichlegt.unwrap())
        ));
    }
    let tasaldluud = tasaldluud
        .iter()
        .filter(|g| g.tugsgul > ekhlel && g.ekhlel < tugsgul)
        .cloned()
        .collect();
    duusgay(bulgem, turluud, niit, ognoo, ekhlel, tugsgul, bosgo, tasaldluud, odoo, temdeglel)
}

/// GET /serverKhyanalt/tailan-ийн хариу (сервер талд нэгтгэсэн, limit-гүй) — үндсэн арга.
pub fn tailan_serverees(
    v: &Value,
    ognoo: &str,
    ekhlel: DateTime<Utc>,
    tugsgul: DateTime<Utc>,
    bosgo: &Bosgo,
    odoo: Option<&Value>,
) -> Tailan {
    // Backend-ийн бодит тохиргоо config-ийн утгыг дарна
    let tokhirgoo = v.get("tokhirgoo");
    let tokh = |k: &str| tokhirgoo.and_then(|t| t.get(k)).and_then(Value::as_f64);
    let bosgo = Bosgo {
        udaan_ms: tokh("udaanKhuseltMs").unwrap_or(bosgo.udaan_ms),
        request_log_ms: tokh("requestLogMs").unwrap_or(bosgo.request_log_ms),
    };

    let mut bulgem: HashMap<Tulkhuur, Acc> = HashMap::new();
    let mut turluud = BTreeMap::new();
    let mut niit = 0;
    for b in v.get("bulgemuud").and_then(Value::as_array).into_iter().flatten() {
        let s = |k: &str| b.get(k).and_then(Value::as_str).unwrap_or("-").to_string();
        let f = |k: &str| b.get(k).and_then(Value::as_f64);
        let too = b.get("too").and_then(Value::as_u64).unwrap_or(0) as usize;
        let turul = b.get("turul").and_then(Value::as_str).unwrap_or("?").to_string();
        niit += too;
        *turluud.entry(turul.clone()).or_insert(0) += too;
        let status = b.get("status").and_then(Value::as_i64);
        let o = |k: &str| b.get(k).and_then(Value::as_str).map(str::to_string);
        let acc = bulgem
            .entry((turul, s("method"), s("route"), status, s("host"), o("aldaaNer"), o("bairshil"), o("khuudas"), o("klientTurul")))
            .or_default();
        if acc.jishee.suuliin_api.is_empty() {
            acc.jishee.suuliin_api = api_jagsaalt(b.pointer("/jishee/suuliinApi"));
        }
        if let Some(sb) = b.get("serverBaidal").filter(|x| x.is_object()) {
            let f = |k: &str| sb.get(k).and_then(Value::as_f64);
            acc.server_nemey(Some(ServerBaidal {
                process_cpu_max: f("processCpuMax"),
                system_cpu_max: f("systemCpuMax"),
                event_loop_max_ms: f("eventLoopMaxMs"),
            }));
        }
        acc.too += too;
        acc.daalgasan += b.get("daalgasanToo").and_then(Value::as_u64).unwrap_or(0) as usize;
        if acc.angilal.is_none() {
            acc.angilal = o("angilal");
        }
        let jishee = b.get("jishee");
        for m in str_jagsaalt(jishee.and_then(|j| j.get("messages"))) {
            jishee_nemey(&mut acc.jishee.messages, m);
        }
        for id in str_jagsaalt(jishee.and_then(|j| j.get("requestIds"))) {
            jishee_nemey(&mut acc.jishee.request_ids, id);
        }
        if acc.jishee.stack.is_empty() {
            acc.jishee.stack = str_jagsaalt(jishee.and_then(|j| j.get("stack"))).collect();
        }
        for sk in jishee.and_then(|j| j.get("skriptuud")).and_then(Value::as_array).into_iter().flatten() {
            jishee_nemey(&mut acc.jishee.skriptuud, sk.clone());
        }
        if let Some(ms) = f("dundajGadaadMs") {
            acc.gadaad_nemey(ms, too);
        }
        if let Some(m) = f("maxGadaadMs") {
            acc.gadaad_max = fmax(acc.gadaad_max, m);
        }
        if let Some(ms) = f("dundajDbMs") {
            acc.db_nemey(ms, too);
        }
        for h in str_jagsaalt(b.get("gadaadKhostuud")) {
            acc.host_nemey(h);
        }
        if let Some(d) = f("dundajMs") {
            acc.sum += d * too as f64;
            acc.n += too;
        }
        if let Some(m) = f("maxMs") {
            acc.max = fmax(acc.max, m);
        }
        if let Some(el) = f("eventLoopMaxMs") {
            acc.el_max = fmax(acc.el_max, el);
        }
        acc.tasarsan += b.get("tasarsanToo").and_then(Value::as_u64).unwrap_or(0) as usize;
        let suuliin = created_at(&serde_json::json!({ "createdAt": b.get("suuliin") }));
        acc.suuliin = acc.suuliin.max(suuliin);
    }

    let host = v.get("host").and_then(Value::as_str).unwrap_or("бүх сервер");
    let mut temdeglel = vec![format!(
        "Эх сурвалж: /serverKhyanalt/tailan — цонхны бүх бичлэгийг сервер талд нэгтгэсэн (limit-гүй), host: {host}."
    )];
    if v.get("khyazgaarKhursen").and_then(Value::as_bool) == Some(true) {
        temdeglel.push("Бүлгийн тоо серверийн дээд хязгаарт хүрсэн — цөөн тохиолдолтой зарим бүлэг орхигдсон.".into());
    }
    duusgay(bulgem, turluud, niit, ognoo, ekhlel, tugsgul, &bosgo, Vec::new(), odoo, temdeglel)
}

/// turul, method, route, status, host, aldaa_ner, bairshil, khuudas, klient_turul
type Tulkhuur = (
    String,
    String,
    String,
    Option<i64>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[allow(clippy::too_many_arguments)]
fn duusgay(
    bulgem: HashMap<Tulkhuur, Acc>,
    turluud: BTreeMap<String, usize>,
    niit: usize,
    ognoo: &str,
    ekhlel: DateTime<Utc>,
    tugsgul: DateTime<Utc>,
    bosgo: &Bosgo,
    tasaldluud: Vec<Tasaldal>,
    odoo: Option<&Value>,
    mut temdeglel: Vec<String>,
) -> Tailan {
    let mut bulgemuud: Vec<Bulgem> = bulgem
        .into_iter()
        .map(|((turul, method, route, status, host, aldaa_ner, bairshil, khuudas, klient_turul), a)| {
            let (ach, mut shaltgaan) = angilay(&turul, status, a.max, a.tasarsan, bosgo);
            if klient_turul.as_deref() == Some("CRASH") {
                shaltgaan = "Frontend tab унасан".into();
            }
            Bulgem {
                ach,
                shaltgaan,
                turul,
                method,
                route,
                status,
                host,
                too: a.too,
                dundaj_ms: (a.n > 0).then(|| a.sum / a.n as f64),
                max_ms: a.max,
                event_loop_max_ms: a.el_max,
                tasarsan_too: a.tasarsan,
                suuliin: a.suuliin.map(|t| ub_tsag(&t)).unwrap_or_default(),
                aldaa_ner,
                bairshil,
                khuudas,
                klient_turul,
                server_baidal: a.server,
                angilal: a.angilal,
                daalgasan_too: a.daalgasan,
                dundaj_gadaad_ms: (a.gadaad_n > 0).then(|| a.gadaad_sum / a.gadaad_n as f64),
                max_gadaad_ms: a.gadaad_max,
                dundaj_db_ms: (a.db_n > 0).then(|| a.db_sum / a.db_n as f64),
                gadaad_hostuud: a.hostuud,
                jishee: a.jishee,
            }
        })
        .collect();
    bulgemuud.sort_by(|a, b| {
        a.ach
            .cmp(&b.ach)
            .then(b.too.cmp(&a.too))
            .then(b.max_ms.partial_cmp(&a.max_ms).unwrap_or(std::cmp::Ordering::Equal))
    });

    let odoo = odoo.map(|v| odoo_tovch(v, ekhlel, tugsgul));

    temdeglel.push(format!(
        "REQUEST_SLOW нь {:.0}ms+ эсвэл status ≥ 500 эсвэл тасарсан хүсэлтийг бичдэг — шошго нь удаан гэсэн баталгаа биш. Жинхэнэ удаан босго {:.0}ms.",
        bosgo.request_log_ms, bosgo.udaan_ms
    ));
    if bulgemuud.iter().any(|b| b.daalgasan_too > 0) {
        temdeglel.push("Минутын хязгаараас (KHYANALT_REQUEST_LOG_LIMIT_MIN) хэтэрсэн хүсэлтүүд REQUEST_STAT-аар тоологдож \"тоо\"-нд орсон (\"хаягдсан\" — бие даасан мөр, requestId-гүй).".to_string());
    } else {
        temdeglel.push("Backend нэг process-оос минутад хязгаартай тооны request log бичдэг (KHYANALT_REQUEST_LOG_LIMIT_MIN). REQUEST_STAT-гүй хуучин backend бол тоо нь бодит тооноос бага байж болно.".to_string());
    }
    temdeglel.push("\"гадаад / db ms\" багана: хүсэлт бүрийн гадаад HTTP (QPay, eBarimt, SMS…) болон Mongo query-ийн нийлбэр хугацааны дундаж (khuseltKontekst). Зэрэг явсан query-үүдийн нийлбэр нь нийт хугацаанаас их байж болно. \"-\" бол шинэчлэлээс өмнөх log.".to_string());
    temdeglel.push("\"алдаа / хуудас\" багана: 5xx-ийн алдаа гарсан repo доторх мөр (aldaaBarigch-ийн stack). \"мэдээлэлгүй\" бол хариуг aldaaBarigch-ээр дамжуулалгүй шууд res.status(500) буцаасан эсвэл шинэчлэлээс өмнөх log.".to_string());
    if odoo.as_ref().is_some_and(|o| o.request_log_daalgasan_too.unwrap_or(0) > 0) {
        temdeglel.push("requestLogDaalgasanToo > 0 — минутын хязгаараар зарим REQUEST_SLOW бичигдээгүй.".into());
    }
    if niit == 0 {
        temdeglel.push("Энэ цонхонд бичлэг алга — service ажиллаж байсан, token хүчинтэй байсан эсэхийг шалгана уу.".into());
    }

    Tailan {
        ognoo: ognoo.to_string(),
        ekhlel: ub_tsag(&ekhlel),
        tugsgul: ub_tsag(&tugsgul),
        niit,
        turluud,
        bulgemuud,
        tasaldluud,
        odoo,
        pm2: None,
        temdeglel,
    }
}

fn ms(v: Option<f64>) -> String {
    v.map(|v| format!("{v:.0}")).unwrap_or_else(|| "-".into())
}

/// `7900 (merchant.qpay.mn) / 120` — хүсэлтийн хугацааны хэд нь гадаад API, хэд нь Mongo
fn khugatsaa_zadargaa(b: &Bulgem) -> String {
    if b.dundaj_gadaad_ms.is_none() && b.dundaj_db_ms.is_none() {
        return "-".into();
    }
    let hostuud = if b.gadaad_hostuud.is_empty() {
        String::new()
    } else {
        format!(" ({})", b.gadaad_hostuud.iter().take(2).cloned().collect::<Vec<_>>().join(", "))
    };
    format!("{}{hostuud} / {}", ms(b.dundaj_gadaad_ms), ms(b.dundaj_db_ms))
}

fn bairshil_tekst(b: &Bulgem) -> String {
    match (&b.aldaa_ner, &b.bairshil, &b.khuudas) {
        (n, Some(bair), _) => format!("{} `{bair}`", n.as_deref().unwrap_or("")).trim().to_string(),
        (_, None, Some(kh)) => format!("`{kh}`"),
        // 5xx боловч алдааны мэдээлэлгүй — aldaaBarigch-ээр дамжаагүй эсвэл хуучин log
        _ if b.status.is_some_and(|s| s >= 500) => "мэдээлэлгүй".into(),
        _ => "-".into(),
    }
}

pub fn markdown(t: &Tailan) -> String {
    let mut s = format!(
        "# serverKhyanaltLog тайлан — {}\n\nЦонх: {} → {} (Asia/Ulaanbaatar, UTC+8)  \nНийт бичлэг: **{}**\n\n## Төрлөөр\n\n| turul | тоо |\n|---|---:|\n",
        t.ognoo, t.ekhlel, t.tugsgul, t.niit
    );
    for (k, v) in &t.turluud {
        s.push_str(&format!("| {k} | {v} |\n"));
    }

    s.push_str("\n## Ач холбогдлоор\n\n| P | turul | method | route | status | алдаа / хуудас | host | тоо | дундаж ms | max ms | гадаад / db ms | eventLoop max | тасарсан | сүүлд | тайлбар |\n|---|---|---|---|---:|---|---|---:|---:|---:|---|---:|---:|---|---|\n");
    for b in &t.bulgemuud {
        s.push_str(&format!(
            "| P{} | {} | {} | `{}` | {} | {} | {} | {}{} | {} | {} | {} | {} | {} | {} | {} |\n",
            b.ach,
            b.turul,
            b.method,
            b.route.replace('|', "\\|"),
            b.status.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
            bairshil_tekst(b).replace('|', "\\|"),
            b.host,
            b.too,
            if b.daalgasan_too > 0 { format!(" ({} хаягдсан)", b.daalgasan_too) } else { String::new() },
            ms(b.dundaj_ms),
            ms(b.max_ms),
            khugatsaa_zadargaa(b).replace('|', "\\|"),
            ms(b.event_loop_max_ms),
            b.tasarsan_too,
            b.suuliin,
            b.shaltgaan
        ));
    }

    let jisheetei: Vec<&Bulgem> = t
        .bulgemuud
        .iter()
        .filter(|b| {
            b.ach <= 2
                && (b.bairshil.is_some() || !b.jishee.skriptuud.is_empty() || !b.jishee.suuliin_api.is_empty())
        })
        .collect();
    if !jisheetei.is_empty() {
        s.push_str("\n## Алдааны жишээ (P1/P2)\n\n");
        for b in jisheetei {
            s.push_str(&format!(
                "### {} `{}` {} — {}\n\n",
                b.method,
                b.route,
                b.status.map(|v| v.to_string()).unwrap_or_default(),
                bairshil_tekst(b)
            ));
            if let Some(a) = &b.angilal {
                s.push_str(&format!("- ангилал: {a}\n"));
            }
            if b.turul == "BROWSER_GATSALT" {
                s.push_str(&format!("- шалтгаан: {}\n", b.shaltgaan));
            }
            if let Some(sb) = &b.server_baidal {
                s.push_str(&format!(
                    "- тухайн үеийн сервер: process CPU max {}% · system CPU max {}% · eventLoop max {}ms — {}\n",
                    ms(sb.process_cpu_max),
                    ms(sb.system_cpu_max),
                    ms(sb.event_loop_max_ms),
                    if sb.kheviin() { "хэвийн, асуудал browser талд" } else { "сервер ачаалалтай байсан" }
                ));
            }
            for m in &b.jishee.messages {
                s.push_str(&format!("- message: `{}`\n", m.replace('`', "'")));
            }
            if !b.jishee.stack.is_empty() {
                s.push_str("- stack:\n");
                for f in &b.jishee.stack {
                    s.push_str(&format!("  - `{f}`\n"));
                }
            }
            for sk in &b.jishee.skriptuud {
                s.push_str(&format!("- script: `{sk}`\n"));
            }
            if !b.jishee.request_ids.is_empty() {
                s.push_str(&format!("- requestId: {}\n", b.jishee.request_ids.join(", ")));
            }
            if !b.jishee.suuliin_api.is_empty() {
                s.push_str("- гацахаас өмнөх API (сүүлийнх нь эхэнд):\n");
                for a in &b.jishee.suuliin_api {
                    let status = match a.get("status").and_then(Value::as_i64) {
                        _ if a.get("duusaagui").and_then(Value::as_bool) == Some(true) => "дуусаагүй".to_string(),
                        Some(0) => "сүлжээний алдаа".to_string(),
                        Some(st) => st.to_string(),
                        None => "-".to_string(),
                    };
                    s.push_str(&format!(
                        "  - {} `{}` — {} · {}ms{}\n",
                        a.get("method").and_then(Value::as_str).unwrap_or("-"),
                        a.get("url").and_then(Value::as_str).unwrap_or(""),
                        status,
                        ms(a.get("khugatsaaMs").and_then(Value::as_f64)),
                        a.get("requestId").and_then(Value::as_str).map(|r| format!(" · {r}")).unwrap_or_default()
                    ));
                }
            }
            s.push('\n');
        }
    }

    if let Some(p) = &t.pm2 {
        s.push_str(&format!(
            "\n## pm2 error log (console.error)\n\nЦонхонд {} бичлэг, {} төрөл (ID, тоо, хэрэглэгчийн утгыг хассан message + байршлаар бүлэглэсэн).\n\n",
            p.niit,
            p.bulgemuud.len()
        ));
        if let Some(a) = &p.aldaa {
            s.push_str(&format!("Уншиж чадсангүй: {a}\n"));
        }
        if p.dutuu {
            s.push_str("Log файл хэт том — цонхны эхэн хэсэг уншигдаагүй (файл бүрийн сүүлийн 16MB).\n\n");
        }
        if !p.bulgemuud.is_empty() {
            s.push_str("| тоо | message | байршил | instance | эхэнд | сүүлд |\n|---:|---|---|---|---|---|\n");
            for b in &p.bulgemuud {
                s.push_str(&format!(
                    "| {} | `{}` | {} | {} | {} | {} |\n",
                    b.too,
                    b.tulkhuur.replace('|', "\\|").replace('`', "'"),
                    b.bairshil.as_ref().map(|x| format!("`{x}`")).unwrap_or_else(|| "-".into()),
                    b.instance.join(", "),
                    b.ekhnii,
                    b.suuliin
                ));
            }
            if p.khyazgaar_khursen {
                s.push_str("\nБүлгийн тоо серверийн дээд хязгаарт (200) хүрсэн — цөөн тохиолдолтой зарим нь орхигдсон.\n");
            }
            let stacktai: Vec<&Pm2Bulgem> = p.bulgemuud.iter().filter(|b| b.stack.len() > 1).take(10).collect();
            if !stacktai.is_empty() {
                s.push_str("\n### pm2 алдааны stack\n\n");
                for b in stacktai {
                    s.push_str(&format!("- `{}` ({}×)\n", b.tulkhuur.replace('`', "'"), b.too));
                    for f in &b.stack {
                        s.push_str(&format!("  - `{f}`\n"));
                    }
                }
            }
        }
    }

    s.push_str("\n## Цуглуулалтын тасалдал\n\n");
    if t.tasaldluud.is_empty() {
        s.push_str("Илрээгүй.\n");
    }
    for g in &t.tasaldluud {
        s.push_str(&format!(
            "- `{}`: {} → {} хооронд хамрах хүрээ дутуу (API-ийн limit хүрсэн)\n",
            g.survalj,
            ub_tsag(&g.ekhlel),
            ub_tsag(&g.tugsgul)
        ));
    }

    if let Some(o) = &t.odoo {
        s.push_str(&format!(
            "\n## Одоогийн төлөв (/odoo)\n\nInstance: `{}` · идэвхтэй хүсэлт: {} · бусад instance: {} · requestLogDaalgasanToo: {}\n\n",
            o.instance,
            o.idevkhtei_too,
            o.busad_instance_too,
            o.request_log_daalgasan_too.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
        ));
        if o.udaan.is_empty() {
            s.push_str("Цонхонд эхэлсэн удаан хүсэлт санах ойд алга.\n");
        } else {
            s.push_str("| method | зам | тоо | max ms | 5xx |\n|---|---|---:|---:|---:|\n");
            for u in &o.udaan {
                s.push_str(&format!(
                    "| {} | `{}` | {} | {:.0} | {} |\n",
                    u.method,
                    u.zam.replace('|', "\\|"),
                    u.too,
                    u.max_ms,
                    u.status_5xx
                ));
            }
        }
    }

    s.push_str("\n## Тэмдэглэл\n\n");
    for n in &t.temdeglel {
        s.push_str(&format!("- {n}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    const BOSGO: Bosgo = Bosgo { udaan_ms: 5000.0, request_log_ms: 1000.0 };

    fn req(id: &str, at: &str, route: &str, status: i64, ms: f64) -> Value {
        json!({
            "_id": id, "turul": "REQUEST_SLOW", "createdAt": at,
            "request": {"method": "POST", "route": route, "status": status, "khugatsaaMs": ms, "tasarsan": false}
        })
    }

    #[test]
    fn bulgemlej_angilna() {
        let recs = [
            req("1", "2026-10-26T23:00:00Z", "/a", 200, 1200.0),
            req("2", "2026-10-26T23:10:00Z", "/a", 200, 1800.0),
            req("3", "2026-10-26T23:20:00Z", "/b", 500, 300.0),
            req("4", "2026-10-26T23:30:00Z", "/c", 200, 9000.0),
            // цонхноос гадуур
            req("5", "2026-10-20T00:00:00Z", "/b", 500, 100.0),
            json!({"_id": "6", "turul": "GATSALT", "createdAt": "2026-10-26T22:00:00Z", "eventLoopMaxMs": 3100}),
        ];
        let tl = tailan_beltgey(
            recs.iter(),
            "2026-10-27",
            t("2026-10-26T00:00:00Z"),
            t("2026-10-27T00:00:00Z"),
            &BOSGO,
            &[],
            None,
            None,
        );
        assert_eq!(tl.niit, 5);
        assert_eq!(tl.bulgemuud.len(), 4);

        let a = tl.bulgemuud.iter().find(|b| b.route == "/a").unwrap();
        assert_eq!(a.too, 2);
        assert_eq!(a.dundaj_ms, Some(1500.0));
        assert_eq!(a.max_ms, Some(1800.0));
        assert_eq!(a.ach, 3, "босгоос доош REQUEST_SLOW жинхэнэ удаан биш");

        let b = tl.bulgemuud.iter().find(|b| b.route == "/b").unwrap();
        assert_eq!((b.ach, b.too), (1, 1));

        let c = tl.bulgemuud.iter().find(|b| b.route == "/c").unwrap();
        assert_eq!(c.ach, 2);

        let g = tl.bulgemuud.iter().find(|b| b.turul == "GATSALT").unwrap();
        assert_eq!((g.ach, g.event_loop_max_ms), (1, Some(3100.0)));

        // P1 эхэнд
        assert!(tl.bulgemuud[0].ach == 1 && tl.bulgemuud.last().unwrap().ach == 3);
        assert_eq!(c.suuliin, "2026-10-27 07:30:00");
    }

    #[test]
    fn tasaldal_tsonkhoor_shuugdene() {
        let gaps = vec![
            Tasaldal { survalj: "x".into(), ekhlel: t("2026-10-26T05:00:00Z"), tugsgul: t("2026-10-26T06:00:00Z") },
            Tasaldal { survalj: "x".into(), ekhlel: t("2026-10-20T05:00:00Z"), tugsgul: t("2026-10-20T06:00:00Z") },
        ];
        let tl = tailan_beltgey(
            std::iter::empty(),
            "d",
            t("2026-10-26T00:00:00Z"),
            t("2026-10-27T00:00:00Z"),
            &BOSGO,
            &gaps,
            Some(t("2026-10-26T12:00:00Z")),
            None,
        );
        assert_eq!(tl.tasaldluud.len(), 1);
        assert_eq!(tl.niit, 0);
        assert!(tl.temdeglel.len() >= 5);
        assert!(markdown(&tl).contains("Цуглуулалтын тасалдал"));
    }

    #[test]
    fn serveriin_negtgelees_tailan() {
        let v = json!({
            "host": "srv1",
            "niit": 13,
            "khyazgaarKhursen": true,
            "tokhirgoo": {"udaanKhuseltMs": 3000, "requestLogMs": 800},
            "bulgemuud": [
                {"turul": "REQUEST_SLOW", "route": "/a", "method": "GET", "status": 200, "host": "srv1",
                 "too": 10, "dundajMs": 1200, "maxMs": 3500, "eventLoopMaxMs": null, "tasarsanToo": 0,
                 "suuliin": "2026-10-27T12:00:00.000Z"},
                {"turul": "REQUEST_SLOW", "route": "/b", "method": "POST", "status": 500, "host": "srv1",
                 "too": 2, "dundajMs": 100, "maxMs": 150, "tasarsanToo": 1, "suuliin": "2026-10-27T11:00:00.000Z"},
                {"turul": "GATSALT", "route": null, "method": null, "status": null, "host": "srv1",
                 "too": 1, "dundajMs": null, "maxMs": null, "eventLoopMaxMs": 2500, "tasarsanToo": 0,
                 "suuliin": "2026-10-27T10:00:00.000Z"}
            ]
        });
        let tl = tailan_serverees(&v, "d", t("2026-10-26T13:00:00Z"), t("2026-10-27T13:00:00Z"), &BOSGO, None);
        assert_eq!(tl.niit, 13);
        assert_eq!(tl.turluud["REQUEST_SLOW"], 12);
        let a = tl.bulgemuud.iter().find(|b| b.route == "/a").unwrap();
        // backend-ийн udaanKhuseltMs=3000 config-ийн 5000-г дарна
        assert_eq!((a.ach, a.dundaj_ms, a.host.as_str()), (2, Some(1200.0), "srv1"));
        assert_eq!(a.suuliin, "2026-10-27 20:00:00");
        let g = tl.bulgemuud.iter().find(|b| b.turul == "GATSALT").unwrap();
        assert_eq!((g.ach, g.route.as_str(), g.dundaj_ms), (1, "-", None));
        assert!(tl.temdeglel.iter().any(|n| n.contains("дээд хязгаарт")));
        assert!(tl.temdeglel.iter().any(|n| n.contains("3000ms")));
    }

    #[test]
    fn aldaanii_bairshil_jishee_khadgalna() {
        let v = json!({
            "host": "srv1",
            "bulgemuud": [
                {"turul": "REQUEST_SLOW", "route": "/msgIlgeeye", "method": "POST", "status": 500, "host": "srv1",
                 "aldaaNer": "Error", "bairshil": "routes/mailRoute.js:1190", "angilal": "SYSTEM",
                 "too": 12, "daalgasanToo": 10, "dundajMs": 30, "maxMs": 90, "tasarsanToo": 0,
                 "dundajGadaadMs": 23, "maxGadaadMs": 30, "dundajDbMs": 5, "gadaadKhostuud": ["sms.mn"],
                 "suuliin": "2026-10-27T12:00:00.000Z",
                 "jishee": {"messages": ["SMS gateway"], "requestIds": ["r2", "r1"], "stack": ["routes/mailRoute.js:1190:9"], "skriptuud": []}},
                {"turul": "REQUEST_SLOW", "route": "/msgIlgeeye", "method": "POST", "status": 500, "host": "srv1",
                 "too": 1, "dundajMs": 10, "maxMs": 10, "tasarsanToo": 0, "suuliin": "2026-10-27T12:00:00.000Z"},
                {"turul": "BROWSER_GATSALT", "host": "srv1", "khuudas": "/khyanalt/geree", "too": 2, "tasarsanToo": 0,
                 "suuliin": "2026-10-27T12:00:00.000Z",
                 "jishee": {"skriptuud": [{"invoker": "onclick", "url": "main.js"}]}}
            ]
        });
        let tl = tailan_serverees(&v, "d", t("2026-10-26T13:00:00Z"), t("2026-10-27T13:00:00Z"), &BOSGO, None);
        // Байршлаар тусдаа бүлэг хэвээр үлдэнэ
        assert_eq!(tl.bulgemuud.iter().filter(|b| b.route == "/msgIlgeeye").count(), 2);
        let m = tl.bulgemuud.iter().find(|b| b.bairshil.is_some()).unwrap();
        assert_eq!((m.too, m.daalgasan_too), (12, 10));
        assert_eq!(m.jishee.request_ids, ["r2", "r1"]);
        let md = markdown(&tl);
        assert!(md.contains("`routes/mailRoute.js:1190`"));
        assert!(md.contains("12 (10 хаягдсан)"));
        assert_eq!((m.dundaj_gadaad_ms, m.dundaj_db_ms), (Some(23.0), Some(5.0)));
        assert!(md.contains("23 (sms.mn) / 5"));
        assert!(md.contains("мэдээлэлгүй"));
        assert!(md.contains("message: `SMS gateway`"));
        assert!(md.contains("`/khyanalt/geree`"));
        assert!(md.contains("onclick"));
        assert!(tl.temdeglel.iter().any(|n| n.contains("REQUEST_STAT")));
    }

    #[test]
    fn browser_crash_omnokh_api_server_baidal() {
        let v = json!({
            "host": "srv1",
            "bulgemuud": [
                {"turul": "BROWSER_GATSALT", "host": "srv1", "khuudas": "/khyanalt/zogsool/camera", "klientTurul": "CRASH",
                 "too": 1, "tasarsanToo": 0, "suuliin": "2026-10-28T00:25:26.000Z",
                 "serverBaidal": {"processCpuMax": 24, "systemCpuMax": 30, "eventLoopMaxMs": 87},
                 "jishee": {"suuliinApi": [
                     {"method": "GET", "url": "/api/sambar/192.168.1.195/7476УБИ/08:23:11", "status": 0, "khugatsaaMs": 2},
                     {"method": "POST", "url": "/api/zogsoolTusBur?x=1", "requestId": "035fb253", "status": 200, "khugatsaaMs": 19}
                 ]}},
                {"turul": "BROWSER_GATSALT", "host": "srv1", "khuudas": "/khyanalt/zogsool/camera", "klientTurul": "GATSALT",
                 "too": 2, "tasarsanToo": 0, "suuliin": "2026-10-27T06:17:00.000Z"}
            ]
        });
        let tl = tailan_serverees(&v, "d", t("2026-10-26T13:00:00Z"), t("2026-10-28T13:00:00Z"), &BOSGO, None);
        // tab унасан ба хөлдсөн тусдаа бүлэг
        assert_eq!(tl.bulgemuud.len(), 2);
        let c = tl.bulgemuud.iter().find(|b| b.klient_turul.as_deref() == Some("CRASH")).unwrap();
        assert_eq!(c.shaltgaan, "Frontend tab унасан");
        assert_eq!(c.server_baidal.unwrap().event_loop_max_ms, Some(87.0));
        let md = markdown(&tl);
        assert!(md.contains("`/api/sambar/:dotoodIp/:id/:id` — сүлжээний алдаа"));
        assert!(md.contains("`/api/zogsoolTusBur` — 200 · 19ms · 035fb253"));
        assert!(md.contains("хэвийн, асуудал browser талд"));
        assert!(!md.contains("7476") && !md.contains("192.168"));
    }

    #[test]
    fn pm2_aldaa_tailand_orno() {
        let v = json!({
            "niit": 7,
            "khyazgaarKhursen": false,
            "failuud": [{"ner": "tureesBack-error-0.log", "dutuu": true}],
            "bulgemuud": [
                {"tulkhuur": "TypeError: Cannot read properties of undefined (reading 'map')", "too": 5,
                 "bairshil": "controller/geree.js:120", "stack": ["controller/geree.js:120:15 gereeJagsaalt", "routes/gereeRoute.js:40:3"],
                 "instance": ["0", "3"], "ekhnii": "2026-10-27T01:00:00.000Z", "suuliin": "2026-10-27T12:00:00.000Z"},
                {"tulkhuur": "qpay aldaa :n | :email", "too": 2, "bairshil": null, "stack": [], "instance": ["-"],
                 "ekhnii": "2026-10-27T02:00:00.000Z", "suuliin": "2026-10-27T03:00:00.000Z"}
            ],
            "murnuud": [{"tekst": "qpay aldaa 99112233 a@b.mn"}]
        });
        let mut tl = tailan_serverees(&json!({"bulgemuud": []}), "d", t("2026-10-26T13:00:00Z"), t("2026-10-27T13:00:00Z"), &BOSGO, None);
        tl.pm2 = Some(pm2_unshiy(&v));
        let p = tl.pm2.as_ref().unwrap();
        assert_eq!((p.niit, p.bulgemuud.len(), p.dutuu), (7, 2, true));
        assert_eq!(p.bulgemuud[0].suuliin, "2026-10-27 20:00:00");
        let md = markdown(&tl);
        assert!(md.contains("## pm2 error log"));
        assert!(md.contains("| 5 | `TypeError: Cannot read properties of undefined (reading 'map')` | `controller/geree.js:120` | 0, 3 |"));
        assert!(md.contains("qpay aldaa :n \\| :email"));
        assert!(md.contains("  - `routes/gereeRoute.js:40:3`"));
        assert!(md.contains("эхэн хэсэг уншигдаагүй"));
        // Түүхий мөр тайланд орохгүй
        assert!(!md.contains("99112233") && !md.contains("a@b.mn"));
    }

    #[test]
    fn tuukhii_log_oos_busad_instance() {
        let r = json!({
            "_id": "1", "createdAt": "2026-10-26T10:00:00Z", "turul": "BROWSER_GATSALT",
            "klient": {"khuudas": "/a?x=1", "turul": "GATSALT", "suuliinApi": [{"method": "GET", "url": "/api/b/123"}]},
            "busadInstance": {"0": {"processCpu": 20, "eventLoopMaxMs": 900}, "1": {"processCpu": 95}}
        });
        let tl = tailan_beltgey([&r], "d", t("2026-10-26T00:00:00Z"), t("2026-10-27T00:00:00Z"), &BOSGO, &[], None, None);
        let b = &tl.bulgemuud[0];
        let sb = b.server_baidal.unwrap();
        assert_eq!((sb.process_cpu_max, sb.event_loop_max_ms), (Some(95.0), Some(900.0)));
        assert!(!sb.kheviin());
        assert_eq!(b.jishee.suuliin_api[0]["url"], "/api/b/:id");
    }

    #[test]
    fn odoo_url_ees_id_query_khasna() {
        assert_eq!(zam_zagvar("http://103.143.40.230:8080/stream"), "http://:ip/stream");
        assert_eq!(zam_zagvar("/api/zogsoolJagsaalt"), "/api/zogsoolJagsaalt");
        assert_eq!(zam_zagvar("/geree/66a1b2c3d4e5f60718293a4b?token=x"), "/geree/:id");
        assert_eq!(zam_zagvar("/tulbur/12345/list"), "/tulbur/:id/list");
        let odoo = json!({
            "instance": "turees-0",
            "idevkhteiKhuseltuud": [{}],
            "requestLogDaalgasanToo": 3,
            "udaanKhuseltuud": [
                {"method": "GET", "url": "/a/123?x=1", "khugatsaaMs": 7000, "status": 200, "ekhlesen": "2026-10-26T10:00:00Z", "ip": "1.2.3.4"},
                {"method": "GET", "url": "/a/456", "khugatsaaMs": 9000, "status": 502, "ekhlesen": "2026-10-26T11:00:00Z"},
                {"method": "GET", "url": "/a/789", "khugatsaaMs": 9999, "status": 200, "ekhlesen": "2026-10-20T11:00:00Z"}
            ]
        });
        let tl = tailan_beltgey(
            std::iter::empty(),
            "d",
            t("2026-10-26T00:00:00Z"),
            t("2026-10-27T00:00:00Z"),
            &BOSGO,
            &[],
            None,
            Some(&odoo),
        );
        let o = tl.odoo.as_ref().unwrap();
        assert_eq!((o.instance.as_str(), o.idevkhtei_too), ("turees-0", 1));
        assert_eq!(o.udaan.len(), 1);
        assert_eq!((o.udaan[0].zam.as_str(), o.udaan[0].too, o.udaan[0].max_ms, o.udaan[0].status_5xx), ("/a/:id", 2, 9000.0, 1));
        let md = markdown(&tl);
        assert!(!md.contains("1.2.3.4") && !md.contains("x=1"));
        assert!(tl.temdeglel.iter().any(|n| n.contains("requestLogDaalgasanToo")));
    }
}
