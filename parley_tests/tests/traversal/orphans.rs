// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! What editing can strand: graphemes that render as nothing but are still in the buffer.
//!
//! A per-codepoint backspace can cut a multi-codepoint grapheme anywhere. Most of the resulting
//! fragments are still *visible* — deleting the acute from `e` + combining acute leaves `e`, and
//! deleting the `e` leaves a lone acute, which at least draws a mark on a dotted circle. This
//! module looks for the sharper case: a grapheme left holding **only** codepoints that have no
//! visual form at all, so the buffer is non-empty, the caret can sit beside it, arrow keys take a
//! press to cross it — and the user sees nothing whatsoever.
//!
//! A buffer that is one lone U+200D is the archetype. It is not a hypothetical: a ZWJ emoji
//! sequence is a chain of visible codepoints joined by ZWJs, and a per-codepoint delete can take
//! the visible ones and leave a joiner behind.
//!
//! # Why this is a search and not a trajectory
//!
//! `tables.rs` measures trajectories: one operation applied repeatedly from one end. That can only
//! ever find what *that* op does from *that* start. An orphan is produced by a **combination** —
//! backspace here, forward delete there — so the question is reachability over the whole state
//! space, not the behaviour of a single op. [`OrphanSearch`] is a breadth-first search over every
//! buffer reachable by `delete` and `backdelete` from every caret position, which makes the answer
//! "no sequence of presses can produce this" rather than "the sequence I happened to try did not".
//!
//! BFS rather than DFS so every witness is a **shortest** one: "two presses strand a ZWJ" is a much
//! stronger claim than "here is some 9-press path that does".
//!
//! # Scope
//!
//! `delete` and `backdelete` only — not their word variants. That is the question being asked, and
//! it keeps every witness a sequence of plain key presses that a user could obviously perform. Word
//! ops can only remove more per press, so adding them could not make an orphan *unreachable*; they
//! could only add paths to orphans, and possibly new ones.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use std::collections::{BTreeMap, VecDeque};

use super::env::TraversalEnv;
use super::measure::{char_boundaries, render_codepoints, with_panics_silenced};
use super::ops::Op;
use super::reference::{egc_boundaries, ink, is_undisplayable};

/// The edits the search explores. See the module docs.
const SEARCH_OPS: &[Op] = &[Op::Delete, Op::Backdelete];

/// Ceiling on distinct buffers explored per entry.
///
/// Every edit either shrinks the buffer or leaves it unchanged, so the search is a DAG on
/// decreasing length and terminates on its own. This is a backstop against a future `parley` whose
/// edits do something else, not a real limit: [`OrphanSearch::truncated`] records if it ever binds,
/// and the golden would show it.
const MAX_STATES: usize = 20_000;

/// One press in a witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Press {
    pub(crate) op: Op,
    /// Byte offset the caret was placed at before the press.
    pub(crate) at: usize,
}

impl Press {
    pub(crate) fn render(self) -> String {
        format!("{}@{}", self.op.slug(), self.at)
    }
}

/// A grapheme that renders as nothing, and the shortest way to produce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Orphan {
    /// The undisplayable grapheme itself, as codepoints.
    pub(crate) egc: String,
    /// What each of its codepoints is, e.g. `ZWJ`.
    pub(crate) classes: String,
    /// The whole buffer it appeared in, as codepoints.
    pub(crate) buffer: String,
    /// Shortest press sequence from the original text. Empty means the original text already
    /// contained it, before any editing.
    pub(crate) witness: Vec<Press>,
}

impl Orphan {
    pub(crate) fn render_witness(&self) -> String {
        if self.witness.is_empty() {
            return String::from("(already present)");
        }
        self.witness
            .iter()
            .map(|p| p.render())
            .collect::<Vec<_>>()
            .join(" → ")
    }
}

/// Breadth-first search over every buffer `delete`/`backdelete` can reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OrphanSearch {
    /// Distinct buffers reached, including the original.
    pub(crate) states: usize,
    /// Whether [`MAX_STATES`] bound. Always expected `false`; recorded so it cannot bind silently.
    pub(crate) truncated: bool,
    /// Distinct undisplayable graphemes found, counted before grouping.
    pub(crate) distinct: usize,
    /// One row per distinct **class signature** (`ZWJ`, `TAG+TAG`, …), holding the shortest-witness
    /// example of it. Sorted by signature so the golden is stable.
    ///
    /// Grouped rather than listed exhaustively because the list is combinatorial and the
    /// combinations say nothing new: a 6-tag flag strands 60-odd distinct subsets of tag
    /// characters, and printing all of them buries the finding — which is simply that a tag
    /// character can be left alone — under its own permutations. The signature is the fact; the
    /// example proves it; `distinct` keeps the full count honest.
    pub(crate) by_class: Vec<Orphan>,
}

impl OrphanSearch {
    pub(crate) fn run(env: &mut TraversalEnv, text: &str) -> Self {
        // buffer -> shortest witness reaching it.
        let mut seen: BTreeMap<String, Vec<Press>> = BTreeMap::new();
        let mut queue: VecDeque<String> = VecDeque::new();
        let mut truncated = false;

        seen.insert(text.to_string(), Vec::new());
        queue.push_back(text.to_string());

        while let Some(buffer) = queue.pop_front() {
            let witness = seen[&buffer].clone();
            for from in char_boundaries(&buffer) {
                for op in SEARCH_OPS {
                    let Some(next) = apply(env, &buffer, from, *op) else {
                        continue; // Panicked. `traversal_findings` is where that is recorded.
                    };
                    if next == buffer || seen.contains_key(&next) {
                        continue;
                    }
                    if seen.len() >= MAX_STATES {
                        truncated = true;
                        continue;
                    }
                    let mut w = witness.clone();
                    w.push(Press { op: *op, at: from });
                    seen.insert(next.clone(), w);
                    queue.push_back(next);
                }
            }
        }

        // Collect the undisplayable graphemes across every reachable buffer, keeping the shortest
        // witness per class signature. BFS order is not enough on its own: two buffers can contain
        // the same orphan, so compare witness lengths explicitly. Ties break on the codepoints so
        // the choice of example is deterministic and the golden cannot flap.
        let mut distinct: BTreeMap<String, ()> = BTreeMap::new();
        let mut best: BTreeMap<String, Orphan> = BTreeMap::new();
        for (buffer, witness) in &seen {
            for egc in undisplayable_egcs(buffer) {
                distinct.insert(egc.clone(), ());
                let classes = egc
                    .chars()
                    .map(|c| ink(c).slug())
                    .collect::<Vec<_>>()
                    .join("+");
                let candidate = Orphan {
                    egc: render_codepoints(&egc),
                    classes: classes.clone(),
                    buffer: render_codepoints(buffer),
                    witness: witness.clone(),
                };
                best.entry(classes)
                    .and_modify(|existing| {
                        let better = (candidate.witness.len(), &candidate.egc)
                            < (existing.witness.len(), &existing.egc);
                        if better {
                            *existing = candidate.clone();
                        }
                    })
                    .or_insert(candidate);
            }
        }

        Self {
            states: seen.len(),
            truncated,
            distinct: distinct.len(),
            by_class: best.into_values().collect(),
        }
    }
}

/// Applies one edit to `buffer` with the caret at `from`. `None` if it panicked.
fn apply(env: &mut TraversalEnv, buffer: &str, from: usize, op: Op) -> Option<String> {
    with_panics_silenced(|| {
        let mut editor = env.editor(buffer);
        {
            let mut drv = env.driver(&mut editor);
            drv.move_to_byte(from);
            op.apply(&mut drv);
        }
        editor.raw_text().to_string()
    })
    .ok()
}

/// Every grapheme in `text` that has no visual form at all.
fn undisplayable_egcs(text: &str) -> Vec<String> {
    let bounds = egc_boundaries(text);
    bounds
        .windows(2)
        .map(|w| &text[w[0]..w[1]])
        .filter(|egc| is_undisplayable(egc))
        .map(String::from)
        .collect()
}
