//! Opt-in downloads from the author's repository. No device data enters HTTP requests.
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;

const API: &str = "https://api.github.com/repos/threethan/LightningLauncher";
const REPO: &str = "https://github.com/threethan/LightningLauncher";
pub const LAUNCHER: &str = "com.threethan.launcher";
pub const NAVIGATOR: &str = "com.threethan.launcher.service.navigator";
pub const VARIANTS: [&str; 3] = [
    LAUNCHER,
    "com.threethan.launcher.metastore",
    "com.threethan.launcher.playstore",
];
const MAX_APK: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub launcher_asset: Option<u64>,
    pub navigator_asset: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub tag: String,
    pub asset_id: u64,
    pub size: u64,
    pub published_at: String,
    pub sha256: Option<String>,
    #[serde(skip)]
    pub url: String,
    #[serde(skip)]
    pub package: String,
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub launchers: Vec<Release>,
    pub navigators: Vec<Release>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub launcher_tag: String,
    pub navigator_tag: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    id: u64,
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Default)]
struct Cache {
    catalog: Option<(Instant, Catalog)>,
    matches: BTreeMap<String, Recommendation>,
}

#[derive(Clone, Default)]
pub struct Lightning {
    cache: Arc<Mutex<Cache>>,
    gate: Arc<tokio::sync::Mutex<()>>,
}

fn allowed_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none_or(|port| port == 443)
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "raw.githubusercontent.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn client() -> Result<Client, String> {
    Client::builder()
        .user_agent("QuestManager/0.1 LightningInstaller")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(600))
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || !allowed_url(attempt.url()) {
                attempt.error("Unsupported download redirect")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| format!("Could not initialize HTTPS: {e}"))
}

async fn response(client: &Client, url: &str, metadata: bool) -> Result<reqwest::Response, String> {
    let url = Url::parse(url).map_err(|_| "Invalid GitHub URL.")?;
    if !allowed_url(&url) {
        return Err("Only the configured GitHub hosts are allowed.".into());
    }
    let mut request = client.get(url).header("X-GitHub-Api-Version", "2022-11-28");
    if metadata {
        request = request.timeout(Duration::from_secs(45));
    }
    let result = request
        .send()
        .await
        .map_err(|_| "Could not reach GitHub. Check your connection and try again.".to_string())?;
    match result.status().as_u16() {
        200 => Ok(result),
        403 | 429 => {
            Err("GitHub has limited requests. Wait a while, then refresh versions.".into())
        }
        404 => Err(
            "This release or its compatibility information is no longer available on GitHub."
                .into(),
        ),
        status => Err(format!("GitHub returned HTTP {status}. Try again later.")),
    }
}

async fn text(client: &Client, url: &str, limit: usize) -> Result<String, String> {
    let mut response = response(client, url, true).await?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "GitHub response was interrupted.")?
    {
        if bytes.len() + chunk.len() > limit {
            return Err("GitHub response exceeds the size limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| "GitHub returned invalid text.".into())
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 80
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        && tag != "."
        && tag != ".."
}

fn numeric_version(tag: &str) -> Vec<u64> {
    tag.trim_start_matches("addons")
        .trim_start_matches('v')
        .split('.')
        .map(|s| s.parse().unwrap_or(0))
        .collect()
}

pub fn matches_version(tag: &str, name: &str) -> bool {
    fn normalized(value: &str) -> Option<Vec<u64>> {
        let value = value.trim_start_matches("addons").trim_start_matches('v');
        let mut parts: Vec<u64> = value
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        while parts.len() > 1 && parts.last() == Some(&0) {
            parts.pop();
        }
        Some(parts)
    }
    normalized(tag).is_some_and(|value| Some(value) == normalized(name))
}

fn release_asset(release: &GithubRelease, name: &str, package: &str) -> Option<Release> {
    if release.draft || release.prerelease || !valid_tag(&release.tag_name) {
        return None;
    }
    let asset = release.assets.iter().find(|a| a.name == name)?;
    let expected = format!("{REPO}/releases/download/{}/{name}", release.tag_name);
    if asset.browser_download_url != expected || asset.size == 0 || asset.size > MAX_APK {
        return None;
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(|s| s.to_ascii_lowercase());
    Some(Release {
        tag: release.tag_name.clone(),
        asset_id: asset.id,
        size: asset.size,
        published_at: release.published_at.clone()?,
        sha256,
        url: expected,
        package: package.into(),
    })
}

fn parse_recommendation(source: &str) -> Option<String> {
    if !source.contains("\"ShortcutNavigator\"") || !source.contains(NAVIGATOR) {
        return None;
    }
    // Read only a recognized declaration; never evaluate upstream Java or infer by dates.
    let declaration = source
        .split("public static final String ADDON_TAG")
        .nth(1)?
        .split(';')
        .next()?;
    let candidates: Vec<_> = declaration
        .split('"')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, s)| s)
        .filter(|s| s.starts_with("addons") && *s != "addonsLegacy" && valid_tag(s))
        .collect();
    (candidates.len() == 1).then(|| candidates[0].to_string())
}

impl Lightning {
    pub async fn catalog(&self, refresh: bool) -> Result<Catalog, String> {
        let _gate = self.gate.lock().await;
        if !refresh
            && let Some((time, catalog)) = &self.cache.lock().unwrap().catalog
            && time.elapsed() < Duration::from_secs(600)
        {
            return Ok(catalog.clone());
        }
        let client = client()?;
        let mut catalog = Catalog::default();
        // Bounded pagination; this repository currently fits in the first page.
        for page in 1..=5 {
            let data = text(
                &client,
                &format!("{API}/releases?per_page=100&page={page}"),
                8 * 1024 * 1024,
            )
            .await?;
            let releases: Vec<GithubRelease> =
                serde_json::from_str(&data).map_err(|_| "Could not read GitHub releases.")?;
            for release in &releases {
                if let Some(asset) = release_asset(release, "LightningLauncher.apk", LAUNCHER) {
                    catalog.launchers.push(asset);
                }
                if let Some(asset) = release_asset(release, "ShortcutNavigator.apk", NAVIGATOR) {
                    catalog.navigators.push(asset);
                }
            }
            if releases.len() < 100 {
                break;
            }
        }
        catalog
            .launchers
            .sort_by_key(|r| std::cmp::Reverse(numeric_version(&r.tag)));
        catalog
            .navigators
            .sort_by_key(|r| std::cmp::Reverse(numeric_version(&r.tag)));
        if catalog.launchers.is_empty() {
            return Err("No stable Lightning Launcher APK releases were found.".into());
        }
        let mut cache = self.cache.lock().unwrap();
        cache.catalog = Some((Instant::now(), catalog.clone()));
        if refresh {
            cache.matches.clear();
        }
        Ok(catalog)
    }

    pub async fn recommendation(&self, tag: String) -> Result<Recommendation, String> {
        if !valid_tag(&tag) {
            return Err("Invalid launcher release.".into());
        }
        {
            let cache = self.cache.lock().unwrap();
            if !cache
                .catalog
                .as_ref()
                .is_some_and(|(_, c)| c.launchers.iter().any(|r| r.tag == tag))
            {
                return Err("Refresh releases before choosing a launcher version.".into());
            }
            if let Some(result) = cache.matches.get(&tag) {
                return Ok(result.clone());
            }
        }
        let source = text(&client()?, &format!("https://raw.githubusercontent.com/threethan/LightningLauncher/{tag}/Launcher/App/src/main/java/com/threethan/launcher/updater/AddonUpdater.java"), 128 * 1024).await?;
        let result = Recommendation {
            launcher_tag: tag.clone(),
            navigator_tag: parse_recommendation(&source),
        };
        self.cache
            .lock()
            .unwrap()
            .matches
            .insert(tag, result.clone());
        Ok(result)
    }

    pub fn resolve(&self, selection: &Selection) -> Result<Vec<Release>, String> {
        let cache = self.cache.lock().unwrap();
        let catalog = &cache
            .catalog
            .as_ref()
            .ok_or("Load available versions before installing.")?
            .1;
        let mut assets = Vec::new();
        for (id, entries) in [
            (selection.launcher_asset, &catalog.launchers),
            (selection.navigator_asset, &catalog.navigators),
        ] {
            if let Some(id) = id {
                assets.push(
                    entries
                        .iter()
                        .find(|r| r.asset_id == id)
                        .ok_or("The selected release is unavailable. Refresh versions.")?
                        .clone(),
                );
            }
        }
        if assets.is_empty() {
            return Err("Select the launcher or Navigator service to install.".into());
        }
        Ok(assets)
    }
}

pub async fn download(
    asset: &Release,
    path: &Path,
    report: &(dyn Fn(&str, Option<f64>) + Sync),
) -> Result<(), String> {
    let response = response(&client()?, &asset.url, false).await?;
    save_download(response, asset, path, report).await
}

async fn save_download(
    mut response: reqwest::Response,
    asset: &Release,
    path: &Path,
    report: &(dyn Fn(&str, Option<f64>) + Sync),
) -> Result<(), String> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
        .map_err(|e| format!("Could not create download: {e}"))?;
    let mut hash = Sha256::new();
    let mut received = 0u64;
    let mut last_percent = u64::MAX;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "APK download was interrupted. Start a new installation to retry.")?
    {
        received += chunk.len() as u64;
        if received > asset.size || received > MAX_APK {
            return Err("Download exceeded its published size.".into());
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Could not save APK: {e}"))?;
        hash.update(&chunk);
        let percent = received.saturating_mul(100) / asset.size;
        if percent != last_percent {
            report(
                &format!(
                    "Downloading {} · {}",
                    if asset.package == LAUNCHER {
                        "Lightning Launcher"
                    } else {
                        "Navigator service"
                    },
                    asset.tag
                ),
                Some(percent as f64),
            );
            last_percent = percent;
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    if received != asset.size {
        return Err("Download did not match the published size.".into());
    }
    let actual = format!("{:x}", hash.finalize());
    if asset
        .sha256
        .as_ref()
        .is_some_and(|expected| *expected != actual)
    {
        return Err("APK checksum did not match GitHub. Installation was stopped.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restricts_network_and_asset_identity() {
        for url in [
            "http://github.com/a",
            "https://github.com.evil.test/a",
            "https://127.0.0.1/a",
            "https://user@github.com/a",
            "https://github.com:444/a",
        ] {
            assert!(!allowed_url(&Url::parse(url).unwrap()));
        }
        assert!(allowed_url(
            &Url::parse("https://release-assets.githubusercontent.com/a").unwrap()
        ));
        assert!(!valid_tag("../../main"));
        let mut release: GithubRelease = serde_json::from_value(serde_json::json!({"tag_name":"1.0", "draft":false,"prerelease":false,"published_at":"2025-01-01", "assets":[{"id":1,"name":"LightningLauncher.apk","size":40,"browser_download_url":format!("{REPO}/releases/download/1.0/LightningLauncher.apk"),"digest":null}]})).unwrap();
        assert!(release_asset(&release, "LightningLauncher.apk", LAUNCHER).is_some());
        release.prerelease = true;
        assert!(release_asset(&release, "LightningLauncher.apk", LAUNCHER).is_none());
        release.prerelease = false;
        release.assets[0].browser_download_url =
            "https://github.com/another/repository/a.apk".into();
        assert!(release_asset(&release, "LightningLauncher.apk", LAUNCHER).is_none());
    }
    #[test]
    fn recommendation_requires_explicit_navigator_mapping() {
        let source = format!(
            "public static final String ADDON_TAG = old() ? \"addonsLegacy\" : \"addons1.2.0\"; \"ShortcutNavigator\" {NAVIGATOR}"
        );
        assert_eq!(
            parse_recommendation(&source).as_deref(),
            Some("addons1.2.0")
        );
        assert_eq!(
            parse_recommendation(&source.replace("ShortcutNavigator", "ShortcutLibrary")),
            None
        );
        assert_eq!(parse_recommendation("unknown layout"), None);
        assert!(numeric_version("10.0.1") > numeric_version("9.1.0"));
    }
    #[tokio::test]
    async fn streamed_download_checks_size_checksum_and_collisions() {
        use std::io::{Read, Write};
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../env/test-artifacts");
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let directory = root.join(format!(
            "lightning-download-{:x}",
            u64::from_le_bytes(random)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let bytes = b"Example APK bytes";
        for mode in ["ok", "checksum", "short", "large", "collision"] {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let _ = socket.read(&mut request);
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                )
                .unwrap();
                socket.write_all(bytes).unwrap();
            });
            let response = Client::builder()
                .no_proxy()
                .build()
                .unwrap()
                .get(format!("http://{address}/example.apk"))
                .send()
                .await
                .unwrap();
            let asset = Release {
                tag: "1.0".into(),
                asset_id: 1,
                size: (bytes.len() as i64
                    + if mode == "short" {
                        1
                    } else if mode == "large" {
                        -1
                    } else {
                        0
                    }) as u64,
                published_at: String::new(),
                sha256: Some(if mode == "checksum" {
                    "0".repeat(64)
                } else {
                    format!("{:x}", Sha256::digest(bytes))
                }),
                url: String::new(),
                package: "com.example.launcher".into(),
            };
            let path = directory.join(format!("{mode}.apk"));
            if mode == "collision" {
                std::fs::write(&path, b"keep existing").unwrap();
            }
            let result = save_download(response, &asset, &path, &|_, _| {}).await;
            assert_eq!(result.is_ok(), mode == "ok", "{mode}: {result:?}");
            if mode == "collision" {
                assert_eq!(std::fs::read(&path).unwrap(), b"keep existing");
            }
            if mode == "ok" {
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
            }
            server.join().unwrap();
        }
        assert!(matches_version("addons1.0.0", "1.0"));
        assert!(!matches_version("1.0", "2.0"));
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[tokio::test]
    #[ignore = "Downloads and verifies current official releases locally. No ADB or device access."]
    async fn github_release_download_and_signature_smoke() {
        let service = Lightning::default();
        let catalog = service.catalog(true).await.unwrap();
        let launcher = &catalog.launchers[0];
        let recommendation = service.recommendation(launcher.tag.clone()).await.unwrap();
        let navigator = catalog
            .navigators
            .iter()
            .find(|r| Some(&r.tag) == recommendation.navigator_tag.as_ref())
            .unwrap();
        let assets = service
            .resolve(&Selection {
                launcher_asset: Some(launcher.asset_id),
                navigator_asset: Some(navigator.asset_id),
            })
            .unwrap();
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../env");
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let directory = root.join("test-artifacts").join(format!(
            "lightning-network-{:x}",
            u64::from_le_bytes(random)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let installer = crate::apk_install::Installer::new(
            root.join("apk-tools"),
            root.join("aapt2/aapt2.exe"),
            directory.join("inspection"),
        );
        let result: Result<(), String> = async {
            for (index, asset) in assets.iter().enumerate() {
                let path = directory.join(format!("{index}.apk"));
                download(asset, &path, &|_, _| {}).await?;
                let apk = installer.inspect(path.to_string_lossy().into()).await?;
                if apk.package_name != asset.package
                    || !matches_version(&asset.tag, &apk.version_name)
                {
                    return Err("Release APK did not match the selected identity.".into());
                }
                installer.verify_original(&path, &directory).await?;
            }
            Ok(())
        }
        .await;
        std::fs::remove_dir_all(directory).unwrap();
        result.unwrap();
    }
}
