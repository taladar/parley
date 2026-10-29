// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Joining separately laid-out paragraphs into one layout.
//!
//! A hard line break ends a paragraph: nothing on one side of a `'\n'` changes
//! how the other side is shaped or broken (as long as no bidi resolution
//! spans it, which the caller rules out). So a text can be laid out a
//! paragraph at a time, each paragraph's layout kept, and the whole assembled
//! from them. This is what lets an editor re-shape only the paragraph an edit
//! touched instead of the whole buffer.
//!
//! The assembly copies each paragraph's runs, clusters, glyphs, lines and items
//! into one [`LayoutData`], shifting every index and text offset by where the
//! paragraph starts, de-duplicating fonts and styles, and placing every line
//! vertically again with [`place_line`] — the same arithmetic line breaking
//! uses, so a joined layout puts each line where breaking the whole text would.

use alloc::vec::Vec;
use core::ops::Range;

use super::line_break::place_line;
use super::{Layout, LayoutData, LayoutItemKind};
use crate::style::Brush;

/// One paragraph to join: its finished (broken, not aligned) layout, and the
/// byte offset its text starts at in the whole.
pub(crate) struct Paragraph<'a, B: Brush> {
    /// Where the paragraph's text starts in the whole text.
    pub(crate) start: usize,
    /// The paragraph's own layout, laid out from its own text.
    pub(crate) layout: &'a Layout<B>,
}

/// Shift a range by `by`.
fn shift(range: &Range<usize>, by: usize) -> Range<usize> {
    range.start + by..range.end + by
}

/// Shift a line's text range by `by` — unless it is the sentinel line breaking
/// leaves on a line no text run sets (one holding only an inline box), whose
/// start is `usize::MAX`: that is kept exactly as breaking the whole text
/// would have left it.
fn shift_line_text(range: &Range<usize>, by: usize) -> Range<usize> {
    if range.start == usize::MAX {
        range.clone()
    } else {
        shift(range, by)
    }
}

/// Join `paragraphs` (in text order) into `target`, which is cleared first.
///
/// Every paragraph but the last ends with its `'\n'`, and was laid out alone,
/// so its layout ends with the empty line a trailing newline opens; that line
/// is not part of the whole (the next paragraph's first line is) and is
/// dropped. The last paragraph keeps its own, which is the whole text's.
///
/// `width` is the maximum advance the paragraphs were broken at. The caller
/// aligns the result.
pub(crate) fn concatenate<B: Brush>(
    target: &mut Layout<B>,
    paragraphs: &[Paragraph<'_, B>],
    text_len: usize,
    width: Option<f32>,
) {
    let out = &mut target.data;
    out.clear();
    let Some(first) = paragraphs.first() else {
        return;
    };
    out.scale = first.layout.data.scale;
    out.quantize = first.layout.data.quantize;
    out.base_level = first.layout.data.base_level;
    out.text_len = text_len;
    out.indent_amount = first.layout.data.indent_amount;
    out.indent_options = first.layout.data.indent_options;

    let last = paragraphs.len() - 1;
    let mut fonts: Vec<usize> = Vec::new();
    let mut styles: Vec<u16> = Vec::new();
    let mut previous_fonts = None;
    let mut previous_styles = None;
    for (number, paragraph) in paragraphs.iter().enumerate() {
        let data: &LayoutData<B> = &paragraph.layout.data;
        let start = paragraph.start;

        // Neighbouring paragraphs nearly always carry the same fonts and styles,
        // so the last paragraph's mapping into the joined tables is kept and
        // reused when its tables are equal — one slice comparison instead of a
        // search per entry.
        if previous_fonts != Some(&data.fonts[..]) {
            fonts = data
                .fonts
                .iter()
                .map(|font| {
                    out.fonts
                        .iter()
                        .position(|known| known == font)
                        .unwrap_or_else(|| {
                            out.fonts.push(font.clone());
                            out.fonts.len() - 1
                        })
                })
                .collect();
            previous_fonts = Some(&data.fonts[..]);
        }
        if previous_styles != Some(&data.styles[..]) {
            styles = data
                .styles
                .iter()
                .map(|style| {
                    let index = out
                        .styles
                        .iter()
                        .position(|known| known == style)
                        .unwrap_or_else(|| {
                            out.styles.push(style.clone());
                            out.styles.len() - 1
                        });
                    u16::try_from(index).expect("more distinct styles than a cluster can index")
                })
                .collect();
            previous_styles = Some(&data.styles[..]);
        }

        let run_base = out.runs.len();
        let box_base = out.inline_boxes.len();
        let cluster_base = out.clusters.len();
        let glyph_base = out.glyphs.len();
        let coord_base = out.coords.len();

        out.coords.extend_from_slice(&data.coords);
        out.glyphs.extend_from_slice(&data.glyphs);
        out.clusters.extend(data.clusters.iter().map(|cluster| {
            let mut cluster = *cluster;
            cluster.style_index = styles[usize::from(cluster.style_index)];
            cluster
        }));
        out.runs.extend(data.runs.iter().map(|run| {
            let mut run = run.clone();
            run.font_index = fonts[run.font_index];
            run.text_range = shift(&run.text_range, start);
            run.cluster_range = shift(&run.cluster_range, cluster_base);
            run.coords_range = shift(&run.coords_range, coord_base);
            run.glyph_start += glyph_base;
            run
        }));
        out.inline_boxes
            .extend(data.inline_boxes.iter().map(|inline_box| {
                let mut inline_box = inline_box.clone();
                inline_box.index += start;
                inline_box
            }));
        let item_base = |kind: LayoutItemKind| match kind {
            LayoutItemKind::TextRun => run_base,
            LayoutItemKind::InlineBox => box_base,
        };
        out.items.extend(data.items.iter().map(|item| {
            let mut item = item.clone();
            item.index += item_base(item.kind);
            item
        }));

        // The empty line a paragraph's own trailing newline opened.
        let lines = if number < last {
            match data.lines.split_last() {
                Some((line, rest))
                    if line.text_range.is_empty() && line.text_range.start == data.text_len =>
                {
                    rest
                }
                _ => &data.lines[..],
            }
        } else {
            &data.lines[..]
        };
        for line in lines {
            let mut line = line.clone();
            let item_start = out.line_items.len();
            out.line_items
                .extend(data.line_items[line.item_range.clone()].iter().map(|item| {
                    let mut item = item.clone();
                    item.index += item_base(item.kind);
                    // A box's text and cluster ranges are not read; a run's are.
                    if item.kind == LayoutItemKind::TextRun {
                        item.text_range = shift(&item.text_range, start);
                        item.cluster_range = shift(&item.cluster_range, cluster_base);
                    }
                    item
                }));
            line.item_range = item_start..out.line_items.len();
            line.text_range = shift_line_text(&line.text_range, start);
            out.lines.push(line);
        }
    }

    // Stack the lines, and measure the whole as line breaking does when it
    // finishes (`BreakLines`'s `Drop`).
    let mut y = 0_f64;
    let mut layout_width = 0_f32;
    let mut layout_full_width = 0_f32;
    for line in &mut out.lines {
        place_line(&mut line.metrics, y, out.quantize);
        y += f64::from(line.metrics.line_height);
        let line_max = line.metrics.inline_min_coord + line.metrics.advance + line.indent.max(0.0);
        layout_full_width = layout_full_width.max(line_max);
        layout_width = layout_width.max(line_max - line.metrics.trailing_whitespace);
    }
    let mut height = y;
    if width.is_none() {
        // Unbounded: every line's box is the longest line, which each
        // paragraph could only measure against itself.
        for line in &mut out.lines {
            line.metrics.inline_max_coord = layout_width;
        }
    }
    // As breaking does: a last line with no items at all takes no height.
    if let Some(last_line) = out.lines.last() {
        if last_line.item_range.is_empty() {
            height -= f64::from(last_line.metrics.line_height);
        }
    }
    out.width = layout_width;
    out.full_width = layout_full_width;
    out.height = height as f32;
    out.layout_max_advance = width.unwrap_or(f32::MAX);
}
