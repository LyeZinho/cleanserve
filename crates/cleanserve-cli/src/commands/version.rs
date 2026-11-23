use super::update::install_info;
use crate::tui::pick_release;
use cleanserve_core::auto_updater::{UpdateChecker, GitHubRelease};
use std::io::IsTerminal;

const CURRENT: &str = env!("CARGO_PKG_VERSION");

fn interactive(no_tui: bool) -> bool {
    !no_tui && std::io::stdout().is_terminal() && std::io::stdin().is_terminal()
}

async fn install(release: &GitHubRelease, force: bool) -> anyhow::Result<()> {
    let label = release.tag_name.trim_start_matches('v');
    if label == CURRENT && !force {
        println!("✓ Already on v{CURRENT} (use --force to reinstall)");
        return Ok(());
    }
    let info = UpdateChecker::evaluate(CURRENT, release).map_err(|e| anyhow::anyhow!(e))?;
    install_info(&info).await
}

/// `cleanserve version [VERSION]` — pick a release interactively or install a specific one.
pub async fn run(version: Option<String>, force: bool, no_tui: bool) -> anyhow::Result<()> {
    if let Some(v) = version {
        let release = if v == "latest" {
            UpdateChecker::list_releases()
                .await
                .map_err(|e| anyhow::anyhow!(e))?
                .into_iter()
                .find(|r| !r.prerelease)
                .ok_or_else(|| anyhow::anyhow!("No releases found"))?
        } else {
            UpdateChecker::fetch_release(&v).await.map_err(|e| anyhow::anyhow!(e))?
        };
        return install(&release, force).await;
    }

    let releases = UpdateChecker::list_releases().await.map_err(|e| anyhow::anyhow!(e))?;
    if releases.is_empty() {
        anyhow::bail!("No releases found");
    }
    if interactive(no_tui) {
        if let Some(r) = pick_release(&releases, CURRENT)? {
            install(&r, force).await?;
        }
    } else {
        println!("Installed: v{CURRENT}\n");
        for r in &releases {
            let label = r.tag_name.trim_start_matches('v');
            println!("  v{}{}", label, if label == CURRENT { " ●" } else { "" });
        }
        println!("\nInstall one with: cleanserve version <version>");
    }
    Ok(())
}
