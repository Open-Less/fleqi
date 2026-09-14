use crate::{
    platform,
    state::{AppState, Interaction, PendingInteraction},
};
use fleqi_core::{
    files::{FileAction, FileExecutor},
    model::{ModelProfile, TaskRecord, TaskStatus},
};
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::Command,
    sync::{Mutex as AsyncMutex, oneshot},
};

pub fn changed(app: &AppHandle) {
    let _ = app.emit("fleqi:changed", ());
}
fn task_state(
    app: &AppHandle,
    state: &AppState,
    task: &TaskRecord,
    status: TaskStatus,
    message: &str,
    completed: usize,
    total: Option<usize>,
) {
    if let Ok(event) = state
        .0
        .db
        .update_task(&task.id, status, message, completed, total)
    {
        let _ = app.emit("fleqi:task", event);
    }
    changed(app);
}
async fn write(
    writer: &Arc<AsyncMutex<tokio::process::ChildStdin>>,
    value: Value,
) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    writer
        .lock()
        .await
        .write_all(&bytes)
        .await
        .map_err(|e| e.to_string())
}
pub async fn ask(
    app: AppHandle,
    state: AppState,
    kind: String,
    message: String,
    task_id: Option<String>,
    options: Value,
    cancelled: Arc<AtomicBool>,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let (send, receive) = oneshot::channel();
    let info = Interaction {
        id: id.clone(),
        kind,
        message,
        task_id,
        options,
    };
    state
        .0
        .pending
        .lock()
        .map_err(|_| "交互状态不可用")?
        .insert(
            id.clone(),
            PendingInteraction {
                info: info.clone(),
                response: send,
            },
        );
    let _ = app.emit("fleqi:interaction", info);
    changed(&app);
    let cancellation = async {
        while !cancelled.load(Ordering::Acquire) {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    };
    let answer = tokio::select! {answer=receive=>answer.map_err(|_|"交互已关闭".into()),_=cancellation=>Err("操作已取消".into())};
    if let Ok(mut pending) = state.0.pending.lock() {
        pending.remove(&id);
    }
    changed(&app);
    answer
}
fn scrub(mut value: String, secrets: &[String]) -> String {
    for secret in secrets {
        if secret.len() > 3 {
            value = value.replace(secret, "[redacted]");
        }
    }
    value.chars().take(1600).collect()
}
fn collect_secrets(value: &Value, output: &mut Vec<String>) {
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if matches!(
                key.as_str(),
                "key" | "access" | "refresh" | "accessToken" | "refreshToken"
            ) {
                if let Some(secret) = value.as_str() {
                    output.push(secret.into());
                }
            } else if value.is_object() {
                collect_secrets(value, output);
            }
        }
    }
}

pub async fn run(
    app: AppHandle,
    state: AppState,
    mode: &str,
    profile: ModelProfile,
    task: Option<TaskRecord>,
    cancelled: Arc<AtomicBool>,
) -> Result<Value, String> {
    let node = state
        .0
        .resources
        .join("bin")
        .join(if cfg!(windows) { "node.exe" } else { "node" });
    let entry = state.0.pi_dir.join("entry.mjs");
    if !node.is_file() || !entry.is_file() {
        return Err("内置运行时未打包，请先准备 Node 与 PI 资源".into());
    }
    let state_directory = state.0.data_dir.join("runtime").join(&profile.id);
    std::fs::create_dir_all(&state_directory).map_err(|e| e.to_string())?;
    let settings = state.0.db.settings().map_err(|e| e.to_string())?;
    let mut command = Command::new(node);
    command
        .arg(entry)
        .current_dir(&state_directory)
        .env_clear()
        .env("HOME", &state_directory)
        .env("USERPROFILE", &state_directory)
        .env("PI_CODING_AGENT_DIR", &state_directory)
        .env(
            "PATH",
            if cfg!(windows) {
                "C:\\Windows\\System32"
            } else {
                "/usr/bin:/bin"
            },
        )
        .env("LANG", "en_US.UTF-8")
        .env("PI_OFFLINE", "1")
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if cfg!(windows) {
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("无法启动内置 PI：{e}"))?;
    let writer = Arc::new(AsyncMutex::new(
        child.stdin.take().ok_or("运行时输入不可用")?,
    ));
    let stdout = child.stdout.take().ok_or("运行时输出不可用")?;
    if let Some(mut stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut buffer = [0; 8192];
            while stderr.read(&mut buffer).await.is_ok_and(|n| n > 0) {}
        });
    }
    let mut lines = BufReader::new(stdout).lines();
    let output_directory = if mode == "selftest" {
        task.as_ref().map(|t| t.context.directory.clone())
    } else {
        settings.output_directory.clone()
    };
    write(&writer,json!({"type":"start","mode":mode,"profile":profile,"stateDirectory":state_directory,"context":task.as_ref().map(|t|&t.context),"prompt":task.as_ref().map(|t|&t.prompt),"outputDirectory":output_directory})).await?;
    let executor = if let Some(task) = &task {
        Some(Arc::new(Mutex::new(
            FileExecutor::new(
                task.context.clone(),
                if mode == "selftest" {
                    None
                } else {
                    settings.output_directory.clone().map(Into::into)
                },
                state.0.engines.clone(),
                cancelled.clone(),
            )
            .map_err(|e| e.to_string())?,
        )))
    } else {
        None
    };
    let mut selftest_cleaned = false;
    let mut completed = 0;
    let mut failed = false;
    let mut called_tool = false;
    let mut test_tool = false;
    let mut secrets = Vec::new();
    let started = Instant::now();
    let mut prompts = Vec::new();
    let outcome = loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = write(&writer, json!({"type":"cancel"})).await;
            break Err("操作已取消".into());
        }
        let line = tokio::select! {
            result=lines.next_line()=>result.map_err(|e|e.to_string())?,
            _=tokio::time::sleep(std::time::Duration::from_millis(100))=>continue,
        };
        let Some(line) = line else {
            break Err("内置 PI 意外退出，请检查运行时依赖或重新尝试".into());
        };
        if line.len() > 8_000_000 {
            break Err("运行时消息超过允许大小".into());
        }
        let event: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => break Err("内置 PI 通信格式错误".into()),
        };
        let id = event.get("id").cloned().unwrap_or(Value::Null);
        match event["type"].as_str().unwrap_or("") {
            "credential_read" | "credential_write" | "credential_delete" => {
                let operation = match event["type"].as_str() {
                    Some("credential_read") => "read",
                    Some("credential_write") => "write",
                    _ => "delete",
                };
                let value = event.get("credential").cloned();
                if let Some(value) = &value {
                    collect_secrets(value, &mut secrets);
                }
                let profile_id = profile.id.clone();
                let result = tokio::task::spawn_blocking(move || {
                    platform::credential(operation, &profile_id, value)
                })
                .await
                .map_err(|e| e.to_string())?;
                match result {
                    Ok(value) => {
                        collect_secrets(&value, &mut secrets);
                        write(&writer, json!({"id":id,"result":value})).await?;
                    }
                    Err(error) => {
                        write(&writer, json!({"id":id,"error":error.to_string()})).await?
                    }
                }
                changed(&app);
            }
            "auth_event" => {
                let mut info = event["event"].clone();
                let kind = info["type"].as_str().unwrap_or("").to_string();
                match kind.as_str() {
                    "auth_url" => {
                        if let Some(url) = info["url"].as_str() {
                            if crate::auth::open_authorized(&app, url) {
                                if let Some(object) = info.as_object_mut() {
                                    object.insert(
                                        "instructions".into(),
                                        json!("已在新窗口打开授权页面。若没有自动打开，请在窗口中点击“重新打开授权页”。"),
                                    );
                                }
                            }
                        }
                    }
                    "device_code" => {
                        let code = info["userCode"].as_str().unwrap_or("").to_string();
                        if let Some(url) = info["verificationUri"].as_str() {
                            crate::auth::open_authorized(&app, url);
                        }
                        if let Some(object) = info.as_object_mut() {
                            object.insert(
                                "message".into(),
                                json!(format!(
                                    "已在浏览器打开设备码页面，请在其中输入设备码：{code}"
                                )),
                            );
                        }
                    }
                    "progress" => {
                        info["message"] = json!(scrub(
                            crate::auth::progress_copy(info["message"].as_str().unwrap_or("")),
                            &secrets
                        ));
                    }
                    _ => {}
                }
                if matches!(kind.as_str(), "auth_url" | "device_code") {
                    if let Ok(mut stored) = state.0.auth.lock() {
                        *stored = Some(info.clone());
                    }
                }
                let _ = app.emit("fleqi:auth", info);
            }
            "auth_prompt" => {
                let app = app.clone();
                let state = state.clone();
                let writer = writer.clone();
                let cancelled = cancelled.clone();
                let prompt = event["prompt"].clone();
                let (message, options) = crate::auth::prompt_copy(&prompt);
                prompts.push(tokio::spawn(async move {
                    let answer =
                        ask(app, state, "auth".into(), message, None, options, cancelled).await;
                    let response = match answer {
                        Ok(value) => json!({"id":id,"result":value}),
                        Err(error) => json!({"id":id,"error":error}),
                    };
                    let _ = write(&writer, response).await;
                }));
            }
            "question" => {
                if let Some(task) = &task {
                    let question = event["question"]
                        .as_str()
                        .unwrap_or("请补充操作要求")
                        .chars()
                        .take(500)
                        .collect::<String>();
                    task_state(
                        &app,
                        &state,
                        task,
                        TaskStatus::AwaitingInput,
                        &question,
                        completed,
                        None,
                    );
                    let answer = ask(
                        app.clone(),
                        state.clone(),
                        "question".into(),
                        question,
                        Some(task.id.clone()),
                        Value::Null,
                        cancelled.clone(),
                    )
                    .await;
                    match answer {
                        Ok(value) => write(&writer, json!({"id":id,"result":value})).await?,
                        Err(error) => {
                            write(&writer, json!({"id":id,"error":error})).await?;
                            break Err("操作已取消".into());
                        }
                    }
                    task_state(
                        &app,
                        &state,
                        task,
                        TaskStatus::WaitingModel,
                        "正在处理补充信息",
                        completed,
                        None,
                    );
                }
            }
            "tool" => {
                let Some(task) = &task else {
                    break Err("当前运行模式不允许文件操作".into());
                };
                let action: FileAction = match serde_json::from_value(event["action"].clone()) {
                    Ok(value) => value,
                    Err(_) => {
                        write(&writer, json!({"id":id,"error":"文件工具参数无效"})).await?;
                        failed = true;
                        continue;
                    }
                };
                let call_id = event["callId"]
                    .as_str()
                    .ok_or("文件动作缺少编号")?
                    .to_string();
                if (action.needs_confirmation() && mode != "selftest")
                    || (settings.conflict_policy == "ask"
                        && mode != "selftest"
                        && !matches!(
                            action.operation,
                            fleqi_core::files::Operation::Inspect
                                | fleqi_core::files::Operation::List
                                | fleqi_core::files::Operation::ZipList
                        ))
                {
                    let question = action.confirmation();
                    task_state(
                        &app,
                        &state,
                        task,
                        TaskStatus::AwaitingConfirmation,
                        &question,
                        completed,
                        None,
                    );
                    let answer = ask(
                        app.clone(),
                        state.clone(),
                        "confirmation".into(),
                        question,
                        Some(task.id.clone()),
                        json!([{"id":"confirm","label":"确认"},{"id":"cancel","label":"取消任务"}]),
                        cancelled.clone(),
                    )
                    .await?;
                    if answer != "confirm" {
                        cancelled.store(true, Ordering::Release);
                        break Err("操作已取消".into());
                    }
                }
                let mut arguments = event["action"].clone();
                if let Some(object) = arguments.as_object_mut() {
                    if let Some(content) = object.remove("content") {
                        object.insert(
                            "contentBytes".into(),
                            json!(content.as_str().map(str::len).unwrap_or(0)),
                        );
                    }
                }
                if !state
                    .0
                    .db
                    .record_action(
                        &task.id,
                        &call_id,
                        event["action"]["operation"].as_str().unwrap_or("file"),
                        &arguments,
                    )
                    .map_err(|e| e.to_string())?
                {
                    let prior = state
                        .0
                        .db
                        .task(&task.id)
                        .map_err(|e| e.to_string())?
                        .actions
                        .into_iter()
                        .find(|a| a.id == call_id);
                    if let Some(prior) = prior.filter(|a| a.status == "completed") {
                        write(&writer, json!({"id":id,"result":prior.result})).await?;
                        continue;
                    }
                    break Err("此动作结果不确定，已停止以避免重复执行".into());
                }
                called_tool = true;
                task_state(
                    &app,
                    &state,
                    task,
                    TaskStatus::Running,
                    "正在执行文件操作",
                    completed,
                    None,
                );
                let engine = executor.as_ref().ok_or("文件执行器未就绪")?.clone();
                let task_copy = task.clone();
                let app_copy = app.clone();
                let state_copy = state.clone();
                let prior_completed = completed;
                let result = tokio::task::spawn_blocking(move || {
                    let mut engine = engine.lock().map_err(|_| "文件执行器不可用".to_string())?;
                    engine
                        .execute(&action, &mut |count, total| {
                            task_state(
                                &app_copy,
                                &state_copy,
                                &task_copy,
                                TaskStatus::Running,
                                "正在处理文件",
                                prior_completed + count,
                                total.map(|n| prior_completed + n),
                            )
                        })
                        .map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())?;
                match result {
                    Ok(mut result) => {
                        let operation = event["action"]["operation"].as_str().unwrap_or("");
                        if ["open", "reveal", "copy_path"].contains(&operation) {
                            let paths = result
                                .details
                                .as_array()
                                .map(|items| {
                                    items
                                        .iter()
                                        .filter_map(|item| item["path"].as_str().map(str::to_owned))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            if operation == "copy_path" {
                                platform::host_call(
                                    json!({"operation":"copy_text","text":paths.join("\n")}),
                                )
                                .map_err(|e| e.to_string())?;
                            } else {
                                result.completed = 0;
                                for path in paths {
                                    if cancelled.load(Ordering::Acquire) {
                                        break;
                                    }
                                    let accepted = if operation == "open" {
                                        app.opener().open_path(path, None::<&str>)
                                    } else {
                                        app.opener().reveal_item_in_dir(path)
                                    };
                                    if let Err(error) = accepted {
                                        result.verified = false;
                                        result.details = json!({"message":error.to_string()});
                                        break;
                                    }
                                    result.completed += 1;
                                }
                            }
                        }
                        completed += result.completed;
                        failed |= !result.verified;
                        task_state(
                            &app,
                            &state,
                            task,
                            TaskStatus::Verifying,
                            "正在核对输出结果",
                            completed,
                            None,
                        );
                        let mut ledger =
                            serde_json::to_value(&result).map_err(|e| e.to_string())?;
                        if matches!(
                            event["action"]["operation"].as_str(),
                            Some("read_text" | "docx_read")
                        ) {
                            ledger["text"] = Value::Null;
                        }
                        state
                            .0
                            .db
                            .finish_action(
                                &task.id,
                                &call_id,
                                if result.verified {
                                    "completed"
                                } else {
                                    "partial"
                                },
                                ledger,
                            )
                            .map_err(|e| e.to_string())?;
                        write(&writer, json!({"id":id,"result":result})).await?;
                    }
                    Err(error) => {
                        failed = true;
                        let outputs = executor
                            .as_ref()
                            .and_then(|e| e.lock().ok().map(|e| e.produced_outputs()))
                            .unwrap_or_default();
                        let result = json!({"verified":false,"message":error,"outputs":outputs});
                        state
                            .0
                            .db
                            .finish_action(&task.id, &call_id, "failed", result.clone())
                            .map_err(|e| e.to_string())?;
                        write(&writer, json!({"id":id,"result":result})).await?;
                    }
                }
                task_state(
                    &app,
                    &state,
                    task,
                    TaskStatus::WaitingModel,
                    "正在整理结果",
                    completed,
                    None,
                );
            }
            "test_tool" => test_tool |= event["ok"] == true,
            "selftest_cleanup" => {
                if mode != "selftest" {
                    break Err("此任务不允许自检清理".into());
                }
                let task = task.as_ref().ok_or("没有自检任务")?;
                let ledger = state.0.db.task(&task.id).map_err(|e| e.to_string())?;
                let verified = ["write_text", "read_text", "rename"].iter().all(|op| {
                    ledger
                        .actions
                        .iter()
                        .any(|a| a.operation == *op && a.status == "completed")
                });
                if !verified {
                    write(
                        &writer,
                        json!({"id":id,"error":"请先完成新建、读取和改名三步"}),
                    )
                    .await?;
                    continue;
                }
                // Only selftest_start supplies this directory; its TempDir owner
                // stays alive in the task supervisor. No user-provided path is used.
                let directory = task.context.directory.clone();
                tokio::task::spawn_blocking(move || std::fs::remove_dir_all(directory))
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?;
                selftest_cleaned = true;
                write(
                    &writer,
                    json!({"id":id,"result":{"verified":true,"cleaned":true}}),
                )
                .await?;
            }
            "provider_error" => {
                break Err(scrub(
                    event["message"].as_str().unwrap_or("模型响应失败").into(),
                    &secrets,
                ));
            }
            "done" => {
                if mode == "test" && !test_tool {
                    break Err("模型回复成功，但没有通过结构化工具调用检查".into());
                }
                if mode == "selftest" && !selftest_cleaned {
                    break Err("PI 未完成全部文件自检步骤，请重试并查看任务结果".into());
                }
                let message = scrub(
                    event["result"]["message"].as_str().unwrap_or("").into(),
                    &secrets,
                );
                if let Some(task) = &task {
                    let status = if !called_tool {
                        TaskStatus::Failed
                    } else if failed {
                        TaskStatus::Partial
                    } else {
                        TaskStatus::Completed
                    };
                    let message = if !called_tool {
                        format!("未执行文件操作。{message}")
                    } else if failed {
                        format!("部分操作未完成或未达到目标。{message}")
                    } else {
                        message.clone()
                    };
                    task_state(
                        &app,
                        &state,
                        task,
                        status,
                        &message,
                        completed,
                        Some(completed),
                    );
                }
                break Ok(
                    json!({"ok":true,"elapsedMs":started.elapsed().as_millis(),"message":message,"result":event["result"]}),
                );
            }
            "error" => {
                break Err(scrub(
                    event["message"]
                        .as_str()
                        .unwrap_or("内置 PI 执行失败")
                        .into(),
                    &secrets,
                ));
            }
            _ => {}
        }
    };
    for prompt in prompts {
        prompt.abort();
    }
    if let Ok(mut pending) = state.0.pending.lock() {
        pending.retain(|_, p| {
            p.info.task_id.is_some()
                && p.info.task_id.as_deref() != task.as_ref().map(|t| t.id.as_str())
        });
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
    if let (Some(task), Err(error)) = (&task, &outcome) {
        let outputs = executor
            .as_ref()
            .and_then(|e| e.lock().ok().map(|e| e.produced_outputs()))
            .unwrap_or_default();
        let status = if cancelled.load(Ordering::Acquire) {
            TaskStatus::Cancelled
        } else if completed > 0 || !outputs.is_empty() {
            TaskStatus::Partial
        } else {
            TaskStatus::Failed
        };
        task_state(
            &app,
            &state,
            task,
            status,
            &format!(
                "{error}{}",
                if completed > 0 || !outputs.is_empty() {
                    "；已完成的文件保留在任务结果中"
                } else {
                    ""
                }
            ),
            completed,
            None,
        );
    }
    changed(&app);
    outcome
}
