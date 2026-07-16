// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! What the standards and reference implementations say text traversal *should* do.
//!
//! This is the "desired behavior" side of the traversal fact-finding harness. It carries no
//! measurements of parley — those come from `measure.rs` — and no opinions. Each [`Expectation`]
//! records that a named [`Authority`] made a specific claim about a specific (corpus entry,
//! operation) pair, with a verbatim quote and a URL.
//!
//! Prose background, per-script notes, and the sources that do not fit in a table cell live in
//! `EXPECTATIONS.md` next to this file.
//!
//! # Why quotes and not paraphrases
//!
//! The single most important distinction in this area is between a normative "shall" and a
//! non-normative "might". UAX #29 *permits* per-codepoint backspace in an explanatory paragraph
//! ("might", "may"); UTS #51 C2b *requires* whole-sequence editing in a conformance clause. A
//! paraphrase is how the first becomes the second. So [`Citation`] stores the source text verbatim
//! and [`Citation::modality`] records which kind of statement it was.
//!
//! # Two kinds of authority
//!
//! Authorities split by density, which is why they are recorded differently:
//!
//! - **Mechanical** ([`Authority::is_mechanical`]) — a total function over any input (ICU's
//!   grapheme segmenter, the AOSP backspace machine, Pango's script rule). These are *implemented*
//!   rather than cited, so their column is computed for every corpus entry at no authoring cost.
//!   Their [`Expectation`] rows here exist only to pin the *rule's* documented statement.
//! - **Cited** — a named source making a specific claim about a specific pair. Sparse and
//!   hand-authored. A pair with no citation is simply **absent**; there are no `Unspecified` rows.
//!
//! # Licence
//!
//! The mechanical rules are **clean-room reimplementations from documented behavior**. xilem,
//! xi-editor, druid and AOSP are Apache-2.0 **only**; parley is Apache-2.0 OR MIT. Their code must
//! not be copied into this workspace. See `EXPECTATIONS.md` §3.

#![allow(dead_code, reason = "consumed as the harness lands, step by step")]

/// A text traversal operation that at least one authority has an opinion about.
///
/// Deliberately smaller than parley's full API surface: line motion, hit-testing and AccessKit are
/// measured by the harness but no authority makes script-specific claims about them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum OpId {
    /// Backspace. The contested one.
    Backdelete,
    /// Forward delete.
    Delete,
    /// Caret motion toward the end of the text (`move_right` in LTR).
    NextVisual,
    /// Caret motion toward the start of the text (`move_left` in LTR).
    PreviousVisual,
    /// Shift+arrow selection extension.
    ExtendSelection,
}

impl OpId {
    /// Fixed iteration order. Golden tables must never iterate a hash map.
    pub(crate) const ALL: &'static [Self] = &[
        Self::Backdelete,
        Self::Delete,
        Self::NextVisual,
        Self::PreviousVisual,
        Self::ExtendSelection,
    ];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Backdelete => "backdelete",
            Self::Delete => "delete",
            Self::NextVisual => "next_visual",
            Self::PreviousVisual => "previous_visual",
            Self::ExtendSelection => "extend_selection",
        }
    }
}

/// Who is making the claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Authority {
    /// UAX #29 extended grapheme cluster boundaries.
    Uax29,
    /// UTS #51 conformance clause C2b. The only *requirement* in this list.
    Uts51C2b,
    /// AOSP `BaseKeyListener.getOffsetForBackspaceKey`, via xi-editor #837 and xilem #303.
    AndroidAosp,
    /// Blink's `BackspaceStateMachine` — AOSP's machine with three documented divergences.
    Blink,
    /// GTK/Pango's `backspace_deletes_character` + the NFD-reinsert step in `gtk_text_buffer_backspace`.
    GtkPango,
    /// Qt `QTextCursor::deletePreviousChar` / `deleteChar`.
    Qt,
    /// `CodeMirror` 6 `deleteCharBackward` (pure extended grapheme cluster).
    CodeMirror6,
    /// Richard Ishida's orthography notes — *tested* browser behavior, not assertion.
    R12aTested,
    /// CLDR `grapheme-usage` design proposal. **Draft, never implemented.**
    CldrDraft,
    /// WTT 2.0, the de-facto Thai standard.
    Wtt2,
    /// W3C i18n "Cursor Movement and Deletion of Unicode Text".
    W3cI18n,
}

impl Authority {
    pub(crate) const ALL: &'static [Self] = &[
        Self::Uax29,
        Self::Uts51C2b,
        Self::AndroidAosp,
        Self::Blink,
        Self::GtkPango,
        Self::Qt,
        Self::CodeMirror6,
        Self::R12aTested,
        Self::CldrDraft,
        Self::Wtt2,
        Self::W3cI18n,
    ];

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Uax29 => "uax29",
            Self::Uts51C2b => "uts51_c2b",
            Self::AndroidAosp => "aosp",
            Self::Blink => "blink",
            Self::GtkPango => "pango",
            Self::Qt => "qt",
            Self::CodeMirror6 => "codemirror6",
            Self::R12aTested => "r12a_tested",
            Self::CldrDraft => "cldr_draft",
            Self::Wtt2 => "wtt2",
            Self::W3cI18n => "w3c_i18n",
        }
    }

    /// Whether this authority's rule is a total function we implement and evaluate per entry,
    /// rather than a source we quote per pair.
    ///
    /// Mechanical authorities fill a dense column for free. Non-mechanical ones are cited sparsely.
    pub(crate) fn is_mechanical(self) -> bool {
        matches!(
            self,
            Self::Uax29
                | Self::AndroidAosp
                | Self::Blink
                | Self::GtkPango
                | Self::Qt
                | Self::CodeMirror6
                | Self::Uts51C2b
        )
    }

    /// Whether this authority binds. Exactly one does.
    ///
    /// Everything else is a permission, a draft, a de-facto convention, or an observation of what
    /// some implementation happens to do. Losing this distinction is how an argument gets built on
    /// a source that does not support it.
    pub(crate) fn is_normative(self) -> bool {
        matches!(self, Self::Uts51C2b)
    }
}

/// How strongly the source states its claim.
///
/// The reason this enum exists: UAX #29's backspace passage and UTS #51's C2b are routinely cited
/// as if they were the same kind of statement. They are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Modality {
    /// A conformance clause or rule. Binding on implementations claiming conformance.
    Required,
    /// Explicitly permitted, explicitly not required ("might", "may", "could").
    Permitted,
    /// Recommended but not required ("should").
    Recommended,
    /// A description of what some implementation does. Carries no authority beyond precedent.
    Observed,
    /// A proposal with no standing. Evidence of intent only.
    Draft,
}

/// Verbatim source text plus where it came from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Citation {
    /// Quoted **verbatim**. Never paraphrase — see the module docs.
    pub(crate) quote: &'static str,
    pub(crate) url: &'static str,
    pub(crate) modality: Modality,
}

/// What an authority says the outcome should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedOutcome {
    /// Exactly N applications of the op to traverse/erase the entry.
    Presses(u32),
    /// One whole extended grapheme cluster per application.
    WholeCluster,
    /// One codepoint per application.
    OneCodepoint,
    /// One whole *typographic unit*, which for Khmer/Myanmar is larger than a grapheme cluster.
    WholeTypographicUnit,
    /// The source addresses this pair but declines to prescribe.
    ///
    /// Distinct from absence: absence means nobody spoke, this means somebody explicitly refused
    /// to. UAX #29 on backspace is the motivating case.
    ExplicitlyUnspecified,
}

/// A single authority's claim about a single (corpus entry, operation) pair.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Expectation {
    /// Must match a `CorpusEntry::id`. Cross-checked by a test once `corpus.rs` lands.
    pub(crate) corpus_id: &'static str,
    pub(crate) op: OpId,
    pub(crate) authority: Authority,
    pub(crate) expected: ExpectedOutcome,
    pub(crate) citation: Citation,
    /// Anything a reader needs that the row cannot carry. Never a number.
    pub(crate) note: &'static str,
}

// -- Shared citations -------------------------------------------------------------------------
//
// Hoisted because they are quoted for many pairs and must stay byte-identical across rows.

/// UAX #29's explanatory paragraph. The whole backspace argument turns on its modality.
const UAX29_BACKSPACE_PERMISSION: Citation = Citation {
    quote: "Similarly, editing a grapheme cluster element by element may be preferable in some \
            circumstances. For example, on a given system the backspace key might delete by code \
            point, while the delete key may delete an entire cluster.",
    url: "https://www.unicode.org/reports/tr29/",
    modality: Modality::Permitted,
};

const UAX29_CURSOR_APPROXIMATION: Citation = Citation {
    quote: "Grapheme clusters can only provide an approximation of where to put cursors.",
    url: "https://www.unicode.org/reports/tr29/",
    modality: Modality::Permitted,
};

const UAX29_UNITS_FOR_OPS: Citation = Citation {
    quote: "Grapheme clusters can be treated as units, by default, for processes such as the \
            formatting of drop caps, as well as the implementation of text selection, arrow key \
            movement, forward deletion, and so forth.",
    url: "https://www.unicode.org/reports/tr29/",
    modality: Modality::Recommended,
};

/// The only binding statement in this file.
const UTS51_C2B: Citation = Citation {
    quote: "C2b editing | The implementation treats each of the characters and sequences in the \
            specified set as an indivisible unit for editing purposes (cursor movement, deletion, \
            line breaking, and so on).",
    url: "https://www.unicode.org/reports/tr51/",
    modality: Modality::Required,
};

const UTS51_ED17: Citation = Citation {
    quote: "Note that all emoji sequences are single grapheme clusters: there is never a grapheme \
            cluster boundary within an emoji sequence. This affects editing operations, such as \
            cursor movement or deletion, as well as word break, line break, and so on.",
    url: "https://www.unicode.org/reports/tr51/",
    modality: Modality::Required,
};

/// AOSP's rule, stated as the fall-through in `getOffsetForBackspaceKey`'s `STATE_START`.
const AOSP_ELSE_ONE_CODEPOINT: Citation = Citation {
    quote: "Returns the start offset to be deleted by a backspace key from the given offset.",
    url: "https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/\
          android/text/method/BaseKeyListener.java",
    modality: Modality::Observed,
};

/// Qt saying out loud what everyone else leaves implicit.
const QT_ASYMMETRY_BY_DESIGN: Citation = Citation {
    quote: "Unlike deleteChar() which uses nextCursorPosition(SkipCharacters) to delete the entire \
            grapheme cluster, deletePreviousChar() intentionally deletes only one codepoint at a \
            time for non-emoji clusters (e.g. Devanagari consonant+vowel), matching the behavior of \
            other major text editors like Microsoft Word. This asymmetry is by design.",
    url: "https://github.com/qt/qtbase/commit/21d6543cf11720579a207198f38d696e1d24529c",
    modality: Modality::Observed,
};

const CODEMIRROR_REVERSAL: Citation = Citation {
    quote: "This was intentional behavior, but on closer look it indeed seems that other editors \
            don't work like that, so attached patch drops it.",
    url: "https://github.com/codemirror/dev/issues/516",
    modality: Modality::Observed,
};

const CLDR_BACKSPACE_CODEPOINT: Citation = Citation {
    quote: "<grapheme-usage type=\"backspace\">codepoint</grapheme-usage> <!-- delete previous \
            character -->",
    url: "https://cldr.unicode.org/development/development-process/design-proposals/grapheme-usage",
    modality: Modality::Draft,
};

const CLDR_SELECTION_AKSARA: Citation = Citation {
    quote: "<grapheme-usage type=\"selection\">aksara</grapheme-usage> <!-- selection boundaries: \
            highlighting, keyboard arrows, cut&paste -->",
    url: "https://cldr.unicode.org/development/development-process/design-proposals/grapheme-usage",
    modality: Modality::Draft,
};

const CLDR_DELETE_EXTENDED: Citation = Citation {
    quote: "<grapheme-usage type=\"delete\">extended</grapheme-usage> <!-- delete next character -->",
    url: "https://cldr.unicode.org/development/development-process/design-proposals/grapheme-usage",
    modality: Modality::Draft,
};

const WTT2_CELLS_AND_BACKSPACE: Citation = Citation {
    quote: "Cursor must be moved from cells to cells, that is, all characters in other levels than \
            the base line must be skipped. Text deletion using the \"Delete\" key must also remove \
            all characters in the current cell, including the above, below and top symbols. \
            Meanwhile, character-by-character, right-to-left, removal is still possible by using \
            the \"Backspace\" key, where the order of removal is considered by the order they are \
            stored.",
    url: "https://www.nectec.or.th/it-standards/thaistd.pdf",
    modality: Modality::Observed,
};

const W3C_KOREAN_ATOMIC: Citation = Citation {
    quote: "When written in the precomposed form, each Korean character remains atomic for all \
            operations. When composed from jamo, most systems allow backspacing into the character \
            (while treating the character as atomic for selection and forward deletion).",
    url: "https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html",
    modality: Modality::Observed,
};

const W3C_BACKSPACE_RATIONALE: Citation = Citation {
    quote: "One reason sometimes attributed for this behavior is that it allows characters that \
            have been 'built-up' using multiple keypresses or other input mechanisms to be \
            corrected without retyping the whole sequence.",
    url: "https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html",
    modality: Modality::Observed,
};

const R12A_HINDI: Citation = Citation {
    quote: "Forward deletion works in the same way as cursor movement. The backspace key deletes \
            code point by code point, for all browsers.",
    url: "https://r12a.github.io/scripts/deva/hi",
    modality: Modality::Observed,
};

const R12A_ARABIC_CURSOR: Citation = Citation {
    quote: "Gecko, Blink, and WebKit browsers steps through the text using grapheme clusters. This \
            means that it takes 2 steps to get past the lam-alif ligature.",
    url: "https://r12a.github.io/scripts/arab/arb.html",
    modality: Modality::Observed,
};

const R12A_ARABIC_DELETE: Citation = Citation {
    quote: "Forward deletion works in the same way as cursor movement. The backspace key deletes \
            code point by code point, for all browsers.",
    url: "https://r12a.github.io/scripts/arab/arb.html",
    modality: Modality::Observed,
};

const R12A_THAI: Citation = Citation {
    quote: "Forward deletion works in the same way as cursor movement. The backspace key deletes \
            code point by code point, for all browsers.",
    url: "https://r12a.github.io/scripts/thai/th.html",
    modality: Modality::Observed,
};

const R12A_KHMER_STACKS: Citation = Citation {
    quote: "Where stacks appear, a typographic unit contains multiple grapheme clusters. The \
            non-final grapheme clusters all end with 17D2, which is never visible, and the final \
            grapheme cluster begins with a consonant.",
    url: "https://r12a.github.io/scripts/khmr/km.html",
    modality: Modality::Observed,
};

const R12A_BURMESE_UNITS: Citation = Citation {
    quote: "Grapheme clusters only equate to Burmese typographic units some of the time.",
    url: "https://r12a.github.io/scripts/mymr/my.html",
    modality: Modality::Observed,
};

// -- The expectations -------------------------------------------------------------------------
//
// Ordering is corpus_id, then op, then authority — the same order the renderer emits. Keep it.
//
// `corpus_id` values are cross-checked against `CORPUS` by a test once `corpus.rs` lands, so a
// typo here becomes a test failure rather than a silently missing row.

pub(crate) const EXPECTATIONS: &[Expectation] = &[
    // ---- Emoji: the unanimous block. UTS #51 C2b is a conformance requirement here. ----
    Expectation {
        corpus_id: "emoji_zwj_family",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "C2b names 'deletion' with no backspace/delete distinction. A per-codepoint \
               backspace is non-conformant to C2b for emoji.",
    },
    Expectation {
        corpus_id: "emoji_zwj_family",
        op: OpId::Delete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "",
    },
    Expectation {
        corpus_id: "emoji_zwj_family",
        op: OpId::NextVisual,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "C2b names cursor movement explicitly.",
    },
    Expectation {
        corpus_id: "emoji_zwj_family",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_ED17,
        note: "GB11 keeps Extended_Pictographic ZWJ sequences together.",
    },
    Expectation {
        corpus_id: "emoji_ri_flag_jp",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "GB12/GB13 pair RIs by parity. Note Wordingham dissents specifically here: he wants \
               to delete just the second RI after a typo.",
    },
    Expectation {
        corpus_id: "emoji_ri_flag_jp",
        op: OpId::NextVisual,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "",
    },
    Expectation {
        corpus_id: "emoji_skin_tone",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "emoji_modifier_sequence := emoji_modifier_base emoji_modifier.",
    },
    Expectation {
        corpus_id: "emoji_keycap_1",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "Keycap is base + VS16 + U+20E3 — note the VS16 in the middle; AOSP needs two states \
               for it.",
    },
    Expectation {
        corpus_id: "emoji_tag_flag_scotland",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "AOSP's IN_TAG_SEQUENCE deletes only U+E007F when the base is not found; Blink fixes \
               this. Divergence expected between the aosp and blink columns.",
    },
    Expectation {
        corpus_id: "emoji_heart_vs16",
        op: OpId::Backdelete,
        authority: Authority::Uts51C2b,
        expected: ExpectedOutcome::WholeCluster,
        citation: UTS51_C2B,
        note: "emoji_presentation_sequence. Also the subject of PR #692 on the font-selection side.",
    },
    // ---- The contested block: non-emoji multi-codepoint clusters. ----
    Expectation {
        corpus_id: "latin_e_combining_acute",
        op: OpId::Backdelete,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::ExplicitlyUnspecified,
        citation: UAX29_BACKSPACE_PERMISSION,
        note: "UAX #29 addresses backspace and explicitly declines to prescribe. 'might'/'may', in \
               a non-normative example paragraph. Permits both, requires neither.",
    },
    Expectation {
        corpus_id: "latin_e_combining_acute",
        op: OpId::Backdelete,
        authority: Authority::AndroidAosp,
        expected: ExpectedOutcome::OneCodepoint,
        citation: AOSP_ELSE_ONE_CODEPOINT,
        note: "STATE_START falls through to FINISHED for a non-emoji, non-LF, non-VS codepoint. \
               One of only two rows where xilem #303 disagrees with PR #693.",
    },
    Expectation {
        corpus_id: "latin_e_combining_acute",
        op: OpId::Backdelete,
        authority: Authority::GtkPango,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "In the default implementation of [func@break], this bit is set on all grapheme \
                    boundaries except those following Latin, Cyrillic or Greek base characters.",
            url: "https://docs.gtk.org/Pango/struct.LogAttr.html",
            modality: Modality::Observed,
        },
        note: "Latin base => backspace_deletes_character = FALSE => whole cluster. GTK disagrees \
               with Android here. NB these docs are STALE: break.c also excludes Kana, Hangul, \
               Emoji and Math. The mechanical pango column follows the code, not this quote.",
    },
    Expectation {
        corpus_id: "latin_e_combining_acute",
        op: OpId::Backdelete,
        authority: Authority::CodeMirror6,
        expected: ExpectedOutcome::WholeCluster,
        citation: CODEMIRROR_REVERSAL,
        note: "CodeMirror removed its codepoint path in 2021 after surveying peers. Strongest \
               precedent for PR #693's position.",
    },
    Expectation {
        corpus_id: "latin_e_combining_acute",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: UAX29_UNITS_FOR_OPS,
        note: "The spec's own worked example is base character + accents under the right arrow key.",
    },
    Expectation {
        corpus_id: "hangul_jamo_gak",
        op: OpId::Backdelete,
        authority: Authority::AndroidAosp,
        expected: ExpectedOutcome::OneCodepoint,
        citation: AOSP_ELSE_ONE_CODEPOINT,
        note: "The other row where xilem #303 disagrees with PR #693 — and the weakest one for \
               #303. See EXPECTATIONS.md §9: jamo-wise backspace lives in the IME preedit, which \
               parley never sees. Committed text is atomic.",
    },
    Expectation {
        corpus_id: "hangul_jamo_gak",
        op: OpId::Backdelete,
        authority: Authority::W3cI18n,
        expected: ExpectedOutcome::WholeCluster,
        citation: W3C_KOREAN_ATOMIC,
        note: "'most systems allow backspacing into the character' refers to jamo-composed text \
               under an IME. parley's buffer holds committed text.",
    },
    Expectation {
        corpus_id: "hangul_jamo_gak",
        op: OpId::Backdelete,
        authority: Authority::GtkPango,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "#define HANGUL(wc)   ((wc) >= 0xAC00 && (wc) <= 0xD7A3)",
            url: "https://gitlab.gnome.org/GNOME/pango/-/blob/main/pango/break.c",
            modality: Modality::Observed,
        },
        note: "Pango excludes Hangul from backspace_deletes_character. NB the macro covers \
               PRECOMPOSED syllables only (U+AC00..U+D7A3) — conjoining jamo U+1100.. are not in \
               the range, so the mechanical pango column may differ between the jamo and \
               precomposed corpus entries. Worth watching.",
    },
    Expectation {
        corpus_id: "hangul_jamo_gak",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "GB6 L × (L | V | LV | LVT)    GB7 (LV | V) × (V | T)    GB8 (LVT | T) × T",
            url: "https://www.unicode.org/reports/tr29/",
            modality: Modality::Required,
        },
        note: "Hangul is the script where UAX #29 works out of the box. Do not special-case it.",
    },
    Expectation {
        corpus_id: "devanagari_ka_i_matra",
        op: OpId::Backdelete,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::OneCodepoint,
        citation: R12A_HINDI,
        note: "Tested, not asserted — all three engines agree.",
    },
    Expectation {
        corpus_id: "devanagari_ka_i_matra",
        op: OpId::NextVisual,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "Gecko, Blink, and WebKit browsers step through the text using grapheme \
                    clusters, including the new rules around conjuncts (ie. they step over a stack \
                    and all associated combining characters in one jump).",
            url: "https://r12a.github.io/scripts/deva/hi",
            modality: Modality::Observed,
        },
        note: "The matra is a SpacingMark (GB9a) and reorders visually before the base. Cursor \
               steps over the pair; backspace peels the matra first.",
    },
    Expectation {
        corpus_id: "devanagari_ka_i_matra",
        op: OpId::Backdelete,
        authority: Authority::W3cI18n,
        expected: ExpectedOutcome::OneCodepoint,
        citation: W3C_BACKSPACE_RATIONALE,
        note: "The rationale row: backspace undoes a keystroke.",
    },
    Expectation {
        corpus_id: "devanagari_conjunct_ksha",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "GB9c \\p{InCB=Consonant} [ \\p{InCB=Extend} \\p{InCB=Linker} ]* \
                    \\p{InCB=Linker} [ \\p{InCB=Extend} \\p{InCB=Linker} ]* × \\p{InCB=Consonant}",
            url: "https://www.unicode.org/reports/tr29/",
            modality: Modality::Required,
        },
        note: "GB9c, Unicode 15.1. Covers ONLY Bengali, Devanagari, Gujarati, Oriya, Telugu, \
               Malayalam — not Tamil, Kannada, Gurmukhi, Khmer, Myanmar.",
    },
    Expectation {
        corpus_id: "tamil_ko_split_vowel",
        op: OpId::NextVisual,
        authority: Authority::W3cI18n,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "cursoring, selection, and forward deletion move over the pair as a single \
                    unit. Backspacing deletes the combining mark first.",
            url: "https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html",
            modality: Modality::Observed,
        },
        note: "A split vowel — glyph on both sides of the base. Strongest case for atomic cursor \
               with per-codepoint backspace. Tamil is NOT covered by GB9c.",
    },
    Expectation {
        corpus_id: "tamil_ko_split_vowel",
        op: OpId::Backdelete,
        authority: Authority::W3cI18n,
        expected: ExpectedOutcome::OneCodepoint,
        citation: Citation {
            quote: "cursoring, selection, and forward deletion move over the pair as a single \
                    unit. Backspacing deletes the combining mark first.",
            url: "https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html",
            modality: Modality::Observed,
        },
        note: "",
    },
    // ---- Thai: the strongest per-op split in the whole corpus. ----
    Expectation {
        corpus_id: "thai_mai_leading_vowel",
        op: OpId::Backdelete,
        authority: Authority::Wtt2,
        expected: ExpectedOutcome::OneCodepoint,
        citation: WTT2_CELLS_AND_BACKSPACE,
        note: "WTT 2.0 prescribes all three ops differently in one paragraph: cursor by cell, \
               Delete by cell, Backspace by stored character.",
    },
    Expectation {
        corpus_id: "thai_mai_leading_vowel",
        op: OpId::Delete,
        authority: Authority::Wtt2,
        expected: ExpectedOutcome::WholeCluster,
        citation: WTT2_CELLS_AND_BACKSPACE,
        note: "The Thai 'cell' is approximately the extended grapheme cluster.",
    },
    Expectation {
        corpus_id: "thai_mai_leading_vowel",
        op: OpId::NextVisual,
        authority: Authority::Wtt2,
        expected: ExpectedOutcome::WholeCluster,
        citation: WTT2_CELLS_AND_BACKSPACE,
        note: "Leading vowel U+0E44 is GCB=Other, so it is its own cluster — correct, it occupies \
               its own cell. This entry is 2 clusters but users want 3 backspaces (FUTO #1532).",
    },
    Expectation {
        corpus_id: "thai_mai_leading_vowel",
        op: OpId::Backdelete,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::OneCodepoint,
        citation: R12A_THAI,
        note: "Browsers independently converged on WTT 2.0's split.",
    },
    Expectation {
        corpus_id: "thai_sara_am",
        op: OpId::NextVisual,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::Presses(2),
        citation: Citation {
            quote: "Gecko and WebKit browsers step through the text using grapheme clusters with \
                    one exception: they take 2 steps to get through ำ.",
            url: "https://r12a.github.io/scripts/thai/th.html",
            modality: Modality::Observed,
        },
        note: "U+0E33 is Lo forced to SpacingMark — the one documented legacy/extended difference \
               for Thai, and a rare case where a tested browser row is a specific count.",
    },
    // ---- Khmer / Myanmar: grapheme is wrong in BOTH directions. ----
    Expectation {
        corpus_id: "khmer_coeng_stack",
        op: OpId::NextVisual,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::WholeTypographicUnit,
        citation: R12A_KHMER_STACKS,
        note: "No browser consensus: Gecko/Blink take 2+ steps per stack, WebKit one jump. \
               Boundaries land at INVISIBLE positions (after U+17D2). GB9c deliberately excluded \
               Khmer.",
    },
    Expectation {
        corpus_id: "khmer_coeng_stack",
        op: OpId::Backdelete,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::OneCodepoint,
        citation: Citation {
            quote: "Forward deletion works in the same way as cursor movement. The backspace key \
                    deletes code point by code point, for all browsers.",
            url: "https://r12a.github.io/scripts/khmr/km.html",
            modality: Modality::Observed,
        },
        note: "",
    },
    Expectation {
        corpus_id: "myanmar_stack",
        op: OpId::NextVisual,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::WholeTypographicUnit,
        citation: R12A_BURMESE_UNITS,
        note: "Three engines, three behaviours. Blink's 'cursor sometimes appears stationary' is a \
               real hazard for a layout library — see the StationaryStep detector.",
    },
    // ---- Arabic / Hebrew ----
    Expectation {
        corpus_id: "arabic_lam_alef",
        op: OpId::NextVisual,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::Presses(2),
        citation: R12A_ARABIC_CURSOR,
        note: "A mandatory RENDERING ligature of two codepoints. No GB rule joins them; every \
               engine gives two stops. Settled. Measurable in-tree today (Noto Kufi Arabic).",
    },
    Expectation {
        corpus_id: "arabic_lam_alef",
        op: OpId::Backdelete,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::Presses(2),
        citation: R12A_ARABIC_DELETE,
        note: "",
    },
    Expectation {
        corpus_id: "arabic_base_harakat",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: UAX29_UNITS_FOR_OPS,
        note: "Harakat are Mn => GB9 × Extend.",
    },
    Expectation {
        corpus_id: "arabic_base_harakat",
        op: OpId::Backdelete,
        authority: Authority::R12aTested,
        expected: ExpectedOutcome::OneCodepoint,
        citation: R12A_ARABIC_DELETE,
        note: "Wikimedia T53472 is the empirical counterweight: Arabic/Hebrew/Indic users filed a \
               bug AGAINST whole-cluster backspace.",
    },
    Expectation {
        corpus_id: "hebrew_base_niqqud",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: UAX29_UNITS_FOR_OPS,
        note: "Niqqud and cantillation are Mn => GB9. Measurable in-tree today (Arimo has Hebrew), \
               though Arimo is not in FONT_FAMILY_LIST so it must be requested explicitly.",
    },
    // ---- CRLF ----
    Expectation {
        corpus_id: "crlf",
        op: OpId::Backdelete,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "GB3 CR × LF    Do not break between a CR and LF.",
            url: "https://www.unicode.org/reports/tr29/",
            modality: Modality::Required,
        },
        note: "The one non-emoji case where AOSP also deletes the pair whole (STATE_LF). Upstream \
               #667 fixed CRLF for LINE BREAKING only, deliberately not for cursor/selection.",
    },
    Expectation {
        corpus_id: "crlf",
        op: OpId::Backdelete,
        authority: Authority::AndroidAosp,
        expected: ExpectedOutcome::WholeCluster,
        citation: Citation {
            quote: "case STATE_LF: if (codePoint == CR) { ++deleteCharCount; } break;",
            url: "https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/\
                  java/android/text/method/BaseKeyListener.java",
            modality: Modality::Observed,
        },
        note: "",
    },
    // ---- The cross-cutting per-op claims (no single corpus entry). ----
    Expectation {
        corpus_id: "*",
        op: OpId::Backdelete,
        authority: Authority::CldrDraft,
        expected: ExpectedOutcome::OneCodepoint,
        citation: CLDR_BACKSPACE_CODEPOINT,
        note: "DRAFT, NEVER IMPLEMENTED. The most quotable source in the whole body of research and \
               the easiest to miscite as normative. Evidence of intent only.",
    },
    Expectation {
        corpus_id: "*",
        op: OpId::Delete,
        authority: Authority::CldrDraft,
        expected: ExpectedOutcome::WholeCluster,
        citation: CLDR_DELETE_EXTENDED,
        note: "DRAFT. Three different granularities for three operations.",
    },
    Expectation {
        corpus_id: "*",
        op: OpId::ExtendSelection,
        authority: Authority::CldrDraft,
        expected: ExpectedOutcome::WholeTypographicUnit,
        citation: CLDR_SELECTION_AKSARA,
        note: "DRAFT. 'aksara' is COARSER than an extended grapheme cluster — the India position. \
               SE Asia wants the opposite (Hosken L2/11-114).",
    },
    Expectation {
        corpus_id: "*",
        op: OpId::Backdelete,
        authority: Authority::Qt,
        expected: ExpectedOutcome::OneCodepoint,
        citation: QT_ASYMMETRY_BY_DESIGN,
        note: "Qt states the asymmetry explicitly. Emoji are the exception, handled via script \
               itemisation rather than a codepoint state machine.",
    },
    Expectation {
        corpus_id: "*",
        op: OpId::NextVisual,
        authority: Authority::Uax29,
        expected: ExpectedOutcome::ExplicitlyUnspecified,
        citation: UAX29_CURSOR_APPROXIMATION,
        note: "Even for cursor motion, UAX #29 hedges. Every source still agrees motion should be \
               AT LEAST grapheme-granular; the dispute is only whether it should be coarser.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Guards the property the whole file exists for.
    ///
    /// A row with an empty quote or a non-URL is a row somebody wrote from memory, which is the
    /// failure mode this harness was built to eliminate.
    #[test]
    fn traversal_expectations_all_cite_a_real_source() {
        for e in EXPECTATIONS {
            assert!(
                !e.citation.quote.trim().is_empty(),
                "{}/{:?}/{:?}: empty quote",
                e.corpus_id,
                e.op,
                e.authority
            );
            assert!(
                e.citation.url.starts_with("https://"),
                "{}/{:?}/{:?}: url is not https",
                e.corpus_id,
                e.op,
                e.authority
            );
        }
    }

    /// The same (entry, op, authority) must not be claimed twice — a duplicate means two different
    /// quotes are competing and the renderer would silently pick one.
    #[test]
    fn traversal_expectations_have_no_duplicate_rows() {
        let mut seen = HashSet::new();
        for e in EXPECTATIONS {
            let key = (e.corpus_id, e.op, e.authority);
            assert!(
                seen.insert(key),
                "duplicate expectation: {}/{:?}/{:?}",
                e.corpus_id,
                e.op,
                e.authority
            );
        }
    }

    /// Exactly one authority binds. If this ever fails, someone has promoted a permission or a
    /// draft to a requirement, which is the specific error that loses the argument.
    #[test]
    fn traversal_expectations_only_uts51_is_normative() {
        let normative: Vec<_> = Authority::ALL.iter().filter(|a| a.is_normative()).collect();
        assert_eq!(
            normative,
            vec![&Authority::Uts51C2b],
            "UTS #51 C2b is the only conformance clause in this area; UAX #29's backspace passage \
             is a non-normative 'might' and CLDR's grapheme-usage is an unimplemented draft"
        );

        for e in EXPECTATIONS {
            if e.citation.modality == Modality::Required {
                assert!(
                    matches!(e.authority, Authority::Uts51C2b | Authority::Uax29),
                    "{:?} claims Required modality; only UTS #51 C2b and UAX #29's actual GB rules \
                     may",
                    e.authority
                );
            }
            if e.authority == Authority::CldrDraft {
                assert_eq!(
                    e.citation.modality,
                    Modality::Draft,
                    "CLDR grapheme-usage was never implemented and must never be cited as anything \
                     but a draft"
                );
            }
        }
    }

    /// UAX #29 must never be recorded as prescribing a backspace granularity. It explicitly
    /// declines to. This is the single most-overread passage in the area, in both directions.
    #[test]
    fn traversal_expectations_uax29_does_not_prescribe_backspace() {
        for e in EXPECTATIONS {
            if e.authority == Authority::Uax29 && e.op == OpId::Backdelete {
                assert!(
                    matches!(
                        e.expected,
                        ExpectedOutcome::ExplicitlyUnspecified | ExpectedOutcome::WholeCluster
                    ),
                    "{}: UAX #29 permits both backspace granularities and requires neither; only a \
                     concrete GB rule (e.g. GB3 CR × LF) may pin an outcome",
                    e.corpus_id
                );
            }
        }
    }
}
