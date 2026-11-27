//! 🧹 The drawings a script deletes (B-064). A script that draws at each bar deletes as much as it makes: a label
//! that replaces the one of the bar before, 200 boxes drawn again at every bar. Each list of drawings only ever grew:
//! a deleted drawing kept its place and its texts until the end of the run, and each new drawing read the whole list
//! again to count the live ones. 20 000 labels made and deleted took 2 × 10⁸ steps, 200 000 took 2 × 10¹⁰: the app
//! stood still, and the chart then went through every deleted drawing at each image.
//!
//! A drawing is known by its id: its rank among all the drawings of its list the script ever made. `Rangs` tells
//! where a drawing is in the list (`place`) and which live one of a sort is the oldest (`plus_vieux`), and takes the
//! deleted ones out of the list once they are more than the live ones (`tasser`). The ids never change, the places do.

use super::memoire;
use std::collections::VecDeque;

/// Deleted entries a list may hold before it is packed. Under that the list is left as it is, and the id of a
/// drawing is its place, as it was before; past that, the deleted ones go once they are more than the live ones, so
/// packing a list costs less than drawing it did.
pub(super) const RESTES: usize = 1024;

/// The ids of one list of drawings (labels, lines and boxes; polylines; linefills).
pub(super) struct Rangs {
    /// The id of each entry of the list, in the order of the list (ids only go up). `None` while the list was never
    /// packed: the id is the place.
    ids: Option<Vec<usize>>,
    /// The id the next entry gets: how many entries the list ever had.
    prochain: usize,
    /// Entries of the list that are deleted and still in it.
    effaces: usize,
    /// By sort: the ids of its entries from the oldest (the deleted ones stay in until they come first or the list is
    /// packed), and how many are alive.
    sortes: Vec<(VecDeque<usize>, usize)>,
}

impl Rangs {
    /// For a list of `sortes` sorts of drawings, each with its own count.
    pub fn new(sortes: usize) -> Rangs {
        Rangs {
            ids: None,
            prochain: 0,
            effaces: 0,
            sortes: (0..sortes).map(|_| (VecDeque::new(), 0)).collect(),
        }
    }

    /// Where the entry `id` is in the list; `None` once it was taken out of it (it was deleted).
    pub fn place(&self, id: usize) -> Option<usize> {
        match &self.ids {
            None => (id < self.prochain).then_some(id),
            Some(ids) => ids.binary_search(&id).ok(),
        }
    }

    /// An entry of the sort `sorte` was just put at the end of the list: its id.
    pub fn nouveau(&mut self, sorte: usize) -> usize {
        let id = self.prochain;
        self.prochain += 1;
        if let Some(ids) = &mut self.ids {
            memoire::pousser(ids, id);
        }
        let (file, vivants) = &mut self.sortes[sorte];
        let avant = file.capacity();
        file.push_back(id);
        memoire::grandi::<usize>(avant, file.capacity());
        *vivants += 1;
        id
    }

    /// A live entry of the sort `sorte` was just deleted: it stays in the list until the list is packed.
    pub fn efface(&mut self, sorte: usize) {
        self.effaces += 1;
        let vivants = &mut self.sortes[sorte].1;
        *vivants = vivants.saturating_sub(1);
    }

    /// While the sort `sorte` has more than `max` live entries: the place of its oldest live one, for the caller to
    /// delete (and to tell with `efface`). `vivant(place)`: whether the entry at that place is alive.
    pub fn plus_vieux(&mut self, sorte: usize, max: usize, vivant: impl Fn(usize) -> bool) -> Option<usize> {
        while self.sortes[sorte].1 > max {
            let id = self.sortes[sorte].0.pop_front()?;
            #[cfg(test)]
            PAS.with(|p| p.set(p.get() + 1));
            if let Some(place) = self.place(id).filter(|p| vivant(*p)) {
                return Some(place);
            }
        }
        None
    }

    /// Takes the deleted entries out of `liste` when they are too many (`RESTES`). `sorte(entry)`: the sort of a
    /// live entry, `None` for a deleted one.
    pub fn tasser<T>(&mut self, liste: &mut Vec<T>, sorte: impl Fn(&T) -> Option<usize>) {
        if self.effaces <= RESTES.max(liste.len().saturating_sub(self.effaces)) {
            return;
        }
        #[cfg(test)]
        PAS.with(|p| p.set(p.get() + liste.len()));
        let avant = self.ids.take();
        let mut ids = Vec::with_capacity(liste.len().saturating_sub(self.effaces));
        for (file, vivants) in &mut self.sortes {
            file.clear();
            *vivants = 0;
        }
        let mut k = 0;
        liste.retain(|x| {
            let id = avant.as_ref().map_or(k, |v| v[k]);
            k += 1;
            let Some(s) = sorte(x) else { return false };
            ids.push(id);
            self.sortes[s].0.push_back(id);
            self.sortes[s].1 += 1;
            true
        });
        self.ids = Some(ids);
        self.effaces = 0;
    }

    /// The bytes these ids hold (B-062).
    pub fn poids(&self) -> usize {
        #[cfg(test)]
        PESES.with(|p| p.set(p.get() + 1));
        let places = self.ids.as_ref().map_or(0, Vec::capacity) + self.sortes.iter().map(|s| s.0.capacity()).sum::<usize>();
        places.saturating_mul(std::mem::size_of::<usize>())
    }
}

#[cfg(test)]
thread_local! {
    /// 🧪 Entries looked at by `plus_vieux` and `tasser` on this thread.
    static PAS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// 🧪 Times a list of ids was asked what it holds (`poids`) on this thread.
    static PESES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// 🧪 For the tests: how many times a list of ids told what it holds since the last call.
#[cfg(test)]
pub(super) fn peses() -> usize {
    PESES.with(|p| p.replace(0))
}

/// 🧪 For the tests: the entries looked at since the last call, to find the oldest drawings and to pack the lists.
#[cfg(test)]
pub(super) fn pas() -> usize {
    PAS.with(|p| p.replace(0))
}
