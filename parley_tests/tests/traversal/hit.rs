// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hit testing: what a click at a point resolves to.
//!
//! The other measurement modules sweep the *keyboard* — press a key, see where the caret goes.
//! This one sweeps the *pointer*, which is a different reachability question and can reach states
//! the keyboard cannot. `Cursor::from_point` is the whole click path: `PlainEditor::move_to_point`
//! and `Selection::from_point` both delegate to it.
//!
//! # Probe points are derived, never absolute
//!
//! `measure.rs` bans floats from recorded data because an advance depends on the font, the
//! rasteriser and the platform. That ban is on **outputs**; probe points are inputs and may be
//! floats. But an absolute grid (`x = 0.0, 0.01, 0.02, …`) would smuggle the float dependence back
//! into the golden: the same structural layout on a platform with slightly different advances would
//! be probed at different *relative* positions and could produce a different reachable set.
//!
//! So every probe is expressed as a fraction of a measured cluster advance — [`FRACTIONS`] — plus
//! epsilons either side. That makes the sweep scale-free: it asks "the left quarter of this
//! cluster", not "x = 2.18". The recorded answer then depends on the layout's structure alone.
//!
//! # What the golden may record
//!
//! Discrete facts only:
//!
//! - **which** byte offsets a click can produce, and with which affinity;
//! - `zero_advance` as a `bool`, exactly as [`super::measure::ClusterRow`] does;
//! - `equal_slices`: whether the clusters covering one shaped glyph all report the *same* advance.
//!   A relation between two floats produced by one division in one run, not a float value — it
//!   cannot flap the way recording `4.43` would.
//!
//! Never an advance, a width, or a coordinate.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use std::collections::BTreeSet;
use std::ops::Range;

use parley::{Affinity, Cluster, Cursor, Layout};

use super::env::TraversalEnv;
use super::reference::{Ink, ink};
use crate::util::ColorBrush;

/// Where within a cluster to probe, as a fraction of its advance.
///
/// Includes both sides of the midpoint because the midpoint is where `from_point_impl` chooses
/// between `ClusterSide::Left` and `Right`, and both exact ends because that is where one cluster
/// stops claiming a coordinate and the next starts.
const FRACTIONS: &[f32] = &[0.0, 0.01, 0.25, 0.49, 0.5, 0.51, 0.75, 0.99, 1.0];

/// Offsets applied either side of every probe, in font-size-relative units.
///
/// The layout is built at font size 16, so these are far below one pixel yet far above `f32` noise
/// at these magnitudes.
const EPSILONS: &[f32] = &[-0.01, -0.0001, 0.0, 0.0001, 0.01];

/// One cluster, as the hit-test tables record it. No floats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HitCluster {
    pub(crate) range: Range<usize>,
    /// What Unicode says this cluster's codepoint renders as on its own.
    pub(crate) ink: Ink,
    /// Whether parley gave it exactly zero width. A zero-advance cluster cannot be clicked *into*.
    pub(crate) zero_advance: bool,
    /// `S` if it starts a ligature, `C` if it continues one, `-` otherwise.
    pub(crate) lig: &'static str,
    pub(crate) glyphs: usize,
}

/// A caret position a click produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct HitPos {
    pub(crate) index: usize,
    /// `D` or `U`. A `&'static str` rather than `Affinity` so the set orders deterministically.
    pub(crate) affinity: &'static str,
}

/// Everything the hit-test golden records about one text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HitTable {
    pub(crate) clusters: Vec<HitCluster>,
    /// Every `(index, affinity)` some click produced.
    pub(crate) reachable: BTreeSet<HitPos>,
    /// Whether every maximal ligature group has all-equal advances among its clusters.
    ///
    /// `None` when the layout contains no ligature group at all.
    pub(crate) equal_slices: Option<bool>,
}

impl HitTable {
    pub(crate) fn measure(env: &mut TraversalEnv, text: &str) -> Self {
        let layout = env.layout(text);
        Self {
            clusters: clusters(&layout, text),
            reachable: reachable(&layout),
            equal_slices: equal_slices(&layout),
        }
    }

    /// The byte offsets a click can produce, ignoring affinity.
    pub(crate) fn reachable_indices(&self) -> BTreeSet<usize> {
        self.reachable.iter().map(|p| p.index).collect()
    }

    /// Every cluster boundary: the caret positions that exist at all.
    pub(crate) fn boundaries(&self) -> BTreeSet<usize> {
        self.clusters
            .iter()
            .flat_map(|c| [c.range.start, c.range.end])
            .collect()
    }
}

fn clusters(layout: &Layout<ColorBrush>, text: &str) -> Vec<HitCluster> {
    let mut v = Vec::new();
    let mut cursor = Cluster::from_byte_index(layout, 0);
    while let Some(cl) = cursor {
        let range = cl.text_range();
        let first = text[range.clone()]
            .chars()
            .next()
            .expect("a cluster covers at least one char");
        v.push(HitCluster {
            range: range.clone(),
            ink: ink(first),
            zero_advance: cl.advance() == 0.0,
            lig: match (cl.is_ligature_start(), cl.is_ligature_continuation()) {
                (true, _) => "S",
                (_, true) => "C",
                _ => "-",
            },
            glyphs: cl.glyphs().count(),
        });
        cursor = cl.next_logical();
    }
    v
}

/// Advance of every cluster, in logical order. Internal: never recorded.
fn advances(layout: &Layout<ColorBrush>) -> Vec<f32> {
    let mut v = Vec::new();
    let mut cursor = Cluster::from_byte_index(layout, 0);
    while let Some(cl) = cursor {
        v.push(cl.advance());
        cursor = cl.next_logical();
    }
    v
}

/// Whether the clusters of each maximal ligature group all report the same advance.
///
/// A group is a `is_ligature_start` cluster plus the `is_ligature_continuation` clusters after it —
/// the N codepoints that shaped into one glyph. This is the mechanism behind mid-grapheme click
/// positions, reduced to a `bool` so no float reaches the golden.
fn equal_slices(layout: &Layout<ColorBrush>) -> Option<bool> {
    let advs = advances(layout);
    let mut groups: Vec<Vec<f32>> = Vec::new();
    let mut cursor = Cluster::from_byte_index(layout, 0);
    let mut i = 0;
    while let Some(cl) = cursor {
        if cl.is_ligature_start() {
            groups.push(vec![advs[i]]);
        } else if let Some(g) = groups.last_mut()
            && cl.is_ligature_continuation()
        {
            g.push(advs[i]);
        }
        i += 1;
        cursor = cl.next_logical();
    }
    // A "group" of one is a ligature start whose continuations went elsewhere; not informative.
    groups.retain(|g| g.len() > 1);
    if groups.is_empty() {
        return None;
    }
    Some(groups.iter().all(|g| g.iter().all(|a| *a == g[0])))
}

/// Every `(index, affinity)` any click anywhere on this layout can produce.
///
/// Sweeps each line at every [`FRACTIONS`] point of every cluster on it, plus [`EPSILONS`] either
/// side, plus well outside both horizontal edges. Probes are derived from measured advances rather
/// than an absolute grid — see the module docs.
fn reachable(layout: &Layout<ColorBrush>) -> BTreeSet<HitPos> {
    let mut out = BTreeSet::new();

    for line_index in 0..layout.len() {
        let line = layout.get(line_index).unwrap();
        let metrics = line.metrics();
        let y = metrics.baseline - metrics.ascent * 0.5;

        // Edges of every cluster on this line, accumulated in visual order.
        let mut edges = vec![metrics.offset];
        let mut x = metrics.offset;
        for run in line.runs() {
            for cl in run.visual_clusters() {
                x += cl.advance();
                edges.push(x);
            }
        }

        let mut xs: Vec<f32> = Vec::new();
        for pair in edges.windows(2) {
            let (edge, next) = (pair[0], pair[1]);
            let advance = next - edge;
            for f in FRACTIONS {
                for e in EPSILONS {
                    xs.push(edge + advance * f + e);
                }
            }
        }
        // Outside the line on both sides: clicking in the margin is an ordinary thing to do.
        let last = *edges.last().unwrap_or(&metrics.offset);
        for e in EPSILONS {
            xs.push(metrics.offset - 1000.0 + e);
            xs.push(metrics.offset - 8.0 + e);
            xs.push(last + 8.0 + e);
            xs.push(last + 1000.0 + e);
        }

        for x in xs {
            let cur = Cursor::from_point(layout, x, y);
            out.insert(HitPos {
                index: cur.index(),
                affinity: match cur.affinity() {
                    Affinity::Downstream => "D",
                    Affinity::Upstream => "U",
                },
            });
        }
    }

    out
}
