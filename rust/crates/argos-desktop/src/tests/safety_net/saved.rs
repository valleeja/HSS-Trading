//! 🕸 The saved files of the safety net: what the app keeps between two launches is read and written back the same.
//!
//! Moving code must not change a saved file: a setting, a list, a trade must be read after the move as it was written
//! before it. For each saved file:
//!
//! - **An example**, made up (never anyone's data, never a key): every field of the file's type is filled, and every
//!   variant of its enums is shown at least once. [`made_up`] asks the type itself what it is made of, so a new field
//!   or a new variant is in the example without a line here. The examples are kept in
//!   `tests/references/saved-files/`, one file per saved file.
//! - **Read and written again**: the app reads the example with the function it reads this file with, writes it with
//!   the one it writes it with, in a settings folder of the test's own, and the file must come back the same.
//! - **The kept examples are the files people have**: what one of them holds must still be there once read and
//!   written again by today's code, even when the examples are being written again (`ARGOS_REFERENCES=maj`). A field
//!   may be added, never lost.
//! - **Nothing forgotten**: every file name written in the code of the app is known here
//!   ([`every_file_name_of_the_code_is_known`]).
//!
//! The 4 files of the vault keep their own tests (`security/`), untouched. The 3 bases (`.redb`) are written, closed,
//! opened again and read ([`the_bases_give_back_what_they_were_given`]).

use super::{references_folder, updating};
use crate::tests::dossier_a_soi;
use crate::*;
use serde::de::{self, DeserializeOwned, DeserializeSeed, Visitor};
use serde::Serialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

/// As many values in a list, entries in a map, examples of a file, at most.
const MOST: usize = 256;
/// A type that holds itself stops there.
const DEEPEST: usize = 24;

/// What the made-up values are counted with.
#[derive(Default)]
struct Maker {
    /// The last number given: every made-up number and text has its own.
    n: u64,
    /// How many variants of each enum met were shown so far (an enum: its name and where its list of variants is).
    shown: BTreeMap<(&'static str, usize), usize>,
    /// How many times a variant was shown for the first time.
    firsts: usize,
    /// How deep in the value being made up.
    depth: usize,
}

/// Makes up a value of any type the app reads from a file: the type says what it is made of (`serde::Deserialize`),
/// and is given a number, a text, a yes, a list… in answer, never the same twice. Of an enum, the next variant each
/// time; of an enum read by trying its variants in turn (`untagged`), the one that takes a number.
#[derive(Clone, Copy)]
struct MadeUp<'a>(&'a RefCell<Maker>);

type Fail = de::value::Error;

impl MadeUp<'_> {
    fn next(self) -> u64 {
        let mut maker = self.0.borrow_mut();
        maker.n += 1;
        maker.n
    }

    fn firsts(self) -> usize {
        self.0.borrow().firsts
    }

    fn deep(self) -> bool {
        self.0.borrow().depth > DEEPEST
    }

    /// What `f` makes up is one level deeper.
    fn inside<T>(self, f: impl FnOnce() -> T) -> T {
        self.0.borrow_mut().depth += 1;
        let out = f();
        self.0.borrow_mut().depth -= 1;
        out
    }
}

macro_rules! whole_numbers {
    ($($method:ident)*) => {
        $(fn $method<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
            v.visit_u64(self.next() % 100 + 1)
        })*
    };
}

macro_rules! numbers {
    ($($method:ident)*) => {
        $(fn $method<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
            v.visit_f64((self.next() % 1000) as f64 + 0.5)
        })*
    };
}

macro_rules! texts {
    ($($method:ident)*) => {
        $(fn $method<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
            v.visit_string(format!("essai-{}", self.next()))
        })*
    };
}

impl<'de> de::Deserializer<'de> for MadeUp<'_> {
    type Error = Fail;

    whole_numbers!(deserialize_i8 deserialize_i16 deserialize_i32 deserialize_i64 deserialize_u8 deserialize_u16 deserialize_u32 deserialize_u64);
    numbers!(deserialize_any deserialize_f32 deserialize_f64);
    texts!(deserialize_str deserialize_string deserialize_identifier);

    fn deserialize_bool<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        v.visit_bool(true)
    }

    fn deserialize_char<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        v.visit_char('e')
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        self.deserialize_byte_buf(v)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        v.visit_byte_buf(vec![(self.next() % 250) as u8])
    }

    fn deserialize_option<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        if self.deep() {
            v.visit_none()
        } else {
            v.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        v.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, v: V) -> Result<V::Value, Fail> {
        v.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, v: V) -> Result<V::Value, Fail> {
        v.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        self.inside(|| v.visit_seq(Items::new(self, None)))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, v: V) -> Result<V::Value, Fail> {
        self.inside(|| v.visit_seq(Items::new(self, Some(len))))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _: &'static str, len: usize, v: V) -> Result<V::Value, Fail> {
        self.deserialize_tuple(len, v)
    }

    fn deserialize_map<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        self.inside(|| v.visit_map(Items::new(self, None)))
    }

    fn deserialize_struct<V: Visitor<'de>>(self, name: &'static str, fields: &'static [&'static str], v: V) -> Result<V::Value, Fail> {
        self.inside(|| {
            v.visit_map(Fields {
                made: self,
                of: name,
                fields,
                at: 0,
            })
        })
    }

    fn deserialize_enum<V: Visitor<'de>>(self, name: &'static str, variants: &'static [&'static str], v: V) -> Result<V::Value, Fail> {
        let k = {
            let mut maker = self.0.borrow_mut();
            let shown = maker.shown.entry((name, variants.as_ptr() as usize)).or_default();
            *shown += 1;
            let k = *shown - 1;
            if k < variants.len() {
                maker.firsts += 1;
            }
            k
        };
        let Some(variant) = variants.get(k % variants.len().max(1)) else {
            return Err(de::Error::custom(format!("the enum {name} has no variant")));
        };
        self.inside(|| v.visit_enum(Variant { made: self, name: variant }))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, v: V) -> Result<V::Value, Fail> {
        v.visit_unit()
    }
}

/// The values of a list or the entries of a map: `len` of them, or (a list, a map of any length) two, and more as
/// long as the last one showed a variant of an enum for the first time.
struct Items<'a> {
    made: MadeUp<'a>,
    len: Option<usize>,
    given: usize,
    /// The variants shown for the first time before the last one, and whether it showed one.
    before: usize,
    useful: bool,
}

impl<'a> Items<'a> {
    fn new(made: MadeUp<'a>, len: Option<usize>) -> Self {
        Self {
            made,
            len,
            given: 0,
            before: 0,
            useful: false,
        }
    }

    fn more(&self) -> bool {
        match self.len {
            Some(len) => self.given < len,
            None => !self.made.deep() && self.given < MOST && (self.given < 2 || self.useful),
        }
    }

    fn one<T>(&mut self, value: T) -> T {
        (self.given, self.useful) = (self.given + 1, self.made.firsts() > self.before);
        value
    }
}

impl<'de> de::SeqAccess<'de> for Items<'_> {
    type Error = Fail;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Fail> {
        if !self.more() {
            return Ok(None);
        }
        self.before = self.made.firsts();
        seed.deserialize(self.made).map(|value| Some(self.one(value)))
    }
}

impl<'de> de::MapAccess<'de> for Items<'_> {
    type Error = Fail;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Fail> {
        if !self.more() {
            return Ok(None);
        }
        self.before = self.made.firsts();
        seed.deserialize(self.made).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Fail> {
        seed.deserialize(self.made).map(|value| self.one(value))
    }
}

/// The fields of a struct, each one with a made-up value.
struct Fields<'a> {
    made: MadeUp<'a>,
    of: &'static str,
    fields: &'static [&'static str],
    at: usize,
}

impl<'de> de::MapAccess<'de> for Fields<'_> {
    type Error = Fail;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Fail> {
        let Some(field) = self.fields.get(self.at) else { return Ok(None) };
        seed.deserialize(de::value::StrDeserializer::new(field)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Fail> {
        let field = self.fields.get(self.at).copied().unwrap_or_default();
        self.at += 1;
        match by_hand(self.of, field) {
            Some(written) => {
                let value: Value = serde_json::from_str(written).map_err(de::Error::custom)?;
                seed.deserialize(value).map_err(de::Error::custom)
            }
            None => seed.deserialize(self.made),
        }
    }
}

/// One variant of an enum, with made-up values in it.
struct Variant<'a> {
    made: MadeUp<'a>,
    name: &'static str,
}

impl<'de> de::EnumAccess<'de> for Variant<'_> {
    type Error = Fail;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Fail> {
        Ok((seed.deserialize(de::value::StrDeserializer::new(self.name))?, self))
    }
}

impl<'de> de::VariantAccess<'de> for Variant<'_> {
    type Error = Fail;

    fn unit_variant(self) -> Result<(), Fail> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Fail> {
        seed.deserialize(self.made)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, v: V) -> Result<V::Value, Fail> {
        v.visit_seq(Items::new(self.made, Some(len)))
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], v: V) -> Result<V::Value, Fail> {
        v.visit_map(Fields {
            made: self.made,
            of: self.name,
            fields,
            at: 0,
        })
    }
}

/// What a type cannot say of itself: the field `field` of the struct `of`, written by hand. (A drawing is read in
/// its shape of today or in an older one, so it does not name its fields: two drawings, their style left to the
/// defaults. Every field of a style is in the examples of `templates.json`.)
fn by_hand(of: &str, field: &str) -> Option<&'static str> {
    match (of, field) {
        ("Saved", "drawings") => Some(
            r#"[{"tool":"trend","pts":[{"t":1700000000,"p":150.5},{"t":1700003600,"p":205.25}],"style":{}},
                {"tool":"hline","pts":[{"t":1700000000,"p":180.5}],"style":{}}]"#,
        ),
        _ => None,
    }
}

/// Made-up values of the type `T`: one, and more as long as the last one showed a variant of an enum for the first
/// time.
fn made_up<T: DeserializeOwned>() -> Result<Vec<T>, String> {
    let maker = RefCell::new(Maker::default());
    let made = MadeUp(&maker);
    let mut all = vec![];
    loop {
        let before = made.firsts();
        all.push(T::deserialize(made).map_err(|e| format!("no example can be made up: {e}"))?);
        if made.firsts() == before || all.len() >= MOST {
            return Ok(all);
        }
    }
}

/// The text of a saved file as a value, the keys of its objects in order: two maps written in another order are the
/// same file.
fn value(text: &str) -> Result<Value, String> {
    fn sorted(v: Value) -> Value {
        match v {
            Value::Array(items) => Value::Array(items.into_iter().map(sorted).collect()),
            Value::Object(map) => {
                let ordered: BTreeMap<String, Value> = map.into_iter().map(|(k, v)| (k, sorted(v))).collect();
                Value::Object(ordered.into_iter().collect())
            }
            other => other,
        }
    }
    serde_json::from_str::<Value>(text).map(sorted).map_err(|e| format!("not read: {e}"))
}

/// Where `new` is not what `old` was: the first place. `grown`: what `new` has and `old` did not is not a difference
/// (a field added since).
fn difference(old: &Value, new: &Value, grown: bool, at: &str) -> Option<String> {
    match (old, new) {
        (Value::Object(a), Value::Object(b)) => a
            .iter()
            .find_map(|(k, x)| match b.get(k) {
                Some(y) => difference(x, y, grown, &format!("{at}.{k}")),
                None => Some(format!("{at}.{k} is no longer there")),
            })
            .or_else(|| b.keys().find(|k| !grown && !a.contains_key(*k)).map(|k| format!("{at}.{k} is new"))),
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(k, (x, y))| difference(x, y, grown, &format!("{at}[{k}]"))),
        (Value::Array(a), Value::Array(b)) => Some(format!("{at} had {} values, has {}", a.len(), b.len())),
        _ if old == new => None,
        _ => Some(format!("{at} was {old}, is {new}")),
    }
}

/// One saved file of the app.
struct Kept {
    /// Its name as the code writes it.
    name: &'static str,
    /// The file the test reads and writes (where the name has a part that changes: « lang/xx.json »).
    file: &'static str,
    /// Made-up examples of it, each one as the app writes it.
    examples: Box<dyn Fn(&str) -> Result<Vec<String>, String>>,
    /// The app reads the file and writes it again.
    again: Box<dyn Fn(&str)>,
}

/// A file the app reads with `App::read_prefs` as a `T` and writes with `App::write_prefs`.
fn kept<T: DeserializeOwned + Serialize + Default + 'static>(name: &'static str) -> Kept {
    let written = |file: &str| platform::read(file).ok_or_else(|| "the app wrote nothing".to_string());
    Kept {
        name,
        file: name,
        examples: Box::new(move |file| {
            let write = |example: &T| {
                App::write_prefs(file, example);
                written(file)
            };
            made_up::<T>()?.iter().map(write).collect()
        }),
        again: Box::new(|file| App::write_prefs(file, &App::read_prefs::<T>(file))),
    }
}

/// The same, for a file kept in the app's state: its type is the one of that part of the state.
fn like<T: DeserializeOwned + Serialize + Default + 'static>(name: &'static str, _: fn(&App) -> &T) -> Kept {
    kept::<T>(name)
}

/// The same, for the file of a bar: the app writes it with its own function (`save`), which leaves in the file, as
/// they were, the fields another file holds since the new interface.
fn bar<T: DeserializeOwned + Serialize + Default + 'static>(name: &'static str, part: fn(&mut App) -> &mut T, save: fn(&App)) -> Kept {
    Kept {
        again: Box::new(move |file| {
            let mut a = super::at_rest(super::Form::Pc, true);
            *part(&mut a) = App::read_prefs(file);
            save(&a);
        }),
        ..kept::<T>(name)
    }
}

/// 🔔 The alerts. The prices of a firing are exact decimals written as text (« 58897.5 »), which no made-up text is:
/// one more example has them.
fn alerts() -> Kept {
    use argos_core::alerts::{decimal, Alerts};
    Kept {
        examples: Box::new(|file| {
            let made = made_up::<Alerts>()?;
            let mut priced = made.first().cloned().ok_or("no example of the alerts")?;
            for (k, fired) in priced.declenchements.iter_mut().enumerate() {
                (fired.target, fired.price) = (decimal(58_897.5 + k as f64), decimal(58_901.25 + k as f64));
            }
            let write = |example: &Alerts| {
                App::write_prefs(file, example);
                platform::read(file).ok_or_else(|| "the app wrote nothing".to_string())
            };
            made.iter().chain([&priced]).map(write).collect()
        }),
        ..like("alerts.json", |a| &a.alerts)
    }
}

/// 📋 The lists: kept by the store, and written as text by `argos_store::listes` for the phone and the web page.
fn lists() -> Kept {
    use argos_store::listes::{appliquer, lire, vers_json, Listes};
    let text = |lists: &Listes| {
        let mut store = Store::in_memory().map_err(|e| e.to_string())?;
        appliquer(&mut store, lists, true);
        Ok::<String, String>(vers_json(&store))
    };
    Kept {
        name: "watchlists.json",
        file: "watchlists.json",
        examples: Box::new(move |_| {
            let item = |section: &str, symbol: &str| (section.to_string(), symbol.to_string());
            let first = vec![item("Section A", "essai:AAA"), item("Section B", "essai:BBB"), item("Section A", "essai:CCC")];
            let made_up = Listes(vec![
                ("Essai 1".into(), vec!["Section A".into(), "Section B".into()], first),
                ("Essai 2".into(), vec![], vec![]),
            ]);
            Ok(vec![text(&made_up)?])
        }),
        again: Box::new(move |file| {
            if let Some(Ok(lists)) = platform::read(file).as_deref().and_then(lire).map(|lists| text(&lists)) {
                let _ = platform::write(file, &lists);
            }
        }),
    }
}

/// 🪙 The map of the CoinMarketCap ids, kept 7 days with the hour it was made (`argos_data::logos`).
fn id_map() -> Kept {
    use argos_data::logos::{carte_du_disque, garder_carte, Carte};
    let path = |file: &str| {
        crate::persist::saved::dir()
            .map(|d| d.join(file))
            .ok_or_else(|| "no settings folder".to_string())
    };
    Kept {
        name: "carte-cmc.json",
        file: "carte-cmc.json",
        examples: Box::new(move |file| {
            let write = |(hour, map): &(i64, Carte)| {
                garder_carte(&path(file)?, map, *hour).map_err(|e| e.to_string())?;
                platform::read(file).ok_or_else(|| "nothing was written".to_string())
            };
            made_up::<(i64, Carte)>()?.iter().map(write).collect()
        }),
        again: Box::new(move |file| {
            let hour = platform::read(file).and_then(|raw| serde_json::from_str::<Value>(&raw).ok()?["t"].as_i64());
            if let (Ok(path), Some(hour)) = (path(file), hour) {
                if let Some(map) = carte_du_disque(&path, hour) {
                    let _ = garder_carte(&path, &map, hour);
                }
            }
        }),
    }
}

/// Every saved file, with the type the app reads it as.
fn all() -> Vec<Kept> {
    use argos_render::indicators::Ind;
    vec![
        kept::<argos_render::academy::Progress>("academy.json"),
        like("ai_memory.json", |a| &a.ui.ai_mem),
        alerts(),
        like("backtest.json", |a| &a.ui.bt_prefs),
        id_map(),
        kept::<argos_data::cmcap::Credits>("cmc_credits.json"),
        like("cmc_history.json", |a| &a.feeds.cmc_hist),
        like("cmc_last.json", |a| &a.feeds.cmc_last),
        kept::<Vec<String>>("couleurs_perso.json"),
        like("demo_account.json", |a| &a.demo),
        like("demo_accounts.json", |a| &a.accounts),
        like("dock.json", |a| &a.watch.dock),
        like("exchanges.json", |a| &a.ex_prefs),
        kept::<crate::actions::hyperliquid::ExplorerPrefs>("explorer.json"),
        // (when the feed of notifications was last opened: a time in seconds)
        kept::<i64>("fil-vu.json"),
        like("ideas.json", |a| &a.ui.ideas),
        kept::<crate::actions::indicators::MesDefauts>("indicator_defaults.json"),
        kept::<argos_render::indicators::Settings>("indicator_inputs.json"),
        like("indicator_legend.json", |a| &a.ind.leg),
        kept::<argos_render::catalogue::ReglagesParNom>("indicator_settings.json"),
        like("indicator_texts.json", |a| &a.ind.texts),
        like("indicators.json", |a| &a.ind.on),
        kept::<Vec<Ind>>("indicators_defaults.json"),
        kept::<crate::actions::Interface2>("interface-2.json"),
        kept::<argos_core::journal::Donnees>("journal.json"),
        Kept {
            file: "lang/xx.json",
            ..kept::<HashMap<String, String>>("lang/{code}.json")
        },
        kept::<crate::actions::layouts::Layouts>("layouts.json"),
        like("liq_hl.json", |a| &a.feeds.liq_hl_oi),
        like("market_fav.json", |a| &a.ind.favs),
        like("modules-catalogue.json", |a| &a.mods.catalog),
        like("modules.json", |a| &a.mods.list),
        like("orderflow.json", |a| &a.feeds.enregistres),
        bar("rail.json", |a| &mut a.rail, App::save_rail),
        like("scripts.json", |a| &a.scripts),
        like("seance_replay.json", |a| &a.ui.seance),
        kept::<Vec<String>>("seeds.json"),
        kept::<crate::actions::synchro::Derniere>("session.json"),
        like("split.json", |a| &a.split),
        like("templates.json", |a| &a.templates),
        like("tool_styles.json", |a| &a.tool_styles),
        bar("toolbar.json", |a| &mut a.tools, App::save_tools),
        bar("topbar.json", |a| &mut a.top.prefs, App::save_top),
        kept::<crate::actions::WalletPrefs>("wallet.json"),
        like("watchlist.json", |a| &a.watch.prefs),
        lists(),
        kept::<crate::actions::WhaleStore>("whales.json"),
        // (one file per market: its chart type, its zoom, its settings, its drawings)
        Kept {
            file: "essai-btc.json",
            ..kept::<crate::persist::saved::Saved>("{}.json")
        },
    ]
}

/// What a file name written in the code is, when it is not a saved file with its example here.
const OTHER_NAMES: &[(&str, &str)] = &[
    ("account.json", "the vault: its own tests (`security/account.rs`)"),
    ("account-key.json", "the vault: its own tests (`security/vault.rs`)"),
    ("claude-key.json", "the vault: its own tests (`security/vault.rs`)"),
    ("passkey.json", "the vault: its own tests (`security/passkey.rs`)"),
    ("argos.redb", "a base: `the_bases_give_back_what_they_were_given`"),
    ("murs.redb", "a base: `the_bases_give_back_what_they_were_given`"),
    ("sensible.redb", "a base: `the_bases_give_back_what_they_were_given`"),
    (
        "../../../../../app/modules/academie.json",
        "the manifest of a module that comes with the app (read when it is built)",
    ),
    (
        "../../../../../app/modules/ema.json",
        "the manifest of a module that comes with the app (read when it is built)",
    ),
    (
        "../../../../../app/modules/moyenne.json",
        "the manifest of a module that comes with the app (read when it is built)",
    ),
    (
        "../../../../../app/modules/pinceau-zone.json",
        "the manifest of a module that comes with the app (read when it is built)",
    ),
    ("../propfirm-defaults.json", "the PropFirm rules that come with the app (read when it is built)"),
    (".json", "the end of a file name"),
    (".wasm,.json", "the kinds of files offered when a module is picked"),
    (
        "argos-catalogue.json",
        "a catalogue of modules to import, each one checked with the kit; what is kept of it is `modules-catalogue.json`",
    ),
    ("argos-idees.json", "an export to the downloads folder"),
    ("argos-journal.json", "an export to the downloads folder"),
    ("argos-module.json", "the manifest inside the package of a module"),
    ("argos-v1.json", "the export of Argos v1, imported once"),
    ("billet.json", "a Support ticket: its own tests (`argos-core/src/retours.rs`)"),
    ("dist/app/version.json", "the version of the web app, written when it is built"),
    (
        "indicator_inputs.avant-noms.json",
        "a copy of `indicator_inputs.json` as it was, never read again",
    ),
    (
        "{base}.avant-conversion-{date}.json",
        "a dated copy of a settings file as it was, never read again",
    ),
    ("lang/{}.json", "the same file as `lang/{code}.json`"),
    ("{dir}/{coin}_{tf}.json", "the candles the scanner tool keeps (not the app)"),
    ("{stem}.json", "the manifest next to the `.wasm` of a module"),
    ("{}/{code}.json", "the address of a language pack on its server"),
];

/// Reads a saved file's examples made up today, has the app read and write each one again, and compares them with
/// the examples kept (or keeps them).
fn check(k: &Kept, folder: &Path) -> Result<(), String> {
    // the app reads this text as the file and writes the file again: what the file then holds
    let again = |example: &Value| {
        platform::write(k.file, &serde_json::to_string_pretty(example).map_err(|e| e.to_string())?)?;
        (k.again)(k.file);
        value(&platform::read(k.file).ok_or("the app did not write it again")?)
    };
    let today = (k.examples)(k.file)?.iter().map(|text| value(text)).collect::<Result<Vec<_>, _>>()?;
    for (n, example) in today.iter().enumerate() {
        if let Some(d) = difference(example, &again(example)?, false, "") {
            return Err(format!("the example {n} made up today, read and written again, is not the same: {d}"));
        }
    }
    let path = folder.join(k.file.replace('/', "-"));
    if let Ok(known) = std::fs::read_to_string(&path) {
        let Value::Array(known) = value(&known)? else {
            return Err(format!("{} is not a list of examples", path.display()));
        };
        // (the kept examples are the files people have: nothing of them may be lost, even when they are written again)
        for (n, example) in known.iter().enumerate() {
            if let Some(d) = difference(example, &again(example)?, true, "") {
                return Err(format!(
                    "the kept example {n} is no longer read and written back whole: {d}. The files people have would \
                     lose it"
                ));
            }
        }
        if !updating() {
            return match difference(&Value::Array(known), &Value::Array(today), false, "") {
                None => Ok(()),
                Some(d) => Err(format!(
                    "its type is not the one its examples were made up from: {d}. A wanted change: ARGOS_REFERENCES=maj, \
                     then read and commit the diff of {}",
                    path.display()
                )),
            };
        }
    }
    let text = serde_json::to_string_pretty(&Value::Array(today)).map_err(|e| e.to_string())?;
    std::fs::write(&path, text + "\n").map_err(|e| e.to_string())
}

/// 🕸 Every saved file: its made-up examples are read and written again the same by the app, they are the examples
/// kept, and nothing the kept examples hold is lost. `ARGOS_REFERENCES=maj` (or no example yet) keeps the examples.
#[test]
fn every_saved_file_is_read_and_written_the_same() {
    dossier_a_soi("safety-net-saved");
    let folder = references_folder().join("saved-files");
    std::fs::create_dir_all(&folder).expect("the folder of the examples");
    let wrong: Vec<String> = all()
        .iter()
        .filter_map(|k| check(k, &folder).err().map(|problem| format!("{}: {problem}", k.name)))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} saved file(s) are not read and written the same:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The file names (`….json`, `….redb`) written in the code of the app: every crate of the workspace, without its
/// tests.
fn names_in_the_code() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for (_, code) in super::boundaries::code_of_the_app() {
        // (the tests at the end of a file are not the app)
        let app = code.split("\n#[cfg(test)]\nmod ").next().unwrap_or_default();
        for written in app.split('"').skip(1).step_by(2) {
            let is_a_name = (written.ends_with(".json") || written.ends_with(".redb")) && !written.contains(' ') && !written.starts_with("http");
            if is_a_name {
                names.insert(written.to_string());
            }
        }
    }
    names
}

/// 🕸 Every file name written in the code of the app is a saved file with its example, or is said here to be
/// something else; and nothing is listed here that the code no longer writes.
#[test]
fn every_file_name_of_the_code_is_known() {
    let in_the_code = names_in_the_code();
    let kept: BTreeSet<&str> = all().iter().map(|k| k.name).collect();
    let known: BTreeSet<&str> = kept.iter().copied().chain(OTHER_NAMES.iter().map(|(name, _)| *name)).collect();
    let unknown: Vec<&String> = in_the_code.iter().filter(|name| !known.contains(name.as_str())).collect();
    let gone: Vec<&&str> = known.iter().filter(|name| !in_the_code.contains(**name)).collect();
    assert!(
        unknown.is_empty(),
        "file name(s) of the code that safety_net/saved.rs does not know (a saved file: one line in `all`, it gets its \
         example; something else: say what in `OTHER_NAMES`): {unknown:?}"
    );
    assert!(
        gone.is_empty(),
        "file name(s) listed in safety_net/saved.rs that the code no longer writes: {gone:?}"
    );
}

/// 🕸 The three bases give back what they were given: made-up entries are written, the base is closed, opened
/// again, and the same entries are read. The journal's base is opened with a made-up key (never the keyring's).
#[test]
fn the_bases_give_back_what_they_were_given() {
    use argos_core::journal::{Changements, Donnees};
    use argos_store::listes::{appliquer, vers_json, Listes};
    let folder = argos_core::dossiers::de_test("safety-net-bases");
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the folder of the bases");
    // 📋 argos.redb: the lists, their sections, their symbols
    let path = folder.join("argos.redb");
    let lists = Listes(vec![
        (
            "Essai 1".into(),
            vec!["Section A".into()],
            vec![("Section A".into(), "essai:AAA".into()), ("Section A".into(), "essai:BBB".into())],
        ),
        ("Essai 2".into(), vec![], vec![]),
    ]);
    let written = {
        let mut store = Store::open(&path).expect("the base of the lists opens");
        assert!(appliquer(&mut store, &lists, true), "the lists are written");
        vers_json(&store)
    };
    assert!(written.contains("essai:BBB"), "the lists are in the base: {written}");
    assert_eq!(vers_json(&Store::open(&path).expect("the base of the lists opens again")), written);
    // 🧱 murs.redb: the walls of a market
    let path = folder.join("murs.redb");
    let mut walls: Vec<argos_core::murs::Event> = made_up().expect("made-up walls").remove(0);
    walls.sort_by_key(|w| w.t_ms);
    argos_store::murs::MursDb::open(&path)
        .and_then(|base| base.add("essai:AAA", &walls))
        .expect("the walls are written");
    let read = argos_store::murs::MursDb::open(&path).and_then(|base| base.read("essai:AAA", i64::MIN, i64::MAX));
    assert_eq!(read.expect("the walls are read"), walls);
    // 🔐 sensible.redb: the journal, its settings and its trades
    let (path, lock) = (folder.join("sensible.redb"), [7u8; 32]);
    let journal: Donnees = made_up().expect("a made-up journal").remove(0);
    let change = Changements {
        cfg: Some(journal.cfg.clone()),
        mettre: journal.trades.clone(),
        retirer: vec![],
    };
    let written = argos_store::journal::un_moment(&path, &lock, |base| argos_store::journal::appliquer(base, &change)).expect("the journal is written");
    assert_eq!((&written.cfg, &written.trades), (&journal.cfg, &journal.trades));
    let read = argos_store::journal::un_moment(&path, &lock, argos_store::journal::lire).expect("the journal is read");
    assert_eq!(read, written);
    std::fs::remove_dir_all(&folder).expect("the folder of the bases is removed");
}
