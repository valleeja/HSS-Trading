// 🛡 Quality gate, in a real browser (Jee, 24 Nov 2026: « plus aucun lag », « net »): the web app built from this
// tree, on an iPhone 13 (390 × 844, ×3, by the finger), its candles made up (no request leaves the computer).
//  - at rest: the app draws at its full density (never blurred);
//  - a finger dragging, then pinching the chart for 4 s: mean time between two images (median and 90th percentile shown) and
//    time to draw one, compared with the reference measured on Jee's PC (tools/porte/navigateur.txt): more than 10 %
//    slower is red. The reference is committed with the code, like tests/references/vitesse.txt: it is written only with
//    ARGOS_VITESSE=maj (Jee's phrase); a missing reference is red (each worktree would else measure against itself).
//  - only the chart is measured (bug B-041, 25 Nov 2026: the welcome page of the first launch, left open, was measured
//    instead of it): before and after each measure the app must say that nothing covers the chart (`argosEcran`);
//    otherwise the measure is INVALID, neither green nor red, and nothing is compared nor written; what the app said
//    and the gate's touches (« déroulé ») are printed either way;
//  - the target, 60 images a second (16.7 ms), is written down; it is the requirement of the graphics-card engine
//    (C): the processor alone does not reach it today, the gate says so and does not pretend.
// usage (from rust/, the server of dist/ already started): node tools/porte/navigateur.mjs http://127.0.0.1:8012
import fs from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

// Playwright as tools/argos_web_shots.mjs finds it, or from NODE_PATH (a global install)
const { chromium, devices } = await import("playwright").catch(() => createRequire(import.meta.url)("playwright"));

const base = process.argv[2] || "http://127.0.0.1:8012";
const REF = new URL("./navigateur.txt", import.meta.url);
const MARGE = 1.1; // Jee's choice (25 Nov): 10 %, the level of lot B as the floor (noise measured: 9 % at most)
const CIBLE_MS = 1000 / 60;

// made-up candles, the same at each run (a seeded walk), for any Binance kline request of the app
function bougies(n, iv, fin) {
  let seed = 12345;
  const rnd = () => ((seed = (seed * 1103515245 + 12345) % 2147483648), seed / 2147483648);
  const out = [];
  let p = 60000;
  for (let i = 0; i < n; i++) {
    const t = fin - (n - i) * iv, o = p, c = o * (1 + Math.sin(i / 7) * 0.004 + (rnd() - 0.5) * 0.006);
    const h = Math.max(o, c) * (1 + rnd() * 0.003), l = Math.min(o, c) * (1 - rnd() * 0.003);
    out.push([t, o.toFixed(1), h.toFixed(1), l.toFixed(1), c.toFixed(1), (100 + rnd() * 500).toFixed(3), t + iv - 1, "0", 100, "50", "0", "0"]);
    p = c;
  }
  return out;
}
const IV = { "1m": 6e4, "5m": 3e5, "15m": 9e5, "30m": 18e5, "1h": 36e5, "4h": 144e5, "1d": 864e5 };

const b = await chromium.launch();
const ctx = await b.newContext({ ...devices["iPhone 13"], deviceScaleFactor: 3, serviceWorkers: "block" });
await ctx.route("**/*", (r) => {
  const u = r.request().url();
  if (u.startsWith(base)) return r.continue();
  const m = /fapi\.binance\.com\/fapi\/v1\/klines\?symbol=([A-Z]+)&interval=(\w+)&limit=(\d+)(&endTime=(\d+))?/.exec(u);
  if (m) {
    const iv = IV[m[2]] || 18e5, fin = m[5] ? +m[5] : Date.now(), n = m[5] ? 0 : +m[3];
    return r.fulfill({ status: 200, contentType: "application/json", headers: { "access-control-allow-origin": "*" }, body: JSON.stringify(bougies(n, iv, Math.floor(fin / iv) * iv + iv)) });
  }
  return r.abort();
});
await ctx.addInitScript(() => {
  try { localStorage.setItem("argos-direct", "1"); } catch (e) {}
  window.__put = [];
  // each time the app reports its drawing time (a running mean, every 20 images), kept to take the gesture's own
  let rendu;
  window.__ms = [];
  Object.defineProperty(window, "argosRender", { configurable: true, get: () => rendu, set: (v) => { rendu = v; window.__ms.push(v.ms); } });
  // 🛡 B-041: each time the app says what covers the chart, and each touch of the gate (seconds since the page opened):
  // told with a measure found invalid, to see what the touches did
  window.__ecrans = [];
  let ecran;
  Object.defineProperty(window, "argosParDessus", { configurable: true, get: () => ecran, set: (v) => { ecran = v; window.__ecrans.push([performance.now(), v]); } });
  const o = CanvasRenderingContext2D.prototype.putImageData;
  CanvasRenderingContext2D.prototype.putImageData = function () { window.__put.push(performance.now()); return o.apply(this, arguments); };
});
const p = await ctx.newPage();
const erreurs = [];
p.on("pageerror", (e) => erreurs.push(e.message));
await p.goto(`${base}/app/`);
await p.waitForTimeout(3000);
const btn = p.getByText("Commencer à charter");
if (await btn.count()) await btn.first().click();
await p.waitForTimeout(8000);
if (!(await p.$("canvas"))) {
  await b.close();
  console.log(`⛔ porte, navigateur : l'app ne s'est pas ouverte sur ${base}/app/ (aucun dessin)${erreurs.length ? " : " + erreurs.join(" ; ") : ""}`);
  process.exit(1);
}
// two touches as a person makes them (the market sheet at the bottom, then the top bar): the screen at rest after.
// (B-041) Each one is played in the page, its down and its lift in the same task, as the gestures below: the test
// driver's touch (Input.dispatchTouchEvent) waits for the page to have handled the down before it can send the lift,
// so a page busy drawing right after the down held the « finger » down more than half a second: a long press, the tap
// lost, the welcome window left open (Jee's PC, 25 Nov 2026; in the cloud with the processor slowed 8 times: a 3 s press)
const toucher = async (x, y) => {
  await p.evaluate(([x, y]) => {
    window.__ecrans.push([performance.now(), `👆 ${x},${y}`]);
    const c = document.querySelector("canvas");
    const ev = (type) => new PointerEvent(type, { pointerType: "touch", pointerId: 1, isPrimary: true, clientX: x, clientY: y, bubbles: true, cancelable: true, button: 0, buttons: type === "pointerdown" ? 1 : 0 });
    c.dispatchEvent(ev("pointerdown"));
    window.dispatchEvent(ev("pointerup"));
  }, [x, y]);
  await p.waitForTimeout(400);
};
await toucher(255, 521);
await p.waitForTimeout(800);
await toucher(362, 22);

// a gesture played in the page at 60 Hz (pointer events of a finger, without the latency of the test driver)
const jouer = (piste) => p.evaluate((piste) => new Promise((fini) => {
  const c = document.querySelector("canvas");
  const tir = (type, id, x, y) => (type === "pointerdown" ? c : window).dispatchEvent(new PointerEvent(type, { pointerType: "touch", pointerId: id, isPrimary: id === 1, clientX: x, clientY: y, bubbles: true, cancelable: true, buttons: type === "pointerup" ? 0 : 1 }));
  let i = 0;
  const t0 = performance.now();
  const pas = () => {
    const t = performance.now() - t0;
    while (i < piste.length && piste[i][0] <= t) { const [, type, id, x, y] = piste[i]; tir(type, id, x, y); i++; }
    if (i < piste.length) setTimeout(pas, 4); else setTimeout(fini, 300);
  };
  pas();
}), piste);
const glisser = (x0, y0, dx, ms) => { const t = [[0, "pointerdown", 1, x0, y0]]; for (let k = 16; k <= ms; k += 16) t.push([k, "pointermove", 1, x0 + dx * Math.sin((k / ms) * Math.PI * 2), y0]); t.push([ms + 16, "pointerup", 1, x0, y0]); return t; };
const pincer = (cx, cy, ms) => { const t = [[0, "pointerdown", 1, cx - 40, cy], [0, "pointerdown", 2, cx + 40, cy]]; for (let k = 16; k <= ms; k += 16) { const d = 40 + 60 * Math.abs(Math.sin((k / ms) * Math.PI * 2)); t.push([k, "pointermove", 1, cx - d, cy], [k, "pointermove", 2, cx + d, cy]); } t.push([ms + 16, "pointerup", 1, cx - 40, cy], [ms + 16, "pointerup", 2, cx + 40, cy]); return t; };
const mesurer = async (piste) => {
  await p.evaluate(() => { window.__put = []; window.__ms = []; });
  await jouer(piste);
  const r = await p.evaluate(() => ({ put: window.__put, ms: window.__ms.slice() }));
  const ecarts = r.put.slice(1).map((x, i) => x - r.put[i]).filter((e) => e < 500).sort((a, c) => a - c);
  const q = (k) => (ecarts.length ? ecarts[Math.min(ecarts.length - 1, Math.floor(ecarts.length * k))] : Infinity);
  // the mean, not the median: the browser shows an image at a screen refresh (16.7 ms), so the gaps are 16.7 or 33.3 ms
  // and the median jumps from one to the other; the mean moves smoothly with the real speed
  const moyen = ecarts.length ? ecarts.reduce((a, e) => a + e, 0) / ecarts.length : Infinity;
  // drawing time: the median of the app's reports during the gesture (not the last one, nor the rest after it)
  const ms = r.ms.length ? r.ms.slice().sort((a, c) => a - c)[Math.floor(r.ms.length / 2)] : NaN;
  return { moyen, median: q(0.5), p90: q(0.9), images: r.put.length, ms };
};

// 🛡 only the chart is measured (B-041): the app tells the page what covers the chart (`argosEcran`: « » = nothing);
// anything else, or no answer, makes the measure invalid: neither green nor red, no reference written
const deroule = (h) => (h || []).map(([t, v]) => `${(t / 1000).toFixed(1).replace(".", ",")} s ${v.startsWith("👆") ? v : `« ${v} »`}`).join(" · ") || "rien";
const surLaChart = async (quand) => {
  const [e, h] = await p.evaluate(() => [window.argosParDessus, window.__ecrans]);
  if (e === "") return;
  await b.close();
  const quoi = e === undefined || e === null ? "l'app ne dit pas ce qui couvre la chart (argosEcran)" : `« ${e} » par-dessus la chart`;
  console.log(`⛔ porte, navigateur : mesure invalide ${quand} : ${quoi} ; ni vert ni rouge, rien n'est comparé ni écrit`);
  console.log(`   déroulé : ${deroule(h)}`);
  process.exit(3);
};

// at rest: the full density of the screen (iPhone 13 ×3 at the app's compact zoom 0.8: ×2.4)
await p.waitForTimeout(3000);
await surLaChart("avant le repos");
const repos = await p.evaluate(() => window.argosRender);
// each gesture played three times, the best of each figure kept: a computer busy elsewhere for a moment does not
// make the gate red, and a real slowdown shows in every pass
const trois = async (piste, nom) => {
  await surLaChart(`avant ${nom}`);
  const r = [await mesurer(piste), await mesurer(piste), await mesurer(piste)];
  await surLaChart(`après ${nom}`);
  const mieux = r.reduce((a, c) => (c.moyen < a.moyen ? c : a));
  return { ...mieux, ms: Math.min(...r.map((x) => (x.ms > 0 ? x.ms : Infinity))), passes: r };
};
const lire = () => {
  try {
    return Object.fromEntries(fs.readFileSync(REF, "utf8").split("\n").filter((l) => l && !l.startsWith("#")).map((l) => { const [n, moy, ms] = l.split(/\s+/); return [n, { moyen: +moy, ms: +ms }]; }));
  } catch { return null; }
};
const ref = lire();
const pistes = { glisser: glisser(250, 250, -120, 4000), pincer: pincer(200, 260, 4000) };
const lent = (nom, m) => ref && ref[nom] && (m.moyen > ref[nom].moyen * MARGE || m.ms > ref[nom].ms * MARGE);
const mesures = {};
for (const [nom, piste] of Object.entries(pistes)) {
  mesures[nom] = await trois(piste, nom);
  // slower than the reference: measured three times again before saying so (a real slowdown stays, a moment of a
  // busy computer goes)
  if (process.env.ARGOS_VITESSE !== "maj" && lent(nom, mesures[nom])) {
    const encore = await trois(piste, nom);
    console.log(`🛡 navigateur : ${nom} plus lent que la référence, mesuré une deuxième fois`);
    mesures[nom] = { ...(encore.moyen < mesures[nom].moyen ? encore : mesures[nom]), ms: Math.min(encore.ms, mesures[nom].ms) };
  }
}
// (B-041) what the app said covered the chart, and the gate's touches, from the opening to the last measure: printed
// with every measure, the proof that the gestures were played on the chart alone
const fil = await p.evaluate(() => window.__ecrans);
await b.close();

const rouge = [];
const scale = repos?.scale ?? 0;
console.log(`🛡 navigateur : la chart seule avant et après chaque mesure ; déroulé : ${deroule(fil)}`);
console.log(`🛡 navigateur : au repos ×${scale.toFixed(2)}`);
if (!(scale >= 2.4 - 0.01)) rouge.push(`au repos, l'app dessine à ×${scale.toFixed(2)} au lieu de ×2,4 : floue`);
for (const [nom, m] of Object.entries(mesures)) {
  console.log(`🛡 navigateur : ${nom} ${(1000 / m.moyen).toFixed(0)} images/s (écart moyen ${m.moyen.toFixed(1)} ms, médian ${m.median.toFixed(1)} ms, p90 ${m.p90.toFixed(1)} ms), dessin ${m.ms.toFixed(1)} ms ; cible ${CIBLE_MS.toFixed(1)} ms (60 i/s, moteur C)`);
  if (m.images < 20) rouge.push(`${nom} : ${m.images} images seulement en 4 s`);
  if (!(m.ms > 0)) rouge.push(`${nom} : temps de dessin non mesuré (l'app ne l'a pas donné pendant le geste)`);
}
if (erreurs.length) rouge.push(`erreurs de la page : ${erreurs.join(" ; ")}`);

if (!ref && process.env.ARGOS_VITESSE !== "maj") {
  rouge.push(`pas de référence (${fileURLToPath(REF)}) : elle s'écrit avec ARGOS_VITESSE=maj, sur le PC de Jee et avec sa phrase, puis se commite`);
} else if (process.env.ARGOS_VITESSE === "maj") {
  const t = ["# Written by tools/porte/navigateur.mjs with ARGOS_VITESSE=maj (Jee's phrase), never by hand.", "# gesture, mean time between two images (ms), time to draw one (ms)"];
  for (const [nom, m] of Object.entries(mesures)) t.push(`${nom} ${m.moyen.toFixed(1)} ${m.ms.toFixed(1)}`);
  fs.writeFileSync(REF, t.join("\n") + "\n");
  console.log(`🛡 navigateur : référence écrite (${fileURLToPath(REF)})`);
} else {
  for (const [nom, m] of Object.entries(mesures)) {
    const r = ref[nom];
    if (!r) { rouge.push(`${nom} : pas de référence (ARGOS_VITESSE=maj, avec la phrase de Jee)`); continue; }
    if (m.moyen > r.moyen * MARGE) rouge.push(`${nom} : ${((m.moyen / r.moyen - 1) * 100).toFixed(0)} % plus lent entre deux images (${m.moyen.toFixed(1)} ms au lieu de ${r.moyen.toFixed(1)})`);
    if (m.ms > r.ms * MARGE) rouge.push(`${nom} : dessin ${((m.ms / r.ms - 1) * 100).toFixed(0)} % plus lent (${m.ms.toFixed(1)} ms au lieu de ${r.ms.toFixed(1)})`);
  }
}
if (rouge.length) {
  console.log("⛔ porte, navigateur :\n  " + rouge.join("\n  "));
  process.exit(1);
}
console.log("🛡 navigateur : vert");
