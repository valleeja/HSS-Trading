# argos-module-sdk

License: MIT OR Apache-2.0 (see LICENSE-MIT and LICENSE-APACHE).

The Argos module kit: the only WASM boundary code, so modules stay
`#![forbid(unsafe_code)]`. Exports the entry as `run` and calls the four `argos`
host functions (`read_file`, `write_file`, `network_request`, `ai_request`).
