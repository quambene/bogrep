use reqwest::header::ToStrError;
use std::{io, string::FromUtf8Error};
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum BogrepError {
    #[error("Can't create client: {0}")]
    CreateClient(reqwest::Error),
    #[error("Can't fetch website: {0}")]
    HttpResponse(reqwest::Error),
    #[error("Invalid status code ({url}): {status}")]
    HttpStatus { status: String, url: String },
    #[error("Can't fetch website: {0}")]
    ParseHttpResponse(reqwest::Error),
    #[error("Can't fetch binary bookmark ({0})")]
    BinaryResponse(String),
    #[error("Can't fetch empty bookmark ({0})")]
    EmptyResponse(String),
    #[error("Can't get host for url: {0}")]
    ConvertHost(String),
    #[error("Can't serialize json: {0}")]
    SerializeJson(serde_json::Error),
    #[error("Can't deserialize json: {0}")]
    DeserializeJson(serde_json::Error),
    #[error("Can't parse url")]
    ParseUrl(#[from] ParseError),
    #[error("Can't parse html")]
    ParseHtml(String),
    #[error("Can't convert html: {0}")]
    ConvertHtml(readability::error::Error),
    #[error("Invalid utf8: {0}")]
    ConvertUtf8(#[from] FromUtf8Error),
    #[error("Can't read from HTML: {0}")]
    ReadHtml(io::Error),
    #[error("Can't serialize HTML: {0}")]
    SerializeHtml(io::Error),
    #[error("Can't create file at {path}: {err}")]
    CreateFile { path: String, err: io::Error },
    #[error("Can't open file at {path}: {err}")]
    OpenFile { path: String, err: io::Error },
    #[error("Can't remove file at {path}: {err}")]
    RemoveFile { path: String, err: io::Error },
    #[error("Can't read file: {0}")]
    ReadFile(io::Error),
    #[error("Can't write: {0}")]
    WriteFile(io::Error),
    #[error("Can't write to file at {path}: {err}")]
    WriteFilePath { path: String, err: io::Error },
    #[error("Can't append file at {path}: {err}")]
    AppendFile { path: String, err: io::Error },
    #[error("Can't rename file from {from} to {to}: {err}")]
    RenameFile {
        from: String,
        to: String,
        err: io::Error,
    },
    #[error("Can't flush file: {0}")]
    FlushFile(io::Error),
    #[error("Can't rewind file: {0}")]
    RewindFile(io::Error),
    #[error("Can't convert header to string: {0}")]
    ConvertToStr(#[from] ToStrError),
    #[error("Can't remove website ({url}) from cache: {err}")]
    RemoveCache { url: String, err: tokio::io::Error },
    #[error("Invalid input")]
    InvalidInput,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
