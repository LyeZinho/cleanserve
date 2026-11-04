use anyhow::Context;
use cleanserve_core::{resolve_apps, CleanServeConfig};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const RESOLVED_DROPIN: &str = "/etc/systemd/resolved.conf.d/cleanserve.conf";
const HOSTS_BEGIN: &str = "# >>> cleanserve managed >>>";
const HOSTS_END: &str = "# <<< cleanserve managed <<<";

pub async fn run(remove: bool) -> anyhow::Result<()> {
    let config_path = Path::new("cleanserve.json");
    if !config_path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }

    let config = CleanServeConfig::load(config_path).context("Failed to load config")?;
    let project_root = std::env::current_dir()?;
    let apps = resolve_apps(&config, &project_root);
    let domains: Vec<String> = apps
        .iter()
        .map(|app| app.domain.clone())
        .filter(|domain| domain != "localhost")
        .collect();
    let suffix = config.server.domain_suffix.trim_start_matches('.').to_string();

    if remove {
        remove_resolver()?;
        remove_hosts_block()?;
        remove_bind_capability()?;
        println!("✓ CleanServe domain setup removed");
        return Ok(());
    }

    if suffix.is_empty() {
        anyhow::bail!("server.domain_suffix cannot be empty");
    }

    if systemd_resolved_active() {
        install_resolver(&suffix)?;
        println!(
            "✓ Configured systemd-resolved: *.{suffix} -> 127.0.0.1:{}",
            cleanserve_core::local_dns::DEFAULT_DNS_PORT
        );
    } else {
        install_hosts_block(&domains)?;
        println!("✓ systemd-resolved not found; added a managed /etc/hosts block");
        println!("  Re-run 'cleanserve setupdomain' when the app list changes.");
    }

    if config.server.port == 443 {
        install_bind_capability()?;
        println!("✓ Allowed the cleanserve binary to bind port 443");
    }

    if domains.is_empty() {
        println!("  No multi-app domains configured yet (single app uses localhost).");
    } else {
        println!("  Domains:");
        for domain in &domains {
            println!("    https://{}", domain);
        }
    }
    println!("\nRun 'cleanserve up' to start the server.");

    Ok(())
}

fn systemd_resolved_active() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "systemd-resolved"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn install_resolver(suffix: &str) -> anyhow::Result<()> {
    let content = format!(
        "# Managed by CleanServe. Remove with: cleanserve setupdomain --remove\n\
         [Resolve]\n\
         DNS=127.0.0.1:{}\n\
         Domains=~{}\n",
        cleanserve_core::local_dns::DEFAULT_DNS_PORT,
        suffix
    );
    sudo_write(RESOLVED_DROPIN, &content)?;
    restart_resolved()?;
    Ok(())
}

fn remove_resolver() -> anyhow::Result<()> {
    let status = Command::new("sudo")
        .args(["rm", "-f", RESOLVED_DROPIN])
        .status()
        .context("Failed to run sudo rm")?;
    if !status.success() {
        anyhow::bail!("Failed to remove {}", RESOLVED_DROPIN);
    }
    restart_resolved()?;
    Ok(())
}

fn restart_resolved() -> anyhow::Result<()> {
    if !systemd_resolved_active() {
        return Ok(());
    }
    let status = Command::new("sudo")
        .args(["systemctl", "restart", "systemd-resolved"])
        .status()
        .context("Failed to restart systemd-resolved")?;
    if !status.success() {
        anyhow::bail!("Failed to restart systemd-resolved");
    }
    Ok(())
}

fn install_hosts_block(domains: &[String]) -> anyhow::Result<()> {
    let current = std::fs::read_to_string("/etc/hosts").context("Failed to read /etc/hosts")?;
    let cleaned = strip_hosts_block(&current);
    let mut next = cleaned.trim_end().to_string();
    next.push_str("\n\n");
    next.push_str(HOSTS_BEGIN);
    next.push('\n');
    for domain in domains {
        next.push_str(&format!("127.0.0.1 {}\n", domain));
    }
    next.push_str(HOSTS_END);
    next.push('\n');
    sudo_write("/etc/hosts", &next)
}

fn remove_hosts_block() -> anyhow::Result<()> {
    let current = std::fs::read_to_string("/etc/hosts").context("Failed to read /etc/hosts")?;
    let cleaned = strip_hosts_block(&current);
    if cleaned != current {
        sudo_write("/etc/hosts", &cleaned)?;
    }
    Ok(())
}

fn strip_hosts_block(content: &str) -> String {
    let mut output = String::new();
    let mut inside = false;
    for line in content.lines() {
        if line.trim() == HOSTS_BEGIN {
            inside = true;
            continue;
        }
        if line.trim() == HOSTS_END {
            inside = false;
            continue;
        }
        if !inside {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn install_bind_capability() -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("Failed to locate cleanserve binary")?;
    let status = Command::new("sudo")
        .args([
            "setcap",
            "CAP_NET_BIND_SERVICE=+eip",
            &exe.to_string_lossy(),
        ])
        .status()
        .context("Failed to run setcap")?;
    if !status.success() {
        anyhow::bail!("Failed to apply CAP_NET_BIND_SERVICE to {}", exe.display());
    }
    Ok(())
}

fn remove_bind_capability() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let _ = Command::new("sudo")
        .args(["setcap", "-r", &exe.to_string_lossy()])
        .status();
    Ok(())
}

fn sudo_write(path: &str, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        let status = Command::new("sudo")
            .args(["mkdir", "-p", &parent.to_string_lossy()])
            .status()
            .context("Failed to run sudo mkdir")?;
        if !status.success() {
            anyhow::bail!("Failed to create {}", parent.display());
        }
    }

    let mut child = Command::new("sudo")
        .args(["tee", path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("Failed to run sudo tee")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(content.as_bytes())
            .context("Failed to write to sudo tee")?;
    }
    let status = child.wait().context("Failed to wait for sudo tee")?;
    if !status.success() {
        anyhow::bail!("Failed to write {}", path);
    }
    Ok(())
}
