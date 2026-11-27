//! Environment variables under both names: `SUNNA_<name>` wins, `HERDR_<name>` still works.
use std::ffi::OsString;

pub fn var_os(name: &str) -> Option<OsString> {
    std::env::var_os(format!("SUNNA_{name}")).or_else(|| std::env::var_os(format!("HERDR_{name}")))
}

pub fn var(name: &str) -> Option<String> {
    var_os(name).and_then(|v| v.into_string().ok())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_new_name_wins_and_the_old_one_still_reads() {
        // SAFETY: a name only this test uses.
        unsafe {
            std::env::set_var("HERDR_ENVX_PROBE", "old");
            assert_eq!(super::var("ENVX_PROBE").as_deref(), Some("old"));
            std::env::set_var("SUNNA_ENVX_PROBE", "new");
            assert_eq!(super::var("ENVX_PROBE").as_deref(), Some("new"));
        }
    }
}
