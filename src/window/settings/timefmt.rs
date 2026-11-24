//! Local wall-clock time for the Updates and Privacy pages, without a date crate.
//! Unix asks the C library for the zone; elsewhere it is UTC (the pages only
//! show an hour or a date, so a few hours' offset on Windows is harmless).

/// (year, month 1-12, day, hour, minute, second) of a unix time.
pub type Civil = (i64, u32, u32, u32, u32, u32);

/// A unix time as civil UTC (Howard Hinnant's days-to-date).
pub fn utc(secs: i64) -> Civil {
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400) as u32;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Days since 1970-01-01 of a civil date.
fn days(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-01-31T12:00:00Z` (or with fractions of a second) as unix seconds.
pub fn parse_iso(s: &str) -> Option<i64> {
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let time = time.trim_end_matches('Z');
    let mut t = time.split(':');
    let h = t.next()?.parse::<i64>().ok()?;
    let mi = t.next()?.parse::<i64>().ok()?;
    let sec = t.next().map_or(Some(0), |x| x.split('.').next()?.parse::<i64>().ok())?;
    ((1..=12).contains(&m) && (1..=31).contains(&day)).then(|| days(y, m, day) * 86_400 + h * 3600 + mi * 60 + sec)
}

/// `2026-01-31T12:00:00.000Z`, as JavaScript's toISOString writes it.
pub fn iso(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = utc(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.000Z")
}

#[cfg(unix)]
pub fn local(secs: i64) -> Civil {
    use std::os::raw::{c_char, c_int, c_long};
    #[repr(C)]
    struct Tm {
        sec: c_int,
        min: c_int,
        hour: c_int,
        mday: c_int,
        mon: c_int,
        year: c_int,
        wday: c_int,
        yday: c_int,
        isdst: c_int,
        gmtoff: c_long,
        zone: *const c_char,
    }
    unsafe extern "C" {
        fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, gmtoff: 0, zone: std::ptr::null() };
    // SAFETY: `tm` matches the C struct of macOS and glibc/musl; localtime_r only writes into it.
    let ok = unsafe { !localtime_r(&secs, &mut tm).is_null() };
    if !ok {
        return utc(secs);
    }
    (i64::from(tm.year) + 1900, (tm.mon + 1) as u32, tm.mday as u32, tm.hour as u32, tm.min as u32, tm.sec as u32)
}

#[cfg(not(unix))]
pub fn local(secs: i64) -> Civil {
    utc(secs)
}

/// `HH:MM`.
pub fn clock(secs: i64) -> String {
    let (_, _, _, h, m, _) = local(secs);
    format!("{h:02}:{m:02}")
}

/// `dd-mm-yyyy HH:MM:SS`, the way Chile writes it.
pub fn stamp(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = local(secs);
    format!("{d:02}-{mo:02}-{y} {h:02}:{mi:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::{clock, iso, parse_iso, stamp, utc};

    #[test]
    fn civil_dates() {
        assert_eq!(utc(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(utc(951_782_400 + 3661), (2000, 2, 29, 1, 1, 1));
        assert_eq!(utc(1_795_000_000), (2026, 11, 18, 11, 6, 40));
    }

    #[test]
    fn iso_round_trips() {
        assert_eq!(parse_iso("2026-01-01T00:00:00Z"), Some(1_767_225_600));
        assert_eq!(parse_iso(&iso(1_795_000_000)), Some(1_795_000_000));
        assert_eq!(parse_iso("not a date"), None);
    }

    #[test]
    fn local_time_has_the_right_shape() {
        assert_eq!(clock(0).len(), 5);
        assert_eq!(stamp(1_795_000_000).len(), 19);
    }
}
