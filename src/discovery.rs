use std::path::PathBuf;

pub fn select_first_existing(
    candidates: &[PathBuf],
    exists: impl Fn(&std::path::Path) -> bool,
) -> Option<PathBuf> {
    candidates.iter().find(|path| exists(path)).cloned()
}
