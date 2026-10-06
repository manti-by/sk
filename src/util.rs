//! Small filesystem helpers.

use std::io::Write;
use std::path::Path;

use crate::error::{Error, Result};

/// Writes a file via a temporary sibling, so readers never see a partial file
/// and a failed write leaves the previous content in place.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|source| {
        Error::with_source(format!("could not create {}", parent.display()), source)
    })?;

    let mut file_name = path
        .file_name()
        .ok_or_else(|| Error::new(format!("{} has no file name", path.display())))?
        .to_os_string();
    file_name.push(".tmp");
    let temp = parent.join(file_name);

    let mut file = std::fs::File::create(&temp).map_err(|source| {
        Error::with_source(format!("could not create {}", temp.display()), source)
    })?;
    let write_result = file.write_all(contents).and_then(|()| file.sync_all());
    if let Err(source) = write_result {
        std::fs::remove_file(&temp).ok();
        return Err(Error::with_source(
            format!("could not write {}", temp.display()),
            source,
        ));
    }
    drop(file);

    std::fs::rename(&temp, path).map_err(|source| {
        std::fs::remove_file(&temp).ok();
        Error::with_source(format!("could not update {}", path.display()), source)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_replaces_atomically() {
        let dir = std::env::temp_dir().join(format!("sk-util-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("nested").join("file.txt");

        write_atomic(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");

        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");

        assert!(!dir.join("nested").join("file.txt.tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
