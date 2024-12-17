use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PathCloner {
    original: PathBuf,
    index_dir: PathBuf,
}

impl PathCloner {
    /// Create a new PathCloner from the given directory path.
    pub fn new<P: AsRef<Path>>(original: P, index_dir: PathBuf) -> Self {
        PathCloner {
            original: original.as_ref().to_path_buf(),
            index_dir,
        }
    }

    /// Clone the original directory into `num_clones` new directories.
    ///
    /// For an original directory like `/Users/someuser/my_project`, this will produce something like:
    /// `/Users/someuser/my_project_clones/clone_1`, `/Users/someuser/my_project_clones/clone_2`, etc.
    ///
    /// Returns the paths of the newly created clone directories.
    pub fn clone_paths(&self, num_clones: usize) -> Result<Vec<PathBuf>> {
        println!(
            "Starting to clone {} paths from {:?}",
            num_clones, self.original
        );

        // Extract the directory name
        let original_name = self
            .original
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("No directory name found"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in directory name"))?;
        println!("Original directory name: {}", original_name);

        // Perform the actual cloning
        let mut cloned_paths = Vec::with_capacity(num_clones);
        for i in 1..=num_clones {
            let clone_path = self.index_dir.join(format!("clone_{}", i));
            println!("Creating clone {} at {:?}", i, clone_path);
            fs::create_dir_all(&clone_path)?;

            // Copy all contents from the original directory to the clone
            println!(
                "Copying contents from {:?} to {:?}",
                self.original, clone_path
            );
            copy_dir_all(&self.original, &clone_path)?;
            cloned_paths.push(clone_path);
        }

        println!("Successfully created {} clones", num_clones);
        Ok(cloned_paths)
    }
}

/// Recursively copy a directory and all its contents.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
    for entry in
        fs::read_dir(src).map_err(|e| anyhow::anyhow!("Failed to read directory: {}", e))?
    {
        let entry = entry.map_err(|e| anyhow::anyhow!("Failed to read entry: {}", e))?;
        let file_type = entry
            .file_type()
            .map_err(|e| anyhow::anyhow!("Failed to get file type: {}", e))?;
        let target = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
