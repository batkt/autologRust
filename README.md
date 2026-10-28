# autolog — serverKhyanaltLog өдрийн тайлан (Rust, Windows Service)

`irtsWifiIPshalgakh` (zevIrts)-ийн бүтэцтэй ижил: `autolog.exe`-ийн хажууд `config.toml`, `autolog.log`, `data/`, `reports/` байна.

Өдөр бүр `tailan_tsag` (анхдагч **21:00, Asia/Ulaanbaatar**)-д нэг удаа API-аас уншиж, өмнөх 24 цагийн тайлан гаргана. Зөвхөн GET хийнэ — backend, MongoDB-д юу ч бичихгүй.

## Татах endpoint-ууд (`https://turees.zevtabs.mn/api`)

| Endpoint | Юу авах |
|---|---|
| `POST /ajiltanNevtrey` | JWT (config-д `token` байхгүй эсвэл 401 болсон үед) |
| **`GET /serverKhyanalt/tailan?ekhlel=&tugsgul=`** | **Үндсэн.** Цонхны бүх log-ийг сервер талд `turul/route/method/status/host`-оор нэгтгэсэн (limit-гүй), backend-ийн бодит босго (`tokhirgoo`). `bukh_host = true` → `&bukhHost=1` |
| `GET /serverKhyanalt/odoo` | Тухайн агшны instance-ийн төлөв, санах ой дахь удаан хүсэлтүүд |
| `GET /serverKhyanalt` ба `?turul=<төрөл>` | *Нөөц:* REQUEST_SLOW-оос бусад log, төрөл бүрт сүүлийн 200 |
| `GET /serverKhyanalt/requests?limit=100` | *Нөөц:* REQUEST_SLOW-ийн `route/method/status/khugatsaaMs` (`requests_tatakh = false` бол унтарна) |

`/serverKhyanalt/tailan` нь tureesBack-д шинээр нэмэгдсэн (`utils/serverKhyanaltTailan.js`). Deploy хийгдэхээс өмнө production дээр `/:id` route-д унаж алдаа буцаадаг тул autolog автоматаар нөөц endpoint-ууд руу шилжинэ.

Бүгд `tokenShalgakh` + `adminShalgay` (Zevtabs `CAdmin1`)-аар хамгаалагдсан. Token-гүй дуудвал backend **HTTP 500** буцаадаг.

## Тайлан

`reports/YYYY-MM-DD.md` ба `.json`:
- `turul / request.method / request.route / request.status`-аар бүлэглэж тоо, дундаж/max `khugatsaaMs`, eventLoop max, тасарсан тоо, сүүлд гарсан цаг (УБ цаг)
- Ач холбогдол: **P1** — 5xx, GATSALT/WATCHDOG/INSTANCE_GATSSAN; **P2** — max ≥ `udaan_bosgo_ms`, тасарсан, BROWSER_GATSALT, DB_UDAAN, CPU; **P3** — босгоос доош REQUEST_SLOW, 4xx
- BROWSER_GATSALT: tab унасан (CRASH) ба хөлдсөнийг тусад нь, гацахаас өмнөх API дуудлагууд (url нь query/IP/ID-гүй, LAN IP нь `:dotoodIp`), тухайн үеийн бүх instance-ийн CPU/eventLoop-ийн дээд утга
- API-ийн limit хүрч цонхыг бүрэн хамраагүй бол «Цуглуулалтын тасалдал»-д тэмдэглэнэ
- tailbar, байгууллагын нэр, IP, ажилтны нэр, URL query/ID тайланд орохгүй

`data/state.json` нь тайлагнасан цонхнуудыг хадгална — нэг өдрийг дахин тайлагнахгүй. Нэвтрэлт/татах бүрэн амжилтгүй бол тайлан гаргахгүй, цагийн дараа дахин оролдоно.

## Claude Code-оор автомат засвар

`claude_ajilluulakh = true` бол өдрийн тайланд P1/P2 бүлэг байхад `claude -p` (headless) ажиллана:

1. `claude_repos`-ийн repo бүрт `git status --porcelain` шалгана — **commit хийгдээгүй өөрчлөлттэй repo-г алгасна** (таны ажил, эсвэл өмнөх өдрийн хянагдаагүй Claude засвар хадгалагдана).
2. Prompt: тайлан (`reports/<огноо>.md/.json`), түүх (`reports/claude-tuukh.md`), шөнийн шалгалтын журам, safety дүрэм.
3. Зөвшөөрөгдсөн tool: Read/Glob/Grep/Edit/Write, `git status/diff/log/show`, `node --test/--check`, `npm test`, `npm run lint/build`, `npx tsc/eslint`. Хориотой: `git commit/push/reset/checkout/restore/clean/stash/rebase/merge`, `npm install/publish`, `pm2`, WebFetch/WebSearch. `-p` горимд жагсаалтад ороогүй tool автоматаар татгалзагдана.
4. `--max-budget-usd` (`claude_budget_usd`) ба `claude_timeout_min`.
5. Гаралт `reports/<огноо>-claude.md`, түүх `reports/claude-tuukh.md`-д нэмэгдэнэ (өөрчлөгдсөн файлын жагсаалттай).

Өөрчлөлтийг өөрөө хянаж commit эсвэл буцаана — commit, push, deploy автоматаар хийгдэхгүй.

```powershell
autolog.exe claude 2026-10-27   # бэлэн тайлангаар гараар ажиллуулах
```

**Service ба Claude-ийн нэвтрэлт:** `install` нь LocalSystem-ээр бүртгэдэг ба тэр account-д Claude-ийн нэвтрэлт байхгүй. Claude-тэй хамт ажиллуулах бол service-ийг өөрийн Windows хэрэглэгчээр ажиллуулна:

```powershell
sc config AutologKhyanalt obj= ".\pc" password= "<Windows нууц үг>"
sc stop AutologKhyanalt; sc start AutologKhyanalt
```

эсвэл Task Scheduler-ээр нэвтрэх үед `autolog.exe run` ажиллуулна.

## Ашиглах

```powershell
cargo build --release
copy config.example.toml target\release\config.toml   # nuutsUg эсвэл token бөглөнө

target\release\autolog.exe tailan 24    # нэг удаа татаж сүүлийн 24ц-ийн тайлан (reports\adhoc-*.md)
target\release\autolog.exe 2026-10-28   # тухайн өдрийн тайлан + Claude-г state-ээс үл хамааран дахин (= dakhin 2026-10-28)
target\release\autolog.exe dakhin       # өнөөдрийнхийг дахин
target\release\autolog.exe run          # console горим (21:00-д тайлан)

# Administrator PowerShell:
target\release\autolog.exe install
sc start AutologKhyanalt
sc stop  AutologKhyanalt
target\release\autolog.exe uninstall
```
