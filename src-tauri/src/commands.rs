use crate::error::AppError;
use crate::pdf_parser::extract_invoice_info;
use crate::renamer::{build_plan, execute_plan, InvoiceRow, ProgressSink, RenameSummary};
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

#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    on_row: Channel<InvoiceRow>,
) -> Result<RenameSummary, AppError> {
    let source = PathBuf::from(&source_dir);
    if !source.is_dir() {
        return Err(AppError::Io(format!("源文件夹不存在：{source_dir}")));
    }

    let summary =
        tauri::async_runtime::spawn_blocking(move || -> Result<RenameSummary, AppError> {
            let plans = build_plan(&source, &user_name, &tracking_number, extract_invoice_info)?;
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
