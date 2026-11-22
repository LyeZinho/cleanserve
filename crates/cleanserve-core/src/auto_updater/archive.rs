use super::{Result, UpdateCheckerError};
use std::io::Read;
use std::path::Path;

fn err(message: impl Into<String>) -> UpdateCheckerError {
    UpdateCheckerError {
        message: message.into(),
    }
}

/// Name of the executable inside release archives.
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "cleanserve.exe"
    } else {
        "cleanserve"
    }
}

/// Extract the `cleanserve` executable from a release archive (`.tar.gz` or `.zip`)
/// into `dest`. Only that one entry is read, so nothing else in the archive can touch the disk.
pub fn extract_binary(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let bytes = if name.ends_with(".zip") {
        extract_zip(archive)?
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(archive)?
    } else {
        return Err(err(format!("Unsupported archive format: {}", name)));
    };

    std::fs::write(dest, bytes).map_err(|e| err(format!("Failed to write extracted binary: {}", e)))
}

fn extract_tar_gz(archive: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(archive).map_err(|e| err(format!("Failed to open archive: {}", e)))?;
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let entries = tar
        .entries()
        .map_err(|e| err(format!("Failed to read archive: {}", e)))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| err(format!("Corrupt archive entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|e| err(format!("Invalid entry path: {}", e)))?;
        if path.file_name().is_some_and(|n| n == binary_name()) && entry.header().entry_type().is_file() {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| err(format!("Failed to read binary from archive: {}", e)))?;
            return Ok(buf);
        }
    }
    Err(err(format!("`{}` not found in archive", binary_name())))
}

fn extract_zip(archive: &Path) -> Result<Vec<u8>> {
    let file = std::fs::File::open(archive).map_err(|e| err(format!("Failed to open archive: {}", e)))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| err(format!("Failed to read zip: {}", e)))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| err(format!("Corrupt zip entry: {}", e)))?;
        let is_binary = Path::new(entry.name())
            .file_name()
            .is_some_and(|n| n == binary_name());
        if is_binary && entry.is_file() {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| err(format!("Failed to read binary from zip: {}", e)))?;
            return Ok(buf);
        }
    }
    Err(err(format!("`{}` not found in archive", binary_name())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn extracts_binary_from_tar_gz() {
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("cleanserve-linux-x86_64.tar.gz");

        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::default(),
        );
        let mut builder = tar::Builder::new(gz);
        for (path, data) in [("README", &b"readme"[..]), (binary_name(), &b"BINARY"[..])] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, path, data).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();

        let out = dir.path().join("out");
        extract_binary(&archive, &out).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"BINARY");
    }

    #[test]
    fn extracts_binary_from_zip() {
        use std::io::Write;
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("cleanserve-windows-x86_64.zip");

        let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
        zip.start_file(binary_name(), zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"ZIPBIN").unwrap();
        zip.finish().unwrap();

        let out = dir.path().join("out");
        extract_binary(&archive, &out).unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"ZIPBIN");
    }

    #[test]
    fn missing_binary_and_unknown_format_are_errors() {
        let dir = TempDir::new().unwrap();
        let archive = dir.path().join("x.tar.gz");
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            flate2::Compression::default(),
        );
        tar::Builder::new(gz).into_inner().unwrap().finish().unwrap();
        assert!(extract_binary(&archive, &dir.path().join("out")).is_err());
        assert!(extract_binary(&dir.path().join("x.rar"), &dir.path().join("out")).is_err());
    }
}
