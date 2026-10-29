// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A `PlainEditor` laid out a paragraph at a time (`editing::paragraphs`) must be
//! indistinguishable from one laid out whole: the same lines at the same
//! heights, every glyph at the same place in the same style, every inline box
//! where it was, and the caret at every offset in the same rectangle — after
//! the first layout and after every edit that reuses the paragraphs it can.

use std::borrow::{Cow, ToOwned};
use std::ops::Range;
use std::vec;
use std::vec::Vec;

use peniko::color::palette;

use super::test_builders::create_font_context;
use super::utils::ColorBrush;
use crate::layout::PositionedLayoutItem;
use crate::{
    Affinity, Cursor, FontContext, FontFamily, FontWeight, InlineBox, InlineBoxKind, Layout,
    LayoutContext, PlainEditor, StyleProperty,
};

/// What one layout looks like from the outside, to compare two by.
#[derive(Debug, PartialEq)]
struct Observed {
    /// The layout's size.
    size: (f32, f32, f32),
    /// Per line: its text, its metrics, and what is drawn on it.
    lines: Vec<ObservedLine>,
    /// The caret rectangle at every character boundary, both affinities.
    carets: Vec<(usize, [f64; 4], [f64; 4])>,
}

/// One line, from the outside.
#[derive(Debug, PartialEq)]
struct ObservedLine {
    /// The bytes it covers.
    text_range: Range<usize>,
    /// Baseline, block extent, line height, advance, inline extent.
    metrics: [f32; 7],
    /// Every glyph: id, x, y, advance, and the brush and weight-bearing style.
    glyphs: Vec<(u32, f32, f32, f32, ColorBrush, bool)>,
    /// Every inline box: id, x, y.
    boxes: Vec<(u64, f32, f32)>,
}

/// Everything observable about `layout` over `text`.
fn observe(layout: &Layout<ColorBrush>, text: &str) -> Observed {
    let mut lines = Vec::new();
    for line in layout.lines() {
        let metrics = line.metrics();
        let mut glyphs = Vec::new();
        let mut boxes = Vec::new();
        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(run) => {
                    let style = run.style();
                    for glyph in run.positioned_glyphs() {
                        glyphs.push((
                            glyph.id,
                            glyph.x,
                            glyph.y,
                            glyph.advance,
                            style.brush,
                            style.underline.is_some(),
                        ));
                    }
                }
                PositionedLayoutItem::InlineBox(inline_box) => {
                    boxes.push((inline_box.id, inline_box.x, inline_box.y));
                }
            }
        }
        lines.push(ObservedLine {
            text_range: line.text_range(),
            metrics: [
                metrics.baseline,
                metrics.block_min_coord,
                metrics.block_max_coord,
                metrics.line_height,
                metrics.advance,
                metrics.inline_min_coord,
                metrics.inline_max_coord,
            ],
            glyphs,
            boxes,
        });
    }
    let carets = text
        .char_indices()
        .map(|(index, _)| index)
        .chain([text.len()])
        .map(|index| {
            let rect = |affinity| {
                let bounds = Cursor::from_byte_index(layout, index, affinity).geometry(layout, 1.0);
                [bounds.x0, bounds.y0, bounds.x1, bounds.y1]
            };
            (index, rect(Affinity::Downstream), rect(Affinity::Upstream))
        })
        .collect();
    Observed {
        size: (layout.width(), layout.full_width(), layout.height()),
        lines,
        carets,
    }
}

/// How an editor in a case is set up beyond its text.
#[derive(Clone)]
struct Setup {
    /// The wrapping width, if any.
    width: Option<f32>,
    /// Range styles, over the case's starting text.
    styles: Vec<(Range<usize>, StyleProperty<'static, ColorBrush>)>,
    /// Inline boxes, over the case's starting text.
    boxes: Vec<InlineBox>,
}

/// An editor over `text` set up as `setup` says.
fn editor(text: &str, setup: &Setup) -> PlainEditor<ColorBrush> {
    let mut editor = PlainEditor::new(16.0);
    editor
        .edit_styles()
        .insert(StyleProperty::FontFamily(FontFamily::Source(
            Cow::Borrowed("Roboto"),
        )));
    editor.set_width(setup.width);
    editor.set_text(text);
    editor.set_range_styles(setup.styles.clone());
    editor.set_inline_boxes(setup.boxes.clone());
    editor
}

/// Lay `text` out whole with `setup`, the reference.
fn whole(
    text: &str,
    setup: &Setup,
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<ColorBrush>,
) -> Observed {
    let mut reference = editor(text, setup);
    reference.update_layout_whole(fcx, lcx);
    observe(
        reference.try_layout().expect("a whole layout is not dirty"),
        text,
    )
}

/// One edit: select a byte range and type over it.
struct Edit {
    /// What to select; an end past the text is the end of the text.
    range: Range<usize>,
    /// What to type.
    text: &'static str,
}

/// Lay `text` out paragraph by paragraph, check it against the whole, then make
/// each edit and check again.
fn check(name: &str, text: &str, setup: &Setup, edits: &[Edit]) {
    let mut fcx = create_font_context();
    let mut lcx = LayoutContext::new();
    let mut cached = editor(text, setup);
    cached.refresh_layout(&mut fcx, &mut lcx);
    let mut current = text.to_owned();
    let expected = whole(&current, setup, &mut fcx, &mut lcx);
    assert_eq!(
        observe(cached.try_layout().expect("laid out"), &current),
        expected,
        "{name}: first layout"
    );
    for (number, edit) in edits.iter().enumerate() {
        let range = edit.range.start..edit.range.end.min(current.len());
        let mut driver = cached.driver(&mut fcx, &mut lcx);
        driver.select_byte_range(range.start, range.end);
        driver.insert_or_replace_selection(edit.text);
        current.replace_range(range, edit.text);
        assert_eq!(cached.raw_text(), current, "{name}: edit {number} text");
        // The reference carries the editor's styles and boxes as they now
        // stand (an edit moves them).
        let now = Setup {
            width: setup.width,
            styles: cached.range_styles().to_vec(),
            boxes: cached.inline_boxes().to_vec(),
        };
        let expected = whole(&current, &now, &mut fcx, &mut lcx);
        assert_eq!(
            observe(cached.try_layout().expect("laid out"), &current),
            expected,
            "{name}: after edit {number}"
        );
    }
}

/// A short script, typed in, broken and joined.
const SCRIPT: &str =
    "default\n{\n    state_entry()\n    {\n        llSay(0, \"Hello, Avatar!\");\n    }\n}\n";

/// A plain setup: no wrapping, no styles, no boxes.
fn plain() -> Setup {
    Setup {
        width: None,
        styles: Vec::new(),
        boxes: Vec::new(),
    }
}

#[test]
fn a_script_matches_whole_through_edits() {
    check(
        "script",
        SCRIPT,
        &plain(),
        &[
            // Type in the middle of a line.
            Edit {
                range: 20..20,
                text: "x",
            },
            // Break a line.
            Edit {
                range: 30..30,
                text: "\n",
            },
            // Join two lines.
            Edit {
                range: 8..9,
                text: "",
            },
            // Replace across several lines.
            Edit {
                range: 3..40,
                text: "a\nb\n\nc",
            },
            // Empty the buffer.
            Edit {
                range: 0..usize::MAX,
                text: "",
            },
        ],
    );
}

#[test]
fn blank_lines_and_line_ends_match_whole() {
    for text in [
        "\n",
        "\n\n\n",
        "a\n",
        "\na",
        "a\n\nb",
        "a\r\nb\r\n",
        "trailing   \n  leading\n",
    ] {
        check(text, text, &plain(), &[]);
    }
}

#[test]
fn wrapped_paragraphs_match_whole() {
    let text = "a fairly long line of prose that has to wrap at least once or twice\nshort\n\
                and another long one, which wraps too, and then some more words\n";
    let setup = Setup {
        width: Some(120.0),
        ..plain()
    };
    check(
        "wrapped",
        text,
        &setup,
        &[Edit {
            range: 5..5,
            text: "inserted words ",
        }],
    );
}

#[test]
fn styles_across_lines_match_whole() {
    let setup = Setup {
        width: None,
        styles: vec![
            // A block comment's colour, across three lines.
            (
                2..30,
                StyleProperty::Brush(ColorBrush::new(palette::css::GREEN)),
            ),
            (10..14, StyleProperty::FontWeight(FontWeight::BOLD)),
            (40..60, StyleProperty::Underline(true)),
        ],
        boxes: Vec::new(),
    };
    check(
        "styled",
        SCRIPT,
        &setup,
        &[Edit {
            range: 12..12,
            text: "zz",
        }],
    );
}

#[test]
fn inline_boxes_match_whole() {
    let inline_box = |id: u64, index: usize| InlineBox {
        id,
        kind: InlineBoxKind::InFlow,
        index,
        width: 30.0,
        height: 12.0,
    };
    let setup = Setup {
        width: None,
        styles: Vec::new(),
        // At a line's start, in the middle of one, and at the very end.
        boxes: vec![
            inline_box(0, 8),
            inline_box(1, 13),
            inline_box(2, SCRIPT.len()),
        ],
    };
    check("boxes", SCRIPT, &setup, &[]);
}

#[test]
fn emoji_and_other_scripts_match_whole() {
    check(
        "emoji",
        "smile 😀 and family 👨‍👩‍👧\nStraße, 日本語, café\n",
        &plain(),
        &[Edit {
            range: 6..6,
            text: "!",
        }],
    );
}

#[test]
fn bidi_text_falls_back_and_still_matches() {
    check(
        "bidi",
        "hello\nمرحبا بالعالم\nworld",
        &plain(),
        &[Edit {
            range: 0..0,
            text: "x",
        }],
    );
}

#[test]
fn styles_and_boxes_follow_an_edit() {
    let mut fcx = create_font_context();
    let mut lcx = LayoutContext::new();
    let inline_box = |index: usize| InlineBox {
        id: 7,
        kind: InlineBoxKind::InFlow,
        index,
        width: 10.0,
        height: 10.0,
    };
    let setup = Setup {
        width: None,
        styles: vec![
            (0..4, StyleProperty::Underline(true)),
            (10..20, StyleProperty::Underline(true)),
            (40..60, StyleProperty::Underline(true)),
        ],
        boxes: vec![inline_box(15), inline_box(50)],
    };
    let mut cached = editor(SCRIPT, &setup);
    cached.refresh_layout(&mut fcx, &mut lcx);
    let mut driver = cached.driver(&mut fcx, &mut lcx);
    // Two bytes typed at 12, inside the second range; then 14..18 deleted,
    // taking the box at 15 with it.
    driver.select_byte_range(12, 12);
    driver.insert_or_replace_selection("zz");
    driver.select_byte_range(14, 18);
    driver.delete_selection();
    let ranges: Vec<Range<usize>> = cached
        .range_styles()
        .iter()
        .map(|(range, _property)| range.clone())
        .collect();
    assert_eq!(ranges, vec![0..4, 10..18, 38..58]);
    let boxes: Vec<usize> = cached.inline_boxes().iter().map(|b| b.index).collect();
    assert_eq!(boxes, vec![48]);
}
