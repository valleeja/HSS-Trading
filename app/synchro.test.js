// Tests de la session partagée de la page (node app/synchro.test.js) : lancés par `cargo test` (argos-desktop).
const assert = require("node:assert");
const { creer, nomPermis, fetchCoque } = require("./synchro.js");

function env(reponse) {
  const store = new Map();
  const envois = [];
  const minuteurs = [];
  return {
    store, envois, minuteurs,
    storage: { setItem: (k, v) => store.set(k, v), getItem: (k) => (store.has(k) ? store.get(k) : null), removeItem: (k) => store.delete(k) },
    setTimeout: (f) => { minuteurs.push(f); return minuteurs.length; },
    clearTimeout: (k) => { if (k) minuteurs[k - 1] = null; },
    fetch: async (url, opt = {}) => {
      if (opt.method === "PUT") { envois.push([url, opt.body]); return { ok: true }; }
      if (reponse instanceof Error) throw reponse;
      return { ok: reponse.ok !== false, json: async () => reponse.json };
    },
    tick() { const f = minuteurs.splice(0); f.forEach((x) => x && x()); },
  };
}

(async () => {
  // noms : session seulement, jamais compte / clés / portefeuille ni sous-dossiers
  for (const ok of ["split.json", "dock.json", "hyperliquid-btc.json", "indicator_inputs.json"]) assert.ok(nomPermis(ok), ok);
  for (const non of ["account.json", "passkey.json", "claude-key.json", "account-key.json", "wallet.json", "lang/de.json", "modules/x.wasm.b64", "../x.json", "x.txt", ""]) assert.ok(!nomPermis(non), non);

  // serveur local : les réglages du bureau remplacent ceux du navigateur (sauf un nom interdit glissé dans la réponse)
  let e = env({ json: { fichiers: { "dock.json": '{"on":true}', "account.json": "{}", "x.json": 3 } } });
  let S = creer(e);
  assert.strictEqual(S.ecrire("dock.json", "{}"), false, "rien n'est envoyé avant le chargement");
  assert.strictEqual(await S.charger(), 1);
  assert.strictEqual(e.store.get("argos:dock.json"), '{"on":true}');
  assert.ok(!e.store.has("argos:account.json"));
  // écritures regroupées : seule la dernière part
  assert.ok(S.ecrire("dock.json", '{"on":false}'));
  assert.ok(S.ecrire("dock.json", '{"on":true,"pos":1}'));
  assert.ok(!S.ecrire("account.json", "{}"), "jamais le compte");
  assert.ok(S.enCours(), "💾 Sauvegarde… tant que rien n'est parti");
  e.tick();
  await new Promise((ok) => setImmediate(ok));
  assert.ok(!S.enCours(), "✓ Sauvegardé une fois reçu");
  assert.deepStrictEqual(e.envois, [["/api/reglages/dock.json", '{"on":true,"pos":1}']]);
  // la page se ferme : ce qui attend part tout de suite
  S.ecrire("split.json", '{"n":2}');
  S.vider();
  assert.deepStrictEqual(e.envois[1], ["/api/reglages/split.json", '{"n":2}']);

  // 🔄 en direct : ce qui a changé ailleurs est appliqué ; nos propres écritures (identiques) et un réglage qui
  // attend encore d'être envoyé d'ici sont laissés ; un retiré disparaît
  e = env({ json: { e: "ep1", v: 3, fichiers: { "dock.json": "{}", "split.json": '{"n":1}' } } });
  S = creer(e);
  await S.charger();
  assert.deepStrictEqual([S.e, S.v], ["ep1", 3]);
  S.ecrire("alerts.json", "[1]");
  const noms = S.appliquer({ e: "ep1", v: 7, fichiers: { "dock.json": "{}", "split.json": '{"n":4}', "alerts.json": "[]", "account.json": "{}" }, retires: ["dock.json", "passkey.json"] });
  assert.deepStrictEqual(noms, ["split.json", "dock.json"]);
  assert.strictEqual(e.store.get("argos:split.json"), '{"n":4}');
  assert.ok(!e.store.has("argos:dock.json") && !e.store.has("argos:account.json"));
  assert.strictEqual(S.v, 7);
  // la boucle : une réponse, puis l'app est réveillée avec les noms changés
  let reveils = [];
  e.fetch = async () => ({ ok: true, json: async () => ({ e: "ep1", v: 8, fichiers: { "rail.json": '{"x":1}' } }) });
  await S.suivre((n) => reveils.push(n), 1);
  assert.deepStrictEqual(reveils, ["rail.json"]);
  assert.strictEqual(S.v, 8);

  // sans serveur (site en ligne, coque, hors ligne) : rien ne change, rien n'est envoyé
  for (const r of [new Error("réseau"), { ok: false }, { json: null }, { json: { autre: 1 } }]) {
    e = env(r);
    S = creer(e);
    assert.strictEqual(await S.charger(), -1);
    assert.strictEqual(S.ecrire("dock.json", "{}"), false);
    assert.strictEqual(e.store.size, 0);
  }
  // 📱 coque : les mêmes questions vont au cœur Rust (invoke)
  const appels = [];
  const f = fetchCoque(async (cmd, args) => { appels.push([cmd, args]); return cmd === "reglage_ecrire" ? 9 : { e: "c", v: 1, fichiers: {} }; });
  assert.deepStrictEqual(await (await f("/api/reglages", { cache: "no-store" })).json(), { e: "c", v: 1, fichiers: {} });
  assert.deepStrictEqual(await (await f("/api/reglages/dock.json", { method: "PUT", body: "{}" })).json(), { v: 9 });
  await f("/api/reglages/changements?e=c&depuis=4&attente=25");
  assert.deepStrictEqual(appels, [["reglages_tout", undefined], ["reglage_ecrire", { nom: "dock.json", texte: "{}" }], ["reglages_changements", { e: "c", depuis: 4 }]]);
  assert.strictEqual((await f("/autre")).ok, false);
  console.log("synchro.js : ok");
})().catch((x) => { console.error(x); process.exit(1); });
