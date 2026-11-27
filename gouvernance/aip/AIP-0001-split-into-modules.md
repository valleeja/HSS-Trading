# AIP-0001: Split the app into modules

- **Author(s)**: Jee (`valleeja`), with Claude
- **Status**: Discussion
- **Created**: 2026-11-22
- **Area**: Rust (the layout of the whole app)

This text is the English summary of the plan Jee approved on 2026-11-22 (« ok plan v2, go filet »). The plan itself
(in French: one sheet per big module, every file and its box, every link and what becomes of it) is kept with the
project's documents, outside this repository: `2026-11-22-plan-separation-modules-v2.md` and its two annexes.

**How this AIP is decided.** `GOVERNANCE.md` (section 3) asks for a pull request holding only this text and for at
least 7 days of public discussion. Today the project is worked on locally, without any push to GitHub, and it has one
maintainer, the founder. So this text comes as a commit of its own in the local branch of step 0
(`separation-filet-1`), and Jee's written « ok fusion » for that branch is its acceptance. The status line above then
becomes `Acceptée` in the next branch of the split. (The statuses are those of `GOVERNANCE.md`: `Brouillon`,
`Discussion`, `Acceptée` or `Refusée`, `Livrée`.)

## Summary

Argos HSS Trading is rearranged the way its map is drawn: by category, then by big module, then by module (4
categories, 23 big modules, 83 modules). Each big module becomes a crate of its own with everything that touches it
inside, and it talks to the others only through one fixed door, so the compiler refuses an update of one module that
reaches into another one. Nothing changes on the screen, in what the app does or in the saved files: the work is
about 35 merges, the app is whole after each one, and the proof of each one is that everything is identical.

## Why

Jee's goal, in his words: « tout construit par module… quand on met à jour une section elle ne touche à rien
d'autre » (everything built module by module: updating one section touches nothing else), and for each update « le
module à toucher et les impacts que ça a sur le code ailleurs » (the module to touch, and what it impacts elsewhere).

What the code looks like today. It was read automatically, without compiling, on the branch `interface-2` at
`f1d89e7d` (2026-11-22): 281 files and 162 357 lines in the five big crates. These are reliable orders of magnitude,
not counts to the link.

- **A module is scattered.** The walls of the order book (« Murs ») live in five crates: `argos-render/src/murs.rs`,
  `argos-desktop/src/actions/murs.rs`, `argos-core/src/murs.rs`, `argos-data/src/murs.rs`, `argos-store/src/murs.rs`.
- **Everything hangs on one object.** 829 functions are written on the same `App` object. It carries 342 state
  fields: 145 serve one big module only, 107 are shared, 90 belong to the shell.
- **16 mixed files** (16 970 lines) hold pieces of several modules. In them, three giant functions are switchboards:
  `draw_frame` (1 298 lines) draws the window, `left_button` (1 241) sorts the clicks, `on_user_event` (489) sorts
  the messages. In all, the shell names the big modules 1 559 times.
- **The chart draws the others.** Its central file `argos-render/src/lib.rs` names the layers of the other big
  modules 71 times, and 15 of the 37 fields of `Chart` belong to others.
- **11 central lists name the modules one by one** (the modules of the column, the panels, the tiles of the Argos
  menu, who receives the keyboard…), plus the list of the 55 indicators and the list of the 52 kinds of messages.
- **721 direct links between big modules** have no declared reason. 934 more go from a big module up to the shell
  (« redraw the window » 245 times, « keep this file » 128 times), and 59 go from the centre up.
- **52 saved files**, 20 of them touched by another big module than their owner.
- **13 reference images for the whole app.** A screen without an image can change without any test seeing it.

If nothing is done: an update of one module can break another one without anybody seeing it; people working at the
same time step on each other in the mixed files; every change recompiles and retests the whole app, which is slow and
heavy on memory; and what comes next (a version per module with signed updates, computations in WASM modules, sharing
through the Marketplace and peer to peer) has nothing to stand on.

## Proposal

### The layout

Category, then big module, then module. The folders carry the names of the map, in French, as on the map.

```
rust/
├─ crates/                     the centre: the shell and what is shared
│  ├─ argos-core/              base types
│  ├─ argos-data/              shared data (exchanges, candle cache, live feeds)
│  ├─ argos-store/             shared storage
│  ├─ argos-kit/               NEW: the drawing kit (text, shapes, glass, colours, menus, name box, translation)
│  ├─ argos-porte/             NEW: the door and the registry
│  ├─ argos-render/            what is left of the shell's drawing (column, panels, frames)
│  ├─ argos-desktop/           the shell: window, switchboards, key vault
│  └─ argos-serveur/, argos-module-host/
├─ marches-et-suivi/           category 1: a-carnet, b-flux-ordres, c-marches, e-watchlist, f-alertes, g-actualites
├─ chart-et-analyse/           category 2: j-chart, h-argos-script, i-argos-ai, d-scanner
├─ trading-et-entrainement/    category 3: l-trading-demo, m-portefeuille-journal, n-simulateur, k-bots-strategies,
│                              o-propfirm, p-academie
├─ compte-et-communaute/       category 4: q-compte, r-communaute, s-support, t-marketplace, u-gagner, v-langues,
│                              w-interface
└─ modules/, vendor/, tools/, argos-tauri/      unchanged
```

- **One crate per big module** (`argos-mod-<name>`; `argos-propfirm` keeps its name): the hard boundary, held by the
  compiler.
- **One folder per module** inside that crate: a boundary watched by the boundary test.
- **Everything that touches a module is in its folder**: its computation, its data (what it fetches from the
  network), its state, its drawing, its clicks, its saved files, and its tests.
- A computation that must work without a screen (the local server, a WASM module) keeps a small crate of its own in
  the same folder, next to the screen crate. `argos-script`, `argos-lens`, `argos-ai`, `argos-scanner`,
  `argos-academy` and `argos-propfirm` are already like that: they enter the folder of their big module unchanged.

Inside a big module, always the same shape. Example with A Carnet:

```
rust/marches-et-suivi/a-carnet/
├─ Cargo.toml
├─ IMPACT.md                   its impact sheet, kept true by a test
├─ src/
│  ├─ lib.rs                   its sheet for the registry: name, icon, needs, plugs
│  ├─ carnet_ordres/           module 1
│  ├─ carte_thermique/         module 2
│  └─ murs/                    module 5
│     ├─ calcul.rs             today argos-core/src/murs.rs
│     ├─ donnees.rs            today argos-data/src/murs.rs
│     ├─ stockage.rs           today argos-store/src/murs.rs
│     ├─ dessin.rs             today argos-render/src/murs.rs
│     └─ actions.rs            today argos-desktop/src/actions/murs.rs
└─ tests/                      its tests and its reference images
```

### The door

To talk to the other modules and to the shell, a module knows only the door (`argos-porte`). It has three parts.

**Plugs: what a module can plug in.**

| Plug | What it is | Held by |
|---|---|---|
| column module | a box in the column of modules | the shell |
| panel | a panel on the right | the shell |
| tab of the Trading panel | a tab in the panel of another module (the simulator) | L Trading démo |
| line of the Exchanges panel | a line in the Exchanges panel (CoinMarketCap) | E Watchlist |
| page, and the registry of screens | a whole screen: home, academy, marketplace, portfolio, the month | the shell |
| chart layer | a drawing under or over the candles, or in a pane under the chart, with its place in the drawing order | J Chart |
| catalogue indicator | an entry of the ƒx menu, with its ⚙ settings | H Argos Script |
| menu lines and shortcuts | Argos menu, right click, the phone's sheet, the Ctrl + K palette, the toolbar | the shell |
| saved files | « this file, or this setting of a shared file, is mine; keep it; sync it or not » | the shell |
| data feed | « I need this live feed » | the shared data |
| candle source | giving candles to the chart (replay, DEX, market cap) | J Chart |
| symbol source | giving results to the symbol search | E Watchlist |
| home card | a card on the home page | W Interface |
| summary for the Argos AI circle | what the Argos AI circle may read of a module | I Argos AI |
| input field | « this field receives the keyboard » | the shell |

**Services: what a module can ask for.**

| Service | What it is | Given by |
|---|---|---|
| the window | redraw; the size of the window and of its areas (top bar, the phone's height, the place of the Argos AI circle); full screen | the shell |
| open a panel | open a panel, know whether it is open, keep the right-hand column | the shell |
| the Argos AI circle | ask a question, ask for an opinion, place the circle in a screen | I Argos AI |
| open a market | load a symbol, change the timeframe, reload the chart (the one being left is saved) | J Chart |
| read the chart | its symbol, candles, timeframe, time zone, visible area and image, without touching it | J Chart |
| ask for a text | the name box and the phone's keyboard; the answer comes back to the one who asked | the drawing kit |
| ask for a choice | a drop-down list; the choice comes back to the one who asked | the drawing kit |
| open a page | open a screen by its name, without knowing its module | the shell |
| the vault | store a key, know whether it is there, « sign this ». Never the key itself | the shell |
| demo account | balance, positions, prices; « demo order » | L Trading démo |
| the followed list | the followed symbols and their prices | E Watchlist |
| notify | banner, sound, 🔔; create an alert | F Alertes |
| market cap of a symbol | the market cap and the dominances | C Marchés |
| switch an indicator | switch an indicator on or off | H Argos Script |
| open a lesson | open a lesson of the academy | P Académie |
| news in progress | whether an announcement is under way (for the guardian) | G Actualités et calendrier |
| platform | download a file for the user, the clipboard, open a link | the shell |
| market sessions | the hours of the market sessions | G Actualités et calendrier |

Translating, writing a price, drawing glass or drawing a menu are not services: they are tools of the drawing kit,
which everybody uses directly.

**Events.** « trade closed », sent by L Trading démo and heard by the journal; « market opened », sent by J Chart
and heard by the PC ↔ phone sync. An event that carries base types only is defined in the door. An event that carries
the type of a module is defined by that module, and hearing it takes a declared need.

**What crosses the door**: base types and the door's own small types (a price, a symbol, a rectangle, a menu line).
If a service had to carry the type of a module, there are two ways out, chosen in the module's « door » lot and
written in its report: the type goes down into the base types, or the one who asks declares the need on its sheet.

**What a module may use directly**: the door, the drawing kit, the shared data, the base types, and the big modules
written in its declared needs. Nothing else.

**What a module never sees**: the state or the code of another module, unless it declared it as a need; a key; the
network outside the allow-list (`rust/crates/argos-data/src/net.rs`).

**The state.** The 145 fields that serve one big module go into that module's own state. Each of the 107 shared
fields gets ONE owner, and the others read it through a service. The 90 fields of the shell stay with the shell.

**The assembly is fixed.** The app stays one program: the shell keeps one state per big module, and tables filled at
start-up. Nothing is loaded on the fly, so nothing gets slower. The switchboards keep their written order: each
branch becomes a one-line call to the module, and each layer keeps its place in the drawing order. The numbers kept
in the saved files (column modules, indicators) do not change.

**When the door freezes.** It can still move during the first column: that is what the pilot is for. After that it is
frozen, and changing it takes an AIP.

### The impact sheet

Each big module has an `IMPACT.md` in its folder. It says, in seven lines:

```
IMPACT  a-carnet (argos-mod-carnet)
needs            (none)
used by          (nobody directly)
plugs into       dock module "Carnet d'ordres" · panel "Carnet" · 4 chart layers · indicators BookHeat, ArgosMurs
asks             notify · window layout · data feeds (order book, trades)
owns files       murs.redb
events           (none)
after an update  run its own tests and its reference screens ; nothing else to check
```

- **It cannot lie**: the boundary test reads the code and fails if a module uses something that is not on its sheet.
- **Before an update**, the sheet gives the exact list of what to check elsewhere. For 14 big modules out of 23 the
  answer will be « nothing else »: no other big module depends on them directly.
- **For each of today's links, two outcomes only**: it goes through the door, or it is written on the sheet as a
  declared need. Never a hidden link.

The declared needs the split starts from: H Argos Script needs J Chart; I Argos AI needs J and H; D Scanner needs B
and H; L Trading démo needs J; M Portefeuille et journal needs J and L; N Simulateur needs J, H, I, L and O; K Bots
et stratégies needs J, H, I, L and N; R Communauté needs M and Q; T Marketplace needs H; U Gagner needs Q. The 13
others need nobody.

### The rules

1. **Two phases, never mixed in one lot.** First decouple in place (the code does not change file), then move (the
   code does not change). A lot is one change checked alone, with the tests and the screen references.
2. **Five lots per big module**, grouped in 1 to 3 merges according to its size: *tidy* (put its pieces together in
   today's crate, take back those kept in the shell or in a mixed file, give back to the others what is theirs: pure
   moves); *state* (group its fields in a state of its own: pure renaming); *door* (its functions leave the `App`
   object and receive their state and the door; its direct links to the other modules and its calls to the shell
   become calls to the door: the only risky lot); *move* (take it out into its folder: pure move); *clean* (write
   `IMPACT.md`, tighten the boundary test).
3. **Facades.** For drawing and actions the old path stays for a while and forwards to the new one, so that work in
   progress in other branches still compiles; it leaves when nobody uses it any more, at the latest in the final
   clean-up. For computation, data and storage (the lower layers) a facade is impossible: every path is updated in
   the same lot, and the compiler finds them all.
4. **A link is cut only if it is covered** by a test or by a screen reference. Otherwise the test is written first
   and seen passing on the old code.
5. **Nothing else changes**: no new feature, no screen retouched, no new dependency, the same names and formats for
   the saved files.
6. **A test that needs two big modules** without a declared need between them goes with the tests of the whole app
   (the shell). A test stays with its module when it needs only that module and the door.
7. **If a « general » piece turns out to be tied to its module**, it stays there and its link goes through the door.
   If a link is better handled another way than planned, the report of the lot says so, and the counts of the
   boundary test show the gap.
8. **One branch per merge.** `make check` under the PC's compilation lock, its result written in the report. Never
   `--no-verify`. No merge without Jee's written « ok fusion <branch> ». Everything local, no push.
9. **One big module frozen at a time**: during its 1 to 3 merges nobody else has an open branch on it.
10. **What is not built yet is born as a module**: once the foundations are merged, a new module is created directly
    in its folder, with its sheet.
11. **After 3 failed attempts** on a lot, the work stops and Jee is told what blocks, with the output.
12. **Sensitive areas** (`GOVERNANCE.md`, `.github/CODEOWNERS`): the vault does not move; `argos-propfirm` moves as
    one block; a path change in `CODEOWNERS` waits for Jee's written agreement.

### What stays in the centre

| Part | Today | What it becomes |
|---|---|---|
| The shell | 30 files, 19 602 lines: the window, the start-up, the 🧩 Modules page, the key vault, the tests that click through the whole app | stays; it assembles the modules |
| Mixed files | 16 files, 16 970 lines: the switchboards and the frames, with pieces of modules inside | the pieces go home; the switchboards stay, shorter |
| The drawing kit | 17 files, 10 516 lines: text, shapes, glass, colours, icons, menus | moves out to `crates/argos-kit` in step 1, with the general pieces |
| Shared data | 28 files, 9 222 lines: exchanges, candle cache, gaps, live feeds, database | stays; names no module any more |
| Base types | 10 files, 1 911 lines: candle, interval, money, folders | stays; receives the shared types |

### The saved files

After the split, **one saved file has one owner**, and the others go through it. Two shared files are the exception:
in `watchlist.json` and `interface-2.json` each *setting* has one owner, which reads and writes it through the
« saved files » plug. The PC ↔ phone sync reads the registry, where each module says which files it wants synced.

### What the split does not do

- No new feature, no screen changed.
- The app stays ONE program. Updating a module still means a new version of the app. Only the packs (languages,
  lessons, indicators in Argos Script, themes) and the WASM modules of the kit install on their own, as today.
- Between two big modules the boundary is held by the compiler; between two modules of the same big module it is
  watched by a test.
- It fixes none of the defects that already exist: they move as they are.

## Compatibility

- **Saved files: same names, same formats.** Settings, lists and the journal are neither converted nor moved, and
  the numbers written in them (column modules, indicators) stay. A test reads and rewrites an example of every saved
  file at each merge and fails on the first difference.
- **No MAJOR version**: nothing breaks for a user, and nothing has to be migrated.
- The kit of the WASM modules (`rust/vendor/kit-module-argos-1.0.2`), its manifest and `rust/modules/` do not change
  during the split.
- Work in progress in other branches: see the facades (rule 3) and the freeze of one big module at a time (rule 9).

## Security

- **Nothing new is kept, no new network address is called, no dependency is added.** The network allow-list
  (`rust/crates/argos-data/src/net.rs`) and its test do not change.
- **The vault does not move** (`rust/crates/argos-desktop/src/security/`). Through the door a module can store a
  key, know whether it is there and ask « sign this »: it never receives the key itself.
- **No real order.** The only order service of the door is « demo order ». No path to a real order exists, and none
  is created.
- **Sensitive areas** change only with Jee's written agreement: `argos-propfirm` is moved as one block, and any path
  change in `.github/CODEOWNERS` or under `gouvernance/` waits for it.
- **The safety net never reads real data**: its examples of saved files are made up, the journal's database is
  opened with a made-up key (never the system keyring), and the copies of the test program that draw the screens have
  no network, no program to start, no keyring and no personal folder.

The risks, and what answers them:

| Risk | Answer |
|---|---|
| A screen without a test changes and nobody sees it. This is the one real risk. | Step 0: a reference for every screen, PC and phone, compared exactly; a screen without a reference fails a test. |
| A saved file can no longer be read. | Same names, same formats; every saved file has a « read and rewritten the same » test before the first move. |
| The work of another branch collides with a move. | One big module frozen at a time; facades where possible; pure moves rebase well. |
| The app gets slower or bigger. | Fixed assembly, nothing loaded on the fly; size and drawing time measured at each merge, stop beyond 2 %. |
| The order of the clicks or of the layers changes. | The switchboards and the layers keep their written order; the tests that click through the whole app; the screen references. |
| The door is frozen too early, or keeps changing. | It may move during the first column (the pilot), then it is frozen; after that, an AIP. |
| Two crates that depend on each other: the compiler refuses. | The order of the map respects the declared needs (checked); no facade in the lower layers; the centre names no module before that module leaves. |
| The reading of the code missed a link. | The compiler is the real judge: once a module is in its crate it can use nothing that is not declared. A missed link shows as a compilation error, not as a defect on the screen. |
| The work stops half-way. | The app is whole after each merge: the work can stop between two merges, and resume. |
| A defect gets through anyway. | That merge alone is undone. Nothing else is lost. |

## Alternatives rejected

- **A plan that does not follow the map.** A first plan was refused by Jee on 2026-11-22 because it did not follow the
  map of the app. The layout and the order of the work are the map's: same 4 columns, same 23 big modules, same order.
- **Waiting until the whole app is finished.** The 18 modules that are not built yet would be written in today's
  layout and moved afterwards. Instead the split starts once the first column (« Marchés et suivi ») is finished, and
  what is not built yet is born as a module (rule 10).
- **Everything in one go.** Nobody could tell which change broke what. Instead: about 35 merges, the app whole after
  each one, decoupling and moving never in the same lot. Redoing ONE step can happen; starting everything over cannot.
- **Folders only, in today's crates.** A folder is a convention: nothing stops a file from reaching into another
  one. A crate can use only the crates it declares, which is what makes a boundary hard.
- **Loading the modules on the fly.** Slower, and not needed for the goal. The assembly stays fixed in one program;
  computations in WASM modules are another piece of work, after the split.
- **Reference images with a tolerance** (today: 13 images, tolerance 12 out of 255). A tolerance lets a small change
  through. The screen references of step 0 are compared exactly, and each scene is drawn until two images in a row
  are the same, which proves it is stable.
- **A script outside the tests to count the links.** The repository accepts Rust only (no `.py` file) and no new
  dependency: the boundary test is a Rust test of the app, run by `make check` like the others.

## Delivery plan

No pull request and no release channel yet: each merge is a local branch merged into `interface-2` with Jee's written
« ok fusion <branch> ». The split itself (steps 1 to 6) starts after the column « Marchés et suivi » is finished, on
Jee's go. The measurements are taken again at that moment.

| Step | What | Merges |
|---|---|---|
| 0 | The safety net: tests only, and this text | 1 |
| 1 | The foundations: the drawing kit `argos-kit` and the shared types, then the door `argos-porte` and the registry | 2 |
| 2 | Marchés et suivi: A Carnet (2), B Flux d'ordres (1), C Marchés (1), E Watchlist (2), F Alertes (1), G Actualités et calendrier (1) | 8 |
| 3 | Chart et analyse: J Chart (3), H Argos Script (3), I Argos AI (1), D Scanner (1) | 8 |
| 4 | Trading et entraînement: L Trading démo (2), M Portefeuille et journal (2), N Simulateur (1), K Bots et stratégies (born as a module), O PropFirm (1), P Académie (1) | 7 |
| 5 | Compte et communauté: Q Compte (1), R Communauté (1), S Support (1), T Marketplace (1), U Gagner (born as a module), V Langues (1), W Interface (3) | 8 |
| 6 | The final clean-up: the last facades leave, the boundary test becomes strict (no undeclared link), the 23 final `IMPACT.md`, `ARCHITECTURE.md` rewritten, `Makefile` and `CODEOWNERS` up to date | 1 |

The pilot is module 1 (the order book) of A Carnet: small, but it has everything (a column module, a panel, a live
feed). After the pilot, the time for the rest is measured, not guessed, and the number of merges (35 is an estimate)
is adjusted. The order of the map respects the declared needs: no big module moves before one it needs. W Interface
comes last: it is the one that names all the others.

**Step 0, the safety net** (delivered with this text; it does not touch the code of the app):

1. **A reference for every screen**, on the PC (1440 × 900 with the mouse) and on the phone (502 × 972 by the
   finger, the size Jee's own phone gives the app), and, outside that universe, on the tablet by the finger, lying
   down (1092 × 734) and upright (819 × 1008): the families of screens a narrower window lays out another way. The
   scenes come from the code's own lists, so a new screen without a scene fails a test. Every kind of message the
   app receives (candles, an order book, the answer of a source or its refusal, a setting changed on another device)
   also arrives, made up, in a scene: the screens are compared with their data too, and a new kind of message
   without a scene fails the same test. The windows are pressed control by control, and each screen that follows
   is kept too. Each reference is a fingerprint kept as text, and the comparison is exact. The 13 reference images
   that existed before stay.
2. **The saved files**: for each one a made-up example, read and rewritten by the app's own functions, which must
   give the same file; the three databases are reopened and give back what they were given; a new file name in the
   code fails a test until it is classified. The four files of the vault keep their own tests.
3. **The boundary test**: every source file has its box on the map (`safety_net/map.txt`); the links from one box to
   another that the plan does not let through are counted for each pair of boxes, and the counts are kept in
   `tests/references/safety-net-boundaries.txt`. It works like a ratchet: a count can only go down. A new link, a
   file without a box or a new dependency between crates fails the test.
4. **The list of the links no test runs through**, by big module: where to write tests before cutting.
5. **The starting photo**: the number of tests, the size of the web app and the time to draw an image, written in the
   report of the merge and compared at each merge that follows.

A change that is wanted (a screen, the type of a saved file, a link that disappears) is written with
`ARGOS_REFERENCES=maj`; the diff of the reference files is then read and committed with the change. That command
never writes a boundary count that goes up: such a count is written by hand, with the reason in the report of the lot.

**The proofs at each merge:**

1. `make check` green, with at least as many tests as before; the number is written.
2. Every screen reference identical: PC, phone and tablet.
3. Every saved file read and rewritten the same.
4. Boundaries: no count goes up; those of the lot go down by the number announced.
5. Size of the web app and drawing time measured before and after; beyond 2 %, stop and look.
6. A trial version for Jee, checked to be up to date with `interface-2`, with a short « what to try » list.
7. Jee's written « ok fusion <branch> ».
8. Afterwards: the branch and its working folder removed, the build folder emptied.

**After the split** (other pieces of work, not in this AIP): a version per module and updates signed with the
maintainers' keys; computations in WASM modules; the kit of the WASM modules.
