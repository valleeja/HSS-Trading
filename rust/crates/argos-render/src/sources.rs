//! { } Code source des indicateurs intégrés (bouton « { } » de la légende, comme TradingView) : les scripts Pine de
//! Jee dont l'indicateur d'Argos est le portage, inclus à la compilation depuis `pine/`, ou le script Argos Script qui
//! le calcule. B1 (tableaux-1, Jee 22 Nov): the indicators that stay in Rust show their real Rust source, read only
//! ([`source_rust`]): the files themselves, included at compile time (one source of truth, nothing copied by hand).

use crate::indicators::Ind;

/// Le texte source d'un indicateur intégré, s'il existe : le Pine de Jee, sinon le script Argos Script qui le calcule
/// vraiment (Spring Tension, les moyennes de TradingView…).
pub fn source_de(i: Ind) -> Option<&'static str> {
    Some(match i {
        Ind::VectorZone => include_str!("../../../../pine/argos-vector-zone-pro.pine"),
        Ind::Tbo => include_str!("../../../../pine/CCQc_TBO_Pro_Div.pine"),
        Ind::Cipher => include_str!("../../../../pine/CCQc_Cipher_Pro.pine"),
        Ind::AiTrend => include_str!("../../../../pine/argos-ai-trend-pro.pine"),
        Ind::Tvd => include_str!("../../../../pine/CCQc_TVD_Pro.pine"),
        _ => return argos_script::catalogue::source(crate::catalogue::fiche(i)),
    })
}

// (each Rust file once: several indicators share one)
const CLASSIC: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/classic.rs",
    include_str!("../../argos-script/src/indicators/classic.rs"),
);
const TEXTBOOK: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/textbook.rs",
    include_str!("../../argos-script/src/indicators/textbook.rs"),
);
const FLOW: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/flow.rs",
    include_str!("../../argos-script/src/indicators/flow.rs"),
);
const SMC: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/smc.rs",
    include_str!("../../argos-script/src/indicators/smc.rs"),
);
const SMC_PRO: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/smc_pro.rs",
    include_str!("../../argos-script/src/indicators/smc_pro.rs"),
);
const PIVOT_VWAP: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/pivot_vwap.rs",
    include_str!("../../argos-script/src/indicators/pivot_vwap.rs"),
);
const ORDER_BLOCKS: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/order_blocks.rs",
    include_str!("../../argos-script/src/indicators/order_blocks.rs"),
);
const ANCHORED_VWAP: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/anchored_vwap.rs",
    include_str!("../../argos-script/src/indicators/anchored_vwap.rs"),
);
const PATTERNS: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/patterns.rs",
    include_str!("../../argos-script/src/indicators/patterns.rs"),
);
const MTF_TABLE: (&str, &str) = (
    "rust/crates/argos-script/src/indicators/mtf_table.rs",
    include_str!("../../argos-script/src/indicators/mtf_table.rs"),
);
const LENS: (&str, &str) = ("rust/crates/argos-render/src/indicators/lens.rs", include_str!("indicators/lens.rs"));
const LENS_ANALYSE: (&str, &str) = ("rust/crates/argos-lens/src/analyse.rs", include_str!("../../argos-lens/src/analyse.rs"));
const BOOKHEAT: (&str, &str) = ("rust/crates/argos-render/src/bookheat.rs", include_str!("bookheat.rs"));
const LIQVIEW: (&str, &str) = ("rust/crates/argos-render/src/liqview.rs", include_str!("liqview.rs"));
const MURS: (&str, &str) = ("rust/crates/argos-data/src/murs.rs", include_str!("../../argos-data/src/murs.rs"));

/// { } The Rust file(s) that compute an indicator kept in Rust (path from the repository root, text); `None`: it has
/// a Pine or Argos Script source ([`source_de`]).
pub fn fichiers_rust(i: Ind) -> Option<&'static [(&'static str, &'static str)]> {
    use Ind::*;
    if source_de(i).is_some() {
        return None;
    }
    Some(match i {
        Rsi | Macd | Bollinger | EmaCross | Stochastic | Atr | Vwap | RelVolume | EmaMulti | Heatmap | Profile | Bands | BandsOsc => &[CLASSIC],
        StochRsi | Ichimoku | Keltner | Donchian | Psar | Dmi | Cci | WilliamsR | Obv | Supertrend | Aroon | Mfi => &[TEXTBOOK],
        Cvd | Footprint | Bubbles | Tape => &[FLOW],
        Smc => &[SMC],
        SmcPro => &[SMC_PRO],
        SwingVwap | SmartSwingVwap => &[PIVOT_VWAP],
        PriceAction => &[ORDER_BLOCKS],
        AnchoredVwap => &[ANCHORED_VWAP],
        Patterns => &[PATTERNS],
        MtfTable => &[MTF_TABLE],
        Lens => &[LENS, LENS_ANALYSE],
        BookHeat => &[BOOKHEAT],
        LiqMap => &[LIQVIEW],
        ArgosMurs => &[MURS],
        _ => return None,
    })
}

/// { } The Rust source of an indicator kept in Rust, as the read-only code view shows it: a first comment line
/// naming its file(s) and « Rust, lecture seule », then each file whole.
pub fn source_rust(i: Ind) -> Option<String> {
    let f = fichiers_rust(i)?;
    let noms: Vec<&str> = f.iter().map(|(p, _)| *p).collect();
    // (tableaux-1 tour 2, F8: « Rust, lecture seule » first, visible on the cell's narrow view)
    let mut s = format!("// Rust, lecture seule · {} · {}\n", i.name(), noms.join(" + "));
    for (p, t) in f {
        if f.len() > 1 {
            s.push_str(&format!("\n// ───── {p} ─────\n\n"));
        } else {
            s.push('\n');
        }
        s.push_str(t);
    }
    Some(s)
}

/// { } The indicator has a code to show (Pine, Argos Script or Rust): its legend pill has { }.
pub fn a_un_code(i: Ind) -> bool {
    source_de(i).is_some() || fichiers_rust(i).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// { } Les 5 indicateurs avec leur Pine ; les intégrés en Argos Script, leur script ; les autres (en Rust), leur
    /// vrai code Rust en lecture seule (B1).
    #[test]
    fn sources_des_indicateurs() {
        for i in [Ind::VectorZone, Ind::Tbo, Ind::Cipher, Ind::AiTrend, Ind::Tvd] {
            let s = source_de(i).unwrap_or_else(|| panic!("{i:?} : pas de source"));
            assert!(s.contains("//@version") || s.contains("indicator("), "{i:?} : un script Pine");
            assert!(fichiers_rust(i).is_none(), "{i:?} : Pine, unchanged");
        }
        assert!(source_de(Ind::Lens).is_none() && source_de(Ind::Rsi).is_none());
        // the built-ins computed by their Argos Script: their own script, never empty
        for i in [
            Ind::SpringTension,
            Ind::Sma,
            Ind::Wma,
            Ind::Hma,
            Ind::Vwma,
            Ind::Smma,
            Ind::MaRibbon,
            Ind::LuxAlgoSmc,
        ] {
            let s = source_de(i).unwrap_or_else(|| panic!("{i:?} : pas de source"));
            assert!(!s.trim().is_empty() && s.contains("input"), "{i:?} : son script");
        }
        // every indicator with a source has it; every { } opens a non-empty text
        for i in Ind::ALL {
            let script = argos_script::catalogue::source(crate::catalogue::fiche(i)).is_some();
            assert!(!script || source_de(i).is_some(), "{i:?} : script sans {{ }}");
            assert!(source_de(i).is_none_or(|s| !s.trim().is_empty()), "{i:?} : {{ }} vide");
        }
    }

    /// B3: every catalogue entry has a non-empty source text (Pine, Argos Script or Rust); the Rust one names its
    /// file(s) and « Rust, lecture seule » on its first line and is the file itself (`fn`).
    #[test]
    fn chaque_indicateur_a_son_code() {
        let mut rust = 0;
        for i in Ind::ALL {
            assert!(a_un_code(i), "{i:?} : pas de {{ }}");
            if let Some(s) = source_rust(i) {
                rust += 1;
                let l1 = s.lines().next().unwrap_or("");
                assert!(
                    l1.starts_with("// ") && l1.contains("Rust, lecture seule") && l1.contains("rust/crates/"),
                    "{i:?} : {l1}"
                );
                assert!(s.contains("fn ") && s.len() > 500, "{i:?} : the Rust file itself");
            }
        }
        assert!(rust >= 30, "the indicators kept in Rust: {rust}");
        let m = source_rust(Ind::ArgosMurs).unwrap();
        assert!(m.lines().next().unwrap().contains("argos-data/src/murs.rs"));
    }
}
