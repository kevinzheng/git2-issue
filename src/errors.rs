use std::io;

use log::{error, info, trace, warn};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Internal server error, `{0}`")]
    InternalServerError(String),
    #[error("Invalid input:\n`{0}`")]
    InvalidInputError(String),
    #[error("Bad request:\n`{0}`")]
    BadRequestError(String),
    #[error("Failed to execute command, error:\n`{0}`")]
    CommandError(String),
    #[error("IO error")]
    IOError(#[from] io::Error),
    #[error("FromUtf8Error")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("Git2Error")]
    Git2Error(#[from] git2::Error),
    #[error("Unknown data store error")]
    Unknown,
}
