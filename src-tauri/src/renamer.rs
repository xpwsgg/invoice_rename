use crate::error::AppError;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlan {
    pub source: PathBuf,
    pub target: PathBuf,
    pub invoice_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RenameSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    #[serde(rename = "outputDir")]
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: String,
    pub level: String,
    pub message: String,
}

pub trait Logger {
    fn log(&mut self, entry: LogEntry);
}

/// 文件名中不允许出现的字符（跨平台保守集合）
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

pub fn validate_name(field: &str, value: &str) -> Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::InvalidInput(format!("{field} 不能为空")));
    }
    if value.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(AppError::InvalidInput(format!(
            "{field} 不能包含特殊字符 / \\ : * ? \" < > |"
        )));
    }
    Ok(())
}

/// 扫描源目录顶层 PDF 文件，结合提取函数构造重命名计划。
pub fn build_plan<F>(
    source_dir: &Path,
    user_name: &str,
    tracking_number: &str,
    extract_fn: F,
) -> Result<Vec<RenamePlan>, AppError>
where
    F: Fn(&Path) -> Result<Option<String>, AppError>,
{
    validate_name("用户名", user_name)?;
    validate_name("Tracking Number", tracking_number)?;

    if !source_dir.is_dir() {
        return Err(AppError::Io(format!(
            "源文件夹不存在或不是目录：{}",
            source_dir.display()
        )));
    }

    let output_dir = source_dir.join(tracking_number);

    let mut plans = Vec::new();
    for entry in std::fs::read_dir(source_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !is_pdf(&path) {
            continue;
        }

        let invoice = extract_fn(&path).unwrap_or(None);
        let prefix = invoice.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        let filename = format!("{prefix}-{user_name}-{tracking_number}.pdf");
        let target = output_dir.join(filename);

        plans.push(RenamePlan {
            source: path,
            target,
            invoice_number: invoice,
        });
    }

    plans.sort_by(|a, b| a.source.file_name().cmp(&b.source.file_name()));
    Ok(plans)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

const MAX_DEDUPE: u32 = 999;

fn timestamp() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn make_log(level: &str, message: impl Into<String>) -> LogEntry {
    LogEntry {
        ts: timestamp(),
        level: level.to_string(),
        message: message.into(),
    }
}

/// 在 `target` 已存在时尝试 `name-1.pdf`、`name-2.pdf`… 直到上限 999。
/// 返回最终使用的路径以及"是否加了序号"，找不到空位时返回 None。
fn resolve_target(target: &Path) -> Option<(PathBuf, u32)> {
    if !target.exists() {
        return Some((target.to_path_buf(), 0));
    }
    let parent = target.parent()?;
    let stem = target.file_stem()?.to_str()?;
    let ext = target.extension().and_then(|e| e.to_str()).unwrap_or("");
    for n in 1..=MAX_DEDUPE {
        let candidate_name = if ext.is_empty() {
            format!("{stem}_{n}")
        } else {
            format!("{stem}_{n}.{ext}")
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return Some((candidate, n));
        }
    }
    None
}

pub fn execute_plan<L: Logger>(plans: &[RenamePlan], logger: &mut L) -> RenameSummary {
    let total = plans.len();
    let mut success = 0usize;
    let mut failed = 0usize;
    let mut copied = 0usize;

    let output_dir: Option<String> = plans
        .first()
        .and_then(|p| p.target.parent())
        .map(|p| p.display().to_string());

    if total == 0 {
        logger.log(make_log("warn", "未找到 PDF 文件"));
        return RenameSummary {
            total,
            success,
            failed,
            output_dir: None,
        };
    }

    logger.log(make_log("info", format!("扫描到 {total} 个 PDF 文件")));

    // 输出目录由第一个 plan 的 target.parent 决定
    if let Some(first) = plans.first() {
        if let Some(out_dir) = first.target.parent() {
            if let Err(e) = std::fs::create_dir_all(out_dir) {
                logger.log(make_log(
                    "error",
                    format!("创建输出目录失败：{} ({e})", out_dir.display()),
                ));
                return RenameSummary {
                    total,
                    success,
                    failed: total,
                    output_dir: None,
                };
            }
        }
    }

    let started = std::time::Instant::now();

    for (idx, plan) in plans.iter().enumerate() {
        let i = idx + 1;
        let src_name = plan
            .source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        match resolve_target(&plan.target) {
            None => {
                logger.log(make_log(
                    "error",
                    format!("[{i}/{total}] {src_name} 跳过：同名文件序号已耗尽（>{MAX_DEDUPE}）"),
                ));
                failed += 1;
            }
            Some((final_target, dedupe_n)) => match std::fs::copy(&plan.source, &final_target) {
                Ok(_) => {
                    copied += 1;
                    let target_name = final_target
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let suffix_note = if dedupe_n > 0 {
                        "（同名已存在，加序号）"
                    } else {
                        ""
                    };
                    if plan.invoice_number.is_none() {
                        logger.log(make_log(
                            "warn",
                            format!(
                                "[{i}/{total}] {src_name} → {target_name}{suffix_note}（未识别发票号，需手工补救）"
                            ),
                        ));
                        failed += 1;
                    } else {
                        logger.log(make_log(
                            "info",
                            format!("[{i}/{total}] {src_name} → {target_name}{suffix_note}"),
                        ));
                        success += 1;
                    }
                }
                Err(e) => {
                    logger.log(make_log(
                        "error",
                        format!("[{i}/{total}] {src_name} 复制失败：{e}"),
                    ));
                    failed += 1;
                }
            },
        }
    }

    let secs = started.elapsed().as_secs_f32();
    logger.log(make_log(
        "info",
        format!("完成：成功 {success}，失败 {failed}，耗时 {secs:.1}s"),
    ));

    RenameSummary {
        total,
        success,
        failed,
        output_dir: if copied > 0 { output_dir } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        File::create(&p).expect("touch file");
        p
    }

    fn ok_extract(_: &Path) -> Result<Option<String>, AppError> {
        Ok(Some("26322000000893295511".to_string()))
    }

    fn none_extract(_: &Path) -> Result<Option<String>, AppError> {
        Ok(None)
    }

    #[test]
    fn rejects_empty_user_name() {
        let dir = TempDir::new().unwrap();
        let err = build_plan(dir.path(), "", "TN", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn rejects_forbidden_chars_in_tracking() {
        let dir = TempDir::new().unwrap();
        let err = build_plan(dir.path(), "Felix", "abc/def", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[test]
    fn rejects_missing_source_dir() {
        let bogus = PathBuf::from("/tmp/__definitely_not_existing_dir__");
        let err = build_plan(&bogus, "Felix", "TN", ok_extract).unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
    }

    #[test]
    fn collects_only_top_level_pdfs_case_insensitive() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");
        touch(dir.path(), "b.PDF");
        touch(dir.path(), "c.txt");
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        touch(&dir.path().join("sub"), "ignored.pdf");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let names: Vec<_> = plans
            .iter()
            .map(|p| p.source.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["a.pdf", "b.PDF"]);
    }

    #[test]
    fn target_path_uses_invoice_number_user_and_tracking() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");

        let plans = build_plan(dir.path(), "Felix", "000-115-216", ok_extract).unwrap();
        assert_eq!(plans.len(), 1);
        let p = &plans[0];
        assert_eq!(p.invoice_number.as_deref(), Some("26322000000893295511"));
        assert_eq!(
            p.target,
            dir.path()
                .join("000-115-216")
                .join("26322000000893295511-Felix-000-115-216.pdf")
        );
    }

    #[test]
    fn unknown_prefix_when_extract_returns_none() {
        let dir = TempDir::new().unwrap();
        touch(dir.path(), "a.pdf");
        let plans = build_plan(dir.path(), "Felix", "TN", none_extract).unwrap();
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].invoice_number, None);
        assert_eq!(
            plans[0]
                .target
                .file_name()
                .unwrap()
                .to_string_lossy(),
            "UNKNOWN-Felix-TN.pdf"
        );
    }

    #[derive(Default)]
    struct VecLogger {
        entries: Vec<LogEntry>,
    }
    impl Logger for VecLogger {
        fn log(&mut self, entry: LogEntry) {
            self.entries.push(entry);
        }
    }

    fn make_pdf(dir: &Path, name: &str, payload: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, payload).unwrap();
        p
    }

    #[test]
    fn execute_copies_files_to_output_subdir() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        make_pdf(dir.path(), "b.pdf", "BBB");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let mut logger = VecLogger::default();
        let summary = execute_plan(&plans, &mut logger);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.success, 2);
        assert_eq!(summary.failed, 0);

        let out = dir.path().join("TN");
        assert!(out.is_dir());
        let count = std::fs::read_dir(&out).unwrap().count();
        assert_eq!(count, 2);
        assert!(dir.path().join("a.pdf").exists());
        assert!(dir.path().join("b.pdf").exists());
    }

    #[test]
    fn execute_adds_sequence_suffix_on_conflict() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "first");
        make_pdf(dir.path(), "b.pdf", "second");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let mut logger = VecLogger::default();
        let summary = execute_plan(&plans, &mut logger);

        assert_eq!(summary.success, 2);
        let out = dir.path().join("TN");
        let base = out.join("26322000000893295511-Felix-TN.pdf");
        let seq = out.join("26322000000893295511-Felix-TN_1.pdf");
        assert!(base.exists());
        assert!(seq.exists());
    }

    #[test]
    fn execute_empty_plan_logs_warn() {
        let plans: Vec<RenamePlan> = Vec::new();
        let mut logger = VecLogger::default();
        let s = execute_plan(&plans, &mut logger);
        assert_eq!(s.total, 0);
        assert_eq!(s.success, 0);
        assert_eq!(s.failed, 0);
        assert!(logger
            .entries
            .iter()
            .any(|e| e.message.contains("未找到 PDF")));
    }

    #[test]
    fn execute_unknown_logs_warn_but_still_copies() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        let plans = build_plan(dir.path(), "Felix", "TN", none_extract).unwrap();
        let mut logger = VecLogger::default();
        let s = execute_plan(&plans, &mut logger);
        assert_eq!(s.success, 0);
        assert_eq!(s.failed, 1);
        assert!(logger
            .entries
            .iter()
            .any(|e| e.level == "warn" && e.message.contains("UNKNOWN")));
        assert!(dir.path().join("TN").join("UNKNOWN-Felix-TN.pdf").exists());
        // UNKNOWN 占位但已落地：仍应返回 output_dir，便于用户手工补救
        assert!(s.output_dir.is_some());
    }
}
