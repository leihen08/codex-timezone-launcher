use std::path::Path;

pub fn should_block_for_process(
    target: &Path,
    process_name: &str,
    process_path: Option<&Path>,
) -> bool {
    if process_name.eq_ignore_ascii_case("ChatGPT.exe") {
        return true;
    }
    if !process_name.eq_ignore_ascii_case("Codex.exe") {
        return false;
    }

    process_path.is_some_and(|running_path| {
        target
            .to_string_lossy()
            .eq_ignore_ascii_case(&running_path.to_string_lossy())
    })
}
