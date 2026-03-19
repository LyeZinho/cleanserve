# Virtual File System (VFS)

The Virtual File System, implemented in the `crates/cleanserve-vfs/` directory, provides a consistent, high-level interface for file operations. It allows CleanServe to serve content from various sources, including physical disks, memory, and ZIP archives.

## Core Components

The VFS architecture is built around several key modules and traits:

*   **`FileSystem` Trait**: The core abstraction that defines methods for file operations (reading, writing, metadata retrieval). All backends must implement this trait.
*   **`VfsMetadata`**: A platform-independent representation of file metadata, including permissions, file type, and timestamps.
*   **`VfsError`**: A specialized error type for VFS operations, ensuring consistent error handling across all backends.

## Pluggable Backends

CleanServe's VFS supports multiple backends to cater to different use cases:

### Memory Backend

The `MemoryBackend` provides an in-memory file system. This is particularly useful for:

*   **Testing**: Rapidly simulating file system structures without needing disk I/O.
*   **Temporary Assets**: Serving files that are generated dynamically during the server's execution.

### Zip Backend

The `ZipBackend` allows the VFS to treat a ZIP archive as a file system. This is a crucial component for:

*   **Deployment**: Serving standalone, bundled applications where all assets are packaged into a single file.
*   **Efficiency**: Reducing disk I/O and overhead by serving directly from compressed archives.

### Symlink Cache

The `SymlinkCache` is a specialized layer that improves performance when dealing with file systems containing many symlinks (common in modern web development projects). It caches the results of symlink resolution to minimize expensive disk hits during path lookups.

## Architecture

```text
[ VFS Layer ]
     |
     |-- FileSystem Trait Implementation --|
     |                                     |
[ MemoryBackend ]                    [ ZipBackend ]
     |                                     |
[ Data in Memory ]                   [ Data in ZIP Archive ]
```

## Further Reading

*   [Architecture Overview](overview.md)
*   [Bundling and Deployment](bundle.md)
*   [Proxy Server Details](proxy-server.md)
