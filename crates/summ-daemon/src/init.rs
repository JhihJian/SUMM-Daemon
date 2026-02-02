// summ-daemon/src/init.rs
// Initialization functions for session workdir setup
use anyhow::{Context, Result};
use compress_tools::Ownership;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Initialize a workdir from a source (directory, zip, or tar.gz)
pub fn initialize_workdir(workdir: &Path, init_path: &Path) -> Result<()> {
    if !init_path.exists() {
        anyhow::bail!(
            "Initialization source not found: {}",
            init_path.display()
        );
    }

    if init_path.is_dir() {
        copy_dir_contents(init_path, workdir)?;
    } else if init_path.extension().map_or(false, |e| e == "zip") {
        extract_zip(init_path, workdir)?;
    } else if init_path.to_string_lossy().ends_with(".tar.gz")
        || init_path.to_string_lossy().ends_with(".tgz")
    {
        extract_tar_gz(init_path, workdir)?;
    } else {
        anyhow::bail!(
            "Unsupported initialization source: {}. Expected directory, .zip, or .tar.gz",
            init_path.display()
        );
    }
    Ok(())
}

/// Copy directory contents recursively from source to destination
pub fn copy_dir_contents(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        anyhow::bail!("Source directory does not exist: {}", source.display());
    }

    // Create destination directory if it doesn't exist
    fs::create_dir_all(destination)
        .context(format!("Failed to create directory: {}", destination.display()))?;

    // Iterate through entries in source directory
    for entry in fs::read_dir(source)
        .context(format!("Failed to read directory: {}", source.display()))?
    {
        let entry = entry.context("Failed to read directory entry")?;
        let file_type = entry.file_type().context("Failed to get file type")?;
        let src_path = entry.path();
        let dest_path = destination.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_contents(&src_path, &dest_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dest_path).context(format!(
                "Failed to copy file from {} to {}",
                src_path.display(),
                dest_path.display()
            ))?;
        }
        // Skip symlinks and other special files
    }

    Ok(())
}

/// Extract a ZIP archive to the destination directory
pub fn extract_zip(archive_path: &Path, destination: &Path) -> Result<()> {
    if !archive_path.exists() {
        anyhow::bail!("Archive not found: {}", archive_path.display());
    }

    fs::create_dir_all(destination)
        .context(format!("Failed to create directory: {}", destination.display()))?;

    let file = File::open(archive_path).context(format!(
        "Failed to open archive: {}",
        archive_path.display()
    ))?;
    let mut reader = BufReader::new(file);

    compress_tools::uncompress_archive(&mut reader, destination, Ownership::Ignore).context(format!(
        "Failed to extract ZIP archive: {}",
        archive_path.display()
    ))?;

    Ok(())
}

/// Extract a tar.gz archive to the destination directory
pub fn extract_tar_gz(archive_path: &Path, destination: &Path) -> Result<()> {
    if !archive_path.exists() {
        anyhow::bail!("Archive not found: {}", archive_path.display());
    }

    fs::create_dir_all(destination)
        .context(format!("Failed to create directory: {}", destination.display()))?;

    let file = File::open(archive_path).context(format!(
        "Failed to open archive: {}",
        archive_path.display()
    ))?;
    let mut reader = BufReader::new(file);

    compress_tools::uncompress_archive(&mut reader, destination, Ownership::Ignore)
        .context(format!(
            "Failed to extract tar.gz archive: {}",
            archive_path.display()
        ))?;

    Ok(())
}

/// Create the session structure with workspace and runtime directories
pub fn create_session_structure(session_dir: &Path) -> Result<()> {
    let workspace_dir = session_dir.join("workspace");
    let runtime_dir = session_dir.join("runtime");

    fs::create_dir_all(&workspace_dir).context(format!(
        "Failed to create workspace directory: {}",
        workspace_dir.display()
    ))?;

    fs::create_dir_all(&runtime_dir).context(format!(
        "Failed to create runtime directory: {}",
        runtime_dir.display()
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_copy_dir_contents() {
        let source_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();

        // Create test structure in source
        let file1 = source_dir.path().join("file1.txt");
        let subdir = source_dir.path().join("subdir");
        let file2 = subdir.join("file2.txt");

        fs::create_dir(&subdir).unwrap();
        File::create(&file1).unwrap().write_all(b"content1").unwrap();
        File::create(&file2).unwrap().write_all(b"content2").unwrap();

        // Copy
        let result = copy_dir_contents(source_dir.path(), dest_dir.path());
        assert!(result.is_ok());

        // Verify
        assert!(dest_dir.path().join("file1.txt").exists());
        assert!(dest_dir.path().join("subdir").exists());
        assert!(dest_dir.path().join("subdir/file2.txt").exists());

        let content = fs::read_to_string(dest_dir.path().join("file1.txt")).unwrap();
        assert_eq!(content, "content1");
    }

    #[test]
    fn test_copy_dir_contents_empty_source() {
        let source_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();

        // Empty directory copy should succeed
        let result = copy_dir_contents(source_dir.path(), dest_dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_session_structure() {
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join("session_001");

        let result = create_session_structure(&session_dir);
        assert!(result.is_ok());

        assert!(session_dir.exists());
        assert!(session_dir.join("workspace").exists());
        assert!(session_dir.join("runtime").exists());
    }

    #[test]
    fn test_initialize_workdir_from_directory() {
        let source_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();

        // Create source content
        let test_file = source_dir.path().join("test.txt");
        File::create(&test_file).unwrap().write_all(b"hello").unwrap();

        // Initialize from directory
        let result = initialize_workdir(dest_dir.path(), source_dir.path());
        assert!(result.is_ok());

        // Verify content copied
        assert!(dest_dir.path().join("test.txt").exists());
        let content = fs::read_to_string(dest_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_initialize_workdir_nonexistent_source() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");
        let dest_dir = TempDir::new().unwrap();

        let result = initialize_workdir(dest_dir.path(), &nonexistent);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_extract_zip_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let fake_zip = temp_dir.path().join("nonexistent.zip");
        let dest_dir = TempDir::new().unwrap();

        let result = extract_zip(&fake_zip, dest_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_extract_tar_gz_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let fake_tar = temp_dir.path().join("nonexistent.tar.gz");
        let dest_dir = TempDir::new().unwrap();

        let result = extract_tar_gz(&fake_tar, dest_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_initialize_workdir_unsupported_extension() {
        let temp_dir = TempDir::new().unwrap();
        let fake_archive = temp_dir.path().join("file.rar");
        let dest_dir = TempDir::new().unwrap();

        File::create(&fake_archive).unwrap().write_all(b"content").unwrap();

        let result = initialize_workdir(dest_dir.path(), &fake_archive);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }
}
