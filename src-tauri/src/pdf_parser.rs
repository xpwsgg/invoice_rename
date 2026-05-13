use crate::error::AppError;
use once_cell::sync::Lazy;
use pdfium_render::prelude::*;
use regex::Regex;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

static RE_LABELED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})").unwrap()
});

static RE_BARE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[^\d])(\d{20})(?:[^\d]|$)").unwrap()
});

thread_local! {
    static PDFIUM_LOCAL: RefCell<Option<Pdfium>> = RefCell::new(None);
}

fn make_pdfium() -> Result<Pdfium, AppError> {
    let local_dir = locate_lib_dir();
    let bindings = match local_dir.as_ref().and_then(|p| {
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(p)).ok()
    }) {
        Some(b) => b,
        None => Pdfium::bind_to_system_library()
            .map_err(|e| AppError::Pdf(format!("加载 PDFium 库失败：{e}")))?,
    };
    Ok(Pdfium::new(bindings))
}

fn locate_lib_dir() -> Option<PathBuf> {
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");
    if dev.join("libpdfium.dylib").exists() {
        return Some(dev);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for cand in [parent.join("lib"), parent.join("../Resources/lib")] {
                if cand.join("libpdfium.dylib").exists() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// 在已抽取的 PDF 全文中寻找 20 位发票号码。
/// 优先匹配带"发票号码"等字段名的，找不到时回退到"独立 20 位数字"。
pub fn find_invoice_number_in_text(text: &str) -> Option<String> {
    if let Some(caps) = RE_LABELED.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    if let Some(caps) = RE_BARE.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    None
}

/// 打开 PDF 并尝试提取发票号码。
/// - Ok(Some) : 成功匹配到 20 位号码
/// - Ok(None) : PDF 能打开但无法匹配
/// - Err     : PDF 打不开 / 加密 / 损坏
pub fn extract_invoice_number(pdf_path: &Path) -> Result<Option<String>, AppError> {
    PDFIUM_LOCAL.with(|cell| -> Result<Option<String>, AppError> {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            *opt = Some(make_pdfium()?);
        }
        let pdfium = opt.as_ref().unwrap();

        let document = pdfium
            .load_pdf_from_file(pdf_path, None)
            .map_err(|e| AppError::Pdf(format!("打开 PDF 失败：{e}")))?;

        let mut buf = String::new();
        for page in document.pages().iter() {
            let text = page
                .text()
                .map_err(|e| AppError::Pdf(format!("提取文本失败：{e}")))?;
            buf.push_str(&text.all());
            buf.push('\n');
        }
        Ok(find_invoice_number_in_text(&buf))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labeled_chinese() {
        let text = "发票代码: 011002000111\n发票号码: 26322000000893295511\n开票日期";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_chinese_with_colon_variant() {
        let text = "发票号码：26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_english_invoice_number() {
        let text = "Invoice Number: 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn labeled_english_invoice_no() {
        let text = "Invoice No 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn bare_fallback() {
        let text = "杂项文本 26322000000893295511 杂项";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn rejects_21_digit_run() {
        let text = "1234567890123456789012";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn returns_none_when_no_match() {
        let text = "这只是一段普通文本，没有任何 20 位数字";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn prefers_labeled_over_bare() {
        let text = "11111111111111111111\n发票号码: 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn extract_from_sample_pdf_if_present() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sample.pdf");
        if !path.exists() {
            eprintln!("跳过：未找到 {}", path.display());
            return;
        }
        let result = extract_invoice_number(&path).expect("PDF 解析不应失败");
        assert!(result.is_some(), "应该能从样本 PDF 提取到发票号码");
        let num = result.unwrap();
        assert_eq!(num.len(), 20);
        assert!(num.chars().all(|c| c.is_ascii_digit()));
    }
}
