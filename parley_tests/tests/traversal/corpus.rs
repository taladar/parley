// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The text corpus the traversal harness measures.
//!
//! Every entry is a sequence whose "user-perceived character" count is interesting: multi-codepoint
//! graphemes, sequences where shaping reorders or ligates, and the boring controls that prove a
//! measurement is not vacuous (`ab` must take two presses, or the harness is broken).
//!
//! # Why the text is written as escapes
//!
//! [`CorpusEntry::text`] contains **only** `\u{...}` escapes, enforced by
//! `traversal_corpus_source_is_ascii_escaped`. Three reasons, all learned the hard way:
//!
//! - A precomposed e-acute and a decomposed one are indistinguishable on screen but behave
//!   differently under every operation here. Written as escapes, they cannot be confused.
//! - ZWJ, lone combining marks and bidi controls in a source literal are hostile to editors, `git
//!   diff`, and terminals. Escapes are ASCII by construction, so the bytes on disk *are* the
//!   codepoints and no normalization pass can quietly change what is tested.
//! - Rendering the text into a golden is how a human reads a skin-tone emoji as "one thing" and
//!   writes down 1 when the measured answer is 2. The inventory golden emits codepoints, never
//!   glyphs.
//!
//! Each entry carries a comment naming its codepoints, because a bare `\u{1F468}` is unreadable
//! while editing. Those comments were generated from the Unicode character database rather than
//! typed, but they are **documentation only** — `text` is the source of truth, and the inventory
//! golden is generated from `text`, so a stale comment cannot corrupt a measurement.
//!
//! # Font coverage
//!
//! Many entries have no glyph in the bundled test fonts and will shape to `.notdef`. That is
//! deliberate and not (yet) a problem: `ClusterData::text_len` is `char::len_utf8()` per source
//! char, so cluster *text ranges* are expected to be shaping-independent. Whether that is actually
//! true is measured by the font-sensitivity probe rather than assumed — see `probe.rs`. Anything
//! that *is* font-dependent (`is_ligature_*`, `glyph_len`, visual order, hit-testing) is only
//! measurable for the scripts the bundled fonts cover.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

/// What makes an entry interesting. Groups the inventory table and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Group {
    /// Single-codepoint graphemes. The controls.
    Ascii,
    /// Base + combining mark(s), and their precomposed equivalents.
    Combining,
    /// Line terminators, including the CRLF pair.
    LineBreak,
    /// Emoji sequences. The one block where every authority agrees.
    Emoji,
    /// Hangul, precomposed and as conjoining jamo.
    Hangul,
    /// Brahmic scripts: matras, conjuncts, split vowels.
    Indic,
    /// Thai and Lao.
    Thai,
    /// Khmer and Myanmar stacking.
    SoutheastAsian,
    /// Arabic and Hebrew.
    Rtl,
    /// Japanese.
    Cjk,
    /// Ligatures and other shaping artifacts.
    Shaping,
    /// Zero-width and format characters on their own.
    Format,
    /// Whitespace, for word motion.
    Whitespace,
    /// Bidi runs.
    Bidi,
}

impl Group {
    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Combining => "combining",
            Self::LineBreak => "line_break",
            Self::Emoji => "emoji",
            Self::Hangul => "hangul",
            Self::Indic => "indic",
            Self::Thai => "thai",
            Self::SoutheastAsian => "southeast_asian",
            Self::Rtl => "rtl",
            Self::Cjk => "cjk",
            Self::Shaping => "shaping",
            Self::Format => "format",
            Self::Whitespace => "whitespace",
            Self::Bidi => "bidi",
        }
    }
}

/// Whether the bundled test fonts can actually shape this entry.
///
/// Recorded so a reader can tell a measurement of parley from a measurement of `.notdef` without
/// having to know which fonts ship in `parley_dev`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontCoverage {
    /// Every codepoint has a glyph in a bundled font.
    Covered,
    /// No bundled font covers this; it shapes to `.notdef`.
    Tofu,
    /// Some codepoints are covered and some are not.
    Partial,
}

/// One text sequence to measure.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CorpusEntry {
    /// Stable key. Used in golden tables and cross-referenced from `expectations.rs`, so renaming
    /// one is a golden diff plus a test failure, never a silent drop.
    pub(crate) id: &'static str,
    pub(crate) group: Group,
    /// **Escapes only.** See the module docs.
    pub(crate) text: &'static str,
    pub(crate) coverage: FontCoverage,
    /// Why this entry is here. Never a number — numbers come from the harness.
    pub(crate) note: &'static str,
}

/// The corpus.
///
/// Order is fixed and load-bearing: golden tables iterate this slice directly, so reordering it
/// rewrites every table. Append rather than insert.
pub(crate) const CORPUS: &[CorpusEntry] = &[
    // ---- Controls. If these are wrong, nothing else means anything. ----
    CorpusEntry {
        id: "ascii_ab",
        group: Group::Ascii,
        // LATIN SMALL LETTER A, LATIN SMALL LETTER B
        text: "\u{0061}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "Two real graphemes. Every op must take exactly two steps; a harness that reports \
               anything else is broken.",
    },
    CorpusEntry {
        id: "ascii_a",
        group: Group::Ascii,
        // LATIN SMALL LETTER A
        text: "\u{0061}",
        coverage: FontCoverage::Covered,
        note: "Single codepoint, single grapheme. The degenerate case.",
    },
    CorpusEntry {
        id: "ascii_empty",
        group: Group::Ascii,
        text: "",
        coverage: FontCoverage::Covered,
        note: "Empty buffer. Backdelete and delete must be no-ops rather than panics.",
    },
    // ---- Combining marks. The Latin row is where GTK and Android disagree. ----
    CorpusEntry {
        id: "latin_e_combining_acute",
        group: Group::Combining,
        // LATIN SMALL LETTER E, COMBINING ACUTE ACCENT
        text: "\u{0065}\u{0301}",
        coverage: FontCoverage::Covered,
        note: "e + COMBINING ACUTE. One of only two rows where xilem#303 disagrees with PR #693. \
               Android deletes the mark alone; GTK deletes the whole cluster (Latin base).",
    },
    CorpusEntry {
        id: "latin_e_acute_precomposed",
        group: Group::Combining,
        // LATIN SMALL LETTER E WITH ACUTE
        text: "\u{00E9}",
        coverage: FontCoverage::Covered,
        note: "Precomposed e-acute. Canonically equivalent to latin_e_combining_acute but one \
               codepoint, so every op should differ. Pairing them isolates encoding from behavior.",
    },
    CorpusEntry {
        id: "latin_e_two_combining",
        group: Group::Combining,
        // LATIN SMALL LETTER E, COMBINING ACUTE ACCENT, COMBINING DIAERESIS
        text: "\u{0065}\u{0301}\u{0308}",
        coverage: FontCoverage::Covered,
        note: "e + acute + diaeresis. Two marks: a per-codepoint backspace takes three presses, a \
               grapheme one takes one.",
    },
    CorpusEntry {
        id: "combining_acute_alone",
        group: Group::Combining,
        // COMBINING ACUTE ACCENT
        text: "\u{0301}",
        coverage: FontCoverage::Covered,
        note: "A lone combining mark with no base — a legitimate buffer state after deleting the \
               base of latin_e_combining_acute. Targets the OrphanedCombiningMark detector.",
    },
    // ---- Line breaks. CRLF is the one non-emoji case AOSP treats atomically. ----
    CorpusEntry {
        id: "crlf",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, CARRIAGE RETURN, LINE FEED, LATIN SMALL LETTER B
        text: "\u{0061}\u{000D}\u{000A}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "GB3 CR x LF. Upstream #667 made this ONE hard line break for line breaking, \
               deliberately not for cursor/selection. One of the three rows PR #693 got wrong.",
    },
    CorpusEntry {
        id: "lf",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, LINE FEED, LATIN SMALL LETTER B
        text: "\u{0061}\u{000A}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "Bare LF, the CRLF control.",
    },
    CorpusEntry {
        id: "cr",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, CARRIAGE RETURN, LATIN SMALL LETTER B
        text: "\u{0061}\u{000D}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "Bare CR.",
    },
    CorpusEntry {
        id: "line_separator",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, LINE SEPARATOR, LATIN SMALL LETTER B
        text: "\u{0061}\u{2028}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "U+2028 LINE SEPARATOR. `to_whitespace` maps it to Whitespace::Newline.",
    },
    CorpusEntry {
        id: "paragraph_separator",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, PARAGRAPH SEPARATOR, LATIN SMALL LETTER B
        text: "\u{0061}\u{2029}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "U+2029 PARAGRAPH SEPARATOR.",
    },
    CorpusEntry {
        id: "next_line_nel",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, NEXT LINE, LATIN SMALL LETTER B
        text: "\u{0061}\u{0085}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "U+0085 NEL. Listed as a newline in break_overrides.rs but NOT in `to_whitespace`, \
               so is_hard_line_break should be false while line breaking treats it as a break. \
               Measures that asymmetry.",
    },
    CorpusEntry {
        id: "vertical_tab",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, LINE TABULATION, LATIN SMALL LETTER B
        text: "\u{0061}\u{000B}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "U+000B VT. Same asymmetry as NEL.",
    },
    CorpusEntry {
        id: "form_feed",
        group: Group::LineBreak,
        // LATIN SMALL LETTER A, FORM FEED, LATIN SMALL LETTER B
        text: "\u{0061}\u{000C}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "U+000C FF.",
    },
    // ---- Emoji. The unanimous block, and where parley's is_emoji misfires. ----
    CorpusEntry {
        id: "emoji_party_popper",
        group: Group::Emoji,
        // PARTY POPPER
        text: "\u{1F389}",
        coverage: FontCoverage::Covered,
        note: "Single-codepoint emoji, in the bundled subset. ClusterInfo::is_emoji true.",
    },
    CorpusEntry {
        id: "emoji_cowboy",
        group: Group::Emoji,
        // FACE WITH COWBOY HAT
        text: "\u{1F920}",
        coverage: FontCoverage::Covered,
        note: "In the bundled subset, but OUTSIDE the hardcoded is_emoji ranges — measured false. \
               A bundled test emoji that backdelete's special case does not recognise.",
    },
    CorpusEntry {
        id: "emoji_check_mark",
        group: Group::Emoji,
        // WHITE HEAVY CHECK MARK
        text: "\u{2705}",
        coverage: FontCoverage::Covered,
        note: "In the bundled subset and inside the is_emoji ranges.",
    },
    CorpusEntry {
        id: "emoji_victory_hand_vs16",
        group: Group::Emoji,
        // VICTORY HAND, VARIATION SELECTOR-16
        text: "\u{270C}\u{FE0F}",
        coverage: FontCoverage::Covered,
        note: "The ONLY multi-codepoint sequence the bundled emoji font covers. VS16 is not in any \
               emoji font's cmap (see #685), so coverage here means the base only.",
    },
    CorpusEntry {
        id: "emoji_heart_vs16",
        group: Group::Emoji,
        // HEAVY BLACK HEART, VARIATION SELECTOR-16
        text: "\u{2764}\u{FE0F}",
        coverage: FontCoverage::Tofu,
        note: "The PR #692 case. Requests emoji presentation via VS16.",
    },
    CorpusEntry {
        id: "emoji_heart_vs15",
        group: Group::Emoji,
        // HEAVY BLACK HEART, VARIATION SELECTOR-15
        text: "\u{2764}\u{FE0E}",
        coverage: FontCoverage::Tofu,
        note: "VS15 requests the OPPOSITE presentation. is_variation_selector cannot tell VS15 and \
               VS16 apart even though they request opposite things.",
    },
    CorpusEntry {
        id: "emoji_heart_bare",
        group: Group::Emoji,
        // HEAVY BLACK HEART
        text: "\u{2764}",
        coverage: FontCoverage::Tofu,
        note: "No selector: nothing is requested, so the text font must win. The control for both \
               VS rows.",
    },
    CorpusEntry {
        id: "emoji_zwj_family",
        group: Group::Emoji,
        // MAN, ZERO WIDTH JOINER, WOMAN, ZERO WIDTH JOINER, GIRL, ZERO WIDTH JOINER, BOY
        text: "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}",
        coverage: FontCoverage::Tofu,
        note: "GB11. Seven codepoints, one grapheme. The headline case: measured 7 presses on main \
               DESPITE the emoji special case existing.",
    },
    CorpusEntry {
        id: "emoji_ri_flag_jp",
        group: Group::Emoji,
        // REGIONAL INDICATOR SYMBOL LETTER J, REGIONAL INDICATOR SYMBOL LETTER P
        text: "\u{1F1EF}\u{1F1F5}",
        coverage: FontCoverage::Tofu,
        note: "GB12/GB13. Regional indicators are measured is_emoji=FALSE, so backdelete's special \
               case never fires for flags at all.",
    },
    CorpusEntry {
        id: "emoji_ri_odd_run",
        group: Group::Emoji,
        // REGIONAL INDICATOR SYMBOL LETTER J, REGIONAL INDICATOR SYMBOL LETTER P
        // REGIONAL INDICATOR SYMBOL LETTER J
        text: "\u{1F1EF}\u{1F1F5}\u{1F1EF}",
        coverage: FontCoverage::Tofu,
        note: "Three RIs: one flag plus a dangling half. AOSP's parity trick (EVEN_NUMBERED_RIS \
               subtracts 2) exists for exactly this. Wordingham argues per-codepoint backspace is \
               essential here, to fix a typo'd flag.",
    },
    CorpusEntry {
        id: "emoji_ri_at_text_start",
        group: Group::Emoji,
        // REGIONAL INDICATOR SYMBOL LETTER J, REGIONAL INDICATOR SYMBOL LETTER P
        // LATIN SMALL LETTER A
        text: "\u{1F1EF}\u{1F1F5}\u{0061}",
        coverage: FontCoverage::Tofu,
        note: "GB12 is the sot-anchored rule; GB13 is the general one. Exercises the boundary \
               between them.",
    },
    CorpusEntry {
        id: "emoji_skin_tone",
        group: Group::Emoji,
        // WAVING HAND SIGN, EMOJI MODIFIER FITZPATRICK TYPE-4
        text: "\u{1F44B}\u{1F3FD}",
        coverage: FontCoverage::Tofu,
        note: "Modifier base + modifier. One of the three rows PR #693 got wrong: claimed 1, \
               measured 2. Both codepoints are inside the is_emoji ranges, so the special case \
               fires and deletes only the modifier.",
    },
    CorpusEntry {
        id: "emoji_keycap_1",
        group: Group::Emoji,
        // DIGIT ONE, VARIATION SELECTOR-16, COMBINING ENCLOSING KEYCAP
        text: "\u{0031}\u{FE0F}\u{20E3}",
        coverage: FontCoverage::Tofu,
        note: "digit + VS16 + COMBINING ENCLOSING KEYCAP. The VS16 sits in the MIDDLE, which is \
               why AOSP needs both BEFORE_KEYCAP and BEFORE_VS_AND_KEYCAP.",
    },
    CorpusEntry {
        id: "emoji_tag_flag_scotland",
        group: Group::Emoji,
        // WAVING BLACK FLAG, TAG LATIN SMALL LETTER G, TAG LATIN SMALL LETTER B
        // TAG LATIN SMALL LETTER S, TAG LATIN SMALL LETTER C, TAG LATIN SMALL LETTER T
        // CANCEL TAG
        text: "\u{1F3F4}\u{E0067}\u{E0062}\u{E0073}\u{E0063}\u{E0074}\u{E007F}",
        coverage: FontCoverage::Tofu,
        note: "Tag sequence, 7 codepoints. AOSP deletes only U+E007F when it cannot find the base; \
               Blink fixes this. Expect the aosp and blink reference columns to diverge here.",
    },
    // ---- Hangul. The disputed row, and the one where grapheme is most defensible. ----
    CorpusEntry {
        id: "hangul_jamo_gak",
        group: Group::Hangul,
        // HANGUL CHOSEONG KIYEOK, HANGUL JUNGSEONG A, HANGUL JONGSEONG KIYEOK
        text: "\u{1100}\u{1161}\u{11A8}",
        coverage: FontCoverage::Tofu,
        note: "Conjoining jamo L+V+T = one grapheme by GB6-GB8. The other row where xilem#303 \
               disagrees with PR #693. Claimed 1 press, measured 3. Note Pango's HANGUL macro \
               covers U+AC00..U+D7A3 only, so it does NOT match these jamo.",
    },
    CorpusEntry {
        id: "hangul_precomposed_gak",
        group: Group::Hangul,
        // HANGUL SYLLABLE GAG
        text: "\u{AC01}",
        coverage: FontCoverage::Tofu,
        note: "Precomposed syllable, canonically equivalent to hangul_jamo_gak. What real Korean \
               text contains (KS X 1026-1). The control that shows the jamo count is an encoding \
               artifact, not a Korean-user expectation.",
    },
    CorpusEntry {
        id: "hangul_lv_no_trailing",
        group: Group::Hangul,
        // HANGUL CHOSEONG KIYEOK, HANGUL JUNGSEONG A
        text: "\u{1100}\u{1161}",
        coverage: FontCoverage::Tofu,
        note: "L+V with no T. Windows split these apart and became the documented outlier; ICU, \
               iOS and Android keep them together.",
    },
    // ---- Indic. GB9c covers six scripts; three of these are not among them. ----
    CorpusEntry {
        id: "devanagari_ka_i_matra",
        group: Group::Indic,
        // DEVANAGARI LETTER KA, DEVANAGARI VOWEL SIGN I
        text: "\u{0915}\u{093F}",
        coverage: FontCoverage::Tofu,
        note: "Consonant + I matra. The matra is a SpacingMark (GB9a) and REORDERS to display \
               before the base though stored after. Visual-order cursor motion is the interesting \
               part and needs a font to measure.",
    },
    CorpusEntry {
        id: "devanagari_conjunct_ksha",
        group: Group::Indic,
        // DEVANAGARI LETTER KA, DEVANAGARI SIGN VIRAMA, DEVANAGARI LETTER SSA
        text: "\u{0915}\u{094D}\u{0937}",
        coverage: FontCoverage::Tofu,
        note: "ka + virama + ssa. GB9c (Unicode 15.1) keeps this together; before 15.1 the virama \
               broke it. Devanagari IS one of the six covered scripts.",
    },
    CorpusEntry {
        id: "devanagari_ka_virama",
        group: Group::Indic,
        // DEVANAGARI LETTER KA, DEVANAGARI SIGN VIRAMA
        text: "\u{0915}\u{094D}",
        coverage: FontCoverage::Tofu,
        note: "A dangling virama with no following consonant: GB9c does not apply. The control \
               for devanagari_conjunct_ksha.",
    },
    CorpusEntry {
        id: "tamil_ko_split_vowel",
        group: Group::Indic,
        // TAMIL LETTER KA, TAMIL VOWEL SIGN OO
        text: "\u{0B95}\u{0BCB}",
        coverage: FontCoverage::Tofu,
        note: "A SPLIT vowel: one codepoint renders as glyphs on BOTH sides of the base. Tamil is \
               NOT covered by GB9c. W3C's canonical example of atomic cursor with per-codepoint \
               backspace.",
    },
    CorpusEntry {
        id: "bengali_conjunct",
        group: Group::Indic,
        // BENGALI LETTER KA, BENGALI SIGN VIRAMA, BENGALI LETTER SSA
        text: "\u{0995}\u{09CD}\u{09B7}",
        coverage: FontCoverage::Tofu,
        note: "Bengali is covered by GB9c. Pairs with kannada_conjunct, which is not — same \
               structure, different rule.",
    },
    CorpusEntry {
        id: "kannada_conjunct",
        group: Group::Indic,
        // KANNADA LETTER KA, KANNADA SIGN VIRAMA, KANNADA LETTER SSA
        text: "\u{0C95}\u{0CCD}\u{0CB7}",
        coverage: FontCoverage::Tofu,
        note: "Structurally identical to bengali_conjunct but Kannada is NOT in GB9c's six \
               scripts, so ICU should segment it differently. Isolates the rule from the shape.",
    },
    // ---- Thai. The only script with a standard prescribing all three ops differently. ----
    CorpusEntry {
        id: "thai_mai_leading_vowel",
        group: Group::Thai,
        // THAI CHARACTER SARA AI MAIMALAI, THAI CHARACTER MO MA, THAI CHARACTER MAI EK
        text: "\u{0E44}\u{0E21}\u{0E48}",
        coverage: FontCoverage::Tofu,
        note: "FUTO #1532's example. Leading vowel U+0E44 is GCB=Other so it is its own cluster: \
               2 grapheme clusters, but Thai users want 3 backspaces. Grapheme backspace is \
               demonstrably wrong here.",
    },
    CorpusEntry {
        id: "thai_sara_am",
        group: Group::Thai,
        // THAI CHARACTER KO KAI, THAI CHARACTER SARA AM
        text: "\u{0E01}\u{0E33}",
        coverage: FontCoverage::Tofu,
        note: "U+0E33 SARA AM is Lo forced to SpacingMark — the one documented legacy/extended \
               difference for Thai. r12a measured Gecko/WebKit taking 2 cursor steps through it.",
    },
    CorpusEntry {
        id: "thai_tone_above",
        group: Group::Thai,
        // THAI CHARACTER THO THAHAN, THAI CHARACTER SARA II, THAI CHARACTER MAI EK
        text: "\u{0E17}\u{0E35}\u{0E48}",
        coverage: FontCoverage::Tofu,
        note: "Base + vowel above + tone mark = one cluster. Contradicts the common belief that \
               Thai marks are not Extend; verified against GraphemeBreakProperty.txt.",
    },
    CorpusEntry {
        id: "thai_word_sequence",
        group: Group::Thai,
        // THAI CHARACTER SARA AI MAIMALAI, THAI CHARACTER MO MA, THAI CHARACTER MAI EK
        // THAI CHARACTER SARA AI MAIMALAI, THAI CHARACTER DO DEK, THAI CHARACTER MAI THO
        // THAI CHARACTER KO KAI, THAI CHARACTER SARA I, THAI CHARACTER NO NU
        text: "\u{0E44}\u{0E21}\u{0E48}\u{0E44}\u{0E14}\u{0E49}\u{0E01}\u{0E34}\u{0E19}",
        coverage: FontCoverage::Tofu,
        note: "Thai has no spaces between words. The complex-scripts feature switches ICU between \
               dictionary and char-level word breaking; this entry's is_word_boundary bitvector \
               is the probe that makes the axis label a measurement.",
    },
    // ---- Khmer / Myanmar: grapheme is wrong in BOTH directions, no browser consensus. ----
    CorpusEntry {
        id: "khmer_coeng_stack",
        group: Group::SoutheastAsian,
        // KHMER LETTER KA, KHMER SIGN COENG, KHMER LETTER KA
        text: "\u{1780}\u{17D2}\u{1780}",
        coverage: FontCoverage::Tofu,
        note: "COENG U+17D2 is an Invisible_Stacker. Pre-15.1 rules break after it, so cluster \
               boundaries land at INVISIBLE positions. GB9c deliberately excluded Khmer.",
    },
    CorpusEntry {
        id: "myanmar_stack",
        group: Group::SoutheastAsian,
        // MYANMAR LETTER KA, MYANMAR SIGN VIRAMA, MYANMAR LETTER KA
        text: "\u{1000}\u{1039}\u{1000}",
        coverage: FontCoverage::Tofu,
        note: "Burmese virama stack. r12a measured three engines behaving three ways, incl. \
               Blink's 'cursor sometimes appears stationary' — the StationaryStep detector's \
               reason for existing.",
    },
    // ---- RTL. Arabic lam-alef and the ligature are measurable in-tree today. ----
    CorpusEntry {
        id: "arabic_lam_alef",
        group: Group::Rtl,
        // ARABIC LETTER LAM, ARABIC LETTER ALEF
        text: "\u{0644}\u{0627}",
        coverage: FontCoverage::Covered,
        note: "A MANDATORY rendering ligature of two codepoints with no GB rule joining them. \
               Every engine gives 2 stops. Covered by Noto Kufi Arabic, so the ligature flags and \
               visual order are measurable here and almost nowhere else.",
    },
    CorpusEntry {
        id: "arabic_base_harakat",
        group: Group::Rtl,
        // ARABIC LETTER BEH, ARABIC FATHA
        text: "\u{0628}\u{064E}",
        coverage: FontCoverage::Covered,
        note: "beh + fatha. Harakat are Mn => GB9 x Extend. Wikimedia T53472 was filed BY Arabic \
               users against whole-cluster backspace.",
    },
    CorpusEntry {
        id: "arabic_shadda_fatha",
        group: Group::Rtl,
        // ARABIC LETTER BEH, ARABIC SHADDA, ARABIC FATHA
        text: "\u{0628}\u{0651}\u{064E}",
        coverage: FontCoverage::Covered,
        note: "Two stacked harakat. Per-codepoint backspace takes three presses.",
    },
    CorpusEntry {
        id: "arabic_tatweel",
        group: Group::Rtl,
        // ARABIC LETTER BEH, ARABIC TATWEEL, ARABIC LETTER BEH
        text: "\u{0628}\u{0640}\u{0628}",
        coverage: FontCoverage::Covered,
        note: "Tatweel is Lm, a spacing LETTER, not a mark: its own cluster, own stop, own \
               backspace. The control that shows not every Arabic joiner is special.",
    },
    CorpusEntry {
        id: "hebrew_base_niqqud",
        group: Group::Rtl,
        // HEBREW LETTER ALEF, HEBREW POINT QAMATS
        text: "\u{05D0}\u{05B8}",
        coverage: FontCoverage::Covered,
        note: "alef + qamats. Arimo has Hebrew — but Arimo is NOT in FONT_FAMILY_LIST, so this is \
               only covered if the family is requested explicitly.",
    },
    CorpusEntry {
        id: "hebrew_cantillation",
        group: Group::Rtl,
        // HEBREW LETTER ALEF, HEBREW POINT QAMATS, HEBREW ACCENT ETNAHTA
        text: "\u{05D0}\u{05B8}\u{0591}",
        coverage: FontCoverage::Covered,
        note: "Vowel point plus cantillation mark: three codepoints, one cluster.",
    },
    // ---- CJK ----
    CorpusEntry {
        id: "japanese_ka_dakuten",
        group: Group::Cjk,
        // HIRAGANA LETTER KA, COMBINING KATAKANA-HIRAGANA VOICED SOUND MARK
        text: "\u{304B}\u{3099}",
        coverage: FontCoverage::Tofu,
        note: "ka + COMBINING dakuten, canonically equivalent to precomposed ga. Rare in real text \
               (required only for half-width kana) but a clean Mn/Extend case.",
    },
    CorpusEntry {
        id: "japanese_ga_precomposed",
        group: Group::Cjk,
        // HIRAGANA LETTER GA
        text: "\u{304C}",
        coverage: FontCoverage::Tofu,
        note: "Precomposed ga. The control for japanese_ka_dakuten.",
    },
    CorpusEntry {
        id: "japanese_ivs",
        group: Group::Cjk,
        // CJK UNIFIED IDEOGRAPH-845B, VARIATION SELECTOR-17
        text: "\u{845B}\u{E0100}",
        coverage: FontCoverage::Tofu,
        note: "Ideographic variation sequence. Deleting the selector alone would silently leave a \
               differently-shaped kanji. NB: subsetters drop cmap format 14 by default, so a naive \
               font subset would make this measure a lie.",
    },
    // ---- Shaping ----
    CorpusEntry {
        id: "ligature_fi",
        group: Group::Shaping,
        // LATIN SMALL LETTER F, LATIN SMALL LETTER I
        text: "\u{0066}\u{0069}",
        coverage: FontCoverage::Covered,
        note: "Roboto ligates f+i into one glyph, but parley keeps two clusters flagged \
               LigatureStart/LigatureComponent. The existing cursor_ligature_selection test pins \
               text_range() == 1..2 for the second. Two graphemes: two presses expected.",
    },
    CorpusEntry {
        id: "ligature_ffi",
        group: Group::Shaping,
        // LATIN SMALL LETTER F, LATIN SMALL LETTER F, LATIN SMALL LETTER I
        text: "\u{0066}\u{0066}\u{0069}",
        coverage: FontCoverage::Covered,
        note: "Three-char ligature: LigatureStart + two LigatureComponents. Exercises the \
               num_components loop in process_clusters.",
    },
    CorpusEntry {
        id: "ligature_fi_space",
        group: Group::Shaping,
        // LATIN SMALL LETTER F, LATIN SMALL LETTER I, SPACE, LATIN SMALL LETTER F
        // LATIN SMALL LETTER I
        text: "\u{0066}\u{0069}\u{0020}\u{0066}\u{0069}",
        coverage: FontCoverage::Covered,
        note: "The ligature-component loop in data.rs breaks on Whitespace::Space. Targets the \
               ClusterCoverage detector: if that path pushes fewer clusters than chars, \
               sum(text_len) != text.len().",
    },
    // ---- Format characters alone ----
    CorpusEntry {
        id: "zwj_alone",
        group: Group::Format,
        // LATIN SMALL LETTER A, ZERO WIDTH JOINER, LATIN SMALL LETTER B
        text: "\u{0061}\u{200D}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "ZWJ between non-pictographs: GB9 says do not break BEFORE it, so a+ZWJ is one \
               cluster and b is another. GB11 does not apply.",
    },
    CorpusEntry {
        id: "zwnj",
        group: Group::Format,
        // LATIN SMALL LETTER A, ZERO WIDTH NON-JOINER, LATIN SMALL LETTER B
        text: "\u{0061}\u{200C}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "ZWNJ is Extend, same structure as ZWJ here.",
    },
    CorpusEntry {
        id: "zwsp",
        group: Group::Format,
        // LATIN SMALL LETTER A, ZERO WIDTH SPACE, LATIN SMALL LETTER B
        text: "\u{0061}\u{200B}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "ZWSP is NOT Extend — its own cluster, and a line break opportunity with zero \
               advance. A cursor stop at a zero-width position is the stationary-cursor hazard.",
    },
    CorpusEntry {
        id: "arabic_number_sign_prepend",
        group: Group::Format,
        // ARABIC NUMBER SIGN, ARABIC-INDIC DIGIT ONE
        text: "\u{0600}\u{0661}",
        coverage: FontCoverage::Partial,
        note: "U+0600 is Prepend: GB9b says do not break AFTER it. The only rule that attaches \
               forwards, and the one most likely to be missed.",
    },
    CorpusEntry {
        id: "control_null",
        group: Group::Format,
        // LATIN SMALL LETTER A, NULL, LATIN SMALL LETTER B
        text: "\u{0061}\u{0000}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "GB4/GB5 break around Control. Also checks nothing treats NUL as a terminator.",
    },
    // ---- Whitespace, for word motion ----
    CorpusEntry {
        id: "words_spaces",
        group: Group::Whitespace,
        // LATIN SMALL LETTER A, LATIN SMALL LETTER B, SPACE, LATIN SMALL LETTER C
        // LATIN SMALL LETTER D
        text: "\u{0061}\u{0062}\u{0020}\u{0063}\u{0064}",
        coverage: FontCoverage::Covered,
        note: "Plain word motion control.",
    },
    CorpusEntry {
        id: "tab",
        group: Group::Whitespace,
        // LATIN SMALL LETTER A, CHARACTER TABULATION, LATIN SMALL LETTER B
        text: "\u{0061}\u{0009}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "Whitespace::Tab.",
    },
    CorpusEntry {
        id: "nbsp",
        group: Group::Whitespace,
        // LATIN SMALL LETTER A, NO-BREAK SPACE, LATIN SMALL LETTER B
        text: "\u{0061}\u{00A0}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "NBSP is Whitespace::NoBreakSpace. is_space_or_nbsp gates word motion, so the word \
               ops should treat it differently from a plain space.",
    },
    // ---- Bidi ----
    CorpusEntry {
        id: "bidi_ltr_rtl_ltr",
        group: Group::Bidi,
        // LATIN SMALL LETTER A, LATIN SMALL LETTER B, ARABIC LETTER BEH
        // ARABIC LETTER TEH MARBUTA, LATIN SMALL LETTER C, LATIN SMALL LETTER D
        text: "\u{0061}\u{0062}\u{0628}\u{0629}\u{0063}\u{0064}",
        coverage: FontCoverage::Covered,
        note: "Latin/Arabic/Latin. Visual and logical order diverge, so next_visual is expected to \
               be non-involutive at the run boundaries — a legitimate NotInvolutive firing to pin \
               rather than fix.",
    },
    CorpusEntry {
        id: "bidi_rtl_only",
        group: Group::Bidi,
        // ARABIC LETTER BEH, ARABIC LETTER TEH MARBUTA, ARABIC LETTER TEH
        text: "\u{0628}\u{0629}\u{062A}",
        coverage: FontCoverage::Covered,
        note: "Pure RTL. next_visual must still move toward the text end.",
    },
    CorpusEntry {
        id: "bidi_rtl_with_combining",
        group: Group::Bidi,
        // LATIN SMALL LETTER A, ARABIC LETTER BEH, ARABIC FATHA, LATIN SMALL LETTER B
        text: "\u{0061}\u{0628}\u{064E}\u{0062}",
        coverage: FontCoverage::Covered,
        note: "A multi-codepoint grapheme inside an RTL run inside LTR text. A grapheme is \
               contiguous and single-direction so visual motion and grapheme granularity SHOULD \
               compose — the obvious review question for issue #694, made measurable.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_name;
    use crate::util::TestEnv;
    use std::collections::HashSet;
    use std::fmt::Write;

    /// Every `text:` literal must be pure ASCII, i.e. written only as `\u{...}` escapes.
    ///
    /// Scoped to the literals rather than the whole file: prose may contain an em-dash, because
    /// prose cannot corrupt a measurement. A literal can. An escape is ASCII by construction, so
    /// this also means no editor, terminal or normalization pass can silently alter what is tested
    /// — the bytes on disk *are* the codepoints.
    #[test]
    fn traversal_corpus_source_is_ascii_escaped() {
        const SRC: &str = include_str!("corpus.rs");
        let mut checked = 0;
        for (i, line) in SRC.lines().enumerate() {
            if !line.trim_start().starts_with("text: \"") {
                continue;
            }
            checked += 1;
            assert!(
                line.is_ascii(),
                "corpus.rs:{}: non-ASCII text literal. Corpus text must use \\u{{...}} escapes so \
                 that no reviewer, editor or terminal can silently alter what is tested.\n  {}",
                i + 1,
                line
            );
        }
        // Guards the guard: if the literal format ever changes, this test must not silently pass
        // by matching nothing.
        assert_eq!(
            checked,
            CORPUS.len(),
            "expected one `text:` literal per corpus entry; the scan matched {checked} but CORPUS \
             has {}. Did the literal format change?",
            CORPUS.len()
        );
    }

    #[test]
    fn traversal_corpus_ids_are_unique() {
        let mut seen = HashSet::new();
        for e in CORPUS {
            assert!(seen.insert(e.id), "duplicate corpus id: {}", e.id);
        }
    }

    /// Every `corpus_id` named in `expectations.rs` must exist here, so a typo is a failure rather
    /// than a silently dropped row. `"*"` is the wildcard for cross-cutting per-op claims.
    #[test]
    fn traversal_expectations_reference_real_corpus_entries() {
        use crate::traversal::expectations::EXPECTATIONS;

        let ids: HashSet<&str> = CORPUS.iter().map(|e| e.id).collect();
        for e in EXPECTATIONS {
            assert!(
                e.corpus_id == "*" || ids.contains(e.corpus_id),
                "expectation names corpus id {:?} ({:?}/{:?}), which does not exist in CORPUS",
                e.corpus_id,
                e.op,
                e.authority
            );
        }
    }

    /// Emits the corpus inventory as a golden.
    ///
    /// Codepoints, never glyphs: a reader of this table cannot mistake a 7-codepoint ZWJ family
    /// for "one thing", which is the mistake that produced the wrong PR #693 rows.
    ///
    /// Doubles as the proof that the `PARLEY_TEST=accept` loop works for text.
    #[test]
    fn traversal_corpus_inventory() {
        let mut env = TestEnv::new(test_name!(), None);

        let mut out = String::new();
        writeln!(&mut out, "# Traversal corpus inventory").unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "Generated by `traversal_corpus_inventory`. Do not edit; run `PARLEY_TEST=accept cargo \
             test -p parley_tests traversal`."
        )
        .unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "`coverage` is whether the bundled `parley_dev` fonts can shape the entry, so a reader \
             can tell a measurement of parley from a measurement of `.notdef`."
        )
        .unwrap();
        writeln!(&mut out).unwrap();
        writeln!(
            &mut out,
            "| id | group | bytes | chars | codepoints | coverage |"
        )
        .unwrap();
        writeln!(&mut out, "|---|---|---|---|---|---|").unwrap();

        for e in CORPUS {
            let codepoints = if e.text.is_empty() {
                "(empty)".to_string()
            } else {
                e.text
                    .chars()
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            writeln!(
                &mut out,
                "| {} | {} | {} | {} | {} | {:?} |",
                e.id,
                e.group.slug(),
                e.text.len(),
                e.text.chars().count(),
                codepoints,
                e.coverage,
            )
            .unwrap();
        }

        writeln!(&mut out).unwrap();
        writeln!(&mut out, "{} entries.", CORPUS.len()).unwrap();

        env.check_text_snapshot("traversal/corpus_inventory.md", &out);
    }
}
