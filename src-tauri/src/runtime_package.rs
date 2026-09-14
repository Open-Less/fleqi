use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub fn prepare(resources: &Path, data: &Path) -> io::Result<PathBuf> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(resources.join("runtime-manifest.json"))?)?;
    let expected = manifest["piArchiveSha256"]
        .as_str()
        .filter(|s| s.len() == 64 && s.bytes().all(|v| v.is_ascii_hexdigit()))
        .ok_or_else(|| io::Error::other("内置 PI 归档清单无效，请重新安装应用"))?;
    let root = data.join("runtime-cache");
    fs::create_dir_all(&root)?;
    let destination = root.join(expected);
    let ready = |folder: &Path| {
        folder.join(".ready").is_file()
            && folder.join("entry.mjs").is_file()
            && folder
                .join("node_modules/@earendil-works/pi-coding-agent/dist/index.js")
                .is_file()
    };
    if ready(&destination) {
        return Ok(destination);
    }
    let archive = resources.join("pi.tar.gz");
    let mut source = fs::File::open(&archive)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = source.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        return Err(io::Error::other("内置 PI 归档校验失败，请重新安装应用"));
    }
    let staging = tempfile::Builder::new()
        .prefix(".unpack-")
        .tempdir_in(&root)?;
    tar::Archive::new(GzDecoder::new(fs::File::open(&archive)?)).unpack(staging.path())?;
    fs::write(staging.path().join(".ready"), expected)?;
    if !ready(staging.path()) {
        return Err(io::Error::other("内置 PI 依赖不完整，请重新安装应用"));
    }
    // A crash during extraction never leaves a usable marker in the final path.
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::rename(staging.path(), &destination)?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_os = "macos")]
    fn packaged_runtime_loads_without_global_node_or_pi() {
        let resources = std::env::var_os("FLEQI_TEST_RESOURCES")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime-resources"));
        let data = tempfile::tempdir().unwrap();
        let pi = prepare(&resources, data.path()).unwrap();
        let output=std::process::Command::new(resources.join("bin/node"))
            .current_dir(&pi).env_clear().env("PATH","/usr/bin:/bin").env("PI_OFFLINE","1")
            .args(["--input-type=module","-e","const sdk=await import('@earendil-works/pi-coding-agent'); if(typeof sdk.createAgentSession!=='function')process.exit(1); console.log('bundled-sdk-ready')"])
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("bundled-sdk-ready"));
        assert_eq!(prepare(&resources, data.path()).unwrap(), pi);
    }
}
