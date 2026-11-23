//! The DMG has no installer to put the app's CLI on PATH. `~/.local/bin/sunna` is a small
//! launcher script, not a link: Sunna is not linked to anything.

use std::io;
use std::path::Path;

fn herdr_bundle(exe: &Path) -> bool {
    // The bundle was called Herdr before: an old install still counts.
    exe.is_absolute() && (exe.ends_with("Sunna.app/Contents/MacOS/Sunna") || exe.ends_with("Herdr.app/Contents/MacOS/Herdr"))
}

fn installed_bundle(exe: &Path) -> bool {
    herdr_bundle(exe)
        && !exe.starts_with("/Volumes")
        && !exe.components().any(|c| c.as_os_str() == "AppTranslocation" || c.as_os_str() == "target")
}

const MARK: &str = "# sunna cli launcher";

fn launcher(exe: &Path) -> String {
    format!("#!/bin/sh\n{MARK}\nexec \"{}\" \"$@\"\n", exe.display())
}

pub(super) fn ensure(home: &Path, exe: &Path) -> io::Result<()> {
    if !installed_bundle(exe) {
        return Ok(());
    }
    let path = home.join(".local/bin/sunna");
    let want = launcher(exe);
    match std::fs::symlink_metadata(&path) {
        // An older install made a link to the app: ours if it points into a Sunna/Herdr bundle.
        Ok(meta) if meta.file_type().is_symlink() => {
            let target = std::fs::read_link(&path)?;
            let resolved = if target.is_absolute() { target } else { path.parent().unwrap().join(target) };
            if !herdr_bundle(&resolved) {
                return Ok(());
            }
            std::fs::remove_file(&path)?;
        }
        // Our own launcher: rewritten only when it names another app; anything else is the person's.
        Ok(_) => match std::fs::read_to_string(&path) {
            Ok(text) if text == want => return Ok(()),
            Ok(text) if text.contains(MARK) => {}
            _ => return Ok(()),
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, want)?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn writes_a_launcher_and_repairs_only_our_own() {
        let home = std::env::temp_dir().join(format!("sunna-cli-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let exe = home.join("Applications/Sunna.app/Contents/MacOS/Sunna");
        let path = home.join(".local/bin/sunna");
        ensure(&home, &exe).unwrap();
        assert!(!path.symlink_metadata().unwrap().file_type().is_symlink(), "a file, not a link");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), launcher(&exe));
        // Running it again leaves it alone; a launcher naming another app is rewritten.
        ensure(&home, &exe).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), launcher(&exe));
        std::fs::write(&path, format!("#!/bin/sh\n{MARK}\nexec \"/old/Sunna\"\n")).unwrap();
        ensure(&home, &exe).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), launcher(&exe));

        // An older install's link into a bundle becomes the launcher.
        for old in [
            home.join("old/Sunna.app/Contents/MacOS/Sunna"),
            "../../Applications/Herdr.app/Contents/MacOS/Herdr".into(),
        ] {
            std::fs::remove_file(&path).unwrap();
            symlink(old, &path).unwrap();
            ensure(&home, &exe).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), launcher(&exe));
        }

        // The person's own files and links stay.
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, "personal CLI").unwrap();
        ensure(&home, &exe).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "personal CLI");
        std::fs::remove_file(&path).unwrap();
        let foreign = home.join("other-app");
        symlink(&foreign, &path).unwrap();
        ensure(&home, &exe).unwrap();
        assert_eq!(std::fs::read_link(&path).unwrap(), foreign);
        std::fs::remove_file(&path).unwrap();

        for skipped in [
            "/Volumes/Sunna/Sunna.app/Contents/MacOS/Sunna",
            "/private/var/folders/AppTranslocation/random/d/Sunna.app/Contents/MacOS/Sunna",
            "/work/target/debug/Sunna.app/Contents/MacOS/Sunna",
            "/work/target/debug/sunna",
            "/Applications/Other.app/Contents/MacOS/Sunna",
        ] {
            ensure(&home, Path::new(skipped)).unwrap();
            assert!(path.symlink_metadata().is_err(), "{skipped}");
        }
        std::fs::remove_dir_all(home).unwrap();
    }
}
