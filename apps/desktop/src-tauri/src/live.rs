//! What the running downloads are doing now, from the runner's events: the
//! bytes on disk, the speed, and the file. The queues file has the rest.

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use tdm_core::Event;
use tdm_core::queue::QueueEvent;
use ts_rs::TS;

/// The page is told at most this often that something changed, except for
/// a download that ends, which is told at once.
/// Twice a second (the owner's choice, after measuring the list's cost:
/// it was four times).
pub const NOTIFY_EVERY: Duration = Duration::from_millis(500);
/// A download with no progress for this long shows a speed of 0.
const STALLED_AFTER: Duration = Duration::from_secs(2);
const MAX_ACTIVITY_EVENTS: usize = 1000;
const MAX_ACTIVITY_TEXT: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "UPPERCASE")]
#[ts(export)]
pub enum ActivityLevel {
    Info,
    Success,
    Warning,
    Error,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ActivityEntry {
    #[ts(type = "number")]
    pub id: u64,
    pub timestamp: String,
    pub level: ActivityLevel,
    pub engine: String,
    pub operation: String,
    pub message: String,
    pub details: Option<String>,
}

/// What to do with a change the page should hear about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notify {
    /// Tell it now.
    Now,
    /// Tell it after this delay (the last change is never lost: review of
    /// #45); nothing else is scheduled until then.
    Later(Duration),
    /// A later notification is already scheduled: it will carry this one.
    Scheduled,
}

#[derive(Debug, Default)]
pub struct Live {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    items: HashMap<u64, ItemLive>,
    activity: VecDeque<ActivityEntry>,
    next_activity_id: u64,
    progress_mark: HashMap<u64, u8>,
    problem: Option<String>,
    last_notify: Option<Instant>,
    /// A download ended since the last notification: tell at once.
    urgent: bool,
    /// Something other than the running downloads' progress changed since
    /// the last notification: the page reads the whole list again, not only
    /// the running rows ([`Live::take_changed`]).
    changed: bool,
    /// A notification is scheduled for later.
    scheduled: bool,
    /// Downloads that wait for their browser's cookies, which is not open
    /// or did not answer (M5): «افتح المتصفح ليكمل».
    waiting_for_browser: std::collections::HashSet<u64>,
    /// Downloads that wait for a free connection on their site (#170).
    waiting_for_site: std::collections::HashSet<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct ItemLive {
    pub done: u64,
    pub total: Option<u64>,
    /// Bytes per second, smoothed.
    pub speed: f64,
    pub file: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
    last: Option<(Instant, u64)>,
    /// When the last progress came, to show 0 for a stalled download.
    last_progress: Option<Instant>,
}

impl Live {
    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn record(&self, event: &QueueEvent) {
        let mut inner = self.inner();
        // Bytes coming in change only the running rows' numbers.
        if !matches!(
            event,
            QueueEvent::Download {
                event: Event::Progress { .. } | Event::Restarted { .. },
                ..
            }
        ) {
            inner.changed = true;
        }

        match event {
            QueueEvent::Started { item, .. } => {
                inner.items.insert(*item, ItemLive::default());
                inner.waiting_for_browser.remove(item);
                inner.waiting_for_site.remove(item);
                inner.urgent = true;
            }
            QueueEvent::Download { item, event } => {
                let live = inner.items.entry(*item).or_default();
                match event {
                    Event::Started {
                        file,
                        state_file,
                        total,
                        from,
                        ..
                    } => {
                        live.file = Some(file.clone());
                        live.state_file = Some(state_file.clone());
                        live.total = *total;
                        live.done = *from;
                        live.last = Some((Instant::now(), *from));
                    }
                    Event::Progress { done, total } => {
                        let now = Instant::now();
                        live.last_progress = Some(now);
                        if let Some((at, before)) = live.last {
                            let seconds = now.duration_since(at).as_secs_f64();
                            if seconds >= 0.2 {
                                let rate = done.saturating_sub(before) as f64 / seconds;
                                // A short memory, so the number does not jump.
                                live.speed = if live.speed == 0.0 {
                                    rate
                                } else {
                                    live.speed * 0.6 + rate * 0.4
                                };
                                live.last = Some((now, *done));
                            }
                        } else {
                            live.last = Some((now, *done));
                        }
                        live.done = *done;
                        live.total = *total;
                    }
                    Event::Restarted { .. } => {
                        live.done = 0;
                        live.speed = 0.0;
                        live.last = Some((Instant::now(), 0));
                    }
                    _ => {}
                }
            }
            QueueEvent::Finished { item, .. }
            | QueueEvent::Failed { item, .. }
            | QueueEvent::Retrying { item, .. }
            | QueueEvent::Stopped { item } => {
                inner.items.remove(item);
                inner.waiting_for_site.remove(item);
                inner.urgent = true;
            }
            QueueEvent::WaitingForSite { item } => {
                inner.waiting_for_site.insert(*item);
            }
            // The app decides, once it has asked the browser.
            QueueEvent::Waiting { .. }
            | QueueEvent::NeedsBrowser { .. }
            | QueueEvent::QueueDone { .. } => {}
        }
    }

    fn add_activity(
        &self,
        level: ActivityLevel,
        engine: &str,
        operation: &str,
        message: &str,
        details: Option<&str>,
    ) -> ActivityEntry {
        let mut inner = self.inner();
        inner.next_activity_id = inner.next_activity_id.saturating_add(1);
        let entry = ActivityEntry {
            id: inner.next_activity_id,
            timestamp: jiff::Timestamp::now().to_string(),
            level,
            engine: sanitize_activity_text(engine),
            operation: sanitize_activity_text(operation),
            message: sanitize_activity_text(message),
            details: details.map(sanitize_activity_text),
        };
        inner.activity.push_back(entry.clone());
        while inner.activity.len() > MAX_ACTIVITY_EVENTS {
            inner.activity.pop_front();
        }
        entry
    }

    fn activity_events(&self) -> Vec<ActivityEntry> {
        self.inner().activity.iter().cloned().collect()
    }

    fn clear_activity(&self) {
        self.inner().activity.clear();
    }

    pub(crate) fn should_log_progress(&self, item: u64, done: u64, total: Option<u64>) -> bool {
        let Some(total) = total.filter(|total| *total > 0) else {
            return false;
        };
        let percent = ((u128::from(done).min(u128::from(total)) * 100) / u128::from(total)) as u8;
        let mark = percent / 10 * 10;
        if mark == 0 {
            return false;
        }
        let mut inner = self.inner();
        let previous = inner.progress_mark.entry(item).or_insert(0);
        if mark <= *previous {
            return false;
        }
        *previous = mark;
        true
    }

    /// Whether download `id` waits for a free connection on its site.
    pub fn waiting_for_site(&self, id: u64) -> bool {
        self.inner().waiting_for_site.contains(&id)
    }

    /// Marks download `id` as waiting for its browser, or not; whether
    /// that changed.
    pub fn set_waiting_for_browser(&self, id: u64, waiting: bool) -> bool {
        let mut inner = self.inner();
        if waiting {
            inner.waiting_for_browser.insert(id)
        } else {
            inner.waiting_for_browser.remove(&id)
        }
    }

    pub fn waiting_for_browser(&self, id: u64) -> bool {
        self.inner().waiting_for_browser.contains(&id)
    }

    /// When to tell the page about a change, at `now`: at once, at most
    /// every [`NOTIFY_EVERY`] (at once when a download starts or ends), and
    /// the last change of a burst after the delay.
    pub fn notify_at(&self, now: Instant) -> Notify {
        let mut inner = self.inner();
        let wait = match inner.last_notify {
            Some(at) if !inner.urgent => NOTIFY_EVERY.saturating_sub(now.duration_since(at)),
            _ => Duration::ZERO,
        };
        if wait.is_zero() {
            inner.last_notify = Some(now);
            inner.urgent = false;
            Notify::Now
        } else if inner.scheduled {
            Notify::Scheduled
        } else {
            inner.scheduled = true;
            Notify::Later(wait)
        }
    }

    /// Whether something other than progress changed since the last call:
    /// the notification sent now says so.
    pub fn take_changed(&self) -> bool {
        std::mem::take(&mut self.inner().changed)
    }

    /// The scheduled notification is being sent, at `now`.
    pub fn notified_later(&self, now: Instant) {
        let mut inner = self.inner();
        inner.scheduled = false;
        inner.last_notify = Some(now);
        inner.urgent = false;
    }

    pub fn item(&self, id: u64) -> Option<ItemLive> {
        self.item_at(id, Instant::now())
    }

    /// The download `id` as it is at `now`: a speed of 0 once no progress
    /// has come for [`STALLED_AFTER`] (review of #45: it stayed at its last
    /// value).
    fn item_at(&self, id: u64, now: Instant) -> Option<ItemLive> {
        let mut item = self.inner().items.get(&id).cloned()?;
        if item
            .last_progress
            .is_some_and(|at| now.duration_since(at) >= STALLED_AFTER)
        {
            item.speed = 0.0;
        }
        Some(item)
    }

    /// Forgets the running downloads: the runner stopped with an error, so
    /// none of them runs (review of #45: their speed stayed on screen).
    pub fn clear(&self) {
        let mut inner = self.inner();
        inner.items.clear();
        inner.progress_mark.clear();
        // Nothing runs, so nothing waits for its site either (review of
        // #170).
        inner.waiting_for_site.clear();
    }

    pub(crate) fn forget_progress_mark(&self, item: u64) {
        self.inner().progress_mark.remove(&item);
    }

    pub fn set_problem(&self, problem: Option<String>) {
        self.inner().problem = problem;
    }

    pub fn problem(&self) -> Option<String> {
        self.inner().problem.clone()
    }
}

pub(crate) fn queue_activity<R: tauri::Runtime>(handle: &tauri::AppHandle<R>, event: &QueueEvent) {
    let id = match event {
        QueueEvent::Started { item, .. }
        | QueueEvent::Download { item, .. }
        | QueueEvent::Finished { item, .. }
        | QueueEvent::Failed { item, .. }
        | QueueEvent::Stopped { item }
        | QueueEvent::WaitingForSite { item }
        | QueueEvent::Retrying { item, .. }
        | QueueEvent::NeedsBrowser { item, .. } => Some(*item),
        QueueEvent::QueueDone { .. } | QueueEvent::Waiting { .. } => None,
    };
    match event {
        QueueEvent::Started { queue, item } => activity(
            handle,
            ActivityLevel::Info,
            "HTTP",
            "download",
            &format!("download {item} started in queue {queue}"),
            None,
        ),
        QueueEvent::Download {
            item,
            event:
                Event::Started {
                    file,
                    total,
                    from,
                    status,
                    content_type,
                    ..
                },
        } => {
            let filename = file
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();
            let details = format!(
                "HTTP status: {}\nContent-Type: {}\nExpected bytes: {}\nResumed from byte: {}\nFile: {}",
                status.map_or_else(|| "not reported".into(), |code| code.to_string()),
                content_type.as_deref().unwrap_or("not reported"),
                total.map_or_else(|| "unknown".into(), |size| size.to_string()),
                from,
                filename
            );
            activity(
                handle,
                ActivityLevel::Info,
                "HTTP",
                "response",
                &format!("download {item}: server response accepted; transfer started"),
                Some(&details),
            );
        }
        QueueEvent::Download {
            item,
            event: Event::Progress { done, total },
        } if handle
            .state::<crate::App>()
            .live
            .should_log_progress(*item, *done, *total) =>
        {
            activity(
                handle,
                ActivityLevel::Debug,
                "HTTP",
                "transfer",
                &format!(
                    "download {item}: {}% transferred",
                    total.map_or(0, |total| (u128::from(*done).min(u128::from(total)) * 100
                        / u128::from(total))
                        as u64)
                ),
                None,
            );
        }
        QueueEvent::Download {
            event: Event::Progress { .. },
            ..
        } => {}
        QueueEvent::Download {
            item,
            event: Event::Restarted { reason },
        } => activity(
            handle,
            ActivityLevel::Warning,
            "HTTP",
            "resume",
            &format!("download {item} restarted: {reason:?}"),
            None,
        ),
        QueueEvent::Download {
            item,
            event: Event::Split { segment, at },
        } => activity(
            handle,
            ActivityLevel::Debug,
            "HTTP",
            "segments",
            &format!("download {item}: segment {segment} split at byte {at}"),
            None,
        ),
        QueueEvent::Download {
            item,
            event:
                Event::ServerBusy {
                    status,
                    connections,
                    wait,
                },
        } => activity(
            handle,
            ActivityLevel::Warning,
            "HTTP",
            "retry",
            &format!(
                "download {item}: server returned HTTP {status}; connections limited to {connections}"
            ),
            wait.map(|wait| format!("Retry delay: {wait:?}")).as_deref(),
        ),
        QueueEvent::Download {
            item,
            event: Event::Retrying { wait, reason },
        } => activity(
            handle,
            ActivityLevel::Warning,
            "HTTP",
            "retry",
            &format!("download {item}: retrying after {wait:?}: {reason}"),
            None,
        ),
        QueueEvent::Download {
            item,
            event: Event::Finished { bytes, .. },
        } => activity(
            handle,
            ActivityLevel::Success,
            "HTTP",
            "download",
            &format!("download {item}: transfer completed ({bytes} bytes)"),
            None,
        ),
        QueueEvent::Finished { item, .. } => {
            activity(
                handle,
                ActivityLevel::Success,
                "HTTP",
                "complete",
                &format!("download {item} completed"),
                None,
            );
            handle
                .state::<crate::App>()
                .live
                .inner()
                .progress_mark
                .remove(item);
        }
        QueueEvent::Failed { item, error } => {
            activity(
                handle,
                ActivityLevel::Error,
                "HTTP",
                "download",
                &format!("download {item} failed"),
                Some(error),
            );
            handle
                .state::<crate::App>()
                .live
                .inner()
                .progress_mark
                .remove(item);
        }
        QueueEvent::Retrying { item, error } => activity(
            handle,
            ActivityLevel::Warning,
            "HTTP",
            "retry",
            &format!("download {item} will be retried"),
            Some(error),
        ),
        QueueEvent::WaitingForSite { item } => activity(
            handle,
            ActivityLevel::Info,
            "HTTP",
            "queue",
            &format!("download {item} waits for a free connection to its site"),
            None,
        ),
        QueueEvent::Stopped { item } => activity(
            handle,
            ActivityLevel::Info,
            "HTTP",
            "download",
            &format!("download {item} stopped"),
            None,
        ),
        QueueEvent::NeedsBrowser { item, .. } => activity(
            handle,
            ActivityLevel::Info,
            "HTTP",
            "browser",
            &format!("download {item} is waiting for browser credentials"),
            None,
        ),
        QueueEvent::Waiting { queue, until } => activity(
            handle,
            ActivityLevel::Info,
            "queue",
            "schedule",
            &format!("queue {queue} is waiting for its next schedule"),
            Some(&until.to_string()),
        ),
        QueueEvent::QueueDone { queue } => activity(
            handle,
            ActivityLevel::Success,
            "queue",
            "complete",
            &format!("queue {queue} completed"),
            None,
        ),
    }
    if let Some(item) = id
        && matches!(event, QueueEvent::Stopped { .. })
    {
        handle
            .state::<crate::App>()
            .live
            .inner()
            .progress_mark
            .remove(&item);
    }
}

fn sanitize_activity_text(text: &str) -> String {
    if [
        "cookie:",
        "authorization:",
        "proxy-authorization:",
        "set-cookie:",
    ]
    .iter()
    .any(|secret| text.to_ascii_lowercase().contains(secret))
    {
        return "[sensitive header redacted]".into();
    }
    let mut safe = String::new();
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let url_len = if rest
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        {
            7
        } else if rest
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        {
            8
        } else {
            0
        };
        if url_len == 0 {
            let ch = rest.chars().next().expect("at is a character boundary");
            safe.push(ch);
            at += ch.len_utf8();
            continue;
        }
        safe.push_str("[URL redacted]");
        at += url_len;
        while at < text.len() {
            let ch = text[at..]
                .chars()
                .next()
                .expect("at is a character boundary");
            if ch.is_whitespace() || "\"'<>[]{}(),".contains(ch) {
                break;
            }
            at += ch.len_utf8();
        }
    }
    safe.chars().take(MAX_ACTIVITY_TEXT).collect()
}

pub(crate) fn activity<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
    level: ActivityLevel,
    engine: &str,
    operation: &str,
    message: &str,
    details: Option<&str>,
) {
    let entry = handle
        .state::<crate::App>()
        .live
        .add_activity(level, engine, operation, message, details);
    eprintln!(
        "tdm activity [{}] {} / {}: {}{}",
        match entry.level {
            ActivityLevel::Info => "INFO",
            ActivityLevel::Success => "SUCCESS",
            ActivityLevel::Warning => "WARNING",
            ActivityLevel::Error => "ERROR",
            ActivityLevel::Debug => "DEBUG",
        },
        entry.engine,
        entry.operation,
        entry.message,
        entry
            .details
            .as_deref()
            .map(|details| format!("\n{details}"))
            .unwrap_or_default()
    );
    if let Err(error) = handle.emit("activity-event", &entry) {
        eprintln!("tdm activity: could not notify the interface: {error}");
    }
}

#[tauri::command]
pub fn activity_events(app: State<'_, crate::App>) -> Vec<ActivityEntry> {
    app.live.activity_events()
}

#[tauri::command]
pub fn clear_activity_events(app: State<'_, crate::App>) {
    app.live.clear_activity();
}

#[tauri::command]
pub async fn export_activity_log(
    path: String,
    format: String,
    entries: Vec<ActivityEntry>,
) -> Result<(), String> {
    if entries.len() > MAX_ACTIVITY_EVENTS {
        return Err("the activity export contains too many events".into());
    }
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("the activity export path must be absolute".into());
    }
    let contents = match format.as_str() {
        "json" => serde_json::to_vec_pretty(&entries).map_err(|error| error.to_string())?,
        "txt" => entries
            .iter()
            .map(|entry| {
                format!(
                    "{} [{}] {} / {}: {}{}",
                    entry.timestamp,
                    match entry.level {
                        ActivityLevel::Info => "INFO",
                        ActivityLevel::Success => "SUCCESS",
                        ActivityLevel::Warning => "WARNING",
                        ActivityLevel::Error => "ERROR",
                        ActivityLevel::Debug => "DEBUG",
                    },
                    entry.engine,
                    entry.operation,
                    entry.message,
                    entry
                        .details
                        .as_deref()
                        .map(|details| format!("\n{details}"))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
            .into_bytes(),
        _ => return Err("the activity export format must be txt or json".into()),
    };
    tokio::task::spawn_blocking(move || {
        let mut file = match tdm_platform::fs::open_regular_nofollow(&path, true) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tdm_platform::fs::create_private(&path).map_err(|error| error.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        };
        file.set_len(0).map_err(|error| error.to_string())?;
        file.write_all(&contents)
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdm_core::queue::QueueEvent;

    fn progress(live: &Live, item: u64, done: u64) {
        live.record(&QueueEvent::Download {
            item,
            event: Event::Progress {
                done,
                total: Some(1000),
            },
        });
    }

    #[test]
    fn the_page_is_told_at_most_twice_a_second_and_never_misses_the_last_change() {
        let ms = Duration::from_millis;
        assert_eq!(NOTIFY_EVERY, ms(500));
        let live = Live::default();
        let t0 = Instant::now();
        assert_eq!(live.notify_at(t0), Notify::Now);
        // Within 500 ms: later, once.
        let later = live.notify_at(t0 + ms(200));
        assert_eq!(later, Notify::Later(ms(300)));
        assert_eq!(live.notify_at(t0 + ms(400)), Notify::Scheduled);
        live.notified_later(t0 + ms(500));
        // After the scheduled one, the next waits its turn again.
        assert_eq!(live.notify_at(t0 + ms(600)), Notify::Later(ms(400)));
        live.notified_later(t0 + ms(1000));
        assert_eq!(live.notify_at(t0 + ms(1600)), Notify::Now);
    }

    /// Bytes coming in change only the running rows: the page is told
    /// those; anything else, and it reads the whole list again.
    #[test]
    fn progress_alone_is_told_apart_from_other_changes() {
        let live = Live::default();
        assert!(!live.take_changed());
        live.record(&QueueEvent::Started {
            queue: "main".into(),
            item: 1,
        });
        assert!(live.take_changed());
        assert!(!live.take_changed(), "told once");
        progress(&live, 1, 100);
        progress(&live, 1, 200);
        assert!(!live.take_changed());
        live.record(&QueueEvent::WaitingForSite { item: 1 });
        progress(&live, 1, 300);
        assert!(live.take_changed());
        live.record(&QueueEvent::Stopped { item: 1 });
        assert!(live.take_changed());
    }

    #[test]
    fn a_download_that_starts_or_ends_is_told_at_once() {
        let live = Live::default();
        let t0 = Instant::now();
        assert_eq!(live.notify_at(t0), Notify::Now);
        live.record(&QueueEvent::Started {
            queue: "main".into(),
            item: 1,
        });
        assert_eq!(live.notify_at(t0 + Duration::from_millis(10)), Notify::Now);
        live.record(&QueueEvent::Stopped { item: 1 });
        assert_eq!(live.notify_at(t0 + Duration::from_millis(20)), Notify::Now);
    }

    #[test]
    fn a_stalled_download_shows_a_speed_of_zero() {
        let live = Live::default();
        live.record(&QueueEvent::Started {
            queue: "main".into(),
            item: 1,
        });
        progress(&live, 1, 0);
        std::thread::sleep(Duration::from_millis(250));
        progress(&live, 1, 500);
        let now = Instant::now();
        assert!(live.item_at(1, now).unwrap().speed > 0.0);
        assert_eq!(live.item_at(1, now + STALLED_AFTER).unwrap().speed, 0.0);
        // Cleared when the runner stops with an error.
        live.clear();
        assert!(live.item(1).is_none());
    }

    /// #170: a download waiting for its site is marked so until it starts.
    #[test]
    fn waiting_for_its_site_lasts_until_it_starts() {
        let live = Live::default();
        assert!(!live.waiting_for_site(7));
        live.record(&QueueEvent::WaitingForSite { item: 7 });
        assert!(live.waiting_for_site(7));
        live.record(&QueueEvent::Started {
            queue: "main".into(),
            item: 7,
        });
        assert!(!live.waiting_for_site(7));
    }

    #[test]
    fn activity_entries_are_redacted_and_the_ring_keeps_only_recent_events() {
        let live = Live::default();
        let secret = live.add_activity(
            ActivityLevel::Info,
            "HTTP",
            "response",
            "requested https://example.test/video?token=SECRET",
            Some("Cookie: SESSION_SECRET"),
        );
        assert!(!secret.message.contains("SECRET"));
        assert_eq!(
            secret.details.as_deref(),
            Some("[sensitive header redacted]")
        );

        for index in 0..=MAX_ACTIVITY_EVENTS {
            live.add_activity(
                ActivityLevel::Debug,
                "FFprobe",
                "inspection",
                &format!("event {index}"),
                None,
            );
        }
        let entries = live.activity_events();
        assert_eq!(entries.len(), MAX_ACTIVITY_EVENTS);
        assert!(entries[0].id > secret.id);
        assert_eq!(entries.last().unwrap().message, "event 1000");
    }
}
