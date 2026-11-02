//! A Linux AppImage starts PandaCode with its own libraries and runtimes on the search paths
//! (`LD_LIBRARY_PATH`, `PYTHONHOME`, `GIO_*`, `GTK_*`, `XDG_DATA_DIRS`, `PATH`…), all pointing
//! into the mounted image. External tools must not inherit them: the system `/usr/bin/env`,
//! `curl` or `python3` then load the bundled copies and fail. Every spawn of an external
//! process applies `sanitize`, which drops those entries and is a no-op outside an AppImage.

use std::ffi::{OsStr, OsString};
use std::path::{Component, Path};

/// Variables the AppImage runtime sets to describe itself.
const APPIMAGE_MARKERS: [&str; 4] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

/// Whether a path lies in an AppImage's extract-and-run copy (`appimage_extracted_<hash>`).
/// A copy restarted by itself (after an update, say) inherits the previous copy's paths
/// beside its own APPDIR, and that older extraction can still be on disk; its libraries
/// break system tools just the same, for example `git` over HTTPS.
fn in_extraction(entry: &OsStr) -> bool {
    Path::new(entry).components().any(|part| matches!(part, Component::Normal(name) if name.to_str().is_some_and(|name| name.starts_with("appimage_extracted_"))))
}

/// Changes that undo the AppImage environment: `(name, Some(value))` sets, `(name, None)`
/// removes. Empty unless both `APPIMAGE` and an absolute `APPDIR` are present.
fn appimage_overrides(vars: &[(OsString, OsString)]) -> Vec<(OsString, Option<OsString>)> {
    let get = |name: &str| vars.iter().find(|(key, _)| key == name).map(|(_, value)| value);
    let (Some(_), Some(appdir)) = (get("APPIMAGE"), get("APPDIR")) else {
        return Vec::new();
    };
    let appdir = Path::new(appdir);
    if !appdir.is_absolute() || appdir.parent().is_none() {
        return Vec::new();
    }
    let inside = |entry: &OsStr| Path::new(entry).starts_with(appdir) || in_extraction(entry);
    vars.iter()
        .filter_map(|(key, value)| {
            if APPIMAGE_MARKERS.iter().any(|marker| key == marker) {
                return Some((key.clone(), None));
            }
            let entries: Vec<_> = std::env::split_paths(value).collect();
            if !entries.iter().any(|entry| inside(entry.as_os_str())) {
                return None;
            }
            let kept: Vec<_> = entries
                .into_iter()
                .filter(|entry| !entry.as_os_str().is_empty() && !inside(entry.as_os_str()))
                .collect();
            let joined = (!kept.is_empty()).then(|| std::env::join_paths(kept).ok()).flatten();
            Some((key.clone(), joined))
        })
        .collect()
}

pub(crate) trait EnvTarget {
    fn set_var(&mut self, key: &OsStr, value: &OsStr);
    fn remove_var(&mut self, key: &OsStr);
}

impl EnvTarget for std::process::Command {
    fn set_var(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }
    fn remove_var(&mut self, key: &OsStr) {
        self.env_remove(key);
    }
}

impl EnvTarget for tokio::process::Command {
    fn set_var(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }
    fn remove_var(&mut self, key: &OsStr) {
        self.env_remove(key);
    }
}

impl EnvTarget for portable_pty::CommandBuilder {
    fn set_var(&mut self, key: &OsStr, value: &OsStr) {
        self.env(key, value);
    }
    fn remove_var(&mut self, key: &OsStr) {
        self.env_remove(key);
    }
}

/// Removes this process's AppImage environment from a command about to start an external
/// tool. Does nothing on macOS and Windows, or on Linux outside an AppImage.
pub(crate) fn sanitize(command: &mut impl EnvTarget) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let vars: Vec<_> = std::env::vars_os().collect();
    for (key, value) in appimage_overrides(&vars) {
        match value {
            Some(value) => command.set_var(&key, &value),
            None => command.remove_var(&key),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs.iter().map(|(key, value)| (key.into(), value.into())).collect()
    }

    fn overrides(pairs: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
        appimage_overrides(&vars(pairs))
            .into_iter()
            .map(|(key, value)| {
                (key.into_string().unwrap(), value.map(|value| value.into_string().unwrap()))
            })
            .collect()
    }

    #[test]
    fn appimage_entries_are_removed_and_the_rest_is_kept() {
        let mount = "/tmp/.mount_pandacjIIGkD";
        let found = overrides(&[
            ("APPIMAGE", "/home/me/.local/bin/pandacode"),
            ("APPDIR", mount),
            ("ARGV0", "pandacode"),
            ("OWD", "/home/me"),
            ("PATH", "/tmp/.mount_pandacjIIGkD/usr/bin:/home/me/.local/bin:/usr/bin"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_pandacjIIGkD/usr/lib:/tmp/.mount_pandacjIIGkD/lib"),
            ("PYTHONHOME", "/tmp/.mount_pandacjIIGkD/usr"),
            ("GDK_PIXBUF_MODULE_FILE", "/tmp/.mount_pandacjIIGkD/usr/lib/loaders.cache"),
            ("XDG_DATA_DIRS", "/tmp/.mount_pandacjIIGkD/usr/share:/usr/local/share:/usr/share"),
            // A sibling mount sharing the prefix is not inside APPDIR.
            ("OTHER", "/tmp/.mount_pandacjIIGkDx/lib"),
            ("HOME", "/home/me"),
        ]);
        assert_eq!(
            found,
            [
                ("APPIMAGE".into(), None),
                ("APPDIR".into(), None),
                ("ARGV0".into(), None),
                ("OWD".into(), None),
                ("PATH".into(), Some("/home/me/.local/bin:/usr/bin".into())),
                ("LD_LIBRARY_PATH".into(), None),
                ("PYTHONHOME".into(), None),
                ("GDK_PIXBUF_MODULE_FILE".into(), None),
                ("XDG_DATA_DIRS".into(), Some("/usr/local/share:/usr/share".into())),
            ]
        );
    }

    #[test]
    fn an_earlier_extraction_inherited_by_a_restart_is_removed_too() {
        let current = "/tmp/appimage_extracted_8fbe56b022643af849a024db80d273d9";
        let found = overrides(&[
            ("APPIMAGE", "/home/me/Applications/PandaCode-Linux-x86_64.AppImage"),
            ("APPDIR", current),
            ("LD_LIBRARY_PATH", "/tmp/appimage_extracted_8fbe56b022643af849a024db80d273d9/usr/lib/:/tmp/appimage_extracted_994eac3088c340640db3d83f6d65fdbc/usr/lib/:/opt/mine/lib"),
            ("PATH", "/tmp/appimage_extracted_994eac3088c340640db3d83f6d65fdbc/usr/bin:/usr/bin"),
            ("HOME", "/home/me"),
        ]);
        assert_eq!(
            found,
            [
                ("APPIMAGE".into(), None),
                ("APPDIR".into(), None),
                ("LD_LIBRARY_PATH".into(), Some("/opt/mine/lib".into())),
                ("PATH".into(), Some("/usr/bin".into())),
            ]
        );
    }

    #[test]
    fn nothing_changes_outside_an_appimage() {
        let path = ("PATH", "/tmp/.mount_x/usr/bin:/usr/bin");
        assert!(overrides(&[path, ("HOME", "/home/me")]).is_empty());
        assert!(overrides(&[path, ("APPDIR", "/tmp/.mount_x")]).is_empty());
        assert!(overrides(&[path, ("APPIMAGE", "/a"), ("APPDIR", "relative")]).is_empty());
        assert!(overrides(&[path, ("APPIMAGE", "/a"), ("APPDIR", "/")]).is_empty());
    }
}
