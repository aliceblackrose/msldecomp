use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use zip::ZipArchive;

pub struct ExtractedInputs {
    pub metadata: Vec<u8>,
    pub il2cpp: Option<Vec<u8>>,
    pub unity_version: Option<String>,
}

pub fn extract_il2cpp_inputs(path: &Path, abi: &str) -> Result<ExtractedInputs> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    if path.extension().and_then(|x| x.to_str()).is_some_and(|x| x.eq_ignore_ascii_case("apk")) {
        return inspect_apk(bytes, abi);
    }

    let mut outer = ZipArchive::new(Cursor::new(bytes)).context("opening XAPK")?;
    let mut metadata = None;
    let mut il2cpp = None;
    let mut unity_version = None;

    for i in 0..outer.len() {
        let mut file = outer.by_index(i)?;
        let name = file.name().to_owned();
        if !name.ends_with(".apk") { continue; }
        let mut apk = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut apk)?;
        if let Ok(found) = inspect_apk(apk, abi) {
            if metadata.is_none() && !found.metadata.is_empty() { metadata = Some(found.metadata); }
            if il2cpp.is_none() { il2cpp = found.il2cpp; }
            if unity_version.is_none() { unity_version = found.unity_version; }
        }
    }

    let metadata = metadata.context("could not find assets/bin/Data/Managed/Metadata/global-metadata.dat in XAPK")?;
    Ok(ExtractedInputs { metadata, il2cpp, unity_version })
}

fn inspect_apk(bytes: Vec<u8>, abi: &str) -> Result<ExtractedInputs> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).context("opening APK")?;
    let metadata_path = "assets/bin/Data/Managed/Metadata/global-metadata.dat";
    let lib_path = format!("lib/{abi}/libil2cpp.so");
    let mut metadata = None;
    let mut il2cpp = None;
    let mut unity_version = None;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_owned();
        if name == metadata_path {
            let mut b = Vec::with_capacity(file.size() as usize); file.read_to_end(&mut b)?; metadata = Some(b);
        } else if name == lib_path {
            let mut b = Vec::with_capacity(file.size() as usize); file.read_to_end(&mut b)?; il2cpp = Some(b);
        } else if name.ends_with("globalgamemanagers") {
            let mut b = Vec::with_capacity(file.size() as usize); file.read_to_end(&mut b)?;
            unity_version = find_unity_version(&b);
        }
    }

    if metadata.is_none() && il2cpp.is_none() && unity_version.is_none() {
        bail!("APK does not contain requested IL2CPP inputs")
    }
    Ok(ExtractedInputs { metadata: metadata.unwrap_or_default(), il2cpp, unity_version })
}

fn find_unity_version(data: &[u8]) -> Option<String> {
    for i in 0..data.len().saturating_sub(10) {
        if data.get(i..i + 2) != Some(b"60") && data.get(i..i + 2) != Some(b"20") { continue; }
        let end = data[i..].iter().position(|b| !b.is_ascii_alphanumeric() && *b != b'.').unwrap_or(0);
        if !(8..=24).contains(&end) { continue; }
        let s = std::str::from_utf8(&data[i..i + end]).ok()?;
        if s.matches('.').count() == 2 && s.chars().any(|c| matches!(c, 'f' | 'a' | 'b' | 'p')) {
            return Some(s.to_owned());
        }
    }
    None
}
