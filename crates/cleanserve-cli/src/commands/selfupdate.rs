//! CleanServe self-management: list releases, install latest or a specific version.

use anyhow::{bail, Context};
use cleanserve_core::auto_updater::UpdateInstaller;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

const REPO: &str = "LyeZinho/cleanserve";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn version(&self) -> Option<Version> {
        Version::parse(self.tag_name.trim_start_matches('v')).ok()
    }

    pub fn label(&self) -> String {
        self.tag_name.trim_start_matches('v').to_string()
    }

    pub fn is_current(&self) -> bool {
        self.label() == CURRENT
    }

    /// Release archive for this OS/arch, e.g. `cleanserve-linux-x86_64.tar.gz`.
    pub fn platform_asset(&self) -> Option<&Asset> {
        let prefix = format!("cleanserve-{}-{}", os_name(), arch_name());
        self.assets.iter().find(|a| a.name.starts_with(&prefix))
    }

    fn checksums_asset(&self) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == "SHA256SUMS")
    }
}

fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        other => other,
    }
}

fn arch_name() -> &'static str {
    std::env::consts::ARCH
}

fn client() -> anyhow::Result<reqwest::Client> {
    // GitHub release downloads redirect to its CDN; only follow https hops to GitHub hosts.
    let policy = reqwest::redirect::Policy::custom(|attempt| {
        let ok = attempt.previous().len() < 5
            && attempt.url().scheme() == "https"
            && attempt.url().host_str().is_some_and(|h| {
                h == "github.com" || h.ends_with(".github.com") || h.ends_with(".githubusercontent.com")
            });
        if ok {
            attempt.follow()
        } else {
            attempt.stop()
        }
    });
    reqwest::Client::builder()
        .redirect(policy)
        .user_agent(format!("cleanserve/{}", CURRENT))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .context("Failed to build HTTP client")
}

pub async fn fetch_releases() -> anyhow::Result<Vec<Release>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=30");
    let res = client()?.get(url).send().await.context("Could not reach GitHub")?;
    if !res.status().is_success() {
        bail!("GitHub API returned {}", res.status());
    }
    let mut releases: Vec<Release> = res.json().await.context("Invalid GitHub response")?;
    releases.sort_by(|a, b| b.version().cmp(&a.version()));
    Ok(releases)
}

pub async fn latest_release() -> anyhow::Result<Release> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let res = client()?.get(url).send().await.context("Could not reach GitHub")?;
    if !res.status().is_success() {
        bail!("GitHub API returned {}", res.status());
    }
    Ok(res.json().await.context("Invalid GitHub response")?)
}

pub async fn release_by_version(version: &str) -> anyhow::Result<Release> {
    let v = normalize_version(version)?;
    let url = format!("https://api.github.com/repos/{REPO}/releases/tags/v{v}");
    let res = client()?.get(url).send().await.context("Could not reach GitHub")?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        bail!("Version {v} does not exist. Run 'cleanserve version' to see available versions.");
    }
    if !res.status().is_success() {
        bail!("GitHub API returned {}", res.status());
    }
    Ok(res.json().await.context("Invalid GitHub response")?)
}

pub fn normalize_version(input: &str) -> anyhow::Result<String> {
    let v = input.trim().trim_start_matches('v');
    Version::parse(v).with_context(|| format!("'{input}' is not a valid version (expected e.g. 0.3.1)"))?;
    Ok(v.to_string())
}

#[derive(Clone, Debug)]
pub enum Progress {
    Stage(Stage),
    Download { done: u64, total: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Backup,
    Download,
    Verify,
    Install,
    Check,
    Done,
}

impl Stage {
    pub const ALL: [Stage; 5] = [Stage::Backup, Stage::Download, Stage::Verify, Stage::Install, Stage::Check];

    pub fn label(self) -> &'static str {
        match self {
            Stage::Backup => "Back up current binary",
            Stage::Download => "Download release",
            Stage::Verify => "Verify checksum",
            Stage::Install => "Install",
            Stage::Check => "Sanity check",
            Stage::Done => "Done",
        }
    }
}

/// Download, verify and atomically install `release`, rolling back on failure.
pub async fn install_release(release: &Release, tx: UnboundedSender<Progress>) -> anyhow::Result<()> {
    let asset = release.platform_asset().with_context(|| {
        format!(
            "Release {} has no build for {}-{}",
            release.label(),
            os_name(),
            arch_name()
        )
    })?;
    let target = std::env::current_exe().context("Cannot locate the running binary")?;
    let target = std::fs::canonicalize(&target).unwrap_or(target);

    let _ = tx.send(Progress::Stage(Stage::Backup));
    let backup = UpdateInstaller::backup_current_binary(&target, None).map_err(|e| anyhow::anyhow!(e.message))?;

    let tmp = tempfile::tempdir().context("Failed to create temp dir")?;
    let archive = tmp.path().join(&asset.name);

    let _ = tx.send(Progress::Stage(Stage::Download));
    let digest = download(&asset.browser_download_url, &archive, asset.size, &tx).await?;

    let _ = tx.send(Progress::Stage(Stage::Verify));
    verify_checksum(release, &asset.name, &digest).await?;

    let new_bin = extract_binary(&archive, tmp.path())?;

    let _ = tx.send(Progress::Stage(Stage::Install));
    replace_binary(&new_bin, &target).await?;

    let _ = tx.send(Progress::Stage(Stage::Check));
    if let Err(e) = UpdateInstaller::verify_installation(&target).await {
        let _ = std::fs::copy(&backup, &target);
        bail!("New binary failed its sanity check, rolled back: {}", e.message);
    }
    let _ = UpdateInstaller::cleanup_old_backups(None);
    ensure_short_alias(&target);

    let _ = tx.send(Progress::Stage(Stage::Done));
    Ok(())
}

async fn download(
    url: &str,
    dest: &Path,
    expected_size: u64,
    tx: &UnboundedSender<Progress>,
) -> anyhow::Result<String> {
    use tokio::io::AsyncWriteExt;
    let mut res = client()?.get(url).send().await.context("Download failed")?;
    if !res.status().is_success() {
        bail!("Download failed: HTTP {}", res.status());
    }
    let total = res.content_length().unwrap_or(expected_size);
    let mut file = tokio::fs::File::create(dest).await?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    while let Some(chunk) = res.chunk().await.context("Download interrupted")? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        done += chunk.len() as u64;
        let _ = tx.send(Progress::Download { done, total });
    }
    file.flush().await?;
    Ok(format!("{:x}", hasher.finalize()))
}

async fn verify_checksum(release: &Release, asset_name: &str, actual: &str) -> anyhow::Result<()> {
    let Some(sums) = release.checksums_asset() else {
        eprintln!("⚠️  Release publishes no SHA256SUMS; skipping checksum verification");
        return Ok(());
    };
    let text = client()?
        .get(&sums.browser_download_url)
        .send()
        .await?
        .error_for_status()
        .context("Failed to fetch SHA256SUMS")?
        .text()
        .await?;
    let expected = text
        .lines()
        .find(|l| l.split_whitespace().nth(1).map(|n| n.trim_start_matches('*')) == Some(asset_name))
        .and_then(|l| l.split_whitespace().next())
        .with_context(|| format!("No checksum for {asset_name} in SHA256SUMS"))?;
    if !expected.eq_ignore_ascii_case(actual) {
        bail!("Checksum mismatch for {asset_name}: expected {expected}, got {actual}");
    }
    Ok(())
}

fn extract_binary(archive: &Path, out_dir: &Path) -> anyhow::Result<PathBuf> {
    let bin_name = if cfg!(windows) { "cleanserve.exe" } else { "cleanserve" };
    let dest = out_dir.join(format!("new-{bin_name}"));

    if archive.extension().is_some_and(|e| e == "zip") {
        #[cfg(windows)]
        {
            let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?)?;
            let mut entry = zip.by_name(bin_name).context("Binary not found in archive")?;
            std::io::copy(&mut entry, &mut std::fs::File::create(&dest)?)?;
            return Ok(dest);
        }
        #[cfg(not(windows))]
        bail!("zip archives are only supported on Windows");
    }

    let gz = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries()? {
        let mut entry = entry?;
        if entry.path()?.file_name().is_some_and(|n| n == bin_name) {
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(dest);
        }
    }
    bail!("Binary '{bin_name}' not found in archive")
}

async fn replace_binary(new_bin: &Path, target: &Path) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        // A running .exe can be renamed but not overwritten.
        let old = target.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(target, &old).context("Failed to move the running binary aside")?;
    }
    UpdateInstaller::install_binary(new_bin, target)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "{} (if this is a permission error, re-run with sudo or reinstall into ~/.local/bin)",
                e.message
            )
        })
}

/// Make `cs` available next to `cleanserve` (symlink on Unix).
pub fn ensure_short_alias(target: &Path) {
    #[cfg(unix)]
    if let Some(dir) = target.parent() {
        let alias = dir.join("cs");
        if std::fs::symlink_metadata(&alias).is_err() {
            let _ = std::os::unix::fs::symlink(target.file_name().unwrap_or_default(), alias);
        }
    }
    #[cfg(not(unix))]
    let _ = target;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_versions() {
        assert_eq!(normalize_version("v0.3.1").unwrap(), "0.3.1");
        assert_eq!(normalize_version("1.2.3").unwrap(), "1.2.3");
        assert!(normalize_version("abc").is_err());
    }

    #[test]
    fn picks_platform_asset() {
        let name = format!("cleanserve-{}-{}.tar.gz", os_name(), arch_name());
        let rel = Release {
            tag_name: "v9.9.9".into(),
            body: None,
            published_at: None,
            prerelease: false,
            assets: vec![
                Asset { name: "other.tar.gz".into(), browser_download_url: "x".into(), size: 1 },
                Asset { name, browser_download_url: "y".into(), size: 2 },
            ],
        };
        assert_eq!(rel.platform_asset().unwrap().browser_download_url, "y");
        assert_eq!(rel.label(), "9.9.9");
    }
}
