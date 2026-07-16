// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Measurement primitives: what parley's layout actually contains.
//!
//! Everything here observes; nothing here judges. Comparison against the ICU oracle happens in the
//! tables, and the detectors live in `findings.rs`.
//!
//! # No floats in recorded data
//!
//! [`ClusterRow::zero_advance`] is a `bool`, not the `f32` it came from. Text goldens compare
//! byte-exactly, and an advance depends on the font, the rasteriser and the platform's float
//! behavior — recording it would produce a table that flaps without any behavior having changed.
//! The existing image snapshots need a tolerance for exactly this reason; text goldens have none,
//! so the data must be discrete at the point of capture.
//!
//! Floats are fine as *inputs* (hit-test probe points), just never as outputs.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use std::ops::Range;
use std::panic::{AssertUnwindSafe, catch_unwind};

use parley::{Affinity, Cluster, Layout, PlainEditor};

use super::env::TraversalEnv;
use super::ops::Op;
use crate::util::ColorBrush;

/// One `ClusterData` as parley built it.
///
/// This is the font-sensitive artifact: if shaping affects anything the traversal APIs read, it
/// shows up as a difference between two [`ClusterTable`]s built with different fonts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClusterRow {
    /// Logical index within the layout.
    pub(crate) idx: usize,
    /// `Cluster::text_range()`. **The load-bearing field.**
    pub(crate) range: Range<usize>,
    pub(crate) is_word_boundary: bool,
    /// `Cluster::is_emoji()` — the hardcoded-range one that `backdelete` consults, NOT the Emoji
    /// property. See `reference::is_uts51_emoji` for the oracle.
    pub(crate) is_emoji: bool,
    pub(crate) is_hard_line_break: bool,
    pub(crate) is_soft_line_break: bool,
    pub(crate) is_ligature_start: bool,
    pub(crate) is_ligature_continuation: bool,
    pub(crate) is_rtl: bool,
    pub(crate) glyph_len: usize,
    /// Whether the advance is exactly zero. A cursor stop at a zero-advance position is invisible
    /// to the user, which is the Blink/Burmese "stationary cursor" hazard.
    pub(crate) zero_advance: bool,
}

/// Every cluster in a layout, in logical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClusterTable {
    pub(crate) rows: Vec<ClusterRow>,
    pub(crate) line_count: usize,
}

impl ClusterTable {
    /// Walks the layout's clusters via `next_logical`, the same traversal `Cluster`-based code uses.
    pub(crate) fn measure(layout: &Layout<ColorBrush>) -> Self {
        let mut rows = Vec::new();
        let mut cluster = Cluster::from_byte_index(layout, 0);
        let mut idx = 0;
        while let Some(c) = cluster {
            rows.push(ClusterRow {
                idx,
                range: c.text_range(),
                is_word_boundary: c.is_word_boundary(),
                is_emoji: c.is_emoji(),
                is_hard_line_break: c.is_hard_line_break(),
                is_soft_line_break: c.is_soft_line_break(),
                is_ligature_start: c.is_ligature_start(),
                is_ligature_continuation: c.is_ligature_continuation(),
                is_rtl: c.is_rtl(),
                glyph_len: c.glyphs().count(),
                zero_advance: c.advance() == 0.0,
            });
            idx += 1;
            cluster = c.next_logical();
        }
        Self {
            rows,
            line_count: layout.len(),
        }
    }

    /// Cluster start offsets. These are the positions a codepoint-granular cursor can stop at.
    pub(crate) fn boundaries(&self) -> Vec<usize> {
        self.rows.iter().map(|r| r.range.start).collect()
    }

    /// `text_range` for every cluster — the projection the font-sensitivity probe compares.
    pub(crate) fn ranges(&self) -> Vec<Range<usize>> {
        self.rows.iter().map(|r| r.range.clone()).collect()
    }

    /// Total bytes covered by clusters. Should equal `text.len()`; if it does not, some source
    /// char produced no cluster and traversal cannot reach it.
    pub(crate) fn covered_bytes(&self) -> usize {
        self.rows.iter().map(|r| r.range.len()).sum()
    }
}

/// A selection, flattened to the parts a golden can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelState {
    pub(crate) anchor: usize,
    pub(crate) focus: usize,
    pub(crate) affinity: Affinity,
}

impl SelState {
    fn of(editor: &PlainEditor<ColorBrush>) -> Self {
        let sel = editor.raw_selection();
        Self {
            anchor: sel.anchor().index(),
            focus: sel.focus().index(),
            affinity: sel.focus().affinity(),
        }
    }

    pub(crate) fn is_collapsed(self) -> bool {
        self.anchor == self.focus
    }

    /// `focus@affinity` for a collapsed selection, `anchor-focus@affinity` otherwise.
    pub(crate) fn render(self) -> String {
        let aff = match self.affinity {
            Affinity::Downstream => "D",
            Affinity::Upstream => "U",
        };
        if self.is_collapsed() {
            format!("{}{}", self.focus, aff)
        } else {
            format!("{}-{}{}", self.anchor, self.focus, aff)
        }
    }
}

/// How a single motion step moved the caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorDelta {
    /// The index changed.
    Moved,
    /// The index did not change but the affinity flipped.
    ///
    /// Its own category because it is invisible in a table of indices, yet it is exactly what
    /// "two presses to cross one thing" looks like from the user's side.
    AffinityOnlyFlip,
    /// Nothing changed at all.
    Stationary,
}

/// The result of applying one operation once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    Motion {
        to: SelState,
        delta: CursorDelta,
    },
    Edit {
        to: SelState,
        /// The codepoints the buffer lost, as `U+XXXX`. Computed by diffing, not narrated.
        removed: String,
        /// Byte offset where the removal began.
        removed_at: usize,
    },
    /// The operation panicked. Recorded as data: `parley_tests` is a single test binary, so a
    /// propagating panic would destroy every other measurement in the run.
    Panicked(String),
}

/// One row of the step sweep: apply `op` once, starting from `from`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepRow {
    /// The byte offset we asked the editor to move to.
    pub(crate) from: usize,
    /// Where it actually put the caret. May differ from `from`: `move_to_byte` snaps.
    pub(crate) from_state: SelState,
    pub(crate) outcome: Outcome,
}

/// Applying one op once from every char boundary in the text.
///
/// This is the operation's function graph, and it is what a single trajectory cannot see: a
/// trajectory from the end of the text is blind to a step that fails in the middle of it.
///
/// # Affinity is not swept
///
/// The public driver API has no way to place a caret at a given index with a chosen affinity —
/// `move_to_byte` delegates to `cursor_at`, which picks one. So each index appears once, with
/// whatever affinity the editor chose, recorded in `from_state`. That is the state a real editor
/// can actually reach through this API; positions reachable only with the other affinity are out
/// of scope for a public-API measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepTable {
    pub(crate) op: Op,
    pub(crate) rows: Vec<StepRow>,
}

impl StepTable {
    pub(crate) fn measure(env: &mut TraversalEnv, text: &str, op: Op) -> Self {
        let mut rows = Vec::new();
        for from in char_boundaries(text) {
            rows.push(step_once(env, text, op, from));
        }
        Self { op, rows }
    }
}

/// Every byte offset a caret may legally occupy, ascending.
pub(crate) fn char_boundaries(text: &str) -> Vec<usize> {
    (0..=text.len())
        .filter(|i| text.is_char_boundary(*i))
        .collect()
}

/// Positions a caret at `from`, applies `op` once, and records what happened.
fn step_once(env: &mut TraversalEnv, text: &str, op: Op, from: usize) -> StepRow {
    let mut editor = env.editor(text);
    {
        let mut drv = env.driver(&mut editor);
        drv.move_to_byte(from);
    }
    let from_state = SelState::of(&editor);
    let before = editor.raw_text().to_string();

    let result = with_panics_silenced(|| {
        let mut ed = env.editor(text);
        {
            let mut drv = env.driver(&mut ed);
            drv.move_to_byte(from);
            op.apply(&mut drv);
        }
        let to = SelState::of(&ed);
        let after = ed.raw_text().to_string();
        (to, after)
    });

    let outcome = match result {
        Err(msg) => Outcome::Panicked(msg),
        Ok((to, after)) => {
            if op.is_edit() {
                let (at, removed) = diff_removal(&before, &after);
                Outcome::Edit {
                    to,
                    removed: render_codepoints(&removed),
                    removed_at: at,
                }
            } else {
                let delta = if to.focus != from_state.focus || to.anchor != from_state.anchor {
                    CursorDelta::Moved
                } else if to.affinity != from_state.affinity {
                    CursorDelta::AffinityOnlyFlip
                } else {
                    CursorDelta::Stationary
                };
                Outcome::Motion { to, delta }
            }
        }
    };

    StepRow {
        from,
        from_state,
        outcome,
    }
}

/// Why a trajectory stopped.
///
/// The distinction between [`Self::Converged`] and [`Self::Stuck`] is the whole point.
/// `Cursor::next_visual` returns `*self` when there is nothing to move to, so "the state did not
/// change" is *both* the normal terminator at the end of the text and the signature of the
/// stationary-cursor bug in the middle of it. A loop that just stops on "didn't move" conflates
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Termination {
    /// Reached a fixpoint at a text boundary, or emptied the buffer. Normal.
    Converged,
    /// Stopped changing while still mid-text.
    Stuck {
        at: usize,
    },
    /// Revisited a state. Catches 2-cycles, which a "did it change" check cannot see.
    Cycle,
    /// Exceeded the step limit.
    Diverged,
    Panicked {
        at_step: usize,
        msg: String,
    },
}

/// One press within a trajectory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Step {
    /// The buffer after this press, as `U+XXXX` codepoints — never as text.
    ///
    /// Rendering it as text is how a reader sees a ZWJ family as "one thing" and writes down 1
    /// when the answer is 7.
    pub(crate) buffer: String,
    pub(crate) sel: SelState,
}

/// Repeatedly applying one op until it stops doing anything.
///
/// [`Self::presses`] is the cell that was wrong in PR #693.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Trajectory {
    pub(crate) op: Op,
    pub(crate) start: usize,
    pub(crate) steps: Vec<Step>,
    pub(crate) termination: Termination,
    /// Presses until the trajectory terminated.
    pub(crate) presses: usize,
}

/// Far above the longest corpus entry; a backstop against a runaway loop, not a real bound.
const MAX_STEPS: usize = 256;

impl Trajectory {
    /// Runs `op` from the far side of the text, so the trajectory crosses all of it.
    ///
    /// For **edits** that is a logical end, because `delete`/`backdelete` act on logical
    /// neighbours. For **motion** it is a *visual* extreme, which is not the same thing: in RTL
    /// text logical byte 0 renders at the visual right edge, so starting `move_right` there would
    /// report 0 presses and a spurious "stuck" for every RTL entry — which is exactly what this
    /// harness did before the RTL rows were measured.
    ///
    /// The visual extreme is derived rather than assumed: the visually-leftmost caret position is
    /// the fixpoint of `move_left`, whatever direction the text runs. That needs no knowledge of
    /// the base direction and behaves for LTR, RTL and mixed alike.
    pub(crate) fn measure(env: &mut TraversalEnv, text: &str, op: Op) -> Self {
        let start = match op.opposite() {
            // Motion: walk the other way first to find the far edge.
            Some(back) => Self::fixpoint_of(env, text, back),
            // Edit: logical ends.
            None if op.is_forward_edit() => 0,
            None => text.len(),
        };
        Self::measure_from(env, text, op, start)
    }

    /// Where `op` comes to rest, starting from the logical start of the text.
    fn fixpoint_of(env: &mut TraversalEnv, text: &str, op: Op) -> usize {
        let mut editor = env.editor(text);
        {
            let mut drv = env.driver(&mut editor);
            drv.move_to_byte(0);
        }
        for _ in 0..MAX_STEPS {
            let before = state_key(&editor);
            let panicked = with_panics_silenced(AssertUnwindSafe(|| {
                let mut drv = env.driver(&mut editor);
                op.apply(&mut drv);
            }))
            .is_err();
            if panicked || state_key(&editor) == before {
                break;
            }
        }
        SelState::of(&editor).focus
    }

    pub(crate) fn measure_from(env: &mut TraversalEnv, text: &str, op: Op, start: usize) -> Self {
        // The edge this trajectory is heading for: the fixpoint of the op itself. Used only to
        // tell a normal stop from a mid-text stall.
        let far_edge = match op.opposite() {
            Some(_) => Self::fixpoint_of(env, text, op),
            None => usize::MAX,
        };

        let mut editor = env.editor(text);
        {
            let mut drv = env.driver(&mut editor);
            drv.move_to_byte(start);
        }

        let mut steps = Vec::new();
        let mut visited = vec![state_key(&editor)];
        let termination = loop {
            if steps.len() >= MAX_STEPS {
                break Termination::Diverged;
            }

            let before = state_key(&editor);
            let result = with_panics_silenced(AssertUnwindSafe(|| {
                let mut drv = env.driver(&mut editor);
                op.apply(&mut drv);
            }));
            if let Err(msg) = result {
                break Termination::Panicked {
                    at_step: steps.len(),
                    msg,
                };
            }

            let after = state_key(&editor);
            steps.push(Step {
                buffer: render_codepoints(editor.raw_text()),
                sel: SelState::of(&editor),
            });

            if after == before {
                // Undo the recorded no-op step: it is the terminator, not a press that did work.
                steps.pop();
                let sel = SelState::of(&editor);
                // Whether this is a normal stop or the stationary-cursor bug turns on *where* it
                // stopped. For motion the far edge is visual, so it is derived the same way the
                // start was rather than assumed to be a logical end — assuming otherwise reports
                // every RTL entry as stuck.
                let at_boundary = editor.raw_text().is_empty()
                    || match op.opposite() {
                        Some(_) => sel.focus == far_edge,
                        None if op.is_forward_edit() => sel.focus == editor.raw_text().len(),
                        None => sel.focus == 0,
                    };
                break if at_boundary {
                    Termination::Converged
                } else {
                    Termination::Stuck { at: sel.focus }
                };
            }
            if visited.contains(&after) {
                break Termination::Cycle;
            }
            visited.push(after);

            if editor.raw_text().is_empty() && op.is_edit() {
                break Termination::Converged;
            }
        };

        Self {
            op,
            start,
            presses: steps.len(),
            steps,
            termination,
        }
    }
}

/// The observable state of an editor, for cycle detection.
fn state_key(editor: &PlainEditor<ColorBrush>) -> (String, usize, usize, bool) {
    let sel = SelState::of(editor);
    (
        editor.raw_text().to_string(),
        sel.anchor,
        sel.focus,
        sel.affinity == Affinity::Upstream,
    )
}

/// Finds the contiguous range `after` is missing relative to `before`.
///
/// Returns the byte offset and the removed text. Computed rather than narrated: an operation that
/// claims to delete one thing and deletes another is exactly what this harness exists to catch.
/// Compared char-wise rather than byte-wise, deliberately. A byte-wise common prefix lands inside a
/// character whenever two different ones share a lead byte — `U+0301` and `U+0308` both start
/// `0xCC` — and then slicing panics. The corpus caught this on `e` + acute + diaeresis.
fn diff_removal(before: &str, after: &str) -> (usize, String) {
    if before == after {
        return (0, String::new());
    }
    let prefix: usize = before
        .chars()
        .zip(after.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    // Search the suffix only in what the prefix did not already claim, so the two cannot overlap.
    let suffix: usize = before[prefix..]
        .chars()
        .rev()
        .zip(after[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    let removed = &before[prefix..before.len() - suffix];
    (prefix, removed.to_string())
}

/// Renders text as space-separated `U+XXXX`.
pub(crate) fn render_codepoints(text: &str) -> String {
    if text.is_empty() {
        return String::from("(empty)");
    }
    text.chars()
        .map(|c| format!("U+{:04X}", c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Runs `f`, converting a panic into an error string and keeping it off stderr.
///
/// A panic in one measurement must not abort the run: `parley_tests` is a single `[[test]]`
/// binary, so one unwind would take every other corpus entry with it and we would measure nothing.
/// The message is normalized to just its payload — a file path or line number in a golden would
/// churn on every unrelated refactor.
fn with_panics_silenced<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(f));
    std::panic::set_hook(prev);
    result.map_err(|e| {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            String::from("<non-string panic payload>")
        };
        msg.lines().next().unwrap_or("").to_string()
    })
}
