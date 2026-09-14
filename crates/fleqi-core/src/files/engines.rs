use super::*;
use std::{
    ffi::OsString,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

fn drain(mut pipe: impl Read) -> Vec<u8> {
    let mut saved = Vec::new();
    let mut buffer = [0u8; 8192];
    while let Ok(n) = pipe.read(&mut buffer) {
        if n == 0 {
            break;
        }
        let keep = n.min(1_000_000usize.saturating_sub(saved.len()));
        saved.extend_from_slice(&buffer[..keep]);
    }
    saved
}
pub fn run(executable: &Path, args: &[OsString], cancel: &AtomicBool) -> Result<String> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::Invalid("无法获取引擎输出".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| Error::Invalid("无法获取引擎错误".into()))?;
    let out = thread::spawn(move || drain(stdout));
    let err = thread::spawn(move || drain(stderr));
    let mut cancelled = false;
    let status = loop {
        if cancel.load(Ordering::Acquire) {
            let _ = child.kill();
            cancelled = true;
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        thread::sleep(Duration::from_millis(50));
    };
    let stdout = out
        .join()
        .map_err(|_| Error::Invalid("引擎输出读取失败".into()))?;
    let stderr = err
        .join()
        .map_err(|_| Error::Invalid("引擎错误读取失败".into()))?;
    if cancelled {
        return Err(Error::Cancelled);
    }
    if !status.success() {
        return Err(Error::Invalid(format!(
            "引擎执行失败（{}）：{}",
            status,
            String::from_utf8_lossy(&stderr)
                .chars()
                .take(1200)
                .collect::<String>()
        )));
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}
impl FileExecutor {
    fn probe(&self, path: &Path) -> Result<serde_json::Value> {
        let executable = self
            .engines
            .ffprobe
            .as_ref()
            .ok_or_else(|| Error::Invalid("随包 ffprobe 未就绪，请检查文件引擎".into()))?;
        let args = [
            "-v".into(),
            "error".into(),
            "-show_format".into(),
            "-show_streams".into(),
            "-of".into(),
            "json".into(),
            path.as_os_str().to_owned(),
        ];
        Ok(serde_json::from_str(&run(
            executable,
            &args,
            &self.cancelled,
        )?)?)
    }
    pub(super) fn media(
        &mut self,
        action: &FileAction,
        source: &Path,
    ) -> Result<(PathBuf, serde_json::Value)> {
        let ffmpeg = self
            .engines
            .ffmpeg
            .clone()
            .ok_or_else(|| Error::Invalid("随包 FFmpeg 未就绪，请检查文件引擎".into()))?;
        let format = action.format.as_deref().unwrap_or(
            if matches!(action.operation, Operation::MediaExtractAudio) {
                "m4a"
            } else {
                "mp4"
            },
        );
        if !["mp3", "m4a", "wav", "mp4"].contains(&format) {
            return Err(Error::Invalid("音视频输出仅支持 MP3、M4A、WAV、MP4".into()));
        }
        let input = self.probe(source)?;
        let duration = input["format"]["duration"]
            .as_str()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v > 0.)
            .ok_or_else(|| Error::Invalid("无法确定媒体时长".into()))?;
        let start = action.start.unwrap_or(0.);
        let length = action.duration.unwrap_or(duration - start);
        if !start.is_finite()
            || !length.is_finite()
            || start < 0.
            || length <= 0.
            || start >= duration
            || start + length > duration + 0.5
        {
            return Err(Error::Invalid("裁剪起点或时长超出文件范围".into()));
        }
        let name = action.name.clone().unwrap_or_else(|| {
            format!(
                "{}.{}",
                source.file_stem().unwrap_or_default().to_string_lossy(),
                format
            )
        });
        let output = unique_path(&self.output_folder(action)?.join(safe_name(&name)?));
        let mut temp = TemporaryOutput::new(&output)?;
        let mut args: Vec<OsString> = vec![
            "-nostdin".into(),
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            source.as_os_str().to_owned(),
        ];
        if action.start.is_some() {
            args.extend(["-ss".into(), start.to_string().into()]);
        }
        if action.duration.is_some() {
            args.extend(["-t".into(), length.to_string().into()]);
        }
        if format == "mp4" {
            if !input["streams"].as_array().is_some_and(|s| {
                s.iter().any(|s| {
                    s["codec_type"] == "video"
                        && matches!(s["codec_name"].as_str(), Some("h264" | "hevc"))
                })
            }) {
                return Err(Error::Invalid("首版视频输入要求 H.264 或 HEVC 编码".into()));
            }
            let encoder = if cfg!(target_os = "macos") {
                "h264_videotoolbox"
            } else {
                "libopenh264"
            };
            args.extend([
                "-c:v".into(),
                encoder.into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "aac".into(),
                "-b:a".into(),
                "128k".into(),
                "-movflags".into(),
                "+faststart".into(),
            ]);
            let bitrate = action.bitrate.unwrap_or_else(|| {
                action
                    .max_bytes
                    .map(|size| (((size as f64 * 8. / length) / 1000. - 160.).max(100.)) as u32)
                    .unwrap_or(4000)
            });
            if !(100..=100_000).contains(&bitrate) {
                return Err(Error::Invalid("视频码率应在 100–100000 kbps 之间".into()));
            }
            args.extend(["-b:v".into(), format!("{bitrate}k").into()]);
            if action.width.is_some() || action.height.is_some() {
                let w = action.width.unwrap_or(0);
                let h = action.height.unwrap_or(0);
                if w > 7680 || h > 7680 {
                    return Err(Error::Invalid("视频尺寸超过 7680 像素".into()));
                }
                let filter = if w > 0 && h > 0 {
                    format!(
                        "scale={w}:{h}:force_original_aspect_ratio=decrease:force_divisible_by=2"
                    )
                } else {
                    format!(
                        "scale={}:{}",
                        if w == 0 { "-2".into() } else { w.to_string() },
                        if h == 0 { "-2".into() } else { h.to_string() }
                    )
                };
                args.extend(["-vf".into(), filter.into()]);
            }
        } else {
            args.extend([
                "-vn".into(),
                "-c:a".into(),
                match format {
                    "mp3" => "libmp3lame",
                    "m4a" => "aac",
                    _ => "pcm_s16le",
                }
                .into(),
            ]);
            if format != "wav" {
                args.extend([
                    "-b:a".into(),
                    format!("{}k", action.bitrate.unwrap_or(192).clamp(32, 320)).into(),
                ]);
            }
        }
        args.extend([
            "-f".into(),
            match format {
                "m4a" => "ipod",
                other => other,
            }
            .into(),
            temp.path.as_os_str().to_owned(),
        ]);
        run(&ffmpeg, &args, &self.cancelled)?;
        let output_probe = self.probe(&temp.path)?;
        let actual = output_probe["format"]["duration"]
            .as_str()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.);
        if !actual.is_finite()
            || actual <= 0.
            || (actual - length).abs() > (length * 0.01).clamp(0.5, 1.5)
        {
            return Err(Error::Invalid("输出时长核验未通过".into()));
        }
        if format == "mp4"
            && !output_probe["streams"]
                .as_array()
                .is_some_and(|s| s.iter().any(|v| v["codec_name"] == "h264"))
        {
            return Err(Error::Invalid("视频编码核验失败".into()));
        }
        let size = fs::metadata(&temp.path)?.len();
        let details = serde_json::json!({"duration":actual,"size":size,"format":format,"goalMet":action.max_bytes.is_none_or(|limit|size<=limit)});
        self.check_cancelled()?;
        temp.commit(&output)?;
        Ok((self.add_output(output), details))
    }
    fn pdf_pages(&self, qpdf: &Path, path: &Path) -> Result<u32> {
        run(
            qpdf,
            &["--show-npages".into(), path.as_os_str().to_owned()],
            &self.cancelled,
        )?
        .trim()
        .parse()
        .map_err(invalid)
    }
    pub(super) fn pdf(&mut self, action: &FileAction, sources: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let qpdf = self
            .engines
            .qpdf
            .clone()
            .ok_or_else(|| Error::Invalid("随包 qpdf 未就绪，请检查文件引擎".into()))?;
        let counts = sources
            .iter()
            .map(|p| self.pdf_pages(&qpdf, p))
            .collect::<Result<Vec<_>>>()?;
        let pages = action.pages.as_deref().unwrap_or("1-z");
        if pages.is_empty()
            || pages.len() > 1000
            || !pages
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '-' | ',' | 'z'))
        {
            return Err(Error::Invalid("页码应类似 1-3,5 或 1-z".into()));
        }
        let mut outputs = Vec::new();
        let split = matches!(action.operation, Operation::PdfSplit);
        if !matches!(action.operation, Operation::PdfMerge) && sources.len() != 1 {
            return Err(Error::Invalid(
                "本次 PDF 操作请只选择一个文件；合并可多选".into(),
            ));
        }
        let amount = if split { counts[0] } else { 1 };
        if amount > 10_000 {
            return Err(Error::Invalid("拆分页数超过 10000".into()));
        }
        for index in 1..=amount {
            self.check_cancelled()?;
            let name = if split {
                format!(
                    "{}_{}.pdf",
                    sources[0].file_stem().unwrap_or_default().to_string_lossy(),
                    index
                )
            } else {
                action.name.clone().unwrap_or_else(|| "处理结果.pdf".into())
            };
            let path = unique_path(&self.output_folder(action)?.join(safe_name(&name)?));
            let mut temp = TemporaryOutput::new(&path)?;
            let mut args: Vec<OsString> = Vec::new();
            match action.operation {
                Operation::PdfMerge => {
                    args.extend(["--empty".into(), "--pages".into()]);
                    for source in sources {
                        args.extend([source.as_os_str().to_owned(), "1-z".into()]);
                    }
                    args.push("--".into());
                }
                Operation::PdfExtract | Operation::PdfSplit => {
                    args.extend([
                        sources[0].as_os_str().to_owned(),
                        "--pages".into(),
                        sources[0].as_os_str().to_owned(),
                        if split {
                            index.to_string().into()
                        } else {
                            pages.into()
                        },
                        "--".into(),
                    ]);
                }
                Operation::PdfRotate => {
                    let rotation = action.rotation.unwrap_or(90);
                    if ![90, 180, 270].contains(&rotation) {
                        return Err(Error::Invalid("PDF 旋转仅支持 90、180、270 度".into()));
                    }
                    args.extend([
                        sources[0].as_os_str().to_owned(),
                        format!("--rotate=+{rotation}:{pages}").into(),
                    ]);
                }
                _ => {
                    args.extend([
                        sources[0].as_os_str().to_owned(),
                        "--object-streams=generate".into(),
                        "--recompress-flate".into(),
                        "--compression-level=9".into(),
                    ]);
                }
            }
            args.push(temp.path.as_os_str().to_owned());
            run(&qpdf, &args, &self.cancelled)?;
            run(
                &qpdf,
                &["--check".into(), temp.path.as_os_str().to_owned()],
                &self.cancelled,
            )?;
            let actual = self.pdf_pages(&qpdf, &temp.path)?;
            let expected = if matches!(action.operation, Operation::PdfMerge) {
                Some(counts.iter().sum())
            } else if split {
                Some(1)
            } else if !matches!(action.operation, Operation::PdfExtract) {
                Some(counts[0])
            } else {
                None
            };
            if actual == 0 || expected.is_some_and(|n| actual != n) {
                return Err(Error::Invalid("PDF 页数核验失败".into()));
            }
            self.check_cancelled()?;
            temp.commit(&path)?;
            outputs.push(self.add_output(path));
        }
        Ok(outputs)
    }
}
