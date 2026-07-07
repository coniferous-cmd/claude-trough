use anyhow::{Context, Result};

pub fn edit(initial_text: &str) -> Result<String> {
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let tmp_dir = std::env::temp_dir();
    let path = tmp_dir.join("trough_edit.md");

    std::fs::write(&path, initial_text).context("failed to write to temporary file")?;

    let status = std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("failed to launch editor '{}'", editor))?;

    if !status.success() {
        std::fs::remove_file(&path).ok();
        anyhow::bail!("editor exited with non-zero status");
    }

    let new_text =
        std::fs::read_to_string(&path).context("failed to read temporary file after editing")?;

    std::fs::remove_file(&path).context("failed to clean up temporary file")?;

    Ok(new_text)
}
