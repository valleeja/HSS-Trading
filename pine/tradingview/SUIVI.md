# TradingView codes used as Argos Script built-ins: tracking

Each file of this folder is run as it is by the Argos Script interpreter (`argos_script::integres`).

| File | Indicator (shorttitle) | Origin | Licence | Defaults | Lot |
|---|---|---|---|---|---|
| sma.pine | Simple moving average (SMA) | pasted by Jee, 18 Nov 2026 | TradingView built-in, header absent | Length 9, Source close, Offset 0 (hidden from the legend); Smoothing: Type None, Length 14, BB StdDev 2 | moyennes |
| wma.pine | Moving Average Weighted (WMA) | written by Argos from Jee's captures (18 Nov 2026), exact code not received | TradingView built-in (behaviour), header absent | Length 9, Source close, Offset 0 (hidden from the legend), #2962FF | moyennes |
| hma.pine | Hull moving average (HMA) | pasted by Jee, 18 Nov 2026 | TradingView built-in, header absent | Length 9, Source close | moyennes |
| vwma.pine | Volume-weighted moving average (VWMA) | pasted by Jee, 18 Nov 2026; captures | TradingView built-in, header absent | Length 20, Source close, Offset 0, #2962FF | moyennes |
| smma.pine | Smoothed moving average (Smoothed MA) | pasted by Jee, 18 Nov 2026 | TradingView built-in, header absent | Length 7, Source close, #673AB7 | moyennes |
| ma-ribbon.pine | Moving average ribbon (MA ribbon) | pasted by Jee, 18 Nov 2026; captures | TradingView built-in, header absent | MA #1..#4: SMA, close, 20 / 50 / 100 / 200; #f6c309, #fb9800, #fb6500, #f60c0c | moyennes |
| bollinger.pine | Bollinger Bands (BB) | written by Argos from TradingView's built-in, 19 Nov 2026 (replaces Argos' native « Bandes de Bollinger », same catalogue id « Bollinger ») | TradingView built-in, header absent | Length 20, Basis MA Type SMA, Source close, StdDev 2, Offset 0 (hidden from the legend); Basis #2962FF, Upper #F23645, Lower #089981, fill rgb(33, 150, 243, 95) | bollinger-script-1 |

## Known gaps with TradingView (for later)

- The ⚙ has no « Style » tab (each plot's colour, width, type) nor « Visibilité » tab, and no « CALCUL » section
  (« Plage temporelle », « Attendre la fermeture de la plage temporelle »): the scripts' `timeframe = ""` runs on the
  chart's unit only.
- `display = display.data_window` / `display.none` on an input hides it from the legend in TradingView; Argos'
  legend writes all its values.
- The plots' current values are not yet written after the legend (« 5,00 ∅ ∅ … »).
- Source list: the outputs of the other indicators of the chart are offered after the 9 prices (as TradingView);
  a script's `input.source` can read only the 9 prices for now.
- HMA: `length/2` with `length` an `input.int` — to check against a TradingView value on the same bars.

## Settings keys (for Argos AI)

The keys are the `input()` titles. An untitled input on a line (`inline`) gets the key « <first title of the line> ·
<rank> »: the ribbon's « MA #1 · 2 » (type), « MA #1 · 3 » (source), « MA #1 · 4 » (length), and the same for #2..#4.
A choice is the index in the script's `options` (TradingView's order); a Source is the index in `inputs::SOURCES`
(0 close … 6 ohlc4, 7 volume, 8 hlcc4).
A title met a second time gets the key « <title>@<rank> » (rank among the script's inputs, from 0), shown with its
title: the SMA's smoothing length is « Length@4 », shown « Length », and keeps its own value.
