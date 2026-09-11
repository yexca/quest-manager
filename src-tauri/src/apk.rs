//! Bounded, read-only APK metadata extraction. ZIP seeks are translated to ADB
//! byte-range reads, so game assets and native libraries never cross the cable.
use crate::{
    adb::{Adb, shell_quote},
    metadata::{ApkFile, AppAssets},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

const PAGE: u64 = 256 * 1024;
const READ_BUDGET: u64 = 64 * 1024 * 1024;
const RESOURCE_LIMIT: u64 = 32 * 1024 * 1024;
const ICON_LIMIT: u64 = 2 * 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalApk {
    pub package_name: String,
    pub version_name: String,
    pub version_code: String,
    pub size: u64,
    pub source_stamp: String,
    pub assets: AppAssets,
    pub split: bool,
    pub verity_signing: Option<bool>,
}

pub fn source_stamp(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("Select a regular APK file.".into());
    }
    let modified = meta
        .modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    Ok(format!("{}:{modified}", meta.len()))
}

pub fn inspect_local(path: &Path, aapt: &Path, cache: &Path) -> Result<LocalApk, String> {
    let stamp = source_stamp(path)?;
    let badging = tool(
        aapt,
        vec![
            "dump".into(),
            "badging".into(),
            path.to_string_lossy().into_owned(),
        ],
    )?;
    let package = badging
        .lines()
        .find(|l| l.starts_with("package:"))
        .ok_or("This file does not contain a readable Android package.")?;
    let package_name = attribute(package, "name").ok_or("APK package name is unavailable.")?;
    crate::adb::validate_package(&package_name)?;
    // Badging omits versionCodeMajor. Read the manifest tree for a comparable
    // long version; a failed read stays unknown instead of using only low bits.
    let version_code = tool(
        aapt,
        vec![
            "dump".into(),
            "xmltree".into(),
            "--file".into(),
            "AndroidManifest.xml".into(),
            path.to_string_lossy().into_owned(),
        ],
    )
    .ok()
    .and_then(|tree| long_version_code(&tree))
    .unwrap_or_default();
    let mut assets = extract(
        fs::File::open(path).map_err(|e| e.to_string())?,
        aapt,
        cache,
    )
    .unwrap_or_else(|error| AppAssets {
        notes: vec![format!("APK artwork could not be previewed: {error}")],
        ..Default::default()
    });
    if let Some(launch) = badging
        .lines()
        .find(|l| l.starts_with("launchable-activity:"))
    {
        if let Some(label) = attribute(launch, "label").filter(|s| !s.is_empty()) {
            assets.display_name = Some(label);
        }
        if let Some(icon) = attribute(launch, "icon").filter(|s| !s.is_empty()) {
            let mut archive = ZipArchive::new(fs::File::open(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            assets.icon_data_url = entry(&mut archive, &icon, ICON_LIMIT)
                .ok()
                .and_then(|data| raster_data_url(&data));
        }
    } else {
        assets
            .notes
            .push("No standard launcher activity was found.".into());
    }
    assets.notes.push("Preview uses APK default launcher resources. Quest language, launcher artwork and adaptive icons may differ.".into());
    if source_stamp(path)? != stamp {
        return Err("The APK changed while it was being read. Select it again.".into());
    }
    let verity_signing = (|| {
        let archive = ZipArchive::new(fs::File::open(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let offset = archive.central_directory_start();
        let mut file = archive.into_inner();
        signing(&mut file, offset).map(|info| info.2)
    })()
    .ok();
    Ok(LocalApk {
        package_name,
        version_name: attribute(package, "versionName").unwrap_or_default(),
        version_code,
        size: fs::metadata(path).map_err(|e| e.to_string())?.len(),
        source_stamp: stamp,
        verity_signing,
        assets,
        split: attribute(package, "split").is_some()
            || badging
                .lines()
                .any(|l| l.starts_with("uses-split:") || l.contains("isSplitRequired='true'")),
    })
}

struct RemoteApk {
    adb: Adb,
    device: String,
    path: String,
    size: u64,
    position: u64,
    page: Vec<u8>,
    page_start: u64,
    transferred: u64,
    started: Instant,
}

impl Read for RemoteApk {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.position >= self.size {
            return Ok(0);
        }
        if self.started.elapsed() > Duration::from_secs(90) {
            return Err(io::Error::other("APK metadata reading timed out."));
        }
        if self.page.is_empty()
            || self.position < self.page_start
            || self.position >= self.page_start + self.page.len() as u64
        {
            self.page_start = self.position / PAGE * PAGE;
            let count = PAGE.min(self.size - self.page_start);
            self.transferred += count;
            if self.transferred > READ_BUDGET {
                return Err(io::Error::other("APK metadata exceeds the read limit."));
            }
            let command = format!(
                "dd if={} bs=65536 skip={} count={} iflag=skip_bytes,count_bytes status=none",
                shell_quote(&self.path),
                self.page_start,
                count
            );
            self.page = tokio::runtime::Handle::current()
                .block_on(self.adb.run(vec![
                    "-s".into(),
                    self.device.clone(),
                    "exec-out".into(),
                    command,
                ]))
                .map_err(io::Error::other)?;
            if self.page.len() != count as usize {
                return Err(io::Error::other(
                    "APK changed or could not be read completely.",
                ));
            }
        }
        let offset = (self.position - self.page_start) as usize;
        let count = output.len().min(self.page.len() - offset);
        output[..count].copy_from_slice(&self.page[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for RemoteApk {
    fn seek(&mut self, seek: SeekFrom) -> io::Result<u64> {
        let position = match seek {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::End(n) => self.size as i128 + n as i128,
            SeekFrom::Current(n) => self.position as i128 + n as i128,
        };
        if position < 0 || position > self.size as i128 {
            return Err(io::Error::other("Invalid APK offset."));
        }
        self.position = position as u64;
        Ok(self.position)
    }
}

struct TemporaryApk(PathBuf);
impl Drop for TemporaryApk {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    let file = archive
        .by_name(name)
        .map_err(|_| "APK resource is unavailable.")?;
    if file.size() > limit || file.compressed_size() > limit {
        return Err("APK resource exceeds the size limit.".into());
    }
    let mut data = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|_| "Could not read APK resource.")?;
    if data.len() as u64 > limit {
        return Err("APK resource exceeds the size limit.".into());
    }
    Ok(data)
}

fn tool(aapt: &Path, args: Vec<String>) -> Result<String, String> {
    let adb = Adb::new(aapt.to_path_buf());
    let output = tokio::runtime::Handle::current()
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(15), adb.command(&args).output()).await
        })
        .map_err(|_| "Application resource parsing timed out.")?
        .map_err(|_| "Could not start the bundled AAPT2 tool.")?;
    if !output.status.success() {
        return Err("Application resources could not be decoded.".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn read_assets(
    adb: &Adb,
    device: &str,
    file: &ApkFile,
    aapt: &Path,
    cache: &Path,
) -> Result<AppAssets, String> {
    let size = file.size.ok_or("APK size is unavailable.")?;
    let reader = RemoteApk {
        adb: adb.clone(),
        device: device.into(),
        path: file.path.clone(),
        size,
        position: 0,
        page: Vec::new(),
        page_start: 0,
        transferred: 0,
        started: Instant::now(),
    };
    extract(reader, aapt, cache)
}

fn extract<R: Read + Seek>(mut reader: R, aapt: &Path, cache: &Path) -> Result<AppAssets, String> {
    // Bound the central directory before zip allocates entries from untrusted
    // counts. APKs use classic ZIP; unusual ZIP64 archives fall back gracefully.
    let end = reader.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
    let tail_size = end.min(65557);
    reader
        .seek(SeekFrom::End(-(tail_size as i64)))
        .map_err(|e| e.to_string())?;
    let mut tail = vec![0; tail_size as usize];
    reader.read_exact(&mut tail).map_err(|e| e.to_string())?;
    let eocd = tail
        .windows(4)
        .rposition(|v| v == b"PK\x05\x06")
        .ok_or("Invalid APK directory.")?;
    if tail.len() < eocd + 22 {
        return Err("Invalid APK directory.".into());
    }
    let count = u16::from_le_bytes(tail[eocd + 10..eocd + 12].try_into().unwrap());
    let directory_size = u32::from_le_bytes(tail[eocd + 12..eocd + 16].try_into().unwrap());
    if count == u16::MAX || directory_size > 8 * 1024 * 1024 {
        return Err("APK directory exceeds the metadata limit.".into());
    }
    let mut archive = ZipArchive::new(reader).map_err(|_| "Could not open APK resources.")?;
    let manifest = entry(&mut archive, "AndroidManifest.xml", 4 * 1024 * 1024)?;
    let resources = if archive.index_for_name("resources.arsc").is_some() {
        Some(entry(&mut archive, "resources.arsc", RESOURCE_LIMIT)?)
    } else {
        None
    };
    // AAPT2 needs only the manifest and resource table to resolve label/icon
    // references. This temporary file contains no code, libraries, or game data.
    fs::create_dir_all(cache).map_err(|_| "Could not create the metadata cache.")?;
    let path = cache.join(format!(
        "resource-{}-{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|_| "Could not stage APK resources.")?;
    let temporary = TemporaryApk(path);
    let mut writer = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer
        .start_file("AndroidManifest.xml", options)
        .map_err(|e| e.to_string())?;
    writer.write_all(&manifest).map_err(|e| e.to_string())?;
    if let Some(resources) = resources {
        writer
            .start_file("resources.arsc", options)
            .map_err(|e| e.to_string())?;
        writer.write_all(&resources).map_err(|e| e.to_string())?;
    }
    drop(writer.finish().map_err(|e| e.to_string())?);
    let badging = tool(
        aapt,
        vec![
            "dump".into(),
            "badging".into(),
            temporary.0.to_string_lossy().into_owned(),
        ],
    )?;
    let xml = tool(
        aapt,
        vec![
            "dump".into(),
            "xmltree".into(),
            "--file".into(),
            "AndroidManifest.xml".into(),
            temporary.0.to_string_lossy().into_owned(),
        ],
    )
    .unwrap_or_default();
    let mut assets = parse_badging(&badging);
    assets.vr_features.extend(parse_vr_declarations(&xml));
    assets.vr_features.sort();
    assets.vr_features.dedup();
    let mut candidates: Vec<_> = badging
        .lines()
        .filter(|l| l.starts_with("application-icon-"))
        .filter_map(|l| {
            let (density, path) = l.strip_prefix("application-icon-")?.split_once(':')?;
            Some((density.parse::<u32>().unwrap_or(0), unquote(path)))
        })
        .collect();
    candidates.sort_by_key(|(density, _)| std::cmp::Reverse(*density));
    if let Some(line) = badging.lines().find(|l| l.starts_with("application:"))
        && let Some(icon) = attribute(line, "icon")
    {
        candidates.push((0, icon));
    }
    for (_, path) in candidates {
        if let Ok(data) = entry(&mut archive, &path, ICON_LIMIT)
            && let Some(url) = raster_data_url(&data)
        {
            assets.icon_data_url = Some(url);
            break;
        }
    }
    if assets.icon_data_url.is_none() {
        assets.notes.push("A standard icon is shown when raster artwork is unavailable (including adaptive, vector, or split-only icons).".into());
    }
    let directory = archive.central_directory_start();
    let mut reader = archive.into_inner();
    match signing(&mut reader, directory) {
        Ok((schemes, certificates, _)) => {
            assets.signing_schemes = schemes;
            assets.certificate_sha256 = certificates;
        }
        Err(_) => assets
            .notes
            .push("Signing block metadata is unavailable.".into()),
    }
    if assets.certificate_sha256.is_empty() {
        assets.notes.push(
            "No v2/v3 certificate fingerprint was available. V1-only signatures are not decoded."
                .into(),
        );
    }
    Ok(assets)
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    unescape_badging(
        value
            .strip_prefix('\'')
            .and_then(|v| v.strip_suffix('\''))
            .unwrap_or(value),
    )
}
fn unescape_badging(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => output.push('\\'),
            Some('\'') => output.push('\''),
            Some('"') => output.push('"'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('t') => output.push('\t'),
            Some(other) => {
                output.push('\\');
                output.push(other);
            }
            None => output.push('\\'),
        }
    }
    output
}
// Match the package manager's long version code, keeping IPC lossless as text.
fn long_version_code(tree: &str) -> Option<String> {
    let mut lines = tree.lines();
    lines.find(|line| line.trim_start().starts_with("E: manifest ("))?;
    let mut minor = None;
    let mut major = 0;
    for line in lines {
        let line = line.trim_start();
        if line.starts_with("E:") || line.starts_with("N:") {
            break;
        }
        let Some(attribute) = line.strip_prefix("A: http://schemas.android.com/apk/res/android:")
        else {
            continue;
        };
        let target = if attribute.starts_with("versionCode(") {
            &mut minor
        } else if attribute.starts_with("versionCodeMajor(") {
            let value = manifest_version_integer(attribute)?;
            major = value;
            continue;
        } else {
            continue;
        };
        *target = Some(manifest_version_integer(attribute)?);
    }
    let minor = minor?;
    Some(((u64::from(major) << 32) | u64::from(minor)).to_string())
}

fn manifest_version_integer(attribute: &str) -> Option<u32> {
    let value = attribute.split_once(")=")?.1.trim();
    match value.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

fn attribute(line: &str, key: &str) -> Option<String> {
    let value = line.split_once(&format!("{key}='"))?.1;
    // Find an unescaped quote so apostrophes in labels stay intact.
    let mut escaped = false;
    for (i, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == '\'' {
            return Some(unescape_badging(&value[..i]));
        }
    }
    None
}

fn parse_badging(raw: &str) -> AppAssets {
    let label = [
        "application-label-en-US:",
        "application-label-en:",
        "application-label:",
    ]
    .into_iter()
    .find_map(|prefix| {
        raw.lines()
            .find_map(|l| l.strip_prefix(prefix))
            .map(unquote)
    })
    .filter(|l| !l.is_empty())
    .or_else(|| {
        raw.lines()
            .find(|l| l.starts_with("application:"))
            .and_then(|line| attribute(line, "label"))
            .filter(|label| !label.is_empty())
    });
    let vr_features = raw
        .lines()
        .filter(|l| l.starts_with("uses-feature") || l.starts_with("launchable-activity"))
        .filter_map(|l| attribute(l, "name"))
        .filter(|v| is_vr(v))
        .collect();
    AppAssets {
        display_name: label,
        vr_features,
        ..Default::default()
    }
}

fn is_vr(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("oculus")
        || lower.contains("openxr")
        || lower.contains("vr.")
        || lower.contains(".vr")
        || lower.contains("headtracking")
}

fn parse_vr_declarations(xml: &str) -> Vec<String> {
    // Only manifest declaration elements, never arbitrary resource strings.
    let mut result = Vec::new();
    let mut element = "";
    let mut name: Option<String> = None;
    for line in xml.lines() {
        let line = line.trim();
        if let Some(next) = line.strip_prefix("E: ") {
            element = next.split_whitespace().next().unwrap_or("");
            name = None;
        }
        if !["meta-data", "uses-feature", "category"].contains(&element) {
            continue;
        }
        if line.starts_with("A: android:name")
            && let Some((_, rest)) = line.split_once("=\"")
        {
            let value = rest.split('"').next().unwrap_or("");
            if is_vr(value) {
                name = Some(value.into());
                result.push(value.into());
            }
        }
        if line.starts_with("A: android:value")
            && let (Some(name), Some((_, value))) = (&name, line.split_once('='))
            && let Some(last) = result.last_mut()
        {
            *last = format!("{name} = {}", value.split(" (Raw:").next().unwrap_or(value));
        }
    }
    result
}

fn raster_data_url(data: &[u8]) -> Option<String> {
    let mime = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if data.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        return None;
    };
    Some(format!("data:{mime};base64,{}", STANDARD.encode(data)))
}

fn take_length<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], String> {
    if input.len() < 4 {
        return Err("Truncated signing record.".into());
    }
    let length = u32::from_le_bytes(input[..4].try_into().unwrap()) as usize;
    *input = &input[4..];
    if length > input.len() {
        return Err("Invalid signing record length.".into());
    }
    let (value, tail) = input.split_at(length);
    *input = tail;
    Ok(value)
}

// Fingerprints identify certificates. This does not verify APK digests, signer
// trust, ownership, or a proof-of-rotation lineage.
fn signing<R: Read + Seek>(
    reader: &mut R,
    directory: u64,
) -> Result<(Vec<String>, Vec<String>, bool), String> {
    if directory < 24 {
        return Ok((vec![], vec![], false));
    }
    reader
        .seek(SeekFrom::Start(directory - 24))
        .map_err(|e| e.to_string())?;
    let mut footer = [0; 24];
    reader.read_exact(&mut footer).map_err(|e| e.to_string())?;
    if &footer[8..] != b"APK Sig Block 42" {
        return Ok((vec![], vec![], false));
    }
    let length = u64::from_le_bytes(footer[..8].try_into().unwrap());
    if !(24..=4 * 1024 * 1024).contains(&length) || length + 8 > directory {
        return Err("Invalid signing block length.".into());
    }
    reader
        .seek(SeekFrom::Start(directory - length - 8))
        .map_err(|e| e.to_string())?;
    let mut data = vec![0; length as usize + 8];
    reader.read_exact(&mut data).map_err(|e| e.to_string())?;
    if data[..8] != footer[..8] {
        return Err("Signing block lengths do not match.".into());
    }
    let mut records = &data[8..data.len() - 24];
    let mut schemes = Vec::new();
    let mut certificates = Vec::new();
    let mut verity = false;
    while !records.is_empty() {
        if records.len() < 12 {
            return Err("Truncated signing block.".into());
        }
        let length = u64::from_le_bytes(records[..8].try_into().unwrap());
        if length < 4 || length > (records.len() - 8) as u64 {
            return Err("Invalid signing pair length.".into());
        }
        let record = &records[8..8 + length as usize];
        records = &records[8 + length as usize..];
        let id = u32::from_le_bytes(record[..4].try_into().unwrap());
        let scheme = match id {
            0x7109871a => "v2",
            0xf05368c0 => "v3",
            0x1b93ad61 => "v3.1",
            _ => continue,
        };
        schemes.push(scheme.into());
        let mut encoded = &record[4..];
        let mut signers = take_length(&mut encoded)?;
        while !signers.is_empty() {
            let mut signer = take_length(&mut signers)?;
            let mut signed = take_length(&mut signer)?;
            let mut digests = take_length(&mut signed)?;
            while !digests.is_empty() {
                let digest = take_length(&mut digests)?;
                let id = u32::from_le_bytes(
                    digest
                        .get(..4)
                        .ok_or("Truncated signature digest.")?
                        .try_into()
                        .unwrap(),
                );
                verity |= matches!(id, 0x421..=0x423);
            }
            let mut certs = take_length(&mut signed)?;
            // Only the first certificate is the signer; remaining certificates
            // are chain entries, not additional signers.
            let certificate = take_length(&mut certs)?;
            if certificate.is_empty() {
                return Err("Empty signing certificate.".into());
            }
            let digest = Sha256::digest(certificate)
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(":");
            if !certificates.contains(&digest) {
                certificates.push(digest);
            }
        }
    }
    Ok((schemes, certificates, verity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn local_version_codes_match_package_manager_long_versions() {
        let tree = |minor: &str, major: Option<&str>| {
            let mut xml = format!(
                "N: android=http://schemas.android.com/apk/res/android\n  E: manifest (line=1)\n    A: http://schemas.android.com/apk/res/android:versionCode(0x0101021b)={minor}\n"
            );
            if let Some(major) = major {
                xml.push_str(&format!("    A: http://schemas.android.com/apk/res/android:versionCodeMajor(0x01010576)={major}\n"));
            }
            // Child attributes must never be interpreted as package versions.
            xml.push_str("      E: application (line=2)\n        A: http://schemas.android.com/apk/res/android:versionCodeMajor(0x01010576)=9\n");
            xml
        };
        assert_eq!(long_version_code(&tree("100", None)), Some("100".into()));
        assert_eq!(long_version_code(&tree("0", None)), Some("0".into()));
        assert_eq!(
            long_version_code(&tree("9", Some("1"))),
            Some("4294967305".into())
        );
        assert_eq!(
            long_version_code(&tree("0x1", Some("0x200000"))),
            Some("9007199254740993".into())
        );
        for invalid in ["", "Unknown", "-1", "4294967296", "\"1\"", "@0x7f010001"] {
            assert_eq!(long_version_code(&tree(invalid, None)), None);
            assert_eq!(long_version_code(&tree("1", Some(invalid))), None);
        }
        assert_eq!(long_version_code(""), None);
    }

    #[test]
    fn labels_prefer_english_and_xml_is_not_image_content() {
        let assets = parse_badging(
            "application-label:'Default'\napplication-label-fr:'Exemple'\napplication-label-en:'Example game'\n",
        );
        assert_eq!(assets.display_name.as_deref(), Some("Example game"));
        assert_eq!(unquote(r"'Players\''"), "Players'");
        assert_eq!(unquote(r"'Path\\'"), "Path\\");
        assert_eq!(
            attribute(
                "application: label='Player\\'s game' icon='res/icon.png'",
                "label"
            )
            .as_deref(),
            Some("Player's game")
        );
        assert!(raster_data_url(b"<svg onload='bad()'/>").is_none());
        assert!(take_length(&mut &[255, 255, 255, 255][..]).is_err());
    }

    #[test]
    fn oversized_and_malformed_zip_resources_are_rejected() {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("AndroidManifest.xml", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&[0; 64]).unwrap();
        let mut archive = ZipArchive::new(writer.finish().unwrap()).unwrap();
        assert!(entry(&mut archive, "AndroidManifest.xml", 16).is_err());
        assert!(entry(&mut archive, "missing", 64).is_err());
        assert!(
            signing(&mut Cursor::new(vec![0; 100]), 100)
                .unwrap()
                .0
                .is_empty()
        );
    }

    #[tokio::test]
    async fn bundled_aapt_reads_synthetic_fixture() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        tokio::task::spawn_blocking(move || {
            let file = fs::File::open(root.join("tests/fixtures/verification.apk")).unwrap();
            let assets = extract(
                file,
                &root.join("env/aapt2/aapt2.exe"),
                &root.join("env/test-artifacts/metadata"),
            )
            .unwrap();
            assert_eq!(
                assets.display_name.as_deref(),
                Some("Quest Manager verification")
            );
            assert!(assets.icon_data_url.is_none());
            assert!(assets.signing_schemes.contains(&"v2".to_string()));
            assert!(!assets.certificate_sha256.is_empty());
            let local = inspect_local(
                &root.join("tests/fixtures/verification.apk"),
                &root.join("env/aapt2/aapt2.exe"),
                &root.join("env/test-artifacts/metadata"),
            )
            .unwrap();
            assert_eq!(local.version_code, "1");
        })
        .await
        .unwrap();
    }
}
