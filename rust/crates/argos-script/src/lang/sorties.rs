//! 🖍 Does a script output anything? A static answer from the parsed script (no run): true when it holds at least one
//! call to an output function anywhere — top level, `if`, loop, function body — or is a `strategy(...)`. The editor
//! uses it to say « ce script ne trace rien » instead of adding an empty pane (a paste cut after its helpers).

use super::ast::*;
use std::collections::HashMap;

/// Output functions called by name (`plot(...)`).
const SORTIES: &[&str] = &[
    "plot",
    "plotshape",
    "plotchar",
    "plotarrow",
    "plotcandle",
    "plotbar",
    "hline",
    "fill",
    "bgcolor",
    "barcolor",
    "alert",
    "alertcondition",
    // (a strategy draws its trades and equity)
    "strategy",
];

/// Output functions called in a namespace (`label.new(...)`).
const SORTIES_NS: &[&str] = &[
    "label.new",
    "line.new",
    "box.new",
    "table.new",
    "linefill.new",
    "polyline.new",
    "strategy.entry",
    "strategy.order",
    "strategy.exit",
    "strategy.close",
    "strategy.close_all",
];

/// True when the script holds a call to an output function anywhere, or is a `strategy(...)`.
pub fn has_output_calls(s: &Script) -> bool {
    bloc(&s.body, &mut |f, _| match &f.k {
        Ek::Ident(n) => SORTIES.contains(&n.as_str()),
        Ek::Member(b, m) => matches!(&b.k, Ek::Ident(ns) if SORTIES_NS.contains(&format!("{ns}.{m}").as_str())),
        _ => false,
    })
}

/// The place of each `input(...)` / `input.*(...)` call among the inputs written in the script, in the order of its
/// text (1 for the first), by the id of the call. An untitled input is named after it (« Input 3 »): the name comes
/// from the text alone, so it is the same at every bar, in the chart's run and in a run on another timeframe,
/// whatever the order in which a run reaches the inputs.
pub(super) fn places_des_inputs(s: &Script) -> HashMap<u32, usize> {
    let mut places = HashMap::new();
    bloc(&s.body, &mut |f, id| {
        let input = match &f.k {
            Ek::Ident(n) => n == "input",
            Ek::Member(b, _) => matches!(&b.k, Ek::Ident(ns) if ns == "input"),
            _ => false,
        };
        if input {
            let place = places.len() + 1;
            places.insert(id, place);
        }
        false
    });
    places
}

/// What is asked of each call, in the order of the text: its function and its id. The walk stops at the first yes.
type Vu<'v> = &'v mut dyn FnMut(&Expr, u32) -> bool;

fn bloc(b: &Block, vu: Vu) -> bool {
    b.iter().any(|st| stmt(st, vu))
}

fn stmt(st: &Stmt, vu: Vu) -> bool {
    match st {
        Stmt::Decl { value, .. } => expr(value, vu),
        Stmt::Assign { target, value, .. } => expr(target, vu) || expr(value, vu),
        Stmt::Expr(e) => expr(e, vu),
        Stmt::Fn { params, body, .. } => params.iter().filter_map(|p| p.1.as_ref()).any(|e| expr(e, vu)) || bloc(body, vu),
        Stmt::Type { fields, .. } => fields.iter().filter_map(|f| f.1.as_ref()).any(|e| expr(e, vu)),
        Stmt::Break | Stmt::Continue => false,
    }
}

fn expr(e: &Expr, vu: Vu) -> bool {
    match &e.k {
        Ek::Num(_) | Ek::Str(_) | Ek::Bool(_) | Ek::Na | Ek::Color(_) | Ek::Ident(_) => false,
        Ek::Member(b, _) => expr(b, vu),
        Ek::Call { id, f, args, .. } => vu(f, *id) || expr(f, vu) || args.iter().any(|a| expr(&a.1, vu)),
        Ek::Index { e, n, .. } => expr(e, vu) || expr(n, vu),
        Ek::Unary(_, a) | Ek::Lambda(_, a) => expr(a, vu),
        Ek::Binary(_, a, b) => expr(a, vu) || expr(b, vu),
        Ek::Ternary(c, a, b) => expr(c, vu) || expr(a, vu) || expr(b, vu),
        Ek::Tuple(v) => v.iter().any(|x| expr(x, vu)),
        Ek::If(arms, otherwise) => arms.iter().any(|(c, b)| expr(c, vu) || bloc(b, vu)) || otherwise.as_ref().is_some_and(|b| bloc(b, vu)),
        Ek::Switch(s, cases, d) => {
            s.as_deref().is_some_and(|e| expr(e, vu)) || cases.iter().any(|(c, b)| expr(c, vu) || bloc(b, vu)) || d.as_ref().is_some_and(|b| bloc(b, vu))
        }
        Ek::For { from, to, by, body, .. } => expr(from, vu) || expr(to, vu) || by.as_deref().is_some_and(|e| expr(e, vu)) || bloc(body, vu),
        Ek::ForIn { coll, body, .. } => expr(coll, vu) || bloc(body, vu),
        Ek::While(c, body) => expr(c, vu) || bloc(body, vu),
    }
}
