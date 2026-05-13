use once_cell::sync::Lazy;
use regex::Regex;

static RE_LABELED: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:发票号码|Invoice\s*Number|Invoice\s*No)[：:.\s]*?(\d{20})").unwrap()
});

static RE_BARE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[^\d])(\d{20})(?:[^\d]|$)").unwrap()
});

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
}
