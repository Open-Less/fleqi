//! File operations are scoped to a captured selection and verified before completion.
use crate::{Error, Result, model::ContextSnapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

mod documents;
mod engines;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnginePaths {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub qpdf: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Inspect,
    List,
    CreateDirectory,
    WriteText,
    ReadText,
    Copy,
    Move,
    Rename,
    Trash,
    ZipCreate,
    ZipList,
    ZipExtract,
    ImageConvert,
    MediaConvert,
    MediaTrim,
    MediaExtractAudio,
    PdfMerge,
    PdfSplit,
    PdfExtract,
    PdfRotate,
    PdfCompress,
    DocxCreate,
    DocxRead,
    Open,
    Reveal,
    CopyPath,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileAction {
    pub operation: Operation,
    #[serde(default)]
    pub sources: Vec<PathBuf>,
    pub destination: Option<PathBuf>,
    pub name: Option<String>,
    pub format: Option<String>,
    pub content: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub find: Option<String>,
    pub replace: Option<String>,
    pub numbering: Option<u32>,
    pub letter_case: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub quality: Option<u8>,
    pub rotation: Option<u16>,
    pub pages: Option<String>,
    pub start: Option<f64>,
    pub duration: Option<f64>,
    pub bitrate: Option<u32>,
    pub max_bytes: Option<u64>,
    #[serde(default)]
    pub recursive: bool,
}
impl FileAction {
    fn validate(&self) -> Result<()> {
        use Operation::*;
        let allowed: &[&str] = match self.operation {
            Rename => &[
                "prefix",
                "suffix",
                "find",
                "replace",
                "numbering",
                "letterCase",
            ],
            WriteText | DocxCreate => &["content"],
            ImageConvert => &["format", "width", "height", "quality", "rotation"],
            MediaConvert | MediaTrim | MediaExtractAudio => {
                &["format", "width", "height", "start", "duration", "bitrate"]
            }
            PdfExtract | PdfSplit => &["pages"],
            PdfRotate => &["pages", "rotation"],
            _ => &[],
        };
        for (name, present) in [
            ("prefix", self.prefix.is_some()),
            ("suffix", self.suffix.is_some()),
            ("find", self.find.is_some()),
            ("replace", self.replace.is_some()),
            ("numbering", self.numbering.is_some()),
            ("letterCase", self.letter_case.is_some()),
            ("content", self.content.is_some()),
            ("format", self.format.is_some()),
            ("width", self.width.is_some()),
            ("height", self.height.is_some()),
            ("quality", self.quality.is_some()),
            ("rotation", self.rotation.is_some()),
            ("pages", self.pages.is_some()),
            ("start", self.start.is_some()),
            ("duration", self.duration.is_some()),
            ("bitrate", self.bitrate.is_some()),
        ] {
            if present && !allowed.contains(&name) {
                return Err(Error::Invalid(format!(
                    "此操作不支持参数 {name}，请使用对应工具操作"
                )));
            }
        }
        if self.quality.is_some_and(|v| v == 0 || v > 100) {
            return Err(Error::Invalid("图片质量应在 1–100 之间".into()));
        }
        Ok(())
    }
    pub fn needs_confirmation(&self) -> bool {
        matches!(
            self.operation,
            Operation::Trash | Operation::ReadText | Operation::DocxRead
        ) || self.recursive
            || (matches!(self.operation, Operation::Move | Operation::Rename)
                && self.sources.len() >= 100)
    }
    pub fn confirmation(&self) -> String {
        if matches!(self.operation, Operation::ReadText | Operation::DocxRead) {
            format!(
                "读取 {} 个文件的正文并交给本次模型服务处理？",
                self.sources.len()
            )
        } else if matches!(self.operation, Operation::Trash) {
            format!("将 {} 个选中对象移入回收站？", self.sources.len())
        } else {
            format!(
                "将处理 {} 个对象{}，是否继续？",
                self.sources.len(),
                if self.recursive {
                    "及其子目录"
                } else {
                    ""
                }
            )
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub outputs: Vec<PathBuf>,
    pub completed: usize,
    pub verified: bool,
    pub text: Option<String>,
    pub details: serde_json::Value,
}

pub struct FileExecutor {
    context: ContextSnapshot,
    selected: Vec<PathBuf>,
    outputs: HashSet<PathBuf>,
    destination_roots: Vec<PathBuf>,
    output_directory: PathBuf,
    pub engines: EnginePaths,
    pub cancelled: Arc<AtomicBool>,
    identities: HashMap<PathBuf, FileIdentity>,
}
#[derive(PartialEq, Eq)]
struct FileIdentity {
    length: u64,
    modified: Option<std::time::SystemTime>,
    directory: bool,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}
impl FileIdentity {
    fn read(path: &Path) -> Result<Self> {
        let meta = fs::metadata(path)?;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Ok(Self {
            length: if meta.is_dir() { 0 } else { meta.len() },
            modified: if meta.is_dir() {
                None
            } else {
                meta.modified().ok()
            },
            directory: meta.is_dir(),
            #[cfg(unix)]
            device: meta.dev(),
            #[cfg(unix)]
            inode: meta.ino(),
        })
    }
}
impl FileExecutor {
    pub fn produced_outputs(&self) -> Vec<PathBuf> {
        self.outputs
            .iter()
            .filter(|p| p.exists())
            .cloned()
            .collect()
    }
    pub fn new(
        context: ContextSnapshot,
        output_directory: Option<PathBuf>,
        engines: EnginePaths,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self> {
        let directory = fs::canonicalize(&context.directory)?;
        let mut selected = Vec::new();
        let mut roots = vec![directory.clone()];
        for path in &context.files {
            let path = PathBuf::from(path);
            reject_symlinks(&path)?;
            let path = fs::canonicalize(path)?;
            if path.is_dir() {
                roots.push(path.clone());
            } else if let Some(parent) = path.parent() {
                roots.push(parent.to_path_buf());
            }
            selected.push(path);
        }
        let output_directory = fs::canonicalize(output_directory.unwrap_or(directory))?;
        roots.push(output_directory.clone());
        let identities = selected
            .iter()
            .map(|p| Ok((p.clone(), FileIdentity::read(p)?)))
            .collect::<Result<_>>()?;
        Ok(Self {
            context,
            selected,
            outputs: HashSet::new(),
            destination_roots: roots,
            output_directory,
            engines,
            cancelled,
            identities,
        })
    }
    fn check_cancelled(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    fn source(&self, path: &Path) -> Result<PathBuf> {
        reject_symlinks(path)?;
        let real = fs::canonicalize(path)?;
        if self.identities.get(&real).is_some_and(|identity| {
            FileIdentity::read(&real).is_ok_and(|current| current != *identity)
        }) {
            return Err(Error::Invalid(
                "文件已被其他操作修改，请重新选择并提交".into(),
            ));
        }
        if self
            .selected
            .iter()
            .any(|root| real == *root || (root.is_dir() && real.starts_with(root)))
            || self
                .outputs
                .iter()
                .any(|root| real == *root || (root.is_dir() && real.starts_with(root)))
        {
            Ok(real)
        } else {
            Err(Error::Invalid(
                "文件不在提交时固定的选区中，请重新选择后提交".into(),
            ))
        }
    }
    fn destination(&self, path: &Path) -> Result<PathBuf> {
        if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(Error::Invalid("目标位置必须是范围内的绝对路径".into()));
        }
        reject_symlinks(path)?;
        let mut existing = path;
        while !existing.exists() {
            existing = existing
                .parent()
                .ok_or_else(|| Error::Invalid("目标位置无效".into()))?;
        }
        let suffix = path
            .strip_prefix(existing)
            .map_err(|_| Error::Invalid("目标位置无效".into()))?;
        let real = if suffix.as_os_str().is_empty() {
            fs::canonicalize(existing)?
        } else {
            fs::canonicalize(existing)?.join(suffix)
        };
        if self
            .destination_roots
            .iter()
            .any(|root| real.starts_with(root))
        {
            Ok(real)
        } else {
            Err(Error::Invalid(
                "目标位置超出本次任务范围；请先在设置中选择输出目录".into(),
            ))
        }
    }
    fn output_folder(&self, action: &FileAction) -> Result<PathBuf> {
        let path = self.destination(
            action
                .destination
                .as_deref()
                .unwrap_or(&self.output_directory),
        )?;
        if !path.is_dir() {
            return Err(Error::Invalid("请先创建目标文件夹".into()));
        }
        Ok(path)
    }
    fn add_output(&mut self, path: PathBuf) -> PathBuf {
        if let Ok(identity) = FileIdentity::read(&path) {
            self.identities.insert(path.clone(), identity);
        }
        self.outputs.insert(path.clone());
        path
    }
    pub fn execute(
        &mut self,
        action: &FileAction,
        progress: &mut dyn FnMut(usize, Option<usize>),
    ) -> Result<FileResult> {
        self.check_cancelled()?;
        action.validate()?;
        if action.sources.len() > 10_000 {
            return Err(Error::Invalid("单次最多处理 10000 个对象".into()));
        }
        let mut seen = HashSet::new();
        let sources = action
            .sources
            .iter()
            .map(|p| self.source(p))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(|p| seen.insert(p.clone()))
            .collect::<Vec<_>>();
        use Operation::*;
        if sources.is_empty()
            && !matches!(
                action.operation,
                CreateDirectory | WriteText | DocxCreate | List
            )
        {
            return Err(Error::Invalid("请先选择需要处理的文件".into()));
        }
        let mut result = FileResult {
            verified: true,
            ..FileResult::default()
        };
        match action.operation {
            Inspect | Open | Reveal | CopyPath => {
                if matches!(action.operation, Open)
                    && sources.iter().any(|p| {
                        (!p.is_dir()
                            && !matches!(
                                p.extension()
                                    .and_then(|e| e.to_str())
                                    .map(str::to_ascii_lowercase)
                                    .as_deref(),
                                Some(
                                    "txt"
                                        | "md"
                                        | "pdf"
                                        | "docx"
                                        | "png"
                                        | "jpg"
                                        | "jpeg"
                                        | "webp"
                                        | "mp3"
                                        | "m4a"
                                        | "wav"
                                        | "mp4"
                                        | "mov"
                                        | "zip"
                                )
                            ))
                            || p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app"))
                    })
                {
                    return Err(Error::Invalid(
                        "底部仅打开数据文件与普通目录；可执行文件或脚本请在 Finder 中手动处理"
                            .into(),
                    ));
                }
                result.details = serde_json::Value::Array(
                    sources.iter().map(|p| info(p)).collect::<Result<_>>()?,
                );
                result.completed = sources.len();
                if !matches!(action.operation, Inspect) {
                    result.text = Some(
                        sources
                            .iter()
                            .map(|p| p.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    );
                }
            }
            List => {
                let root = if let Some(root) = sources.first() {
                    root.clone()
                } else {
                    PathBuf::from(&self.context.directory)
                };
                if !root.is_dir() {
                    return Err(Error::Invalid("请选择一个文件夹".into()));
                }
                let entries = walkdir::WalkDir::new(root)
                    .follow_links(false)
                    .min_depth(1)
                    .max_depth(if action.recursive { 32 } else { 1 });
                let mut values = Vec::new();
                for entry in entries.into_iter().take(10_001) {
                    self.check_cancelled()?;
                    let entry = entry.map_err(invalid)?;
                    values.push(info(entry.path())?);
                }
                if values.len() > 10_000 {
                    return Err(Error::Invalid("目录项目过多，请缩小范围".into()));
                }
                result.completed = values.len();
                result.details = serde_json::Value::Array(values);
            }
            CreateDirectory => {
                let folder = self.output_folder(action)?;
                let path = unique_path(
                    &folder.join(safe_name(action.name.as_deref().unwrap_or("新建文件夹"))?),
                );
                fs::create_dir(&path)?;
                result.outputs.push(self.add_output(path));
                result.completed = 1;
            }
            WriteText | DocxCreate => {
                let folder = self.output_folder(action)?;
                let name = safe_name(action.name.as_deref().unwrap_or(
                    if matches!(action.operation, DocxCreate) {
                        "说明.docx"
                    } else {
                        "说明.md"
                    },
                ))?;
                let path = unique_path(&folder.join(name));
                let content = action
                    .content
                    .as_deref()
                    .ok_or_else(|| Error::Invalid("缺少文件内容".into()))?;
                if content.len() > 2_000_000 {
                    return Err(Error::Invalid("单个文档内容过大".into()));
                }
                let mut temp = TemporaryOutput::new(&path)?;
                if matches!(action.operation, DocxCreate) {
                    documents::write_docx(&temp.path, content)?;
                    documents::read_docx(&temp.path)?;
                } else {
                    fs::write(&temp.path, content)?;
                    if fs::read_to_string(&temp.path)? != content {
                        return Err(Error::Invalid("文本核验失败".into()));
                    }
                }
                self.check_cancelled()?;
                temp.commit(&path)?;
                result.outputs.push(self.add_output(path));
                result.completed = 1;
            }
            ReadText | DocxRead => {
                let mut texts = Vec::new();
                for path in sources {
                    self.check_cancelled()?;
                    let content = if matches!(action.operation, DocxRead) {
                        documents::read_docx(&path)?
                    } else {
                        let meta = fs::metadata(&path)?;
                        if meta.len() > 2_000_000 {
                            return Err(Error::Invalid("文本超过 2 MB，请缩小范围".into()));
                        }
                        fs::read_to_string(&path)?
                    };
                    texts.push(format!(
                        "{}\n{}",
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        content
                    ));
                    result.completed += 1;
                }
                let text = texts.join("\n\n");
                if text.len() > 4_000_000 {
                    return Err(Error::Invalid("正文总量超过 4 MB，请分批处理".into()));
                }
                result.text = Some(text);
            }
            Copy | Move | Rename | Trash => {
                let folder = self.output_folder(action)?;
                for (index, source) in sources.iter().enumerate() {
                    self.check_cancelled()?;
                    self.source(source)?;
                    if matches!(action.operation, Trash) {
                        trash::delete(source).map_err(invalid)?;
                        if source.exists() {
                            return Err(Error::Invalid("回收站操作尚未完成".into()));
                        }
                    } else {
                        let base = source
                            .file_name()
                            .ok_or_else(|| Error::Invalid("不能处理磁盘根目录".into()))?
                            .to_string_lossy();
                        let target = if matches!(action.operation, Rename) {
                            source
                                .parent()
                                .unwrap_or(&folder)
                                .join(rename_name(&base, action, index)?)
                        } else {
                            folder.join(if sources.len() == 1 {
                                action.name.as_deref().unwrap_or(&base)
                            } else {
                                &base
                            })
                        };
                        let target = self.destination(&target)?;
                        safe_name(
                            target
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .as_ref(),
                        )?;
                        if target == *source && matches!(action.operation, Rename) {
                            result.outputs.push(source.clone());
                            result.completed += 1;
                            continue;
                        }
                        let target = unique_path(&target);
                        if matches!(action.operation, Move | Rename)
                            && move_exclusive(source, &target)?
                        {
                            self.selected.retain(|p| p != source);
                            self.identities.remove(source);
                            result.outputs.push(self.add_output(target));
                            result.completed += 1;
                            progress(result.completed, Some(sources.len()));
                            continue;
                        }
                        let identity = FileIdentity::read(source)?;
                        let copied_tree = if source.is_dir() {
                            if !action.recursive {
                                return Err(Error::Invalid("文件夹操作需要明确包含子目录".into()));
                            }
                            if target.starts_with(source) {
                                return Err(Error::Invalid("不能将文件夹放入自身".into()));
                            }
                            self.outputs.insert(target.clone());
                            Some(copy_directory(source, &target, &self.cancelled)?)
                        } else {
                            copy_file(source, &target, &self.cancelled)?;
                            None
                        };
                        // The verified copy is durable before removing the original. A crash can
                        // leave both files; the action ledger then requires review, never replay.
                        self.outputs.insert(target.clone());
                        if matches!(action.operation, Move | Rename) {
                            self.check_cancelled()?;
                            if FileIdentity::read(source)? != identity {
                                return Err(Error::Invalid(
                                    "原文件在复制过程中发生变化，已保留原件与副本，请核对".into(),
                                ));
                            }
                            if let Some(tree) = copied_tree {
                                remove_copied_directory(source, &tree, &self.cancelled)?;
                            } else {
                                fs::remove_file(source)?;
                            }
                            self.selected.retain(|p| p != source);
                        }
                        result.outputs.push(self.add_output(target));
                    }
                    result.completed += 1;
                    progress(result.completed, Some(sources.len()));
                }
            }
            ZipCreate => {
                result.outputs.push(self.zip_create(action, &sources)?);
                result.completed = sources.len();
            }
            ZipList => {
                let mut items = Vec::new();
                for source in sources {
                    let mut zip = zip::ZipArchive::new(File::open(source)?).map_err(invalid)?;
                    if zip.len() > 10_000 {
                        return Err(Error::Invalid("压缩包项目过多".into()));
                    }
                    for i in 0..zip.len() {
                        let entry = zip.by_index(i).map_err(invalid)?;
                        items.push(serde_json::json!({"name":entry.name(),"size":entry.size()}));
                    }
                }
                result.completed = items.len();
                result.details = serde_json::Value::Array(items);
            }
            ZipExtract => {
                for source in sources {
                    result.outputs.push(self.zip_extract(action, &source)?);
                    result.completed += 1;
                    progress(result.completed, Some(action.sources.len()));
                }
            }
            ImageConvert => {
                for source in sources {
                    let (path, details) = self.image_convert(action, &source)?;
                    result.verified &= details["goalMet"].as_bool().unwrap_or(true);
                    result.outputs.push(path);
                    result.details = details;
                    result.completed += 1;
                    progress(result.completed, Some(action.sources.len()));
                }
            }
            MediaConvert | MediaTrim | MediaExtractAudio => {
                for source in sources {
                    let (path, details) = self.media(action, &source)?;
                    result.verified &= details["goalMet"].as_bool().unwrap_or(true);
                    result.outputs.push(path);
                    result.details = details;
                    result.completed += 1;
                    progress(result.completed, Some(action.sources.len()));
                }
            }
            PdfMerge | PdfSplit | PdfExtract | PdfRotate | PdfCompress => {
                result.outputs = self.pdf(action, &sources)?;
                result.completed = sources.len();
            }
        }
        if let Some(limit) = action.max_bytes {
            if result
                .outputs
                .iter()
                .any(|p| fs::metadata(p).is_ok_and(|m| m.is_file() && m.len() > limit))
            {
                result.verified = false;
                if !result.details.is_object() {
                    result.details = serde_json::json!({});
                }
                result.details["goalMet"] = serde_json::json!(false);
                result.details["maxBytes"] = serde_json::json!(limit);
            }
        }
        Ok(result)
    }
    fn zip_create(&mut self, action: &FileAction, sources: &[PathBuf]) -> Result<PathBuf> {
        let output = unique_path(
            &self
                .output_folder(action)?
                .join(safe_name(action.name.as_deref().unwrap_or("归档.zip"))?),
        );
        let mut temp = TemporaryOutput::new(&output)?;
        let mut zip = zip::ZipWriter::new(File::create(&temp.path)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut names = HashSet::new();
        let mut count = 0;
        for source in sources {
            if source.is_dir() && !action.recursive {
                return Err(Error::Invalid("打包文件夹需要明确包含子目录".into()));
            }
            let parent = source
                .parent()
                .ok_or_else(|| Error::Invalid("不能打包磁盘根目录".into()))?;
            for entry in walkdir::WalkDir::new(source)
                .follow_links(false)
                .max_depth(33)
            {
                self.check_cancelled()?;
                let entry = entry.map_err(invalid)?;
                count += 1;
                if entry.depth() > 32 {
                    return Err(Error::Invalid("目录深度超过 32 层，请缩小范围".into()));
                }
                if count > 10_000 {
                    return Err(Error::Invalid("打包超过 10000 个项目".into()));
                }
                if entry.file_type().is_symlink() {
                    return Err(Error::Invalid(
                        "打包不跟随符号链接，请明确选择真实文件".into(),
                    ));
                }
                let name = entry
                    .path()
                    .strip_prefix(parent)
                    .map_err(invalid)?
                    .to_string_lossy()
                    .replace('\\', "/");
                if !names.insert(name.clone()) {
                    return Err(Error::Invalid("不同目录存在同名项目，请分开打包".into()));
                }
                if entry.file_type().is_dir() {
                    zip.add_directory(format!("{name}/"), options)
                        .map_err(invalid)?;
                } else {
                    zip.start_file(name, options).map_err(invalid)?;
                    copy_stream(&mut File::open(entry.path())?, &mut zip, &self.cancelled)?;
                }
            }
        }
        zip.finish().map_err(invalid)?;
        let mut checked = zip::ZipArchive::new(File::open(&temp.path)?).map_err(invalid)?;
        for i in 0..checked.len() {
            let mut entry = checked.by_index(i).map_err(invalid)?;
            copy_stream(&mut entry, &mut std::io::sink(), &self.cancelled)?;
        }
        temp.commit(&output)?;
        Ok(self.add_output(output))
    }
    fn zip_extract(&mut self, action: &FileAction, source: &Path) -> Result<PathBuf> {
        let mut archive = zip::ZipArchive::new(File::open(source)?).map_err(invalid)?;
        let mut total = 0u64;
        if archive.len() > 10_000 {
            return Err(Error::Invalid("解压项目超过 10000 个".into()));
        }
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(invalid)?;
            if entry.enclosed_name().is_none()
                || entry.name().contains('\\')
                || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err(Error::Invalid("压缩包包含越界路径或链接，已停止".into()));
            }
            total = total
                .checked_add(entry.size())
                .ok_or_else(|| Error::Invalid("解压量过大".into()))?;
            if total > 10 * 1024 * 1024 * 1024 {
                return Err(Error::Invalid("单次解压上限 10 GB，请缩小范围".into()));
            }
        }
        let output = unique_path(
            &self.output_folder(action)?.join(safe_name(
                action
                    .name
                    .as_deref()
                    .unwrap_or(&source.file_stem().unwrap_or_default().to_string_lossy()),
            )?),
        );
        fs::create_dir(&output)?;
        self.outputs.insert(output.clone());
        for i in 0..archive.len() {
            self.check_cancelled()?;
            let mut entry = archive.by_index(i).map_err(invalid)?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| Error::Invalid("解压路径无效".into()))?;
            let path = output.join(relative);
            reject_symlinks(&path)?;
            if entry.is_dir() {
                fs::create_dir_all(path)?;
            } else {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)?;
                let expected = entry.size();
                let copied = copy_stream(
                    &mut (&mut entry).take(expected.saturating_add(1)),
                    &mut file,
                    &self.cancelled,
                )?;
                if copied != expected {
                    return Err(Error::Invalid("解压文件长度不一致".into()));
                }
                file.sync_all()?;
            }
        }
        Ok(output)
    }
    fn image_convert(
        &mut self,
        action: &FileAction,
        source: &Path,
    ) -> Result<(PathBuf, serde_json::Value)> {
        let format = action.format.as_deref().unwrap_or("jpg");
        if !["png", "jpg", "jpeg", "webp"].contains(&format) {
            return Err(Error::Invalid("图片仅支持 PNG、JPG 和 WebP".into()));
        }
        let mut reader = image::ImageReader::open(source)?.with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(512 * 1024 * 1024);
        reader.limits(limits);
        let mut image = reader.decode().map_err(invalid)?;
        if action.width.is_some() || action.height.is_some() {
            let width = action.width.unwrap_or(u32::MAX);
            let height = action.height.unwrap_or(u32::MAX);
            if width == 0
                || height == 0
                || (width != u32::MAX && width > 32768)
                || (height != u32::MAX && height > 32768)
            {
                return Err(Error::Invalid("图片尺寸应在 1–32768 像素之间".into()));
            }
            image = image.resize(width, height, image::imageops::FilterType::Lanczos3);
        }
        image = match action.rotation.unwrap_or(0) {
            0 => image,
            90 => image.rotate90(),
            180 => image.rotate180(),
            270 => image.rotate270(),
            _ => return Err(Error::Invalid("旋转角度仅支持 90、180、270".into())),
        };
        let name = action.name.clone().unwrap_or_else(|| {
            format!(
                "{}.{}",
                source.file_stem().unwrap_or_default().to_string_lossy(),
                format
            )
        });
        let output = unique_path(&self.output_folder(action)?.join(safe_name(&name)?));
        let mut temp = TemporaryOutput::new(&output)?;
        let mut quality = action.quality.unwrap_or(85).clamp(1, 100);
        for _ in 0..8 {
            self.check_cancelled()?;
            if matches!(format, "jpg" | "jpeg") {
                image::codecs::jpeg::JpegEncoder::new_with_quality(
                    File::create(&temp.path)?,
                    quality,
                )
                .encode_image(&image)
                .map_err(invalid)?;
            } else {
                image
                    .save_with_format(
                        &temp.path,
                        if format == "png" {
                            image::ImageFormat::Png
                        } else {
                            image::ImageFormat::WebP
                        },
                    )
                    .map_err(invalid)?;
            }
            if action
                .max_bytes
                .is_none_or(|limit| fs::metadata(&temp.path).is_ok_and(|m| m.len() <= limit))
                || !matches!(format, "jpg" | "jpeg")
                || quality <= 30
            {
                break;
            }
            quality = quality.saturating_sub(10).max(30);
        }
        let check = image::ImageReader::open(&temp.path)?
            .with_guessed_format()?
            .decode()
            .map_err(invalid)?;
        if check.width() != image.width() || check.height() != image.height() {
            return Err(Error::Invalid("图片尺寸核验失败".into()));
        }
        let size = fs::metadata(&temp.path)?.len();
        let details = serde_json::json!({"width":image.width(),"height":image.height(),"size":size,"format":format,"goalMet":action.max_bytes.is_none_or(|v|size<=v)});
        self.check_cancelled()?;
        temp.commit(&output)?;
        Ok((self.add_output(output), details))
    }
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}
fn safe_name(name: &str) -> Result<&str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
        || name.chars().any(char::is_control)
    {
        return Err(Error::Invalid("文件名不能包含路径分隔符或控制字符".into()));
    }
    Ok(name)
}
fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path
        .extension()
        .map(|v| format!(".{}", v.to_string_lossy()))
        .unwrap_or_default();
    for i in 2..100_000 {
        let candidate = path.with_file_name(format!("{stem} ({i}){extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_file_name(format!("{stem}-{}{extension}", uuid::Uuid::new_v4()))
}
fn reject_symlinks(path: &Path) -> Result<()> {
    let mut prefix = PathBuf::new();
    for component in path.components() {
        prefix.push(component.as_os_str());
        // macOS exposes these system-managed aliases even for ordinary temporary
        // files. Canonicalization below resolves them before comparing scopes.
        if cfg!(target_os = "macos")
            && [Path::new("/var"), Path::new("/tmp"), Path::new("/etc")].contains(&prefix.as_path())
        {
            continue;
        }
        if fs::symlink_metadata(&prefix).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::Invalid("此路径包含符号链接，请选择真实位置".into()));
        }
    }
    Ok(())
}
fn info(path: &Path) -> Result<serde_json::Value> {
    let m = fs::symlink_metadata(path)?;
    Ok(
        serde_json::json!({"path":path,"name":path.file_name().unwrap_or_default().to_string_lossy(),"size":m.len(),"directory":m.is_dir(),"symlink":m.file_type().is_symlink(),"modified":m.modified().ok().and_then(|t|t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs())}),
    )
}
fn rename_name(name: &str, action: &FileAction, index: usize) -> Result<String> {
    let path = Path::new(name);
    let mut stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let extension = path
        .extension()
        .map(|v| format!(".{}", v.to_string_lossy()))
        .unwrap_or_default();
    if let Some(case) = action.letter_case.as_deref() {
        stem = match case {
            "lower" => stem.to_lowercase(),
            "upper" => stem.to_uppercase(),
            _ => return Err(Error::Invalid("大小写模式只能为 lower 或 upper".into())),
        };
    }
    if let Some(find) = &action.find {
        if find.is_empty() {
            return Err(Error::Invalid("替换词不能为空".into()));
        }
        stem = stem.replace(find, action.replace.as_deref().unwrap_or(""));
    }
    let value = if let Some(template) = &action.name {
        template
            .replace("{name}", &stem)
            .replace("{ext}", &extension)
            .replace(
                "{index}",
                &format!("{:03}", action.numbering.unwrap_or(1) as usize + index),
            )
    } else {
        format!(
            "{}{}{}{}{}",
            action.prefix.as_deref().unwrap_or(""),
            stem,
            action.suffix.as_deref().unwrap_or(""),
            action
                .numbering
                .map(|n| format!("_{:03}", n as usize + index))
                .unwrap_or_default(),
            extension
        )
    };
    safe_name(&value)?;
    Ok(value)
}
fn copy_stream(reader: &mut dyn Read, writer: &mut dyn Write, cancel: &AtomicBool) -> Result<u64> {
    let mut buffer = vec![0u8; 128 * 1024];
    let mut total = 0;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        total += n as u64;
    }
    Ok(total)
}
fn copy_file(source: &Path, destination: &Path, cancel: &AtomicBool) -> Result<()> {
    let identity = FileIdentity::read(source)?;
    let mut temp = TemporaryOutput::new(destination)?;
    let mut input = File::open(source)?;
    let expected = input.metadata()?.len();
    let mut output = File::create(&temp.path)?;
    let copied = copy_stream(&mut input, &mut output, cancel)?;
    output.sync_all()?;
    fs::set_permissions(&temp.path, input.metadata()?.permissions())?;
    if copied != expected || output.metadata()?.len() != expected {
        return Err(Error::Invalid("复制长度核验失败".into()));
    }
    if FileIdentity::read(source)? != identity {
        return Err(Error::Invalid(
            "文件在复制过程中发生变化，请重新提交".into(),
        ));
    }
    temp.commit(destination)
}
fn copy_directory(
    source: &Path,
    destination: &Path,
    cancel: &AtomicBool,
) -> Result<Vec<(PathBuf, FileIdentity)>> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(destination)?;
    let mut copied = Vec::new();
    for (i, entry) in walkdir::WalkDir::new(source)
        .follow_links(false)
        .min_depth(1)
        .max_depth(33)
        .into_iter()
        .enumerate()
    {
        if i >= 10_000 {
            return Err(Error::Invalid("目录对象超过 10000 个".into()));
        }
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let entry = entry.map_err(invalid)?;
        if entry.depth() > 32 {
            return Err(Error::Invalid("目录深度超过 32 层，原件已保留".into()));
        }
        if entry.file_type().is_symlink() {
            return Err(Error::Invalid("目录内包含符号链接，已停止".into()));
        }
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(invalid)?
            .to_path_buf();
        let target = destination.join(&relative);
        let identity = FileIdentity::read(entry.path())?;
        if entry.file_type().is_dir() {
            builder.create(&target)?;
        } else {
            copy_file(entry.path(), &target, cancel)?;
        }
        copied.push((relative, identity));
    }
    for (relative, identity) in copied.iter().rev() {
        if identity.directory {
            fs::set_permissions(
                destination.join(relative),
                fs::metadata(source.join(relative))?.permissions(),
            )?;
        }
    }
    fs::set_permissions(destination, fs::metadata(source)?.permissions())?;
    Ok(copied)
}
fn remove_copied_directory(
    source: &Path,
    tree: &[(PathBuf, FileIdentity)],
    cancel: &AtomicBool,
) -> Result<()> {
    // Remove only the entries that were copied and still have their captured
    // identity. New files keep their directory nonempty and are never deleted.
    for (relative, expected) in tree.iter().rev() {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        let path = source.join(relative);
        if FileIdentity::read(&path)? != *expected {
            return Err(Error::Invalid(
                "目录内容在移动过程中发生变化，剩余原件和副本已保留".into(),
            ));
        }
        if expected.directory {
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    fs::remove_dir(source)
        .map_err(|_| Error::Invalid("目录中出现新对象，已保留这些对象及副本，请核对".into()))
}
struct TemporaryOutput {
    path: PathBuf,
}
impl TemporaryOutput {
    fn new(destination: &Path) -> Result<Self> {
        let path = destination.with_file_name(format!(
            ".fleqi-{}.{}",
            uuid::Uuid::new_v4(),
            destination
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path)?;
        Ok(Self { path })
    }
    fn commit(&mut self, destination: &Path) -> Result<()> {
        File::open(&self.path)?.sync_all()?;
        tempfile::TempPath::try_from_path(&self.path)?
            .persist_noclobber(destination)
            .map_err(|e| Error::Io(e.error))?;
        Ok(())
    }
}
impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn move_exclusive(source: &Path, destination: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use rustix::fs::{CWD, RenameFlags, renameat_with};
        use rustix::io::Errno;
        match renameat_with(CWD, source, CWD, destination, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(true),
            Err(Errno::XDEV | Errno::NOSYS | Errno::INVAL) => Ok(false),
            Err(error) => Err(Error::Io(error.into())),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (source, destination);
        Ok(false)
    }
}
