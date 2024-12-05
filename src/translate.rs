use std::fmt::Display;

#[derive(Debug)]
#[allow(unused)]
pub enum ErrorType {
    // Invalid arguments
    InvalidArguments,
    // Missing TENCENT_TRANSLATION_SECRET_ID
    MissingSecretId,
    // Missing TENCENT_TRANSLATION_SECRET_KEY
    MissingSecretKey,
    // Network error
    NetworkError,
    // API parse error
    ApiParseError,
    // others
    Others,
}

pub struct Error {
    error_type: ErrorType,
    message: String,
}

pub trait ErrorCast<T, E: ToString>: Sized + Into<Result<T, E>> {
    fn cast(self, error_type: ErrorType) -> Result<T, Error> {
        self.into().map_err(|e| Error {
            error_type,
            message: e.to_string(),
        })
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error {
            error_type: ErrorType::NetworkError,
            message: err.to_string(),
        }
    }
}

impl<T, E: std::error::Error> ErrorCast<T, E> for Result<T, E> {}

impl Error {
    pub fn new(error_type: ErrorType, message: &str) -> Self {
        Error {
            error_type,
            message: message.to_string(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {:?}, MSG: {}", self.error_type, self.message)
    }
}

pub trait Translate {
    fn translate(&self, src_lang: &str, dst_lang: &str, src: &str, idle: u64) -> Result<String, Error>;
}
