use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cleanserve")]
#[command(version = "0.3.0")]
#[command(about = "Zero-Burden PHP Runtime & Development Server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new CleanServe project
    Init {
        /// Project name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// PHP version to use
        #[arg(short, long, default_value = "8.4")]
        php: String,
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
    /// Download and install PHP version
    Update {
        /// PHP version to download (e.g., 8.4, 8.3)
        #[arg(short, long)]
        version: Option<String>,
    },
    /// Run Composer with project's PHP
    Composer {
        /// Composer arguments (e.g., install, require package/name)
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
}

mod commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with output suppression for clean startup
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("error".parse().unwrap_or_default())
                .add_directive("cleanserve_cli=info".parse().unwrap_or_default())
        )
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { name, php, quickstart } => {
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
        Commands::Update { version } => {
            commands::update::run(version).await?;
        }
        Commands::Composer { args } => {
            commands::composer::run(args).await?;
        }
    }
    
    Ok(())
}
