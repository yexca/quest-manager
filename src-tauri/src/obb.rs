//! Local OBB selection and streaming integrity checks. Device work lives in adb/tasks.
use crate::{adb::validate_filename, apk::source_stamp};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs::File, io::Read, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObbSource {
    pub source: String,
    pub source_stamp: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalObb {
    #[serde(flatten)]
    pub input: ObbSource,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObbInstall {
    pub apk_source_stamp: String,
    pub files: Vec<ObbSource>,
}

pub struct CheckedObb {
    pub file: LocalObb,
    pub sha256: String,
}

pub fn package_directory(package: &str) -> Result<String, String> {
    if !package.contains('.')
        || !package.split('.').all(|part| {
            !part.is_empty() && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
        })
    {
        return Err("OBB installation requires a readable, valid APK package name.".into());
    }
    Ok(format!("/sdcard/Android/obb/{package}"))
}

pub fn inspect(sources: Vec<String>) -> Result<Vec<LocalObb>, String> {
    if sources.is_empty() || sources.len() > 128 {
        return Err("Choose between 1 and 128 OBB files per APK.".into());
    }
    let mut names = HashSet::new();
    sources
        .into_iter()
        .map(|source| {
            let path = Path::new(&source);
            if !path.is_absolute()
                || !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("obb"))
            {
                return Err("Choose local .obb files using absolute paths.".into());
            }
            let metadata = std::fs::symlink_metadata(path)
                .map_err(|e| format!("Could not read the OBB file: {e}"))?;
            if !metadata.is_file() {
                return Err("Choose regular OBB files, not folders or symbolic links.".into());
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("Unsupported OBB filename.")?
                .to_string();
            validate_filename(&name)?;
            if !names.insert(name.to_lowercase()) {
                return Err(format!(
                    "More than one OBB file is named {name}. Choose only one."
                ));
            }
            let stamp = source_stamp(path)?;
            let source = std::fs::canonicalize(path)
                .map_err(|e| format!("Could not access the OBB file: {e}"))?
                .to_str()
                .ok_or("Unsupported local OBB path.")?
                .to_string();
            Ok(LocalObb {
                input: ObbSource {
                    source,
                    source_stamp: stamp,
                },
                name,
                size: metadata.len(),
            })
        })
        .collect()
}

pub fn check(selection: &ObbInstall) -> Result<Vec<CheckedObb>, String> {
    let files = inspect(
        selection
            .files
            .iter()
            .map(|file| file.source.clone())
            .collect(),
    )?;
    files
        .into_iter()
        .zip(&selection.files)
        .map(|(file, expected)| {
            if file.input.source_stamp != expected.source_stamp {
                return Err(format!(
                    "{} changed after selection. Select it again.",
                    file.name
                ));
            }
            let sha256 = hash_file(&file.input)?;
            Ok(CheckedObb { file, sha256 })
        })
        .collect()
}

pub fn hash_file(input: &ObbSource) -> Result<String, String> {
    let path = Path::new(&input.source);
    if source_stamp(path)? != input.source_stamp {
        return Err("An OBB file changed after selection. Select it again.".into());
    }
    let mut file = File::open(path).map_err(|e| format!("Could not open the OBB file: {e}"))?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Could not read the OBB file: {e}"))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if source_stamp(path)? != input.source_stamp {
        return Err("An OBB file changed while being read. Select it again.".into());
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn parse_sha256(output: &str) -> Result<String, String> {
    let hash = output.split_whitespace().next().unwrap_or("");
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Could not verify the OBB checksum on the headset.".into());
    }
    Ok(hash.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_paths_and_checksums_reject_unreadable_or_unsafe_values() {
        for package in [
            "",
            "..",
            ".com.example",
            "com..example",
            "com.example/../other",
            "com.example;id",
        ] {
            assert!(package_directory(package).is_err(), "{package}");
        }
        assert_eq!(
            package_directory("com.example.Game").unwrap(),
            "/sdcard/Android/obb/com.example.Game"
        );
        assert_eq!(
            parse_sha256(&format!("{}  -", "AB".repeat(32))).unwrap(),
            "ab".repeat(32)
        );
        for output in ["", "permission denied", "abc", "Success"] {
            assert!(parse_sha256(output).is_err());
        }
        assert!(inspect(vec![]).is_err());
        assert!(inspect(vec!["relative.obb".into()]).is_err());
    }
}
