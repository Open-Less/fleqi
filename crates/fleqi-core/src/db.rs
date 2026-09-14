use crate::{Error, Result, model::*, now_ms};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Mutex};

pub struct Database {
    connection: Mutex<Connection>,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        Self::initialize(Connection::open(path)?)
    }
    pub fn memory() -> Result<Self> {
        Self::initialize(Connection::open_in_memory()?)
    }
    fn initialize(connection: Connection) -> Result<Self> {
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS config (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS profiles (id TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS tasks (id TEXT PRIMARY KEY, request_id TEXT UNIQUE NOT NULL, status TEXT NOT NULL, updated_at INTEGER NOT NULL, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS actions (task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE, id TEXT NOT NULL, value TEXT NOT NULL, PRIMARY KEY(task_id,id));
            PRAGMA user_version=1;")?;
        connection.execute(
            "INSERT OR IGNORE INTO config VALUES ('settings',?1)",
            [serde_json::to_string(&SettingsSnapshot::default())?],
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| Error::Invalid("本地数据存储暂不可用".into()))
    }
    pub fn settings(&self) -> Result<SettingsSnapshot> {
        let text: String =
            self.lock()?
                .query_row("SELECT value FROM config WHERE key='settings'", [], |r| {
                    r.get(0)
                })?;
        Ok(serde_json::from_str(&text)?)
    }
    pub fn save_settings(&self, mut settings: SettingsSnapshot) -> Result<SettingsSnapshot> {
        settings.validate()?;
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let old: String =
            tx.query_row("SELECT value FROM config WHERE key='settings'", [], |r| {
                r.get(0)
            })?;
        let current: SettingsSnapshot = serde_json::from_str(&old)?;
        if current.revision != settings.revision {
            return Err(Error::Invalid("设置已在其他窗口更新，请刷新后重试".into()));
        }
        settings.revision += 1;
        tx.execute(
            "UPDATE config SET value=?1 WHERE key='settings'",
            [serde_json::to_string(&settings)?],
        )?;
        tx.commit()?;
        Ok(settings)
    }
    pub fn import_appearance(&self, appearance: Appearance) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        if tx
            .query_row(
                "SELECT value FROM config WHERE key='appearance_imported'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .is_none()
        {
            let raw: String =
                tx.query_row("SELECT value FROM config WHERE key='settings'", [], |r| {
                    r.get(0)
                })?;
            let mut settings: SettingsSnapshot = serde_json::from_str(&raw)?;
            if settings.revision == 0 {
                settings.appearance = appearance;
                settings.validate()?;
                tx.execute(
                    "UPDATE config SET value=?1 WHERE key='settings'",
                    [serde_json::to_string(&settings)?],
                )?;
            }
            tx.execute(
                "INSERT INTO config VALUES ('appearance_imported','true')",
                [],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn profiles(&self) -> Result<Vec<ModelProfile>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare("SELECT value FROM profiles ORDER BY rowid")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .map(|v| Ok(serde_json::from_str(&v?)?))
            .collect()
    }
    pub fn profile(&self, id: &str) -> Result<ModelProfile> {
        let raw: String =
            self.lock()?
                .query_row("SELECT value FROM profiles WHERE id=?1", [id], |r| r.get(0))?;
        Ok(serde_json::from_str(&raw)?)
    }
    pub fn save_profile(&self, profile: &ModelProfile) -> Result<()> {
        profile.validate()?;
        self.lock()?.execute("INSERT INTO profiles VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value", params![profile.id, serde_json::to_string(profile)?])?;
        Ok(())
    }
    pub fn delete_profile(&self, id: &str) -> Result<()> {
        self.lock()?
            .execute("DELETE FROM profiles WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn create_task(&self, request: &TaskRequest) -> Result<TaskRecord> {
        if request.request_id.is_empty()
            || request.request_id.len() > 100
            || request.prompt.trim().is_empty()
            || request.prompt.len() > 16_000
        {
            return Err(Error::Invalid("任务内容无效".into()));
        }
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        if let Some(raw) = tx
            .query_row(
                "SELECT value FROM tasks WHERE request_id=?1",
                [&request.request_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(serde_json::from_str(&raw)?);
        }
        let count: i64 = tx.query_row("SELECT COUNT(*) FROM tasks WHERE status NOT IN ('completed','partial','failed','cancelled','needs_review')", [], |r| r.get(0))?;
        if count != 0 {
            return Err(Error::Busy);
        }
        let task = TaskRecord {
            id: uuid::Uuid::new_v4().to_string(),
            request_id: request.request_id.clone(),
            prompt: request.prompt.clone(),
            context: request.context.clone(),
            profile: request.profile.clone(),
            status: TaskStatus::Checking,
            message: "正在检查文件对象".into(),
            created_at: now_ms(),
            updated_at: now_ms(),
            sequence: 0,
            completed: 0,
            total: None,
            actions: vec![],
        };
        tx.execute(
            "INSERT INTO tasks VALUES (?1,?2,?3,?4,?5)",
            params![
                task.id,
                task.request_id,
                task.status.key(),
                task.updated_at as i64,
                serde_json::to_string(&task)?
            ],
        )?;
        tx.commit()?;
        Ok(task)
    }
    pub fn task(&self, id: &str) -> Result<TaskRecord> {
        let conn = self.lock()?;
        let raw: String =
            conn.query_row("SELECT value FROM tasks WHERE id=?1", [id], |r| r.get(0))?;
        let mut task: TaskRecord = serde_json::from_str(&raw)?;
        let mut stmt = conn.prepare("SELECT value FROM actions WHERE task_id=?1 ORDER BY rowid")?;
        task.actions = stmt
            .query_map([id], |r| r.get::<_, String>(0))?
            .map(|v| Ok(serde_json::from_str(&v?)?))
            .collect::<Result<_>>()?;
        Ok(task)
    }
    pub fn task_by_request(&self, request_id: &str) -> Result<Option<TaskRecord>> {
        let id = self
            .lock()?
            .query_row(
                "SELECT id FROM tasks WHERE request_id=?1",
                [request_id],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        id.map(|id| self.task(&id)).transpose()
    }
    pub fn tasks(&self, limit: u32) -> Result<Vec<TaskRecord>> {
        let ids = {
            let conn = self.lock()?;
            let mut stmt =
                conn.prepare("SELECT id FROM tasks ORDER BY updated_at DESC LIMIT ?1")?;
            stmt.query_map([limit.min(200)], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        ids.iter().map(|id| self.task(id)).collect()
    }
    pub fn update_task(
        &self,
        id: &str,
        status: TaskStatus,
        message: &str,
        completed: usize,
        total: Option<usize>,
    ) -> Result<TaskRecord> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let raw: String =
            tx.query_row("SELECT value FROM tasks WHERE id=?1", [id], |r| r.get(0))?;
        let mut task: TaskRecord = serde_json::from_str(&raw)?;
        if task.status.is_terminal() {
            return Ok(task);
        }
        task.status = status;
        task.message = message.chars().take(2000).collect();
        task.completed = completed;
        task.total = total;
        task.updated_at = now_ms();
        task.sequence += 1;
        tx.execute(
            "UPDATE tasks SET status=?2,updated_at=?3,value=?4 WHERE id=?1",
            params![
                id,
                status.key(),
                task.updated_at as i64,
                serde_json::to_string(&task)?
            ],
        )?;
        tx.commit()?;
        Ok(task)
    }
    pub fn record_action(
        &self,
        task: &str,
        id: &str,
        operation: &str,
        args: &serde_json::Value,
    ) -> Result<bool> {
        let record = ActionRecord {
            id: id.into(),
            operation: operation.into(),
            status: "running".into(),
            arguments: args.clone(),
            result: None,
        };
        Ok(self.lock()?.execute(
            "INSERT OR IGNORE INTO actions VALUES (?1,?2,?3)",
            params![task, id, serde_json::to_string(&record)?],
        )? == 1)
    }
    pub fn finish_action(
        &self,
        task: &str,
        id: &str,
        status: &str,
        result: serde_json::Value,
    ) -> Result<()> {
        let conn = self.lock()?;
        let raw: String = conn.query_row(
            "SELECT value FROM actions WHERE task_id=?1 AND id=?2",
            params![task, id],
            |r| r.get(0),
        )?;
        let mut record: ActionRecord = serde_json::from_str(&raw)?;
        record.status = status.into();
        record.result = Some(result);
        conn.execute(
            "UPDATE actions SET value=?3 WHERE task_id=?1 AND id=?2",
            params![task, id, serde_json::to_string(&record)?],
        )?;
        Ok(())
    }
    pub fn recover_interrupted(&self) -> Result<()> {
        for task in self.tasks(200)? {
            if !task.status.is_terminal() {
                for action in &task.actions {
                    if action.status == "running" {
                        self.finish_action(
                            &task.id,
                            &action.id,
                            "needs_review",
                            serde_json::json!({"message":"应用中断，结果需要核对"}),
                        )?;
                    }
                }
                self.update_task(
                    &task.id,
                    TaskStatus::NeedsReview,
                    "上次运行中断，请核对已有产物；不会自动重跑",
                    task.completed,
                    task.total,
                )?;
            }
        }
        Ok(())
    }
    pub fn clear_history(&self) -> Result<()> {
        self.lock()?.execute("DELETE FROM tasks WHERE status IN ('completed','partial','failed','cancelled','needs_review')", [])?;
        Ok(())
    }
}
