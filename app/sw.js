/* Argos HSS Trading (/app/) : service worker de l'appli installable.
   - Réseau d'abord : on prend toujours la dernière version ; sans réseau, la dernière gardée (et une page hors ligne).
   - Seulement les fichiers de l'appli (/app/…) : jamais les données de marché, /api ni les autres sites.
   - VERSION est écrite à la construction (argos-outils version-app) : nouvelle version = nouveau cache. */
const VERSION = "__ARGOS_VERSION__";
const CACHE = "argos-app-" + VERSION;
const SHELL = ["/app/", "/app/accueil.js", "/app/synchro.js", "/app/zoom.js", "/app/reprise.js", "/app/pkg/argos.js", "/app/pkg/argos_bg.wasm", "/app/manifest.webmanifest", "/app/icon-192.png", "/app/icon-512.png", "/app/apple-touch-icon.png"];
const OFFLINE = `<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Argos hors ligne</title>
<body style="margin:0;display:grid;place-items:center;height:100vh;background:#131722;color:#d1d4dc;font:16px system-ui,sans-serif;text-align:center">
<div><p style="font-size:40px;margin:0">📡</p><p>Pas de réseau, et Argos n'est pas encore gardé sur cet appareil.</p><p><button onclick="location.reload()" style="font:inherit;padding:10px 18px;border-radius:8px;border:0;background:#2962ff;color:#fff">Réessayer</button></p></div>`;

self.addEventListener("install", (e) => {
  // 🔄 la nouvelle version remplace tout de suite l'ancienne (Safari gardait l'ancienne tant qu'un onglet restait ouvert)
  self.skipWaiting();
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).catch(() => {}));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k.startsWith("argos-app-") && k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});

// « Mettre à jour » : la page demande d'activer la version en attente ; « Vider le cache » : tout effacer
self.addEventListener("message", (e) => {
  if (e.data === "argos-update") self.skipWaiting();
  if (e.data === "argos-clear") e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k.startsWith("argos-app-")).map((k) => caches.delete(k)))));
});

self.addEventListener("fetch", (e) => {
  const req = e.request, url = new URL(req.url);
  if (req.method !== "GET" || url.origin !== self.location.origin || !url.pathname.startsWith("/app/") || url.pathname === "/app/version.json") return;
  e.respondWith(
    fetch(req)
      .then((res) => {
        if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(req, copy)).catch(() => {}); }
        return res;
      })
      // hors réseau : seulement la copie de CETTE version (jamais une plus vieille restée dans un autre cache)
      .catch(() => caches.open(CACHE).then((c) => c.match(req, { ignoreSearch: req.mode === "navigate" })).then((hit) => hit || (req.mode === "navigate" ? new Response(OFFLINE, { headers: { "Content-Type": "text/html; charset=utf-8" } }) : Response.error())))
  );
});
