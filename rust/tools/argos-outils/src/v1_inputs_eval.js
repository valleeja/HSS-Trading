
const fs = require('fs');
const raw = JSON.parse(fs.readFileSync(0, 'utf8'));
const mk = (k, g, label, def, extra) => Object.assign({ k, g, label, def, type: typeof def === "boolean" ? "bool" : typeof def === "string" ? (/^#[0-9a-f]{6}$/i.test(def) ? "col" : "sel") : "num" }, extra || {});
const sel = (k, g, l, d, o) => mk(k, g, l, d, { type: "sel", opts: o });
const P = mk, P2 = mk, P3 = mk, P4 = mk, SEL = sel, S3 = sel, S4 = sel;
__CONSTS__
const out = {};
for (const [id, v] of Object.entries(raw)) {
  const groups = eval(v.groups), params = eval(v.params);
  // comme nvTblParams de v1 : position / taille (et « afficher dans » pour un oscillateur) après l'input du tableau
  const i = params.findIndex(x => ["showTable", "htf_box", "dash"].includes(x.k));
  if (i >= 0) {
    const g = params[i].g, add = [{ k: "tblPos", g, label: "Table : position", def: v.tblDef || "tr", type: "sel", opts: [["tr", "Top Right"], ["tl", "Top Left"], ["tc", "Top Center"], ["mr", "Middle Right"], ["ml", "Middle Left"], ["br", "Bottom Right"], ["bl", "Bottom Left"], ["bc", "Bottom Center"]] },
      { k: "tblSize", g, label: "Table : taille", def: "small", type: "sel", opts: [["tiny", "Tiny"], ["small", "Petit"], ["normal", "Normal"], ["large", "Grand"], ["huge", "Énorme"]] }];
    if (v.pane === "sub") add.push({ k: "tblIn", g, label: "Table : afficher dans", def: "chart", type: "sel", opts: [["chart", "La chart"], ["pane", "Le panneau de l'indicateur"]] });
    params.splice(i + 1, 0, ...add);
  }
  const f = (k) => params.find(x => x.k === k);
  if (id === "AI_TREND") { if (f("tblIn")) f("tblIn").def = "pane"; if (f("tblPos")) f("tblPos").def = "br"; }
  if (id === "SMC_PRO" && f("tblSize")) f("tblSize").def = "tiny";
  // ligne signal blanche par défaut (choix d'Argos) : couleur ajoutée à la fin de la table (les réglages gardés par
  // position ne bougent pas), dans le même onglet que l'affichage de la ligne signal
  const sig = { AI_TREND: "sig_show", TBO: "sig_on" }[id];
  if (sig && f(sig)) params.push({ k: "sig_col", g: f(sig).g, label: "Couleur de la ligne signal", def: "#ffffff", type: "col" });
  out[id] = { groups, params };
}
console.log(JSON.stringify(out));
