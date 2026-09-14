use fleqi_core::{db::Database, model::*};

fn request(id: &str) -> TaskRequest {
    TaskRequest {
        request_id: id.into(),
        prompt: "复制选中的文件".into(),
        context: ContextSnapshot {
            id: "snapshot".into(),
            host_window: "finder:1".into(),
            directory: "/tmp".into(),
            files: vec!["/tmp/原件.txt".into()],
            captured_at: 1,
        },
        profile: ModelProfile {
            id: "test".into(),
            name: "Local".into(),
            protocol: Protocol::OpenaiCompletions,
            base_url: "http://127.0.0.1:1234/v1".into(),
            model_id: "test".into(),
            thinking: "off".into(),
            auth_type: AuthType::ApiKey,
            has_credential: false,
            models: vec![],
            models_fetched_at: None,
            service_tier: None,
        },
    }
}

#[test]
fn deduplicates_requests_and_does_not_replace_frozen_context() {
    let db = Database::memory().unwrap();
    let first = db.create_task(&request("same")).unwrap();
    let mut duplicate = request("same");
    duplicate.context.files = vec!["/tmp/另一个.txt".into()];
    let repeated = db.create_task(&duplicate).unwrap();
    assert_eq!(first.id, repeated.id);
    assert_eq!(first.context.files, repeated.context.files);
    assert!(db.create_task(&request("second")).is_err());
}

#[test]
fn restart_marks_interrupted_work_for_review_without_replaying() {
    let db = Database::memory().unwrap();
    let task = db.create_task(&request("work")).unwrap();
    db.record_action(
        &task.id,
        "action",
        "copy",
        &serde_json::json!({"source": "file"}),
    )
    .unwrap();
    db.recover_interrupted().unwrap();
    let restored = db.task(&task.id).unwrap();
    assert_eq!(restored.status, TaskStatus::NeedsReview);
    assert_eq!(restored.actions[0].status, "needs_review");
    assert!(db.create_task(&request("new")).is_ok());
}

#[test]
fn imports_appearance_only_once_and_settings_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fleqi.sqlite");
    {
        let db = Database::open(&path).unwrap();
        db.import_appearance(Appearance {
            theme: "dark".into(),
            material: "liquid".into(),
        })
        .unwrap();
        db.import_appearance(Appearance::default()).unwrap();
    }
    let db = Database::open(&path).unwrap();
    assert_eq!(db.settings().unwrap().appearance.theme, "dark");
    assert!(!db.settings().unwrap().launch_at_login);
}
