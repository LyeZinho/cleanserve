//! CleanServe Preload Generator
//!
//! Automatically generates PHP OpCache preload scripts based on composer.json PSR-4 autoload.
//!
//! # Usage
//!
//! Generate preload script from a project directory:
//! ```no_run
//! use cleanserve_preload::PreloadGenerator;
//!
//! let generator = PreloadGenerator::from_project("/path/to/project").unwrap();
//! let script = generator.generate_script().unwrap();
//! println!("{}", script);
//! ```

pub mod composer;
pub mod generator;

pub use composer::{ComposerExtra, ComposerJson};
pub use generator::PreloadGenerator;
