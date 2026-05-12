//! Background input thread + mpsc handover (#22).
//!
//! Splits keyboard input off from the main render loop so the renderer
//! can tick on its own cadence without ever blocking on `event::read`.
//! The architectural goal — and the spec contract — is that a player
//! who already knows the answer can begin typing during a `jiwa_core`
//! reveal without waiting for the animation to finish ("先打ち可能").
//!
//! Pattern:
//! - [`InputChannel::spawn`] starts a worker thread that runs a poll /
//!   read loop and forwards every [`KeyEvent`] over an `mpsc` channel.
//! - The main thread calls [`InputChannel::recv_until`] each frame: a
//!   `Some(key)` means "handle this", `None` means "no input within
//!   the redraw window — just redraw and try again".
//! - Cleanup is cooperative: dropping the channel flips an atomic
//!   shutdown flag that the worker thread checks on each poll. The
//!   worker exits within ~`INPUT_POLL` of the flag flip and the join
//!   happens in `Drop`.
//!
//! `crossterm` permits only one reader of terminal events at a time,
//! so callers MUST NOT also call `event::read` on the main thread for
//! the duration the channel is alive. The channel is the single source
//! of input events.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Polling interval inside the worker thread. Short enough that the
/// worker notices a shutdown signal within roughly one frame, long
/// enough that the thread doesn't spin on an idle terminal.
const INPUT_POLL: Duration = Duration::from_millis(50);

/// Reasons [`InputChannel::recv_until`] may return without a key.
#[derive(Debug, PartialEq, Eq)]
pub enum RecvOutcome {
    /// A key arrived within the timeout.
    Key(KeyEvent),
    /// The redraw window elapsed with no key. The caller should
    /// redraw and call again.
    Timeout,
    /// The worker thread has exited (e.g. terminal disconnected).
    /// The caller should treat this as a quit signal.
    Disconnected,
}

/// Abstraction over "wherever the next key event comes from" (#106).
///
/// The Quiz/Listening render loops don't care whether a `KeyEvent` was
/// produced by a human pressing a key (`InputChannel`) or by an auto-demo
/// driver synthesising keystrokes (`DemoInputSource`). Sharing a trait
/// keeps the existing input pipeline (rejection flash, jiwa reveal,
/// auto-confirm on correct answer) intact for the demo path without any
/// special-casing in the session code.
pub trait KeyEventSource {
    /// Block up to `timeout` for the next synthetic / real key event.
    fn recv_until(&self, timeout: Duration) -> RecvOutcome;
}

pub struct InputChannel {
    rx: mpsc::Receiver<KeyEvent>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl KeyEventSource for InputChannel {
    fn recv_until(&self, timeout: Duration) -> RecvOutcome {
        InputChannel::recv_until(self, timeout)
    }
}

impl InputChannel {
    /// Spawn the worker thread and return the receiving end. Raw mode
    /// MUST already be enabled — the worker calls `event::read`
    /// directly.
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        let handle = thread::spawn(move || run_input(tx, worker_shutdown));
        Self {
            rx,
            shutdown,
            handle: Some(handle),
        }
    }

    /// Block up to `timeout` for the next key. The redraw cadence of
    /// the surrounding loop is the timeout the caller should pass.
    pub fn recv_until(&self, timeout: Duration) -> RecvOutcome {
        match self.rx.recv_timeout(timeout) {
            Ok(key) => RecvOutcome::Key(key),
            Err(RecvTimeoutError::Timeout) => RecvOutcome::Timeout,
            Err(RecvTimeoutError::Disconnected) => RecvOutcome::Disconnected,
        }
    }
}

impl Drop for InputChannel {
    fn drop(&mut self) {
        // Tell the worker to stop on its next poll. We can't interrupt
        // a `crossterm::event::read` call mid-flight, but the poll-then-
        // read pattern means the worker is at most `INPUT_POLL` away
        // from observing the flag.
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run_input(tx: mpsc::Sender<KeyEvent>, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        match event::poll(INPUT_POLL) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) => {
                    if tx.send(key).is_err() {
                        // Receiver dropped — main loop is gone, time to leave.
                        break;
                    }
                }
                // Resize, mouse, focus, paste — currently unused by Quiz;
                // drop and keep polling so input latency stays constant.
                Ok(_) => {}
                Err(_) => break,
            },
            Ok(false) => {} // No event in this poll window; loop and re-check shutdown.
            Err(_) => break, // Terminal disconnected or worse.
        }
    }
}

// ---------------------------------------------------------------------------
// Auto-demo input source (#106)
// ---------------------------------------------------------------------------

/// Internal demo state machine. Lives behind a `Mutex` inside
/// [`DemoInputSource`] so the session code can refresh the "target string
/// to type" each time a new question begins without taking a reference
/// across the render loop.
#[derive(Debug)]
struct DemoState {
    /// The full string the demo should type for the active question.
    /// Cleared once typing is finished; the session is expected to call
    /// `DemoInputSource::set_target` again before the next question.
    target: Vec<char>,
    /// How many characters of `target` have already been delivered.
    cursor: usize,
    /// Wall-clock instant at which the *next* event (either the first
    /// keystroke after the wait window, or the next keystroke during
    /// typing) is allowed to fire. Acts as a single throttle for both
    /// the per-question wait and the per-keystroke spacing.
    next_fire_at: Option<Instant>,
    /// Per-keystroke interval derived from `--demo-type-cps`. A floor of
    /// 1 ms is enforced so a pathological CPS doesn't make the demo
    /// effectively block-write the answer in zero time.
    type_interval: Duration,
    /// Initial wait between question reveal and the first keystroke
    /// (`--demo-wait-ms`).
    wait_per_question: Duration,
}

/// Synthetic [`KeyEventSource`] used by the auto-demo (#106).
///
/// The session driver calls [`DemoInputSource::set_target`] each time a
/// new question becomes active. `recv_until` then emits one `KeyEvent`
/// per call (paced by `--demo-type-cps`, with an initial `--demo-wait-ms`
/// gap) until the full target has been typed. When the buffer is empty
/// or fully delivered, `recv_until` returns [`RecvOutcome::Timeout`] so
/// the render loop keeps repainting between keystrokes.
///
/// The demo never produces Esc / Ctrl+C itself; those still come from
/// the real keyboard via a separately spawned [`InputChannel`] that the
/// CLI layer multiplexes against this source.
pub struct DemoInputSource {
    state: Arc<Mutex<DemoState>>,
}

impl DemoInputSource {
    /// Build a demo source with the given typing speed (characters per
    /// second) and per-question wait. `type_cps == 0` is normalised to
    /// 1 cps so we never divide by zero; very high CPS is clamped to a
    /// 1 ms minimum interval to keep redraws happening between events.
    pub fn new(type_cps: u32, wait_per_question: Duration) -> Self {
        let cps = type_cps.max(1);
        let interval_ms = (1000 / cps).max(1);
        let state = DemoState {
            target: Vec::new(),
            cursor: 0,
            next_fire_at: None,
            type_interval: Duration::from_millis(interval_ms as u64),
            wait_per_question,
        };
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Replace the string the demo should type next. Resets the cursor
    /// and arms the per-question wait timer.
    pub fn set_target(&self, target: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.target = target.chars().collect();
            state.cursor = 0;
            state.next_fire_at = Some(Instant::now() + state.wait_per_question);
        }
    }

    /// `true` once the current target has been fully delivered.
    ///
    /// Originally intended as a production-side affordance for "demo is
    /// idle, advance to the next question", but the actual production
    /// path advances through the normal auto-confirm-on-correct branch
    /// in `QuizUI::submit_current_answer` — so this method is only
    /// reachable from tests. S-4: gated behind `cfg(test)` rather than
    /// papered over with `#[allow(dead_code)]` so the surface area of
    /// `DemoInputSource` in release builds stays minimal. `KeyEventSource`
    /// is a separate trait that doesn't reference `target_consumed`, so
    /// removing it from the production build doesn't affect object
    /// safety.
    #[cfg(test)]
    pub fn target_consumed(&self) -> bool {
        match self.state.lock() {
            Ok(state) => state.cursor >= state.target.len(),
            Err(_) => true,
        }
    }
}

impl KeyEventSource for DemoInputSource {
    fn recv_until(&self, timeout: Duration) -> RecvOutcome {
        // We pick a "wait deadline" for this call and either deliver a
        // synthetic key at the right moment, or fall through to the
        // timeout so the render loop keeps repainting (timer/reveal).
        let started = Instant::now();
        let deadline = started + timeout;

        loop {
            let now = Instant::now();
            let next_char = {
                let mut state = self.state.lock().expect("demo state poisoned");
                if state.cursor >= state.target.len() {
                    None
                } else {
                    let fire_at = state.next_fire_at.unwrap_or(now);
                    if now < fire_at {
                        // Not yet time for the next key. Sleep until
                        // either the deadline or the fire time, whichever
                        // is sooner, then re-check.
                        let sleep_until = fire_at.min(deadline);
                        drop(state);
                        if sleep_until > now {
                            thread::sleep(sleep_until - now);
                        }
                        if Instant::now() >= deadline {
                            return RecvOutcome::Timeout;
                        }
                        continue;
                    }
                    let c = state.target[state.cursor];
                    state.cursor += 1;
                    // Schedule the next keystroke `type_interval` after
                    // *this* one's fire time so the cadence is steady
                    // regardless of how long the caller waits between
                    // `recv_until` calls.
                    state.next_fire_at = Some(fire_at + state.type_interval);
                    Some(c)
                }
            };

            match next_char {
                Some(c) => {
                    return RecvOutcome::Key(synth_key(c));
                }
                None => {
                    // Nothing left to type for the current target — keep
                    // the render loop ticking. The session will pump a
                    // new target on the next question.
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return RecvOutcome::Timeout;
                    }
                    thread::sleep(remaining);
                    return RecvOutcome::Timeout;
                }
            }
        }
    }
}

/// Build a `KeyEvent` matching what crossterm would deliver for a single
/// character keystroke. No modifiers — the demo only types printable
/// chars and the Quiz input handler drops modifier chords anyway.
fn synth_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// Multiplex two key-event sources, returning whichever produces a key
/// first within `timeout`. Used by the demo path so the real keyboard's
/// Esc / Ctrl+C still aborts the run even while the synthetic source is
/// driving the typing. The primary source (passed as `a`) is polled in a
/// short slice; if it has nothing, `b` is polled for the remainder.
pub struct MultiplexedSource<A: KeyEventSource, B: KeyEventSource> {
    pub a: A,
    pub b: B,
}

impl<A: KeyEventSource, B: KeyEventSource> KeyEventSource for MultiplexedSource<A, B> {
    fn recv_until(&self, timeout: Duration) -> RecvOutcome {
        // Poll the human source first with a small slice so abort keys
        // are responsive. The rest of the budget goes to the demo source.
        const HUMAN_POLL: Duration = Duration::from_millis(5);
        let human_slice = HUMAN_POLL.min(timeout);
        match self.a.recv_until(human_slice) {
            RecvOutcome::Key(k) => return RecvOutcome::Key(k),
            RecvOutcome::Disconnected => {
                // Human channel is gone; fall back to demo for the rest
                // of the budget so the run can still complete cleanly.
            }
            RecvOutcome::Timeout => {}
        }
        let remaining = timeout.saturating_sub(human_slice);
        if remaining.is_zero() {
            return RecvOutcome::Timeout;
        }
        self.b.recv_until(remaining)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// Build a complete `InputChannel` without spawning the real
    /// crossterm worker — we just want to drive `recv_until` directly
    /// against an `mpsc` we control. Mirrors the production layout so
    /// the `Drop` cleanup path stays exercised.
    fn channel_with_dummy_worker() -> (InputChannel, mpsc::Sender<KeyEvent>) {
        let (tx, rx) = mpsc::channel();
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        // The "worker" just sits and watches the shutdown flag; tests
        // push events through `tx` directly.
        let handle = thread::spawn(move || {
            while !worker_shutdown.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(10));
            }
        });
        (
            InputChannel {
                rx,
                shutdown,
                handle: Some(handle),
            },
            tx,
        )
    }

    #[test]
    fn recv_until_returns_key_when_one_arrives() {
        let (input, tx) = channel_with_dummy_worker();
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        tx.send(key).unwrap();
        match input.recv_until(Duration::from_millis(100)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('x')),
            other => panic!("expected Key, got {other:?}"),
        }
    }

    #[test]
    fn recv_until_returns_timeout_when_idle() {
        let (input, _tx) = channel_with_dummy_worker();
        let started = Instant::now();
        let outcome = input.recv_until(Duration::from_millis(40));
        assert_eq!(outcome, RecvOutcome::Timeout);
        // recv_until should not return early — the timeout is the
        // entire wait window.
        assert!(started.elapsed() >= Duration::from_millis(35));
    }

    #[test]
    fn recv_until_reports_disconnected_when_sender_drops() {
        let (input, tx) = channel_with_dummy_worker();
        drop(tx);
        match input.recv_until(Duration::from_millis(100)) {
            RecvOutcome::Disconnected => {}
            other => panic!("expected Disconnected, got {other:?}"),
        }
    }

    // ----- DemoInputSource sanity tests (#106) -----

    #[test]
    fn demo_source_waits_then_emits_target_chars_in_order() {
        // 200 cps → 5 ms per keystroke; 0 ms initial wait so the test
        // doesn't have to sleep through `--demo-wait-ms` first.
        let demo = DemoInputSource::new(200, Duration::from_millis(0));
        demo.set_target("abc");
        let mut typed = String::new();
        for _ in 0..3 {
            match demo.recv_until(Duration::from_millis(100)) {
                RecvOutcome::Key(k) => {
                    if let KeyCode::Char(c) = k.code {
                        typed.push(c);
                    }
                }
                other => panic!("expected Key, got {other:?}"),
            }
        }
        assert_eq!(typed, "abc");
        assert!(demo.target_consumed());
    }

    #[test]
    fn demo_source_idle_after_target_consumed() {
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("x");
        // Drain the single char.
        let _ = demo.recv_until(Duration::from_millis(50));
        // With no remaining target, recv_until must return Timeout
        // (NOT Disconnected) so the session keeps redrawing.
        let outcome = demo.recv_until(Duration::from_millis(20));
        assert_eq!(outcome, RecvOutcome::Timeout);
    }

    #[test]
    fn demo_source_set_target_resets_cursor() {
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("a");
        let _ = demo.recv_until(Duration::from_millis(50));
        assert!(demo.target_consumed());
        // Priming a new target must allow another keystroke.
        demo.set_target("b");
        assert!(!demo.target_consumed());
        match demo.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('b')),
            other => panic!("expected Key('b'), got {other:?}"),
        }
    }

    #[test]
    fn demo_source_zero_cps_is_normalised_to_one_cps() {
        // Defensive: --demo-type-cps 0 must not panic on divide-by-zero
        // and must still produce keys (just slowly).
        let demo = DemoInputSource::new(0, Duration::from_millis(0));
        demo.set_target("z");
        match demo.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('z')),
            other => panic!("expected Key('z'), got {other:?}"),
        }
    }

    // ----- DemoInputSource behavior tests (#106) -----

    #[test]
    fn demo_source_empty_target_returns_timeout() {
        // No target armed → recv_until must keep ticking (Timeout), not
        // Disconnected. The session relies on the redraw loop continuing.
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("");
        let outcome = demo.recv_until(Duration::from_millis(20));
        assert_eq!(outcome, RecvOutcome::Timeout);
    }

    #[test]
    fn demo_source_long_target_delivers_all_chars_in_order() {
        // 200-char target at 10000 cps (1 ms floor → 100 chars / 100 ms).
        // Every character must arrive once and in input order.
        let target: String = (0..200).map(|i| (b'a' + (i % 26) as u8) as char).collect();
        let demo = DemoInputSource::new(10000, Duration::from_millis(0));
        demo.set_target(&target);
        let mut typed = String::new();
        for _ in 0..target.chars().count() {
            match demo.recv_until(Duration::from_millis(500)) {
                RecvOutcome::Key(k) => {
                    if let KeyCode::Char(c) = k.code {
                        typed.push(c);
                    }
                }
                other => panic!("expected Key, got {other:?}"),
            }
        }
        assert_eq!(typed, target);
        assert!(demo.target_consumed());
    }

    #[test]
    fn demo_source_unicode_long_vowel_target() {
        // `ko-hi-` (コーヒー) uses ASCII `-` for long vowels per #93.
        // The source must deliver each char including the dashes intact.
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("ko-hi-");
        let mut typed = String::new();
        for _ in 0..6 {
            match demo.recv_until(Duration::from_millis(100)) {
                RecvOutcome::Key(k) => {
                    if let KeyCode::Char(c) = k.code {
                        typed.push(c);
                    }
                }
                other => panic!("expected Key, got {other:?}"),
            }
        }
        assert_eq!(typed, "ko-hi-");
    }

    #[test]
    fn demo_source_cadence_is_steady_when_caller_pauses_between_calls() {
        // 100 cps = 10 ms interval. If the caller pauses 50 ms between
        // recv_until calls, the cadence is anchored to the prior fire time
        // so the 2nd char must be available immediately (≤ 5 ms).
        let demo = DemoInputSource::new(100, Duration::from_millis(0));
        demo.set_target("abc");
        // First keystroke (no anchor yet) — discard.
        let _ = demo.recv_until(Duration::from_millis(50));
        thread::sleep(Duration::from_millis(50));
        let started = Instant::now();
        match demo.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('b')),
            other => panic!("expected Key('b'), got {other:?}"),
        }
        assert!(
            started.elapsed() < Duration::from_millis(5),
            "cadence not steady: 2nd char took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn demo_source_set_target_during_active_typing_resets_cursor() {
        // Mid-stream `set_target` (e.g. question advanced) must drop the
        // remainder of the old buffer and start the new one from index 0.
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("abc");
        let _ = demo.recv_until(Duration::from_millis(50)); // 'a'
        demo.set_target("xyz");
        match demo.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('x')),
            other => panic!("expected Key('x'), got {other:?}"),
        }
    }

    #[test]
    fn demo_source_high_cps_clamped_to_min_1ms_interval() {
        // Insane CPS must not panic and must not produce a zero interval
        // (which would let the demo deliver everything in one tight loop
        // and starve redraws). 10 chars at 1 ms each = ~10 ms minimum.
        let demo = DemoInputSource::new(10_000_000, Duration::from_millis(0));
        demo.set_target("0123456789");
        let started = Instant::now();
        for _ in 0..10 {
            match demo.recv_until(Duration::from_millis(200)) {
                RecvOutcome::Key(_) => {}
                other => panic!("expected Key, got {other:?}"),
            }
        }
        // Sanity: total time >= 9 * 1ms (9 inter-keystroke gaps).
        assert!(
            started.elapsed() >= Duration::from_millis(8),
            "interval not clamped: 10 chars in {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn demo_source_one_cps_uses_1000ms_interval() {
        // 1 cps = 1000 ms between keystrokes. 2nd char must NOT arrive
        // before ~900 ms after the 1st.
        let demo = DemoInputSource::new(1, Duration::from_millis(0));
        demo.set_target("ab");
        // First key fires nearly immediately (no wait, no prior anchor).
        let _ = demo.recv_until(Duration::from_millis(50));
        let started = Instant::now();
        let outcome = demo.recv_until(Duration::from_millis(800));
        // Within 800 ms of the first keystroke, the second must NOT have
        // fired yet — the spacing contract is 1000 ms.
        assert_eq!(outcome, RecvOutcome::Timeout);
        assert!(started.elapsed() >= Duration::from_millis(750));
    }

    #[test]
    fn demo_source_space_and_digit_in_target() {
        // Non-alphabetic chars (space, digits) must round-trip as
        // KeyCode::Char so phrase / number answers can be typed.
        let demo = DemoInputSource::new(1000, Duration::from_millis(0));
        demo.set_target("a 1");
        let mut chars = Vec::new();
        for _ in 0..3 {
            match demo.recv_until(Duration::from_millis(50)) {
                RecvOutcome::Key(k) => {
                    if let KeyCode::Char(c) = k.code {
                        chars.push(c);
                    }
                }
                other => panic!("expected Key, got {other:?}"),
            }
        }
        assert_eq!(chars, vec!['a', ' ', '1']);
    }

    #[test]
    fn synth_key_produces_no_modifiers() {
        // The demo never holds Ctrl/Alt/Shift; modifier bits on a synthetic
        // event would trip the Quiz input handler's Esc/Ctrl-C branches.
        let k = synth_key('q');
        assert_eq!(k.code, KeyCode::Char('q'));
        assert_eq!(k.modifiers, KeyModifiers::NONE);
    }

    // ----- MultiplexedSource tests (#106) -----

    /// Mock `KeyEventSource` driven by a `Mutex<VecDeque<RecvOutcome>>`
    /// so a single test can stage a deterministic sequence of returns
    /// without spawning crossterm or sleeping for real timeouts.
    struct MockSource {
        queue: Mutex<std::collections::VecDeque<RecvOutcome>>,
        default: fn() -> RecvOutcome,
    }

    impl MockSource {
        fn new(default: fn() -> RecvOutcome) -> Self {
            Self {
                queue: Mutex::new(std::collections::VecDeque::new()),
                default,
            }
        }
        fn push(&self, outcome: RecvOutcome) {
            self.queue.lock().unwrap().push_back(outcome);
        }
    }

    impl KeyEventSource for MockSource {
        fn recv_until(&self, _timeout: Duration) -> RecvOutcome {
            match self.queue.lock().unwrap().pop_front() {
                Some(o) => o,
                None => (self.default)(),
            }
        }
    }

    #[test]
    fn multiplexed_returns_human_key_when_human_has_event() {
        // Human source has a key queued → mux must return it without
        // consulting the demo source.
        let human = MockSource::new(|| RecvOutcome::Timeout);
        human.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Esc,
            KeyModifiers::NONE,
        )));
        let demo = MockSource::new(|| RecvOutcome::Timeout);
        demo.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::NONE,
        )));
        let mux = MultiplexedSource { a: human, b: demo };
        match mux.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Esc),
            other => panic!("expected Esc from human, got {other:?}"),
        }
        // Demo queue still has its 'z' — mux must NOT have drained it.
        assert_eq!(mux.b.queue.lock().unwrap().len(), 1);
    }

    #[test]
    fn multiplexed_falls_through_to_demo_when_human_idle() {
        let human = MockSource::new(|| RecvOutcome::Timeout);
        let demo = MockSource::new(|| RecvOutcome::Timeout);
        demo.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        )));
        let mux = MultiplexedSource { a: human, b: demo };
        match mux.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('a')),
            other => panic!("expected Key('a') from demo, got {other:?}"),
        }
    }

    #[test]
    fn multiplexed_continues_on_human_disconnected() {
        // If the human channel drops (e.g. terminal vanished mid-demo),
        // the demo must keep driving rather than the whole mux returning
        // Disconnected and killing the run.
        let human = MockSource::new(|| RecvOutcome::Disconnected);
        let demo = MockSource::new(|| RecvOutcome::Timeout);
        demo.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::NONE,
        )));
        let mux = MultiplexedSource { a: human, b: demo };
        match mux.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('d')),
            other => panic!("expected Key('d') from demo, got {other:?}"),
        }
    }

    #[test]
    fn multiplexed_source_with_zero_duration_does_not_panic() {
        // S-7: `recv_until(Duration::ZERO)` should be a no-op poll that
        // returns Timeout (or whatever the human source happens to
        // deliver in that instant) without dividing by zero, sleeping
        // negative durations, or panicking in `saturating_sub`. Both
        // queues empty → must come back as Timeout.
        let human = MockSource::new(|| RecvOutcome::Timeout);
        let demo = MockSource::new(|| RecvOutcome::Timeout);
        let mux = MultiplexedSource { a: human, b: demo };
        let outcome = mux.recv_until(Duration::ZERO);
        assert_eq!(outcome, RecvOutcome::Timeout);
    }

    #[test]
    fn multiplexed_does_not_double_dispatch_demo_key() {
        // Two successive recv_until calls with two demo events queued must
        // deliver them in order ('a' then 'b'), not duplicate the first.
        let human = MockSource::new(|| RecvOutcome::Timeout);
        let demo = MockSource::new(|| RecvOutcome::Timeout);
        demo.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
        )));
        demo.push(RecvOutcome::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::NONE,
        )));
        let mux = MultiplexedSource { a: human, b: demo };
        match mux.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('a')),
            other => panic!("expected Key('a'), got {other:?}"),
        }
        match mux.recv_until(Duration::from_millis(50)) {
            RecvOutcome::Key(k) => assert_eq!(k.code, KeyCode::Char('b')),
            other => panic!("expected Key('b'), got {other:?}"),
        }
    }

    #[test]
    fn drop_signals_shutdown_and_joins_worker() {
        let (input, _tx) = channel_with_dummy_worker();
        // The worker thread is alive and looping on the shutdown flag.
        // `Drop` must flip the flag and `join` the thread; if it didn't,
        // the thread would outlive this test and the assertion below
        // would race.
        let started = Instant::now();
        drop(input);
        // Drop returns only after the worker observes the flag and exits;
        // worst case ~1 sleep tick (10 ms) inside our dummy.
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
