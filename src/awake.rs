//! Keeps the computer awake: an OS inhibitor that lives as long as a child
//! process, held while the person's mode asks for it.

use serde::{Deserialize, Serialize};
use std::process::{Child, Stdio};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Off,
    /// Awake while an agent works.
    Auto,
    On,
}

impl Mode {
    pub fn is_off(&self) -> bool {
        *self == Mode::Off
    }
}

static HELD: Mutex<Option<Child>> = Mutex::new(None);
/// The log line is written once; retrying is never blocked by it.
static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static LAST_TRY: Mutex<Option<std::time::Instant>> = Mutex::new(None);

fn start() -> Option<Child> {
    let pid = std::process::id().to_string();
    let mut command = if cfg!(target_os = "macos") {
        let mut c = crate::spawn::command("caffeinate");
        c.args(["-dis", "-w", &pid]);
        c
    } else if cfg!(target_os = "linux") {
        // `tail --pid` ends with the app, so a crash can't leave the inhibitor behind.
        let mut c = crate::spawn::command("systemd-inhibit");
        c.args(["--what=idle:sleep", "--who=Sunna", "--why=Sunna", "--mode=block", "tail", &format!("--pid={pid}"), "-f", "/dev/null"]);
        c
    } else {
        return None;
    };
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    match command.spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
                tracing::warn!("keep awake unavailable: {e}");
            }
            None
        }
    }
}

/// Takes or drops the inhibitor for `mode` and whether agents are working. Cheap when nothing changes.
pub fn update(mode: Mode, working: bool) {
    // No inhibitor here (see `start`): nothing to hold, so don't look every tick.
    if cfg!(not(any(target_os = "macos", target_os = "linux"))) {
        return;
    }
    let want = mode == Mode::On || (mode == Mode::Auto && working);
    let Ok(mut held) = HELD.lock() else { return };
    if let Some(child) = held.as_mut()
        && child.try_wait().ok().flatten().is_some()
    {
        *held = None;
    }
    if want && held.is_none() {
        // A failed start is tried again, but not on every tick.
        let Ok(mut last) = LAST_TRY.lock() else { return };
        if last.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(10)) {
            return;
        }
        *last = Some(std::time::Instant::now());
        *held = start();
    } else if !want
        && let Some(mut child) = held.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

pub fn release() {
    if let Ok(mut held) = HELD.lock()
        && let Some(mut child) = held.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_serialize_in_lowercase_and_default_to_off() {
        assert_eq!(serde_json::to_string(&Mode::Auto).unwrap(), "\"auto\"");
        assert_eq!(serde_json::from_str::<Mode>("\"on\"").unwrap(), Mode::On);
        assert_eq!(Mode::default(), Mode::Off);
    }
}
