use crate::error::AppError;
use crate::pdf_parser::InvoiceInfo;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePlan {
    pub source: PathBuf,
    pub target: PathBuf,
    pub invoice_number: Option<String>,
    pub total_amount_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenameSummary {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    pub output_dir: Option<String>,
    /// 已识别金额之和（分）。基于所有扫描到的发票，与复制成败无关。
    pub total_amount_cents: i64,
    pub amount_recognized: usize,
    pub amount_missing: usize,
}

/// 发给前端的「一行发票结果」。serde 统一 camelCase 供前端直接消费。
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceRow {
    pub index: usize,
    pub total: usize,
    pub source_name: String,
    pub invoice_number: Option<String>,
    pub amount_cents: Option<i64>,
    /// 后端预格式化的金额，如 "¥93.33"；无金额时为 None。
    pub amount_display: Option<String>,
    /// "success" | "failed"
    pub status: String,
    /// 失败/警示原因；成功且无异常时为空串。
    pub note: String,
}

/// 进度回调：每处理完一张发票推送一行结果。
pub trait ProgressSink {
    fn row(&mut self, row: InvoiceRow);
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

/// 把「分」格式化为带千分位的人民币显示：98301 -> "¥983.01"、1234567 -> "¥12,345.67"、0 -> "¥0.00"。
pub fn format_amount(cents: i64) -> String {
    let yuan = cents / 100;
    let frac = cents % 100;
    format!("¥{}.{:02}", with_thousands(yuan), frac)
}

/// 为非负整数插入千分位逗号。
fn with_thousands(n: i64) -> String {
    let digits = n.to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// 扫描源目录顶层 PDF 文件，结合提取函数构造重命名计划。
pub fn build_plan<F>(
    source_dir: &Path,
    user_name: &str,
    tracking_number: &str,
    extract_fn: F,
) -> Result<Vec<RenamePlan>, AppError>
where
    F: Fn(&Path) -> Result<InvoiceInfo, AppError>,
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

        let info = extract_fn(&path).unwrap_or_else(|_| InvoiceInfo {
            number: None,
            total_amount_cents: None,
        });
        let invoice = info.number;
        let prefix = invoice.clone().unwrap_or_else(|| "UNKNOWN".to_string());
        let filename = format!("{prefix}-{user_name}-{tracking_number}.pdf");
        let target = output_dir.join(filename);

        plans.push(RenamePlan {
            source: path,
            target,
            invoice_number: invoice,
            total_amount_cents: info.total_amount_cents,
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

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
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

/// 执行重命名计划：逐张复制并通过 `sink` 推送每行结果，最后返回汇总。
/// 金额统计基于所有扫描到的发票（与复制成败无关）。
pub fn execute_plan<S: ProgressSink>(plans: &[RenamePlan], sink: &mut S) -> RenameSummary {
    let total = plans.len();
    let mut success = 0usize;
    let mut failed = 0usize;
    let mut copied = 0usize;
    let mut total_amount_cents = 0i64;
    let mut amount_recognized = 0usize;
    let mut amount_missing = 0usize;

    let output_dir: Option<String> = plans
        .first()
        .and_then(|p| p.target.parent())
        .map(|p| p.display().to_string());

    if total == 0 {
        return RenameSummary {
            total,
            success,
            failed,
            output_dir: None,
            total_amount_cents,
            amount_recognized,
            amount_missing,
        };
    }

    // 尽力创建输出目录（幂等）。command 层已提前创建并对失败返回全局错误；
    // 万一此处仍失败，后续 copy 会各自失败并被记为 failed 行。
    if let Some(out_dir) = plans.first().and_then(|p| p.target.parent()) {
        let _ = std::fs::create_dir_all(out_dir);
    }

    for (idx, plan) in plans.iter().enumerate() {
        let i = idx + 1;
        let src_name = file_name_of(&plan.source);

        // 金额汇总：基于所有扫描到的发票，与复制成败无关。
        match plan.total_amount_cents {
            Some(c) => {
                total_amount_cents += c;
                amount_recognized += 1;
            }
            None => amount_missing += 1,
        }

        let (status, note) = match resolve_target(&plan.target) {
            None => {
                failed += 1;
                (
                    "failed".to_string(),
                    format!("同名文件序号已耗尽（>{MAX_DEDUPE}）"),
                )
            }
            Some((final_target, dedupe_n)) => match std::fs::copy(&plan.source, &final_target) {
                Ok(_) => {
                    copied += 1;
                    let suffix_note = if dedupe_n > 0 {
                        "（同名已存在，已加序号）"
                    } else {
                        ""
                    };
                    if plan.invoice_number.is_none() {
                        failed += 1;
                        (
                            "failed".to_string(),
                            format!("未识别发票号，已存为 UNKNOWN{suffix_note}，需手工补救"),
                        )
                    } else {
                        success += 1;
                        ("success".to_string(), suffix_note.to_string())
                    }
                }
                Err(e) => {
                    failed += 1;
                    ("failed".to_string(), format!("复制失败：{e}"))
                }
            },
        };

        sink.row(InvoiceRow {
            index: i,
            total,
            source_name: src_name,
            invoice_number: plan.invoice_number.clone(),
            amount_cents: plan.total_amount_cents,
            amount_display: plan.total_amount_cents.map(format_amount),
            status,
            note,
        });
    }

    RenameSummary {
        total,
        success,
        failed,
        output_dir: if copied > 0 { output_dir } else { None },
        total_amount_cents,
        amount_recognized,
        amount_missing,
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

    fn ok_extract(_: &Path) -> Result<InvoiceInfo, AppError> {
        Ok(InvoiceInfo {
            number: Some("26322000000893295511".to_string()),
            total_amount_cents: Some(9333),
        })
    }

    fn none_extract(_: &Path) -> Result<InvoiceInfo, AppError> {
        Ok(InvoiceInfo {
            number: None,
            total_amount_cents: None,
        })
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
        assert_eq!(p.total_amount_cents, Some(9333));
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
        assert_eq!(plans[0].total_amount_cents, None);
        assert_eq!(
            plans[0].target.file_name().unwrap().to_string_lossy(),
            "UNKNOWN-Felix-TN.pdf"
        );
    }

    #[derive(Default)]
    struct RowSink {
        rows: Vec<InvoiceRow>,
    }
    impl ProgressSink for RowSink {
        fn row(&mut self, row: InvoiceRow) {
            self.rows.push(row);
        }
    }

    fn make_pdf(dir: &Path, name: &str, payload: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, payload).unwrap();
        p
    }

    #[test]
    fn format_amount_basic() {
        assert_eq!(format_amount(98301), "¥983.01");
        assert_eq!(format_amount(1234567), "¥12,345.67");
        assert_eq!(format_amount(0), "¥0.00");
        assert_eq!(format_amount(9333), "¥93.33");
        assert_eq!(format_amount(100), "¥1.00");
        assert_eq!(format_amount(5), "¥0.05");
    }

    #[test]
    fn execute_copies_files_to_output_subdir() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        make_pdf(dir.path(), "b.pdf", "BBB");

        let plans = build_plan(dir.path(), "Felix", "TN", ok_extract).unwrap();
        let mut sink = RowSink::default();
        let summary = execute_plan(&plans, &mut sink);

        assert_eq!(summary.total, 2);
        assert_eq!(summary.success, 2);
        assert_eq!(summary.failed, 0);
        // 金额汇总：每张 9333，共 18666，全部识别
        assert_eq!(summary.total_amount_cents, 18666);
        assert_eq!(summary.amount_recognized, 2);
        assert_eq!(summary.amount_missing, 0);
        assert_eq!(sink.rows.len(), 2);

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
        let mut sink = RowSink::default();
        let summary = execute_plan(&plans, &mut sink);

        assert_eq!(summary.success, 2);
        let out = dir.path().join("TN");
        let base = out.join("26322000000893295511-Felix-TN.pdf");
        let seq = out.join("26322000000893295511-Felix-TN_1.pdf");
        assert!(base.exists());
        assert!(seq.exists());
    }

    #[test]
    fn execute_empty_plan_returns_zeroed_summary() {
        let plans: Vec<RenamePlan> = Vec::new();
        let mut sink = RowSink::default();
        let s = execute_plan(&plans, &mut sink);
        assert_eq!(s.total, 0);
        assert_eq!(s.success, 0);
        assert_eq!(s.failed, 0);
        assert_eq!(s.total_amount_cents, 0);
        assert_eq!(s.amount_recognized, 0);
        assert_eq!(s.amount_missing, 0);
        assert_eq!(s.output_dir, None);
        assert!(sink.rows.is_empty());
    }

    #[test]
    fn execute_unknown_marks_failed_but_still_copies() {
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        let plans = build_plan(dir.path(), "Felix", "TN", none_extract).unwrap();
        let mut sink = RowSink::default();
        let s = execute_plan(&plans, &mut sink);
        assert_eq!(s.success, 0);
        assert_eq!(s.failed, 1);
        assert_eq!(sink.rows.len(), 1);
        assert_eq!(sink.rows[0].status, "failed");
        assert!(sink.rows[0].note.contains("未识别发票号"));
        assert!(dir.path().join("TN").join("UNKNOWN-Felix-TN.pdf").exists());
        // UNKNOWN 占位但已落地：仍应返回 output_dir，便于用户手工补救
        assert!(s.output_dir.is_some());
    }

    #[test]
    fn execute_aggregates_amounts_independent_of_copy() {
        // 手工构造混合计划：a 有发票号+金额，b 无发票号+无金额
        let dir = TempDir::new().unwrap();
        make_pdf(dir.path(), "a.pdf", "AAA");
        make_pdf(dir.path(), "b.pdf", "BBB");
        let out = dir.path().join("TN");
        let plans = vec![
            RenamePlan {
                source: dir.path().join("a.pdf"),
                target: out.join("INV1-Felix-TN.pdf"),
                invoice_number: Some("INV1".to_string()),
                total_amount_cents: Some(9333),
            },
            RenamePlan {
                source: dir.path().join("b.pdf"),
                target: out.join("UNKNOWN-Felix-TN.pdf"),
                invoice_number: None,
                total_amount_cents: None,
            },
        ];
        let mut sink = RowSink::default();
        let s = execute_plan(&plans, &mut sink);

        assert_eq!(s.total_amount_cents, 9333);
        assert_eq!(s.amount_recognized, 1);
        assert_eq!(s.amount_missing, 1);
        assert_eq!(sink.rows.len(), 2);

        assert_eq!(sink.rows[0].amount_cents, Some(9333));
        assert_eq!(sink.rows[0].amount_display.as_deref(), Some("¥93.33"));
        assert_eq!(sink.rows[0].status, "success");
        assert_eq!(sink.rows[0].index, 1);
        assert_eq!(sink.rows[0].total, 2);

        assert_eq!(sink.rows[1].amount_cents, None);
        assert_eq!(sink.rows[1].amount_display, None);
        assert_eq!(sink.rows[1].status, "failed");
    }
}
