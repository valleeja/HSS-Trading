//! 🎨 Icônes d'Argos v1 (les mêmes dessins SVG que le site : outils de dessin, types de chart, aimant, cadenas…),
//! dessinées avec resvg à la taille et la couleur voulues, puis gardées en mémoire.

use crate::text::Rgb;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tiny_skia::Pixmap;

/// id → dessin SVG (24 × 24, traits), copié tel quel d'Argos v1.
const ICONS: &[(&str, &str)] = &[
    ("abcd", "<path d=\"M3 18l6-12 5 7 7-9\"/>"),
    ("alarm", "<circle cx=\"12\" cy=\"13\" r=\"7\"/><path d=\"M12 10v3l2 2M5 4L2.5 6.5M19 4l2.5 2.5\"/><path d=\"M17 17h4M19 15v4\" stroke-width=\"1.6\"/>"),
    ("amdn", "<path d=\"M12 20l6-7h-4V4h-4v9H6z\"/>"),
    ("amleft", "<path d=\"M4 12l7-6v4h9v4h-9v4z\"/>"),
    ("amright", "<path d=\"M20 12l-7-6v4H4v4h9v4z\"/>"),
    ("amup", "<path d=\"M12 4l6 7h-4v9h-4v-9H6z\"/>"),
    ("arc", "<path d=\"M3 18a10 10 0 0 1 18 0\"/>"),
    ("area", "<path d=\"M3 17l5-6 4 3 8-9v15H3z\" fill=\"currentColor\" fill-opacity=\".25\"/><path d=\"M3 17l5-6 4 3 8-9\"/>"),
    ("arrow", "<path d=\"M4 20L19 5\"/><path d=\"M12 5h7v7\"/>"),
    ("arrowdn", "<path d=\"M12 20l-7-8h4V4h6v8h4z\"/>"),
    ("arrowmarker", "<path d=\"M4 20L16 8\"/><path d=\"M12 6h8v8z\" fill=\"currentColor\"/>"),
    ("arrowup", "<path d=\"M12 4l7 8h-4v8H9v-8H5z\"/>"),
    ("avp", "<path d=\"M20 4v16M20 6h-6M20 10h-11M20 14h-8M20 18h-4\"/><circle cx=\"20\" cy=\"4\" r=\"1.5\"/>"),
    ("avwap", "<path d=\"M6 4v16\"/><path d=\"M6 14c4-1 6-6 14-6\" /><rect x=\"4\" y=\"9\" width=\"4\" height=\"6\" rx=\"1\"/>"),
    ("bars", "<path d=\"M7 4v16M4 8h3M7 15h3M17 3v16M14 6h3M17 13h3\"/>"),
    ("barspattern", "<path d=\"M5 6v12M9 9v8M13 5v10M17 8v9\"/><rect x=\"4\" y=\"9\" width=\"2\" height=\"5\"/><rect x=\"12\" y=\"7\" width=\"2\" height=\"5\"/>"),
    ("baseline", "<path d=\"M3 12h18\" stroke-dasharray=\"2 2\"/><path d=\"M3 15l4-6 4 6 4-8 5 5\"/>"),
    ("brush", "<path d=\"M4 20c2 0 4-1 4-4 0-2 2-3 3-2s1 3-1 4\"/><path d=\"M11 14l9-10\"/>"),
    ("callout", "<path d=\"M4 4h16v11H11l-5 5v-5H4z\"/>"),
    ("candles", "<path d=\"M7 4v3M7 17v3M17 3v4M17 15v5\"/><rect x=\"5\" y=\"7\" width=\"4\" height=\"10\" rx=\".5\"/><rect x=\"15\" y=\"7\" width=\"4\" height=\"8\" rx=\".5\" fill=\"currentColor\"/>"),
    ("channel", "<path d=\"M3 15L15 4M9 20L21 9\"/><circle cx=\"3\" cy=\"15\" r=\"1.6\"/><circle cx=\"15\" cy=\"4\" r=\"1.6\"/><circle cx=\"21\" cy=\"9\" r=\"1.6\"/>"),
    // 🎯 the Stratégie volet (Argos' own drawing: three rings)
    ("cible", "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><circle cx=\"12\" cy=\"12\" r=\"5\"/><circle cx=\"12\" cy=\"12\" r=\"1.6\"/>"),
    ("circle", "<circle cx=\"12\" cy=\"12\" r=\"8\"/><circle cx=\"12\" cy=\"12\" r=\"1\"/>"),
    ("close", "<path d=\"M6 6l12 12M18 6L6 18\"/>"),
    ("code", "<path d=\"M9 7l-5 5 5 5M15 7l5 5-5 5\"/>"),
    ("columns", "<path d=\"M5 20v-6h3v6M10.5 20V8h3v12M16 20v-9h3v9\"/>"),
    ("comment", "<rect x=\"3\" y=\"5\" width=\"18\" height=\"11\" rx=\"5.5\"/><path d=\"M7 16v4l4-4\"/>"),
    ("cross", "<path d=\"M12 3v18M3 12h18\"/><circle cx=\"12\" cy=\"12\" r=\"1.8\"/>"),
    ("cursor", "<path d=\"M12 3v18M3 12h18\"/>"),
    // 🖱 curseurs « Point » et « Flèche » (1er bouton de la barre de gauche, comme TradingView)
    // { } code de l'indicateur (légende), au même trait que 👁 ⚙
    ("braces", "<path d=\"M9 4C7 4 6 5 6 7v2c0 1.5-1 2.5-2.5 3 1.5.5 2.5 1.5 2.5 3v2c0 2 1 3 3 3\"/><path d=\"M15 4c2 0 3 1 3 3v2c0 1.5 1 2.5 2.5 3-1.5.5-2.5 1.5-2.5 3v2c0 2-1 3-3 3\"/>"),
    ("curseur-point", "<circle cx=\"12\" cy=\"12\" r=\"2.6\" fill=\"currentColor\"/>"),
    ("curseur-fleche", "<path d=\"M6 3l12 9-6 1.2L9 19z\"/>"),
    ("curve", "<path d=\"M3 18C6 4 14 4 21 12\"/>"),
    ("cyclic", "<path d=\"M4 4v16M9 4v16M14 4v16M19 4v16\"/>"),
    ("cypher", "<path d=\"M3 18l5-12 3 6 6-6 4 12\"/><text x=\"9\" y=\"22\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">C</text>"),
    ("daterange", "<path d=\"M4 12h16M4 8v8M20 8v8\"/><path d=\"M17 9l3 3-3 3\"/>"),
    ("dedekind", "<path d=\"M2 19h20\"/><path d=\"M2 19a10 10 0 0 1 20 0M2 19a5 5 0 0 1 10 0M12 19a5 5 0 0 1 10 0M8.7 19a1.7 1.7 0 0 1 3.3 0M12 19a1.7 1.7 0 0 1 3.3 0\"/>"),
    ("dcurve", "<path d=\"M3 16c3-10 7-10 9-4s6 6 9-4\"/>"),
    ("disjoint", "<path d=\"M3 9l18-5M3 15l18 5\"/>"),
    ("dprange", "<rect x=\"4\" y=\"5\" width=\"16\" height=\"14\" rx=\"1\"/><path d=\"M12 7v10M7 12h10M10 9l2-2 2 2M10 15l2 2 2-2\"/>"),
    ("ellipse", "<ellipse cx=\"12\" cy=\"12\" rx=\"9\" ry=\"6\"/>"),
    ("erase", "<path d=\"M7 20h12M5 14l8-8 6 6-6 6H9z\"/>"),
    ("ewcorr", "<path d=\"M3 6l6 10 4-5 8 9\"/><text x=\"8\" y=\"22\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">A</text>"),
    ("ewdouble", "<path d=\"M3 6l6 11 4-5 8 8\"/><text x=\"15\" y=\"8\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">W</text>"),
    ("ewimp", "<path d=\"M2 19l4-7 3 3 5-10 3 4 5-6\"/><text x=\"2\" y=\"8\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">1</text><text x=\"18\" y=\"5\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">5</text>"),
    ("ewtri", "<path d=\"M3 5l4 14 3-10 3 7 3-4 3 3\"/>"),
    ("ewtriple", "<path d=\"M2 5l4 12 3-5 4 8 3-4 5 5\"/>"),
    ("ext", "<path d=\"M2 21L22 3\"/><circle cx=\"8\" cy=\"16\" r=\"1.8\"/><circle cx=\"16\" cy=\"9\" r=\"1.8\"/>"),
    ("eye2", "<path d=\"M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/>"),
    ("eyeoff2", "<path d=\"M3 3l18 18M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3 3.9M6.3 6.3C3.7 8 2 12 2 12s3.5 7 10 7c1.6 0 3-.4 4.3-1\"/>"),
    ("fib", "<path d=\"M3 5h18M3 10h18M3 14h18M3 19h18\"/><path d=\"M5 19L19 5\" stroke-dasharray=\"2 2\"/>"),
    ("fibarcs", "<path d=\"M4 18a8 8 0 0 1 16 0M7 18a5 5 0 0 1 10 0M10 18a2 2 0 0 1 4 0\"/>"),
    ("fibch", "<path d=\"M3 13L14 3M3 17L17 5M3 21L20 7\" /><path d=\"M6 21L21 11\" stroke-dasharray=\"2 2\"/>"),
    ("fibcircles", "<circle cx=\"12\" cy=\"12\" r=\"3\"/><circle cx=\"12\" cy=\"12\" r=\"6\"/><circle cx=\"12\" cy=\"12\" r=\"9\"/>"),
    ("fibext", "<path d=\"M3 5h18M3 10h18M3 15h18M3 20h18\"/><path d=\"M4 20l5-10 4 5\" opacity=\".6\"/>"),
    ("fibfan", "<path d=\"M3 20L21 4M3 20L21 9M3 20L21 14M3 20L21 18\"/>"),
    ("fibspiral", "<path d=\"M12 12a2 2 0 1 1 2 2 4 4 0 0 1-6-2 6 6 0 0 1 8-6 8 8 0 0 1 2 12\"/>"),
    ("fibtime", "<path d=\"M5 4v16M8 4v16M11 4v16M15 4v16M21 4v16\"/>"),
    ("fibtrendtime", "<path d=\"M6 4v16M11 4v16M18 4v16\"/><path d=\"M3 18l4-8 4 6\" opacity=\".6\"/>"),
    ("fibwedge", "<path d=\"M4 19L20 7M4 19h16\"/><path d=\"M10 19a6 6 0 0 0-1.4-4M15 19a11 11 0 0 0-2.8-7.4\"/>"),
    ("flag", "<path d=\"M5 21V4M5 4h12l-3 4 3 4H5\"/>"),
    ("flattop", "<path d=\"M3 7h18M3 18L21 11\"/><circle cx=\"3\" cy=\"18\" r=\"1.4\"/><circle cx=\"21\" cy=\"11\" r=\"1.4\"/>"),
    ("footprint", "<path d=\"M12 3v18\"/><path d=\"M5 6h6M5 10h6M5 14h6M5 18h6M13 6h4M13 10h6M13 14h5M13 18h3\"/>"),
    ("fvg", "<path d=\"M6 3v6M6 15v6M18 3v6M18 15v6\"/><rect x=\"4\" y=\"9\" width=\"16\" height=\"6\" fill=\"currentColor\" fill-opacity=\".25\"/>"),
    ("fvp", "<path d=\"M4 4v16M4 6h7M4 10h12M4 14h9M4 18h5\"/><path d=\"M20 4v16\" opacity=\".5\"/>"),
    ("gannbox", "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\"/><path d=\"M9 4v16M15 4v16M3 9h18M3 15h18\"/>"),
    ("gannfan", "<path d=\"M3 20L21 4M3 20L21 10M3 20L21 15M3 20L16 3M3 20L10 3\"/>"),
    ("gannsq", "<rect x=\"4\" y=\"4\" width=\"16\" height=\"16\"/><path d=\"M4 4l16 16M20 4L4 20\"/><circle cx=\"12\" cy=\"12\" r=\"5\"/>"),
    ("gannsqfixed", "<rect x=\"4\" y=\"4\" width=\"16\" height=\"16\"/><path d=\"M4 4l16 16M20 4L4 20M9.3 4v16M14.7 4v16\"/>"),
    ("gear", "<path d=\"M12 2.8l8 4.6v9.2l-8 4.6-8-4.6V7.4z\"/><circle cx=\"12\" cy=\"12\" r=\"3\"/>"),
    ("ghostfeed", "<path d=\"M6 8v8M10 6v10M14 9v7M18 7v9\" stroke-dasharray=\"2 2\"/><rect x=\"5\" y=\"10\" width=\"2\" height=\"4\"/><rect x=\"13\" y=\"11\" width=\"2\" height=\"3\"/>"),
    ("ha", "<path d=\"M7 5v14M17 3v18\"/><rect x=\"5\" y=\"8\" width=\"4\" height=\"7\" rx=\".5\"/><rect x=\"15\" y=\"6\" width=\"4\" height=\"10\" rx=\".5\" fill=\"currentColor\"/>"),
    ("highlighter", "<path d=\"M9 14l-3 6h5l1-3\"/><path d=\"M9 14l8-10 4 3-8 10z\"/>"),
    ("hilo", "<rect x=\"5\" y=\"5\" width=\"4\" height=\"12\" rx=\".5\"/><rect x=\"15\" y=\"8\" width=\"4\" height=\"9\" rx=\".5\"/>"),
    ("hlcarea", "<path d=\"M3 13l5-5 4 3 9-7M3 19l5-4 4 2 9-6\" stroke-opacity=\".7\"/><path d=\"M3 13l5-5 4 3 9-7v6l-9 6-4-2-5 4z\" fill=\"currentColor\" fill-opacity=\".2\" stroke=\"none\"/>"),
    ("hline", "<path d=\"M3 12h18\"/><circle cx=\"12\" cy=\"12\" r=\"1.6\" fill=\"currentColor\"/>"),
    ("hollow", "<path d=\"M7 4v3M7 17v3M17 3v4M17 15v5\"/><rect x=\"5\" y=\"7\" width=\"4\" height=\"10\" rx=\".5\"/><rect x=\"15\" y=\"7\" width=\"4\" height=\"8\" rx=\".5\"/>"),
    ("hray", "<path d=\"M6 12h16\"/><circle cx=\"5\" cy=\"12\" r=\"1.8\"/>"),
    ("hs", "<path d=\"M2 18l3-6 3 3 4-10 4 10 3-3 3 6\"/>"),
    ("ibook", "<path d=\"M7 3.5h10v17l-5-4-5 4z\"/>"),
    ("icon", "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><circle cx=\"9\" cy=\"10\" r=\"1\" fill=\"currentColor\"/><circle cx=\"15\" cy=\"10\" r=\"1\" fill=\"currentColor\"/><path d=\"M8.5 14.5c1.8 2 5.2 2 7 0\"/>"),
    ("ifund", "<path d=\"M4 20h16M6 20v-6h3v6M10.5 20V9h3v11M15 20V5h3v15\"/>"),
    ("image", "<rect x=\"3.5\" y=\"5\" width=\"17\" height=\"14\" rx=\"1.5\"/><circle cx=\"9\" cy=\"10\" r=\"1.6\"/><path d=\"M4 17l5-4 3 2.5 3-3 5 4.5\"/>"),
    ("infoline", "<path d=\"M4 18L15 7\"/><rect x=\"13\" y=\"13\" width=\"8\" height=\"6\" rx=\"1.5\"/><circle cx=\"4\" cy=\"18\" r=\"1.6\"/><circle cx=\"15\" cy=\"7\" r=\"1.6\"/>"),
    ("insidepf", "<path d=\"M3 13h18M8 8l13 0M8 18h13M3 13l5-5M3 13l5 5\"/>"),
    ("iplus", "<path d=\"M12 5v14M5 12h14\"/>"),
    ("istar", "<path d=\"M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.8L12 16.8l-5.2 2.8 1-5.8-4.3-4.1 5.9-.8z\"/>"),
    ("itech", "<path d=\"M3 13l4-4 3 3 4-5 3 3 4-4\"/><path d=\"M4 20v-3M7 20v-5M10 20v-4M13 20v-6M16 20v-4M19 20v-5\"/>"),
    ("itop", "<path d=\"M4 20h16M6 20v-5h3v5M10.5 20v-8h3v8M15 20V7h3v13M15 7l3-3\"/>"),
    ("iuser", "<circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M4 20c1.5-4 4.5-5.5 8-5.5s6.5 1.5 8 5.5\"/>"),
    ("kagi", "<path d=\"M3 20v-8h4V4h4v12h4V8h4v9\"/>"),
    ("keep", "<path d=\"M14 4l6 6L9 21H3v-6z\"/><path d=\"M15 14.5v-1a2 2 0 0 1 4 0v1\"/><rect x=\"14\" y=\"14.5\" width=\"6\" height=\"5\" rx=\"1\"/>"),
    ("lbreak", "<rect x=\"4\" y=\"11\" width=\"4\" height=\"9\"/><rect x=\"10\" y=\"4\" width=\"4\" height=\"9\" fill=\"currentColor\"/><rect x=\"16\" y=\"7\" width=\"4\" height=\"7\"/>"),
    ("line", "<path d=\"M3 17l5-6 4 3 8-9\"/>"),
    ("liq", "<path d=\"M3 9h14\" stroke-dasharray=\"2 2\"/><path d=\"M12 4v10M10 6h3a1.5 1.5 0 0 1 0 3h-2a1.5 1.5 0 0 0 0 3h3\"/><path d=\"M3 19h18\"/>"),
    ("linemk", "<path d=\"M3 17l5-6 4 3 8-9\"/><circle cx=\"8\" cy=\"11\" r=\"1.6\"/><circle cx=\"12\" cy=\"14\" r=\"1.6\"/><circle cx=\"20\" cy=\"5\" r=\"1.6\"/>"),
    ("lines", "<path d=\"M4 19L20 5\"/><circle cx=\"4\" cy=\"19\" r=\"1.8\"/><circle cx=\"20\" cy=\"5\" r=\"1.8\"/>"),
    ("lock", "<rect x=\"5\" y=\"11\" width=\"14\" height=\"9\" rx=\"1.5\"/><path d=\"M8 11V8a4 4 0 0 1 8 0v3\"/>"),
    ("lock2", "<rect x=\"5\" y=\"11\" width=\"14\" height=\"10\" rx=\"2\"/><path d=\"M8 11V7a4 4 0 0 1 8 0v4\"/>"),
    ("long", "<rect x=\"4\" y=\"5\" width=\"16\" height=\"7\" fill=\"currentColor\" fill-opacity=\".25\"/><rect x=\"4\" y=\"12\" width=\"16\" height=\"6\"/><path d=\"M4 12h16\"/>"),
    ("magnet", "<path d=\"M6 4v8a6 6 0 0 0 12 0V4h-4v8a2 2 0 0 1-4 0V4z\"/>"),
    ("magnet2", "<path d=\"M6 4v7a6 6 0 0 0 12 0V4\"/><path d=\"M6 8h4M14 8h4\"/><path d=\"M9 20l1.5-2M12 21v-2M15 20l-1.5-2\" stroke-width=\"1.3\"/>"),
    ("measure", "<path d=\"M4 17L17 4l3 3L7 20z\"/><path d=\"M8 13l2 2M11 10l2 2M14 7l2 2\"/>"),
    ("more", "<circle cx=\"6\" cy=\"12\" r=\"1.3\" fill=\"currentColor\"/><circle cx=\"12\" cy=\"12\" r=\"1.3\" fill=\"currentColor\"/><circle cx=\"18\" cy=\"12\" r=\"1.3\" fill=\"currentColor\"/>"),
    ("move", "<path d=\"M12 3v18M3 12h18M9.5 5.5L12 3l2.5 2.5M9.5 18.5L12 21l2.5-2.5M5.5 9.5L3 12l2.5 2.5M18.5 9.5L21 12l-2.5 2.5\"/>"),
    ("mschiff", "<path d=\"M3 18l7-5M10 13l11-7M10 13h11M10 13l11 7\"/><circle cx=\"10\" cy=\"13\" r=\"1.4\"/>"),
    ("note", "<rect x=\"4\" y=\"3\" width=\"16\" height=\"13\" rx=\"1.5\"/><path d=\"M8 8h8M8 11h5M10 16l-2 4\"/>"),
    ("ob", "<path d=\"M7 4v3M7 17v3\"/><rect x=\"5\" y=\"7\" width=\"4\" height=\"10\" fill=\"currentColor\"/><rect x=\"9\" y=\"7\" width=\"12\" height=\"10\" fill=\"currentColor\" fill-opacity=\".2\"/>"),
    ("path", "<path d=\"M3 18l5-8 5 5 6-9\"/><path d=\"M15 6h4v4\"/>"),
    ("pencil", "<path d=\"M15 4l5 5L9 20H4v-5z\"/><path d=\"M13 6l5 5\"/>"),
    ("pin", "<path d=\"M12 21s-6-6-6-11a6 6 0 0 1 12 0c0 5-6 11-6 11z\"/><circle cx=\"12\" cy=\"10\" r=\"2\"/>"),
    ("pitchfan", "<path d=\"M3 20L20 4M3 20L21 9M3 20L21 15M3 20l15 1\"/>"),
    ("pitchfork", "<path d=\"M3 13h7M10 13l11-7M10 13h11M10 13l11 7\"/>"),
    ("pnf", "<path d=\"M4 4l3 3M7 4L4 7M4 10l3 3M7 10l-3 3M4 16l3 3M7 16l-3 3\"/><circle cx=\"12\" cy=\"8.5\" r=\"1.8\"/><circle cx=\"12\" cy=\"14.5\" r=\"1.8\"/><path d=\"M17 4l3 3M20 4l-3 3M17 10l3 3M20 10l-3 3\"/>"),
    ("polyline", "<path d=\"M3 17l5-10 6 7 7-9\"/><circle cx=\"3\" cy=\"17\" r=\"1.4\"/><circle cx=\"8\" cy=\"7\" r=\"1.4\"/><circle cx=\"14\" cy=\"14\" r=\"1.4\"/><circle cx=\"21\" cy=\"5\" r=\"1.4\"/>"),
    ("posforecast", "<path d=\"M4 18L18 7\"/><path d=\"M14 6h5v5\"/><rect x=\"3\" y=\"15\" width=\"5\" height=\"5\" rx=\"1\"/>"),
    ("pricelabel", "<path d=\"M3 12l5-6h13v12H8z\"/>"),
    ("pricenote", "<path d=\"M4 19l7-7\"/><rect x=\"11\" y=\"5\" width=\"10\" height=\"7\" rx=\"1.5\"/><circle cx=\"4\" cy=\"19\" r=\"1.5\"/>"),
    ("pricerange", "<path d=\"M12 4v16M8 4h8M8 20h8\"/><path d=\"M9 7l3-3 3 3\"/>"),
    ("range", "<path d=\"M6 4v16M3 8h3M6 15h3M15 6v12M12 9h3M15 14h3\"/><path d=\"M20 4v6\" stroke-width=\"1\"/><text x=\"18\" y=\"22\" font-size=\"6\" fill=\"currentColor\" stroke=\"none\">R</text>"),
    ("ray", "<path d=\"M5 18L21 4\"/><circle cx=\"5\" cy=\"18\" r=\"1.8\"/><circle cx=\"12\" cy=\"12\" r=\"1.8\"/>"),
    ("rect", "<rect x=\"4\" y=\"6\" width=\"16\" height=\"12\" rx=\"1\"/>"),
    ("regression", "<path d=\"M3 15l18-8M3 19l18-8M3 11l18-8\" opacity=\".9\"/><path d=\"M3 15l18-8\" stroke-dasharray=\"2 2\"/>"),
    ("renko", "<rect x=\"3\" y=\"15\" width=\"5\" height=\"5\"/><rect x=\"8\" y=\"10\" width=\"5\" height=\"5\"/><rect x=\"13\" y=\"5\" width=\"5\" height=\"5\" fill=\"currentColor\"/>"),
    ("rotrect", "<path d=\"M9 3l12 6-6 12L3 15z\"/>"),
    ("schiff", "<path d=\"M3 16l6-3M9 13l12-7M9 13h12M9 13l12 7\"/>"),
    ("search", "<circle cx=\"11\" cy=\"11\" r=\"6.5\"/><path d=\"M16 16l4.5 4.5\"/>"),
    ("sector", "<path d=\"M4 20V6a14 14 0 0 1 14 14z\"/>"),
    ("short", "<rect x=\"4\" y=\"6\" width=\"16\" height=\"6\"/><rect x=\"4\" y=\"12\" width=\"16\" height=\"7\" fill=\"currentColor\" fill-opacity=\".25\"/><path d=\"M4 12h16\"/>"),
    ("signpost", "<path d=\"M12 21V10\"/><rect x=\"5\" y=\"3\" width=\"14\" height=\"7\" rx=\"1\"/>"),
    ("sine", "<path d=\"M2 12c3-8 6-8 7 0s4 8 7 0 4-8 6-2\"/>"),
    ("sonic", "<path d=\"M4 3l8 18\"/><circle cx=\"14\" cy=\"10\" r=\"4\"/><circle cx=\"16\" cy=\"9\" r=\"6.5\" opacity=\".6\"/>"),
    ("step", "<path d=\"M3 18h5v-6h5v-5h8\"/>"),
    ("table", "<rect x=\"3.5\" y=\"5\" width=\"17\" height=\"14\" rx=\"1\"/><path d=\"M3.5 10h17M3.5 14.5h17M10 5v14\"/>"),
    ("text", "<path d=\"M5 5h14M12 5v14M9 19h6\"/>"),
    ("threedrives", "<path d=\"M2 18l3-5 2 2 4-7 2 3 5-8 3 3\"/>"),
    ("timecycles", "<path d=\"M2 16a4 4 0 0 1 8 0M10 16a4 4 0 0 1 8 0M18 16a3 3 0 0 1 4-2.8\"/>"),
    ("tpl", "<rect x=\"4\" y=\"4\" width=\"6\" height=\"6\" rx=\"1\"/><rect x=\"14\" y=\"4\" width=\"6\" height=\"6\" rx=\"1\"/><rect x=\"4\" y=\"14\" width=\"6\" height=\"6\" rx=\"1\"/><path d=\"M17 14v6M14 17h6\"/>"),
    ("tpo", "<path d=\"M4 20V4\"/><rect x=\"6\" y=\"5\" width=\"4\" height=\"4\"/><rect x=\"6\" y=\"10\" width=\"4\" height=\"4\"/><rect x=\"11\" y=\"10\" width=\"4\" height=\"4\"/><rect x=\"16\" y=\"10\" width=\"4\" height=\"4\"/><rect x=\"6\" y=\"15\" width=\"4\" height=\"4\"/><rect x=\"11\" y=\"15\" width=\"4\" height=\"4\"/>"),
    ("trash", "<path d=\"M4 7h16M9 7V4h6v3M6 7l1 13h10l1-13\"/>"),
    // 🔔 the alert line menu (right-click): pause / resume, the « Lignes d'alerte » checkbox
    ("pause", "<path d=\"M9 6v12M15 6v12\"/>"),
    ("play", "<path d=\"M8 5l11 7-11 7z\"/>"),
    ("case", "<rect x=\"5\" y=\"5\" width=\"14\" height=\"14\" rx=\"3\"/>"),
    ("case-cochee", "<rect x=\"5\" y=\"5\" width=\"14\" height=\"14\" rx=\"3\"/><path d=\"M8.5 12.2l2.4 2.4 4.6-5\"/>"),
    // 🔊 the alert sound (menu of an alert line ▸ « Son… »)
    ("son", "<path d=\"M4 9.5h3.5L12 6v12l-4.5-3.5H4z\"/><path d=\"M15.5 9.5a3.5 3.5 0 0 1 0 5\"/><path d=\"M18 7a7 7 0 0 1 0 10\"/>"),
    ("trend", "<path d=\"M4 19L20 5\"/><circle cx=\"4\" cy=\"19\" r=\"1.8\"/><circle cx=\"20\" cy=\"5\" r=\"1.8\"/>"),
    ("trendangle", "<path d=\"M4 19L19 6M4 19h15\"/><path d=\"M11 19a7 7 0 0 0-2.2-5\"/>"),
    ("triangle", "<path d=\"M12 4l9 16H3z\"/>"),
    ("tripattern", "<path d=\"M3 6l6 12 4-9 4 6 4-3\"/><path d=\"M3 6l18 6M9 18l12-6\" opacity=\".5\"/>"),
    ("undo", "<path d=\"M9 7L4 12l5 5\"/><path d=\"M4 12h10a6 6 0 0 1 0 12\"/>"),
    ("redo", "<path d=\"M15 7l5 5-5 5\"/><path d=\"M20 12H10a6 6 0 0 0 0 12\"/>"),
    ("unlock", "<rect x=\"5\" y=\"11\" width=\"14\" height=\"9\" rx=\"1.5\"/><path d=\"M8 11V8a4 4 0 0 1 7.5-2\"/>"),
    ("unlock2", "<rect x=\"5\" y=\"11\" width=\"14\" height=\"10\" rx=\"2\"/><path d=\"M8 11V7a4 4 0 0 1 7.5-2\"/>"),
    ("vector", "<path d=\"M6 5v14M12 3v18M18 7v10\"/><rect x=\"4\" y=\"8\" width=\"4\" height=\"8\" rx=\".5\" fill=\"currentColor\"/><rect x=\"10\" y=\"6\" width=\"4\" height=\"12\" rx=\".5\"/><rect x=\"16\" y=\"9\" width=\"4\" height=\"5\" rx=\".5\"/>"),
    ("vline", "<path d=\"M12 3v18\"/><circle cx=\"12\" cy=\"12\" r=\"1.6\" fill=\"currentColor\"/>"),
    ("xabcd", "<path d=\"M3 17l4-11 4 7 5-6 5 10\"/><path d=\"M3 17l8-4M11 13l10 4\" opacity=\".5\"/>"),
    ("zoom", "<circle cx=\"10.5\" cy=\"10.5\" r=\"6.5\"/><path d=\"M15.5 15.5L21 21M10.5 7.5v6M7.5 10.5h6\"/>"),
];

/// Outils qui reprennent l'icône d'un autre (même famille).
const SAME_AS: &[(&str, &str)] = &[
    ("fibext2", "fibext"),
    ("gartley", "xabcd"),
    ("bat", "xabcd"),
    ("butterfly", "xabcd"),
    ("crab", "xabcd"),
    ("shark", "xabcd"),
    ("supersonic", "sonic"),
    ("gsonic", "sonic"),
    ("gsupersonic", "sonic"),
    ("loupe", "zoom"),
    ("aicapture", ARGOS_A),
];

/// L'id de l'icône vraiment dessinée pour `id`.
fn resolve(id: &str) -> &str {
    SAME_AS.iter().find(|(k, _)| *k == id).map_or(id, |(_, v)| *v)
}

/// Le dessin SVG d'une icône.
pub fn svg(id: &str) -> Option<&'static str> {
    let id = resolve(id);
    ICONS.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

/// L'outil a-t-il une icône (un dessin SVG, ou le A d'Argos) ?
pub fn has(id: &str) -> bool {
    svg(id).is_some() || resolve(id) == ARGOS_A
}

type Key = (String, u32, (u8, u8, u8));

fn cache() -> &'static Mutex<HashMap<Key, Pixmap>> {
    static C: OnceLock<Mutex<HashMap<Key, Pixmap>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Icône carrée de `size` pixels, traits de couleur `c` (comme `.ib svg` d'Argos v1 : trait 1,4, bouts ronds).
pub fn icon(id: &str, size: u32, c: Rgb) -> Option<Pixmap> {
    let key = (id.to_string(), size, (c.0, c.1, c.2));
    if let Some(p) = cache().lock().ok()?.get(&key) {
        return Some(p.clone());
    }
    let body = svg(id)?;
    let doc = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="{size}" height="{size}" fill="none" stroke="#{:02x}{:02x}{:02x}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##,
        c.0, c.1, c.2
    );
    let tree = resvg::usvg::Tree::from_str(&doc, &resvg::usvg::Options::default()).ok()?;
    let mut pm = Pixmap::new(size, size)?;
    resvg::render(&tree, tiny_skia::Transform::identity(), &mut pm.as_mut()); // `size` : pixels de l'écran
    if let Ok(mut c) = cache().lock() {
        c.insert(key, pm.clone());
    }
    Some(pm)
}

/// Id de l'icône Argos AI : l'icône de tous les boutons d'assistance.
pub const ARGOS_A: &str = "argos-a";

/// Pose une icône centrée sur `(cx, cy)`.
/// Un émoji d'interface (« 🔒 ») prend son icône Phosphor ; un autre émoji se dessine en couleur.
pub fn draw(pm: &mut Pixmap, id: &str, cx: f64, cy: f64, size: u32, c: Rgb) {
    let id = resolve(id);
    if id == ARGOS_A {
        return crate::aibtn::ai_icon(pm, cx, cy, size as f64);
    }
    // émoji d'interface (« 📅 ») : son icône Phosphor au trait fin, à la couleur demandée
    if let Some(name) = crate::phosphor::from_emoji(id) {
        crate::phosphor::draw(pm, name, cx, cy, size, c);
        return;
    }
    if id.is_ascii() {
        put(
            pm,
            icon(id, crate::px::dev(size as f64).round() as u32, crate::palette::icon_tint(id, c)),
            cx,
            cy,
        );
    } else {
        draw_emoji(pm, id, cx, cy, size);
    }
}

/// Pose un émoji de contenu en couleur (sélecteur d'émojis, drapeaux, marqueurs), jamais remplacé par une icône.
pub fn draw_emoji(pm: &mut Pixmap, e: &str, cx: f64, cy: f64, size: u32) {
    put(
        pm,
        crate::emoji::image(e, crate::px::dev(size.saturating_sub(4).max(8) as f64).round() as u32),
        cx,
        cy,
    );
}

fn put(pm: &mut Pixmap, ic: Option<Pixmap>, cx: f64, cy: f64) {
    if let Some(ic) = ic {
        // image déjà en pixels de l'écran : centrée en points
        let half = crate::px::lw(&ic) / 2.0;
        crate::px::put(pm, cx - half, cy - half, &ic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icones_v1_dessinees() {
        let p = icon("trend", 24, Rgb(255, 255, 255)).expect("icône de la ligne de tendance");
        assert!(p.pixels().iter().any(|px| px.alpha() > 0), "des traits dessinés");
        assert!(icon("inconnue", 24, Rgb(255, 255, 255)).is_none());
        assert!(icon("redo", 24, Rgb(255, 255, 255)).is_some(), "↷ refaire");
        for c in ["curseur-point", "curseur-fleche", "cible"] {
            let p = icon(c, 24, Rgb(255, 255, 255)).expect(c);
            assert!(p.pixels().iter().any(|px| px.alpha() > 0), "{c} dessiné");
        }
        assert_eq!(
            ICONS.len(),
            157,
            "137 de v1 + déplacer, code, fermer (légende LuxAlgo) + footprint, TPO + FVG, OB, liquidité + Dedekind, Sonic + refaire \
             + curseurs Point et Flèche + {{ }} du code + pause, lecture, case et case cochée (menu d'une ligne d'alerte) + son \
             + cible (🎯 Stratégie)"
        );
    }
}
