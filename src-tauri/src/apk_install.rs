//! Local, opt-in APK preparation. Called under the existing mutation queue.
use crate::{
    adb::Adb,
    apk::{self, LocalApk},
    apk_edit,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::Semaphore,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallOptions {
    pub source_stamp: String,
    pub display_name: Option<String>,
    pub icon_png: Option<String>,
    pub compatibility: bool,
}

impl InstallOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_stamp.is_empty() || self.source_stamp.len() > 100 {
            return Err("Read the APK preview again before preparing it.".into());
        }
        if !self.compatibility && self.display_name.is_none() && self.icon_png.is_none() {
            return Err("Choose an appearance change or Compatibility install.".into());
        }
        if let Some(name) = &self.display_name
            && (name.trim().is_empty()
                || name.chars().count() > 120
                || name.chars().any(char::is_control))
        {
            return Err(
                "Use an application name of 1–120 characters without control characters.".into(),
            );
        }
        self.icon()?;
        Ok(())
    }
    fn icon(&self) -> Result<Option<Vec<u8>>, String> {
        self.icon_png
            .as_ref()
            .map(|encoded| {
                if encoded.len() > 3 * 1024 * 1024 {
                    return Err("The icon is too large.".into());
                }
                let data = STANDARD
                    .decode(encoded)
                    .map_err(|_| "The icon is not valid PNG data.")?;
                apk_edit::validate_icon(&data)?;
                Ok(data)
            })
            .transpose()
    }
}

#[derive(Clone)]
pub struct Installer {
    pub tools: PathBuf,
    pub aapt: PathBuf,
    pub storage: PathBuf,
    preview_gate: Arc<Semaphore>,
}

pub struct PreparedApk {
    pub path: PathBuf,
    pub directory: PathBuf,
    pub preview: LocalApk,
}

impl Installer {
    pub fn new(tools: PathBuf, aapt: PathBuf, storage: PathBuf) -> Self {
        Self {
            tools,
            aapt,
            storage,
            preview_gate: Arc::new(Semaphore::new(1)),
        }
    }
    pub async fn inspect(&self, source: String) -> Result<LocalApk, String> {
        let _permit = self
            .preview_gate
            .acquire()
            .await
            .map_err(|e| e.to_string())?;
        let path = local_apk(&source)?;
        let aapt = self.aapt.clone();
        let scratch = self.storage.join("preview");
        blocking(move || apk::inspect_local(&path, &aapt, &scratch)).await
    }
    async fn java(
        &self,
        jar: &str,
        args: Vec<String>,
        stage: &str,
        directory: &Path,
    ) -> Result<String, String> {
        let mut arguments = vec![
            "-Xmx1024m".into(),
            "-Djava.awt.headless=true".into(),
            format!("-Djava.io.tmpdir={}", text_path(directory)),
            "-jar".into(),
            text_path(&self.tools.join(jar)),
        ];
        arguments.extend(args);
        run_tool(self.tools.join("jre/bin/java.exe"), arguments, stage).await
    }
    pub async fn prepare(
        &self,
        source: &Path,
        options: &InstallOptions,
        id: &str,
        report: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<PreparedApk, String> {
        options.validate()?;
        if apk::source_stamp(source)? != options.source_stamp {
            return Err("The APK changed after preview. Select it again before installing.".into());
        }
        for path in [
            "jre/bin/java.exe",
            "jre/bin/keytool.exe",
            "apksigner.jar",
            "apktool.jar",
            "zipalign.exe",
        ] {
            if !self.tools.join(path).is_file() {
                return Err("APK preparation tools are missing. Run run-install.ps1 or restore the bundled apk-tools folder.".into());
            }
        }
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Invalid preparation task identifier.".into());
        }
        let parent = self.storage.join("staging");
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        check_space(
            &parent,
            fs::metadata(source)
                .map_err(|e| e.to_string())?
                .len()
                .saturating_mul(4)
                .saturating_add(512 * 1024 * 1024),
        )
        .await?;
        let directory = parent.join(id);
        fs::create_dir(&directory)
            .map_err(|e| format!("Could not create a private APK working directory: {e}"))?;
        let result = self.prepare_in(source, options, &directory, report).await;
        match result {
            Ok((path, preview)) => Ok(PreparedApk {
                path,
                directory,
                preview,
            }),
            Err(error) => Err(cleanup_result(&directory, Err(error)).await.unwrap_err()),
        }
    }
    async fn prepare_in(
        &self,
        source: &Path,
        options: &InstallOptions,
        directory: &Path,
        report: &(dyn Fn(&str) + Send + Sync),
    ) -> Result<(PathBuf, LocalApk), String> {
        report("Copying the original APK to a private working file…");
        let original = directory.join("original.apk");
        tokio::fs::copy(source, &original)
            .await
            .map_err(|e| format!("Could not stage APK: {e}"))?;
        if apk::source_stamp(source)? != options.source_stamp {
            return Err("The original APK changed while being copied. Select it again.".into());
        }
        let before = self.inspect(text_path(&original)).await?;
        if before.split {
            return Err("Split APKs cannot be prepared by this installer.".into());
        }
        let editing = options.display_name.is_some() || options.icon_png.is_some();
        let mut input = original.clone();
        if editing {
            report("Decoding application resources. Game files are kept intact…");
            let resources = directory.join("resources.apk");
            let source = original.clone();
            let target = resources.clone();
            blocking(move || apk_edit::stage_resources(&source, &target)).await?;
            let decoded = directory.join("decoded");
            let framework = directory.join("framework");
            self.java(
                "apktool.jar",
                vec![
                    "d".into(),
                    text_path(&resources),
                    "-s".into(),
                    "--no-assets".into(),
                    "-j".into(),
                    "1".into(),
                    "-p".into(),
                    text_path(&framework),
                    "-o".into(),
                    text_path(&decoded),
                ],
                "Resource decoding",
                directory,
            )
            .await?;
            let name = options.display_name.clone();
            let icon = options.icon()?;
            let folder = decoded.clone();
            blocking(move || apk_edit::edit_decoded(&folder, name.as_deref(), icon.as_deref()))
                .await?;
            report("Rebuilding application name and icon…");
            let rebuilt = directory.join("resources-rebuilt.apk");
            self.java(
                "apktool.jar",
                vec![
                    "b".into(),
                    text_path(&decoded),
                    "-j".into(),
                    "1".into(),
                    "--no-crunch".into(),
                    "--aapt".into(),
                    text_path(&self.aapt),
                    "-p".into(),
                    text_path(&framework),
                    "-o".into(),
                    text_path(&rebuilt),
                ],
                "Resource rebuilding",
                directory,
            )
            .await?;
            let merged = directory.join("modified.apk");
            let source = original.clone();
            let target = merged.clone();
            blocking(move || apk_edit::merge_resources(&source, &rebuilt, &target)).await?;
            input = merged;
        }
        report("Aligning APK files before signing…");
        let aligned = directory.join("aligned.apk");
        run_tool(
            self.tools.join("zipalign.exe"),
            vec![
                "-P".into(),
                "16".into(),
                "4".into(),
                text_path(&input),
                text_path(&aligned),
            ],
            "APK alignment",
        )
        .await?;
        if input != original {
            tokio::fs::remove_file(&input)
                .await
                .map_err(|e| e.to_string())?;
        }
        let key = self.key_for(&before.package_name).await?;
        report("Signing a compatible APK with verity disabled…");
        let output = directory.join("prepared.apk");
        self.java(
            "apksigner.jar",
            vec![
                "sign".into(),
                "--ks".into(),
                text_path(&key.join("signing.p12")),
                "--ks-key-alias".into(),
                "local-apk".into(),
                "--ks-pass".into(),
                format!("file:{}", text_path(&key.join("password.txt"))),
                "--v1-signing-enabled".into(),
                "true".into(),
                "--v2-signing-enabled".into(),
                "true".into(),
                "--v3-signing-enabled".into(),
                "false".into(),
                "--v4-signing-enabled".into(),
                "false".into(),
                "--verity-enabled".into(),
                "false".into(),
                "--out".into(),
                text_path(&output),
                text_path(&aligned),
            ],
            "APK signing",
            directory,
        )
        .await?;
        report("Verifying the signature and the prepared application preview…");
        self.java(
            "apksigner.jar",
            vec!["verify".into(), "--verbose".into(), text_path(&output)],
            "APK signature verification",
            directory,
        )
        .await?;
        run_tool(
            self.tools.join("zipalign.exe"),
            vec![
                "-c".into(),
                "-P".into(),
                "16".into(),
                "4".into(),
                text_path(&output),
            ],
            "APK alignment verification",
        )
        .await?;
        let after = self.inspect(text_path(&output)).await?;
        if after.verity_signing != Some(false) {
            return Err("The prepared APK compatibility signature could not be confirmed.".into());
        }
        if before.package_name != after.package_name
            || before.version_code != after.version_code
            || before.version_name != after.version_name
        {
            return Err(
                "APK identity or version changed during preparation. Installation was stopped."
                    .into(),
            );
        }
        if let Some(name) = &options.display_name
            && after.assets.display_name.as_deref() != Some(name)
        {
            return Err(format!(
                "The prepared APK name did not match. Expected {name:?}, read {:?}. Installation was stopped.",
                after.assets.display_name
            ));
        }
        if let Some(icon) = &options.icon_png
            && after.assets.icon_data_url.as_deref()
                != Some(&format!("data:image/png;base64,{icon}"))
        {
            return Err(
                "The prepared APK icon did not match the preview. Installation was stopped.".into(),
            );
        }
        let source = original.clone();
        let target = output.clone();
        blocking(move || apk_edit::compare_payloads(&source, &target)).await?;
        Ok((output, after))
    }
    async fn key_for(&self, package: &str) -> Result<PathBuf, String> {
        // Separate app keys avoid introducing shared signature-level permissions.
        let name = format!("{:x}", Sha256::digest(package.as_bytes()));
        let directory = self.storage.join("signing-keys").join(name);
        if directory.is_dir() {
            if directory.join("signing.p12").is_file() && directory.join("password.txt").is_file() {
                return Ok(directory);
            }
            return Err(format!(
                "The signing key is incomplete. Restore its backup before retrying: {}",
                directory.display()
            ));
        }
        fs::create_dir_all(directory.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::create_dir(&directory).map_err(|e| e.to_string())?;
        // ACLs are applied before creating the secret. No key material enters logs.
        let mut command = Adb::new(powershell()).command(&["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), "$ErrorActionPreference='Stop'; $p=$env:QUEST_APK_KEY_DIRECTORY; $a=[IO.Directory]::GetAccessControl($p); $a.SetAccessRuleProtection($true,$false); $s=[Security.Principal.WindowsIdentity]::GetCurrent().User; $r=[Security.AccessControl.FileSystemAccessRule]::new($s,'FullControl','ContainerInherit,ObjectInherit','None','Allow'); $a.AddAccessRule($r); [IO.Directory]::SetAccessControl($p,$a)".into()]);
        command.env("QUEST_APK_KEY_DIRECTORY", text_path(&directory));
        let status = tokio::time::timeout(Duration::from_secs(30), command.output())
            .await
            .map_err(|_| "Signing storage protection timed out.")?
            .map_err(|e| e.to_string())?;
        if !status.status.success() {
            return Err(format!(
                "Could not restrict access to the local signing key folder: {}",
                String::from_utf8_lossy(&status.stderr)
            ));
        }
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
        fs::write(directory.join("password.txt"), STANDARD.encode(bytes))
            .map_err(|e| e.to_string())?;
        run_tool(
            self.tools.join("jre/bin/keytool.exe"),
            vec![
                "-genkeypair".into(),
                "-keystore".into(),
                text_path(&directory.join("signing.p12")),
                "-storetype".into(),
                "PKCS12".into(),
                "-storepass:file".into(),
                text_path(&directory.join("password.txt")),
                "-alias".into(),
                "local-apk".into(),
                "-keyalg".into(),
                "RSA".into(),
                "-keysize".into(),
                "2048".into(),
                "-validity".into(),
                "10000".into(),
                "-dname".into(),
                "CN=Local APK Compatibility".into(),
                "-noprompt".into(),
            ],
            "Local signing key generation",
        )
        .await?;
        Ok(directory)
    }
}

pub fn local_apk(source: &str) -> Result<PathBuf, String> {
    let path = Path::new(source);
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("apk"))
    {
        return Err("Select an absolute local .apk file path.".into());
    }
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    apk::source_stamp(&path)?;
    Ok(path)
}
fn text_path(path: &Path) -> String {
    // Java's JAR class loader cannot resolve Windows verbatim path prefixes.
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{unc}")
    } else {
        text.strip_prefix("\\\\?\\").unwrap_or(&text).to_owned()
    }
}
fn powershell() -> PathBuf {
    PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
        .join("System32/WindowsPowerShell/v1.0/powershell.exe")
}

async fn check_space(path: &Path, required: u64) -> Result<(), String> {
    let mut command = Adb::new(powershell()).command(&["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), "$ErrorActionPreference='Stop'; $p=$env:QUEST_APK_WORK_DIRECTORY; $r=[IO.Path]::GetPathRoot($p); $d=[IO.DriveInfo]::new($r); [Console]::Write($d.AvailableFreeSpace)".into()]);
    // PowerShell/.NET DriveInfo expects a conventional drive path.
    command.env(
        "QUEST_APK_WORK_DIRECTORY",
        path.to_string_lossy().trim_start_matches("\\\\?\\"),
    );
    let result = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| "Disk space check timed out.")?
        .map_err(|e| e.to_string())?;
    let available = String::from_utf8_lossy(&result.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|_| "Could not check free space for APK preparation.")?;
    if !result.status.success() || available < required {
        return Err(format!(
            "APK preparation requires about {:.1} GiB of free local space. Free space and try again.",
            required as f64 / 1073741824.0
        ));
    }
    Ok(())
}

pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}
async fn tail(mut input: impl AsyncRead + Unpin) -> Vec<u8> {
    let mut tail = Vec::new();
    let mut chunk = [0; 4096];
    while let Ok(count) = input.read(&mut chunk).await {
        if count == 0 {
            break;
        }
        tail.extend_from_slice(&chunk[..count]);
        if tail.len() > 16384 {
            tail.drain(..tail.len() - 8192);
        }
    }
    tail
}
async fn run_tool(path: PathBuf, args: Vec<String>, stage: &str) -> Result<String, String> {
    let mut child = Adb::new(path)
        .command(&args)
        .spawn()
        .map_err(|e| format!("{stage} could not start: {e}"))?;
    let stdout = tokio::spawn(tail(child.stdout.take().ok_or("Missing tool output.")?));
    let stderr = tokio::spawn(tail(child.stderr.take().ok_or("Missing tool output.")?));
    let status = match tokio::time::timeout(Duration::from_secs(3600), child.wait()).await {
        Ok(status) => status.map_err(|e| e.to_string())?,
        Err(_) => {
            // Apktool may own an AAPT2 child. Terminate only this task's process
            // tree so a timeout cannot leave it writing into the staging folder.
            if let Some(pid) = child.id() {
                let taskkill = powershell()
                    .ancestors()
                    .nth(3)
                    .unwrap()
                    .join("taskkill.exe");
                let _ = tokio::time::timeout(
                    Duration::from_secs(15),
                    Adb::new(taskkill)
                        .command(&["/PID".into(), pid.to_string(), "/T".into(), "/F".into()])
                        .output(),
                )
                .await;
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            stdout.abort();
            stderr.abort();
            return Err(format!("{stage} timed out after one hour."));
        }
    };
    let out = stdout.await.unwrap_or_default();
    let err = stderr.await.unwrap_or_default();
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );
    if !status.success() {
        return Err(format!(
            "{stage} failed. {}",
            combined
                .chars()
                .rev()
                .take(1600)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ));
    }
    Ok(combined)
}
pub async fn cleanup_result(
    directory: &Path,
    result: Result<String, String>,
) -> Result<String, String> {
    match tokio::fs::remove_dir_all(directory).await {
        Ok(()) => result,
        Err(error) => Err(format!(
            "{}\nCould not remove APK working files at {}: {error}",
            result.unwrap_or_else(|e| e),
            directory.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 64, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[30, 120, 80, 255].repeat(64 * 64))
                .unwrap();
        }
        bytes
    }

    #[tokio::test]
    #[ignore = "Local-only preparation of QUEST_TEST_APK, optionally editing with QUEST_TEST_APK_EDIT=1. Never installs anything."]
    async fn local_selected_apk_preparation() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .unwrap();
        let source = local_apk(
            &std::env::var("QUEST_TEST_APK").expect("Set QUEST_TEST_APK to a local APK."),
        )
        .unwrap();
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let work = root
            .join("env/test-artifacts")
            .join(format!("selected-apk-{:x}", u64::from_le_bytes(random)));
        let installer = Installer::new(
            root.join("env/apk-tools"),
            root.join("env/aapt2/aapt2.exe"),
            work.clone(),
        );
        let result: Result<(), String> = async {
            let preview = installer.inspect(text_path(&source)).await?;
            let edit = std::env::var("QUEST_TEST_APK_EDIT").as_deref() == Ok("1");
            let options = InstallOptions {
                source_stamp: preview.source_stamp,
                display_name: edit.then(|| "Example local preview".into()),
                icon_png: edit.then(|| STANDARD.encode(test_png())),
                compatibility: true,
            };
            let prepared = installer
                .prepare(&source, &options, "local", &|stage| eprintln!("{stage}"))
                .await?;
            assert_eq!(prepared.preview.verity_signing, Some(false));
            cleanup_result(&prepared.directory, Ok(String::new())).await?;
            Ok(())
        }
        .await;
        fs::remove_dir_all(&work).unwrap();
        result.unwrap();
    }

    #[test]
    fn refuses_invalid_options_and_malformed_icons() {
        let mut options = InstallOptions {
            source_stamp: "123:456".into(),
            display_name: None,
            icon_png: None,
            compatibility: false,
        };
        assert!(options.validate().is_err());
        options.compatibility = true;
        assert!(options.validate().is_ok());
        options.display_name = Some("\n".into());
        assert!(options.validate().is_err());
        options.display_name = None;
        options.icon_png = Some(STANDARD.encode(b"not a PNG"));
        assert!(options.validate().is_err());
        options.icon_png = Some(STANDARD.encode(test_png()));
        assert!(options.validate().is_ok());
    }

    #[tokio::test]
    async fn prepares_signed_apks_locally_and_preserves_game_payloads() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .unwrap();
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let work = root
            .join("env/test-artifacts")
            .join(format!("apk-prepare-{:x}", u64::from_le_bytes(random)));
        fs::create_dir_all(&work).unwrap();
        let source = work.join("Original 日本語.apk");
        fs::copy(root.join("tests/fixtures/verification.apk"), &source).unwrap();
        // Synthetic compressed game content exercises the raw-copy reconstruction path.
        {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&source)
                .unwrap();
            let mut archive = zip::ZipWriter::new_append(file).unwrap();
            archive
                .start_file(
                    "assets/example-game.bin",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .unwrap();
            archive.write_all(&[1, 2, 3, 4].repeat(1024 * 64)).unwrap();
            archive.finish().unwrap();
        }
        let original_hash = Sha256::digest(fs::read(&source).unwrap());
        let installer = Installer::new(
            root.join("env/apk-tools"),
            root.join("env/aapt2/aapt2.exe"),
            work.join("private"),
        );
        let result: Result<(), String> = async {
            let preview = installer.inspect(text_path(&source)).await?;
            let mut options = InstallOptions {
                source_stamp: preview.source_stamp,
                display_name: Some("Example 日本語 & Player's \"Game\"'".into()),
                icon_png: Some(STANDARD.encode(test_png())),
                compatibility: false,
            };
            let prepared = installer
                .prepare(&source, &options, "appearance", &|_| {})
                .await?;
            assert_eq!(prepared.preview.assets.display_name, options.display_name);
            assert!(!prepared.preview.assets.certificate_sha256.is_empty());
            assert_eq!(prepared.preview.assets.signing_schemes, vec!["v2"]);
            assert_eq!(prepared.preview.verity_signing, Some(false));
            let key = prepared.preview.assets.certificate_sha256.clone();
            // Check content bytes as well as preparation's archive metadata check.
            let mut result = zip::ZipArchive::new(fs::File::open(&prepared.path).unwrap()).unwrap();
            let mut payload = Vec::new();
            std::io::Read::read_to_end(
                &mut result.by_name("assets/example-game.bin").unwrap(),
                &mut payload,
            )
            .unwrap();
            assert_eq!(payload, [1, 2, 3, 4].repeat(1024 * 64));
            drop(result);
            cleanup_result(&prepared.directory, Ok(String::new())).await?;
            options.display_name = None;
            options.icon_png = None;
            options.compatibility = true;
            let compatible = installer
                .prepare(&source, &options, "compatibility", &|_| {})
                .await?;
            assert_eq!(key, compatible.preview.assets.certificate_sha256);
            assert_eq!(
                compatible.preview.assets.display_name,
                preview.assets.display_name
            );
            cleanup_result(&compatible.directory, Ok(String::new())).await?;
            options.source_stamp = "changed".into();
            assert!(
                installer
                    .prepare(&source, &options, "changed", &|_| {})
                    .await
                    .is_err()
            );
            assert!(!installer.storage.join("staging/changed").exists());
            assert_eq!(Sha256::digest(fs::read(&source).unwrap()), original_hash);
            Ok(())
        }
        .await;
        fs::remove_dir_all(&work).unwrap();
        result.unwrap();
    }
}
