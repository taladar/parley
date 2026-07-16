// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Automated detectors over the measurements.
//!
//! Each detector is a pure function over the primitives in `measure.rs`. Their output is itself a
//! golden, which is the point: a **known** defect is a committed, visible row that a reader can
//! see and argue with, and a **new** one is a golden diff that turns CI red. Neither is an
//! `#[ignore]`d test that quietly rots.
//!
//! # These are observations, not verdicts
//!
//! A firing detector does not mean parley is wrong. [`Detector::MidGrapheme`] fires on every
//! contested entry precisely because parley is codepoint-granular, which is a documented and
//! defensible position — UAX #29 permits it, and Android, Blink and Qt all do it for backspace.
//! The detector quantifies the behaviour; whether it is desirable is what `EXPECTATIONS.md`
//! discusses and what issue #694 has to decide.
//!
//! [`Detector::NotInvolutive`] is the sharpest case: it is *expected* to fire at bidi run
//! boundaries, where visual motion legitimately is not reversible. Recording it and pinning the
//! count is more useful than pretending it should be zero.

use std::collections::BTreeSet;
use std::fmt::Write;

use icu_properties::CodePointSetData;
use icu_properties::props::GraphemeExtend;

use crate::test_name;
use crate::util::TestEnv;

use super::corpus::CORPUS;
use super::env::{FontTier, TraversalEnv};
use super::measure::{
    ClusterTable, CursorDelta, Outcome, SelState, StepTable, Termination, Trajectory,
    char_boundaries,
};
use super::ops::Op;
use super::reference::egc_boundaries;

const TIER: FontTier = FontTier::Bundled;

/// What was detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Detector {
    /// A caret position that is not an extended grapheme cluster boundary.
    ///
    /// The quantification of issue #694: how many places the caret can land inside a
    /// user-perceived character.
    MidGrapheme,
    /// A motion step that changed nothing, away from the edge it should stop at.
    ///
    /// The Blink/Burmese "cursor sometimes appears stationary" hazard.
    StationaryStep,
    /// A motion step that changed only the affinity, not the index.
    ///
    /// Invisible in a table of indices, but it is what "two presses to cross one thing" looks like.
    AffinityOnlyFlip,
    /// `previous(next(i)) != i`. Expected at bidi boundaries; pinned rather than assumed zero.
    NotInvolutive,
    /// `delete` at `i` and `backdelete` at its far end do not remove the same bytes.
    ///
    /// A direct consequence of the `is_emoji`/`is_hard_line_break` fork existing in `backdelete`
    /// and not in `delete`.
    DeleteBackdeleteAsymmetry,
    /// An edit left the buffer starting with a `Grapheme_Extend` character.
    ///
    /// Not automatically a bug — deleting the base of `e` + combining acute legitimately leaves a
    /// lone mark, and that is exactly what a per-codepoint backspace is *for*. Recorded because it
    /// is the visible symptom users report.
    OrphanedCombiningMark,
    /// A trajectory removed a different multiset of characters than the text contained.
    LostOrDuplicatedChars,
    /// An edit step that did not shrink the buffer. The root cause of edit-loop divergence.
    NonShrinkingEdit,
    /// A selection index out of bounds, or not at a cluster start.
    SelectionInvariant,
    /// A trajectory that cycled, diverged, or stalled mid-text.
    BadTermination,
    /// An operation panicked.
    Panicked,
}

impl Detector {
    fn slug(self) -> &'static str {
        match self {
            Self::MidGrapheme => "MidGrapheme",
            Self::StationaryStep => "StationaryStep",
            Self::AffinityOnlyFlip => "AffinityOnlyFlip",
            Self::NotInvolutive => "NotInvolutive",
            Self::DeleteBackdeleteAsymmetry => "DeleteBackdeleteAsymmetry",
            Self::OrphanedCombiningMark => "OrphanedCombiningMark",
            Self::LostOrDuplicatedChars => "LostOrDuplicatedChars",
            Self::NonShrinkingEdit => "NonShrinkingEdit",
            Self::SelectionInvariant => "SelectionInvariant",
            Self::BadTermination => "BadTermination",
            Self::Panicked => "Panicked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Finding {
    corpus_id: &'static str,
    detector: Detector,
    op: &'static str,
    detail: String,
}

fn is_grapheme_extend(c: char) -> bool {
    CodePointSetData::new::<GraphemeExtend>().contains(c)
}

/// Runs every detector over every corpus entry and emits the findings table.
#[test]
fn traversal_findings() {
    let mut env = TestEnv::new(test_name!(), None);
    let mut t = TraversalEnv::new(TIER);
    let mut findings: Vec<Finding> = Vec::new();

    for e in CORPUS {
        if e.text.is_empty() {
            continue;
        }
        let egc: BTreeSet<usize> = egc_boundaries(e.text).into_iter().collect();
        let clusters = ClusterTable::measure(&t.layout(e.text));
        let cluster_starts: BTreeSet<usize> = clusters.boundaries().into_iter().collect();

        // -- MidGrapheme: caret positions reachable by motion that split a grapheme.
        for op in [Op::MoveRight, Op::MoveLeft] {
            let table = StepTable::measure(&mut t, e.text, op);
            let mut mid = Vec::new();
            for row in &table.rows {
                if let Outcome::Motion { to, .. } = &row.outcome
                    && !egc.contains(&to.focus)
                {
                    mid.push(to.focus);
                }
            }
            mid.sort_unstable();
            mid.dedup();
            if !mid.is_empty() {
                findings.push(Finding {
                    corpus_id: e.id,
                    detector: Detector::MidGrapheme,
                    op: op.slug(),
                    detail: format!(
                        "reaches {} position(s) inside a grapheme: {:?}",
                        mid.len(),
                        mid
                    ),
                });
            }
        }

        // -- Per-step motion anomalies.
        for op in Op::MOTION {
            let table = StepTable::measure(&mut t, e.text, *op);
            // A stationary step is legitimate only where the op comes to rest.
            let rest = Trajectory::measure(&mut t, e.text, *op)
                .steps
                .last()
                .map(|s| s.sel.focus);
            for row in &table.rows {
                match &row.outcome {
                    Outcome::Motion { to, delta } => {
                        match delta {
                            CursorDelta::Stationary => {
                                let at_rest = rest == Some(row.from_state.focus)
                                    || row.from_state.focus == 0
                                    || row.from_state.focus == e.text.len();
                                if !at_rest {
                                    findings.push(Finding {
                                        corpus_id: e.id,
                                        detector: Detector::StationaryStep,
                                        op: op.slug(),
                                        detail: format!(
                                            "no movement from {} (not an edge)",
                                            row.from_state.render()
                                        ),
                                    });
                                }
                            }
                            CursorDelta::AffinityOnlyFlip => findings.push(Finding {
                                corpus_id: e.id,
                                detector: Detector::AffinityOnlyFlip,
                                op: op.slug(),
                                detail: format!(
                                    "{} -> {} (index unchanged)",
                                    row.from_state.render(),
                                    to.render()
                                ),
                            }),
                            CursorDelta::Moved => {}
                        }
                        check_selection(
                            &mut findings,
                            e.id,
                            op.slug(),
                            *to,
                            e.text,
                            &cluster_starts,
                        );
                    }
                    Outcome::Panicked(msg) => findings.push(Finding {
                        corpus_id: e.id,
                        detector: Detector::Panicked,
                        op: op.slug(),
                        detail: format!("from {}: {msg}", row.from_state.render()),
                    }),
                    Outcome::Edit { .. } => {}
                }
            }
        }

        // -- NotInvolutive: move right then left should return to where it started.
        {
            let right = StepTable::measure(&mut t, e.text, Op::MoveRight);
            let mut broken = Vec::new();
            for row in &right.rows {
                let Outcome::Motion { to, delta } = &row.outcome else {
                    continue;
                };
                if *delta == CursorDelta::Stationary {
                    continue;
                }
                let back = Trajectory::measure_from(&mut t, e.text, Op::MoveLeft, to.focus);
                let landed = back.steps.first().map_or(to.focus, |s| s.sel.focus);
                if landed != row.from_state.focus {
                    broken.push(format!(
                        "{} ->{}-> {}",
                        row.from_state.focus, to.focus, landed
                    ));
                }
            }
            if !broken.is_empty() {
                findings.push(Finding {
                    corpus_id: e.id,
                    detector: Detector::NotInvolutive,
                    op: "move_right/move_left",
                    detail: broken.join(", "),
                });
            }
        }

        // -- Edit anomalies.
        for op in [Op::Backdelete, Op::Delete] {
            let traj = Trajectory::measure(&mut t, e.text, op);

            match &traj.termination {
                Termination::Converged => {}
                Termination::Panicked { at_step, msg } => findings.push(Finding {
                    corpus_id: e.id,
                    detector: Detector::Panicked,
                    op: op.slug(),
                    detail: format!("at step {at_step}: {msg}"),
                }),
                other => findings.push(Finding {
                    corpus_id: e.id,
                    detector: Detector::BadTermination,
                    op: op.slug(),
                    detail: format!("{other:?}"),
                }),
            }

            // NonShrinkingEdit: each press must consume something.
            let mut prev_len = e.text.chars().count();
            for (i, step) in traj.steps.iter().enumerate() {
                let len = if step.buffer == "(empty)" {
                    0
                } else {
                    step.buffer.split_whitespace().count()
                };
                if len >= prev_len {
                    findings.push(Finding {
                        corpus_id: e.id,
                        detector: Detector::NonShrinkingEdit,
                        op: op.slug(),
                        detail: format!("press {} did not shrink the buffer", i + 1),
                    });
                }
                prev_len = len;
            }

            // OrphanedCombiningMark: an intermediate state starting with a combining mark.
            for (i, step) in traj.steps.iter().enumerate() {
                if let Some(first) = decode_first(&step.buffer)
                    && is_grapheme_extend(first)
                {
                    findings.push(Finding {
                        corpus_id: e.id,
                        detector: Detector::OrphanedCombiningMark,
                        op: op.slug(),
                        detail: format!(
                            "after press {}, buffer starts with U+{:04X}",
                            i + 1,
                            first as u32
                        ),
                    });
                    break;
                }
            }

            // LostOrDuplicatedChars: the trajectory must consume exactly the original text.
            if traj.termination == Termination::Converged {
                let final_len = traj.steps.last().map_or(e.text.chars().count(), |s| {
                    if s.buffer == "(empty)" {
                        0
                    } else {
                        s.buffer.split_whitespace().count()
                    }
                });
                if final_len != 0 {
                    findings.push(Finding {
                        corpus_id: e.id,
                        detector: Detector::LostOrDuplicatedChars,
                        op: op.slug(),
                        detail: format!("converged with {final_len} codepoint(s) left"),
                    });
                }
            }
        }

        // -- DeleteBackdeleteAsymmetry: the two must agree on what lies between two boundaries.
        {
            let del = StepTable::measure(&mut t, e.text, Op::Delete);
            let back = StepTable::measure(&mut t, e.text, Op::Backdelete);
            let bounds = char_boundaries(e.text);
            for (i, row) in del.rows.iter().enumerate() {
                let Outcome::Edit { removed, .. } = &row.outcome else {
                    continue;
                };
                if removed.is_empty() || removed == "(empty)" {
                    continue;
                }
                let n = removed.split_whitespace().count();
                // The boundary `delete` consumed up to.
                let Some(end_idx) = bounds.iter().position(|b| *b == bounds[i]).map(|p| p + n)
                else {
                    continue;
                };
                let Some(back_row) = back.rows.get(end_idx) else {
                    continue;
                };
                let Outcome::Edit {
                    removed: back_removed,
                    ..
                } = &back_row.outcome
                else {
                    continue;
                };
                if back_removed != removed {
                    findings.push(Finding {
                        corpus_id: e.id,
                        detector: Detector::DeleteBackdeleteAsymmetry,
                        op: "delete/backdelete",
                        detail: format!(
                            "delete@{} removed [{}], backdelete@{} removed [{}]",
                            bounds[i], removed, bounds[end_idx], back_removed
                        ),
                    });
                }
            }
        }
    }

    findings.sort();
    findings.dedup();

    // -- Render.
    let mut out = String::new();
    writeln!(&mut out, "# Traversal findings").unwrap();
    writeln!(&mut out).unwrap();
    writeln!(
        &mut out,
        "Generated by `traversal_findings`. Do not edit; run `PARLEY_TEST=accept cargo test -p \
         parley_tests traversal`."
    )
    .unwrap();
    writeln!(&mut out).unwrap();
    writeln!(
        &mut out,
        "**A row here is an observation, not a verdict.** `MidGrapheme` fires on every contested \
         entry because parley is codepoint-granular — which UAX #29 permits and Android, Blink and \
         Qt all do for backspace. `NotInvolutive` is *expected* at bidi run boundaries, where \
         visual motion legitimately is not reversible. The value is in the counts being measured \
         and pinned rather than argued about."
    )
    .unwrap();
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "## Counts by detector").unwrap();
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "| detector | findings | entries |").unwrap();
    writeln!(&mut out, "|---|---|---|").unwrap();
    let mut detectors: Vec<Detector> = findings.iter().map(|f| f.detector).collect();
    detectors.sort();
    detectors.dedup();
    for d in &detectors {
        let rows: Vec<&Finding> = findings.iter().filter(|f| f.detector == *d).collect();
        let entries: BTreeSet<&str> = rows.iter().map(|f| f.corpus_id).collect();
        writeln!(
            &mut out,
            "| {} | {} | {} |",
            d.slug(),
            rows.len(),
            entries.len()
        )
        .unwrap();
    }
    // Detectors that found nothing are the interesting negative space.
    let silent: Vec<&str> = [
        Detector::MidGrapheme,
        Detector::StationaryStep,
        Detector::AffinityOnlyFlip,
        Detector::NotInvolutive,
        Detector::DeleteBackdeleteAsymmetry,
        Detector::OrphanedCombiningMark,
        Detector::LostOrDuplicatedChars,
        Detector::NonShrinkingEdit,
        Detector::SelectionInvariant,
        Detector::BadTermination,
        Detector::Panicked,
    ]
    .iter()
    .filter(|d| !detectors.contains(d))
    .map(|d| d.slug())
    .collect();
    writeln!(&mut out).unwrap();
    writeln!(
        &mut out,
        "Detectors that found nothing: {}.",
        if silent.is_empty() {
            String::from("(none)")
        } else {
            silent.join(", ")
        }
    )
    .unwrap();

    writeln!(&mut out).unwrap();
    writeln!(&mut out, "## Findings").unwrap();
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "| id | detector | op | detail |").unwrap();
    writeln!(&mut out, "|---|---|---|---|").unwrap();
    for f in &findings {
        writeln!(
            &mut out,
            "| {} | {} | {} | {} |",
            f.corpus_id,
            f.detector.slug(),
            f.op,
            f.detail
        )
        .unwrap();
    }
    writeln!(&mut out).unwrap();
    writeln!(&mut out, "{} findings total.", findings.len()).unwrap();

    env.check_text_snapshot("traversal/findings.md", &out);
}

fn check_selection(
    findings: &mut Vec<Finding>,
    id: &'static str,
    op: &'static str,
    sel: SelState,
    text: &str,
    cluster_starts: &BTreeSet<usize>,
) {
    for (name, idx) in [("anchor", sel.anchor), ("focus", sel.focus)] {
        if idx > text.len() {
            findings.push(Finding {
                corpus_id: id,
                detector: Detector::SelectionInvariant,
                op,
                detail: format!(
                    "{name} {idx} is past the end of a {}-byte buffer",
                    text.len()
                ),
            });
        } else if !text.is_char_boundary(idx) {
            findings.push(Finding {
                corpus_id: id,
                detector: Detector::SelectionInvariant,
                op,
                detail: format!("{name} {idx} is not a char boundary"),
            });
        } else if idx != text.len() && !cluster_starts.contains(&idx) {
            findings.push(Finding {
                corpus_id: id,
                detector: Detector::SelectionInvariant,
                op,
                detail: format!("{name} {idx} is not a cluster start"),
            });
        }
    }
}

/// First codepoint of a rendered `U+XXXX ...` buffer.
fn decode_first(rendered: &str) -> Option<char> {
    let first = rendered.split_whitespace().next()?;
    let hex = first.strip_prefix("U+")?;
    char::from_u32(u32::from_str_radix(hex, 16).ok()?)
}
