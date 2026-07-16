// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reference backspace rules, reimplemented so "what would X do" is a measurement.
//!
//! The dispute on PR #693 turns on what other implementations actually do. Quoting them is weaker
//! than running them: a quote can be stale (xilem #303's stated motivation described a VS Code
//! behaviour that had already been fixed three years earlier) and a summary can lose the case that
//! matters. So each rule here is executed over the same corpus as parley, and the comparison table
//! is generated.
//!
//! # ⚠ Licence: these are clean-room reimplementations
//!
//! xilem, xi-editor, druid and AOSP are **Apache-2.0 only**. parley is **Apache-2.0 OR MIT** across
//! its whole tree. **Copying their code into this workspace would break parley's dual licence.**
//!
//! Everything below is written from the *documented behaviour* — the state table in
//! `EXPECTATIONS.md` §3, Blink's documented divergences in §4, Pango's macro in §5, Qt's stated
//! rule in §6 — not transcribed from their sources. The structure deliberately does not mirror
//! theirs: no `deleteCharCount` accumulator, no 14-state enum. Where a rule needs a Unicode
//! property, it comes from ICU4X rather than from a vendored table.
//!
//! # What these are and are not
//!
//! They are good enough to answer "would this rule delete the whole sequence or one codepoint on
//! this input", which is the question the dispute turns on. They are **not** faithful ports, and a
//! disagreement between one of these and the real implementation is a bug *here*, not evidence
//! about them. Anything load-bearing should be confirmed against the real thing before being put
//! in an issue.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

use icu_properties::CodePointSetData;
use icu_properties::props::{Emoji, EmojiModifier, EmojiModifierBase, ExtendedPictographic};
use icu_segmenter::GraphemeClusterSegmenter;

/// Which authority's rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Rule {
    /// UAX #29 extended grapheme clusters. Also `CodeMirror` 6, which is exactly this.
    Egc,
    /// AOSP `BaseKeyListener.getOffsetForBackspaceKey`, as ported by xi-editor and xilem.
    Aosp,
    /// Blink's `BackspaceStateMachine`: AOSP's rule with `Extended_Pictographic` instead of
    /// `Emoji`.
    Blink,
    /// GTK/Pango: whole cluster for Latin/Cyrillic/Greek/Kana/Hangul/Emoji/Math bases, one
    /// NFD character otherwise.
    ///
    /// Specifically `gtk_text_buffer_backspace` consuming `PangoLogAttr` — i.e. `GtkTextView`.
    /// `GtkEntry`/`GtkEditable` were not examined.
    Pango,
    /// Qt `QTextCursor::deletePreviousChar`: whole cluster for emoji, one codepoint otherwise.
    ///
    /// **Scope**: `QTextCursor` is **`QtGui`** (`src/gui/text/qtextcursor.cpp`; the fix was tagged
    /// `[ChangeLog][QtGui][QTextCursor]`), the shared `QTextDocument` layer. So this covers the
    /// `QWidget` editors `QTextEdit` and `QPlainTextEdit`, and QML's `TextEdit`, which all drive a
    /// `QTextDocument` through a `QTextCursor`. Note it is live code rather than legacy: the fix is
    /// dated 2026-03-15 and closes QTBUG-67358 among others.
    ///
    /// It does **not** cover `QLineEdit`, which does not use `QTextCursor` — single-line editing
    /// goes through `QWidgetLineControl` instead. That is a separate implementation and was not
    /// examined, so this column says nothing about it.
    Qt,
}

impl Rule {
    pub(crate) const ALL: &'static [Self] =
        &[Self::Egc, Self::Aosp, Self::Blink, Self::Pango, Self::Qt];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Egc => "egc",
            Self::Aosp => "aosp",
            Self::Blink => "blink",
            Self::Pango => "pango",
            Self::Qt => "qt",
        }
    }

    /// How many bytes this rule's backspace removes from the end of `text`.
    pub(crate) fn backspace_len(self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }
        match self {
            Self::Egc => last_cluster(text).len(),
            Self::Aosp => emoji_aware_backspace(text, EmojiTest::Emoji),
            Self::Blink => emoji_aware_backspace(text, EmojiTest::ExtendedPictographic),
            Self::Pango => pango_backspace(text),
            Self::Qt => qt_backspace(text),
        }
    }

    /// Presses this rule needs to erase `text` entirely.
    pub(crate) fn presses(self, text: &str) -> usize {
        let mut buf = text.to_string();
        let mut n = 0;
        while !buf.is_empty() && n <= 256 {
            let len = self.backspace_len(&buf);
            if len == 0 {
                break;
            }
            buf.truncate(buf.len() - len);
            n += 1;
        }
        n
    }
}

/// The two implementations disagree on which property gates "is this emoji", and it is one of the
/// three documented divergences: `Emoji=Yes` includes digits, `#` and `*`, which are not
/// `Extended_Pictographic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmojiTest {
    Emoji,
    ExtendedPictographic,
}

impl EmojiTest {
    fn test(self, c: char) -> bool {
        match self {
            Self::Emoji => CodePointSetData::new::<Emoji>().contains(c),
            Self::ExtendedPictographic => {
                CodePointSetData::new::<ExtendedPictographic>().contains(c)
            }
        }
    }
}

const ZWJ: char = '\u{200D}';
const VS16: char = '\u{FE0F}';
const KEYCAP: char = '\u{20E3}';
const CANCEL_TAG: char = '\u{E007F}';

fn is_ri(c: char) -> bool {
    ('\u{1F1E6}'..='\u{1F1FF}').contains(&c)
}
fn is_tag_spec(c: char) -> bool {
    ('\u{E0020}'..='\u{E007E}').contains(&c)
}
fn is_keycap_base(c: char) -> bool {
    c.is_ascii_digit() || c == '#' || c == '*'
}
fn is_emoji_modifier(c: char) -> bool {
    CodePointSetData::new::<EmojiModifier>().contains(c)
}
fn is_emoji_modifier_base(c: char) -> bool {
    // AOSP keeps U+1F91D and U+1F93C as modifier bases although Emoji 4.0 removed them, because
    // fonts and existing text still treat them as such. Reproduced because it is a documented,
    // deliberate deviation from the property.
    c == '\u{1F91D}' || c == '\u{1F93C}' || CodePointSetData::new::<EmojiModifierBase>().contains(c)
}
fn is_variation_selector(c: char) -> bool {
    ('\u{FE00}'..='\u{FE0F}').contains(&c) || ('\u{E0100}'..='\u{E01EF}').contains(&c)
}

/// The last extended grapheme cluster of `text`.
fn last_cluster(text: &str) -> &str {
    let bounds: Vec<usize> = GraphemeClusterSegmenter::new().segment_str(text).collect();
    match bounds.len() {
        0 | 1 => text,
        n => &text[bounds[n - 2]..],
    }
}

/// The rule both AOSP and Blink implement: delete a complete emoji sequence if one ends here,
/// otherwise exactly one codepoint. CRLF is the sole non-emoji exception.
///
/// Written as a backwards scan over the sequence grammar rather than as their state machine —
/// same decisions, different shape. See the licence note in the module docs.
fn emoji_aware_backspace(text: &str, emoji: EmojiTest) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let Some(&last) = chars.last() else {
        return 0;
    };
    let one = last.len_utf8();

    // CRLF is atomic.
    if last == '\n' && chars.len() >= 2 && chars[chars.len() - 2] == '\r' {
        return one + 1;
    }

    // Tag sequence: CANCEL_TAG, tag specs, then a base.
    if last == CANCEL_TAG {
        let mut i = chars.len() - 1;
        let mut len = one;
        while i > 0 && is_tag_spec(chars[i - 1]) {
            i -= 1;
            len += chars[i].len_utf8();
        }
        if i > 0 && emoji.test(chars[i - 1]) {
            return len + chars[i - 1].len_utf8();
        }
        // No base found: AOSP deletes only the terminator. Blink handles the base properly, but
        // that only differs when a base *is* found, so both land here.
        return one;
    }

    // Keycap: base + VS16 + U+20E3.
    if last == KEYCAP {
        let mut len = one;
        let mut i = chars.len() - 1;
        if i > 0 && chars[i - 1] == VS16 {
            len += VS16.len_utf8();
            i -= 1;
        }
        if i > 0 && is_keycap_base(chars[i - 1]) {
            return len + chars[i - 1].len_utf8();
        }
        return one;
    }

    // Regional indicators, by parity: an odd run before this one means it completes a flag.
    if is_ri(last) {
        let run = chars.iter().rev().take_while(|c| is_ri(**c)).count();
        return if run % 2 == 0 { one * 2 } else { one };
    }

    // Emoji modifier: base + modifier, optionally with a VS between.
    if is_emoji_modifier(last) {
        let mut len = one;
        let mut i = chars.len() - 1;
        if i > 0 && is_variation_selector(chars[i - 1]) {
            len += chars[i - 1].len_utf8();
            i -= 1;
        }
        if i > 0 && is_emoji_modifier_base(chars[i - 1]) {
            len += chars[i - 1].len_utf8();
            i -= 1;
            // A modifier sequence can itself be an element of a ZWJ sequence.
            return len + zwj_tail(&chars[..=i.min(chars.len() - 1)], emoji, i);
        }
        return one;
    }

    // Variation selector.
    if is_variation_selector(last) {
        let mut i = chars.len() - 1;
        let mut len = one;
        while i > 0 && is_variation_selector(chars[i - 1]) {
            i -= 1;
            len += chars[i].len_utf8();
        }
        if i > 0 && emoji.test(chars[i - 1]) {
            len += chars[i - 1].len_utf8();
            return len + zwj_tail(&chars, emoji, i - 1);
        }
        // Not emoji: AOSP and Blink also consume the preceding char when it has combining class 0.
        // xilem is known to omit this check; it is included here because both real
        // implementations have it.
        if i > 0 && !is_variation_selector(chars[i - 1]) && combining_class_zero(chars[i - 1]) {
            return len + chars[i - 1].len_utf8();
        }
        return len;
    }

    // Plain emoji, possibly the tail of a ZWJ sequence.
    if emoji.test(last) {
        return one + zwj_tail(&chars, emoji, chars.len() - 1);
    }

    // Everything else: exactly one codepoint. This is the branch that makes the rule
    // per-codepoint for combining marks, Hangul jamo, Devanagari and Thai.
    one
}

/// Walks `ZWJ element` pairs backwards from `at`, returning the extra bytes consumed.
fn zwj_tail(chars: &[char], emoji: EmojiTest, at: usize) -> usize {
    let mut i = at;
    let mut len = 0;
    loop {
        if i == 0 || chars[i - 1] != ZWJ {
            return len;
        }
        // Look past the ZWJ for an element: emoji, optionally with a VS or a modifier.
        let mut j = i - 1;
        let mut candidate = ZWJ.len_utf8();
        if j == 0 {
            return len;
        }
        j -= 1;
        // A ZWJ element may be `base modifier` or `base VS`; both consume two characters. Kept as
        // one arm because the arithmetic is identical — only the predicate differs.
        let two_char_element =
            (is_emoji_modifier(chars[j]) && j > 0 && is_emoji_modifier_base(chars[j - 1]))
                || (is_variation_selector(chars[j]) && j > 0 && emoji.test(chars[j - 1]));
        if two_char_element {
            candidate += chars[j].len_utf8() + chars[j - 1].len_utf8();
            j -= 1;
        } else if emoji.test(chars[j]) {
            candidate += chars[j].len_utf8();
        } else {
            return len;
        }
        len += candidate;
        i = j;
    }
}

/// Canonical combining class 0, i.e. a starter.
///
/// ICU4X exposes the class as a property; only the "is it zero" question is needed here.
fn combining_class_zero(c: char) -> bool {
    use icu_properties::props::CanonicalCombiningClass;
    icu_properties::CodePointMapData::<CanonicalCombiningClass>::new().get(c)
        == CanonicalCombiningClass::NotReordered
}

/// Pango's rule: delete the whole cluster, then — if `backspace_deletes_character` — put back all
/// but the last NFD character of it.
///
/// The script list follows `pango/break.c`, **not** the Pango docs, which are stale: they mention
/// only Latin/Cyrillic/Greek while the code also excludes Kana, Hangul, Emoji and Math.
fn pango_backspace(text: &str) -> usize {
    let cluster = last_cluster(text);
    let Some(base) = cluster.chars().next() else {
        return 0;
    };

    let latin =
        ('\u{0020}'..='\u{02AF}').contains(&base) || ('\u{1E00}'..='\u{1EFF}').contains(&base);
    let cyrillic = ('\u{0400}'..='\u{052F}').contains(&base);
    let greek =
        ('\u{0370}'..='\u{03FF}').contains(&base) || ('\u{1F00}'..='\u{1FFF}').contains(&base);
    let kana = ('\u{3040}'..='\u{30FF}').contains(&base);
    // NB: covers precomposed syllables only. Conjoining jamo (U+1100..) are NOT in this range, so
    // Pango treats them as deletable character-by-character.
    let hangul = ('\u{AC00}'..='\u{D7A3}').contains(&base);
    // Pango's macro calls `_pango_Is_Emoji_Base_Character`, whose exact definition was not
    // available. `Emoji` is the closest documented property to "emoji base character", and it is
    // the one that reproduces the behaviour every source agrees on (a VS16 sequence deletes
    // whole). `Emoji_Presentation` was the first attempt and is wrong: `U+2764` is
    // `Emoji_Presentation=No` — which is exactly why it needs a VS16 — so that proxy split `❤️`
    // in two. **Approximate**; do not cite Pango's behaviour on a marginal codepoint from this.
    let emoji = CodePointSetData::new::<Emoji>().contains(base);
    let math = ('\u{2200}'..='\u{22FF}').contains(&base);

    let deletes_character = !(latin || cyrillic || greek || kana || hangul || emoji || math);

    if !deletes_character || cluster == "\r\n" {
        return cluster.len();
    }
    // Delete the cluster, reinsert all but the last NFD character. Approximated without a
    // normalizer: for the corpus this differs only for precomposed characters, where real Pango
    // would decompose and delete "less than one character".
    let mut chars = cluster.chars();
    match chars.next_back() {
        Some(last) if cluster.chars().count() > 1 => last.len_utf8(),
        _ => cluster.len(),
    }
}

/// Whether a cluster is an emoji sequence in the UTS #51 sense.
///
/// A stand-in for the `Script_Emoji` tag Qt's emoji segmenter assigns. Testing
/// `Extended_Pictographic` on the first character is **not** good enough and was the first attempt:
/// a keycap sequence begins with an ASCII digit, which is `Emoji=Yes` but
/// `Extended_Pictographic=No`, so that proxy classified `1️⃣` as ordinary text.
fn is_emoji_sequence(cluster: &str) -> bool {
    let chars: Vec<char> = cluster.chars().collect();
    if chars.len() < 2 {
        return false;
    }
    // Keycap: base + VS16 + U+20E3.
    if chars.last() == Some(&KEYCAP) && chars.first().is_some_and(|c| is_keycap_base(*c)) {
        return true;
    }
    // Flag: a pair of regional indicators.
    if chars.len() == 2 && chars.iter().all(|c| is_ri(*c)) {
        return true;
    }
    // Subdivision flag.
    if chars.last() == Some(&CANCEL_TAG) {
        return true;
    }
    // ZWJ sequences, modifier sequences, presentation sequences.
    chars
        .iter()
        .any(|c| CodePointSetData::new::<ExtendedPictographic>().contains(*c))
}

/// Qt's rule: whole cluster if the cluster is emoji, one codepoint otherwise.
///
/// Qt decides via script itemisation (`Script_Emoji` from its emoji segmenter) rather than a
/// codepoint state machine; the outcome rule is what is reproduced here.
///
/// Note Qt is **not documented to special-case CRLF** — its rule is emoji-or-one-codepoint and
/// nothing else, unlike AOSP (`STATE_LF`), Blink, Pango (an explicit `"\r\n"` check) and UAX #29
/// (GB3). So this reimplementation splits a CRLF pair. Whether real Qt does is **unverified**: a
/// `QTextDocument` normalises newlines to paragraph separators, so the case may simply never
/// arise there. Do not cite Qt's CRLF behaviour from this.
fn qt_backspace(text: &str) -> usize {
    let cluster = last_cluster(text);
    if is_emoji_sequence(cluster) {
        return cluster.len();
    }
    text.chars().next_back().map_or(0, char::len_utf8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules must agree on emoji, which is the block every authority agrees on and a UTS #51
    /// C2b conformance requirement. If a reimplementation gets these wrong it is not usable as a
    /// reference, and the comparison table would be misleading rather than merely incomplete.
    #[test]
    fn traversal_rules_agree_on_emoji_sequences() {
        for (name, text) in [
            (
                "zwj family",
                "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
            ),
            ("ri flag", "\u{1F1EF}\u{1F1F5}"),
            ("skin tone", "\u{1F44B}\u{1F3FD}"),
            ("keycap", "\u{0031}\u{FE0F}\u{20E3}"),
            ("heart vs16", "\u{2764}\u{FE0F}"),
            (
                "tag flag",
                "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
            ),
        ] {
            for rule in Rule::ALL {
                assert_eq!(
                    rule.presses(text),
                    1,
                    "{name}: rule {} took more than one press for an emoji sequence; UTS #51 C2b \
                     makes whole-sequence editing a conformance requirement and every \
                     implementation surveyed agrees here, so this is a bug in the \
                     reimplementation",
                    rule.slug(),
                );
            }
        }
    }

    /// The rules must *disagree* on the non-emoji block, in the documented direction. If they all
    /// agreed, the reimplementations would have collapsed into each other and the table would be
    /// vacuous.
    #[test]
    fn traversal_rules_split_on_non_emoji_clusters() {
        // e + combining acute: Android/Blink/Qt peel the mark; EGC and Pango (Latin base) take the
        // pair.
        let e_acute = "\u{0065}\u{0301}";
        assert_eq!(Rule::Aosp.presses(e_acute), 2, "AOSP peels the mark");
        assert_eq!(Rule::Blink.presses(e_acute), 2, "Blink peels the mark");
        assert_eq!(Rule::Qt.presses(e_acute), 2, "Qt peels the mark");
        assert_eq!(Rule::Egc.presses(e_acute), 1, "EGC takes the cluster");
        assert_eq!(
            Rule::Pango.presses(e_acute),
            1,
            "Pango excludes Latin bases from backspace_deletes_character, so it takes the cluster \
             — the documented disagreement with Android"
        );

        // Hangul jamo: the row where xilem #303 and PR #693 disagree.
        let jamo = "\u{1100}\u{1161}\u{11A8}";
        assert_eq!(Rule::Aosp.presses(jamo), 3, "AOSP peels jamo one at a time");
        assert_eq!(Rule::Egc.presses(jamo), 1, "EGC takes the syllable");
    }

    /// CRLF, per rule.
    ///
    /// Deliberately **not** asserted as a shared truth — that was the first attempt and it was
    /// wrong. Four of the five rules document an explicit CRLF case (UAX #29 GB3, AOSP's
    /// `STATE_LF`, Blink's port of it, Pango's `strcmp("\r\n")` guard). Qt documents no such case:
    /// its rule is emoji-or-one-codepoint and nothing else. Asserting that "everyone treats CRLF
    /// atomically" would have been an inference dressed up as a fact.
    #[test]
    fn traversal_rules_crlf_is_atomic_except_qt() {
        let crlf = "\u{000D}\u{000A}";
        for rule in [Rule::Egc, Rule::Aosp, Rule::Blink, Rule::Pango] {
            assert_eq!(
                rule.presses(crlf),
                1,
                "rule {} documents an explicit CRLF case and must not split the pair",
                rule.slug()
            );
        }
        assert_eq!(
            Rule::Qt.presses(crlf),
            2,
            "Qt documents no CRLF case, so this reimplementation splits the pair. Whether real Qt \
             does is UNVERIFIED — a QTextDocument normalises newlines to paragraph separators, so \
             the case may never arise. Do not cite Qt's CRLF behaviour from this."
        );
    }

    /// Two real graphemes take two presses under every rule. The control: a rule that returns 1
    /// here is deleting too much, and one that returns 3 is broken.
    #[test]
    fn traversal_rules_agree_on_plain_text() {
        for rule in Rule::ALL {
            assert_eq!(
                rule.presses("\u{0061}\u{0062}"),
                2,
                "rule {} miscounted plain ASCII",
                rule.slug()
            );
        }
    }
}
