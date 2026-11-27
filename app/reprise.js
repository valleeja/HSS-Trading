// 🔄 Reprise toute seule : la page n'a pas pu démarrer (le PC ne répondait pas, ou ses pages se reconstruisaient).
// Elle réessaie toutes les 4 s et se recharge dès que version.json ET le moteur (pkg/argos_bg.wasm) répondent.
// (En JS : c'est justement quand le moteur Rust n'a pas pu démarrer.)
(function () {
  let actif = false;
  const essai = async () => {
    try {
      const v = await fetch("/app/version.json", { cache: "no-store" });
      const w = await fetch("/app/pkg/argos_bg.wasm", { method: "HEAD", cache: "no-store" });
      if (v.ok && w.ok) return location.reload();
    } catch (_) {}
    setTimeout(essai, 4000);
  };
  window.argosReprise = { demarrer: () => { if (!actif) { actif = true; setTimeout(essai, 4000); } } };
})();
