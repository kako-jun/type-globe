//! `jiwa_core` — text-reveal animation primitives for type-globe.
//!
//! In-tree for v0.2.0; designed to be liftable into a standalone `jiwa`
//! crate later (per `CLAUDE.md`'s **(b) 内製→後で切り出し** plan). The
//! module is intentionally UI-library-agnostic: it returns plain data
//! (graphemes + RGB triples) and the caller maps those to whatever
//! renderer it has — ratatui in our case.
//!
//! # Public surface (Issue #19)
//!
//! - [`reveal::RevealHandle`] / [`reveal::RevealOpts`] — typewriter +
//!   per-grapheme fade-in. Used by the Quiz question reveal.
//! - `pulse(...)` — the listening-mode `♪` pulse — is reserved for
//!   Issue #23 and not implemented yet. The naming is kept here so a
//!   future module sits next to `reveal` without churn.
//!
//! All time-bearing entry points take an explicit `now: Instant` so
//! tests can advance time deterministically without a global clock.

pub mod pulse;
pub mod reveal;

pub use reveal::{lerp_rgb, RevealHandle, RevealOpts, Rgb};
// `RevealedGrapheme` and `pulse::PulseFrame` are part of the public
// surface but constructed inside their respective `snapshot` methods;
// callers receive them by value, so they stay reachable via
// `reveal::RevealedGrapheme` / `pulse::PulseFrame`.
#[allow(unused_imports)]
pub use pulse::{PulseHandle, PulseOpts};
