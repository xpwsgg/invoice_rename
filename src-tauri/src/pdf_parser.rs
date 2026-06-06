use crate::error::AppError;
use once_cell::sync::Lazy;
use pdfium_render::prelude::*;
use regex::Regex;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

static RE_LABELED_20: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})(?:[^\d]|$)").unwrap()
});

static RE_LABELED_8: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{8})(?:[^\d]|$)").unwrap()
});

static RE_BARE_20: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:^|[^\d])(\d{20})(?:[^\d]|$)").unwrap());

// 价税合计（含税总额）小写金额。
// 实测：pdfium 提取顺序下「(小写)」标签与金额并不相邻，但小写金额始终紧跟
// 在「中文大写金额」之后，如「玖拾叁圆叁角叁分 ¥93.33」。以此为主锚点。
static RE_AMOUNT_CN_UPPER: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[零壹贰叁肆伍陆柒捌玖拾佰仟万亿圆元角分整正][\s)）]*[¥￥]\s*([0-9,]+\.[0-9]{2})")
        .unwrap()
});

// 回退锚点：文档中所有 ¥ 金额取最大值。
// 价税合计 = 金额合计 + 税额，恒为发票内最大的 ¥ 金额。
static RE_AMOUNT_ANY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[¥￥]\s*([0-9,]+\.[0-9]{2})").unwrap());

thread_local! {
    static PDFIUM_LOCAL: RefCell<Option<Pdfium>> = const { RefCell::new(None) };
}

fn make_pdfium() -> Result<Pdfium, AppError> {
    let local_dir = locate_lib_dir();
    let bindings = match local_dir
        .as_ref()
        .and_then(|p| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(p)).ok())
    {
        Some(b) => b,
        None => Pdfium::bind_to_system_library()
            .map_err(|e| AppError::Pdf(format!("加载 PDFium 库失败：{e}")))?,
    };
    Ok(Pdfium::new(bindings))
}

#[cfg(windows)]
const EMBEDDED_PDFIUM_DLL: &[u8] = include_bytes!("../lib/pdfium.dll");

#[cfg(windows)]
fn ensure_embedded_pdfium() -> Option<PathBuf> {
    let dir = std::env::temp_dir().join("esi_invoice_rename");
    std::fs::create_dir_all(&dir).ok()?;
    let dll = dir.join("pdfium.dll");
    let needs_write = match std::fs::metadata(&dll) {
        Ok(m) => m.len() as usize != EMBEDDED_PDFIUM_DLL.len(),
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&dll, EMBEDDED_PDFIUM_DLL).ok()?;
    }
    Some(dir)
}

fn locate_lib_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(dir) = ensure_embedded_pdfium() {
        return Some(dir);
    }

    let lib_name = Pdfium::pdfium_platform_library_name();
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lib");
    if dev.join(&lib_name).exists() {
        return Some(dev);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for cand in [
                parent.to_path_buf(),
                parent.join("lib"),
                parent.join("../Resources/lib"),
            ] {
                if cand.join(&lib_name).exists() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// 在已抽取的 PDF 全文中寻找发票号码。
/// 匹配优先级：带标签20位 > 带标签8位 > 裸20位。
pub fn find_invoice_number_in_text(text: &str) -> Option<String> {
    if let Some(caps) = RE_LABELED_20.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    if let Some(caps) = RE_LABELED_8.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    if let Some(caps) = RE_BARE_20.captures(text) {
        return Some(caps.get(1).unwrap().as_str().to_string());
    }
    None
}

/// 把金额字符串（可能含千分位、恰两位小数）转为「分」。
fn amount_str_to_cents(s: &str) -> Option<i64> {
    let s = s.replace(',', "");
    let (int_part, frac_part) = s.split_once('.')?;
    let int_val: i64 = int_part.parse().ok()?;
    let frac_val: i64 = frac_part.parse().ok()?;
    Some(int_val * 100 + frac_val)
}

/// 从已抽取的全文中寻找价税合计（含税总额），返回「分」。
/// 策略：优先「中文大写金额 + ¥小写」锚点（语义最明确）；
/// 未命中时回退为「文档中最大的 ¥ 金额」。
pub fn find_total_amount_in_text(text: &str) -> Option<i64> {
    if let Some(caps) = RE_AMOUNT_CN_UPPER.captures(text) {
        if let Some(cents) = amount_str_to_cents(&caps[1]) {
            return Some(cents);
        }
    }
    RE_AMOUNT_ANY
        .captures_iter(text)
        .filter_map(|c| amount_str_to_cents(&c[1]))
        .max()
}

/// 打开 PDF 并提取全部页面文本（合并为一个字符串）。
/// 复用线程局部的 Pdfium 实例，供发票号与金额解析共用同一份全文，避免重复打开。
fn extract_all_text(pdf_path: &Path) -> Result<String, AppError> {
    PDFIUM_LOCAL.with(|cell| -> Result<String, AppError> {
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
        Ok(buf)
    })
}

/// 一次解析得到的发票关键字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceInfo {
    pub number: Option<String>,
    pub total_amount_cents: Option<i64>,
}

/// 打开 PDF 一次，从同一份全文中同时解析发票号与价税合计，避免重复打开。
pub fn extract_invoice_info(pdf_path: &Path) -> Result<InvoiceInfo, AppError> {
    let text = extract_all_text(pdf_path)?;
    Ok(InvoiceInfo {
        number: find_invoice_number_in_text(&text),
        total_amount_cents: find_total_amount_in_text(&text),
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
    fn labeled_8_digit_chinese() {
        let text = "发票代码: 012002100311\n发票号码: 07765230\n开票日期";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("07765230".to_string())
        );
    }

    #[test]
    fn labeled_8_digit_chinese_full_width_colon() {
        let text = "发票号码：07765230";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("07765230".to_string())
        );
    }

    #[test]
    fn labeled_8_digit_english() {
        let text = "Invoice Number: 07765230";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("07765230".to_string())
        );
    }

    #[test]
    fn prefers_20_digit_over_8_digit() {
        let text = "发票号码: 07765230\n发票号码: 26322000000893295511";
        assert_eq!(
            find_invoice_number_in_text(text),
            Some("26322000000893295511".to_string())
        );
    }

    #[test]
    fn rejects_bare_8_digit() {
        let text = "杂项文本 07765230 杂项";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn rejects_9_digit_labeled() {
        let text = "发票号码: 123456789";
        assert_eq!(find_invoice_number_in_text(text), None);
    }

    #[test]
    fn extract_from_sample_pdf_if_present() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
        if !path.exists() {
            eprintln!("跳过：未找到 {}", path.display());
            return;
        }
        let result = extract_invoice_info(&path)
            .expect("PDF 解析不应失败")
            .number;
        assert!(result.is_some(), "应该能从样本 PDF 提取到发票号码");
        let num = result.unwrap();
        assert!(
            num.len() == 20 || num.len() == 8,
            "发票号码应为 20 位或 8 位，实际 {} 位",
            num.len()
        );
        assert!(num.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn amount_str_to_cents_basic() {
        assert_eq!(amount_str_to_cents("93.33"), Some(9333));
        assert_eq!(amount_str_to_cents("12,345.67"), Some(1234567));
        assert_eq!(amount_str_to_cents("0.00"), Some(0));
        assert_eq!(amount_str_to_cents("439.60"), Some(43960));
    }

    #[test]
    fn amount_cn_upper_anchor_halfwidth() {
        // pdfium 实测片段：价税合计小写紧跟中文大写金额
        let text = "玖拾叁圆叁角叁分 ¥93.33\n订单号:3445258004409231";
        assert_eq!(find_total_amount_in_text(text), Some(9333));
    }

    #[test]
    fn amount_cn_upper_anchor_with_zheng() {
        let text = "肆拾捌圆壹角整 ¥48.10";
        assert_eq!(find_total_amount_in_text(text), Some(4810));
    }

    #[test]
    fn amount_prefers_upper_anchor_over_larger_bare() {
        // 即使后面出现更大的裸 ¥ 金额，也应优先大写锚点指向的价税合计
        let text = "贰拾圆整 ¥20.00\n其它 ¥99.99";
        assert_eq!(find_total_amount_in_text(text), Some(2000));
    }

    #[test]
    fn amount_fallback_to_max_without_upper_anchor() {
        // 无大写锚点时回退为最大 ¥ 金额（金额合计 / 税额 / 价税合计）
        let text = "¥82.59 ¥10.74 ¥93.33";
        assert_eq!(find_total_amount_in_text(text), Some(9333));
    }

    #[test]
    fn amount_thousands_separator() {
        let text = "壹万贰仟叁佰肆拾伍圆陆角柒分 ¥12,345.67";
        assert_eq!(find_total_amount_in_text(text), Some(1234567));
    }

    #[test]
    fn amount_none_when_absent() {
        // 无 ¥ 符号的裸数字（单价/数量）不应被误当金额
        assert_eq!(find_total_amount_in_text("没有任何金额信息"), None);
        assert_eq!(find_total_amount_in_text("单价 68.94 数量 1"), None);
    }

    #[test]
    fn extract_invoice_info_sum_matches_sample_batch() {
        // 端到端回归（fixture-gated）：本地有 13 张样本时验证总和 = ¥983.01
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/invoices_000-116-539");
        if !dir.is_dir() {
            eprintln!("跳过：未找到样本目录 {}", dir.display());
            return;
        }
        let mut sum = 0i64;
        let mut count = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let p = entry.unwrap().path();
            if p.extension().and_then(|e| e.to_str()) != Some("pdf") {
                continue;
            }
            let info = extract_invoice_info(&p).expect("解析发票");
            assert!(info.number.is_some(), "{:?} 应解析到发票号", p.file_name());
            if let Some(c) = info.total_amount_cents {
                sum += c;
                count += 1;
            }
        }
        assert_eq!(count, 13, "应解析到 13 张发票的金额");
        assert_eq!(sum, 98301, "价税合计总和应为 ¥983.01");
    }
}
