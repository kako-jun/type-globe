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

use crossterm::event::{self, Event, KeyEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

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

pub struct InputChannel {
    rx: mpsc::Receiver<KeyEvent>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
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
