use super::selfupdate::{self, Progress, Release, CURRENT};
use super::tui;
use semver::Version;
use std::io::IsTerminal;

fn interactive(no_tui: bool) -> bool {
    !no_tui && std::io::stdout().is_terminal() && std::io::stdin().is_terminal()
}

async fn install(release: &Release, no_tui: bool) -> anyhow::Result<()> {
    if interactive(no_tui) {
        tui::install_screen(release).await?;
    } else {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let rel = release.clone();
        let task = tokio::spawn(async move { selfupdate::install_release(&rel, tx).await });
        let mut last_pct = u64::MAX;
        while let Some(p) = rx.recv().await {
            match p {
                Progress::Stage(s) => println!("→ {}", s.label()),
                Progress::Download { done, total } if total > 0 => {
                    let pct = done * 100 / total / 10 * 10;
                    if pct != last_pct {
                        last_pct = pct;
                        println!("   {pct}%");
                    }
                }
                _ => {}
            }
        }
        task.await??;
    }
    println!("✅ CleanServe v{} installed (was v{})", release.label(), CURRENT);
    Ok(())
}

/// `cleanserve update` — install the latest release.
pub async fn run_update(check_only: bool, force: bool, no_tui: bool) -> anyhow::Result<()> {
    println!("Checking for CleanServe updates...");
    let latest = selfupdate::latest_release().await?;
    let newer = matches!(
        (latest.version(), Version::parse(CURRENT).ok()),
        (Some(l), Some(c)) if l > c
    );
    println!("Current: v{CURRENT}   Latest: v{}", latest.label());

    if !newer && !force {
        println!("✓ Already up to date!");
        return Ok(());
    }
    if check_only {
        println!("✓ Update available: v{CURRENT} → v{}", latest.label());
        return Ok(());
    }
    install(&latest, no_tui).await
}

/// `cleanserve version [VERSION]` — pick or install a specific version.
pub async fn run_version(version: Option<String>, force: bool, no_tui: bool) -> anyhow::Result<()> {
    match version {
        Some(v) => {
            let release = if v == "latest" {
                selfupdate::latest_release().await?
            } else {
                selfupdate::release_by_version(&v).await?
            };
            if release.is_current() && !force {
                println!("✓ Already on v{CURRENT} (use --force to reinstall)");
                return Ok(());
            }
            install(&release, no_tui).await
        }
        None => {
            let releases = selfupdate::fetch_releases().await?;
            if releases.is_empty() {
                anyhow::bail!("No releases found");
            }
            if interactive(no_tui) {
                if let Some(r) = tui::pick_release(&releases)? {
                    if r.is_current() && !force {
                        println!("✓ Already on v{CURRENT}");
                        return Ok(());
                    }
                    install(&r, no_tui).await?;
                }
            } else {
                println!("Installed: v{CURRENT}\n");
                for r in &releases {
                    let mark = if r.is_current() { " ●" } else { "" };
                    println!("  v{}{}", r.label(), mark);
                }
                println!("\nInstall one with: cleanserve version <version>");
            }
            Ok(())
        }
    }
}
