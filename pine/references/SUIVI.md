# TradingView community scripts kept as Argos Script references: tracking

Point 3 of the safeguards (« indicateurs = TradingView », branch tradingview-1). Jee pasted these scripts on
25 Nov 2026 so that Argos Script runs them as they are and draws what TradingView draws. Each one runs in
`rust/crates/argos-script/tests/pine_references.rs`; the Pine features they needed are kept by
`rust/crates/argos-script/src/lang/tests_references.rs` (bugs B-030 to B-040 of `rust/tools/porte/registre.txt`).

They are third-party code under their own licence, kept apart from Argos' code: every file opens with its licence
and its author, and the test refuses a file without them, or any GPL code.

| File | Indicator | Author | Licence | Origin |
|---|---|---|---|---|
| luxalgo-smart-money-concepts-v5.pine | Smart Money Concepts [LuxAlgo] | LuxAlgo | CC BY-NC-SA 4.0 | pasted by Jee (v5), licence header put back |
| luxalgo-smart-money-concepts-v6.pine | the same, Pine v6 | LuxAlgo; v6 by Argos | CC BY-NC-SA 4.0 (ShareAlike) | Argos' SMC reference; its changes are listed in its header |
| luxalgo-liquidity-swings-v5.pine | Liquidity Swings [LuxAlgo] | LuxAlgo | CC BY-NC-SA 4.0 | pasted by Jee (v5), licence header put back |
| luxalgo-liquidity-swings-v6.pine | the same, Pine v6 | LuxAlgo; v6 by Argos | CC BY-NC-SA 4.0 (ShareAlike) | its changes are listed in its header |
| viprasol-fib-ote-fvg.pine | Fib OTE + FVG Confluence [Viprasol] | Viprasol | MPL 2.0 | pasted by Jee, as it is |
| zeiierman-smart-swing-vwap.pine | Smart Swing VWAP (Zeiierman) | Zeiierman | CC BY-NC-SA 4.0 | pasted by Jee; its licence line was cut after « Creative Commons » and is completed |
| zeiierman-smart-swing-vwap-mtf.pine | Smart Swing VWAP + MTF VWAP | Zeiierman, with an MTF VWAP module added | CC BY-NC-SA 4.0 (stated in the script) | pasted by Jee, licence lines put back |

The v6 versions draw exactly what their v5 code draws on the test bars (checked by the test).

## Checked against Jee's TradingView captures (25 Nov 2026)

Liquidity Swings: the same zones and volumes. Smart Money Concepts: the same structure (BOS, CHoCH, strong / weak
high and low, order blocks). Smart Swing VWAP: the same anchors and curves. Fib OTE: the same zone and levels. The
differences left came from the time of the capture (a later bar on TradingView) or a shorter history in Argos.

## Not kept in the repository

Pasted by Jee the same day, they run as they are too, but they carry no licence or author: they stay out of the
repository (kept in the project files, `indicateurs/`), their features are kept by the tests of `tests_references.rs`.

- AI Moving Average (Expo): matrices (B-036).
- SMC Multi-TF Structure Dashboard: `request.security` on a lower timeframe (B-037), `timenow` and `time(timeframe)`
  (B-034).
- A Pine v3 VWMA / SMA breakout: bare styles `style=circles` (B-031).
- Moving average ribbon: TradingView's built-in, already in `pine/tradingview/ma-ribbon.pine`.
