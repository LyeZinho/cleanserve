//! CleanServe Bundle - Self-contained PHP application bundler
//!
//! Creates standalone, portable PHP applications by bundling:
//! - PHP binary (static)
//! - Application code (PHAR)
//! - Configuration (php.ini, preload.php)
//! - Embedded environment (.env inlined)

pub mod phar;
pub mod builder;
pub mod config;

pub use builder::BundleBuilder;
pub use config::BundleConfig;
