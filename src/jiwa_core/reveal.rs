//! Typewriter + per-grapheme fade-in (#20, #21).
//!
//! Given a target `text` and a [`RevealOpts`], a [`RevealHandle`] reports
//! at any wall-clock instant which graphemes are currently visible and
//! what color each one should render at. It does no I/O, owns no thread,
//! and never sleeps: callers tick it at their existing redraw cadence.
//!
//! # Timing model
//!
//! - The first grapheme is visible the moment the reveal starts (`t=0`).
//! - Each subsequent grapheme appears `char_interval` later than the one
//!   before it (`t_i = i * char_interval`).
//! - When a grapheme appears it carries `fade_from` color; over the next
//!   `fade_duration` it linearly interpolates to `fade_to`. After that
//!   it stays at `fade_to`.
//!
//! # Why grapheme clusters
//!
//! Iterating Japanese text by `char` happens to work for kanji because
//! one kanji = one Unicode scalar, but combining marks (e.g. `é` written
//! as `e` + U+0301) and ZWJ emoji sequences (`👨‍👩‍👧‍👦`) would split
//! visually-single characters into multiple frames. `unicode-segmentation`
//! gives us proper extended grapheme clusters per UAX #29.
//!
//! # Concurrency note
//!
//! Per `docs/spec.md` players must be able to type *during* the reveal
//! ("先打ち可能"). That contract is satisfied by the surrounding event
//! loop (`crossterm::event::poll(TICK)`), not by this module — the
//! reveal layer is a pure function of (time, text, opts).

use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation;

/// 24-bit RGB triple. Not a wrapper around `crossterm::Color` /
/// `ratatui::Color` on purpose — the renderer maps to whatever it
/// already uses. Keeps this module liftable into a standalone crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// One grapheme as it appears at a particular instant during the reveal.
#[derive(Debug, Clone, PartialEq)]
pub struct RevealedGrapheme {
    /// The actual text segment the renderer should draw.
    pub text: String,
    /// Foreground color at the snapshot time.
    pub color: Rgb,
    /// Linear progress from 0.0 (just appeared, color = `fade_from`) to
    /// 1.0 (fade complete, color = `fade_to`).
    pub progress: f32,
}

/// Tunables for one reveal animation.
#[derive(Debug, Clone, Copy)]
pub struct RevealOpts {
    /// Time between successive grapheme appearances ("typewriter speed").
    pub char_interval: Duration,
    /// How long each grapheme spends fading from `fade_from` to `fade_to`.
    pub fade_duration: Duration,
    pub fade_from: Rgb,
    pub fade_to: Rgb,
}

impl RevealOpts {
    /// Defaults tuned for Quiz-mode question text on a typical terminal:
    /// ~25 chars/sec typewriter, ~180 ms fade per grapheme, dim grey →
    /// white.
    pub const fn default_quiz() -> Self {
        Self {
            char_interval: Duration::from_millis(40),
            fade_duration: Duration::from_millis(180),
            fade_from: Rgb(60, 60, 60),
            fade_to: Rgb(255, 255, 255),
        }
    }
}

impl Default for RevealOpts {
    fn default() -> Self {
        Self::default_quiz()
    }
}

/// One in-flight reveal. Cheap to construct (`O(graphemes)`); tick by
/// calling [`RevealHandle::snapshot`] each frame.
#[derive(Debug)]
pub struct RevealHandle {
    graphemes: Vec<String>,
    started_at: Instant,
    opts: RevealOpts,
}

impl RevealHandle {
    /// Start a reveal anchored to `now`. Use [`RevealHandle::start`] in
    /// production code; tests should pass an explicit `now` so they can
    /// step time without a sleep.
    pub fn start_at(text: &str, opts: RevealOpts, now: Instant) -> Self {
        let graphemes = text.graphemes(true).map(|g| g.to_string()).collect();
        Self {
            graphemes,
            started_at: now,
            opts,
        }
    }

    /// Convenience for production callers: anchors at `Instant::now()`.
    pub fn start(text: &str, opts: RevealOpts) -> Self {
        Self::start_at(text, opts, Instant::now())
    }

    /// Total grapheme count of the source text.
    ///
    /// Public API surface for callers that want to size a buffer or
    /// reason about the reveal externally; not yet used by the bin
    /// itself, hence the `allow(dead_code)`.
    #[allow(dead_code)]
    pub fn total_graphemes(&self) -> usize {
        self.graphemes.len()
    }

    /// How many graphemes are visible at `now` (regardless of whether
    /// they have finished fading).
    pub fn visible_count(&self, now: Instant) -> usize {
        if self.graphemes.is_empty() {
            return 0;
        }
        let interval_nanos = self.opts.char_interval.as_nanos().max(1);
        let elapsed_nanos = now.saturating_duration_since(self.started_at).as_nanos();
        // First grapheme is visible at t=0 → +1 against the floor div.
        let by_time = (elapsed_nanos / interval_nanos) as usize + 1;
        by_time.min(self.graphemes.len())
    }

    /// True once every grapheme has both appeared *and* finished fading.
    ///
    /// Will be the trigger for #22's concurrent-input handover and for
    /// `pulse` reset logic in #23. Exposed publicly now for that
    /// downstream wiring; the binary itself doesn't poll it yet.
    #[allow(dead_code)]
    pub fn is_done(&self, now: Instant) -> bool {
        if self.graphemes.is_empty() {
            return true;
        }
        let total_appearance =
            self.opts.char_interval * (self.graphemes.len().saturating_sub(1) as u32);
        let total_runtime = total_appearance + self.opts.fade_duration;
        now.saturating_duration_since(self.started_at) >= total_runtime
    }

    /// Snapshot the reveal at `now`. Hidden graphemes are simply absent
    /// from the returned vec — the caller appends a cursor / placeholder
    /// itself if it wants one.
    pub fn snapshot(&self, now: Instant) -> Vec<RevealedGrapheme> {
        let visible = self.visible_count(now);
        let elapsed = now.saturating_duration_since(self.started_at);
        let mut out = Vec::with_capacity(visible);
        for i in 0..visible {
            let appearance = self.opts.char_interval * (i as u32);
            let age = elapsed.saturating_sub(appearance);
            let progress = fade_progress(age, self.opts.fade_duration);
            let color = lerp_rgb(self.opts.fade_from, self.opts.fade_to, progress);
            out.push(RevealedGrapheme {
                text: self.graphemes[i].clone(),
                color,
                progress,
            });
        }
        out
    }
}

/// Linear progress in `[0.0, 1.0]` for a grapheme that has been visible
/// for `age` against a fade window of `fade`. `fade == 0` is treated as
/// "instantly fully faded" so callers can opt out of the fade by zeroing
/// it without a divide-by-zero.
fn fade_progress(age: Duration, fade: Duration) -> f32 {
    if fade.is_zero() {
        return 1.0;
    }
    let raw = age.as_secs_f64() / fade.as_secs_f64();
    raw.clamp(0.0, 1.0) as f32
}

fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    Rgb(
        lerp_u8(a.0, b.0, t),
        lerp_u8(a.1, b.1, t),
        lerp_u8(a.2, b.2, t),
    )
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let af = a as f32;
    let bf = b as f32;
    (af + (bf - af) * t).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> RevealOpts {
        RevealOpts {
            char_interval: Duration::from_millis(50),
            fade_duration: Duration::from_millis(100),
            fade_from: Rgb(0, 0, 0),
            fade_to: Rgb(200, 200, 200),
        }
    }

    fn epoch() -> Instant {
        // Anchor every test at a single Instant; we move forward via
        // `+ Duration` to side-step real-time non-determinism.
        Instant::now()
    }

    #[test]
    fn first_grapheme_is_visible_at_t_zero() {
        let now = epoch();
        let h = RevealHandle::start_at("abc", opts(), now);
        let snap = h.snapshot(now);
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].text, "a");
        // Just appeared → progress 0 → still at fade_from.
        assert_eq!(snap[0].color, Rgb(0, 0, 0));
        assert_eq!(snap[0].progress, 0.0);
    }

    #[test]
    fn typewriter_advances_one_grapheme_per_interval() {
        let now = epoch();
        let h = RevealHandle::start_at("abcd", opts(), now);
        assert_eq!(h.visible_count(now), 1);
        assert_eq!(h.visible_count(now + Duration::from_millis(50)), 2);
        assert_eq!(h.visible_count(now + Duration::from_millis(100)), 3);
        assert_eq!(h.visible_count(now + Duration::from_millis(150)), 4);
        // Past the last grapheme — clamps.
        assert_eq!(h.visible_count(now + Duration::from_millis(500)), 4);
    }

    #[test]
    fn fade_interpolates_over_fade_duration() {
        let now = epoch();
        let h = RevealHandle::start_at("a", opts(), now);

        // Mid-fade: 50 ms into a 100 ms fade → progress 0.5 → channel 100.
        let snap = h.snapshot(now + Duration::from_millis(50));
        assert!((snap[0].progress - 0.5).abs() < 1e-3);
        assert_eq!(snap[0].color, Rgb(100, 100, 100));

        // Fully faded.
        let snap = h.snapshot(now + Duration::from_millis(100));
        assert_eq!(snap[0].progress, 1.0);
        assert_eq!(snap[0].color, Rgb(200, 200, 200));

        // Past the fade — pinned at fade_to.
        let snap = h.snapshot(now + Duration::from_millis(400));
        assert_eq!(snap[0].color, Rgb(200, 200, 200));
    }

    #[test]
    fn later_graphemes_start_their_own_fade() {
        let now = epoch();
        let h = RevealHandle::start_at("ab", opts(), now);
        // At t=50ms, "a" has been visible for 50ms (mid-fade) and "b"
        // just appeared (progress 0).
        let snap = h.snapshot(now + Duration::from_millis(50));
        assert_eq!(snap.len(), 2);
        assert!((snap[0].progress - 0.5).abs() < 1e-3);
        assert_eq!(snap[1].progress, 0.0);
    }

    #[test]
    fn iterates_grapheme_clusters_not_chars() {
        // `é` written as `e` + U+0301 is one grapheme cluster but two
        // chars; the typewriter must reveal it as one tick, not two.
        let composed = "e\u{0301}f"; // "éf"
        let now = epoch();
        let h = RevealHandle::start_at(composed, opts(), now);
        assert_eq!(h.total_graphemes(), 2);

        let snap = h.snapshot(now);
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].text, "e\u{0301}");
    }

    #[test]
    fn handles_japanese_text() {
        let now = epoch();
        let h = RevealHandle::start_at("東京特許許可局", opts(), now);
        assert_eq!(h.total_graphemes(), 7);

        let snap = h.snapshot(now + Duration::from_millis(150));
        assert_eq!(snap.len(), 4);
        assert_eq!(snap[0].text, "東");
        assert_eq!(snap[3].text, "許");
    }

    #[test]
    fn empty_text_has_no_graphemes() {
        let now = epoch();
        let h = RevealHandle::start_at("", opts(), now);
        assert_eq!(h.total_graphemes(), 0);
        assert_eq!(h.visible_count(now), 0);
        assert!(h.is_done(now));
        assert!(h.snapshot(now).is_empty());
    }

    #[test]
    fn is_done_reports_when_last_grapheme_finishes_fading() {
        let now = epoch();
        let h = RevealHandle::start_at("abc", opts(), now);
        // Last grapheme appears at t=100ms, fades over 100ms → done at 200ms.
        assert!(!h.is_done(now));
        assert!(!h.is_done(now + Duration::from_millis(150)));
        assert!(!h.is_done(now + Duration::from_millis(199)));
        assert!(h.is_done(now + Duration::from_millis(200)));
        assert!(h.is_done(now + Duration::from_millis(500)));
    }

    #[test]
    fn zero_fade_produces_instant_full_color() {
        let mut o = opts();
        o.fade_duration = Duration::ZERO;
        let now = epoch();
        let h = RevealHandle::start_at("a", o, now);
        let snap = h.snapshot(now);
        assert_eq!(snap[0].progress, 1.0);
        assert_eq!(snap[0].color, o.fade_to);
    }

    #[test]
    fn lerp_rgb_endpoints_are_exact() {
        let a = Rgb(10, 20, 30);
        let b = Rgb(200, 100, 50);
        assert_eq!(lerp_rgb(a, b, 0.0), a);
        assert_eq!(lerp_rgb(a, b, 1.0), b);
    }

    #[test]
    fn lerp_rgb_clamps_out_of_range_t() {
        let a = Rgb(0, 0, 0);
        let b = Rgb(100, 100, 100);
        assert_eq!(lerp_rgb(a, b, -1.0), a);
        assert_eq!(lerp_rgb(a, b, 2.0), b);
    }
}
