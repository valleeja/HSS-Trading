// 📈 Page de la coque Tauri (T0) : demande la courbe au cœur Rust (commande module_ema) et la trace.
// Aucun calcul de trading ici, aucune clé, aucun réseau : la page ne fait que dessiner les nombres reçus.
"use strict";

const $ = (id) => document.getElementById(id);
let vue = null;

function dessiner() {
  const c = $("chart");
  const r = window.devicePixelRatio || 1;
  c.width = c.clientWidth * r;
  c.height = c.clientHeight * r;
  const g = c.getContext("2d");
  g.setTransform(r, 0, 0, r, 0, 0);
  const w = c.clientWidth, h = c.clientHeight, m = 16;
  g.clearRect(0, 0, w, h);
  if (!vue || vue.cloture.length < 2) return;
  const tout = vue.cloture.concat(vue.valeurs.filter((v) => v !== null));
  const bas = Math.min(...tout), haut = Math.max(...tout);
  const x = (i) => m + (i * (w - 2 * m)) / (vue.cloture.length - 1);
  const y = (v) => h - m - ((v - bas) * (h - 2 * m)) / (haut - bas || 1);
  const trait = (valeurs, couleur, epaisseur) => {
    g.strokeStyle = couleur;
    g.lineWidth = epaisseur;
    g.beginPath();
    let debut = true;
    valeurs.forEach((v, i) => {
      if (v === null) { debut = true; return; }
      if (debut) g.moveTo(x(i), y(v)); else g.lineTo(x(i), y(v));
      debut = false;
    });
    g.stroke();
  };
  trait(vue.cloture, "#5b6275", 1.2);
  trait(vue.valeurs, getComputedStyle(document.documentElement).getPropertyValue("--ligne"), 2);
  g.fillStyle = "#e6e6e6";
  g.font = "13px system-ui";
  g.fillText(`— clôture   — ${vue.ligne}`, m + 4, m + 8);
}

async function calculer() {
  const etat = $("etat");
  etat.className = "";
  etat.textContent = "Calcul dans la bulle du kit…";
  try {
    vue = await window.__TAURI__.core.invoke("module_ema", {
      longueur: Number($("longueur").value) || 20,
      enLigne: $("en-ligne").checked,
    });
    const der = vue.valeurs.filter((v) => v !== null).pop();
    etat.textContent = `${vue.ligne} = ${der === undefined ? "—" : der.toFixed(2)} · ${vue.source} · ${vue.empreinte} · aucun droit`;
    dessiner();
  } catch (e) {
    etat.className = "erreur";
    etat.textContent = `❌ Module refusé ou en erreur : ${e}`;
  }
}

$("calculer").addEventListener("click", calculer);
window.addEventListener("resize", dessiner);
calculer();
