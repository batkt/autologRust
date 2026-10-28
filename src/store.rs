// Local хадгалалт: API зөвхөн сүүлийн 200/100 бичлэг өгдөг тул татсан бүрийг
// _id-аар давхардалгүй logs.jsonl-д хуримтлуулж, тайланг эндээс гаргана.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

pub fn created_at(v: &Value) -> Option<DateTime<Utc>> {
    v.get("createdAt")
        .and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

fn id_of(v: &Value) -> Option<String> {
    v.get("_id").and_then(Value::as_str).map(str::to_string)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

// ─── Бичлэгийн сан ────────────────────────────────────────────────────────────

pub struct LogStore {
    path: PathBuf,
    records: HashMap<String, Value>,
}

impl LogStore {
    pub fn load(path: PathBuf) -> Self {
        let mut records = HashMap::new();
        if let Ok(f) = fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    if let Some(id) = id_of(&v) {
                        records.insert(id, v);
                    }
                }
            }
        }
        Self { path, records }
    }

    /// Шинэ бичлэгүүдийг нэмээд хэд нэмэгдснийг буцаана.
    pub fn merge(&mut self, items: Vec<Value>) -> io::Result<usize> {
        let mut file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        let mut shine = 0;
        for v in items {
            let Some(id) = id_of(&v) else { continue };
            if self.records.contains_key(&id) {
                continue;
            }
            writeln!(file, "{}", serde_json::to_string(&v)?)?;
            self.records.insert(id, v);
            shine += 1;
        }
        Ok(shine)
    }

    /// cutoff-оос хуучин бичлэгийг устгаж файлыг шахна.
    pub fn prune(&mut self, cutoff: DateTime<Utc>) -> io::Result<usize> {
        let umnu = self.records.len();
        self.records
            .retain(|_, v| created_at(v).is_some_and(|t| t >= cutoff));
        let ustgasan = umnu - self.records.len();
        if ustgasan > 0 {
            let mut buf = String::new();
            for v in self.records.values() {
                buf.push_str(&serde_json::to_string(v)?);
                buf.push('\n');
            }
            atomic_write(&self.path, buf.as_bytes())?;
        }
        Ok(ustgasan)
    }

    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn ekhnii(&self) -> Option<DateTime<Utc>> {
        self.records.values().filter_map(created_at).min()
    }
}

// ─── Төлөв (state.json) ───────────────────────────────────────────────────────

/// Татсан багц дүүрэн (limit) ирээд хамгийн хуучин нь тайлангийн цонхны
/// эхлэлээс хойш байвал ekhlel..tugsgul хооронд бичлэг дутуу байж болзошгүй.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Tasaldal {
    pub survalj: String,
    pub ekhlel: DateTime<Utc>,
    pub tugsgul: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TailanBichlega {
    pub ognoo: String,
    pub ekhlel: DateTime<Utc>,
    pub tugsgul: DateTime<Utc>,
    pub niit: usize,
    pub uusgesen: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct State {
    #[serde(default)]
    pub suuliin_tatalt: Option<DateTime<Utc>>,
    #[serde(default)]
    pub suuliin_aldaa: Option<String>,
    #[serde(default)]
    pub tasaldluud: Vec<Tasaldal>,
    /// Хянасан цонхнууд — нэг өдрийг дахин тайлагнахгүй
    #[serde(default)]
    pub tailanguud: Vec<TailanBichlega>,
    /// Claude-г автоматаар ажиллуулсан огноонууд (өдөрт нэг удаа)
    #[serde(default)]
    pub claude_shalgasan: Vec<String>,
}

impl State {
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        atomic_write(path, serde_json::to_string_pretty(self)?.as_bytes())
    }
}

pub fn tasaldal_shalgay(
    survalj: &str,
    items: &[Value],
    limit: usize,
    ekhlel: Option<DateTime<Utc>>,
) -> Option<Tasaldal> {
    if items.len() < limit {
        return None;
    }
    let umnukh = ekhlel?;
    let khuuchin = items.iter().filter_map(created_at).min()?;
    (khuuchin > umnukh).then(|| Tasaldal {
        survalj: survalj.to_string(),
        ekhlel: umnukh,
        tugsgul: khuuchin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tmp_path(ner: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("autolog-test-{}-{ner}", std::process::id()));
        let _ = fs::remove_file(&p);
        p
    }

    #[test]
    fn merge_davkhardahgui_bas_dakhin_achaalna() {
        let path = tmp_path("merge.jsonl");
        let mut s = LogStore::load(path.clone());
        let a = json!({"_id": "a", "createdAt": "2026-10-27T00:00:00.000Z"});
        let b = json!({"_id": "b", "createdAt": "2026-10-27T01:00:00.000Z"});
        assert_eq!(s.merge(vec![a.clone(), b]).unwrap(), 2);
        assert_eq!(s.merge(vec![a, json!({"no_id": 1})]).unwrap(), 0);
        let s2 = LogStore::load(path.clone());
        assert_eq!(s2.len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn prune_khuuchniig_ustgana() {
        let path = tmp_path("prune.jsonl");
        let mut s = LogStore::load(path.clone());
        s.merge(vec![
            json!({"_id": "a", "createdAt": "2026-09-01T00:00:00Z"}),
            json!({"_id": "b", "createdAt": "2026-10-27T00:00:00Z"}),
        ])
        .unwrap();
        let cutoff = DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z").unwrap().with_timezone(&Utc);
        assert_eq!(s.prune(cutoff).unwrap(), 1);
        assert_eq!(LogStore::load(path.clone()).len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn tasaldal_zuvkhun_duurenn_bagts_deer() {
        let t = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        let items = vec![
            json!({"createdAt": "2026-10-27T05:00:00Z"}),
            json!({"createdAt": "2026-10-27T04:00:00Z"}),
        ];
        let umnukh = Some(t("2026-10-27T03:00:00Z"));
        assert!(tasaldal_shalgay("x", &items, 3, umnukh).is_none());
        let g = tasaldal_shalgay("x", &items, 2, umnukh).unwrap();
        assert_eq!(g.ekhlel, t("2026-10-27T03:00:00Z"));
        assert_eq!(g.tugsgul, t("2026-10-27T04:00:00Z"));
        assert!(tasaldal_shalgay("x", &items, 2, Some(t("2026-10-27T04:30:00Z"))).is_none());
    }
}
