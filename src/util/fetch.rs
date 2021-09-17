use sha2::Digest;
use std::io;
use std::path::PathBuf;
use tar::Archive;
use xz::read::XzDecoder;

pub fn get(uri: impl AsRef<str>) -> anyhow::Result<String> {
    let response = reqwest::blocking::get(uri.as_ref())?;
    Ok(response.text()?)
}

pub fn extract(
    uri: impl AsRef<str>,
    dst: impl AsRef<std::path::Path>,
    prefix: Option<&str>,
) -> anyhow::Result<String> {
    let response = reqwest::blocking::get(uri.as_ref())?;
    let content = io::Cursor::new(response.bytes()?);
    let sha256 = sha256(content.get_ref());
    let decompressor = XzDecoder::new(content);
    let mut archive = Archive::new(decompressor);
    let mut target_path = PathBuf::new();
    target_path.push(&dst);
    if let Some(str) = prefix {
        for r in archive.entries()?.filter(|e| e.is_ok()) {
            let mut entry = r?;
            let relative_path = entry.path()?.strip_prefix(str)?.to_owned();
            if relative_path.as_os_str() != "" {
                entry.unpack(target_path.join(relative_path))?;
            }
        }
    } else {
        archive.unpack(dst)?;
    }

    Ok(sha256)
}

pub fn sha256(data: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(data))
}
