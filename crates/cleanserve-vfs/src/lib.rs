//! CleanServe Virtual File System (VFS)
//!
//! In-memory file system for PHP projects with compression support.

pub mod memory;
pub mod symlink;
pub mod traits;
pub mod zip;

pub use memory::MemoryBackend;
pub use symlink::SymlinkCache;
pub use traits::{FileSystem, VfsError, VfsMetadata};
pub use zip::ZipBackend;
