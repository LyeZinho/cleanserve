use anyhow::Context;
use crate::commands::project_config;

pub async fn run() -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Cannot read current directory")?;
    project_config::uninstall_project(&project_root)
}
