// 🔄 Session partagée avec l'app de bureau (seulement avec le serveur local : make web, cet ordi ou Tailscale).
// - Au démarrage, avant la chart : les réglages de l'app de bureau (/api/reglages) remplacent ceux de ce navigateur
//   (dessins, split, colonne, indicateurs, alertes, journal…). Sans serveur (site en ligne, coque, hors ligne) : rien
//   ne change, le stockage du navigateur sert comme avant.
// - Ensuite, chaque réglage changé ici part au serveur (regroupé 0,4 s ; tout part quand la page se ferme). Le
//   stockage du navigateur reste comme copie hors ligne.
// - En direct : la page demande au serveur ce qui a changé ailleurs (attente jusqu'à 25 s) et réveille l'app, qui
//   relit ces réglages (argosSynchroReveil). Ses propres écritures reviennent identiques : ignorées.
// - Jamais de compte, de clé ni de portefeuille (le serveur les refuse aussi).
(function (racine) {
  // 🔐 journal.json : chiffré sur le PC (base sensible), il ne voyage plus en clair
  const INTERDITS = new Set(["account.json", "passkey.json", "claude-key.json", "account-key.json", "wallet.json", "cmc_credits.json", "journal.json"]);
  const nomPermis = (nom) => typeof nom === "string" && nom.length <= 100 && /^[A-Za-z0-9][A-Za-z0-9_-]*\.json$/.test(nom) && !INTERDITS.has(nom);

  function creer(env) {
    const S = { actif: false, attente: new Map(), minuteurs: new Map(), e: "", v: 0, suivi: false, envois: 0, reveil: null };
    // 💾 « Sauvegarde… » tant qu'un réglage n'est pas reçu par le serveur
    S.enCours = () => S.attente.size > 0 || S.envois > 0;
    // ⚡ tout charger d'un coup (3 s au plus) ; rend le nombre de réglages reçus, ou -1 sans serveur
    S.charger = async () => {
      try {
        const c = new AbortController();
        const fin = env.setTimeout(() => c.abort(), 3000);
        const r = await env.fetch("/api/reglages", { cache: "no-store", signal: c.signal });
        env.clearTimeout(fin);
        if (!r.ok) return -1;
        const j = await r.json();
        if (!j || typeof j.fichiers !== "object" || j.fichiers === null) return -1;
        let n = 0;
        for (const [nom, texte] of Object.entries(j.fichiers)) {
          if (nomPermis(nom) && typeof texte === "string") {
            try { env.storage.setItem("argos:" + nom, texte); n++; } catch (_) { /* stockage plein : on garde l'ancien */ }
          }
        }
        S.e = typeof j.e === "string" ? j.e : "";
        S.v = Number.isFinite(j.v) ? j.v : 0;
        S.actif = true;
        return n;
      } catch (_) {
        return -1;
      }
    };
    const envoyer = (nom) => {
      const texte = S.attente.get(nom);
      S.attente.delete(nom);
      S.minuteurs.delete(nom);
      if (texte === undefined) return;
      S.envois++;
      const fini = () => { S.envois--; if (S.reveil) S.reveil(""); };
      env.fetch("/api/reglages/" + encodeURIComponent(nom), {
        method: "PUT",
        body: texte,
        headers: { "Content-Type": "application/json" },
        keepalive: texte.length < 60000,
      }).then(fini, fini);
    };
    // ✍️ appelé par l'app (platform::write) à chaque réglage écrit
    S.ecrire = (nom, texte) => {
      if (!S.actif || !nomPermis(nom) || typeof texte !== "string") return false;
      S.attente.set(nom, texte);
      env.clearTimeout(S.minuteurs.get(nom));
      S.minuteurs.set(nom, env.setTimeout(() => envoyer(nom), 400));
      return true;
    };
    // 🔄 applique une réponse de /api/reglages/changements ; rend les noms vraiment changés (pas nos propres
    // écritures, pas un réglage qui attend encore d'être envoyé d'ici)
    S.appliquer = (j) => {
      if (!j || typeof j !== "object") return [];
      if (typeof j.e === "string") S.e = j.e;
      if (Number.isFinite(j.v)) S.v = j.v;
      const noms = [];
      for (const [nom, texte] of Object.entries(j.fichiers || {})) {
        if (!nomPermis(nom) || typeof texte !== "string" || S.attente.has(nom)) continue;
        let avant = null;
        try { avant = env.storage.getItem("argos:" + nom); } catch (_) { /* bloqué */ }
        if (avant === texte) continue;
        try { env.storage.setItem("argos:" + nom, texte); noms.push(nom); } catch (_) { /* plein */ }
      }
      for (const nom of Array.isArray(j.retires) ? j.retires : []) {
        if (!nomPermis(nom) || S.attente.has(nom)) continue;
        try { env.storage.removeItem("argos:" + nom); noms.push(nom); } catch (_) { /* bloqué */ }
      }
      return noms;
    };
    // 🔁 suit les changements tant que la page est ouverte (une question à la fois ; 5 s de pause après une erreur)
    S.suivre = async (reveil, tours = Infinity) => {
      if (!S.actif || S.suivi) return;
      S.suivi = true;
      S.reveil = reveil;
      for (let k = 0; k < tours; k++) {
        try {
          const r = await env.fetch(`/api/reglages/changements?e=${encodeURIComponent(S.e)}&depuis=${S.v}&attente=25`, { cache: "no-store" });
          if (!r.ok) throw new Error("HTTP " + r.status);
          const noms = S.appliquer(await r.json());
          if (noms.length && reveil) reveil(noms.join("\n"));
        } catch (_) {
          await new Promise((ok) => env.setTimeout(ok, 5000));
        }
      }
      S.suivi = false;
    };
    // 🚪 la page se ferme : tout ce qui attend part tout de suite
    S.vider = () => {
      for (const nom of [...S.attente.keys()]) {
        env.clearTimeout(S.minuteurs.get(nom));
        envoyer(nom);
      }
    };
    return S;
  }

  if (typeof module !== "undefined") module.exports = { creer, nomPermis };
  // 📱 coque Tauri : pas de serveur ; les mêmes questions vont au cœur Rust (mêmes règles, mêmes fichiers)
  function fetchCoque(invoke) {
    const rep = (j) => ({ ok: true, json: async () => j });
    return async (url, opt = {}) => {
      const u = new URL(url, "http://coque");
      if (opt.method === "PUT") {
        const nom = decodeURIComponent(u.pathname.slice("/api/reglages/".length));
        return rep({ v: await invoke("reglage_ecrire", { nom, texte: opt.body }) });
      }
      if (u.pathname === "/api/reglages/changements") {
        return rep(await invoke("reglages_changements", { e: u.searchParams.get("e") || "", depuis: Number(u.searchParams.get("depuis")) || 0 }));
      }
      if (u.pathname === "/api/reglages") return rep(await invoke("reglages_tout"));
      return { ok: false, json: async () => null };
    };
  }

  if (typeof module !== "undefined") module.exports.fetchCoque = fetchCoque;
  if (racine && racine.fetch && racine.localStorage) {
    let storage = null;
    try { storage = racine.localStorage; } catch (_) { /* stockage bloqué */ }
    if (!storage) return;
    const ipc = racine.__TAURI_INTERNALS__ && racine.__TAURI_INTERNALS__.invoke;
    const fetch = ipc ? fetchCoque(ipc.bind(racine.__TAURI_INTERNALS__)) : racine.fetch.bind(racine);
    const S = creer({ fetch, storage, setTimeout: racine.setTimeout.bind(racine), clearTimeout: racine.clearTimeout.bind(racine) });
    racine.argosSynchro = S;
    racine.argosSynchroEcrire = S.ecrire;
    racine.addEventListener("pagehide", S.vider);
    // le direct démarre quand l'app est prête à être réveillée
    const attendre = () => (racine.argosSynchroReveil ? S.suivre((noms) => racine.argosSynchroReveil(noms)) : racine.setTimeout(attendre, 500));
    racine.argosSynchroDirect = () => { if (S.actif) attendre(); };
  }
})(typeof window !== "undefined" ? window : null);
