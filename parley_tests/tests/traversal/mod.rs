// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Text traversal fact-finding harness.
//!
//! Measures what parley's cursor motion, selection, and deletion operations actually do across a
//! corpus of multi-codepoint sequences, and compares that against what the standards and reference
//! implementations say they should do.
//!
//! # The rule
//!
//! **No number in any table this harness produces may be typed by a human.** Every cell is emitted
//! by running code and committed as a regenerable golden (`PARLEY_TEST=accept`).
//!
//! This exists because reading the code to predict its behavior produced wrong claims in a
//! submitted PR: `backdelete`'s `is_hard_line_break() || is_emoji()` special case *looks* like it
//! deletes a whole cluster and is a no-op, because a `ClusterData` is one `char`. Press counts
//! inferred from reading it were wrong for skin tone, Hangul and CRLF.
//!
//! # Layout
//!
//! - [`corpus`] — the text sequences under measurement, as `\u{...}` escapes.
//! - [`expectations`] — what the authorities say, as data with verbatim citations.
//!   `EXPECTATIONS.md` alongside it carries the prose research.

mod corpus;
mod expectations;
mod reference;
