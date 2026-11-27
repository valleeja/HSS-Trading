# Argos's own scripts used as Argos Script built-ins: tracking

Each file of this folder is Argos's own code, run as it is by the Argos Script interpreter (`argos_script::integres`).
An idea taken from another author is named in the header; none of their code is used.

| File | Indicator (shorttitle) | Origin | Catalogue | Defaults | Lot |
|---|---|---|---|---|---|
| argos-bollinger-rsi.pine | Argos Bollinger + RSI | Jee, 19 Nov 2026; the idea of ChartArt's « RSI + Bollinger Bands Double Strategy » | « BollingerRsi », CLASSIQUES, ƒx Classiques | RSI 6, Bollinger 200, ×2, candle and background colours on | bollinger-rsi |
| argos-vague.pine | Argos Vague (Vague) | Jee, 20 Nov 2026 (v8 of `correctifs/2026-11-20-argos-vague-v8.pine`); the ideas of BigBeluga's « Nautilus PRO+ » public page | « ArgosVague », ARGOS, ƒx Oscillateurs | Moteur Classique, Style Auto (≤ 15 min Scalpeur 7/2/5, ≤ 4H Trader du jour 10/3/9, above Swing 14/4/12), adaptive thresholds over 200 bars × 1.5 σ, triangles #FFD600 | argos-vague-1 |

## Engine features used by Argos Vague (added in argos-vague-1)

- `force_overlay = true` on `plotshape`, `line.new`, `box.new`, `label.new`: drawn on the price pane.
- `argos.real_delta`: buys − sells of the bar from the real trades (complete bars only), `na` without them.
- `barstate.isconfirmed`: false on the last bar while it is still forming (the app passes it).
- Labels in shape styles (`label.style_circle`, `style_diamond`…) drawn as the shape with their text above.

## Test fixture (not a built-in)

- `argos-vague-tradingview.pine`: Jee's TradingView edition of Argos Vague (`correctifs/2026-11-20-argos-vague-tradingview-v4.pine`,
  342 lines, copied as it is). Used by `argos_script::lang` tests (editeur-pine-1) for `request.security_lower_tf`,
  with and without one-minute intrabars, and by the editor's « ce script ne trace rien » check (its first 79 lines).
- `argos-vague-tradingview-v5.pine`: the same with the gauge of the Argos edition (`correctifs/2026-11-22-argos-vague-tradingview-v5.pine`,
  357 lines): the four words start right of the ruler, a word within 16 of the pill's level is not drawn.
