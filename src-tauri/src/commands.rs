use crate::error::AppError;
use crate::pdf_parser::extract_invoice_number;
use crate::renamer::{build_plan, execute_plan, LogEntry, Logger, RenameSummary};
use std::path::PathBuf;
use tauri::ipc::Channel;

struct ChannelLogger {
    channel: Channel<LogEntry>,
}

impl Logger for ChannelLogger {
    fn log(&mut self, entry: LogEntry) {
        let _ = self.channel.send(entry);
    }
}

#[tauri::command]
pub async fn rename_pdfs(
    source_dir: String,
    user_name: String,
    tracking_number: String,
    on_log: Channel<LogEntry>,
) -> Result<RenameSummary, AppError> {
    let source = PathBuf::from(&source_dir);
    if !source.is_dir() {
        return Err(AppError::Io(format!("源文件夹不存在：{source_dir}")));
    }

    let summary = tauri::async_runtime::spawn_blocking(move || -> Result<RenameSummary, AppError> {
        let plans = build_plan(&source, &user_name, &tracking_number, extract_invoice_number)?;
        let mut logger = ChannelLogger { channel: on_log };
        Ok(execute_plan(&plans, &mut logger))
    })
    .await
    .map_err(|e| AppError::Io(format!("任务调度失败：{e}")))??;

    Ok(summary)
}
