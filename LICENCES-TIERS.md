# 📜 Licences

**Argos HSS Trading** (app Rust, coque Tauri, modules, outils) est sous **PolyForm Noncommercial 1.0.0**,
© 2026 Jee (`Required Notice` en tête de [`LICENSE`](LICENSE)) : usage, copie et modification permis sans but
commercial ; tout usage commercial demande l'accord de Jee. Les versions publiées avant sous GPL-3.0 restent sous
GPL-3.0 pour ceux qui les ont reçues.

Les dépendances Rust (crates) gardent leur licence ; la liste de celles ajoutées depuis le début de `travail-local`
est dans [`docs/dependances-a-approuver.md`](docs/dependances-a-approuver.md) (aucune GPL / AGPL / LGPL : `cargo deny`
les refuse).

## Exceptions et code de tiers

Ces parties gardent **leur propre licence** (pas celle d'Argos) :

| Fichier(s) | Œuvre d'origine | Licence | Ce que ça demande |
|---|---|---|---|
| `rust/crates/argos-render/assets/twemoji/` | Twemoji, © Twitter / X et les contributeurs | CC BY 4.0 (images) | crédit (voir `LICENSE.txt` du dossier) |
| `emoji/` (servi à côté de l'app, `/emoji/…`) | Twemoji 15.0.0 complet (`@twemoji/svg`), © Twitter / X et les contributeurs | CC BY 4.0 (images) | crédit (voir `LICENSE.txt` du dossier) |
| `rust/tools/unicode/emoji-test-15.0.txt` | Liste des emoji d'Unicode 15.0 (catégories, ordre, noms courts) | Unicode License v3 | voir `LICENSE-Unicode.txt` du dossier |
| `rust/crates/argos-render/assets/*.ttf` | Inter, DejaVu Sans, Liberation Sans | SIL OFL 1.1 et licence DejaVu | voir les `LICENSE-*.txt` du dossier |

## Scripts Pine de tiers (`pine/`)

Ces scripts gardent **leur propre licence** et leur en-tête d'origine. `pine/references/luxalgo-smart-money-concepts-v6.pine`
est inclus dans le programme (`rust/crates/argos-script/src/integres.rs`) : ce crédit part avec chaque copie de l'app.

| Fichier(s) | Œuvre d'origine | Licence | Ce que ça demande |
|---|---|---|---|
| `pine/references/luxalgo-*.pine` (4 fichiers ; les v6 sont adaptés par Argos) | Smart Money Concepts et Liquidity Swings, © LuxAlgo | CC BY-NC-SA 4.0 | crédit, pas d'usage commercial, partage dans les mêmes conditions |
| `pine/references/zeiierman-*.pine` (2 fichiers) | Smart Swing VWAP, © Zeiierman | CC BY-NC-SA 4.0 | crédit, pas d'usage commercial, partage dans les mêmes conditions |
| `pine/references/viprasol-fib-ote-fvg.pine` | Fib OTE FVG, © Viprasol | MPL 2.0 | garder la licence et l'en-tête du fichier |
| `pine/Spring_Tension.pine`, `pine/Super_Scalp.pine` | © cgagnonswtor | MPL 2.0 | garder la licence et l'en-tête du fichier |
| `pine/CCQc_Cipher_Pro.pine` | Cipher Pro, © CCQc | MPL 2.0 | garder la licence et l'en-tête du fichier |
| `pine/tradingview/*.pine` | Indicateurs intégrés de TradingView (sans en-tête de licence) | non indiquée | à faire vérifier par un avocat avant tout usage hors test |

Le détail de chaque fichier (source, version, adaptations) est dans [`pine/references/SUIVI.md`](pine/references/SUIVI.md)
et [`pine/tradingview/SUIVI.md`](pine/tradingview/SUIVI.md).

## AI Agent Lens (crate `argos-lens`)

`rust/crates/argos-lens/` est un portage en Rust (réécrit, aucun code Pine copié) de l'indicateur public TradingView
« AI Agent Lens » de roan-maverick (dépôt `tradingview-indicator-for-ai-agents`), sous licence MIT. Les ajouts
d'Argos (paquet `argos-packet/v1`, chaîne d'empreintes, réutilisation des indicateurs d'argos-script) restent sous la
licence d'Argos HSS Trading. Notice d'origine gardée :

> MIT License
>
> Copyright (c) roan-maverick
>
> Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
> documentation files (the "Software"), to deal in the Software without restriction, including without limitation
> the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
> permit persons to whom the Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or substantial portions of
> the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
> THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
> TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

⚠️ L'année de la ligne « Copyright » n'a pas été relevée (pas d'accès réseau) : la recopier depuis le fichier
`LICENSE` du dépôt d'origine.

## Kit Argos

Copie figée du kit 1.0.2 dans `rust/vendor/kit-module-argos-1.0.2/` (voir `rust/vendor/SOURCE-kit-module-argos.md`).
Chaque partie garde **sa propre licence** et ses fichiers de licence :

| Partie | Licence |
|---|---|
| `argos-module-sdk` | MIT OR Apache-2.0, © 2026 Jee — fichiers `LICENSE-MIT` et `LICENSE-APACHE` gardés |
| `argos-module-policy`, `argos-sandbox` | PolyForm Noncommercial 1.0.0, © 2026 Jee (`LICENSE.md` ; usage commercial : `COMMERCIAL.md`) |

## Provenance des indicateurs

Voir [`docs/PROVENANCE.md`](docs/PROVENANCE.md) : déclaration de Jee du 6 novembre 2026 (indicateurs personnels
créés avec l'IA) et indicateurs réécrits de zéro le même jour. Un avocat doit vérifier avant toute vente.

Si tu es l'auteur d'une œuvre citée ici et qu'une mention manque ou est fausse, ouvre une *issue* : on corrige.
