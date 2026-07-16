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

use parley::{Cluster, Layout};

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
