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
}
