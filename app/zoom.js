/* 🔍 Zoom of the app with the keyboard (Jee, 19 Nov 2026): Ctrl / Cmd + 0 = 100 %, Ctrl / Cmd + « = » or « + » =
   zoom in, Ctrl / Cmd + « − » = zoom out — the same path as « Zoom 100 % » of the ☰ Appli menu (setZoom in
   index.html). Only these keys are taken from the browser (its page zoom); in a text field of the page, the browser
   keeps them. */
function toucheZoom(e) {
  if (!(e.ctrlKey || e.metaKey) || e.altKey) return null;
  switch (e.key) {
    case "0": return "reset";
    case "=": case "+": return "in";
    case "-": case "−": return "out";
    default: return null;
  }
}
// a text field of the page (not the app's canvas): the browser keeps its shortcuts there
function pourApp(cible) {
  if (!cible || !cible.tagName) return true;
  const t = cible.tagName.toUpperCase();
  return !(t === "INPUT" || t === "TEXTAREA" || t === "SELECT" || cible.isContentEditable);
}
if (typeof module !== "undefined") module.exports = { toucheZoom, pourApp };
