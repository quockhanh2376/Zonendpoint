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

/// Explicit Quick Scan exclusion: ONLY %SystemRoot%\System32\drivers\DriverData
/// is excluded because Windows protects it with SYSTEM-only ACLs (os error 5).
pub const QUICK_EXCLUSIONS: [&str; 1] = ["DriverData"];

pub fn targets_for_root(root: &Path) -> Vec<PathBuf> {
    CORE_FILES
        .iter()
        .chain(std::iter::once(&"drivers"))
        .map(|name| root.join(name))
        .collect()
}

pub fn exclusions_for_root(root: &Path) -> Vec<PathBuf> {
    vec![root.join("drivers").join("DriverData")]
}

/// Canonically resolve a path for comparison, stripping Windows verbatim `\\?\` prefix.
/// If `canonicalize` fails (e.g. unreadable/denied directory), walk up to the nearest
/// resolvable ancestor, canonicalize that, and re-append remaining components.
pub fn canonical_path(path: &Path) -> PathBuf {
    if let Ok(canon) = std::fs::canonicalize(path) {
        return strip_verbatim_prefix(&canon);
    }
    let mut ancestors = Vec::new();
    let mut current = Some(path);
    while let Some(p) = current {
        if let Ok(canon) = std::fs::canonicalize(p) {
            let mut result = strip_verbatim_prefix(&canon);
            for comp in ancestors.iter().rev() {
                result.push(comp);
            }
            return result;
        }
        if let Some(file_name) = p.file_name() {
            ancestors.push(file_name.to_os_string());
            current = p.parent();
        } else {
            break;
        }
    }
    normalize_components(path)
}

fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

fn normalize_components(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            c => normalized.push(c.as_os_str()),
        }
    }
    strip_verbatim_prefix(&normalized)
}

/// Check if `path` matches any exclusion in `exclusions`.
/// Matches case-insensitively on canonical absolute paths component-by-component.
/// A match requires that `path` equals the exclusion path or is a descendant within it.
/// It does NOT perform a substring or generic file-name match elsewhere.
pub fn is_path_excluded(path: &Path, exclusions: &[PathBuf]) -> bool {
    let norm_path = canonical_path(path);
    exclusions.iter().any(|ex| {
        let norm_ex = canonical_path(ex);
        path_matches_exclusion(&norm_path, &norm_ex)
    })
}

pub fn path_matches_exclusion(path: &Path, exclusion: &Path) -> bool {
    let path_comps: Vec<_> = path.components().collect();
    let ex_comps: Vec<_> = exclusion.components().collect();
    if path_comps.len() < ex_comps.len() {
        return false;
    }
    for (p, e) in path_comps.iter().zip(ex_comps.iter()) {
        let p_str = p.as_os_str().to_string_lossy();
        let e_str = e.as_os_str().to_string_lossy();
        if !p_str.eq_ignore_ascii_case(&e_str) {
            return false;
        }
    }
    true
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
        Err("Quick Scan requires Windows x64".into())
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

    #[test]
    fn quick_exclusion_resolves_only_driverdata_under_drivers() {
        let root = PathBuf::from(r"C:\Windows\System32");
        let exclusions = exclusions_for_root(&root);
        assert_eq!(exclusions.len(), 1);
        assert_eq!(
            exclusions[0],
            PathBuf::from(r"C:\Windows\System32\drivers\DriverData")
        );

        // Case-insensitive canonical match
        let exact_case = PathBuf::from(r"C:\Windows\System32\drivers\DriverData");
        let diff_case = PathBuf::from(r"c:\windows\system32\DRIVERS\driverdata");
        assert!(is_path_excluded(&exact_case, &exclusions));
        assert!(is_path_excluded(&diff_case, &exclusions));

        // Subpath of excluded directory is also excluded
        let subpath = PathBuf::from(r"C:\Windows\System32\drivers\DriverData\sub\log.txt");
        assert!(is_path_excluded(&subpath, &exclusions));

        // Substring / partial name matches MUST NOT be excluded
        let similar_name = PathBuf::from(r"C:\Windows\System32\drivers\DriverDataBackup");
        assert!(!is_path_excluded(&similar_name, &exclusions));
        let similar_name2 = PathBuf::from(r"C:\Windows\System32\drivers\DriverData_Old");
        assert!(!is_path_excluded(&similar_name2, &exclusions));

        // Folder named DriverData elsewhere MUST NOT be excluded
        let elsewhere1 = PathBuf::from(r"C:\Windows\DriverData");
        assert!(!is_path_excluded(&elsewhere1, &exclusions));
        let elsewhere2 = PathBuf::from(r"C:\Program Files\DriverData");
        assert!(!is_path_excluded(&elsewhere2, &exclusions));
        let elsewhere3 = PathBuf::from(r"D:\Windows\System32\drivers\DriverData");
        assert!(!is_path_excluded(&elsewhere3, &exclusions));

        // Other directories under drivers MUST NOT be excluded
        let other_dir = PathBuf::from(r"C:\Windows\System32\drivers\etc");
        assert!(!is_path_excluded(&other_dir, &exclusions));
    }
}
