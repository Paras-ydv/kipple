//! Network mounts, which the scan never enters unless
//! `scan.include_network = true` (SPEC §5.1).

use std::path::PathBuf;

/// Mount points of network filesystems.
#[cfg(target_os = "linux")]
pub fn network_mounts() -> Vec<PathBuf> {
    const NETWORK: &[&str] = &[
        "nfs",
        "nfs4",
        "cifs",
        "smb3",
        "smbfs",
        "ncpfs",
        "afs",
        "fuse.sshfs",
        "9p",
        "davfs",
        "fuse.rclone",
        "ceph",
        "glusterfs",
    ];
    let Ok(text) = std::fs::read_to_string("/proc/self/mounts") else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let _source = parts.next()?;
            let target = parts.next()?;
            let fstype = parts.next()?;
            NETWORK
                .contains(&fstype)
                .then(|| PathBuf::from(unescape_mount(target)))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn unescape_mount(s: &str) -> String {
    s.replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\134", "\\")
}

#[cfg(target_os = "macos")]
pub fn network_mounts() -> Vec<PathBuf> {
    use std::ffi::CStr;
    let mut buf: *mut libc::statfs = std::ptr::null_mut();
    // SAFETY: getmntinfo fills `buf` with a pointer to an array it owns and
    // returns its length; we only read within that length.
    let n = unsafe { libc::getmntinfo(&mut buf, libc::MNT_NOWAIT) };
    if n <= 0 || buf.is_null() {
        return Vec::new();
    }
    // SAFETY: see above; `n` entries are valid.
    let mounts = unsafe { std::slice::from_raw_parts(buf, n as usize) };
    mounts
        .iter()
        .filter(|m| m.f_flags & libc::MNT_LOCAL as u32 == 0)
        .map(|m| {
            // SAFETY: f_mntonname is a NUL-terminated C string.
            let name = unsafe { CStr::from_ptr(m.f_mntonname.as_ptr()) };
            PathBuf::from(name.to_string_lossy().into_owned())
        })
        .collect()
}

/// Windows network drives are mapped letters outside the profile; v1.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn network_mounts() -> Vec<PathBuf> {
    Vec::new()
}
