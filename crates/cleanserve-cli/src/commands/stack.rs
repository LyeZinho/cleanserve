use anyhow::Context;
use cleanserve_core::{CleanServeConfig, StackService};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const COMPOSE_DIR: &str = ".cleanserve/stack";

#[derive(Clone, Copy)]
struct ServiceDef {
    name: &'static str,
    image: &'static str,
    description: &'static str,
    default_port: u16,
    internal_port: u16,
    volume: Option<&'static str>,
}

const SERVICES: &[ServiceDef] = &[
    ServiceDef {
        name: "mysql",
        image: "mysql:8.0",
        description: "MySQL relational database",
        default_port: 3306,
        internal_port: 3306,
        volume: Some("/var/lib/mysql"),
    },
    ServiceDef {
        name: "mariadb",
        image: "mariadb:11",
        description: "MariaDB relational database",
        default_port: 3307,
        internal_port: 3306,
        volume: Some("/var/lib/mysql"),
    },
    ServiceDef {
        name: "postgres",
        image: "postgres:16",
        description: "PostgreSQL relational database",
        default_port: 5432,
        internal_port: 5432,
        volume: Some("/var/lib/postgresql/data"),
    },
    ServiceDef {
        name: "redis",
        image: "redis:7-alpine",
        description: "Redis in-memory store",
        default_port: 6379,
        internal_port: 6379,
        volume: Some("/data"),
    },
    ServiceDef {
        name: "mailpit",
        image: "axllent/mailpit:latest",
        description: "Local SMTP server and web mail viewer",
        default_port: 8025,
        internal_port: 8025,
        volume: None,
    },
    ServiceDef {
        name: "phpmyadmin",
        image: "phpmyadmin:latest",
        description: "phpMyAdmin web UI for MySQL/MariaDB",
        default_port: 8081,
        internal_port: 80,
        volume: None,
    },
    ServiceDef {
        name: "adminer",
        image: "adminer:latest",
        description: "Adminer web UI for databases",
        default_port: 8082,
        internal_port: 8080,
        volume: None,
    },
];

fn find_service(name: &str) -> Option<&'static ServiceDef> {
    SERVICES.iter().find(|service| service.name == name)
}

pub async fn add(
    name: &str,
    version: Option<&str>,
    port: Option<u16>,
    project_root: &Path,
) -> anyhow::Result<()> {
    let service = find_service(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown service '{}'. Run 'cleanserve stack list'.", name))?;

    let mut config = load_config(project_root)?;
    config.stack.insert(
        name.to_string(),
        StackService {
            version: version.map(str::to_string),
            enabled: true,
            port: port.or(Some(service.default_port)),
            env: HashMap::new(),
        },
    );
    save_config(project_root, &config)?;
    write_compose(project_root, &config)?;

    println!(
        "✓ Added {} ({}) on port {}",
        name,
        service.description,
        port.unwrap_or(service.default_port)
    );
    println!("  Start with: cleanserve stack up");
    Ok(())
}

pub async fn remove(name: &str, project_root: &Path) -> anyhow::Result<()> {
    let mut config = load_config(project_root)?;
    if config.stack.remove(name).is_none() {
        anyhow::bail!("Service '{}' is not configured in this project", name);
    }
    save_config(project_root, &config)?;
    write_compose(project_root, &config)?;

    if docker_available() {
        let _ = compose(project_root, &["stop", name]).status();
    }
    println!("✓ Removed {} from the project stack", name);
    Ok(())
}

pub fn list(project_root: &Path) -> anyhow::Result<()> {
    let config = load_config(project_root).unwrap_or_else(|_| empty_config());
    println!("Available services:");
    for service in SERVICES {
        let marker = if config.stack.contains_key(service.name) {
            "*"
        } else {
            " "
        };
        println!(
            " {} {:<12} {:<42} default port {}",
            marker, service.name, service.description, service.default_port
        );
    }
    if !config.stack.is_empty() {
        println!("\nConfigured in this project:");
        for (name, service) in &config.stack {
            println!(
                "  {} ({}, enabled: {}, port: {})",
                name,
                service.version.as_deref().unwrap_or("default"),
                service.enabled,
                service.port.unwrap_or(0)
            );
        }
    }
    Ok(())
}

pub async fn up(services: &[String], project_root: &Path) -> anyhow::Result<()> {
    ensure_docker()?;
    let config = load_config(project_root)?;
    if config.stack.is_empty() {
        anyhow::bail!("No services configured. Use 'cleanserve stack add <name>' first.");
    }
    write_compose(project_root, &config)?;

    let mut args = vec!["up", "-d"];
    args.extend(services.iter().map(String::as_str));
    run_compose(project_root, &args)?;
    print_endpoints(&config);
    Ok(())
}

pub async fn down(project_root: &Path) -> anyhow::Result<()> {
    ensure_docker()?;
    run_compose(project_root, &["down"])?;
    println!("✓ Stack stopped");
    Ok(())
}

pub async fn status(project_root: &Path) -> anyhow::Result<()> {
    ensure_docker()?;
    run_compose(project_root, &["ps"])?;
    Ok(())
}

pub async fn logs(name: Option<&str>, project_root: &Path) -> anyhow::Result<()> {
    ensure_docker()?;
    let mut args = vec!["logs", "-f"];
    if let Some(name) = name {
        args.push(name);
    }
    run_compose(project_root, &args)?;
    Ok(())
}

/// Start enabled services automatically when `cleanserve up` runs.
pub async fn auto_start(project_root: &Path) -> anyhow::Result<()> {
    let config = load_config(project_root)?;
    let enabled: Vec<String> = config
        .stack
        .iter()
        .filter(|(_, service)| service.enabled)
        .map(|(name, _)| name.clone())
        .collect();
    if enabled.is_empty() {
        return Ok(());
    }

    if !docker_available() {
        println!("⚠️  Stack services are configured but Docker is not installed.");
        println!("   Install Docker, then run: cleanserve stack up");
        return Ok(());
    }

    write_compose(project_root, &config)?;
    let mut args = vec!["up", "-d"];
    args.extend(enabled.iter().map(String::as_str));
    run_compose(project_root, &args)?;
    print_endpoints(&config);
    Ok(())
}

fn print_endpoints(config: &CleanServeConfig) {
    println!("🐳 Stack services:");
    for (name, service) in &config.stack {
        let Some(def) = find_service(name) else {
            continue;
        };
        let port = service.port.unwrap_or(def.default_port);
        if name == "mailpit" {
            println!("   {}: web http://localhost:{} (SMTP localhost:1025)", name, port);
        } else {
            println!("   {}: localhost:{}", name, port);
        }
    }
}

fn load_config(project_root: &Path) -> anyhow::Result<CleanServeConfig> {
    let path = project_root.join("cleanserve.json");
    if !path.exists() {
        anyhow::bail!("No cleanserve.json found. Run 'cleanserve init' first.");
    }
    CleanServeConfig::load(&path).context("Failed to load cleanserve.json")
}

fn save_config(project_root: &Path, config: &CleanServeConfig) -> anyhow::Result<()> {
    config
        .save(project_root.join("cleanserve.json"))
        .context("Failed to save cleanserve.json")
}

fn empty_config() -> CleanServeConfig {
    CleanServeConfig {
        name: "project".to_string(),
        engine: cleanserve_core::EngineConfig {
            php: "8.4".to_string(),
            extensions: vec![],
            display_errors: true,
            memory_limit: None,
        },
        server: cleanserve_core::ServerConfig::default(),
        packages: None,
        apps: HashMap::new(),
        stack: HashMap::new(),
    }
}

fn compose_path(project_root: &Path) -> PathBuf {
    project_root.join(COMPOSE_DIR).join("docker-compose.yml")
}

fn write_compose(project_root: &Path, config: &CleanServeConfig) -> anyhow::Result<()> {
    let dir = project_root.join(COMPOSE_DIR);
    std::fs::create_dir_all(&dir).context("Failed to create .cleanserve/stack")?;

    let mut yaml = String::from("# Generated by CleanServe. Do not edit by hand.\n");
    yaml.push_str("services:\n");

    for (name, service) in &config.stack {
        let Some(def) = find_service(name) else {
            continue;
        };
        let image = match (&service.version, name.as_str()) {
            (Some(version), "mysql") => format!("mysql:{}", version),
            (Some(version), "mariadb") => format!("mariadb:{}", version),
            (Some(version), "postgres") => format!("postgres:{}", version),
            (Some(version), "redis") => format!("redis:{}", version),
            (Some(version), _) => format!("{}:{}", def.image.split(':').next().unwrap_or(name), version),
            (None, _) => def.image.to_string(),
        };
        yaml.push_str(&format!("  {}:\n", name));
        yaml.push_str(&format!("    image: {}\n", image));
        yaml.push_str("    restart: unless-stopped\n");

        let port = service.port.unwrap_or(def.default_port);
        yaml.push_str("    ports:\n");
        yaml.push_str(&format!("      - \"{}:{}\"\n", port, def.internal_port));
        if name == "mailpit" {
            yaml.push_str("      - \"1025:1025\"\n");
        }

        let env = default_env(name, &service.env);
        if !env.is_empty() {
            yaml.push_str("    environment:\n");
            for (key, value) in env {
                yaml.push_str(&format!("      {}: \"{}\"\n", key, value.replace('"', "\\\"")));
            }
        }

        if let Some(target) = def.volume {
            yaml.push_str("    volumes:\n");
            yaml.push_str(&format!("      - ./data/{}:{}\n", name, target));
        }

        if matches!(name.as_str(), "phpmyadmin" | "adminer") {
            yaml.push_str("    depends_on:\n");
            yaml.push_str("      - mysql\n");
        }
    }

    std::fs::write(compose_path(project_root), yaml).context("Failed to write docker-compose.yml")?;
    Ok(())
}

fn default_env(name: &str, overrides: &HashMap<String, String>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    match name {
        "mysql" | "mariadb" => {
            env.insert("MYSQL_ROOT_PASSWORD".into(), "cleanserve".into());
            env.insert("MYSQL_DATABASE".into(), "app".into());
            env.insert("MYSQL_USER".into(), "app".into());
            env.insert("MYSQL_PASSWORD".into(), "secret".into());
        }
        "postgres" => {
            env.insert("POSTGRES_DB".into(), "app".into());
            env.insert("POSTGRES_USER".into(), "app".into());
            env.insert("POSTGRES_PASSWORD".into(), "secret".into());
        }
        "phpmyadmin" => {
            env.insert("PMA_HOST".into(), "mysql".into());
            env.insert("PMA_USER".into(), "root".into());
            env.insert("PMA_PASSWORD".into(), "cleanserve".into());
        }
        _ => {}
    }
    for (key, value) in overrides {
        env.insert(key.clone(), value.clone());
    }
    env
}

fn project_name(project_root: &Path, config: &CleanServeConfig) -> String {
    let raw = if config.name.is_empty() {
        project_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "cleanserve".to_string())
    } else {
        config.name.clone()
    };
    let sanitized: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("cleanserve-{}", sanitized.trim_matches('-').to_lowercase())
}

fn compose_command(project_root: &Path) -> anyhow::Result<Command> {
    let config = load_config(project_root)?;
    let mut command = Command::new("docker");
    command
        .arg("compose")
        .arg("-f")
        .arg(compose_path(project_root))
        .arg("-p")
        .arg(project_name(project_root, &config));
    Ok(command)
}

fn compose(project_root: &Path, args: &[&str]) -> Command {
    let mut command = compose_command(project_root).unwrap_or_else(|_| Command::new("docker"));
    command.args(args);
    command
}

fn run_compose(project_root: &Path, args: &[&str]) -> anyhow::Result<()> {
    let status = compose(project_root, args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run docker compose")?;
    if !status.success() {
        anyhow::bail!("docker compose {} failed", args.join(" "));
    }
    Ok(())
}

fn docker_available() -> bool {
    Command::new("docker")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_docker() -> anyhow::Result<()> {
    if !docker_available() {
        anyhow::bail!(
            "Docker is required for 'cleanserve stack'. Install Docker and the Compose plugin, then retry."
        );
    }
    let compose_ok = Command::new("docker")
        .args(["compose", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !compose_ok {
        anyhow::bail!(
            "Docker Compose plugin not found. Install it (e.g. 'docker compose version') and retry."
        );
    }
    Ok(())
}
