# Text traversal: what the authorities actually say

Raw research material for the traversal fact-finding harness. Every claim below carries a URL and a
verbatim quote. **Do not paraphrase when citing these** — the single most important distinction in
this document is between a normative "shall" and a non-normative "might", and a paraphrase is how a
"might" becomes a "must".

Machine-readable form: `expectations.rs` in this directory. That file is the one that feeds the
generated tables; this file exists to explain *why* each row says what it says, and to record the
things that do not fit in a table cell.

> **Provenance note.** This document was assembled with LLM assistance from web research.
> Linebender's [LLM policy](https://linebender.org/wiki/llm_policy/) forbids AI-generated PR/issue
> text and AI-generated analyses in discussion spaces. Nothing here is intended to be pasted
> anywhere. It is input for a human to write from, and the disclosure belongs up front in whatever
> they write.

---

## 0. The one-paragraph summary

There is no single correct granularity, and the Unicode Consortium's own working documents say so.
The consensus that *does* exist is a split by operation:

- **Emoji sequences** — whole cluster, for every operation. Unanimous, and it is a **conformance
  requirement** (UTS #51 C2b).
- **Cursor movement, selection, forward-delete** — grapheme cluster. Near-unanimous, with India
  wanting coarser and SE Asia wanting finer.
- **Backspace** — genuinely contested. Android/Blink/Qt and every browser r12a tested say *code
  point*. CodeMirror 6 says *grapheme*. GTK/Pango says *it depends on the script*.

So `backspace ≠ forward-delete ≠ arrows` is the cross-standard, cross-engine pattern. Any design
that gives all three the same granularity is knowingly wrong for at least one script.

---

## 1. UAX #29 — Unicode Text Segmentation

<https://www.unicode.org/reports/tr29/>

### The rules

```
GB1    sot ÷ Any                          GB2    Any ÷ eot
GB3    CR × LF                            GB4    (Control | CR | LF) ÷
GB5    ÷ (Control | CR | LF)
GB6    L × (L | V | LV | LVT)             GB7    (LV | V) × (V | T)
GB8    (LVT | T) × T                      GB9    × (Extend | ZWJ)
GB9a   × SpacingMark   [extended only]    GB9b   Prepend ×   [extended only]
GB9c   \p{InCB=Consonant} [\p{InCB=Extend} \p{InCB=Linker}]* \p{InCB=Linker}
       [\p{InCB=Extend} \p{InCB=Linker}]* × \p{InCB=Consonant}   [extended only]
GB11   \p{Extended_Pictographic} Extend* ZWJ × \p{Extended_Pictographic}
GB12   sot (RI RI)* RI × RI               GB13   [^RI] (RI RI)* RI × RI
GB999  Any ÷ Any
```

GB10 no longer exists (removed in Unicode 11 when `Extended_Pictographic` replaced
`E_Base`/`E_Modifier`). GB9c is the Unicode 15.1 Indic conjunct addition.

### ⭐ The passage that decides the backspace argument

This is the single most load-bearing quote in the whole document, and it is **non-normative prose in
an explanatory paragraph**, not a rule:

> "This document defines a default specification for grapheme clusters. It may be customized for
> particular languages, operations, or other situations. For example, arrow key movement could be
> tailored by language, or could use knowledge specific to particular fonts to move in a more
> granular manner, in circumstances where it would be useful to edit individual components. This
> could apply, for example, to the complex editorial requirements for the Northern Thai script Tai
> Tham (Lanna). **Similarly, editing a grapheme cluster element by element may be preferable in some
> circumstances. For example, on a given system the _backspace key_ might delete by code point,
> while the _delete key_ may delete an entire cluster.** Moreover, there is not a one-to-one
> relationship between grapheme clusters and keys on a keyboard. A single key on a keyboard may
> correspond to a whole grapheme cluster, a part of a grapheme cluster, or a sequence of more than
> one grapheme cluster. **Grapheme clusters can only provide an approximation of where to put
> cursors.** Detailed cursor placement depends on the text editing framework."

Note the modality: *"may be"*, *"might"*, *"may"*. **UAX #29 permits both and requires neither.**
Anyone citing this as mandating per-codepoint backspace is overreading it; anyone citing UAX #29 as
mandating grapheme backspace is overreading it in the other direction.

Also:

> "For cursor placement, grapheme clusters boundaries can only supply an approximate guide for
> cursor placement using least-common-denominator fonts for the script."

### What it *does* say about the ops

> "Grapheme clusters can be treated as units, by default, for processes such as the formatting of
> drop caps, as well as the implementation of text selection, arrow key movement, forward deletion,
> and so forth. For example, when a grapheme cluster is represented internally by a character
> sequence consisting of base character + accents, then using the right arrow key would skip from
> the start of the base character to the end of the last accent."

**"forward deletion"** — not backward. The asymmetry is in the spec's own word choice.

### On the extended-vs-legacy choice

> "Extended grapheme clusters should be used in implementations in preference to legacy grapheme
> clusters, because they provide better results for Indic scripts such as Tamil or Devanagari **in
> which editing by orthographic syllable is typically preferred**."

And why aksara was not made the default:

> "such consonant cluster aksaras are not incorporated into default rules for extended grapheme
> clusters because **not all such sequences are considered single 'characters' by users**, and Indic
> scripts vary considerably in how they render such aksaras—some stacking them into consonant
> conjuncts and others stringing them out horizontally."

### Conformance clauses

**UAX29-C1** requires choosing C1-1 (the UCD rules) or **C1-2** (*"Declare the use of a profile …
with a precise specification of any changes"*). These constrain **boundary determination only** —
nothing in UAX #29 constrains what a key deletes. If parley deviates, C1-2 is the mechanism for
declaring it rather than deviating silently.

---

## 2. UTS #51 — Unicode Emoji

<https://www.unicode.org/reports/tr51/>

Far more prescriptive than UAX #29, and it points the other way.

### ⭐ C2b — the conformance clause

> "**C2b editing** | The implementation treats each of the characters and sequences in the specified
> set as an **indivisible unit for editing purposes (cursor movement, deletion, line breaking, and so
> on)**."

This is a **conformance clause**, it names *"deletion"* with **no backspace/delete distinction**, and
it is why every implementation surveyed agrees on the emoji rows. A pure per-codepoint backspace is
non-conformant to C2b for emoji.

### ED-17

> "Note that all emoji sequences are single grapheme clusters: there is never a grapheme cluster
> boundary within an emoji sequence. **This affects editing operations, such as cursor movement or
> deletion**, as well as word break, line break, and so on."

### Sequence forms

| form | shape |
|---|---|
| ZWJ | `emoji_zwj_element ( ZWJ emoji_zwj_element )+`, ZWJ = U+200D |
| flag | RI pair, U+1F1E6..U+1F1FF, parity per GB12/GB13 |
| modifier | `emoji_modifier_base emoji_modifier` |
| keycap | `[0-9#*] U+FE0F U+20E3` — **note the VS16 in the middle** |
| tag | tag_base + tag_spec (U+E0020..U+E007E) + U+E007F CANCEL TAG |

> "A text presentation selector applied to any element of an emoji ZWJ sequence breaks that
> sequence, preventing it from displaying as a single image."

**⚠ The direct contradiction, stated plainly**: UAX #29 says backspace *might* delete by code point;
UTS #51 C2b says emoji sequences are *indivisible for deletion*. For emoji, C2b wins in practice.

---

## 3. The xilem #303 chain — what raphlinus is pointing at

<https://github.com/linebender/xilem/pull/303> — *"Actually use the druid backspace logic"*,
DJMcNab, merged 2024-05-12.

The **entire** PR description:

> This is followup to #273.
> This code originally came from https://github.com/xi-editor/xi-editor/pull/837
> I've also brought the tests back

The citation added in the diff (`masonry/src/text2/backspace.rs`):

```rust
/// Logic adapted from Android and
/// https://github.com/xi-editor/xi-editor/pull/837
/// See links present in that PR for upstream Android Source
/// Matches Android Logic as at 2024-05-10
```

So the chain is **xilem #303 → xi-editor #837 → AOSP `BaseKeyListener.getOffsetForBackspaceKey`**.

### ⭐ Two facts most likely to be lost

1. **No Unicode standard is cited anywhere in the chain.** It is a port of a vendor heuristic. That
   is not a criticism — it is a widely-copied, well-tested heuristic — but it is not a standard, and
   it should not be described as one.
2. **xilem still carries xi's known deviation from Android.** xi-editor #837's author wrote: *"The
   only difference is in `editor.rs:474` because I haven't thought up a implementation of
   `UCharacter.getCombiningClass(codePoint) == 0`."* That check is still missing in xilem:
   ```rust
   State::BeforeVs => {
       if code_point.is_emoji() { ... } else {
           if !is_variation_selector(code_point) {
               //TODO: UCharacter.getCombiningClass(codePoint) == 0
               delete_code_point_count += 1;
           }
           state = State::Finished;
       }
   }
   ```
   Android and Blink both have it. So xilem deletes `base + combining_mark + VS` differently from
   Android when the char before the VS has nonzero combining class.

### The rule, stated plainly

> **Delete one complete emoji sequence if the caret is immediately after one; otherwise delete
> exactly one codepoint** (CRLF being the one non-emoji special case).

It is **not** a "finer than grapheme clusters" algorithm in general. For `e`+U+0301 it deletes only
the mark; for Hangul jamo, one jamo; for Devanagari, one codepoint. For every emoji sequence it
deletes the whole thing.

### AOSP source of truth

- <https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/text/method/BaseKeyListener.java>
- <https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/text/Emoji.java>
- <https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/tests/coretests/src/android/text/method/BackspaceTest.java>

14-state backward state machine over codepoints. States: `START, LF, ODD_NUMBERED_RIS,
EVEN_NUMBERED_RIS, BEFORE_KEYCAP, BEFORE_VS_AND_KEYCAP, BEFORE_EMOJI_MODIFIER,
BEFORE_VS_AND_EMOJI_MODIFIER, BEFORE_VS, BEFORE_EMOJI, BEFORE_ZWJ, BEFORE_VS_AND_ZWJ,
IN_TAG_SEQUENCE, FINISHED`.

Key transitions:

| state | rule |
|---|---|
| `START` | dispatch on the first codepoint read backwards; **else → FINISHED** (⇒ exactly one codepoint) |
| `LF` | if CR, `++deleteCharCount`. (CRLF atomic) |
| `ODD_NUMBERED_RIS` | if RIS: `+= 2`, → EVEN |
| `EVEN_NUMBERED_RIS` | if RIS: **`-= 2`**, → ODD (the parity trick: an even run means the pair before is complete, so back off) |
| `BEFORE_VS` | if emoji: `+= charCount` → BEFORE_EMOJI. **else if `!isVariationSelector(cp) && getCombiningClass(cp) == 0`: `+= charCount`.** |
| `BEFORE_ZWJ` | if emoji: `+= charCount + 1`; → BEFORE_EMOJI_MODIFIER or BEFORE_EMOJI |
| `IN_TAG_SEQUENCE` | if tag_spec: `+= 2`, stay. else if emoji: `+= charCount`, finish. else **`deleteCharCount = 2`** (delete only CANCEL TAG) |

**Android's forward delete is a different mechanism entirely:**

```java
private static int getOffsetForForwardDeleteKey(CharSequence text, int offset, Paint paint) {
    offset = paint.getTextRunCursor(text, offset, len, false, offset, Paint.CURSOR_AFTER);
    return adjustReplacementSpan(text, offset, false);
}
```

Font/textrun cursor, i.e. cluster-based. **Android deliberately makes BACKSPACE ≠ DELETE.**

Note `isEmojiModifierBase` carries a hardcoded exception:

```java
// These two characters were removed from Emoji_Modifier_Base in Emoji 4.0, but we need to
// keep them as emoji modifier bases since there are fonts and user-generated text out there
// that treats these as potential emoji bases.
if (c == 0x1F91D || c == 0x1F93C) { return true; }
```

### ⚠ Licence — this is why we reimplement rather than port

xilem, xi-editor, druid and AOSP are **Apache-2.0 only**. parley is **Apache-2.0 OR MIT** across all
132 `.rs` files. **Copying any of their code would break parley's dual licence.** The reference rule
in `expectations.rs` is a clean-room reimplementation from the state table above.

Separately: `xi_unicode`'s emoji tables are **frozen at Unicode 11 (2018)** and are not a usable
oracle regardless.

---

## 4. Blink — Android's machine, with three fixes

<https://chromium.googlesource.com/chromium/src/+/refs/heads/main/third_party/blink/renderer/core/editing/state_machines/backspace_state_machine.cc>

Same state names, ported to C++. Three deliberate divergences from AOSP, all of which the harness
must model separately:

1. **`Extended_Pictographic` instead of `Emoji`.** `IsExtendedPictographicGb11(cp)` vs Android's
   `Emoji.isEmoji(cp)`. Blink aligns with UAX #29 GB11; Android does not. Different results for
   `#`, `*`, digits (which are `Emoji=Yes` but `Extended_Pictographic=No`).
2. **Blink has the combining-class check** that xilem lacks:
   ```cpp
   case BackspaceState::kBeforeVS:
     if (IsExtendedPictographicGb11(code_point)) { ... }
     if (!u_hasBinaryProperty(code_point, UCHAR_VARIATION_SELECTOR) &&
         u_getCombiningClass(code_point) == 0)
       code_units_to_be_deleted_ += U16_LENGTH(code_point);
     return Finish();
   ```
3. **Proper tag-sequence base handling**, which Android TODOs (`// TODO: Need handle emoji variation
   selectors. Issue 35224297`).

---

## 5. GTK / Pango — the existing art for having *both*

This is the model worth studying, because it is the only one that decouples the two granularities
explicitly.

### `PangoLogAttr`

<https://docs.gtk.org/Pango/struct.LogAttr.html> ·
<https://gitlab.gnome.org/GNOME/pango/-/blob/main/pango/pango-break.h>

```c
 * @is_cursor_position: if set, cursor can appear in front of character.
 *   i.e. this is a grapheme boundary, or the first character in the text.
 *   This flag implements Unicode's
 *   [Grapheme Cluster Boundaries](http://www.unicode.org/reports/tr29/) semantics.
 ...
 * @backspace_deletes_character: if set, backspace deletes one character
 *   rather than the entire grapheme cluster. This field is only meaningful
 *   on grapheme boundaries (where @is_cursor_position is set). In some languages,
 *   the full grapheme (e.g. letter + diacritics) is considered a unit, while in
 *   others, each decomposed character in the grapheme is a unit. In the default
 *   implementation of [func@break], this bit is set on all grapheme boundaries
 *   except those following Latin, Cyrillic or Greek base characters.
```

**Two independent flags per position**: `is_cursor_position` (UAX #29) for movement,
`backspace_deletes_character` (script-dependent) for deletion. If parley ever wants to satisfy both
camps, this is the shape — and parley already computes the underlying data.

### ⚠ The Pango docs are stale — do not cite them

The docs say "except those following Latin, Cyrillic or Greek". The code
(<https://gitlab.gnome.org/GNOME/pango/-/blob/main/pango/break.c>) also excludes **Kana, Hangul,
Emoji and Math**:

```c
#define LATIN(wc)    (((wc) >= 0x0020 && (wc) <= 0x02AF) || ((wc) >= 0x1E00 && (wc) <= 0x1EFF))
#define CYRILLIC(wc) (((wc) >= 0x0400 && (wc) <= 0x052F))
#define GREEK(wc)    (((wc) >= 0x0370 && (wc) <= 0x3FF) || ((wc) >= 0x1F00 && (wc) <= 0x1FFF))
#define KANA(wc)     ((wc) >= 0x3040 && (wc) <= 0x30FF)
#define HANGUL(wc)   ((wc) >= 0xAC00 && (wc) <= 0xD7A3)
#define EMOJI(wc)    (_pango_Is_Emoji_Base_Character (wc))
#define MATH(wc)     ((wc) >= 0x2200 && (wc) <= 0x22FF)
#define BACKSPACE_DELETES_CHARACTER(wc) (!LATIN (wc) && !CYRILLIC (wc) && !GREEK (wc) \
                                         && !KANA (wc) && !HANGUL (wc) && !EMOJI (wc) && !MATH (wc))
```

plus:

```c
if (is_grapheme_boundary) {
    attrs[i].backspace_deletes_character = BACKSPACE_DELETES_CHARACTER (base_character);
    /* Dependent Vowels for Indic language */
    if (_pango_is_Virama (prev_wc) || _pango_is_Vowel_Dependent (prev_wc))
      attrs[i].backspace_deletes_character = TRUE;
}
```

### GTK's consumption is a genuinely different algorithm

`gtk_text_buffer_backspace()`
(<https://gitlab.gnome.org/GNOME/gtk/-/blob/main/gtk/gtktextbuffer.c>) always deletes the whole
grapheme cluster, then — if `backspace_deletes_character` — **NFD-normalises the deleted text and
re-inserts all but the last NFD character**. So "delete one character" means *one NFD-decomposed
character*, and it can turn a precomposed `é` into `e`.

<https://docs.gtk.org/gtk4/method.TextBuffer.backspace.html>:

> "In the normal case a single character will be deleted, but when combining accents are involved,
> more than one character can be deleted, and when precomposed character and accent combinations are
> involved, **less than one character will be deleted**."

**Concretely**, for `e`+U+0301: Android/Blink/xilem delete U+0301 leaving `e`; **GTK deletes the whole
`é`** (Latin base ⇒ flag false). For Devanagari `क`+`ि` GTK agrees with Android. So GTK agrees with
Android for Indic and disagrees for Latin/Cyrillic/Greek/Kana/Hangul/Emoji/Math.

---

## 6. Qt — explicit, recent, and states the rationale

<https://github.com/qt/qtbase/commit/21d6543cf11720579a207198f38d696e1d24529c> — *"QTextCursor:
special-case emoji in deletePreviousChar()"*, 2026-03-15. Fixes QTBUG-112693, QTBUG-133843,
QTBUG-67358.

The best single statement of the design trade-off I found anywhere:

> "**Unlike `deleteChar()` which uses `nextCursorPosition(SkipCharacters)` to delete the entire
> grapheme cluster, `deletePreviousChar()` intentionally deletes only one codepoint at a time for
> non-emoji clusters (e.g. Devanagari consonant+vowel), matching the behavior of other major text
> editors like Microsoft Word. This asymmetry is by design.**"

> "[ChangeLog][QtGui][QTextCursor] Fixed `deletePreviousChar()` (Backspace) to delete entire emoji
> sequences … **while preserving the existing per-codepoint deletion behavior for non-emoji grapheme
> clusters such as Devanagari or Thai.**"

Mechanism is different from Android's — it uses script itemisation (`Script_Emoji` from the emoji
segmenter), not a codepoint state machine — but the outcome rule is the same. On movement
(<https://doc.qt.io/qt-6/qtextcursor.html>):

> "In some other writing systems cursor movements are limited to 'clusters' (e.g. a syllable in
> Devanagari, or a base letter plus diacritics). Functions such as `movePosition()` and
> `deleteChar()` limit cursor movement to these valid positions."

---

## 7. CodeMirror 6 — reversed *toward* grapheme after surveying peers

<https://github.com/codemirror/dev/issues/516>. Marijn Haverbeke:

> "**This was intentional behavior, but on closer look it indeed seems that other editors don't work
> like that**, so attached patch drops it."

<https://github.com/codemirror/commands/commit/05812d896fcdcbdb22a2bca30ec83988d7373578> (2021-06-21):

> "Remove deleteCodePointBackward/Forward
> **BREAKING: Change default binding for backspace to `deleteCharBackward`**, drop
> `deleteCodePointBackward`/`Forward` from the library."

Now a pure "backspace = one extended grapheme cluster" implementation — i.e. exactly what PR #693
does, and the opposite of xilem #303. **This is the strongest precedent for our position.**

VS Code went the same way in 2021 (<https://github.com/microsoft/vscode/issues/99629> → PR #122991),
which means **xilem #303's stated motivation is stale**: DJMcNab wrote *"doing what vscode does and
deleting leading to `👨🏿‍❤️`"* in 2024, but VS Code had fixed that three years earlier. The VS Code
reporter cited UTS #51 C2b directly.

---

## 8. r12a's tested browser behaviour — the most valuable rows

Richard Ishida's orthography notes are **tested**, not asserted, which makes them the closest thing
to a measured cross-engine baseline.

| script | cursor | forward delete | backspace | source |
|---|---|---|---|---|
| Hindi | Gecko/Blink/WebKit: grapheme clusters incl. GB9c conjuncts | same as cursor | **code point, all browsers** | <https://r12a.github.io/scripts/deva/hi> |
| Arabic | all three: grapheme clusters — **2 steps past lam-alef** | same as cursor | **code point, all browsers** | <https://r12a.github.io/scripts/arab/arb.html> |
| Thai | grapheme clusters, **except 2 steps through ำ** (Gecko/WebKit) | same as cursor | **code point, all browsers** | <https://r12a.github.io/scripts/thai/th.html> |
| Khmer | **Gecko/Blink: 2+ steps per stack; WebKit: one jump** | same as cursor | **code point, all browsers** | <https://r12a.github.io/scripts/khmr/km.html> |
| Burmese | **Gecko: combining sequences; Blink: grapheme clusters but cursor sometimes appears stationary; WebKit: whole orthographic syllable** | same as cursor | Gecko/Blink code point; **WebKit deletes virama+ZWJ together** | <https://r12a.github.io/scripts/mymr/my.html> |

Two things worth extracting:

- **"cursor sometimes appears stationary"** (Blink/Burmese) is a real hazard for a layout library: a
  grapheme boundary at a zero-advance position looks like a dead key to the user. The harness has a
  `StationaryStep` detector precisely for this.
- **Khmer/Myanmar have no browser consensus at all** — three engines, three behaviours. There is
  nothing to copy.

On lam-alef specifically: it is a mandatory *rendering* ligature of two codepoints, no GB rule joins
them, and every engine gives **two cursor stops / two backspaces**. Settled and consistent.

---

## 9. Per-script notes

### Korean / Hangul — grapheme cluster is right; do not special-case

GB6–GB8 make `U+1100 U+1161 U+11AB` one cluster, identical to precomposed `한`. Lindenberg's
proposed replacement text for UAX #29 (L2/23-140, §1 below):

> "The default behavior for grapheme clusters leans towards the larger user-perceived units.
> **Hangul text is segmented into syllable blocks, not into jamo.**"

**On raphlinus's "Korean IMEs delete Hangul jamo-by-jamo":** true *during IME composition*, false for
committed text — and the distinction is architectural, not pedantic.

- Apple's Korean input guide makes it a **user preference**, scoped explicitly: *"To delete text
  letter by letter **while composing a syllable**, choose Jaso. To delete text by syllable, choose
  Gulja."* (<https://support.apple.com/guide/korean-input-method/welcome/mac>)
- Oracle/Solaris: *"The Backspace and Delete keys remove the last character (JaMo) **of the current
  syllable**."* (<https://docs.oracle.com/cd/E19253-01/817-2522/userkorinputmethod-33122/index.html>)
- W3C: *"When written in the precomposed form, each Korean character remains **atomic for all
  operations**. When composed from jamo, most systems allow backspacing into the character (while
  treating the character as atomic for selection and forward deletion)."*
  (<https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html>)

**The decisive point: jamo-wise backspace happens inside the IME's preedit buffer, which the IME
owns. parley never sees those keystrokes** — the IME swallows the backspace and replaces the
composition string via `setComposingText`/`NSTextInputClient`. By the time text reaches parley's
buffer it is committed.

Also: real Korean text is precomposed (KS X 1026-1 requires it for modern syllable blocks).
**Windows' jamo-splitting is the documented outlier that ICU, iOS and Android all declined to copy** —
Raymond Chen, *The sad story of Korean jamo*
(<https://devblogs.microsoft.com/oldnewthing/20201009-00/?p=104351>).

⚠ **This means the Hangul row may cut against raphlinus rather than for him.** It is one of only two
rows where xilem #303 disagrees with PR #693. Worth checking carefully before conceding it.

⚠ **Sourcing caveat**: I could not locate raphlinus's original statement (searched linebender/parley
issues, xi-editor, druid — it may be on Zulip, which is not publicly indexed). The claim's
*substance* is evaluated above; if it was made specifically about IME composition, it is correct and
the above argues against a strawman.

### Thai / Lao — ⚠ correcting a common misconception

**Thai vowel signs and tone marks ARE `Extend`** and DO group into one grapheme cluster. Verified
against <https://www.unicode.org/Public/UCD/latest/ucd/auxiliary/GraphemeBreakProperty.txt>:

```
0E31          ; Extend       # Mn       THAI CHARACTER MAI HAN-AKAT
0E33          ; SpacingMark  # Lo       THAI CHARACTER SARA AM
0E34..0E3A    ; Extend       # Mn   [7] THAI CHARACTER SARA I..THAI CHARACTER PHINTHU
0E47..0E4E    ; Extend       # Mn   [8] THAI CHARACTER MAITAIKHU..THAI CHARACTER YAMAKKAN
```

Three real nuances:
1. **U+0E33 SARA AM is `Lo` forced to `SpacingMark`** — the one documented legacy/extended
   difference for Thai. r12a: Gecko/WebKit *"take 2 steps to get through ำ"*.
2. **Leading vowels U+0E40–U+0E44 (เ แ โ ใ ไ) are absent from the file** → `GCB=Other` → each is its
   own cluster. Correct: they are stored in visual order and occupy their own cell.
3. Thai clusters ≈ the WTT "cell", so grapheme is right for arrows/forward-delete and wrong for
   backspace.

**WTT 2.0** (Theppitak Karoonboonyanan, NECTEC, <https://www.nectec.or.th/it-standards/thaistd.pdf>)
splits the three operations explicitly — and is the strongest source in this document:

> "WTT 2.0 has also specified the cursor movements and editing behavior of Thai text editors and word
> processors. **Cursor must be moved from cells to cells**, that is, all characters in other levels
> than the base line must be skipped. **Text deletion using the "Delete" key must also remove all
> characters in the current cell**, including the above, below and top symbols. Meanwhile,
> **character-by-character, right-to-left, removal is still possible by using the "Backspace" key**,
> where the order of removal is considered by the order they are stored."

Status — de facto, not de jure, but widely deployed:

> "Although not proclaimed as a standard, due to the wide cooperation among industrial companies,
> such as Digital, Sun, Microsoft and IBM, WTT 2.0 has been adopted in many systems … **This makes
> WTT 2.0 become de facto.**"

**User evidence that grapheme backspace is wrong for Thai:**
- Obsidian forum: *"The first press of backspace should reduce ที่ to ที, then again, to ท, before
  deleting the entire character."*
  (<https://forum.obsidian.md/t/backspace-behaviour-with-thai-could-be-other-languages-too/30514>)
- FUTO Android keyboard #1532: *"Deletion should remove only one logical unit at a time"* — expected
  `ไม่ → ไม → ไ → ""`. (<https://github.com/futo-org/android-keyboard/issues/1532>)

Work that example: `ไม่` = ไ(own cluster) + ม + ่(Extend) = **2 grapheme clusters**, but users want
**3 backspaces**. Grapheme-cluster backspace is demonstrably wrong for Thai.

### Devanagari / Indic

- **GB9c covers only six scripts**: **Bengali, Devanagari, Gujarati, Oriya, Telugu, Malayalam**.
  **Not** Tamil, Kannada, Gurmukhi, Khmer, Myanmar. (<https://www.w3.org/International/questions/qa-indic-graphemes>:
  *"**The problem remains for several other scripts.**"*) Unicode 17.0 adds Balinese and Javanese —
  still not Tamil/Kannada/Gurmukhi.
- **The unfixable part** — segmentation is font-dependent, which no codepoint algorithm can solve:
  > "There is no difference whatsoever in the underlying code point sequence, and yet the
  > segmentation behaviour has to be different. The only clue as to how to segment this sequence
  > comes in the visually rendered shapes."
- W3C on the concrete cost: positioning after the Hindi word यूनिकोड and pressing backspace *"requires
  7 key presses in order to erase the entire word as the characters are erased one Unicode code point
  at a time"*, while *"cursoring, selection, and forward deletion move over the pair as a single
  unit."*
- Tamil `கோ` (a **split** vowel, glyph on both sides of the base): *"cursoring, selection, and forward
  deletion move over the pair as a single unit. **Backspacing deletes the combining mark first.**"*
- The rationale for the asymmetry:
  > "One reason sometimes attributed for this behavior is that it allows characters that have been
  > 'built-up' using multiple keypresses or other input mechanisms to be corrected without retyping
  > the whole sequence."
  > "Removing the base character usually consumes any combining marks associated with it… Backspace,
  > meanwhile, can safely remove combining characters hanging from a given base character without
  > causing other characters in the character sequence to change meaning."

**The akshara push is live and governmental** — L2/26-061 (MeitY/C-DAC/BIS, UTC #185, 2025-10-29,
<https://www.unicode.org/L2/L2026/26061-discussion-points-akshar-utc185.pdf>) argues the Akshar is
*"the fundamental orthographic unit"* and lists **"Cursor / Caret - Movement"** among its use cases.

### Arabic / Hebrew

- **Harakat / niqqud / cantillation** are `Mn` → GB9 `× Extend` → part of the cluster. Cursor and
  forward-delete atomic; backspace peels them off individually.
- **Lam-alef `لا`**: two codepoints, no joining rule, **two presses**, all engines. Settled.
- **Tatweel U+0640** is `Lm`, a spacing letter — its own cluster, own stop, own backspace.
- Everyday Hebrew (*ktiv haser*) has no niqqud, so the issue is confined to religious/educational
  texts.

### Japanese / Vietnamese — non-issues

- Dakuten U+3099 is `Mn`/`Extend`/ccc=8, so か+U+3099 clusters like precomposed が. But precomposed is
  universal in practice; the decomposed form matters *"required in half-width kana"* only.
- IVS (U+E0100–U+E01EF) are `Extend` → one cluster. Clearly right: an IVS selects a glyph variant of
  a single kanji.
- Vietnamese is overwhelmingly NFC; Telex/VNI strip tones with a **dedicated key** (`Z`/`0`), not
  backspace, so there is no pressure for codepoint backspace.

### Khmer / Myanmar — grapheme is wrong in *both* directions

- Khmer COENG U+17D2 is an `Invisible_Stacker`; pre-15.1 rules break after it, so a stack is
  **multiple clusters with boundaries at invisible positions**. r12a: *"Where stacks appear, a
  typographic unit contains multiple grapheme clusters. The non-final grapheme clusters all end with
  17D2, **which is never visible**."*
- r12a on Burmese: *"**Grapheme clusters only equate to Burmese typographic units some of the
  time.**"*
- Typographic unit is `(Base Invisible_Stacker)* Grapheme_cluster` — **larger** than a grapheme
  cluster. And **15.1's GB9c deliberately excluded them**; Lindenberg: *"In Unicode 15.1, this is
  being corrected for six scripts, **while leaving the others broken**."*
- r12a also notes for Khmer that *"editorial operations … should never split stacked forms apart.
  **This may not apply, however, for some other operations such as cursor movement or backwards
  delete.**"*

### Tibetan — under-evidenced, do not design around it

r12a's Tibetan notes have **no** cursor/deletion section. Subjoined consonants (U+0F90–U+0FBC) are
`Extend`, so stacks *are* single clusters — better-behaved than Khmer/Myanmar. The tsheg-bar
(syllable delimited by U+0F0B) is word-like, arguably a ctrl+arrow concern rather than an arrow one.

---

## 10. The meta-sources: Unicode admitting the model does not fit

### L2/23-140 — Lindenberg, "Setting expectations for grapheme clusters"

<https://www.unicode.org/L2/L2023/23140-graphemes-expectations.pdf>, 2023-07-04. **This is proposed
replacement text for UAX #29 itself**, so it is close to normative intent.

> "In relation to grapheme clusters, the current wording of UAX 29 unfortunately **creates
> unrealistic expectations both for what default grapheme clusters are and what they can be used
> for.**"

Its model of user perception — **users perceive two levels, not one**:

> "Base-level marks that represent individual consonants, vowels, or other linguistic entities, and
> that typically are the units of text input. It's quite common for these base-level components to
> have names."
> "Two-dimensional arrangements of these base-level marks that are important units for rendering,
> line breaking, and editing."

On deletion:

> "When deleting characters, it seems reasonable to expect that the base-level units of text are
> deleted as atomic units … **This expectation is, however, contradicted both by the statement in
> UAX 29 that 'on a given system the backspace key might delete by code point' and by the current
> grapheme cluster breaks after viramas**, which in numerous scripts break apart two-code point
> sequences encoding conjunct forms such as Khmer coengs."

The conclusion, which is the one-liner for any design doc:

> "The phrase **'differently depending on the operation' seems essential. There's no single set of
> segmentation rules that works for all the use cases**."
> "It's not clear if there's any use case that's visible to end users for which grapheme clusters
> today work out of the box, without tailoring, across all of Unicode."

### ⭐ CLDR "Grapheme Usage" design proposal — the explicit per-op mapping

<https://cldr.unicode.org/development/development-process/design-proposals/grapheme-usage>

```xml
<grapheme-usage type="count">extended</grapheme-usage>      <!-- counting 'user characters' -->
<grapheme-usage type="drop-cap">legacy</grapheme-usage>
<grapheme-usage type="selection">aksara</grapheme-usage>    <!-- highlighting, keyboard arrows, cut&paste -->
<grapheme-usage type="backspace">codepoint</grapheme-usage> <!-- delete previous character -->
<grapheme-usage type="delete">extended</grapheme-usage>     <!-- delete next character -->
```

**Three different granularities for three operations**, tailorable per locale.

> ⚠ **This is a Draft design proposal that was never implemented. Do NOT cite it as a standard.**
> It is the most quotable source in this entire document and citing it as normative would be exactly
> the next wrong claim. It is evidence of intent.

### L2/11-114 — Hosken, and the conflict that has no default answer

<https://www.unicode.org/L2/L2011/11114-uax29-changes.pdf>

> "There are two approaches to grapheme clustering within the scripts of the Indic subcontinent and
> southeast Asia. The first is that typified within India, that **a cluster corresponds to an
> orthographic syllable** … The second, which is used in southeast Asia is that **wherever you can
> sensibly, visually, insert a cursor, that corresponds to a cluster break**."

And the coupling that makes this parley's problem:

> "**If UAX#29 says that a grapheme cluster break cannot occur in a text sequence, then neither can a
> cursor occur within that sequence.** By changing UAX#29, therefore, highly visible editing
> behaviours can change … **This highly visible regression (in their eyes) is cause for complaint.**"

He rejects both escape hatches:

> On per-language tailoring: *"far fewer implementations will support per language tailoring … the use
> of per language tailoring to resolve this issue is considered an unacceptable solution."*
> On legacy clusters: *"this means that the right behaviour is made non-default and the wrong
> behaviour is made default… This is also an unaccpetable solution."*

**India wants fewer cursor stops (akshara); SE Asia wants more (every visually-insertable point).
Same rule, opposite demands.**

### CSS Text 3 — "typographic character unit"

<https://www.w3.org/TR/css-text-3/#typographic-character-unit>

> "A **typographic character** represents a unit of the writing system … that is **indivisible with
> respect to a particular typographic operation**."
> "the relevant character unit depends on the operation … **the default rules are not always
> appropriate or ideal**—and is expected to **tailor them differently depending on the operation as
> needed**."

Says nothing about backspace/delete/cursor. Note it observes **Thai and Lao units for letter-spacing
may be *less* than grapheme clusters**.

### Wordingham, Unicode ML 2019 — the rationale, and the anger

<https://corp.unicode.org/pipermail/unicode/2019-October/008333.html>

> "**The compromise that has generally been reached is that 'delete' deletes a grapheme cluster and
> 'backspace' deletes a scalar value.** The rationale for this is that **backspace undoes the effect
> of a keystroke.**"

Reacting to a proposal that backspace delete grapheme clusters: *"my overwhelming reaction is one of
extreme anger"*. He notes emoji RI pairs as a case where per-codepoint backspace is essential — *"be
able to delete just the last if I made an error"*.

### The empirical counterweight — Wikimedia T53472

<https://phabricator.wikimedia.org/T53472> — *"VisualEditor: Backspace deletes combined character
clusters together with diacritics"*.

- Affected: Arabic, Hebrew, Devanagari, other Indic and SE Asian scripts.
- Expectation: backspace deletes combining marks **before** base characters.
- Resolution: reverted to UTF-16 code units; resolved 2016 noting **Indic scripts require separate
  deletion capability, while Latin/Greek/Cyrillic users are happy treating base+mark as a unit.**

**The same communities that want akshara *cursor movement* filed bugs against akshara *backspace*.**

---

## 11. Bidi caret — an unresolved platform split

<https://groups.google.com/a/chromium.org/g/blink-dev/c/Rm1CGy6RBAI> (Blink PSA, shipped M76):

| platform | approach |
|---|---|
| Windows, Android, Microsoft Word, Google Docs | **Logical** |
| macOS, WebKit, Firefox (configurable) | **Visual** |

Chrome switched visual→logical in M76 **for engine-internal reasons** — the visual implementation
relied on *"hacks with many bugs"* and on legacy `InlineBox` structures LayoutNG removed — not user
research.

**UAX #9 does not settle it**: it defines reordering for display only, with no standard inverse
visual→logical mapping. MediaWiki's bidi requirements confirm *"there is no standard on how such
mapping should be done"*.

⚠ **Direct conflict**: W3C i18n asserts arrow keys *should* be visual (*"The left arrow button on a
keyboard always moves the cursor one user-perceived character (grapheme) to the left … always refer
to the same **visual** direction"*), while shipped Chrome/Windows behaviour is logical. This is the
sharpest disagreement in the document, and it argues for a **policy knob, not a constant**.

---

## 12. Accessibility — no constraint, but a consistency requirement

AccessKit (<https://docs.rs/accesskit/latest/accesskit/struct.Node.html>):

> "A character is defined as the smallest unit of text that can be selected. This isn't necessarily a
> single Unicode scalar value (code point)."
> "…this information must be provided by the text editing implementation."

AccessKit **cannot compute `character_lengths`** from the text — the editor supplies it. UIA
(`TextUnit_Character`) and ATK/AT-SPI (`ATK_TEXT_BOUNDARY_CHAR`) delegate identically, defining
"character" circularly as "what the editor can select".

**So whatever parley picks *becomes* the accessibility answer.** No external constraint — only an
internal one: **the unit exposed to AccessKit must equal the unit the arrow keys use**, or
screen-reader navigation desyncs from the visible caret.

---

## 13. ⚠ Explicitly unverified — do not cite these

- **macOS / AppKit** (`NSTextView deleteBackward:`) — no authoritative Apple documentation or source
  found. Apple exposes `-[NSString rangeOfComposedCharacterSequenceAtIndex:]` (a *legacy*-grapheme-ish
  notion, not UAX #29 extended) but I could not confirm it backs `deleteBackward:`.
- **Windows / Notepad / RichEdit** — no authoritative Microsoft source found. The only attributable
  claim is second-hand, from Qt's commit message (*"matching the behavior of other major text editors
  like Microsoft Word"*) — Qt maintainers' assertion, not Microsoft's.
- **Firefox / Gecko** — inferred only: `nsIFrame::PeekOffset` with `eSelectCluster`, where "cluster"
  comes from `gfxTextRun`/`IsClusterStart` (font/shaping clusters), not directly from UAX #29. Same
  category of mechanism as Qt, and the same category PR #693 criticises for parley's `delete`.
- **Chromium issue 41094726** and the Discord thread — behind sign-in/403; content only via search
  snippets.
- **Korean-language user discussion** is thin; the strongest Korean evidence is vendor docs (Apple,
  Oracle) + KS X 1026-1. A native-speaker check on §9 would be worthwhile.

**Test these empirically rather than citing them.** Three of the five rows above are exactly the kind
of claim that produced the wrong PR #693 table.

---

## 14. Provisional measurement — the two `is_emoji` functions

Measured 2026-07-16 on `main` @ `7993939` with a throwaway probe over `ClusterInfo::is_emoji`
(what `backdelete` calls, via `Cluster::is_emoji`) vs `parley_data::Properties::is_emoji_or_pictograph`
(what `select_font` calls, via `CharCluster::is_emoji`). **Superseded by the harness golden once
step 4/7 land** — recorded here only because it corrects a live misconception.

| case | cp | `ClusterInfo::is_emoji` (backdelete) | `is_emoji_or_pictograph` (font sel) | agree |
|---|---|---|---|---|
| digit 5 | U+0035 | false | true | **NO** |
| number sign | U+0023 | false | true | **NO** |
| asterisk | U+002A | false | true | **NO** |
| black right triangle | U+25B6 | false | true | **NO** |
| party popper | U+1F389 | true | true | yes |
| eyes | U+1F440 | true | true | yes |
| check mark | U+2705 | true | true | yes |
| **cowboy hat face** | U+1F920 | **false** | true | **NO** |
| victory hand | U+270C | true | true | yes |
| heart | U+2764 | true | true | yes |
| VS16 | U+FE0F | false | false | yes |
| VS15 | U+FE0E | false | false | yes |
| ZWJ | U+200D | false | false | yes |
| **RI J** | U+1F1EF | **false** | true | **NO** |
| **RI P** | U+1F1F5 | **false** | true | **NO** |
| waving hand | U+1F44B | true | true | yes |
| skin tone 4 | U+1F3FD | true | true | yes |
| man | U+1F468 | true | true | yes |
| boy | U+1F466 | true | true | yes |
| keycap enclose | U+20E3 | false | false | yes |
| tag latin s | U+E0073 | false | false | yes |
| cancel tag | U+E007F | false | false | yes |
| black flag | U+1F3F4 | true | true | yes |
| latin a | U+0061 | false | false | yes |
| combining acute | U+0301 | false | false | yes |
| hangul L | U+1100 | false | false | yes |
| devanagari ka | U+0915 | false | false | yes |

**7 of 27 diverge.** Three consequences:

1. `ClusterInfo::is_emoji` (`parley_core/src/shape/data.rs:79`) is a **hardcoded 5-range match**
   (`0x1F600..=0x1F64F | 0x1F300..=0x1F5FF | 0x1F680..=0x1F6FF | 0x2600..=0x26FF | 0x2700..=0x27BF`)
   carrying `// TODO: Defer to ICU4X properties`. It is **not** the Emoji property.
2. **🤠 U+1F920 — one of the five bundled test emoji — is `false`.** So is every regional indicator.
   `backdelete`'s emoji special case does not fire for flags at all.
3. The claim *"`cluster.is_emoji` is the raw Emoji/Extended_Pictographic property — true for `5`,
   `#`, `▶`"* is **true of the font-selection function and false of the backdelete one**. Both
   functions are called `is_emoji`. Any statement about "parley's `is_emoji`" must name which.

---

## 15. Consolidated: presses to erase, backspace at end of buffer

⚠ **Every cell in this table is from a secondary source or reasoning, EXCEPT the `parley` column
which is not yet measured.** This table is a research summary, **not** a result. It is superseded by
`expectations__backdelete__*.md` once the harness lands. It is here so the shape of the disagreement
is visible while the harness is being built.

| input | UAX #29 EGC | UTS #51 C2b | Android / Blink / xilem | GTK/Pango | Qt (2026) | CodeMirror 6 |
|---|---|---|---|---|---|---|
| 👨‍👩‍👧‍👦 ZWJ family | 1 | **1 (required)** | 1 | 1 | 1 | 1 |
| 🇯🇵 RI flag | 1 | **1 (required)** | 1 | 1 | 1 | 1 |
| 👋🏽 skin tone | 1 | **1 (required)** | 1 | 1 | 1 | 1 |
| 1️⃣ keycap | 1 | **1 (required)** | 1 | 1 | 1 | 1 |
| 🏴󠁧󠁢󠁳󠁣󠁴󠁿 tag seq | 1 | **1 (required)** | 1 | 1 | 1 | 1 |
| ❤️ U+2764 U+FE0F | 1 | 1 | 1 | 1 | 1 | 1 |
| **`e` + U+0301** | 1 | n/a | **2** | **1** (Latin base) | **2** | **1** |
| **`क` + `ि`** | 1 | n/a | **2** | **2** (NFD−1) | **2** | **1** |
| **`각` jamo ×3** | 1 | n/a | **3** | **1** (Hangul excluded) | **3** | **1** |
| **Thai `ก` + `ำ`** | 1 | n/a | **2** | **2** | **2** | **1** |
| CRLF | 1 | n/a | 1 | 1 | 1 | 1 |

**The emoji block is unanimous. The non-emoji block is a 3-way split.** PR #693 sits with CodeMirror
6; xilem #303 sits with Android/Blink/Qt; GTK agrees with neither consistently.

The two rows where xilem #303 disagrees with PR #693 are **`e`+U+0301 and Hangul jamo** — and §9
argues the Hangul row favours PR #693.

---

## 16. Source index

**Standards** · [UAX #29](https://www.unicode.org/reports/tr29/) ·
[UTS #51](https://www.unicode.org/reports/tr51/) ·
[GraphemeBreakProperty.txt](https://www.unicode.org/Public/UCD/latest/ucd/auxiliary/GraphemeBreakProperty.txt) ·
[Unicode 15.1](https://www.unicode.org/versions/Unicode15.1.0/) ·
[CSS Text 3](https://www.w3.org/TR/css-text-3/#typographic-character-unit)

**UTC / CLDR** · [L2/23-140 Lindenberg](https://www.unicode.org/L2/L2023/23140-graphemes-expectations.pdf) ·
[L2/11-114 Hosken](https://www.unicode.org/L2/L2011/11114-uax29-changes.pdf) ·
[L2/26-061 Akshar](https://www.unicode.org/L2/L2026/26061-discussion-points-akshar-utc185.pdf) ·
[CLDR grapheme-usage](https://cldr.unicode.org/development/development-process/design-proposals/grapheme-usage) ·
[UTN #61 Khmer](https://www.unicode.org/notes/tn61/utn61-Khmer_Encoding_Structure_V2.pdf)

**W3C** · [Cursor Movement and Deletion](https://w3c.github.io/i18n-drafts/questions/qa-backwards-deletion.en.html) ·
[Typographic character units in complex scripts](https://www.w3.org/International/questions/qa-indic-graphemes) ·
[ilreq](https://w3c.github.io/ilreq/) · [alreq](https://w3c.github.io/alreq/) ·
[hlreq](https://w3c.github.io/hlreq/) · [elreq](https://w3c.github.io/elreq/) ·
[deva-gap](https://www.w3.org/TR/deva-gap/) · [language enablement index](https://www.w3.org/TR/typography/)

**r12a tested pages** · [Hindi](https://r12a.github.io/scripts/deva/hi) ·
[Arabic](https://r12a.github.io/scripts/arab/arb.html) · [Hebrew](https://r12a.github.io/scripts/hebr/he.html) ·
[Thai](https://r12a.github.io/scripts/thai/th.html) · [Khmer](https://r12a.github.io/scripts/khmr/km.html) ·
[Burmese](https://r12a.github.io/scripts/mymr/my.html) · [Tibetan](https://r12a.github.io/scripts/tibt/bo.html)

**Implementations** · [xilem #303](https://github.com/linebender/xilem/pull/303) ·
[xi-editor #837](https://github.com/xi-editor/xi-editor/pull/837) ·
[AOSP BaseKeyListener](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/text/method/BaseKeyListener.java) ·
[AOSP Emoji.java](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/text/Emoji.java) ·
[AOSP BackspaceTest](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/tests/coretests/src/android/text/method/BackspaceTest.java) ·
[Blink backspace_state_machine.cc](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/third_party/blink/renderer/core/editing/state_machines/backspace_state_machine.cc) ·
[PangoLogAttr](https://docs.gtk.org/Pango/struct.LogAttr.html) ·
[pango break.c](https://gitlab.gnome.org/GNOME/pango/-/blob/main/pango/break.c) ·
[gtk_text_buffer_backspace](https://docs.gtk.org/gtk4/method.TextBuffer.backspace.html) ·
[Qt commit 21d6543](https://github.com/qt/qtbase/commit/21d6543cf11720579a207198f38d696e1d24529c) ·
[QTextCursor docs](https://doc.qt.io/qt-6/qtextcursor.html) ·
[CodeMirror #516](https://github.com/codemirror/dev/issues/516) ·
[CodeMirror commit 05812d8](https://github.com/codemirror/commands/commit/05812d896fcdcbdb22a2bca30ec83988d7373578) ·
[VS Code #99629](https://github.com/microsoft/vscode/issues/99629) ·
[Flutter engine #17960](https://github.com/flutter/engine/pull/17960) ·
[ICU Boundary Analysis](https://unicode-org.github.io/icu/userguide/boundaryanalysis/)

**Bidi** · [Blink PSA visual→logical](https://groups.google.com/a/chromium.org/g/blink-dev/c/Rm1CGy6RBAI) ·
[Chromium RTL in WebKit](https://www.chromium.org/developers/rtl-in-webkit/) ·
[MediaWiki bidi requirements](https://www.mediawiki.org/wiki/Visual_editor/Bidirectional_text_requirements)

**User reports / discussion** · [Wikimedia T53472](https://phabricator.wikimedia.org/T53472) ·
[Wordingham 2019](https://corp.unicode.org/pipermail/unicode/2019-October/008333.html) ·
[Obsidian Thai](https://forum.obsidian.md/t/backspace-behaviour-with-thai-could-be-other-languages-too/30514) ·
[FUTO #1532 Thai](https://github.com/futo-org/android-keyboard/issues/1532) ·
[spacemacs #13303 Korean](https://github.com/syl20bnr/spacemacs/issues/13303)

**Korean** · [Apple Korean IM guide](https://support.apple.com/guide/korean-input-method/welcome/mac) ·
[Oracle Entering Korean Text](https://docs.oracle.com/cd/E19253-01/817-2522/userkorinputmethod-33122/index.html) ·
[Chen, The sad story of Korean jamo](https://devblogs.microsoft.com/oldnewthing/20201009-00/?p=104351) ·
[Sivonen, IME smoke testing](https://hsivonen.fi/ime/)

**Accessibility** · [AccessKit Node docs](https://docs.rs/accesskit/latest/accesskit/struct.Node.html)
