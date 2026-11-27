//! Morceaux de l'état de `App`, regroupés par sujet (un sous-état par fichier).

mod chart;
mod dock;
mod feeds;
mod indicators;
mod pointer;
mod session;
mod ui;

pub(crate) use chart::{ChartState, HIST_MAX};
pub(crate) use dock::{DockState, GesteDock};
pub(crate) use feeds::{EtatPetites, Feeds, ScriptMtf};
pub(crate) use indicators::IndState;
pub(crate) use pointer::PointerState;
pub(crate) use session::SessionState;
pub(crate) use ui::{TicketZoom, UiState, OUVERTS};
