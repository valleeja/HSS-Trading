# Provenance des indicateurs d'Argos HSS Trading

> Licence d'Argos HSS Trading : PolyForm Noncommercial 1.0.0, © 2026 Jee (voir `LICENSE` et `LICENCES-TIERS.md`).

## Déclaration de Jee du 6 novembre 2026

Jee déclare que les six indicateurs ci-dessous sont **ses indicateurs personnels, créés avec l'IA** :

| Indicateur | Fichier(s) |
|---|---|
| Argos Cipher Pro | `rust/crates/argos-script/src/indicators/cipher.rs` |
| Argos TBO Pro Div | `rust/crates/argos-script/src/indicators/tbo.rs` |
| Argos AI Trend V Pro | `rust/crates/argos-script/src/indicators/ai_trend.rs` |
| Argos SMC (Pro) | `rust/crates/argos-script/src/indicators/smc_pro.rs` |
| Argos Bubbles | `rust/crates/argos-script/src/indicators/flow.rs` |
| Argos Tape | `rust/crates/argos-script/src/indicators/flow.rs` |

⚖️ **À faire vérifier par un avocat avant toute vente** : des commentaires du code (et l'historique git) mentionnent
une ressemblance avec des scripts publics de TradingView (par exemple la logique d'un oscillateur à vagues pour le
Cipher, un ruban et des bulles de déséquilibre pour le flux). Cet historique est gardé tel quel ; il n'est pas réécrit.

## Réécrits de zéro le 6 novembre 2026 (conception d'Argos)

Le code d'autres auteurs a été retiré et remplacé par du code écrit à partir des idées générales du trading :

| Indicateur (nom affiché) | Nouveau code | Remplace |
|---|---|---|
| Argos Pivot VWAP, Argos Pivot VWAP Multi | `indicators/pivot_vwap.rs` | deux transcriptions sous CC BY-NC-SA 4.0 (retirées) |
| Argos Price Action (order blocks d'Argos) | `indicators/order_blocks.rs` | un port de script tiers (retiré) |
| Argos Bollinger Oscillator (force des cassures) | `indicators/classic.rs` | un port de script tiers (retiré) |
| Argos Heatmap Volume (chaleur du volume) | `indicators/classic.rs` | un port de script tiers (retiré) |
| Argos Volume Profile | `indicators/classic.rs` | un port de script tiers (retiré) |

Les autres indicateurs (TVD Pro, Vector Zone Pro, Argos OB · BB · FVG · iFVG, Patterns, classiques comme RSI ou MACD)
sont d'Argos ou des formules publiques standard. Les noms d'autres auteurs n'apparaissent pas dans l'interface.
