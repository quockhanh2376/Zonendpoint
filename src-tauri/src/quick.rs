use std::path::{Path, PathBuf};

pub const CORE_FILES: [&str; 13] = [
    "ntoskrnl.exe",
    "ntdll.dll",
    "kernel32.dll",
    "kernelbase.dll",
    "ci.dll",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "winlogon.exe",
    "winload.efi",
];

pub fn targets_for_root(root: &Path) -> Vec<PathBuf> {
    CORE_FILES
        .iter()
        .chain(std::iter::once(&"drivers"))
        .map(|name| root.join(name))
        .collect()
}

pub fn system_root() -> Result<PathBuf, String> {
    #[cfg(all(feature = "native-ui-tests", debug_assertions))]
    if let Some(root) = std::env::var_os("ZONENDPOINT_TEST_SYSTEM_ROOT") {
        return Ok(PathBuf::from(root));
    }
    #[cfg(windows)]
    {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetSystemDirectoryW(buffer: *mut u16, size: u32) -> u32;
        }
        let mut buffer = vec![0u16; 32768];
        // Native x64 process: no WOW64 redirection and no environment-provided root.
        let length =
            unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
        if length == 0 || length >= buffer.len() {
            return Err("Windows system directory is unavailable".into());
        }
        Ok(PathBuf::from(OsString::from_wide(&buffer[..length])))
    }
    #[cfg(not(windows))]
    {
        Err("Quick scan requires Windows x64".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(windows, not(feature = "native-ui-tests")))]
    #[test]
    fn system_root_comes_from_native_windows_api() {
        let root = system_root().unwrap();
        assert!(root.is_absolute());
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .eq_ignore_ascii_case("System32")
        );
    }
    #[test]
    fn quick_manifest_is_bounded_and_stays_inside_native_system_root() {
        let root = PathBuf::from("system-root");
        let targets = targets_for_root(&root);
        assert_eq!(targets.len(), 14);
        assert!(targets.contains(&root.join("ntoskrnl.exe")));
        assert!(targets.contains(&root.join("drivers")));
        assert!(targets.iter().all(|p| p.parent() == Some(root.as_path())));
        assert_eq!(
            targets
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            14
        );
    }
}
