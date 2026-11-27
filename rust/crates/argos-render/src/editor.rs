//! 📜 Éditeur Argos Script, comme celui d'Argos v1 : une fenêtre au-dessus de la chart avec le menu du nom
//! (💾 sauvegarder, copie, renommer, historique des versions, nouveau, supprimer), ✨ Claude, 🌲 Pine, 📤 publier,
//! 📖 Doc (la page de doc à la place du code : `doc_editeur`), « ▶ Ajouter à la chart », ✕ ; le code (numéros de ligne, couleurs de syntaxe, police mono) et le terminal
//! (messages avec l'heure, erreurs en rouge) avec « Ligne N, Col M ».
//! Le script est du Pine v6 qui roule tel quel (`argos_script::lang`).
//! Clavier (routé par la fenêtre) : lettres, Entrée (garde l'indentation, +4 après `if`, `for`, `=>`…), ⌫ / Suppr,
//! flèches, Début / Fin, PgUp / PgDn, Tab = 4 espaces ; Ctrl + V coller, Ctrl + Entrée tester, Ctrl + S sauvegarder,
//! Échap ferme le menu, puis la fenêtre.

use crate::doc_editeur::{Doc, DocAct, ETROIT};
use crate::palette as pal;
use crate::palette::{accent, CODE_COM, CODE_FN, CODE_KW, CODE_NUM, CODE_STR, DIM, GREY, ICON, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Mono, Rgb, Text};
use std::cell::Cell;
use tiny_skia::Pixmap;

const HEAD: f64 = 40.0;
const TERM: f64 = 96.0;
const STAT: f64 = 24.0;
const SIZE: f32 = 13.0;
const LINE: f64 = 19.0;
const GUTTER: f64 = 52.0;
const PAD: f64 = 12.0;
const ITEM: f64 = 30.0;
/// Messages gardés dans le terminal (comme v1).
const LOG_MAX: usize = 40;

/// Boutons de l'en-tête.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdBtn {
    /// « 📜 nom ▾ » : ouvre le menu.
    Menu,
    Claude,
    Pine,
    Share,
    Help,
    Run,
    Close,
}

/// Lignes du menu du nom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdItem {
    Save,
    Copy,
    Rename,
    History,
    New,
    Delete,
}

/// Ce qu'un clic (ou Entrée dans un champ) demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdClick {
    None,
    Btn(EdBtn),
    Item(EdItem),
    /// OK du champ (renommer / copie) : son texte est dans `Extra::Prompt`.
    PromptOk,
    /// Version de l'historique à restaurer (index, la plus récente d'abord).
    Version(usize),
    /// Curseur posé dans le texte.
    Caret,
    /// 📖 Doc: a copy asked (the window puts the text in the clipboard).
    Doc(DocAct),
}

/// Bande sous l'en-tête : champ de nom, historique.
#[derive(Debug, Clone, PartialEq)]
pub enum Extra {
    None,
    /// Champ texte pour `Rename` ou `Copy`.
    Prompt(EdItem, String),
    /// Versions : (heure lisible, nombre de lignes), la plus récente d'abord.
    History(Vec<(String, usize)>),
}

/// Sorte de message du terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Info,
    Ok,
    Err,
    /// ⏳ / ⚠ : en orange (pas encore de bougies, rien à tracer pour l'instant).
    Warn,
    /// En gris : un détail sous un message.
    Note,
}

/// Touches d'édition (les lettres passent par `insert`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdKey {
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Tab,
}

/// L'éditeur ouvert.
#[derive(Debug, Clone)]
pub struct Editor {
    pub name: String,
    pub lines: Vec<String>,
    /// Curseur : (ligne, colonne en caractères).
    pub cur: (usize, usize),
    /// Première ligne affichée.
    pub top: usize,
    /// Script de la liste qu'il modifie (`None` : brouillon, pas encore ajouté).
    pub slot: Option<usize>,
    /// Ligne de la dernière erreur (en rouge dans le code).
    pub err_line: Option<u32>,
    /// Terminal : (heure HH:MM:SS, message, sorte).
    pub log: Vec<(String, String, LogKind)>,
    pub menu: bool,
    /// 🗑️ cliqué une fois : le 2e clic confirme.
    pub armed: bool,
    pub extra: Extra,
    /// { } Code d'un indicateur intégré (bouton { } de la légende) : on le lit, on ne le modifie pas.
    pub lecture_seule: bool,
    /// { } A Rust indicator's source (B1): read only, no « Créer une copie », plain text (the colours are Pine's).
    pub rust: bool,
    /// Ctrl + A : tout le texte est sélectionné (un texte tapé ou collé le remplace, ⌫ / Suppr le vident, Ctrl + C le
    /// copie ; une flèche, un clic ou Échap en sortent sans rien changer). L'éditeur n'a pas d'autre sélection.
    pub tout: bool,
    /// 📖 The Doc page (kept while the editor is open: « ‹ Code » then 📖 come back to the same place).
    pub doc: Doc,
    /// The Doc is shown in place of the code (the script, its cursor and its scroll stay as they are).
    pub doc_ouverte: bool,
    /// Lignes visibles au dernier dessin (pour garder le curseur à l'écran).
    rows: Cell<usize>,
}

/// Script de départ (« Nouveau script »), le même que celui de v1 (RSI 14 avec 70 / 30 et signaux), en Pine v6.
pub const TEMPLATE: &str = "//@version=6\nindicator(\"Mon script\", overlay = false)\n\nr = ta.rsi(close, 14)\nplot(r, \"RSI\", color = #7e57c2, linewidth = 2)\nhline(70)\nhline(30)\nplotshape(ta.crossover(r, 30), \"Achat\", shape.triangleup, location.bottom, color.teal, text = \"Achat\")\nplotshape(ta.crossunder(r, 70), \"Vente\", shape.triangledown, location.top, color.red, text = \"Vente\")\n";

impl Editor {
    pub fn new(name: &str, src: &str, slot: Option<usize>) -> Self {
        let mut e = Editor {
            name: name.into(),
            lines: vec![String::new()],
            cur: (0, 0),
            top: 0,
            slot,
            err_line: None,
            log: vec![],
            menu: false,
            armed: false,
            extra: Extra::None,
            lecture_seule: false,
            rust: false,
            tout: false,
            doc: Doc::default(),
            doc_ouverte: false,
            rows: Cell::new(10),
        };
        e.set_text(src);
        e
    }

    /// Remplace tout le texte (curseur au début).
    pub fn set_text(&mut self, src: &str) {
        self.lines = src.replace("\r\n", "\n").replace('\t', "    ").split('\n').map(str::to_string).collect();
        self.cur = (0, 0);
        self.top = 0;
        self.err_line = None;
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Ajoute un message au terminal ; `now_local` : heure locale (secondes depuis 1970, fuseau déjà ajouté).
    pub fn log(&mut self, now_local: i64, msg: impl Into<String>, kind: LogKind) {
        let s = now_local.rem_euclid(86_400);
        // (📱 phone view: « colle ton script (Ctrl + V) » → « colle ton script »)
        let msg = crate::actions::sans_touches(&msg.into());
        self.log.push((format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60), msg, kind));
        let n = self.log.len();
        self.log.drain(..n.saturating_sub(LOG_MAX));
    }

    fn len(&self, l: usize) -> usize {
        self.lines[l].chars().count()
    }

    fn byte(&self, l: usize, col: usize) -> usize {
        self.lines[l].char_indices().nth(col).map_or(self.lines[l].len(), |(b, _)| b)
    }

    /// Ctrl + A : tout sélectionner (aussi en lecture seule, pour Ctrl + C).
    pub fn tout_selectionner(&mut self) {
        if !matches!(self.extra, Extra::Prompt(..)) && !self.doc_ouverte {
            self.tout = true;
        }
    }

    /// Tape ou colle du texte : dans le champ ouvert (renommer / copie), sinon au curseur. Tout sélectionné
    /// (Ctrl + A) : il remplace tout. Un script entier (avec sa déclaration `indicator(`…) collé dans le modèle de
    /// départ jamais modifié le remplace aussi (Jee, 21 nov. : sinon deux `indicator()` dans un texte).
    pub fn insert(&mut self, s: &str) {
        // 📖 Doc open: only its search field takes text, the script never changes behind it
        if self.doc_ouverte {
            self.doc.taper(s);
            return;
        }
        if let Extra::Prompt(_, t) = &mut self.extra {
            let room = 40usize.saturating_sub(t.chars().count());
            t.extend(s.chars().filter(|c| !c.is_control()).take(room));
            return;
        }
        // { } lecture seule : le texte ne change pas
        if self.lecture_seule {
            return;
        }
        let modele = self.slot.is_none() && self.text() == TEMPLATE && declare_un_script(s);
        if std::mem::take(&mut self.tout) || modele {
            self.set_text("");
        }
        let s = s.replace("\r\n", "\n").replace('\t', "    ");
        for (k, part) in s.split('\n').enumerate() {
            if k > 0 {
                self.newline(false);
            }
            let (l, c) = self.cur;
            let b = self.byte(l, c);
            self.lines[l].insert_str(b, part);
            self.cur.1 += part.chars().count();
        }
        self.edited();
    }

    /// Coupe la ligne au curseur ; `indent` : garde l'indentation (et +4 après un début de bloc).
    fn newline(&mut self, indent: bool) {
        let (l, c) = self.cur;
        let b = self.byte(l, c);
        let rest = self.lines[l].split_off(b);
        let mut pad = 0;
        if indent {
            let line = &self.lines[l];
            pad = line.len() - line.trim_start().len();
            let t = line.trim();
            let opens = ["if ", "else", "for ", "while ", "switch", "type ", "method "].iter().any(|k| t.starts_with(k)) || t.ends_with("=>");
            if opens && !t.starts_with("//") {
                pad += 4;
            }
        }
        self.lines.insert(l + 1, format!("{}{rest}", " ".repeat(pad)));
        self.cur = (l + 1, pad);
    }

    fn edited(&mut self) {
        self.err_line = None;
        self.keep_visible();
    }

    fn keep_visible(&mut self) {
        let rows = self.rows.get().max(1);
        if self.cur.0 < self.top {
            self.top = self.cur.0;
        } else if self.cur.0 >= self.top + rows {
            self.top = self.cur.0 + 1 - rows;
        }
    }

    /// Une touche d'édition ; dans le champ ouvert, Entrée rend `PromptOk`.
    pub fn key(&mut self, k: EdKey) -> EdClick {
        // 📖 Doc: Entrée / ↓ next match, ↑ previous one, ⌫ in the search, Page ↑ / ↓ scroll; the script is not touched
        if self.doc_ouverte {
            match k {
                EdKey::Enter | EdKey::Down => self.doc.suivant(1),
                EdKey::Up => self.doc.suivant(-1),
                EdKey::Backspace => self.doc.effacer(),
                EdKey::PageUp => self.doc.wheel(-400.0),
                EdKey::PageDown => self.doc.wheel(400.0),
                EdKey::Home => self.doc.wheel(-1e9),
                EdKey::End => self.doc.wheel(1e9),
                _ => {}
            }
            return EdClick::None;
        }
        if let Extra::Prompt(_, t) = &mut self.extra {
            match k {
                EdKey::Enter => return EdClick::PromptOk,
                EdKey::Backspace => _ = t.pop(),
                _ => {}
            }
            return EdClick::None;
        }
        // (tout sélectionné : ⌫ / Suppr vident le texte, les autres touches en sortent sans rien changer)
        if std::mem::take(&mut self.tout) {
            if matches!(k, EdKey::Backspace | EdKey::Delete) && !self.lecture_seule {
                self.set_text("");
                self.edited();
                return EdClick::None;
            }
            if k != EdKey::Enter && k != EdKey::Tab {
                return EdClick::None;
            }
            if !self.lecture_seule {
                self.set_text("");
            }
        }
        let (l, c) = self.cur;
        let n = self.lines.len();
        // { } lecture seule : on se déplace, on ne modifie rien
        if self.lecture_seule && matches!(k, EdKey::Enter | EdKey::Tab | EdKey::Backspace | EdKey::Delete) {
            return EdClick::None;
        }
        match k {
            EdKey::Enter => {
                self.newline(true);
                self.edited();
            }
            EdKey::Tab => self.insert(&" ".repeat(4 - c % 4)),
            EdKey::Backspace => {
                if c > 0 {
                    // dans l'indentation : recule d'un cran de 4
                    let lead = self.lines[l].chars().take(c).all(|ch| ch == ' ');
                    let k = if lead { (c - 1) % 4 + 1 } else { 1 };
                    let (a, b) = (self.byte(l, c - k), self.byte(l, c));
                    self.lines[l].replace_range(a..b, "");
                    self.cur.1 -= k;
                } else if l > 0 {
                    let line = self.lines.remove(l);
                    self.cur = (l - 1, self.len(l - 1));
                    self.lines[l - 1].push_str(&line);
                }
                self.edited();
            }
            EdKey::Delete => {
                if c < self.len(l) {
                    let (a, b) = (self.byte(l, c), self.byte(l, c + 1));
                    self.lines[l].replace_range(a..b, "");
                } else if l + 1 < n {
                    let next = self.lines.remove(l + 1);
                    self.lines[l].push_str(&next);
                }
                self.edited();
            }
            EdKey::Left if c > 0 => self.cur.1 -= 1,
            EdKey::Left if l > 0 => self.cur = (l - 1, self.len(l - 1)),
            EdKey::Right if c < self.len(l) => self.cur.1 += 1,
            EdKey::Right if l + 1 < n => self.cur = (l + 1, 0),
            EdKey::Up if l > 0 => self.cur = (l - 1, c.min(self.len(l - 1))),
            EdKey::Down if l + 1 < n => self.cur = (l + 1, c.min(self.len(l + 1))),
            EdKey::Home => {
                // 1er appui : début du texte de la ligne ; 2e : colonne 0
                let first = self.lines[l].len() - self.lines[l].trim_start().len();
                self.cur.1 = if c == first { 0 } else { first };
            }
            EdKey::End => self.cur.1 = self.len(l),
            EdKey::PageUp | EdKey::PageDown => {
                let r = self.rows.get();
                self.cur.0 = if k == EdKey::PageUp { l.saturating_sub(r) } else { (l + r).min(n - 1) };
                self.cur.1 = c.min(self.len(self.cur.0));
            }
            _ => {}
        }
        self.keep_visible();
        EdClick::None
    }

    /// Échap : ferme le menu ou la bande ; `true` = rien d'ouvert, la fenêtre peut se fermer.
    pub fn escape(&mut self) -> bool {
        if std::mem::take(&mut self.tout) {
            return false;
        }
        // 📖 Doc: the sheet, then the search, then back to the code (the window stays)
        if self.doc_ouverte {
            self.menu = false;
            if self.doc.escape() {
                self.doc_ouverte = false;
            }
            return false;
        }
        if self.menu || self.extra != Extra::None {
            self.menu = false;
            self.armed = false;
            self.extra = Extra::None;
            return false;
        }
        true
    }

    /// 📖 Opens the Doc in place of the code, or goes back to the code (button 📖).
    pub fn basculer_doc(&mut self) {
        self.doc_ouverte = !self.doc_ouverte;
        self.menu = false;
        self.tout = false;
        self.doc.saisie = false;
        self.doc.feuille = false;
    }

    /// Wheel / finger on the editor: the Doc scrolls by `dy` points; the code by lines (3 per notch, as before).
    pub fn molette(&mut self, steps: f64) {
        if self.doc_ouverte {
            self.doc.wheel(-steps * 40.0);
        } else {
            self.scroll((-steps * 3.0).round() as i64);
        }
    }

    /// Where a tap opens the keyboard (window coordinates): the whole editor, or the Doc's search field.
    pub fn zone_saisie(&self, text: &Text, w: f64, h: f64) -> Rect {
        if self.doc_ouverte {
            self.doc.champ(text, w, h, Self::doc_top(w))
        } else {
            Rect::new(0.0, 0.0, w, h)
        }
    }

    /// Top of the Doc: under the editor header on a wide window; the whole window on a narrow one (its own header).
    fn doc_top(w: f64) -> f64 {
        if w < ETROIT {
            0.0
        } else {
            HEAD
        }
    }

    /// Molette : `steps` lignes (positif = vers le bas).
    pub fn scroll(&mut self, steps: i64) {
        let max = self.lines.len().saturating_sub(1);
        self.top = (self.top as i64 + steps).clamp(0, max as i64) as usize;
    }

    /// Montre la ligne `line` (1 = première) en rouge, curseur au début : une erreur.
    pub fn goto(&mut self, line: u32) {
        self.cur = ((line as usize).saturating_sub(1).min(self.lines.len() - 1), 0);
        self.top = self.cur.0.saturating_sub(self.rows.get() / 3);
        self.err_line = Some(line);
    }

    fn label(&self, b: EdBtn) -> &'static str {
        match b {
            EdBtn::Menu => "",
            EdBtn::Claude => "✨ Claude",
            EdBtn::Pine => "🌲 Pine",
            EdBtn::Share => "📤",
            EdBtn::Help if self.doc_ouverte => "📖 Doc",
            EdBtn::Help => "📖",
            EdBtn::Run => "▶ Ajouter à la chart",
            EdBtn::Close => "✕",
        }
    }

    fn item_label(&self, i: EdItem) -> &'static str {
        match i {
            EdItem::Save => "💾 Sauvegarder le script",
            EdItem::Copy => "📄 Créer une copie…",
            EdItem::Rename => "✏️ Renommer…",
            EdItem::History => "🕘 Historique des versions…",
            EdItem::New => "+ Nouveau script",
            EdItem::Delete if self.armed => "Confirmer la suppression",
            EdItem::Delete => "🗑️ Supprimer",
        }
    }

    fn title(&self) -> String {
        format!("📜 {} ▾", self.name)
    }

    /// Boutons de l'en-tête (coordonnées de la fenêtre `w`).
    fn buttons(&self, text: &Text, w: f64) -> Vec<(Rect, EdBtn)> {
        let titre = Rect::new(PAD - 4.0, 7.0, text.width(&self.title(), 14.0) as f64 + 16.0, 26.0);
        let mut v = vec![(titre, EdBtn::Menu)];
        let mut x = w - PAD;
        for b in [EdBtn::Close, EdBtn::Run, EdBtn::Help, EdBtn::Share, EdBtn::Pine, EdBtn::Claude] {
            // (📖 Doc open: the header of the mockup, without ✨ Claude and 🌲 Pine)
            if self.doc_ouverte && matches!(b, EdBtn::Pine | EdBtn::Claude) {
                continue;
            }
            // ({ } a Rust indicator's code — tableaux-1 tour 2, F8: only what acts on it, 📖 and ✕; no « ▶ Ajouter à
            // la chart », ✨ Claude, 🌲 Pine nor 📤, which work on a script)
            if self.rust && matches!(b, EdBtn::Run | EdBtn::Claude | EdBtn::Pine | EdBtn::Share) {
                continue;
            }
            let bw = text.width(self.label(b), 12.5) as f64 + if b == EdBtn::Run { 24.0 } else { 14.0 };
            // (📱 narrow window: the extra buttons that would cover the name are left out; ✕ and ▶ always stay)
            if x - bw < titre.x + titre.w + 6.0 && !matches!(b, EdBtn::Close | EdBtn::Run) {
                continue;
            }
            x -= bw;
            v.push((Rect::new(x, 8.0, bw, 24.0), b));
            x -= 6.0;
        }
        v
    }

    /// Lignes du menu ouvert (sous le nom ; un trait avant « Nouveau »).
    fn items(&self, text: &Text) -> Vec<(Rect, EdItem)> {
        if !self.menu {
            return vec![];
        }
        let all: Vec<EdItem> = [EdItem::Save, EdItem::Copy, EdItem::Rename, EdItem::History, EdItem::New, EdItem::Delete]
            .into_iter()
            // ({ } a Rust indicator: no copy, it is not a script)
            .filter(|i| !(self.rust && *i == EdItem::Copy))
            .collect();
        let w = all.iter().map(|i| text.width(self.item_label(*i), 13.0)).fold(0.0f32, f32::max) as f64 + 90.0;
        all.into_iter()
            .enumerate()
            .map(|(k, i)| {
                (
                    Rect::new(
                        PAD - 4.0,
                        HEAD + 2.0 + k as f64 * ITEM + if matches!(i, EdItem::New | EdItem::Delete) { 7.0 } else { 0.0 },
                        w,
                        ITEM,
                    ),
                    i,
                )
            })
            .collect()
    }

    /// Hauteur de la bande sous l'en-tête.
    fn extra_h(&self) -> f64 {
        match &self.extra {
            Extra::None => 0.0,
            Extra::Prompt(..) => 44.0,
            Extra::History(v) => v.len().clamp(1, 6) as f64 * 28.0 + 12.0,
        }
    }

    /// Zones de la bande : OK du champ, versions de l'historique.
    fn extra_zones(&self, w: f64) -> Vec<(Rect, EdClick)> {
        match &self.extra {
            Extra::Prompt(..) => vec![(Rect::new(w - PAD - 60.0, HEAD + 8.0, 60.0, 28.0), EdClick::PromptOk)],
            Extra::History(v) => (0..v.len().min(6))
                .map(|k| (Rect::new(PAD, HEAD + 6.0 + k as f64 * 28.0, w - 2.0 * PAD, 26.0), EdClick::Version(k)))
                .collect(),
            _ => vec![],
        }
    }

    /// Haut du code.
    fn code_top(&self) -> f64 {
        HEAD + self.extra_h()
    }

    /// Clic en `(x, y)` (coordonnées de la fenêtre `w` × `h`).
    pub fn click(&mut self, text: &Text, x: f64, y: f64, w: f64, h: f64) -> EdClick {
        self.tout = false;
        // 📖 Doc on a narrow window: the page has its own header (‹ Code, ✕)
        if self.doc_ouverte && w < ETROIT {
            self.menu = false;
            return self.doc_click(text, x, y, w, h);
        }
        if let Some((_, i)) = self.items(text).into_iter().find(|(r, _)| r.contains(x, y)) {
            if !(i == EdItem::Delete && !self.armed) {
                self.menu = false;
            }
            // (a field or the history opens under the header: back to the code to show it)
            if matches!(i, EdItem::Copy | EdItem::Rename | EdItem::History | EdItem::New) {
                self.doc_ouverte = false;
            }
            return EdClick::Item(i);
        }
        let was_menu = std::mem::take(&mut self.menu);
        self.armed = false;
        if let Some((_, b)) = self.buttons(text, w).into_iter().find(|(r, _)| r.contains(x, y)) {
            self.menu = b == EdBtn::Menu && !was_menu;
            return if b == EdBtn::Menu { EdClick::None } else { EdClick::Btn(b) };
        }
        if self.doc_ouverte {
            return self.doc_click(text, x, y, w, h);
        }
        if let Some((_, c)) = self.extra_zones(w).into_iter().find(|(r, _)| r.contains(x, y)) {
            return c;
        }
        let top = self.code_top();
        if y < top || y > h - TERM - STAT {
            return EdClick::None;
        }
        let l = (self.top + ((y - top - 4.0) / LINE).max(0.0) as usize).min(self.lines.len() - 1);
        let c = ((x - GUTTER - PAD) / Mono::advance(SIZE) as f64).round().max(0.0) as usize;
        self.cur = (l, c.min(self.len(l)));
        EdClick::Caret
    }

    fn doc_click(&mut self, text: &Text, x: f64, y: f64, w: f64, h: f64) -> EdClick {
        match self.doc.click(text, w, h, Self::doc_top(w), x, y) {
            Some(DocAct::Code) => {
                self.doc_ouverte = false;
                EdClick::None
            }
            Some(DocAct::Fermer) => EdClick::Btn(EdBtn::Close),
            Some(a) => EdClick::Doc(a),
            None => EdClick::None,
        }
    }

    /// Image de la fenêtre (`w` × `h`).
    pub fn draw(&self, text: &Text, w: f64, h: f64, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let mut pm = crate::px::new(w.max(1.0) as u32, h.max(1.0) as u32)?;
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let frame = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
        round_fill(&mut pm, frame, 10.0, pal::panel(), 1.0);
        // 📖 Doc on a narrow window (cell): the page and its own header take the whole editor
        if self.doc_ouverte && w < ETROIT {
            self.doc.draw(&mut pm, text, w, h, 0.0, mouse);
            round_stroke(&mut pm, frame, 10.0, pal::edge(), 1.0);
            return Some(pm);
        }
        // en-tête : 📜 nom ▾ et les boutons
        for (r, b) in self.buttons(text, w) {
            match b {
                EdBtn::Menu => {
                    if hover(&r) || self.menu {
                        round_fill(&mut pm, r, 5.0, pal::hover(), 1.0);
                    }
                    text.draw(&mut pm, &self.title(), ((r.x + 8.0) as f32, (r.y + 18.0) as f32), 14.0, WHITE, Align::Left);
                }
                EdBtn::Run => {
                    round_fill(&mut pm, r, 5.0, accent(), if hover(&r) { 0.85 } else { 1.0 });
                    text.draw(&mut pm, self.label(b), (r.cx() as f32, (r.y + 16.5) as f32), 12.5, WHITE, Align::Center);
                }
                EdBtn::Help if self.doc_ouverte => {
                    round_fill(&mut pm, r, 12.0, accent(), 0.28);
                    crate::shapes::round_stroke_alpha(&mut pm, r, 12.0, accent(), 1.0, 0.55);
                    text.draw(&mut pm, self.label(b), (r.cx() as f32, (r.y + 16.5) as f32), 12.5, WHITE, Align::Center);
                }
                _ => {
                    if hover(&r) {
                        round_fill(&mut pm, r, 5.0, pal::hover(), 1.0);
                    }
                    text.draw(&mut pm, self.label(b), (r.cx() as f32, (r.y + 16.5) as f32), 12.5, ICON, Align::Center);
                }
            }
        }
        crate::fill(&mut pm, 0.0, HEAD, w, 1.0, pal::edge(), 1.0);
        if self.doc_ouverte {
            self.doc.draw(&mut pm, text, w, h, HEAD, mouse);
            self.draw_menu(&mut pm, text, mouse);
            round_stroke(&mut pm, frame, 10.0, pal::edge(), 1.0);
            return Some(pm);
        }
        // bande : aide, champ, historique
        let top = self.code_top();
        match &self.extra {
            Extra::None => {}
            Extra::Prompt(_, t) => {
                let r = Rect::new(PAD, HEAD + 8.0, w - 3.0 * PAD - 60.0, 28.0);
                round_fill(&mut pm, r, 5.0, pal::field(), 1.0);
                round_stroke(&mut pm, r, 5.0, accent(), 1.0);
                text.draw(&mut pm, t, ((r.x + 8.0) as f32, (r.y + 19.0) as f32), 13.0, WHITE, Align::Left);
                crate::fill(&mut pm, r.x + 9.0 + text.width(t, 13.0) as f64, r.y + 6.0, 1.5, 16.0, accent(), 1.0);
                for (ok, _) in self.extra_zones(w) {
                    round_fill(&mut pm, ok, 5.0, accent(), 1.0);
                    text.draw(&mut pm, "OK", (ok.cx() as f32, (ok.y + 19.0) as f32), 13.0, WHITE, Align::Center);
                }
            }
            Extra::History(v) if v.is_empty() => text.draw(
                &mut pm,
                "Aucune version sauvegardée : 💾 crée une version.",
                (PAD as f32, (HEAD + 24.0) as f32),
                12.5,
                GREY,
                Align::Left,
            ),
            Extra::History(v) => {
                for ((r, _), (when, n)) in self.extra_zones(w).iter().zip(v) {
                    if hover(r) {
                        round_fill(&mut pm, *r, 5.0, pal::hover(), 1.0);
                    }
                    text.draw(
                        &mut pm,
                        &format!("🕘 {when} · {n} lignes"),
                        ((r.x + 8.0) as f32, (r.y + 17.5) as f32),
                        12.5,
                        ICON,
                        Align::Left,
                    );
                    text.draw(&mut pm, "restaurer", ((r.x + r.w - 8.0) as f32, (r.y + 17.5) as f32), 12.0, GREY, Align::Right);
                }
            }
        }
        if self.extra != Extra::None {
            crate::fill(&mut pm, 0.0, top, w, 1.0, pal::edge(), 1.0);
        }
        // code : marge des numéros, ligne de l'erreur en rouge, ligne du curseur
        let tt = h - TERM - STAT;
        let rows = ((tt - top - 4.0) / LINE).max(1.0) as usize;
        self.rows.set(rows);
        crate::fill(&mut pm, 1.0, top + 1.0, GUTTER, tt - top - 1.0, pal::field(), 1.0);
        let adv = Mono::advance(SIZE) as f64;
        for (k, line) in self.lines.iter().enumerate().skip(self.top).take(rows) {
            let y = top + 4.0 + (k - self.top) as f64 * LINE;
            if self.err_line == Some(k as u32 + 1) {
                crate::fill(&mut pm, GUTTER, y, w - GUTTER - 1.0, LINE, crate::palette::down(), 0.18);
            } else if self.tout {
                crate::fill(&mut pm, GUTTER, y, w - GUTTER - 1.0, LINE, accent(), 0.22);
            } else if k == self.cur.0 {
                crate::fill(&mut pm, GUTTER, y, w - GUTTER - 1.0, LINE, pal::hover(), 0.6);
            }
            let num = (k + 1).to_string();
            Mono::draw(
                &mut pm,
                &num,
                ((GUTTER - 10.0 - num.len() as f64 * adv) as f32, (y + 14.0) as f32),
                SIZE - 1.0,
                if k == self.cur.0 { ICON } else { DIM },
            );
            if self.rust {
                Mono::draw(&mut pm, line, ((GUTTER + PAD) as f32, (y + 14.0) as f32), SIZE, WHITE);
            } else {
                for (from, piece, c) in spans(line) {
                    Mono::draw(&mut pm, piece, ((GUTTER + PAD + from as f64 * adv) as f32, (y + 14.0) as f32), SIZE, c);
                }
            }
        }
        if (self.top..self.top + rows).contains(&self.cur.0) && !matches!(self.extra, Extra::Prompt(..)) {
            let x = GUTTER + PAD + self.cur.1 as f64 * adv;
            crate::fill(
                &mut pm,
                x.round(),
                top + 5.0 + (self.cur.0 - self.top) as f64 * LINE,
                2.0,
                LINE - 2.0,
                accent(),
                1.0,
            );
        }
        // terminal : derniers messages avec l'heure, puis « ⌨️ Terminal » et la position du curseur
        crate::fill(&mut pm, 1.0, tt, w - 2.0, TERM + STAT - 6.0, pal::field(), 1.0);
        crate::fill(&mut pm, 0.0, tt, w, 1.0, pal::edge(), 1.0);
        let fit = ((TERM - 8.0) / 17.0) as usize;
        for (k, (t, m, kind)) in self.log.iter().rev().take(fit).rev().enumerate() {
            let y = tt + 18.0 + k as f64 * 17.0;
            let c = match kind {
                LogKind::Ok => crate::palette::up(),
                LogKind::Err => crate::palette::down(),
                LogKind::Warn => crate::palette::ORANGE,
                LogKind::Note => GREY,
                LogKind::Info => ICON,
            };
            Mono::draw(&mut pm, t, (PAD as f32, y as f32), 12.0, GREY);
            text.draw(
                &mut pm,
                &text.fit(m, 12.5, w - 2.0 * PAD - 70.0),
                ((PAD + 70.0) as f32, y as f32),
                12.5,
                c,
                Align::Left,
            );
        }
        let sy = h - STAT;
        crate::fill(&mut pm, 0.0, sy, w, 1.0, pal::edge(), 1.0);
        text.draw(&mut pm, "⌨️ Terminal", (PAD as f32, (sy + 16.0) as f32), 11.5, GREY, Align::Left);
        text.draw(
            &mut pm,
            &if self.tout {
                "Tout sélectionné".to_string()
            } else {
                format!("Ligne {}, Col {}", self.cur.0 + 1, self.cur.1 + 1)
            },
            ((w - PAD) as f32, (sy + 16.0) as f32),
            11.5,
            GREY,
            Align::Right,
        );
        self.draw_menu(&mut pm, text, mouse);
        round_stroke(&mut pm, frame, 10.0, pal::edge(), 1.0);
        Some(pm)
    }

    /// Le menu du nom, par-dessus le reste.
    fn draw_menu(&self, pm: &mut Pixmap, text: &Text, mouse: Option<(f64, f64)>) {
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let items = self.items(text);
        if let Some(((first, _), (last, _))) = items.first().zip(items.last()) {
            let bg = Rect::new(first.x - 4.0, first.y - 4.0, first.w + 8.0, last.y + last.h - first.y + 8.0);
            round_fill(pm, bg, 8.0, pal::tile(), 1.0);
            round_stroke(pm, bg, 8.0, pal::edge(), 1.0);
            if let Some((r, _)) = items.iter().find(|(_, i)| *i == EdItem::New) {
                crate::fill(pm, first.x + 6.0, r.y - 4.0, first.w - 12.0, 1.0, pal::edge(), 1.0);
            }
            for (r, i) in &items {
                if hover(r) {
                    round_fill(pm, *r, 5.0, pal::hover(), 1.0);
                }
                text.draw(
                    pm,
                    self.item_label(*i),
                    ((r.x + 10.0) as f32, (r.y + 20.0) as f32),
                    13.0,
                    if *i == EdItem::Delete && self.armed { crate::palette::down() } else { WHITE },
                    Align::Left,
                );
                if *i == EdItem::Save && crate::actions::montrer_raccourcis() {
                    text.draw(pm, "Ctrl+S", ((r.x + r.w - 10.0) as f32, (r.y + 20.0) as f32), 11.5, GREY, Align::Right);
                }
            }
        }
    }
}

/// Mots-clés de Pine v6 (et d'Argos Script).
const KEYWORDS: [&str; 22] = [
    "if", "else", "for", "to", "by", "in", "while", "switch", "var", "varip", "and", "or", "not", "true", "false", "na", "import", "export", "method", "type",
    "break", "continue",
];

/// Morceaux colorés d'une ligne : (colonne de départ, texte, couleur).
/// Le texte collé est un script entier : une ligne commence par sa déclaration (`indicator(`, `strategy(`, `study(`,
/// `library(`).
fn declare_un_script(s: &str) -> bool {
    s.lines().any(|l| {
        let l = l.trim_start();
        ["indicator(", "strategy(", "study(", "library("].iter().any(|d| l.starts_with(d))
    })
}

pub(crate) fn spans(line: &str) -> Vec<(usize, &str, Rgb)> {
    let cs: Vec<(usize, char)> = line.char_indices().collect();
    let byte = |k: usize| cs.get(k).map_or(line.len(), |c| c.0);
    let mut out = vec![];
    let mut k = 0;
    while k < cs.len() {
        let ch = cs[k].1;
        let start = k;
        let color = if line[byte(k)..].starts_with("//") {
            k = cs.len();
            CODE_COM
        } else if ch == '"' || ch == '\'' {
            k += 1;
            while k < cs.len() && cs[k].1 != ch {
                k += if cs[k].1 == '\\' { 2 } else { 1 };
            }
            k = (k + 1).min(cs.len());
            CODE_STR
        } else if ch.is_ascii_digit() || (ch == '#' && cs.get(k + 1).is_some_and(|c| c.1.is_ascii_hexdigit())) {
            k += 1;
            while k < cs.len() && (cs[k].1.is_ascii_alphanumeric() || cs[k].1 == '.') {
                k += 1;
            }
            CODE_NUM
        } else if ch.is_alphabetic() || ch == '_' {
            while k < cs.len() && (cs[k].1.is_alphanumeric() || cs[k].1 == '_' || cs[k].1 == '.') {
                k += 1;
            }
            let word = &line[byte(start)..byte(k)];
            if KEYWORDS.contains(&word) {
                CODE_KW
            } else if cs.get(k).is_some_and(|c| c.1 == '(') || word.contains('.') {
                CODE_FN
            } else {
                WHITE
            }
        } else {
            k += 1;
            while k < cs.len() && !(cs[k].1.is_alphanumeric() || matches!(cs[k].1, '_' | '"' | '\'' | '#') || line[byte(k)..].starts_with("//")) {
                k += 1;
            }
            ICON
        };
        out.push((start, &line[byte(start)..byte(k)], color));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A5 (Jee, 21 Nov: « Ctrl + A ne fait rien »): Ctrl + A selects everything; typed or pasted text replaces it,
    /// ⌫ / Suppr empty it, an arrow, a click or Échap leave the state without changing the text; read-only code
    /// stays as it is.
    #[test]
    fn tout_selectionner() {
        let mut e = Editor::new("s", "a = 1\nb = 2", Some(0));
        e.tout_selectionner();
        e.insert("plot(close)");
        assert_eq!(
            (e.text().as_str(), e.cur, e.tout),
            ("plot(close)", (0, 11), false),
            "a paste replaces everything"
        );
        for k in [EdKey::Backspace, EdKey::Delete] {
            let mut e = Editor::new("s", "a = 1\nb = 2", Some(0));
            e.tout_selectionner();
            e.key(k);
            assert_eq!(e.text(), "", "{k:?} empties the text");
        }
        let mut e = Editor::new("s", "a = 1\nb = 2", Some(0));
        e.tout_selectionner();
        e.key(EdKey::Down);
        assert_eq!((e.text().as_str(), e.cur, e.tout), ("a = 1\nb = 2", (0, 0), false), "an arrow leaves it");
        e.tout_selectionner();
        assert!(!e.escape(), "Échap leaves the state first, the window stays");
        assert!(!e.tout && e.text() == "a = 1\nb = 2");
        e.tout_selectionner();
        let t = Text::default();
        e.click(&t, 300.0, 200.0, 800.0, 600.0);
        assert!(!e.tout, "a click leaves it");
        let mut l = Editor::new("s", "a = 1", None);
        l.lecture_seule = true;
        l.tout_selectionner();
        l.insert("x");
        l.key(EdKey::Backspace);
        assert_eq!(l.text(), "a = 1", "read-only: Ctrl + A only serves Ctrl + C");
    }

    /// D1 / D8: 📖 shows the Doc in place of the code; typing, keys and Ctrl + A do not touch the script; « ‹ Code »
    /// (and Échap) come back to the script exactly as it was: text, cursor, scroll.
    #[test]
    fn doc_puis_code_garde_le_script() {
        let t = Text::default();
        for (w, h) in [(1000.0, 700.0), (390.0, 780.0)] {
            let mut e = Editor::new("Ma moyenne", TEMPLATE, Some(0));
            e.click(&t, GUTTER + PAD + 30.0, HEAD + 4.0 + LINE * 3.5, w, h);
            e.key(EdKey::End);
            e.scroll(2);
            let avant = (e.text(), e.cur, e.top);
            e.basculer_doc();
            assert!(e.doc_ouverte);
            assert!(e.draw(&t, w, h, None).is_some());
            e.insert("plot(x)");
            for k in [EdKey::Enter, EdKey::Backspace, EdKey::Delete, EdKey::Tab, EdKey::Down] {
                e.key(k);
            }
            e.tout_selectionner();
            e.molette(-3.0);
            assert!(!e.tout);
            assert_eq!((e.text(), e.cur, e.top), avant.clone(), "{w}: the script does not move behind the Doc");
            // ‹ Code
            let code = if w < ETROIT { (30.0, 26.0) } else { (40.0, HEAD + 25.0) };
            assert_eq!(e.click(&t, code.0, code.1, w, h), EdClick::None);
            assert!(!e.doc_ouverte, "{w}: ‹ Code");
            assert_eq!((e.text(), e.cur, e.top), avant, "{w}: same text, cursor, scroll");
            // Échap from the Doc: back to the code, the window stays
            e.basculer_doc();
            assert!(!e.escape() && !e.doc_ouverte);
        }
        // wide: the header keeps the name, 📤, « 📖 Doc », ▶ and ✕ (no ✨ Claude, 🌲 Pine)
        let mut e = Editor::new("Ma moyenne", TEMPLATE, None);
        e.basculer_doc();
        let b: Vec<EdBtn> = e.buttons(&t, 1000.0).into_iter().map(|(_, b)| b).collect();
        assert_eq!(b, vec![EdBtn::Menu, EdBtn::Close, EdBtn::Run, EdBtn::Help, EdBtn::Share]);
        assert_eq!(e.label(EdBtn::Help), "📖 Doc");
        // narrow: its ✕ closes the editor
        assert_eq!(e.click(&t, 390.0 - 29.0, 26.0, 390.0, 780.0), EdClick::Btn(EdBtn::Close));
    }

    /// 📱 A narrow editor (390 px phone): the name and the header buttons never cover each other; ✕ and ▶ stay.
    #[test]
    fn en_tete_etroit_sans_chevauchement() {
        let t = Text::default();
        let e = Editor::new("Mon script", TEMPLATE, None);
        for w in [350.0, 390.0, 1000.0] {
            let b = e.buttons(&t, w);
            for (i, (r, _)) in b.iter().enumerate() {
                for (q, _) in &b[i + 1..] {
                    assert!(r.x + r.w <= q.x || q.x + q.w <= r.x, "{w} px: buttons overlap");
                }
            }
            assert!(b.iter().any(|(_, x)| *x == EdBtn::Close) && b.iter().any(|(_, x)| *x == EdBtn::Run));
        }
        assert_eq!(e.buttons(&t, 1000.0).len(), 7, "wide: all of them");
    }

    /// A4: a whole script pasted into the untouched « Nouveau script » template replaces it (no two `indicator()` in
    /// one text); any other paste goes in at the cursor.
    #[test]
    fn coller_un_script_dans_le_modele_le_remplace() {
        let script = "//@version=6\nindicator(\"Mien\")\nplot(close)";
        let mut e = Editor::new("Mon script", TEMPLATE, None);
        e.insert(script);
        assert_eq!(e.text(), script);
        assert_eq!(e.cur, (2, 11), "cursor at the end of the pasted text");
        let mut e = Editor::new("Mon script", TEMPLATE, None);
        e.insert("plot(close)");
        assert!(e.text().starts_with("plot(close)//@version=6"), "not a whole script: at the cursor");
        let mut e = Editor::new("Mon script", TEMPLATE, None);
        e.key(EdKey::End);
        e.insert("x");
        e.insert(script);
        assert!(
            e.text().contains("indicator(\"Mon script\"") && e.text().contains("Mien"),
            "edited: at the cursor"
        );
        let mut e = Editor::new("Mon script", TEMPLATE, Some(0));
        e.insert(script);
        assert!(e.text().contains("Mon script"), "a saved script: at the cursor");
    }

    #[test]
    fn edition_comme_un_editeur_de_code() {
        let mut e = Editor::new("t", "", None);
        e.insert("if close > open");
        e.key(EdKey::Enter);
        assert_eq!(e.cur, (1, 4), "bloc : +4");
        e.insert("x := 1");
        e.key(EdKey::Enter);
        assert_eq!(e.cur, (2, 4), "garde l'indentation");
        e.key(EdKey::Backspace);
        assert_eq!(e.cur, (2, 0), "⌫ dans l'indentation : un cran");
        e.insert("plot(x)");
        assert_eq!(e.text(), "if close > open\n    x := 1\nplot(x)");
        e.key(EdKey::Home);
        e.key(EdKey::Backspace);
        assert_eq!(e.lines.len(), 2, "⌫ en début de ligne : joint");
        e.insert("\n");
        e.insert("a\tb\r\nc");
        assert_eq!(e.lines[2], "a    b");
        assert_eq!(e.lines[3], "cplot(x)");
        // couleurs : commentaire, chaîne, nombre, mot-clé, fonction
        let s = spans("x = ta.ema(close, 14) // ok \"z\"");
        assert!(s.iter().any(|(_, t, c)| *t == "ta.ema" && *c == CODE_FN));
        assert!(s.iter().any(|(_, t, c)| *t == "14" && *c == CODE_NUM));
        assert!(s.iter().any(|(_, t, c)| t.starts_with("//") && *c == CODE_COM));
        assert!(spans("var s = \"a\"").iter().any(|(_, t, c)| *t == "var" && *c == CODE_KW));
    }

    #[test]
    fn menu_champ_terminal_comme_v1() {
        let t = Text::default();
        let (w, h) = (1000.0, 700.0);
        let mut e = Editor::new("Mon script", TEMPLATE, None);
        // clic dans le code : curseur posé
        assert_eq!(e.click(&t, GUTTER + PAD + 2.0, HEAD + 4.0 + LINE * 1.5, w, h), EdClick::Caret);
        assert_eq!(e.cur, (1, 0));
        // 📜 nom ▾ : le menu ; 🗑️ demande une confirmation (la fenêtre arme `armed`)
        assert_eq!(e.click(&t, 30.0, 20.0, w, h), EdClick::None);
        assert!(e.menu);
        let del = e.items(&t).last().unwrap().0;
        assert_eq!(e.click(&t, del.cx(), del.cy(), w, h), EdClick::Item(EdItem::Delete));
        assert!(e.menu, "1er clic sur Supprimer : le menu reste pour confirmer");
        e.armed = true;
        assert_eq!(e.click(&t, del.cx(), del.cy(), w, h), EdClick::Item(EdItem::Delete));
        assert!(!e.menu);
        // champ « Renommer » : on tape, Entrée = OK ; Échap ferme le champ, puis la fenêtre
        e.extra = Extra::Prompt(EdItem::Rename, "RSI".into());
        e.insert(" perso");
        e.key(EdKey::Backspace);
        assert_eq!(e.extra, Extra::Prompt(EdItem::Rename, "RSI pers".into()));
        assert_eq!(e.key(EdKey::Enter), EdClick::PromptOk);
        assert!(!e.escape());
        assert!(e.escape());
        // terminal : heure HH:MM:SS, 40 messages au plus
        for k in 0..45 {
            e.log(3_600 * 13 + 61, format!("m{k}"), LogKind::Info);
        }
        assert_eq!(e.log.len(), LOG_MAX);
        assert_eq!(e.log[0].0, "13:01:01");
        // ▶ Ajouter à la chart
        let run = e.buttons(&t, w).into_iter().find(|(_, b)| *b == EdBtn::Run).unwrap().0;
        assert_eq!(e.click(&t, run.cx(), run.cy(), w, h), EdClick::Btn(EdBtn::Run));
        assert!(e.draw(&t, w, h, None).is_some());
    }

    /// { } (tableaux-1 tour 2, F8) A Rust indicator's code: its header keeps only the name, 📖 and ✕ (no « ▶ Ajouter
    /// à la chart », ✨ Claude, 🌲 Pine nor 📤); its first line says « Rust, lecture seule » first; 🧱 is drawn.
    #[test]
    fn code_rust_sans_boutons_de_script() {
        let t = Text::default();
        let src = crate::sources::source_rust(crate::indicators::Ind::ArgosMurs).expect("its Rust source");
        assert!(
            src.lines()
                .next()
                .is_some_and(|l| l.starts_with("// Rust, lecture seule · ") && l.ends_with("rust/crates/argos-data/src/murs.rs")),
            "{:?}",
            src.lines().next()
        );
        let mut e = Editor::new("Argos Murs (code Rust)", &src, None);
        (e.lecture_seule, e.rust) = (true, true);
        for w in [390.0, 820.0, 1440.0] {
            let b: Vec<EdBtn> = e.buttons(&t, w).into_iter().map(|(_, b)| b).collect();
            assert!(b.contains(&EdBtn::Close) && b.contains(&EdBtn::Menu), "{w}: {b:?}");
            assert!(
                !b.iter().any(|x| matches!(x, EdBtn::Run | EdBtn::Claude | EdBtn::Pine | EdBtn::Share)),
                "{w}: {b:?}"
            );
            assert!(e.draw(&t, w, 700.0, None).is_some());
        }
        // (🧱 in the code font: the app's emoji — in colour once its image is there, nothing before —, never the
        // font's empty box drawn in white)
        assert!(crate::emoji::at_start("🧱").is_some(), "🧱 is an emoji");
        let mut pm = Pixmap::new(40, 30).unwrap();
        Mono::draw(&mut pm, "🧱", (2.0, 20.0), SIZE, WHITE);
        let blanc = pm
            .pixels()
            .iter()
            .any(|p| p.alpha() > 0 && p.red() == p.green() && p.green() == p.blue() && p.red() > 200);
        let couleur = pm.pixels().iter().any(|p| p.alpha() > 0 && (p.red() != p.green() || p.green() != p.blue()));
        assert!(couleur || !blanc, "🧱 not drawn as a white box");
    }
}
