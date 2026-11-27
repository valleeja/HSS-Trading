> Specification of the Argos module interface v2. In this kit, the conformance vectors are
> `argos-module-sdk/tests/vectors/abi-v2.json` and the example is `exemple-moyenne/`.

# Module interface v2: input data, outputs and settings (step 15)

Answer to the request of Argos HSS Trading (`docs/requests/2026-11-06-trading-kit-v2.md`). The
rights, the manifest and the checks of Argos stay the reference. This page is precise enough
for a second host (the browser Web Worker of Argos HSS Trading) to apply it identically; the
conformance vectors of the kit check both.

## What changes from the request, and why

| Request | v2 | Why |
| --- | --- | --- |
| postcard encoding | a small fixed binary layout, written by hand in the SDK | postcard needs serde's derive macros; procedural macros are refused in modules, and a hand-written layout has no dependency at all |
| prices as `f64` | integers at a fixed scale (`price_scale` decimals, `volume_scale` decimals) | no `f64` for money (Argos rule); exact and identical on every host; a module may still compute in floating point inside |
| `NaN` for an empty value | `i64::MIN` | integers have no NaN |
| 16 MB input and output | 4 MiB each | the sandbox has 16 MiB of memory in total |
| settings outside the approval | settings in the signed policy, shown on the rights sheet as "not a right" | they are part of the module's interface; changing them is a new version |
| numbers as real numbers | integers with a `scale` (decimals) per setting | no floating point in the interface |

## ABI versions

- **v1**: export `run() -> i32` (unchanged).
- **v2**: exports `argos_abi() -> i32` returning `2`, `alloc(len: i32) -> i32`, `free(ptr: i32, len: i32)`
  and `run_data(ptr: i32, len: i32) -> i64`, plus `memory`. A v2 module must not export `run`; a v1
  module must not export `run_data`, `alloc`, `free` or `argos_abi`. Mixtures are refused by the
  build tool and by the host.
- The manifest says `"abi": 1` (default) or `"abi": 2`, and must match the exports.

## Host side of one v2 call

1. Check `argos_abi()` returns 2 (else refused).
2. Encode the input (below); refuse if over 4 MiB.
3. `ptr = alloc(len)`; refuse if `ptr` is negative or `ptr + len` exceeds the module's memory.
4. Copy the input to `ptr`; call `run_data(ptr, len)`.
5. A negative result is an error status of the module. Otherwise the high 32 bits are the output
   pointer and the low 32 bits its length; refuse a length over 4 MiB or bytes outside the memory.
6. Decode and validate the output (below). The usual fuel, memory and time limits apply to the
   whole call; `free` may be a no-op (a fresh instance is used for every call).

## Encoding (little-endian, no padding)

Strings are UTF-8: `u16` length, then the bytes. Integers are fixed width. `EMPTY = i64::MIN`.

**Input**

| Field | Type | Limits |
| --- | --- | --- |
| abi | u32 | = 2 |
| timeframe | u32 | seconds, > 0 |
| price_scale | u8 | 0..=12 |
| volume_scale | u8 | 0..=12 |
| symbol | string | 1..=32 bytes |
| bar count | u32 | 0..=50 000 |
| bars | count × (t: i64 Unix seconds, o, h, l, c, v: i64) | 48 bytes each, `t` strictly increasing |
| setting count | u16 | 0..=16 |
| settings | count × i64 | in the order of the manifest |

**Output**

| Field | Type | Limits |
| --- | --- | --- |
| abi | u32 | = 2 |
| line count | u16 | 0..=16 |
| lines | count × (name: string 1..=64, scale: u8 0..=12, value count: u32 = bar count, values: i64) | `EMPTY` = no value |
| label count | u16 | 0..=1000 |
| labels | count × (bar: u32 < bar count, text: string 1..=256) | |
| card count | u16 | 0..=32 |
| cards | count × (name: string 1..=64, value: i64, scale: u8 0..=12) | |

Anything else (wrong version, a length past the end, trailing bytes, invalid UTF-8, a limit
exceeded) is refused, never partly used.

## Settings (`settings` in `argos-module.json`, part of the signed policy)

```json
{ "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500, "scale": 0 }
{ "id": "show", "name": "Afficher", "type": "bool", "default": true }
{ "id": "color", "name": "Couleur de la ligne", "type": "color", "default": "#2962ff" }
{ "id": "mode", "name": "Mode", "type": "choice", "choices": ["a", "b"], "default": "a" }
```

At most 16; `id` of 1 to 32 letters, digits, `-` or `_`, unique; `name` 1 to 64 bytes; `number`
has integer `min <= default <= max` at the given `scale` (decimals, 0 to 6); `choice` has 1 to 32
choices and a default among them. The value given to the module is: the number at its scale, 0 or
1, `0xRRGGBB`, or the index of the choice. The host checks every chosen value against the manifest
before the call. Settings are not rights: the rights sheet lists them as such.

## Module side (`argos-module-sdk`)

```rust
#![forbid(unsafe_code)]
use argos_module_sdk::{Input, Line, Output, EMPTY};
fn run(input: Input) -> Output { /* compute, integers at the input's scales */ }
argos_module_sdk::entry_data!(run);
```

`entry_data!` exports the four v2 functions; the module keeps `forbid(unsafe_code)`. The kit's
`alloc` leaks a zeroed buffer (a fresh instance serves every call). `run_data` statuses: `-1` bad
input, `-2` output over 4 MiB, `-3` no input memory (outside the sandbox). Host functions
(`read_file`, ...) keep working in v2 for the rights the module was granted.

## Host side (`argos-sandbox`)

`Sandbox::run_data(approval, wasm, &input, limits, services) -> Result<Output, SandboxError>`:
the approval must say `abi: 2`; the settings are checked against the approved policy
(`ServicePolicy::check_values`), the input is encoded and checked, the time budget is the policy's,
fuel and memory come from `limits`. Errors: `WrongAbi` (interface mismatch, missing or mistyped
export, version not 2, both interfaces exported), `BadInput`, `BadOutput` (status, size, address
outside memory, invalid output), plus the v1 errors (digest, fuel, time, memory, traps).
`Sandbox::run` refuses a v2 approval or a module that exports v2 functions.

The build tool (`argos-modules`) checks the same thing statically: the manifest's `abi` decides
the exports required, with their exact types, and the other interface's exports are refused.

## Conformance vectors

`crates/argos-module-sdk/tests/vectors/abi-v2.json` (also in the kit) holds valid and invalid
inputs and outputs, with the expected error, and one run of the `moyenne` example. They were
written from this page by an independent encoder; the SDK and the sandbox pass them. A second host
must accept the valid cases byte for byte and refuse every invalid case.

## Example

`modules/examples/moyenne` (`trading/moyenne`, no right, settings `length` 1 to 500 and `source`
close/open/high/low): simple moving average in integers, rounded half away from zero, `EMPTY`
until the window is full or while it holds an `EMPTY` value. 50 000 bars with a length of 500 run
in the sandbox (tested with 200 million fuel; the host sets fuel for its largest charts).
