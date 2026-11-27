# 🦀 Argos Scanner

Scanner **double structure** (Daily + 4H) pour les perps **Hyperliquid**, écrit en Rust.

Pour chaque coin :
- structure **Daily** et **4H** (pivots HH/HL ou LH/LL, BOS, CHoCH) ;
- RSI 4H, Stoch 4H, position par rapport à l'EMA 200 4H ;
- verdict : ✅ LONG / ✅ SHORT (les 2 timeframes alignés), 👀 À SURVEILLER (le daily est à moins de 2 % de s'aligner), ❌ pas aligné ;
- plan proposé si aligné : entrée sur le retest du niveau cassé, SL structurel (ramené à 2 ATR s'il est trop loin), cible = niveau daily qui donne au moins 2R.

## Installation (une seule fois)

```bash
sudo apt install build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Compiler

```bash
cd ~/claude-tradingview/argos-scanner
cargo build --release
```

## Utiliser

```bash
./target/release/argos-scanner                        # tes 17 coins Hyperliquid
./target/release/argos-scanner --coins BTC,ZEC,NEAR   # coins choisis
./target/release/argos-scanner --loop 15              # rescanne toutes les 15 min
./target/release/argos-scanner --pivot 2              # pivots plus sensibles (défaut : 3)
```

## Tests

```bash
cargo test
```

⚠️ Détection automatique : valide toujours sur ta chart avec Argos Market Structure. Pas un conseil financier.
