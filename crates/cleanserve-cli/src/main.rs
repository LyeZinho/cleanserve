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
    /// Download and install a PHP version
    Update {
        /// PHP version to download (e.g., 8.4, 8.3). Defaults to the project version.
        #[arg(short, long)]
        version: Option<String>,
        /// Check for CleanServe updates instead of PHP
        #[arg(long)]
        check: bool,
        /// Force update even if versions match
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
    /// Manage project packages (MySQL, Redis, phpMyAdmin, etc)
    Package {
        #[command(subcommand)]
        action: PackageAction,
    },
    /// Configure local development domains (`*.serve`)
    #[command(alias = "setup-domain")]
    Setupdomain {
        /// Remove the resolver/hosts configuration created by this command
        #[arg(long)]
        remove: bool,
    },
    /// Manage Docker-backed development services
    Stack {
        #[command(subcommand)]
        action: StackAction,
    },
}

#[derive(Subcommand)]
pub enum StackAction {
    /// Add a service to cleanserve.json and its generated compose file
    Add {
        /// Service name (mysql, mariadb, postgres, redis, mailpit, phpmyadmin)
        name: String,
        /// Service version/tag (optional)
        version: Option<String>,
        /// Host port to expose (optional)
        #[arg(long)]
        port: Option<u16>,
    },
    /// Remove a service
    Remove {
        name: String,
    },
    /// List services available and configured in this project
    List,
    /// Start configured services
    Up {
        /// Start only these services
        services: Vec<String>,
    },
    /// Stop configured services
    Down,
    /// Show container status
    Status,
    /// Follow service logs
    Logs {
        /// Service name (optional, all services when omitted)
        name: Option<String>,
    },
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("error".parse().unwrap_or_default())
                .add_directive("cleanserve_cli=info".parse().unwrap_or_default()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            php,
            quickstart,
        } => {
            commands::init::run(name, php, quickstart).await?;
        }
        Commands::Up { port } => {
            commands::up::run(port).await?;
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
            check,
            force,
        } => {
            if check {
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
        Commands::Setupdomain { remove } => {
            commands::setupdomain::run(remove).await?;
        }
        Commands::Stack { action } => {
            let project_root = std::env::current_dir()?;
            match action {
                StackAction::Add {
                    name,
                    version,
                    port,
                } => commands::stack::add(&name, version.as_deref(), port, &project_root).await?,
                StackAction::Remove { name } => {
                    commands::stack::remove(&name, &project_root).await?
                }
                StackAction::List => commands::stack::list(&project_root)?,
                StackAction::Up { services } => {
                    commands::stack::up(&services, &project_root).await?
                }
                StackAction::Down => commands::stack::down(&project_root).await?,
                StackAction::Status => commands::stack::status(&project_root).await?,
                StackAction::Logs { name } => {
                    commands::stack::logs(name.as_deref(), &project_root).await?
                }
            }
        }
    }

    Ok(())
}
