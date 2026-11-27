/* ⚠️ NON UTILISÉE (2026-11-05) : sur un vrai iPhone en mode appli, la vue fait vraiment innerHeight (751) ;
   dessiner sur screen.height coupait la barre du bas. Gardée pour mémoire, la page ne la charge plus.
   📱 Taille de mise en page de l'appli installée sur iPhone / iPad (mode appli, barre d'état translucide) : iOS donne
   une fenêtre plus courte de la hauteur de la barre d'état (innerHeight = écran − zone sûre du haut) alors que la vue
   est dessinée depuis le haut de l'écran : sans correction, il reste une bande vide en bas. Règle : en mode appli
   iOS, si (écran − fenêtre) ≈ zone sûre du haut (portrait) ou de gauche + droite (paysage), la mise en page prend
   la taille de l'écran. Ailleurs : rien ne change (null). */
function iosLayoutSize(s) {
  if (!s.standalone || !s.ios) return null;
  const near = (a, b) => Math.abs(a - b) <= 2;
  // iOS donne screen.width × screen.height en portrait, même couché
  const portrait = s.innerH >= s.innerW;
  const [sw, sh] = portrait ? [Math.min(s.screenW, s.screenH), Math.max(s.screenW, s.screenH)] : [Math.max(s.screenW, s.screenH), Math.min(s.screenW, s.screenH)];
  let w = s.innerW, h = s.innerH;
  if (s.safeTop > 0 && sh > h && near(sh - h, s.safeTop)) h = sh;
  if (!portrait && sw > w && near(sw - w, s.safeLeft + s.safeRight)) w = sw;
  return w === s.innerW && h === s.innerH ? null : { w, h };
}
if (typeof module !== "undefined") module.exports = { iosLayoutSize };
