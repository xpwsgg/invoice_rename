use crate::error::AppError;
use crate::pdf_parser::extract_invoice_info;
use crate::renamer::{build_plan_for_files, execute_plan, format_amount, InvoiceRow, ProgressSink, RenameSummary};
use serde::Serialize;
use std::path::PathBuf;
use tauri::ipc::Channel;

struct ChannelSink {
    channel: Channel<InvoiceRow>,
}

impl ProgressSink for ChannelSink {
    fn row(&mut self, row: InvoiceRow) {
        let _ = self.channel.send(row);
    }
}

/// 扫描结果中的单个文件信息
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFile {
    pub index: usize,
    pub source_name: String,
    pub invoice_number: Option<String>,
    pub amount_cents: Option<i64>,
    pub amount_display: Option<String>,
    pub parse_error: Option<String>,
}

/// 扫描文件夹的结果摘要
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub total: usize,
    pub total_amount_cents: i64,
    pub amount_recognized: usize,
    pub amount_missing: usize,
    pub parse_errors: usize,
    pub files: Vec<ScannedFile>,
}

#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    file_names: Vec<String>, // 新增参数：要处理的文件名列表
    on_row: Channel<InvoiceRow>,
) -> Result<RenameSummary, AppError> {
    let source = PathBuf::from(&source_dir);
    if !source.is_dir() {
        return Err(AppError::Io(format!("源文件夹不存在：{source_dir}")));
    }

    let summary =
        tauri::async_runtime::spawn_blocking(move || -> Result<RenameSummary, AppError> {
            let plans = build_plan_for_files(&source, &user_name, &tracking_number, &file_names, extract_invoice_info)?;
            // 全局性失败：输出目录无法创建 → 整体报错，前端在表单错误区提示。
            if let Some(out_dir) = plans.first().and_then(|p| p.target.parent()) {
                std::fs::create_dir_all(out_dir).map_err(|e| {
                    AppError::Io(format!("创建输出目录失败：{} ({e})", out_dir.display()))
                })?;
            }
            let mut sink = ChannelSink { channel: on_row };
            Ok(execute_plan(&plans, &mut sink))
        })
        .await
        .map_err(|e| AppError::Io(format!("任务调度失败：{e}")))??;

    Ok(summary)
}

/// 扫描源文件夹，返回所有 PDF 文件的信息和总金额
#[tauri::command]
pub async fn scan_pdfs(source_dir: String) -> Result<ScanSummary, AppError> {
    let source = PathBuf::from(&source_dir);
    if !source.is_dir() {
        return Err(AppError::Io(format!("源文件夹不存在：{source_dir}")));
    }

    let summary = tauri::async_runtime::spawn_blocking(move || -> Result<ScanSummary, AppError> {
        let mut files = Vec::new();
        let mut total_amount_cents = 0i64;
        let mut amount_recognized = 0usize;
        let mut amount_missing = 0usize;
        let mut parse_errors = 0usize;

        // 收集所有 PDF 文件
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&source)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && is_pdf(&path) {
                entries.push(path);
            }
        }

        // 按文件名排序
        entries.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        // 解析每个 PDF
        for (idx, path) in entries.iter().enumerate() {
            let source_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let (invoice_number, amount_cents, parse_error) = match extract_invoice_info(&path) {
                Ok(info) => (info.number, info.total_amount_cents, None),
                Err(e) => (None, None, Some(e.to_string())),
            };

            if parse_error.is_some() {
                parse_errors += 1;
            }

            match amount_cents {
                Some(cents) => {
                    total_amount_cents += cents;
                    amount_recognized += 1;
                }
                None => {
                    amount_missing += 1;
                }
            }

            files.push(ScannedFile {
                index: idx + 1,
                source_name,
                invoice_number,
                amount_cents,
                amount_display: amount_cents.map(format_amount),
                parse_error,
            });
        }

        Ok(ScanSummary {
            total: files.len(),
            total_amount_cents,
            amount_recognized,
            amount_missing,
            parse_errors,
            files,
        })
    })
    .await
    .map_err(|e| AppError::Io(format!("任务调度失败：{e}")))??;

    Ok(summary)
}

fn is_pdf(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

#[tauri::command]
pub fn open_folder(path: String) -> Result<(), AppError> {
    let target = PathBuf::from(&path);
    if !target.is_dir() {
        return Err(AppError::Io(format!("路径不存在或不是文件夹：{path}")));
    }

    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");

    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("explorer");

    #[cfg(target_os = "linux")]
    let mut cmd = std::process::Command::new("xdg-open");

    cmd.arg(&target)
        .spawn()
        .map_err(|e| AppError::Io(format!("打开文件夹失败：{e}")))?;
    Ok(())
}
