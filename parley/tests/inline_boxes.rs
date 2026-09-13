// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! `PlainEditor`'s inline boxes and per-range styles.
//!
//! These exercise the two additions a *rich* editor needs from the plain one: a
//! box laid out in the flow of the text, and a style property scoped to a byte
//! range of the buffer.

use parley::{
    FontContext, InlineBox, InlineBoxKind, LayoutContext, PlainEditor, PositionedLayoutItem,
    StyleProperty,
};

/// A brush that is just a unit — the editor only needs `Brush`.
type Brush = peniko::Brush;

/// The width every probe box reserves.
const BOX_WIDTH: f32 = 40.0;

/// The height every probe box reserves.
const BOX_HEIGHT: f32 = 16.0;

/// An editor over `text`, laid out unconstrained.
fn editor(text: &str) -> (PlainEditor<Brush>, FontContext, LayoutContext<Brush>) {
    let mut editor = PlainEditor::<Brush>::new(16.0);
    editor.set_text(text);
    editor.set_width(None);
    (editor, FontContext::new(), LayoutContext::new())
}

/// The inline boxes the layout positioned, as `(id, x)` pairs in visual order.
fn positioned_boxes(
    editor: &mut PlainEditor<Brush>,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<Brush>,
) -> Vec<(u64, f32)> {
    let layout = editor.layout(font_cx, layout_cx);
    let mut found = Vec::new();
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::InlineBox(inline_box) = item {
                found.push((inline_box.id, inline_box.x));
            }
        }
    }
    found
}

/// A box anchored at a byte offset is laid out in the flow, in text order.
#[test]
fn inline_boxes_are_laid_out_in_the_flow() {
    let (mut editor, mut font_cx, mut layout_cx) = editor("ab cd");
    editor.set_inline_boxes(vec![
        InlineBox {
            id: 7,
            kind: InlineBoxKind::InFlow,
            index: 3,
            width: BOX_WIDTH,
            height: BOX_HEIGHT,
        },
        InlineBox {
            id: 9,
            kind: InlineBoxKind::InFlow,
            index: 5,
            width: BOX_WIDTH,
            height: BOX_HEIGHT,
        },
    ]);
    let boxes = positioned_boxes(&mut editor, &mut font_cx, &mut layout_cx);
    assert_eq!(boxes.len(), 2, "both boxes should be positioned: {boxes:?}");
    assert_eq!(boxes[0].0, 7);
    assert_eq!(boxes[1].0, 9);
    assert!(
        boxes[0].1 < boxes[1].1,
        "the earlier box should sit left of the later one: {boxes:?}"
    );
    assert!(
        boxes[1].1 - boxes[0].1 >= BOX_WIDTH,
        "the first box should reserve its own width: {boxes:?}"
    );
}

/// A box whose index is past the end of the buffer, or inside a multi-byte
/// character, is dropped rather than panicking the layout.
#[test]
fn an_out_of_bounds_box_is_ignored() {
    let (mut editor, mut font_cx, mut layout_cx) = editor("é");
    editor.set_inline_boxes(vec![
        InlineBox {
            id: 1,
            kind: InlineBoxKind::InFlow,
            // Inside the two-byte `é`.
            index: 1,
            width: BOX_WIDTH,
            height: BOX_HEIGHT,
        },
        InlineBox {
            id: 2,
            kind: InlineBoxKind::InFlow,
            index: 99,
            width: BOX_WIDTH,
            height: BOX_HEIGHT,
        },
    ]);
    let boxes = positioned_boxes(&mut editor, &mut font_cx, &mut layout_cx);
    assert!(boxes.is_empty(), "neither box is placeable: {boxes:?}");
}

/// A per-range style applies to its range only: a zero font size collapses the
/// styled characters' advance while the rest of the line keeps its own.
#[test]
fn a_zero_font_size_range_collapses_its_advance() {
    let (mut editor, mut font_cx, mut layout_cx) = editor("abcd");
    let full_width = editor.layout(&mut font_cx, &mut layout_cx).full_width();
    editor.set_range_styles(vec![(2..4, StyleProperty::FontSize(0.0))]);
    let hidden_width = editor.layout(&mut font_cx, &mut layout_cx).full_width();
    assert!(
        hidden_width < full_width,
        "hiding two of four characters should narrow the line: {hidden_width} vs {full_width}"
    );
    assert!(
        hidden_width > 0.0,
        "the unstyled characters should still be laid out"
    );
}

/// Setting the same boxes or styles twice does not invalidate the layout, so a
/// caller can hand its model over unconditionally each frame.
#[test]
fn setting_the_same_model_twice_keeps_the_layout() {
    let (mut editor, mut font_cx, mut layout_cx) = editor("abcd");
    let boxes = vec![InlineBox {
        id: 1,
        kind: InlineBoxKind::InFlow,
        index: 2,
        width: BOX_WIDTH,
        height: BOX_HEIGHT,
    }];
    editor.set_inline_boxes(boxes.clone());
    editor.set_range_styles(vec![(0..1, StyleProperty::FontSize(0.0))]);
    editor.refresh_layout(&mut font_cx, &mut layout_cx);
    let generation = editor.generation();
    editor.set_inline_boxes(boxes);
    editor.set_range_styles(vec![(0..1, StyleProperty::FontSize(0.0))]);
    editor.refresh_layout(&mut font_cx, &mut layout_cx);
    assert!(
        generation == editor.generation(),
        "an unchanged model should not relayout"
    );
}
