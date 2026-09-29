//! Free disk space for the folder where models are saved.

use std::path::Path;

use serde::Serialize;
use sysinfo::Disks;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiskSpace {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// Finds the disk that holds `path` (the mount point that matches the longest part of the path).
/// `path` does not need to exist yet.
pub fn space_for(path: &Path) -> Option<DiskSpace> {
    // dunce: `C:\Users\…` instead of `\\?\C:\Users\…`, so it matches the mount point.
    let target = dunce::canonicalize(existing_ancestor(path)?).ok()?;
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .filter(|d| target.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| DiskSpace {
            free_bytes: d.available_space(),
            total_bytes: d.total_space(),
        })
}

fn existing_ancestor(path: &Path) -> Option<&Path> {
    path.ancestors().find(|p| p.exists())
}

#[cfg(test)]
mod tests {
    #[test]
    fn finds_space_for_a_missing_folder() {
        let space = super::space_for(&std::env::temp_dir().join("kuttylm-no-such-dir/x")).unwrap();
        assert!(space.total_bytes > 0);
        assert!(space.free_bytes <= space.total_bytes);
    }
}
