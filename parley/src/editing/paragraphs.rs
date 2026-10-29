// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A [`PlainEditor`](super::PlainEditor)'s layout, rebuilt a paragraph at a
//! time.
//!
//! Laying a whole buffer out again on every keystroke costs time in proportion
//! to the buffer — about 20 ms for a 64 KB source file — while a keystroke
//! changes one paragraph. So the editor keeps each paragraph's finished layout,
//! keyed by everything that went into it (its text, the range styles and inline
//! boxes that fall in it, rebased to it, and the preedit), and on each relayout
//! reuses every paragraph whose key is unchanged, matching from both ends of the
//! buffer, before joining them into the one layout the caret and selection work
//! on ([`crate::layout::concat`]).
//!
//! Text that needs bidi resolution falls back to laying the whole buffer out:
//! parley resolves bidi over the whole text with one base level, which a
//! paragraph laid out alone cannot reproduce. Plain ASCII never needs it, so the
//! check is a byte scan for anything else.

use alloc::string::String;
use alloc::vec::Vec;
use core::ops::Range;

use crate::layout::concat::{Paragraph, concatenate};
use crate::style::Brush;
use crate::{
    Alignment, AlignmentOptions, FontContext, InlineBox, Layout, LayoutContext, StyleProperty,
    StyleSet,
};

/// What every paragraph's layout depends on beyond its own content. A change in
/// any of it lays every paragraph out again.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParagraphSettings {
    /// The maximum advance lines are broken at.
    pub(crate) width: Option<f32>,
    /// The layout scale.
    pub(crate) scale: f32,
    /// Whether metrics are quantized.
    pub(crate) quantize: bool,
    /// Bumped whenever the default styles may have changed.
    pub(crate) style_generation: u64,
}

/// Everything that went into one paragraph's layout, rebased to it.
#[derive(Clone, Debug, PartialEq)]
struct ParagraphKey<B: Brush> {
    /// The paragraph's text, with its newline.
    text: String,
    /// The range styles that fall in it, clipped and rebased, in push order.
    styles: Vec<(Range<usize>, StyleProperty<'static, B>)>,
    /// The inline boxes anchored in it, rebased.
    boxes: Vec<InlineBox>,
    /// The part of the preedit in it, rebased.
    compose: Option<Range<usize>>,
}

/// One paragraph's kept layout.
#[derive(Clone)]
struct CachedParagraph<B: Brush> {
    /// What it was laid out from.
    key: ParagraphKey<B>,
    /// Its broken, unaligned layout.
    layout: Layout<B>,
}

/// The kept paragraph layouts of one editor.
#[derive(Clone, Default)]
pub(crate) struct ParagraphCache<B: Brush> {
    /// What the kept layouts were made under.
    settings: Option<ParagraphSettings>,
    /// The kept layouts, in text order.
    paragraphs: Vec<CachedParagraph<B>>,
}

/// Whether `text` needs bidi resolution anywhere. Exact for ASCII, which never
/// does; anything else is looked up.
fn needs_bidi<B: Brush>(text: &str, layout_cx: &LayoutContext<B>) -> bool {
    text.chars().any(|c| {
        !c.is_ascii()
            && crate::bidi::needs_bidi_resolution(
                layout_cx.analysis_data_sources.properties(c).bidi_class(),
            )
    })
}

/// The inputs to one relayout of an editor.
pub(crate) struct Inputs<'a, B: Brush> {
    /// The buffer.
    pub(crate) text: &'a str,
    /// The editor's default styles.
    pub(crate) defaults: &'a StyleSet<B>,
    /// The range styles over the whole buffer.
    pub(crate) range_styles: &'a [(Range<usize>, StyleProperty<'static, B>)],
    /// The inline boxes over the whole buffer.
    pub(crate) inline_boxes: &'a [InlineBox],
    /// The preedit range, if composing.
    pub(crate) compose: Option<Range<usize>>,
    /// The settings the layout is made under.
    pub(crate) settings: ParagraphSettings,
    /// The alignment applied to the joined layout.
    pub(crate) alignment: Alignment,
}

impl<B: Brush> ParagraphCache<B> {
    /// Lay `inputs` out into `layout` a paragraph at a time, reusing what is
    /// unchanged; `false` (with nothing written) when the text has to be laid
    /// out whole instead.
    pub(crate) fn relayout(
        &mut self,
        layout: &mut Layout<B>,
        inputs: &Inputs<'_, B>,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<B>,
    ) -> bool {
        let text = inputs.text;
        // One paragraph is the whole text anyway; justification moves clusters
        // the cached layouts would then carry; and bidi spans paragraphs.
        if !text.contains('\n')
            || inputs.alignment == Alignment::Justify
            || needs_bidi(text, layout_cx)
        {
            self.paragraphs.clear();
            self.settings = None;
            return false;
        }
        if self.settings.as_ref() != Some(&inputs.settings) {
            self.paragraphs.clear();
            self.settings = Some(inputs.settings.clone());
        }

        let keys = paragraph_keys(inputs);
        let old = core::mem::take(&mut self.paragraphs);
        let prefix = old
            .iter()
            .zip(&keys)
            .take_while(|(cached, (_start, key))| cached.key == *key)
            .count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(keys[prefix..].iter().rev())
            .take_while(|(cached, (_start, key))| cached.key == *key)
            .count();

        let mut old = old.into_iter();
        let mut kept: Vec<CachedParagraph<B>> = Vec::with_capacity(keys.len());
        kept.extend(old.by_ref().take(prefix));
        let changed = keys.len() - prefix - suffix;
        for (_start, key) in &keys[prefix..prefix + changed] {
            let layout = lay_out_paragraph(key, inputs, font_cx, layout_cx);
            kept.push(CachedParagraph {
                key: key.clone(),
                layout,
            });
        }
        let reused_tail = old.len() - suffix;
        kept.extend(old.skip(reused_tail));

        let parts: Vec<Paragraph<'_, B>> = kept
            .iter()
            .zip(&keys)
            .map(|(cached, (start, _key))| Paragraph {
                start: *start,
                layout: &cached.layout,
            })
            .collect();
        concatenate(layout, &parts, text.len(), inputs.settings.width);
        layout.align(inputs.alignment, AlignmentOptions::default());
        self.paragraphs = kept;
        true
    }
}

/// Every paragraph of `inputs.text` — each ends with its `'\n'`, but for the
/// last — with where it starts and its key.
fn paragraph_keys<B: Brush>(inputs: &Inputs<'_, B>) -> Vec<(usize, ParagraphKey<B>)> {
    let text = inputs.text;
    let mut keys: Vec<(usize, ParagraphKey<B>)> = Vec::new();
    let mut start = 0;
    for paragraph in text.split_inclusive('\n') {
        keys.push((
            start,
            ParagraphKey {
                text: String::from(paragraph),
                styles: Vec::new(),
                boxes: Vec::new(),
                compose: None,
            },
        ));
        start += paragraph.len();
    }
    let starts: Vec<usize> = keys.iter().map(|(start, _key)| *start).collect();
    for (range, property) in inputs.range_styles {
        if !is_valid_range(text, range) {
            continue;
        }
        clip_to_paragraphs(&starts, text.len(), range, |number, clipped| {
            if let Some((_start, key)) = keys.get_mut(number) {
                key.styles.push((clipped, property.clone()));
            }
        });
    }
    if let Some(compose) = &inputs.compose {
        clip_to_paragraphs(&starts, text.len(), compose, |number, clipped| {
            if let Some((_start, key)) = keys.get_mut(number) {
                key.compose = Some(clipped);
            }
        });
    }
    for inline_box in inputs.inline_boxes {
        if !text.is_char_boundary(inline_box.index) {
            continue;
        }
        let number = paragraph_at(&starts, inline_box.index);
        if let Some((start, key)) = keys.get_mut(number) {
            let mut inline_box = inline_box.clone();
            inline_box.index -= *start;
            key.boxes.push(inline_box);
        }
    }
    keys
}

/// The paragraph byte `index` falls in: the last one starting at or before it.
/// The end of the text falls in the last paragraph.
fn paragraph_at(starts: &[usize], index: usize) -> usize {
    starts.partition_point(|start| *start <= index).max(1) - 1
}

/// Hand `piece` each part of `range` in a paragraph it touches, rebased to the
/// paragraph: `(paragraph number, range within it)`. An empty range has none.
fn clip_to_paragraphs(
    starts: &[usize],
    text_len: usize,
    range: &Range<usize>,
    mut piece: impl FnMut(usize, Range<usize>),
) {
    if range.start >= range.end {
        return;
    }
    let mut number = paragraph_at(starts, range.start);
    while let Some(&start) = starts.get(number) {
        if start >= range.end {
            break;
        }
        let end = starts.get(number + 1).copied().unwrap_or(text_len);
        let clipped = range.start.max(start) - start..range.end.min(end) - start;
        if !clipped.is_empty() {
            piece(number, clipped);
        }
        number += 1;
    }
}

/// Whether `range` is a valid byte range of `text`.
fn is_valid_range(text: &str, range: &Range<usize>) -> bool {
    text.is_char_boundary(range.start) && text.is_char_boundary(range.end)
}

/// Lay one paragraph out from its key, broken but not aligned — what the editor
/// would do for the whole buffer, for this paragraph alone.
fn lay_out_paragraph<B: Brush>(
    key: &ParagraphKey<B>,
    inputs: &Inputs<'_, B>,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<B>,
) -> Layout<B> {
    let mut builder = layout_cx.ranged_builder(
        font_cx,
        &key.text,
        inputs.settings.scale,
        inputs.settings.quantize,
    );
    for prop in inputs.defaults.inner().values() {
        builder.push_default(prop.clone());
    }
    for (range, property) in &key.styles {
        builder.push(property.clone(), range.clone());
    }
    if let Some(compose) = &key.compose {
        builder.push(StyleProperty::Underline(true), compose.clone());
    }
    for inline_box in &key.boxes {
        builder.push_inline_box(inline_box.clone());
    }
    let mut layout = builder.build(&key.text);
    layout.break_all_lines(inputs.settings.width);
    layout
}
