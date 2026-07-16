// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Font tiers for the traversal harness.
//!
//! # Why this does not use `util::env::create_font_context`
//!
//! That helper loads **every** directory in `parley_dev::font_dirs()` into one collection, which is
//! what the 276 existing PNG snapshots were generated against. The harness needs to vary font
//! availability deliberately, so it composes its own collections and leaves `font_dirs()` alone.
//!
//! This is also why any future traversal-only font assets must go in a **new**
//! `parley_dev::traversal_font_dirs()` rather than being added to `font_dirs()`: adding a
//! Devanagari font to the shared list would change fallback resolution for every existing
//! snapshot, producing large silent churn unrelated to what is being measured.
//!
//! # A font family must be pushed, or nothing is measured
//!
//! Measured, not assumed: with `system_fonts: false` and **no `FontFamily` in the style**, no font
//! resolves, the layout gets **zero runs and zero clusters**, and every traversal API silently
//! falls back to a whole-text index. A test written that way passes for any input — see
//! `traversal_cursor_test_helper_produces_clusters` in `probe.rs`, which pins that trap.
//!
//! Note the requirement is only that *some* font resolves, **not** that it covers the script.
//! An uncovered codepoint still produces its own cluster with the correct `text_range`, so the
//! whole corpus is measurable with the bundled fonts.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use std::borrow::Cow;

use fontique::{Collection, CollectionOptions, SourceCache};
use parley::{
    FontContext, FontFamily, FontFamilyName, LayoutContext, PlainEditor, PlainEditorDriver,
    StyleProperty,
};

use crate::util::ColorBrush;
use crate::util::env::load_fonts;

/// Roboto only: covers Latin/Greek/Cyrillic and ligates `fi`, but has no Arabic.
const LATIN_ONLY_FAMILIES: &[FontFamilyName<'_>] =
    &[FontFamilyName::Named(Cow::Borrowed("Roboto"))];

/// Roboto plus Noto Kufi Arabic: adds real Arabic shaping, including the lam-alef ligature.
///
/// Deliberately the same list as `util::env::FONT_FAMILY_LIST`, but a separate constant — the
/// harness must not be perturbed by an unrelated change to the shared snapshot font stack, and
/// vice versa.
const BUNDLED_FAMILIES: &[FontFamilyName<'_>] = &[
    FontFamilyName::Named(Cow::Borrowed("Roboto")),
    FontFamilyName::Named(Cow::Borrowed("Noto Kufi Arabic")),
];

/// How much font support a measurement was taken with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontTier {
    /// Nothing registered at all.
    ///
    /// Degenerate: produces zero clusters, so traversal has nothing to walk. Kept because it is a
    /// real state a consumer can land in, and because it is the trap that makes a test vacuous —
    /// not because it is a useful comparison baseline.
    NoFonts,
    /// Roboto only. Latin shapes and ligates; Arabic does not resolve to a covering font.
    LatinOnly,
    /// Roboto + Noto Kufi Arabic. The tier every measurement is taken at.
    Bundled,
}

impl FontTier {
    pub(crate) const ALL: &'static [Self] = &[Self::NoFonts, Self::LatinOnly, Self::Bundled];

    /// The tiers that actually produce clusters, i.e. the ones a comparison can use.
    pub(crate) const MEASURABLE: &'static [Self] = &[Self::LatinOnly, Self::Bundled];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::NoFonts => "no_fonts",
            Self::LatinOnly => "latin_only",
            Self::Bundled => "bundled",
        }
    }

    fn families(self) -> &'static [FontFamilyName<'static>] {
        match self {
            Self::NoFonts | Self::LatinOnly => LATIN_ONLY_FAMILIES,
            Self::Bundled => BUNDLED_FAMILIES,
        }
    }

    pub(crate) fn font_context(self) -> FontContext {
        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: false,
        });
        if self != Self::NoFonts {
            load_fonts(&mut collection, parley_dev::font_dirs()).unwrap();
            for family in self.families() {
                if let FontFamilyName::Named(name) = family {
                    collection
                        .family_id(name)
                        .unwrap_or_else(|| panic!("{name} font not found"));
                }
            }
        }
        FontContext {
            collection,
            source_cache: SourceCache::default(),
        }
    }
}

/// A font context plus layout context for one tier.
pub(crate) struct TraversalEnv {
    pub(crate) tier: FontTier,
    pub(crate) font_cx: FontContext,
    pub(crate) layout_cx: LayoutContext<ColorBrush>,
}

impl TraversalEnv {
    pub(crate) fn new(tier: FontTier) -> Self {
        Self {
            tier,
            font_cx: tier.font_context(),
            layout_cx: LayoutContext::new(),
        }
    }

    /// An editor holding `text`, styled with this tier's font family.
    ///
    /// Pushing the family matters for the same reason it does in [`Self::layout`]: without it no
    /// font resolves and the editor has no clusters, so every operation becomes a silent no-op.
    pub(crate) fn editor(&self, text: &str) -> PlainEditor<ColorBrush> {
        let mut editor = PlainEditor::new(16.0);
        editor
            .edit_styles()
            .insert(StyleProperty::FontFamily(FontFamily::List(Cow::Borrowed(
                self.tier.families(),
            ))));
        editor
            .edit_styles()
            .insert(StyleProperty::Brush(ColorBrush::default()));
        editor.set_text(text);
        editor
    }

    pub(crate) fn driver<'a>(
        &'a mut self,
        editor: &'a mut PlainEditor<ColorBrush>,
    ) -> PlainEditorDriver<'a, ColorBrush> {
        editor.driver(&mut self.font_cx, &mut self.layout_cx)
    }

    /// Lays out `text` with this tier's font family, breaking only at hard line breaks.
    pub(crate) fn layout(&mut self, text: &str) -> parley::Layout<ColorBrush> {
        let families = self.tier.families();
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(FontFamily::List(Cow::Borrowed(
            families,
        ))));
        builder.push_default(StyleProperty::Brush(ColorBrush::default()));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    }
}
