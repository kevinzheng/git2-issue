use std::io;

use actix_threadpool;
use actix_web::error;
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
    #[error("ValidationErrors")]
    ValidationErrors(#[from] validator::ValidationErrors),
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

impl Into<actix_web::Error> for AppError {
    fn into(self) -> actix_web::Error {
        match self {
            AppError::InternalServerError(e) => {
                println!("Interval server error: {}", e);
                error::ErrorInternalServerError("Interval server error!")
            }
            AppError::InvalidInputError(e) | AppError::BadRequestError(e) => {
                error::ErrorBadRequest(e)
            }
            AppError::ValidationErrors(e) => error::ErrorBadRequest(e),
            AppError::CommandError(message) => {
                println!("Interval server error: {}", message);
                error::ErrorInternalServerError("Interval server error!")
            }
            AppError::IOError(e) => {
                println!("Interval server error: {}", e.to_string());
                error::ErrorInternalServerError("Interval server error!")
            }
            AppError::FromUtf8Error(e) => {
                println!("Interval server error: {}", e.to_string());
                error::ErrorInternalServerError("Interval server error!")
            }
            AppError::Git2Error(e) => {
                println!("Interval server error: {}", e.message());
                error::ErrorInternalServerError("Interval server error!")
            }
            AppError::Unknown => {
                println!("Interval server error: Unknown");
                error::ErrorInternalServerError("Interval server error!")
            }
        }
    }
}
