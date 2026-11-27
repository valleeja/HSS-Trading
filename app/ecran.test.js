// Test de la règle de taille iOS (node app/ecran.test.js) : lancé par `cargo test` (test pwa d'argos-desktop)
const assert = require("node:assert");
const { iosLayoutSize } = require("./ecran.js");
const base = { standalone: true, ios: true, screenW: 375, screenH: 812, safeLeft: 0, safeRight: 0 };
// iPhone de Jee : fenêtre 375 × 751, zone sûre du haut 61 → toute la hauteur de l'écran
assert.deepStrictEqual(iosLayoutSize({ ...base, innerW: 375, innerH: 751, safeTop: 61 }), { w: 375, h: 812 });
// déjà la bonne taille : rien
assert.strictEqual(iosLayoutSize({ ...base, innerW: 375, innerH: 812, safeTop: 47 }), null);
// Safari normal (pas en mode appli), Android : rien
assert.strictEqual(iosLayoutSize({ ...base, standalone: false, innerW: 375, innerH: 751, safeTop: 61 }), null);
assert.strictEqual(iosLayoutSize({ ...base, ios: false, innerW: 375, innerH: 751, safeTop: 61 }), null);
// écart qui n'est pas la barre d'état (clavier ouvert…) : rien
assert.strictEqual(iosLayoutSize({ ...base, innerW: 375, innerH: 500, safeTop: 61 }), null);
// paysage : la largeur manquante = zones sûres gauche + droite
assert.deepStrictEqual(iosLayoutSize({ ...base, innerW: 724, innerH: 375, safeTop: 0, safeLeft: 44, safeRight: 44 }), { w: 812, h: 375 });
console.log("ecran.js : ok");
