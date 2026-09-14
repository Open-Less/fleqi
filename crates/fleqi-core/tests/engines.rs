use fleqi_core::{files::*, model::ContextSnapshot};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, atomic::AtomicBool},
};

fn paths() -> EnginePaths {
    let bin = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/runtime-resources/bin");
    EnginePaths {
        ffmpeg: Some(bin.join("ffmpeg")),
        ffprobe: Some(bin.join("ffprobe")),
        qpdf: Some(bin.join("qpdf")),
    }
}
fn executor(dir: &Path, sources: &[PathBuf]) -> FileExecutor {
    FileExecutor::new(
        ContextSnapshot {
            id: "fixture".into(),
            host_window: "fixture".into(),
            directory: dir.to_string_lossy().into(),
            files: sources.iter().map(|p| p.to_string_lossy().into()).collect(),
            captured_at: 0,
        },
        None,
        paths(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}
fn action(value: serde_json::Value) -> FileAction {
    serde_json::from_value(value).unwrap()
}
fn pdf(path: &Path, pages: usize) {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!(
            "<< /Type /Pages /Count {pages} /Kids [{}] >>",
            (0..pages)
                .map(|i| format!("{} 0 R", 3 + i * 2))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    ];
    for i in 0..pages {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 400] /Resources << >> /Contents {} 0 R >>",4+i*2));
        objects.push("<< /Length 0 >>\nstream\n\nendstream".into());
    }
    let mut body = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(body.len());
        write!(&mut body, "{} 0 obj\n{}\nendobj\n", i + 1, object).unwrap();
    }
    let xref = body.len();
    write!(
        &mut body,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    )
    .unwrap();
    for offset in offsets {
        writeln!(&mut body, "{offset:010} 00000 n ").unwrap();
    }
    write!(
        &mut body,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .unwrap();
    fs::write(path, body).unwrap();
}
#[test]
#[ignore = "requires bundled FFmpeg/ffprobe; run pnpm test:engines"]
fn converts_real_audio_and_reports_unmet_size_goals() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("中文 audio.wav");
    let samples = 44100u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + samples * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&44100u32.to_le_bytes());
    bytes.extend_from_slice(&88200u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples * 2).to_le_bytes());
    for i in 0..samples {
        let sample = ((i as f64 * 440. * std::f64::consts::TAU / 44100.).sin() * 16000.) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(&wav, bytes).unwrap();
    let mut engine = executor(dir.path(), std::slice::from_ref(&wav));
    let result = engine
        .execute(
            &action(
                serde_json::json!({"operation":"media_convert","sources":[wav],"format":"mp3"}),
            ),
            &mut |_, _| {},
        )
        .unwrap();
    assert!(result.verified);
    assert_eq!(result.details["format"], "mp3");
    assert!(result.details["duration"].as_f64().unwrap() > 0.9);
    let result=engine.execute(&action(serde_json::json!({"operation":"media_convert","sources":[wav],"format":"m4a","maxBytes":10})),&mut |_,_|{}).unwrap();
    assert!(!result.verified);
    assert!(!result.outputs.is_empty());
    assert!(wav.exists());
}
#[test]
#[ignore = "requires bundled qpdf; run pnpm test:engines"]
fn merges_extracts_rotates_and_splits_real_pdf_files() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.pdf");
    let second = dir.path().join("第二份.pdf");
    pdf(&first, 2);
    pdf(&second, 1);
    let mut engine = executor(dir.path(), &[first.clone(), second.clone()]);
    let merged = engine
        .execute(
            &action(serde_json::json!({"operation":"pdf_merge","sources":[first,second]})),
            &mut |_, _| {},
        )
        .unwrap();
    assert!(merged.verified);
    let extracted=engine.execute(&action(serde_json::json!({"operation":"pdf_extract","sources":merged.outputs,"pages":"2-3"})),&mut |_,_|{}).unwrap();
    let rotated=engine.execute(&action(serde_json::json!({"operation":"pdf_rotate","sources":extracted.outputs,"rotation":90})),&mut |_,_|{}).unwrap();
    let split = engine
        .execute(
            &action(serde_json::json!({"operation":"pdf_split","sources":rotated.outputs})),
            &mut |_, _| {},
        )
        .unwrap();
    assert_eq!(split.outputs.len(), 2);
    let output = Command::new(paths().qpdf.unwrap())
        .arg("--show-npages")
        .arg(&split.outputs[0])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
    assert!(first.exists());
    assert!(second.exists());
}
