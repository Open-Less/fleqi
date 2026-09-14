use fleqi_core::{files::*, model::ContextSnapshot};
use std::{
    fs,
    sync::{Arc, atomic::AtomicBool},
};

fn executor(root: &std::path::Path, files: Vec<String>) -> FileExecutor {
    FileExecutor::new(
        ContextSnapshot {
            id: "test".into(),
            host_window: "fixture".into(),
            directory: root.to_string_lossy().into(),
            files,
            captured_at: 0,
        },
        None,
        EnginePaths::default(),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}
#[test]
fn copies_special_names_without_overwrite_and_rejects_unselected_sources() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("中文 ' $(touch nope).txt");
    fs::write(&source, "original").unwrap();
    let other = dir.path().join("other.txt");
    fs::write(&other, "private").unwrap();
    let mut executor = executor(dir.path(), vec![source.to_string_lossy().into()]);
    let request: FileAction = serde_json::from_value(
        serde_json::json!({"operation":"copy","sources":[source],"destination":dir.path()}),
    )
    .unwrap();
    let result = executor.execute(&request, &mut |_, _| {}).unwrap();
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(fs::read_to_string(&result.outputs[0]).unwrap(), "original");
    assert_eq!(fs::read_to_string(&source).unwrap(), "original");
    let bad: FileAction =
        serde_json::from_value(serde_json::json!({"operation":"trash","sources":[other]})).unwrap();
    assert!(executor.execute(&bad, &mut |_, _| {}).is_err());
}
#[test]
fn image_conversion_verifies_actual_format_and_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("input.png");
    image::RgbImage::new(200, 100).save(&source).unwrap();
    let mut executor = executor(dir.path(), vec![source.to_string_lossy().into()]);
    let request:FileAction=serde_json::from_value(serde_json::json!({"operation":"image_convert","sources":[source],"format":"jpg","width":80})).unwrap();
    let result = executor.execute(&request, &mut |_, _| {}).unwrap();
    let image = image::open(&result.outputs[0]).unwrap();
    assert_eq!((image.width(), image.height()), (80, 40));
    assert!(source.exists());
}
#[test]
fn rejects_zip_traversal_and_obeys_cancellation_before_mutation() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let archive = dir.path().join("bad.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
    zip.start_file("../escape.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"escape").unwrap();
    zip.finish().unwrap();
    let mut executor = executor(dir.path(), vec![archive.to_string_lossy().into()]);
    let request: FileAction =
        serde_json::from_value(serde_json::json!({"operation":"zip_extract","sources":[archive]}))
            .unwrap();
    assert!(executor.execute(&request, &mut |_, _| {}).is_err());
    executor
        .cancelled
        .store(true, std::sync::atomic::Ordering::Release);
    let request: FileAction =
        serde_json::from_value(serde_json::json!({"operation":"create_directory","name":"never"}))
            .unwrap();
    assert!(executor.execute(&request, &mut |_, _| {}).is_err());
    assert!(!dir.path().join("never").exists());
}
#[test]
fn docx_roundtrip_contains_real_document_xml() {
    let dir = tempfile::tempdir().unwrap();
    let mut executor = executor(dir.path(), vec![]);
    let request:FileAction=serde_json::from_value(serde_json::json!({"operation":"docx_create","name":"说明.docx","content":"# 项目说明\n\n你好，Fleqi。\n- 第一项\n| 名称 | 值 |\n| --- | --- |\n| 文件 | 1 |"})).unwrap();
    let result = executor.execute(&request, &mut |_, _| {}).unwrap();
    let read: FileAction = serde_json::from_value(
        serde_json::json!({"operation":"docx_read","sources":result.outputs}),
    )
    .unwrap();
    let result = executor.execute(&read, &mut |_, _| {}).unwrap();
    assert!(result.text.unwrap().contains("你好，Fleqi。"));
}

#[test]
fn refuses_mutation_when_the_selected_file_was_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("selected.txt");
    fs::write(&source, "selected original").unwrap();
    let mut executor = executor(dir.path(), vec![source.to_string_lossy().into()]);
    let replacement = dir.path().join("replacement.txt");
    fs::write(&replacement, "new unrelated object").unwrap();
    fs::rename(&replacement, &source).unwrap();
    let action: FileAction = serde_json::from_value(
        serde_json::json!({"operation":"rename","sources":[source],"name":"changed.txt"}),
    )
    .unwrap();
    assert!(executor.execute(&action, &mut |_, _| {}).is_err());
    assert_eq!(fs::read_to_string(&source).unwrap(), "new unrelated object");
    assert!(!dir.path().join("changed.txt").exists());
}

#[cfg(unix)]
#[test]
fn same_volume_directory_rename_preserves_the_tree_and_file_identity() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("Original");
    fs::create_dir(&folder).unwrap();
    let file = folder.join("keep.txt");
    fs::write(&file, "keep contents").unwrap();
    let inode = fs::metadata(&file).unwrap().ino();
    let mut executor = executor(dir.path(), vec![folder.to_string_lossy().into()]);
    let action: FileAction = serde_json::from_value(
        serde_json::json!({"operation":"rename","sources":[folder],"name":"Renamed"}),
    )
    .unwrap();
    let result = executor.execute(&action, &mut |_, _| {}).unwrap();
    assert!(result.verified);
    assert!(!folder.exists());
    assert_eq!(
        fs::metadata(dir.path().join("Renamed/keep.txt"))
            .unwrap()
            .ino(),
        inode
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("Renamed/keep.txt")).unwrap(),
        "keep contents"
    );
}
