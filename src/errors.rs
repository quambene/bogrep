use reqwest::header::ToStrError;
use std::{io, string::FromUtf8Error};
use thiserror::Error;
use url::ParseError;

#[derive(Debug, Error)]
pub enum BogrepError {
    #[error("Can't serialize json: {}", 0.to_string())]
    SerializeJson(serde_json::Error),
    #[error("Can't deserialize json: {}", 0.to_string())]
    DeserializeJson(serde_json::Error),
    #[error("Can't parse url")]
    ParseUrl(#[from] ParseError),
    #[error("Can't parse url: {}", 0.to_string())]
    ConvertHtml(readability::error::Error),
    #[error("Can't get host for url: {0}")]
    ConvertHost(String),
    #[error("Invalid utf8: {0}")]
    ConvertUtf8(#[from] FromUtf8Error),
    #[error("Can't read from HTML: {0}")]
    ReadHtml(io::Error),
    #[error("Can't serialize HTML: {0}")]
    SerializeHtml(io::Error),
    #[error("Can't create file at {path}: {err}")]
    CreateFile { path: String, err: String },
    #[error("Can't open file at {path}: {err}")]
    OpenFile { path: String, err: String },
    #[error("Can't remove file at {path}: {err}")]
    RemoveFile { path: String, err: String },
    #[error("Can't read file: {0}")]
    ReadFile(String),
    #[error("Can't write to file at {path}: {err}")]
    WriteFile { path: String, err: String },
    #[error("Can't append file at {path}: {err}")]
    AppendFile { path: String, err: String },
    #[error("Can't rename file from {from} to {to}: {err}")]
    RenameFile {
        from: String,
        to: String,
        err: String,
    },
    #[error("Can't flush file: {0}")]
    FlushFile(io::Error),
    #[error("Can't rewind file: {0}")]
    RewindFile(String),
    #[error("Can't create client: {}", 0.to_string())]
    CreateClient(reqwest::Error),
    #[error("Can't fetch website: {}", 0.to_string())]
    FetchError(reqwest::Error),
    #[error("Can't fetch website: {}", 0.to_string())]
    HttpError(reqwest::Error),
    #[error("Can't convert header to string: {0}")]
    ConvertToStr(#[from] ToStrError),
    #[error("Can't remove website ({url}) from cache: {err}")]
    RemoveCache { url: String, err: tokio::io::Error },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
