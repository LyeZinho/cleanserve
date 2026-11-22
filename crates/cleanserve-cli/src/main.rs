use clap::{Parser, Subcommand};

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CLEANSERVE_GIT_HASH"),
    ")"
);

#[derive(Parser)]
#[command(name = "cleanserve")]
#[command(version = VERSION)]
#[command(about = "Zero-Burden PHP Runtime & Development Server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new CleanServe project
    Init {
        /// Project name (defaults to composer.json name or directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// PHP version to use (defaults to composer.json require.php or 8.4)
        #[arg(short, long)]
        php: Option<String>,
        /// Generate quickstart pages
        #[arg(long)]
        quickstart: bool,
    },
    /// Start the development server
    Up {
        /// Port to bind to
        #[arg(short, long)]
        port: Option<u16>,
        /// Plain logs instead of the Ratatui dashboard
        #[arg(long)]
        no_tui: bool,
        /// HTTP only (no TLS). Browsers must use http://localhost:PORT
        #[arg(long)]
        http_only: bool,
    },
    /// Stop the development server
    Down,
    /// Switch PHP version
    Use {
        /// PHP version (e.g., 8.2, 8.4)
        version: String,
    },
    /// List available and installed PHP versions
    List {
        /// Force refresh of the version manifest
        #[arg(long)]
        refresh: bool,
        /// Show only installed versions
        #[arg(long)]
        installed: bool,
    },
    /// Download and install a PHP version, or update CleanServe itself with --self
    Update {
        /// PHP version to download (e.g., 8.4, 8.3). Defaults to the project version.
        #[arg(short, long)]
        version: Option<String>,
        /// Update CleanServe itself (download, verify SHA-256, install, roll back on failure)
        #[arg(long = "self")]
        self_update: bool,
        /// Only check whether a newer CleanServe release exists
        #[arg(long)]
        check: bool,
        /// Reinstall CleanServe even if already up to date (with --self)
        #[arg(long)]
        force: bool,
    },
    /// Run Composer with the project's PHP
    #[command(disable_version_flag = true)]
    Composer {
        /// PHP version when no cleanserve.json exists
        #[arg(long)]
        php: Option<String>,
        /// Composer arguments (e.g., install, require package/name)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run PHP with the project's interpreter
    #[command(disable_version_flag = true)]
    Php {
        /// PHP version when no cleanserve.json exists
        #[arg(long)]
        php: Option<String>,
        /// Arguments passed to the PHP CLI
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove cleanserve.json and .cleanserve/ from this project
    Uninstall,
    /// Check your setup (PHP, Composer, optional Docker)
    Doctor,
    /// Install system dependencies for CleanServe
    Setup {
        #[command(subcommand)]
        action: SetupAction,
    },
    /// Optional Docker stack services (MySQL, Redis, …) — install only what you need
    Stack {
        #[command(subcommand)]
        action: StackAction,
    },
    /// Manage project packages (alias for stack; legacy)
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },
}

#[derive(Subcommand)]
pub enum SetupAction {
    /// Install Docker Engine (distro-aware helper script)
    Docker,
}

#[derive(Subcommand)]
pub enum StackAction {
    /// List available stack services
    List,
    /// Enable a service in this project (generates compose under .cleanserve/stack/)
    Add {
        name: String,
    },
    /// Disable a service
    Remove {
        name: String,
    },
    /// Start enabled stack services
    Up,
    /// Stop stack services (keeps volumes)
    Down,
    /// Show enabled services and container status
    Status,
}

#[derive(Subcommand)]
pub enum PackageAction {
    /// Add a package to the project
    Add {
        /// Package name (e.g., mysql, redis)
        name: String,
        /// Package version (optional, uses default if not specified)
        version: Option<String>,
    },
    /// List available packages
    List,
    /// Show package information
    Info {
        /// Package name
        name: String,
    },
    /// Start a package
    Start {
        /// Package name
        name: String,
    },
    /// Stop a package
    Stop {
        /// Package name
        name: String,
    },
    /// Show package status
    Status {
        /// Package name (optional, show all if not specified)
        name: Option<String>,
    },
}

mod commands;
mod tui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let tui_quiet_tracing = matches!(cli.command, Commands::Up { no_tui: false, .. });
    init_tracing(tui_quiet_tracing);

    match cli.command {
        Commands::Init {
            name,
            php,
            quickstart,
        } => {
            commands::init::run(name, php, quickstart).await?;
        }
        Commands::Up { port, no_tui, http_only } => {
            commands::up::run(port, no_tui, http_only).await?;
        }
        Commands::Down => {
            commands::down::run().await?;
        }
        Commands::Use { version } => {
            commands::use_::run(version).await?;
        }
        Commands::List { refresh, installed } => {
            commands::list::run(refresh, installed).await?;
        }
        Commands::Update {
            version,
            self_update,
            check,
            force,
        } => {
            if check || self_update {
                commands::update::run_cleanserve_update(check, force).await?;
            } else {
                commands::update::run_php_update(version).await?;
            }
        }
        Commands::Composer { php, args } => {
            commands::composer::run(args, php).await?;
        }
        Commands::Php { php, args } => {
            commands::php::run(args, php).await?;
        }
        Commands::Uninstall => {
            commands::uninstall::run().await?;
        }
        Commands::Doctor => {
            commands::setup::run()?;
        }
        Commands::Setup { action } => match action {
            SetupAction::Docker => commands::setup::run_setup_docker()?,
        },
        Commands::Stack { action } => match action {
            StackAction::List => commands::stack::run_list().await?,
            StackAction::Add { name } => commands::stack::run_add(&name).await?,
            StackAction::Remove { name } => commands::stack::run_remove(&name).await?,
            StackAction::Up => commands::stack::run_up().await?,
            StackAction::Down => commands::stack::run_down().await?,
            StackAction::Status => commands::stack::run_status().await?,
        },
        Commands::Package { action } => {
            let project_root = std::env::current_dir()?;
            match action {
                PackageAction::Add { name, version } => {
                    commands::package::PackageCommand::add(&name, version.as_deref(), &project_root)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::List => {
                    commands::package::PackageCommand::list().map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Info { name } => {
                    commands::package::PackageCommand::info(&name)
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Start { name } => {
                    commands::package::PackageCommand::start(&name)
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Stop { name } => {
                    commands::package::PackageCommand::stop(&name)
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PackageAction::Status { name } => {
                    commands::package::PackageCommand::status(name.as_deref())
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
            }
        }
    }

    Ok(())
}

fn init_tracing(sink_stderr: bool) {
    use std::io;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::from_default_env()
        .add_directive("error".parse().unwrap_or_default())
        .add_directive("cleanserve_cli=info".parse().unwrap_or_default());

    let builder = tracing_subscriber::fmt()
        .with_target(false)
        .with_level(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_env_filter(filter);

    if sink_stderr {
        builder.with_writer(io::sink).init();
    } else {
        builder.init();
    }
}
