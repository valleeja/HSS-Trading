// 📸 Vérifie l'app web (WASM) dans un vrai Chromium : ouvre /app sur plusieurs exchanges en 1 minute, attend les
// bougies et le direct, prend des captures (desktop + cell) et note les erreurs de la console dans shots/report.txt.
import { chromium } from "playwright";
import fs from "node:fs";

const base = process.argv[2] || "http://localhost:8000";
fs.mkdirSync("shots", { recursive: true });
const report = [];
const browser = await chromium.launch();
const cases = [
  ["hl", "BTC", "15m", { width: 1400, height: 800 }],
  ["binance", "ETH", "1h", { width: 1400, height: 800 }],
  ["hl", "BTC", "4h", { width: 390, height: 844, isMobile: true, hasTouch: true, deviceScaleFactor: 3 }],
  // 🧩 colonne de modules (grand écran) : à droite, à gauche, sous la chart
  ...["Right", "Left", "Bottom"].map(pos => ["hl", "BTC", "1h", { width: 1600, height: 900, dock: pos }]),
];
for (const [ex, s, tf, vp] of cases) {
  const ctx = await browser.newContext({ viewport: { width: vp.width, height: vp.height }, isMobile: !!vp.isMobile, hasTouch: !!vp.hasTouch, deviceScaleFactor: vp.deviceScaleFactor || 1 });
  // modules de chaque capture : droite = watchlist, compte, positions, baleines ; bas = agenda, news, marché ; gauche = carnet, trades, funding
  const DOCK = { Right: [["Watchlist", "Medium"], ["Account", "Small"], ["Positions", "Small"], ["Whales", "Fill"]], Bottom: [["World", "Fill"], ["Sentiment", "Medium"], ["Liquidations", "Medium"]], Left: [["Book", "Fill"], ["Trades", "Medium"], ["Funding", "Small"]] };
  if (vp.dock) await ctx.addInitScript(([pos, mods]) => localStorage.setItem("argos:dock.json", JSON.stringify({ on: true, pos, size: 1, items: mods.map(([module, size]) => ({ module, on: true, size })) })), [vp.dock, DOCK[vp.dock]]);
  const page = await ctx.newPage();
  const logs = [];
  page.on("console", m => { if (m.type() === "error" || m.type() === "warning") logs.push(`${m.type()}: ${m.text()}`); });
  page.on("pageerror", e => logs.push(`pageerror: ${e.message}`));
  const t0 = Date.now();
  await page.goto(`${base}/app/?ex=${ex}&s=${s}&tf=${tf}`, { waitUntil: "load" });
  await page.waitForTimeout(15000);
    const name = `${ex}-${s}-${tf}-${vp.width}${vp.dock ? "-dock-" + vp.dock : ""}`;
  await page.screenshot({ path: `shots/${name}.png` });
  // 📱 cell : chaque bouton touché, une capture de sa fenêtre (comme les captures de la v1), puis on la referme
  if (vp.isMobile) {
    const W = vp.width, H = vp.height;
    const taps = [["panneaux", 60, H - 28], ["watchlist", 30, H - 78], ["argos", 36, 24], ["reglages", W - 20, 24], ["wallet", W - 84, 24], ["vue", 210, 24], ["plus", W - 24, H - 78], ["cmc", W - 120, H - 28], ["market", W - 170, H - 28], ["ai", W - 40, H - 28]];
    for (const [n, x, y] of taps) {
      await page.touchscreen.tap(x, y);
      await page.waitForTimeout(1200);
      await page.screenshot({ path: `shots/${name}-${n}.png` });
      await page.keyboard.press("Escape");
      await page.touchscreen.tap(W / 2, 300); // à côté : fermer
      await page.waitForTimeout(400);
      if (n === "watchlist" || n === "cmc") { await page.touchscreen.tap(x, y); await page.waitForTimeout(400); }
    }
  }
  report.push(`${name} (${Date.now() - t0} ms)\n  ${logs.join("\n  ") || "aucune erreur"}`);
  await ctx.close();
}
await browser.close();
fs.writeFileSync("shots/report.txt", report.join("\n") + "\n");
console.log(report.join("\n"));
