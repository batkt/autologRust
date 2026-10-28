// Өдрийн тайлангаас Claude Code-д (headless `claude -p`) даалгавар өгнө.
//
// Хамгаалалт:
//  - Commit хийгдээгүй өөрчлөлттэй repo-г алгасна — хэрэглэгчийн ажлыг хадгална,
//    өмнөх өдрийн Claude-ийн засварыг хянаж дуусаагүй бол дахин хүрэхгүй.
//  - Зөвхөн ZUVSHUURUL дахь tool-ууд; -p горимд бусад нь автоматаар татгалзагдана.
//  - git commit/push/reset/checkout, npm install, pm2, вэб хандалтыг тусгайлан хориглоно.
//  - --max-budget-usd ба timeout.

use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

const ZUVSHUURUL: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "Edit",
    "Write",
    "Bash(git status:*)",
    "Bash(git diff:*)",
    "Bash(git log:*)",
    "Bash(git show:*)",
    "Bash(node --test:*)",
    "Bash(node --check:*)",
    "Bash(npm test:*)",
    "Bash(npm run lint:*)",
    "Bash(npm run build:*)",
    "Bash(npx tsc:*)",
    "Bash(npx eslint:*)",
];

const KHORIG: &[&str] = &[
    "Bash(git commit:*)",
    "Bash(git push:*)",
    "Bash(git reset:*)",
    "Bash(git checkout:*)",
    "Bash(git restore:*)",
    "Bash(git clean:*)",
    "Bash(git stash:*)",
    "Bash(git rebase:*)",
    "Bash(git merge:*)",
    "Bash(npm install:*)",
    "Bash(npm publish:*)",
    "Bash(pm2:*)",
    "WebFetch",
    "WebSearch",
];

pub struct ClaudeTokhirgoo {
    pub exe: String,
    pub repos: Vec<String>,
    pub budget_usd: f64,
    pub timeout_min: u64,
    pub model: String,
}

pub struct ClaudeUr {
    pub garalt: PathBuf,
    pub zasvarlasan: Vec<String>,
    pub algassan: Vec<String>,
    pub zardal_usd: Option<f64>,
}

fn tovch(s: &str, n: usize) -> String {
    let s = s.replace(['\n', '\r'], " ");
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s
    }
}

/// stream-json-ийн нэг мөрийг явцын мөр болгоно (assistant-ийн текст, tool дуудлага).
fn yavts_mur(v: &Value) -> Vec<String> {
    if v.get("type").and_then(Value::as_str) != Some("assistant") {
        return Vec::new();
    }
    let content = v.pointer("/message/content").and_then(Value::as_array);
    content
        .into_iter()
        .flatten()
        .filter_map(|c| match c.get("type").and_then(Value::as_str) {
            Some("text") => c
                .get("text")
                .and_then(Value::as_str)
                .filter(|t| !t.trim().is_empty())
                .map(|t| format!("💬 {}", tovch(t, 160))),
            Some("tool_use") => {
                let ner = c.get("name").and_then(Value::as_str).unwrap_or("?");
                let input = c.get("input");
                let s = |k: &str| input.and_then(|i| i.get(k)).and_then(Value::as_str);
                let delgerengui = s("file_path")
                    .or_else(|| s("command"))
                    .or_else(|| s("pattern"))
                    .or_else(|| s("path"))
                    .unwrap_or("");
                Some(format!("🔧 {ner} {}", tovch(delgerengui, 140)))
            }
            _ => None,
        })
        .collect()
}

async fn git_status(repo: &str) -> Result<String, String> {
    let out = Command::new("git")
        .args(["-C", repo, "status", "--porcelain"])
        .output()
        .await
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn prompt_beltgey(
    ognoo: &str,
    md: &Path,
    json: &Path,
    tuukh: &Path,
    repos: &[String],
    algassan: &[String],
) -> String {
    let repo_jagsaalt = repos.iter().map(|r| format!("- {r}")).collect::<Vec<_>>().join("\n");
    let algassan = if algassan.is_empty() {
        String::new()
    } else {
        format!(
            "\nДараах repo-д commit хийгдээгүй өөрчлөлт байсан тул энэ удаа хамруулаагүй (хүрэхгүй): {}\n",
            algassan.join(", ")
        )
    };
    format!(
        r#"Чи turees системийн шөнийн serverKhyanaltLog шалгалтыг хийж байна ({ognoo}).

Эх нотолгоо (зөвхөн унших):
- Өдрийн тайлан: {md}
- Мөн JSON хэлбэрээр: {json}
- Өмнөх шалгалтын түүх: {tuukh} (байхгүй бол анхны шалгалт) — аль хэдийн шийдсэн, шалгасан асуудлыг дахин судлахгүй.

Засвар хийж болох repo-ууд:
{repo_jagsaalt}
{algassan}
Журам:
1. Тайлангаас P1 (HTTP 5xx, серверийн гацалт), дараа нь P2 (жинхэнэ удаан, тасарсан, frontend гацалт, баазын bottleneck) асуудлуудыг сонго. REQUEST_SLOW шошгыг khugatsaaMs болон босгыг шалгалгүйгээр удаан гэж бүү үз. Тайлангийн "Тэмдэглэл", "Цуглуулалтын тасалдал" хэсгээс хамрах хүрээний хязгаарыг харгалз.
2. Асуудал бүрийг route-аас эх код руу мөшгиж, засвар хийхээс өмнө root cause-ийн нотолгоог ол. "алдаа / хуудас" багана болон "Алдааны жишээ" хэсэгт `bairshil` (file:line), stack, message байвал тэр мөрөөс эхэл — route-аар таамаглахгүй. Ангилал VALIDATION боловч 5xx бол kod дутуу throw гэсэн үг. BROWSER_GATSALT-д хуудас, script (invoker/url/funktsiin)-ээр frontend кодыг ол. "тухайн үеийн сервер" хэвийн бол backend-ийг бүү зас — асуудал browser талд. Script url нь `/_next/static/chunks/*.js` (minified, source map-гүй) бол мөрийн дугаарыг эх кодтой бүү тулга; invoker `MessagePort.onmessage` нь React scheduler — тухайн хуудасны component-ийн хүнд render (virtualization-гүй том жагсаалт, render доторх sort/filter/map, useMemo-гүй тооцоолол, давтагдах setState)-ийг хай. "Frontend tab унасан" (CRASH) нь ихэвчлэн санах ой: Browser санах ой (JS heap) бага байсан ч видео/stream, canvas, interval/listener-ийн цэвэрлэгээ (useEffect cleanup), дахин холбогдох давталт байгаа эсэхийг шалга. "гацахаас өмнөх API" жагсаалтад `:dotoodIp` (камер, самбар зэрэг LAN төхөөрөмж) руу сүлжээний алдаа давтагдаж байвал retry-ийн backoff/хязгаар, унтарсан төхөөрөмжийн үед resource чөлөөлж байгаа эсэхийг шалга. requestId-г тайлангийн удаан/5xx хүсэлтийн requestId-тэй тулгаж сервер талын хугацааг ол. Удаан хүсэлтэд "гадаад / db ms" баганыг харьцуул: гадаад хугацаа нийт хугацааны ихэнхийг эзэлж байвал root cause нь тухайн host (QPay, eBarimt г.м.) — манай кодод timeout/retry зөв эсэхийг л шалга; db их бол query/index; хоёулаа бага бол манай код (sync тооцоолол, давталт). Байршил "мэдээлэлгүй" 5xx-ийг "шалтгаан олдсонгүй" гэхээсээ өмнө тухайн route aldaaBarigch-ээр дамжилгүй шууд res.status(500) буцааж байгаа эсэхийг шалга.
3. Нотолгоо баттай бол жижиг, төвлөрсөн засвар хийж, боломжтой бол regression тест нэм. Баталж чадахгүй бол засварлахгүй — зөвлөмж болгон бич.
4. Холбогдох тест, lint, type check, build ажиллуул. Унасан бол унасан гэж бич, давсан гэж бүү хэл.

Хатуу дүрэм:
- git commit, push, deploy, production service restart хийхгүй. Өөрчлөлт зөвхөн local working tree-д үлдэнэ.
- Бодит төлбөр (qpay), ebarimt, SMS, бусад гадаад side-effect үйлчилгээг тестэд дуудахгүй.
- Monitoring бичлэг, production өгөгдлийг өөрчлөхгүй.
- Credential, token, харилцагчийн мэдээлэл, sensitive payload-ийг тайланд бүү оруул.
- Бүх дэлгэцийг шалгасан гэж route/module-ийн жагсаалтаар нотлохгүйгээр бүү хэл.

Эцэст нь дараах бүтэцтэй Markdown тайлан хэвлэ (энэ нь файлд хадгалагдана):
# Claude шалгалт — {ognoo}
## Тэргүүлэх асуудлууд (нотолгоо, root cause)
## Өөрчилсөн файлууд
## Тест / lint / build үр дүн
## Зөвлөмж (засаагүй)
## Үлдсэн эрсдэл
## Түүхэнд нэмэх (нэг мөрөөр: route — төлөв: засварласан / зөвлөмж / шалтгаан олдсонгүй)
"#,
        md = md.display(),
        json = json.display(),
        tuukh = tuukh.display(),
    )
}

/// Тайлангаар Claude-г ажиллуулж, гаралтыг reports/<ognoo>-claude.md-д бичнэ.
pub async fn ajilluulya(
    t: &ClaudeTokhirgoo,
    ognoo: &str,
    md: &Path,
    json: &Path,
    reports_dir: &Path,
    yavts: impl Fn(&str),
) -> Result<ClaudeUr, String> {
    let mut tseverkhen = Vec::new();
    let mut algassan = Vec::new();
    for repo in &t.repos {
        if !Path::new(repo).is_dir() {
            algassan.push(format!("{repo} (олдсонгүй)"));
            continue;
        }
        match git_status(repo).await {
            Ok(s) if s.trim().is_empty() => tseverkhen.push(repo.clone()),
            Ok(_) => algassan.push(repo.clone()),
            Err(e) => algassan.push(format!("{repo} ({e})")),
        }
    }
    if tseverkhen.is_empty() {
        return Err(format!(
            "Засварлах цэвэр repo алга — бүгд commit хийгдээгүй өөрчлөлттэй: {}",
            algassan.join(", ")
        ));
    }

    yavts(&format!(
        "Засварлах repo: {} · алгассан: {}",
        tseverkhen.join(", "),
        if algassan.is_empty() { "-".to_string() } else { algassan.join(", ") }
    ));
    let tuukh = reports_dir.join("claude-tuukh.md");
    let prompt = prompt_beltgey(ognoo, md, json, &tuukh, &tseverkhen, &algassan);

    let mut cmd = Command::new(&t.exe);
    cmd.current_dir(&tseverkhen[0])
        .args(["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", "acceptEdits"])
        .arg("--max-budget-usd")
        .arg(t.budget_usd.to_string());
    if !t.model.is_empty() {
        cmd.arg("--model").arg(&t.model);
    }
    for dir in tseverkhen.iter().skip(1) {
        cmd.arg("--add-dir").arg(dir);
    }
    cmd.arg("--add-dir").arg(reports_dir);
    cmd.arg("--allowedTools").arg(ZUVSHUURUL.join(","));
    cmd.arg("--disallowedTools").arg(KHORIG.join(","));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| format!("'{}' ажиллуулж чадсангүй: {e}", t.exe))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await.map_err(|e| format!("stdin: {e}"))?;
        // stdin хаагдсанаар Claude prompt дууссаныг мэднэ
    }
    let stdout = child.stdout.take().ok_or("stdout алга")?;
    let mut stderr = child.stderr.take().ok_or("stderr алга")?;

    // Бүх event-ийг аудитад хадгалж, явцыг шууд log-д бичнэ
    let stream_path = reports_dir.join(format!("{ognoo}-claude.jsonl"));
    let mut stream_file = std::fs::File::create(&stream_path).map_err(|e| format!("{}: {e}", stream_path.display()))?;
    let mut ur_text: Option<String> = None;
    let mut zardal_usd = None;
    let mut aldaa_ur = false;
    let unshilt = async {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = std::io::Write::write_all(&mut stream_file, format!("{line}\n").as_bytes());
            let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
            for m in yavts_mur(&v) {
                yavts(&m);
            }
            if v.get("type").and_then(Value::as_str) == Some("result") {
                ur_text = v.get("result").and_then(Value::as_str).map(str::to_string);
                zardal_usd = v.get("total_cost_usd").and_then(Value::as_f64);
                aldaa_ur = v.get("is_error").and_then(Value::as_bool) == Some(true);
            }
        }
        child.wait().await
    };
    let status = tokio::time::timeout(Duration::from_secs(t.timeout_min * 60), unshilt)
        .await
        .map_err(|_| format!("Claude {} минутад дуусаагүй — зогсоолоо", t.timeout_min))?
        .map_err(|e| format!("Claude: {e}"))?;

    let stdout = ur_text.unwrap_or_default();
    let garalt = reports_dir.join(format!("{ognoo}-claude.md"));
    std::fs::write(&garalt, &stdout).map_err(|e| format!("{}: {e}", garalt.display()))?;
    if !status.success() || aldaa_ur {
        let mut err = String::new();
        let _ = stderr.read_to_string(&mut err).await;
        let tailbar = if err.trim().is_empty() { tovch(&stdout, 300) } else { tovch(&err, 500) };
        return Err(format!("Claude exit {:?}: {tailbar}", status.code()));
    }

    let mut zasvarlasan = Vec::new();
    for repo in &tseverkhen {
        if let Ok(s) = git_status(repo).await {
            for line in s.lines().filter(|l| !l.trim().is_empty()) {
                zasvarlasan.push(format!("{repo}: {}", line.trim()));
            }
        }
    }
    tuukh_nemey(&tuukh, ognoo, &garalt, &zasvarlasan, &algassan, &stdout);
    Ok(ClaudeUr { garalt, zasvarlasan, algassan, zardal_usd })
}

/// reports/claude-tuukh.md — хянасан цонх, шийдсэн асуудлын түүх (давтан судлахгүйн тулд).
fn tuukh_nemey(path: &Path, ognoo: &str, garalt: &Path, zasvarlasan: &[String], algassan: &[String], stdout: &str) {
    let mut s = format!("\n## {ognoo}\n\nClaude тайлан: {}\n", garalt.display());
    if let Some(i) = stdout.find("## Түүхэнд нэмэх") {
        let khesgiin = &stdout[i..];
        let tugsgul = khesgiin[3..].find("\n## ").map_or(khesgiin.len(), |j| j + 3);
        for line in khesgiin[..tugsgul].lines().skip(1).filter(|l| !l.trim().is_empty()) {
            s.push_str(line);
            s.push('\n');
        }
    }
    if !zasvarlasan.is_empty() {
        s.push_str("\nWorking tree өөрчлөлт (хянаж, commit эсвэл буцаах хүртэл дараагийн шалгалт энэ repo-г алгасна):\n");
        for z in zasvarlasan {
            s.push_str(&format!("- {z}\n"));
        }
    }
    if !algassan.is_empty() {
        s.push_str(&format!("\nАлгассан repo: {}\n", algassan.join(", ")));
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, s.as_bytes()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_dureem_aguulna() {
        let p = prompt_beltgey(
            "2026-10-27",
            Path::new("r/2026-10-27.md"),
            Path::new("r/2026-10-27.json"),
            Path::new("r/claude-tuukh.md"),
            &["D:/Work/tureesBack".into()],
            &["D:/Work/parking-v2".into()],
        );
        assert!(p.contains("r/2026-10-27.md") && p.contains("D:/Work/tureesBack"));
        assert!(p.contains("git commit, push, deploy"));
        assert!(p.contains("D:/Work/parking-v2") && p.contains("хүрэхгүй"));
        assert!(p.contains("## Түүхэнд нэмэх"));
    }

    #[test]
    fn stream_json_yavts() {
        let v = serde_json::json!({"type": "assistant", "message": {"content": [
            {"type": "text", "text": "qpayShalgay-г шалгая"},
            {"type": "tool_use", "name": "Read", "input": {"file_path": "D:/Work/tureesBack/routes/qpayRoute.js"}},
            {"type": "tool_use", "name": "Bash", "input": {"command": "node --test test/"}}
        ]}});
        let m = yavts_mur(&v);
        assert_eq!(m.len(), 3);
        assert!(m[1].contains("Read") && m[1].contains("qpayRoute.js"));
        assert!(m[2].contains("node --test"));
        assert!(yavts_mur(&serde_json::json!({"type": "user"})).is_empty());
    }

    #[test]
    fn khorig_ni_zuvshuurulduu_baikhgui() {
        for k in KHORIG {
            assert!(!ZUVSHUURUL.contains(k), "{k}");
        }
        assert!(KHORIG.contains(&"Bash(git push:*)") && KHORIG.contains(&"Bash(git commit:*)"));
    }

    #[test]
    fn tuukhend_khesgiig_salgaj_bichne() {
        let path = std::env::temp_dir().join(format!("autolog-tuukh-{}.md", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let stdout = "# x\n## Үлдсэн эрсдэл\nа\n## Түүхэнд нэмэх\n- /qpayShalgay — засварласан\n\n## Өөр\nб\n";
        tuukh_nemey(&path, "2026-10-27", Path::new("g.md"), &["repo: M a.js".into()], &[], stdout);
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.contains("## 2026-10-27") && s.contains("/qpayShalgay — засварласан"));
        assert!(!s.contains("Үлдсэн эрсдэл") && !s.contains("Өөр") && s.contains("repo: M a.js"));
        let _ = std::fs::remove_file(path);
    }
}
