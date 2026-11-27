// 📐 Mesure de la maquette (croquis/maquette-verre) : à coller dans la console de la page /maquette/ ouverte à la
// taille voulue (cell : 390 × 844). Rend un JSON : pour chaque élément de l'écran (rangée du haut, pilules, dock,
// DÉMO, Flux, Argos AI, barre des favoris…) sa boîte, son rayon, ses couleurs, son libellé et ses enfants.
// Le résultat va dans rust/crates/argos-render/src/ecrans/maquette-<vue>.json (versionné) ; le Rust le lit pour
// dessiner et pour le toucher. Ne modifie rien dans la page.
(() => {
  const BLOCS = ["thaut", "demoband", "flux", "favbar", "pg", "pd", "aipill", "icones"];
  const arrondi = (v) => Math.round(v * 10) / 10;
  const boite = (e) => {
    const r = e.getBoundingClientRect();
    return [arrondi(r.x), arrondi(r.y), arrondi(r.width), arrondi(r.height)];
  };
  const visible = (e) => {
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    return r.width > 0 && r.height > 0 && s.display !== "none" && s.visibility !== "hidden" && s.opacity !== "0";
  };
  const style = (e) => {
    const s = getComputedStyle(e);
    return {
      rayon: parseFloat(s.borderTopLeftRadius) || 0,
      fond: s.backgroundColor,
      bord: s.borderTopWidth !== "0px" ? s.borderTopColor : null,
      texte: s.color,
      taille: parseFloat(s.fontSize),
      graisse: s.fontWeight,
    };
  };
  const libelle = (e) => (e.childElementCount ? [...e.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent).join("") : e.textContent).trim();
  const zone = (e, id) => ({ id, boite: boite(e), ...style(e), libelle: libelle(e), titre: e.title || null });
  const out = { vue: `${innerWidth}x${innerHeight}`, ech: getComputedStyle(document.documentElement).getPropertyValue("--ech").trim(), zones: [] };
  for (const id of BLOCS) {
    const b = document.getElementById(id);
    if (!b || !visible(b)) continue;
    const z = zone(b, id);
    // (les enfants hors de l'écran — fin de la rangée d'icônes qui défile — ne comptent pas)
    const dedans = (c) => c.getBoundingClientRect().right <= innerWidth + 0.5;
    z.enfants = [...b.querySelectorAll("button, hr, .roue")].filter(visible).map((c, k) => [c, k]).filter(([c]) => dedans(c)).map(([c, k]) => {
      const e = zone(c, `${id}.${k}`);
      e.balise = c.tagName.toLowerCase();
      if (c.classList.length) e.classe = c.className;
      const i = c.querySelector("i");
      if (i && i.textContent.trim()) e.pastille = i.textContent.trim();
      return e;
    });
    out.zones.push(z);
  }
  return JSON.stringify(out, null, 1);
})();
