use std::path::Path;
use std::path::PathBuf;
use std::sync::LazyLock;

use regex::Regex;

use crate::error::Error;
use crate::error::Result;

static WAL_EXTENDION: &str = ".wal";
static WAL_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([0-9a-f]{8})\.wal$").unwrap());
static SNAPSHOT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9a-f]{8})\.snapshot\.lz4$").unwrap());
static SNAPSHOT_EXTENDION: &str = ".snapshot.lz4";

// return base name of path
fn path_base(path: &str) -> Result<String> {
    let path_buf = PathBuf::from(path);
    path_buf
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .ok_or(Error::InvalidPath(format!("invalid path {}", path)))
}

// parse wal file path, return wal index
pub fn parse_wal_path(path: &str) -> Result<u64> {
    let base = path_base(path)?;
    let a = WAL_REGEX
        .captures(&base)
        .ok_or(Error::InvalidPath(format!("invalid wal path {}", path)))?;
    let a = a
        .get(1)
        .ok_or(Error::InvalidPath(format!("invalid wal path {}", path)))?
        .as_str();

    Ok(u64::from_str_radix(a, 16)?)
}

pub fn format_wal_path(index: u64) -> String {
    format!("{:08X}{}", index, WAL_EXTENDION)
}

// parse snapshot file path, return snapshot index
pub fn parse_snapshot_path(path: &str) -> Result<u64> {
    let base = path_base(path)?;
    let a = SNAPSHOT_REGEX
        .captures(&base)
        .ok_or(Error::InvalidPath(format!(
            "invalid snapshot path {}",
            path
        )))?;
    let a = a
        .get(1)
        .ok_or(Error::InvalidPath(format!(
            "invalid snapshot path {}",
            path
        )))?
        .as_str();

    Ok(u64::from_str_radix(a, 16)?)
}

pub fn format_snapshot_path(index: u64) -> String {
    format!("{:08X}{}", index, SNAPSHOT_EXTENDION)
}

// returns the path of a single generation.
pub fn generations_dir(meta_dir: &str, generation: &str) -> String {
    Path::new(meta_dir)
        .join("generations")
        .join(generation)
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

// returns the path of a single generation.
pub fn snapshots_dir(meta_dir: &str, generation: &str) -> String {
    Path::new(&generations_dir(meta_dir, generation))
        .join("snapshots")
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

pub fn snapshot_file(db: &str, generation: &str, index: u64) -> String {
    Path::new(db)
        .join(generation)
        .join("snapshots")
        .join(format_snapshot_path(index))
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

// returns the path of the name of the current generation.
pub fn generation_file_path(meta_dir: &str) -> String {
    Path::new(meta_dir)
        .join("generation")
        .as_path()
        .to_str()
        .unwrap()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::path_base;
    use crate::base::file::parse_wal_path;
    use crate::base::format_wal_path;
    use crate::error::Error;
    use crate::error::Result;

    #[test]
    fn test_path_base() -> Result<()> {
        let path = "a/b/c";
        let base = path_base(path)?;
        assert_eq!(&base, "c");

        let path = "a-b/..";
        let base = path_base(path);
        assert!(base.is_err());
        let err = base.unwrap_err();
        assert_eq!(err.code(), 54);

        Ok(())
    }

    #[test]
    fn test_parse_wal_path() -> Result<()> {
        let path = "a/b/c/00000019.wal";
        let index = parse_wal_path(path)?;
        assert_eq!(index, 25);

        let path = "a/b/c/0000019.wal";
        let index = parse_wal_path(path);
        assert!(index.is_err());

        let path = format!("a/b/{}", format_wal_path(19));
        let index = parse_wal_path(&path)?;
        assert_eq!(index, 19);
        Ok(())
    }
}
