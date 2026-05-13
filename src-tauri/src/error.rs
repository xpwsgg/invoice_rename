use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(String),
    Pdf(String),
    InvalidInput(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(msg) => write!(f, "IO 错误：{}", msg),
            AppError::Pdf(msg) => write!(f, "PDF 解析错误：{}", msg),
            AppError::InvalidInput(msg) => write!(f, "无效输入：{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_io_error() {
        let err = AppError::Io("permission denied".into());
        assert_eq!(err.to_string(), "IO 错误：permission denied");
    }

    #[test]
    fn display_pdf_error() {
        let err = AppError::Pdf("encrypted".into());
        assert_eq!(err.to_string(), "PDF 解析错误：encrypted");
    }

    #[test]
    fn display_invalid_input_error() {
        let err = AppError::InvalidInput("user_name empty".into());
        assert_eq!(err.to_string(), "无效输入：user_name empty");
    }

    #[test]
    fn serialize_to_string() {
        let err = AppError::Io("disk full".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"IO 错误：disk full\"");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }
}
