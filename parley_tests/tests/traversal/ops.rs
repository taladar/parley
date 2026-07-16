// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The traversal operations under measurement, as data.
//!
//! Each [`Op`] is one thing a user does: a key press. Modelling them as an enum with a fixed
//! iteration order keeps golden tables deterministic and makes "which operations did we actually
//! cover" answerable from the table rather than by reading test names.
//!
//! Ops are split by whether they mutate the buffer, because the two need different measurement:
//! a motion op is a pure function of the layout (so the whole function graph can be swept), while
//! an edit op relaunches layout and invalidates every index (so only trajectories make sense).

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use parley::PlainEditorDriver;

use crate::util::ColorBrush;

/// A single user-level operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Op {
    // -- Motion. Pure with respect to the buffer.
    /// Right arrow in LTR: `Selection::next_visual(.., extend = false)`.
    MoveRight,
    /// Left arrow in LTR.
    MoveLeft,
    /// Ctrl+Right.
    MoveWordRight,
    /// Ctrl+Left.
    MoveWordLeft,
    /// Shift+Right.
    SelectRight,
    /// Shift+Left.
    SelectLeft,

    // -- Edits. Mutate the buffer and relaunch layout.
    /// Forward delete.
    Delete,
    /// Backspace. The contested one.
    Backdelete,
    /// Ctrl+Delete.
    DeleteWord,
    /// Ctrl+Backspace.
    BackdeleteWord,
}

impl Op {
    /// Fixed order. Golden tables iterate this; changing it rewrites every table.
    pub(crate) const ALL: &'static [Self] = &[
        Self::MoveRight,
        Self::MoveLeft,
        Self::MoveWordRight,
        Self::MoveWordLeft,
        Self::SelectRight,
        Self::SelectLeft,
        Self::Delete,
        Self::Backdelete,
        Self::DeleteWord,
        Self::BackdeleteWord,
    ];

    pub(crate) const MOTION: &'static [Self] = &[
        Self::MoveRight,
        Self::MoveLeft,
        Self::MoveWordRight,
        Self::MoveWordLeft,
        Self::SelectRight,
        Self::SelectLeft,
    ];

    pub(crate) const EDITS: &'static [Self] = &[
        Self::Delete,
        Self::Backdelete,
        Self::DeleteWord,
        Self::BackdeleteWord,
    ];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::MoveRight => "move_right",
            Self::MoveLeft => "move_left",
            Self::MoveWordRight => "move_word_right",
            Self::MoveWordLeft => "move_word_left",
            Self::SelectRight => "select_right",
            Self::SelectLeft => "select_left",
            Self::Delete => "delete",
            Self::Backdelete => "backdelete",
            Self::DeleteWord => "delete_word",
            Self::BackdeleteWord => "backdelete_word",
        }
    }

    pub(crate) fn is_edit(self) -> bool {
        Self::EDITS.contains(&self)
    }

    /// Whether an **edit** travels toward the end of the buffer.
    ///
    /// Only meaningful for edits, which act on logical neighbours: `delete` takes the downstream
    /// cluster and `backdelete` the upstream one, so their trajectories start at the logical start
    /// and end of the text respectively.
    ///
    /// Deliberately **not** defined for motion. Motion is *visual*, and conflating the two is a
    /// mistake this harness made and had to measure its way out of: in RTL text logical byte 0
    /// renders at the visual right edge, so a `move_right` trajectory started at byte 0 is
    /// immediately stuck and reports 0 presses for every RTL entry. See [`Self::opposite`].
    pub(crate) fn is_forward_edit(self) -> bool {
        debug_assert!(self.is_edit(), "only edits have a logical direction");
        matches!(self, Self::Delete | Self::DeleteWord)
    }

    /// The motion that travels the other way.
    ///
    /// Used to find where a motion trajectory should start: the visually-leftmost caret position
    /// is exactly the fixpoint of `move_left`, whichever direction the text runs. Deriving the
    /// start this way needs no knowledge of the base direction and works for LTR, RTL and mixed
    /// text alike.
    pub(crate) fn opposite(self) -> Option<Self> {
        Some(match self {
            Self::MoveRight => Self::MoveLeft,
            Self::MoveLeft => Self::MoveRight,
            Self::MoveWordRight => Self::MoveWordLeft,
            Self::MoveWordLeft => Self::MoveWordRight,
            Self::SelectRight => Self::MoveLeft,
            Self::SelectLeft => Self::MoveRight,
            _ => return None,
        })
    }

    /// Applies the operation once.
    pub(crate) fn apply(self, drv: &mut PlainEditorDriver<'_, ColorBrush>) {
        match self {
            Self::MoveRight => drv.move_right(),
            Self::MoveLeft => drv.move_left(),
            Self::MoveWordRight => drv.move_word_right(),
            Self::MoveWordLeft => drv.move_word_left(),
            Self::SelectRight => drv.select_right(),
            Self::SelectLeft => drv.select_left(),
            Self::Delete => drv.delete(),
            Self::Backdelete => drv.backdelete(),
            Self::DeleteWord => drv.delete_word(),
            Self::BackdeleteWord => drv.backdelete_word(),
        }
    }
}
