// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent reference data for the corpus, computed from ICU4X.
//!
//! This module knows nothing about parley. It answers "what does Unicode say about this string?"
//! so that the harness can *compute* parley's divergence from the standard rather than assert it.
//!
//! # How independent is it, really
//!
//! Worth stating plainly, because a reader should not have to work it out:
//!
//! - **Grapheme cluster boundaries: genuinely independent.** parley's cluster boundaries come from
//!   `ClusterData::text_len = char::len_utf8()` — one per source char, with no segmentation
//!   involved. [`GraphemeClusterSegmenter`] is a real oracle for them.
//! - **The Emoji property: genuinely independent.** parley carries its own baked tables in
//!   `parley_data`, and `ClusterInfo::is_emoji` does not consult them at all — it is a hardcoded
//!   codepoint-range match. ICU4X is an oracle for both.
//! - **Word boundaries: NOT independent.** parley's `is_word_boundary` traces back to ICU's
//!   `WordSegmenter` via `CharInfo::boundary`, so comparing them is a pass-through consistency
//!   check, not a test. Any table using it must say so.
//!
//! # Dependency cost
//!
//! Zero new packages in `Cargo.lock`, verified by diffing it: `parley_core` already depends on
//! both `icu_segmenter` and `icu_properties` unconditionally with `compiled_data`, so they are
//! already compiled in every build of this workspace. `parley` itself already carries
//! `icu_properties` as a dev-dependency for the same reason this module does. `parley_tests` is
//! `publish = false`.
//!
//! Deliberately no `unicode-segmentation` as a second oracle: it would be a real new dependency,
//! and it would disagree with ICU purely on UCD version skew — noise that looks like signal.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use icu_properties::CodePointSetData;
use icu_properties::props::Emoji;
use icu_segmenter::GraphemeClusterSegmenter;

use super::corpus::CORPUS;

/// UAX #29 extended grapheme cluster boundaries, as byte offsets into `text`.
///
/// Includes the leading `0` and trailing `text.len()` that the segmenter emits, so the number of
/// clusters is `boundaries.len() - 1` for non-empty text.
pub(crate) fn egc_boundaries(text: &str) -> Vec<usize> {
    GraphemeClusterSegmenter::new().segment_str(text).collect()
}

/// Number of extended grapheme clusters in `text`.
pub(crate) fn egc_count(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    egc_boundaries(text).len() - 1
}

/// Whether `c` has the UTS #51 `Emoji` property.
///
/// The oracle for `ClusterInfo::is_emoji`'s hardcoded ranges, which carry a
/// `// TODO: Defer to ICU4X properties`.
pub(crate) fn is_uts51_emoji(c: char) -> bool {
    CodePointSetData::new::<Emoji>().contains(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_name;
    use crate::util::TestEnv;
    use std::fmt::Write;

    /// Sanity-checks the oracle against boundaries the standard states outright.
    ///
    /// Not redundant with the golden: the golden pins whatever ICU says, so if ICU were wired up
    /// wrong the golden would happily pin the wrong thing. These are the cases where UAX #29 names
    /// the answer in the rule text itself.
    #[test]
    fn traversal_reference_egc_matches_named_uax29_rules() {
        // GB3: CR x LF. One cluster, not two.
        assert_eq!(egc_count("\u{000D}\u{000A}"), 1, "GB3 CR x LF");
        // GB9: base x Extend.
        assert_eq!(egc_count("\u{0065}\u{0301}"), 1, "GB9 e + combining acute");
        // GB6-GB8: conjoining jamo L V T.
        assert_eq!(
            egc_count("\u{1100}\u{1161}\u{11A8}"),
            1,
            "GB6-GB8 Hangul jamo"
        );
        // GB11: Extended_Pictographic ZWJ Extended_Pictographic.
        assert_eq!(
            egc_count("\u{1F468}\u{200D}\u{1F469}"),
            1,
            "GB11 emoji ZWJ sequence"
        );
        // GB12/GB13: RI pairs by parity. Two RIs are one flag; three are a flag plus a remainder.
        assert_eq!(egc_count("\u{1F1EF}\u{1F1F5}"), 1, "GB12/GB13 RI pair");
        assert_eq!(
            egc_count("\u{1F1EF}\u{1F1F5}\u{1F1EF}"),
            2,
            "GB12/GB13 odd RI run"
        );
        // GB999: unrelated codepoints break.
        assert_eq!(egc_count("\u{0061}\u{0062}"), 2, "GB999 ab");
        // Empty text has no clusters.
        assert_eq!(egc_count(""), 0);
    }

    /// Pins the ICU version's answer for the two GB9c scripts that differ.
    ///
    /// Devanagari is one of the six scripts GB9c covers; Kannada is not. If a future ICU bump
    /// extends GB9c, this test fails and tells us the reference moved — which matters, because the
    /// harness reports parley's divergence *from* this.
    #[test]
    fn traversal_reference_gb9c_covers_devanagari_not_kannada() {
        assert_eq!(
            egc_count("\u{0915}\u{094D}\u{0937}"),
            1,
            "GB9c covers Devanagari: ka + virama + ssa is one cluster"
        );
        assert_eq!(
            egc_count("\u{0C95}\u{0CCD}\u{0CB7}"),
            2,
            "GB9c does NOT cover Kannada, despite the identical structure. If this ever becomes 1, \
             Unicode extended GB9c and EXPECTATIONS.md needs updating."
        );
    }

    /// The Emoji property must not be confused with parley's hardcoded ranges.
    ///
    /// These four are the documented surprise: `Emoji=Yes` for characters whose default
    /// presentation is text. This is why the emoji generic cannot go first unconditionally.
    #[test]
    fn traversal_reference_emoji_property_includes_text_presentation_chars() {
        for c in ['\u{0035}', '\u{0023}', '\u{002A}'] {
            assert!(
                is_uts51_emoji(c),
                "U+{:04X} has Emoji=Yes despite a text default presentation",
                c as u32
            );
        }
        assert!(is_uts51_emoji('\u{1F1EF}'), "regional indicators are Emoji");
        assert!(!is_uts51_emoji('\u{0061}'), "'a' is not Emoji");
        assert!(!is_uts51_emoji('\u{FE0F}'), "VS16 itself is not Emoji");
    }

    /// Emits the ICU reference table.
    ///
    /// No font axis and no segmenter axis: this is a pure function of the text and the UCD.
    /// Grapheme segmentation is unaffected by the `complex-scripts` feature, which only switches
    /// word and line breaking.
    #[test]
    fn traversal_reference_table() {
        let mut env = TestEnv::new(test_name!(), None);

        let mut out = String::new();
        writeln!(&mut out, "# Traversal reference: what Unicode says").unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "Generated by `traversal_reference_table` from ICU4X. Do not edit; run \
             `PARLEY_TEST=accept cargo test -p parley_tests traversal`."
        )
        .unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "This table contains **no parley measurements**. It is the independent oracle the \
             harness computes divergence against."
        )
        .unwrap();
        writeln!(&mut out).unwrap();
        writeln!(&mut out, "- `chars` — Unicode scalar values.").unwrap();
        writeln!(
            &mut out,
            "- `egc` — UAX #29 extended grapheme clusters. This is the count a whole-cluster \
             backspace would take."
        )
        .unwrap();
        writeln!(
            &mut out,
            "- `egc_bounds` — cluster boundaries as byte offsets, including 0 and len."
        )
        .unwrap();
        writeln!(
            &mut out,
            "- `chars_minus_egc` — how many extra steps a per-codepoint operation takes over a \
             per-grapheme one. **The disputed quantity**: 0 means every authority agrees, >0 means \
             the operation's granularity is user-visible for this entry."
        )
        .unwrap();
        writeln!(
            &mut out,
            "- `emoji_cps` — how many codepoints have the UTS #51 `Emoji` property."
        )
        .unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "| id | bytes | chars | egc | chars_minus_egc | emoji_cps | egc_bounds |"
        )
        .unwrap();
        writeln!(&mut out, "|---|---|---|---|---|---|---|").unwrap();

        for e in CORPUS {
            let chars = e.text.chars().count();
            let egc = egc_count(e.text);
            let bounds = egc_boundaries(e.text);
            let emoji_cps = e.text.chars().filter(|c| is_uts51_emoji(*c)).count();
            let bounds_str = if bounds.is_empty() {
                "(empty)".to_string()
            } else {
                bounds
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            writeln!(
                &mut out,
                "| {} | {} | {} | {} | {} | {} | {} |",
                e.id,
                e.text.len(),
                chars,
                egc,
                chars - egc,
                emoji_cps,
                bounds_str,
            )
            .unwrap();
        }

        // A summary a reader can check the table against without re-adding the column by hand.
        let contested = CORPUS
            .iter()
            .filter(|e| e.text.chars().count() > egc_count(e.text))
            .count();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "{} of {} entries have `chars_minus_egc > 0`, i.e. the granularity of an operation is \
             user-visible for them.",
            contested,
            CORPUS.len()
        )
        .unwrap();

        env.check_text_snapshot("traversal/reference.md", &out);
    }
}
