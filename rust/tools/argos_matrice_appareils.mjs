// Utilisation : node rust/tools/argos_matrice_appareils.mjs <dossier captures> <profil temporaire> [adresse, défaut http://127.0.0.1:8000]
// (Brave ou Chromium : /opt/brave.com/brave/brave ; profil temporaire = dossier vide, jamais le profil de tous les jours)
// 📱 Matrice d'appareils : chaque appareil émulé dans Brave (taille CSS, DPR, doigt, zones sûres), la chart doit aller
// exactement d'une zone sûre à l'autre, sans bande vide ; le menu Argos ▾ doit tenir dans l'écran. Captures + rapport.
import { spawn } from "node:child_process";
import fs from "node:fs";
const [out, prof] = process.argv.slice(2);
const port = 9339, base = process.argv[4] || "http://127.0.0.1:8000";
const U16 = "useyourloaf.com/blog/iphone-16-screen-sizes", U17 = "useyourloaf.com/blog/iphone-17-screen-sizes", IR = "ios-resolution.com", BL = "blisk.io/devices", YV = "yesviz.com/viewport";
const IOS = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const IPAD = "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const AND = "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Mobile Safari/537.36";
// [nom, largeur, hauteur, DPR, zones sûres {t,b,l,r}, agent, source]
const P = (t, b) => ({ t, b, l: 0, r: 0 }), L = (l, r, t, b) => ({ t, b, l, r });
const D = [
  ["iPhone SE (3e)", 375, 667, 2, P(20, 0), IOS, IR + " (zones : barre d'état 20, pas d'indicateur)"],
  ["iPhone 16e", 390, 844, 3, P(47, 34), IOS, IR + " (zones : n/d, valeurs de l'iPhone 14)"],
  ["iPhone 16", 393, 852, 3, P(59, 34), IOS, U16],
  ["iPhone 16 Plus", 430, 932, 3, P(59, 34), IOS, U16],
  ["iPhone 16 Pro", 402, 874, 3, P(62, 34), IOS, U16],
  ["iPhone 16 Pro Max", 440, 956, 3, P(62, 34), IOS, U16],
  ["iPhone 17", 402, 874, 3, P(62, 34), IOS, U17],
  ["iPhone Air", 420, 912, 3, P(68, 34), IOS, U17],
  ["iPhone 17 Pro", 402, 874, 3, P(62, 34), IOS, U17],
  ["iPhone 17 Pro Max", 440, 956, 3, P(62, 34), IOS, U17],
  ["iPhone 17 Pro (paysage)", 874, 402, 3, L(62, 62, 20, 20), IOS, U17],
  ["iPhone 16 (paysage)", 852, 393, 3, L(59, 59, 0, 21), IOS, U16],
  ["iPhone de Jee, appli (fenêtre 751 sur 812)", 375, 751, 3, P(61, 30), IOS, "mesures de l'iPhone de Jee"],
  ["iPad mini (7e)", 744, 1133, 2, P(24, 20), IPAD, IR + " (zones : n/d, valeurs usuelles)"],
  ["iPad Air 11", 820, 1180, 2, P(24, 20), IPAD, IR],
  ["iPad Air 13", 1024, 1366, 2, P(24, 20), IPAD, IR],
  ["iPad Pro 11", 834, 1210, 2, P(24, 20), IPAD, IR],
  ["iPad Pro 13", 1032, 1376, 2, P(24, 20), IPAD, IR],
  ["iPad Pro 13 (paysage)", 1376, 1032, 2, P(24, 20), IPAD, IR],
  ["Pixel 8", 412, 915, 2.625, P(0, 0), AND, BL],
  ["Pixel 8 Pro", 448, 998, 3, P(0, 0), AND, BL + " / " + YV],
  ["Pixel 9", 360, 808, 3, P(0, 0), AND, YV],
  ["Pixel 9 Pro", 427, 952, 3, P(0, 0), AND, YV],
  ["Pixel 9 Pro XL", 448, 997, 3, P(0, 0), AND, YV],
  ["Galaxy S24", 360, 780, 3, P(0, 0), AND, BL + " / " + YV],
  ["Galaxy S24 Ultra", 384, 824, 3.75, P(0, 0), AND, BL],
  ["Galaxy S25", 360, 780, 3, P(0, 0), AND, YV],
  ["Galaxy S25 Ultra", 412, 891, 3.5, P(0, 0), AND, YV],
  ["Galaxy Z Fold 7 (déplié)", 984, 1092, 2, P(0, 0), AND, YV],
  ["Galaxy Z Flip 7", 360, 840, 3, P(0, 0), AND, YV],
  ["Desktop 1366", 1366, 768, 1, P(0, 0), null, "écran courant"],
  ["Desktop 1080p", 1920, 1080, 1, P(0, 0), null, "écran courant"],
  ["Desktop 1440p", 2560, 1440, 1, P(0, 0), null, "écran courant"],
  ["Desktop 4K (×2)", 1920, 1080, 2, P(0, 0), null, "écran courant"],
];
const br = spawn("/opt/brave.com/brave/brave", ["--headless=new", `--remote-debugging-port=${port}`, `--user-data-dir=${prof}`, "--no-first-run", "--disable-gpu", "about:blank"], { stdio: "ignore" });
const sleep = (ms) => new Promise(r => setTimeout(r, ms));
let ok; for (let i = 0; i < 40 && !ok; i++) { await sleep(250); ok = await fetch(`http://127.0.0.1:${port}/json/version`).then(r => r.json()).catch(() => null); }
const tabs = await fetch(`http://127.0.0.1:${port}/json/list`).then(r => r.json());
const ws = new WebSocket(tabs.find(t => t.type === "page").webSocketDebuggerUrl); await new Promise(r => ws.onopen = r);
let id = 0; const pend = new Map(); ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && pend.has(d.id)) { pend.get(d.id)(d); pend.delete(d.id); } };
const send = (method, params = {}) => new Promise(r => { const i = ++id; pend.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const ev = async (e) => (await send("Runtime.evaluate", { expression: e, awaitPromise: true, returnByValue: true })).result?.result?.value;
const shot = async (n) => { const r = await send("Page.captureScreenshot", { format: "jpeg", quality: 70 }); fs.writeFileSync(`${out}/${n}.jpg`, Buffer.from(r.result.data, "base64")); };
await send("Page.enable"); await send("Runtime.enable");
const rows = [];
for (const [name, w, h, dpr, sa, ua, src] of D) {
  const mobile = !!ua;
  await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: dpr, mobile });
  await send("Emulation.setTouchEmulationEnabled", { enabled: mobile, maxTouchPoints: mobile ? 5 : 0 });
  await send("Emulation.setUserAgentOverride", { userAgent: ua || "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36" });
  await send("Emulation.setSafeAreaInsetsOverride", { insets: { top: sa.t, bottom: sa.b, left: sa.l, right: sa.r } });
  await send("Page.navigate", { url: `${base}/favicon.png` }); await sleep(400);
  await ev(`localStorage.clear(); localStorage.setItem('argos:indicators.json', JSON.stringify(['Rsi'])); 1`);
  await send("Page.navigate", { url: `${base}/app/?ex=hl&s=BTC&tf=1h` }); await sleep(7000);
  const m = await ev(`(() => { const c = document.querySelector('canvas'), r = c ? c.getBoundingClientRect() : null; return r ? { x: r.left, y: r.top, w: r.width, h: r.height, pw: c.width, ph: c.height, iw: innerWidth, ih: innerHeight } : null; })()`);
  // attendu : la chart va d'une zone sûre à l'autre (± 1 point)
  const near = (a, b) => Math.abs(a - b) <= 1;
  const okTop = m && near(m.y, sa.t), okBot = m && near(m.y + m.h, m.ih - sa.b), okL = m && near(m.x, sa.l), okR = m && near(m.x + m.w, m.iw - sa.r);
  const okPx = m && m.pw >= Math.round(m.w * Math.min(dpr, 2)) - 2;
  const slug = name.replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "").toLowerCase();
  await shot(slug);
  // menu Argos ▾ ouvert (toucher le logo) : capture
  if (mobile) { await send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: sa.l + 30, y: sa.t + 22 }] }); await send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] }); }
  else { await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 38, y: 18 }); await send("Input.dispatchMouseEvent", { type: "mousePressed", x: 38, y: 18, button: "left", clickCount: 1 }); await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: 38, y: 18, button: "left", clickCount: 1 }); await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 40, y: 20 }); }
  await sleep(900); await shot(slug + "-menu");
  const all = okTop && okBot && okL && okR && okPx;
  rows.push({ name, w, h, dpr, sa, src, m, okTop, okBot, okL, okR, okPx, all });
  console.log(`${all ? "OK" : "KO"}  ${name}  chart ${m ? `${Math.round(m.x)},${Math.round(m.y)} ${Math.round(m.w)}×${Math.round(m.h)} · ${m.pw}×${m.ph} px` : "—"}`);
}
fs.writeFileSync(`${out}/matrice.json`, JSON.stringify(rows, null, 1));
ws.close(); br.kill();
