//! Resource-only APK rebuilding. Game code/assets are copied as compressed ZIP entries.
use crate::adb::validate_windows_filename;
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, Event},
};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::Path,
};
use zip::{ZipArchive, ZipWriter};

const RESOURCE_BUDGET: u64 = 128 * 1024 * 1024;

fn resource(name: &str) -> bool {
    name == "AndroidManifest.xml" || name == "resources.arsc" || name.starts_with("res/")
}

fn signature(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    upper == "META-INF/MANIFEST.MF"
        || (upper.starts_with("META-INF/")
            && [".SF", ".RSA", ".DSA", ".EC"]
                .iter()
                .any(|ext| upper.ends_with(ext)))
}

pub fn stage_resources(source: &Path, output: &Path) -> Result<(), String> {
    let mut archive = ZipArchive::new(File::open(source).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut writer = ZipWriter::new(File::create_new(output).map_err(|e| e.to_string())?);
    let mut names = HashSet::new();
    let mut windows_names = HashSet::new();
    let mut size = 0u64;
    for i in 0..archive.len() {
        let file = archive.by_index(i).map_err(|e| e.to_string())?;
        if !names.insert(file.name().to_owned()) {
            return Err("APK contains duplicate ZIP entries.".into());
        }
        if !resource(file.name()) {
            continue;
        }
        if !windows_names.insert(file.name().to_lowercase()) {
            return Err("APK resource names differ only by letter case and cannot be rebuilt safely on Windows.".into());
        }
        for component in file.name().trim_end_matches('/').split('/') {
            validate_windows_filename(component)?;
        }
        if file.is_symlink() {
            return Err("APK resources containing symbolic links cannot be edited.".into());
        }
        size = size
            .checked_add(file.size())
            .ok_or("APK resource size overflow.")?;
        if size > RESOURCE_BUDGET {
            return Err("APK resources exceed the 128 MiB editing limit. Ordinary or compatibility-only installation is still available.".into());
        }
        writer.raw_copy_file(file).map_err(|e| e.to_string())?;
    }
    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn merge_resources(source: &Path, rebuilt: &Path, output: &Path) -> Result<(), String> {
    let mut original = ZipArchive::new(File::open(source).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut replacement = ZipArchive::new(File::open(rebuilt).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut writer = ZipWriter::new(File::create_new(output).map_err(|e| e.to_string())?);
    for i in 0..original.len() {
        let file = original.by_index(i).map_err(|e| e.to_string())?;
        if !resource(file.name()) && !signature(file.name()) {
            writer.raw_copy_file(file).map_err(|e| e.to_string())?;
        }
    }
    for i in 0..replacement.len() {
        let file = replacement.by_index(i).map_err(|e| e.to_string())?;
        if resource(file.name()) {
            writer.raw_copy_file(file).map_err(|e| e.to_string())?;
        }
    }
    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, Debug)]
struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Element>,
}
impl Element {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    fn set(&mut self, key: &str, value: &str) {
        self.attrs.retain(|(k, _)| k != key);
        self.attrs.push((key.into(), value.into()));
    }
    fn launcher(&self) -> bool {
        ["activity", "activity-alias"].contains(&self.name.as_str())
            && self.children.iter().any(|filter| {
                filter.name == "intent-filter"
                    && filter.children.iter().any(|n| {
                        n.name == "action"
                            && n.attr("android:name") == Some("android.intent.action.MAIN")
                    })
                    && filter.children.iter().any(|n| {
                        n.name == "category"
                            && matches!(
                                n.attr("android:name"),
                                Some(
                                    "android.intent.category.LAUNCHER"
                                        | "android.intent.category.LEANBACK_LAUNCHER"
                                        | "com.oculus.intent.category.VR"
                                )
                            )
                    })
            })
    }
    fn write(&self, writer: &mut Writer<Vec<u8>>) -> Result<(), String> {
        let mut event = BytesStart::new(&self.name);
        for (k, v) in &self.attrs {
            event.push_attribute((k.as_str(), v.as_str()));
        }
        writer
            .write_event(Event::Start(event))
            .map_err(|e| e.to_string())?;
        for child in &self.children {
            child.write(writer)?;
        }
        writer
            .write_event(Event::End(BytesEnd::new(&self.name)))
            .map_err(|e| e.to_string())
    }
}

fn parse_manifest(xml: &str) -> Result<Element, String> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;
    loop {
        let event = reader.read_event().map_err(|e| e.to_string())?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name().as_ref().to_owned();
                let attrs = e
                    .attributes()
                    .map(|a| {
                        let a = a.map_err(|e| e.to_string())?;
                        Ok((
                            a.key.as_ref().to_owned(),
                            a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                .map_err(|e| e.to_string())?
                                .into_owned(),
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let node = Element {
                    name,
                    attrs,
                    children: Vec::new(),
                };
                if stack.len() > 64 {
                    return Err("Manifest nesting exceeds the editing limit.".into());
                }
                if empty {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        root = Some(node);
                    }
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                let node = stack.pop().ok_or("Invalid manifest XML.")?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    root = Some(node);
                }
            }
            Event::DocType(_) => return Err("Manifest document types are not supported.".into()),
            Event::Text(t) if !t.as_ref().trim().is_empty() => {
                return Err("Unexpected text in manifest.".into());
            }
            Event::Eof => break,
            Event::CData(_) | Event::GeneralRef(_) => {
                return Err("Unexpected manifest content.".into());
            }
            _ => {}
        }
    }
    root.filter(|n| n.name == "manifest" && stack.is_empty())
        .ok_or("Invalid manifest XML.".into())
}

pub fn edit_decoded(
    directory: &Path,
    name: Option<&str>,
    icon: Option<&[u8]>,
) -> Result<(), String> {
    let path = directory.join("AndroidManifest.xml");
    let xml = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut root = parse_manifest(&xml)?;
    if root.attr("xmlns:android") != Some("http://schemas.android.com/apk/res/android") {
        return Err("This manifest uses an unsupported Android namespace.".into());
    }
    // Apktool represents a resource-free APK with package ID 0. Adding the first
    // label/icon needs a normal application resource package, not --shared-lib.
    let config_path = directory.join("apktool.yml");
    let config = fs::read_to_string(&config_path)
        .map_err(|e| e.to_string())?
        .replace("\r\n", "\n");
    let empty_package = "resourcesInfo:\n  packageId: 0\n  packageName: \n";
    if config.contains(empty_package) {
        let package = root.attr("package").ok_or("Missing package name.")?;
        crate::adb::validate_package(package)?;
        fs::write(
            &config_path,
            config.replace(
                empty_package,
                &format!("resourcesInfo:\n  packageId: 127\n  packageName: {package}\n"),
            ),
        )
        .map_err(|e| e.to_string())?;
    }
    if root.attr("split").is_some()
        || root.attr("android:sharedUserId").is_some()
        || root.children.iter().any(|n| n.name == "uses-split")
    {
        return Err(
            "Split APKs and shared-user packages cannot be edited or re-signed here.".into(),
        );
    }
    let app = root
        .children
        .iter_mut()
        .find(|n| n.name == "application")
        .ok_or("APK has no application declaration.")?;
    if app.attr("android:isSplitRequired") == Some("true") {
        return Err("This application requires split APKs.".into());
    }
    if app.children.iter().filter(|n| n.launcher()).count() > 1 {
        return Err("This APK has multiple launcher entries. Appearance editing is not supported; compatibility-only installation remains available.".into());
    }
    if name.is_some() {
        app.set("android:label", "@string/quest_manager_label");
    }
    if icon.is_some() {
        app.set("android:icon", "@drawable/quest_manager_icon");
        app.set("android:roundIcon", "@drawable/quest_manager_icon");
    }
    for launcher in app.children.iter_mut().filter(|n| n.launcher()) {
        if name.is_some() {
            launcher.set("android:label", "@string/quest_manager_label");
        }
        if icon.is_some() {
            launcher.set("android:icon", "@drawable/quest_manager_icon");
        }
    }
    // Never overwrite another resource with the reserved names.
    let resources = directory.join("res");
    if resources.exists() {
        for folder in fs::read_dir(&resources).map_err(|e| e.to_string())? {
            let folder = folder.map_err(|e| e.to_string())?.path();
            if !folder.is_dir() {
                continue;
            }
            for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?.path();
                let filename = entry.file_name().unwrap_or_default().to_string_lossy();
                if filename.starts_with("quest_manager_") {
                    return Err("Reserved editing resources already exist in this APK. Edit the original APK instead.".into());
                }
                if entry.extension().is_some_and(|e| e == "xml")
                    && entry.metadata().map_err(|e| e.to_string())?.len() <= 32 * 1024 * 1024
                {
                    let contents = fs::read_to_string(entry).map_err(|e| e.to_string())?;
                    if contents.contains("name=\"quest_manager_label\"")
                        || contents.contains("name=\"quest_manager_icon\"")
                    {
                        return Err("Reserved editing resource names already exist.".into());
                    }
                }
            }
        }
    }
    if let Some(name) = name {
        fs::create_dir_all(resources.join("values")).map_err(|e| e.to_string())?;
        let escaped = name
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\'', "\\'");
        let escaped = quick_xml::escape::escape(&escaped);
        fs::write(
            resources.join("values/quest_manager.xml"),
            format!(
                "<resources><string name=\"quest_manager_label\">\"{escaped}\"</string></resources>"
            ),
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(icon) = icon {
        fs::create_dir_all(resources.join("drawable-nodpi")).map_err(|e| e.to_string())?;
        fs::write(
            resources.join("drawable-nodpi/quest_manager_icon.png"),
            icon,
        )
        .map_err(|e| e.to_string())?;
    }
    let mut writer = Writer::new(Vec::new());
    root.write(&mut writer)?;
    fs::write(path, writer.into_inner()).map_err(|e| e.to_string())
}

pub fn validate_icon(data: &[u8]) -> Result<(), String> {
    if data.len() > 2 * 1024 * 1024 {
        return Err("Choose an icon smaller than 2 MiB.".into());
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_limits(png::Limits {
        bytes: 16 * 1024 * 1024,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|_| "The icon must be a valid PNG image.")?;
    let info = reader.info();
    if info.width != info.height || !(48..=1024).contains(&info.width) {
        return Err("The icon must be square and between 48 and 1024 pixels.".into());
    }
    let size = reader
        .output_buffer_size()
        .filter(|n| *n <= 16 * 1024 * 1024)
        .ok_or("Icon exceeds decoding limits.")?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|_| "The PNG icon is incomplete or invalid.")?;
    Ok(())
}

pub fn compare_payloads(source: &Path, output: &Path) -> Result<(), String> {
    let mut original = ZipArchive::new(File::open(source).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut result = ZipArchive::new(File::open(output).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    for i in 0..original.len() {
        let before = original.by_index(i).map_err(|e| e.to_string())?;
        if resource(before.name()) || signature(before.name()) {
            continue;
        }
        let after = result
            .by_name(before.name())
            .map_err(|_| "An original game file is missing from the prepared APK.")?;
        if before.size() != after.size()
            || before.crc32() != after.crc32()
            || before.compression() != after.compression()
        {
            return Err("An original game file changed during APK preparation.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_aliases_and_escaped_manifest_values_survive() {
        let xml = r#"<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.example.game"><application android:label="Player&apos;s &amp; game"><activity android:name=".Other"/><activity-alias android:name=".Launcher" android:targetActivity=".Game"><intent-filter><action android:name="android.intent.action.MAIN"/><category android:name="android.intent.category.LAUNCHER"/></intent-filter></activity-alias></application></manifest>"#;
        let manifest = parse_manifest(xml).unwrap();
        let app = &manifest.children[0];
        assert_eq!(app.attr("android:label"), Some("Player's & game"));
        assert!(!app.children[0].launcher());
        assert!(app.children[1].launcher());
        let mut output = Writer::new(Vec::new());
        manifest.write(&mut output).unwrap();
        let again = parse_manifest(&String::from_utf8(output.into_inner()).unwrap()).unwrap();
        assert_eq!(
            again.children[0].attr("android:label"),
            Some("Player's & game")
        );
        assert!(parse_manifest("<!DOCTYPE manifest><manifest/>").is_err());
    }
}
