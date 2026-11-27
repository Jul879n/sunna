//! What changes while a page is open without the person touching it: the
//! machine list is read again every few seconds, and the Updates and Privacy
//! pages follow the app's status (an update arriving, a check finishing).

use super::*;
use std::time::Duration;

const TICK: Duration = Duration::from_millis(300);
/// Ticks between two reads of the machine list (about 3 s).
const MACHINES_EVERY: u32 = 10;

impl SettingsView {
    pub(super) fn start_live(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| {
            let mut n: u32 = 0;
            loop {
                cx.background_executor().timer(TICK).await;
                n = n.wrapping_add(1);
                if this
                    .update_in(cx, |this, _, cx| {
                        match this.section.as_str() {
                            "machines" if n.is_multiple_of(MACHINES_EVERY) => this.load_machines(cx),
                            "updates" | "privacy" | "terminal" => cx.notify(),
                            _ => {}
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Called when a page of this file is shown: it reads what it needs.
    pub(super) fn entered(&mut self, section: &str, window: &mut Window, cx: &mut Context<Self>) {
        match section {
            "machines" => self.load_machines(cx),
            "mods" => {
                self.visit_mods(window, cx);
            }
            _ => {}
        }
    }
}
