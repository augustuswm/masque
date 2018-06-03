use std::error::Error;
use std::fmt;

use hyper;

#[derive(Debug)]
pub enum MasqueError {
    InvalidChunkBytes,
    InvalidEventTag,
    InvalidHeaderValue,
    InvalidUri,
    StorePoisonedError,
}

impl Error for MasqueError {
    fn description(&self) -> &str {
        ""
    }
}

impl fmt::Display for MasqueError {
    fn fmt(&self, _: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Ok(())
    }
}

impl From<hyper::header::InvalidHeaderValue> for MasqueError {
    fn from(_e: hyper::header::InvalidHeaderValue) -> MasqueError {
        MasqueError::InvalidHeaderValue
    }
}

impl From<::std::str::Utf8Error> for MasqueError {
    fn from(_e: ::std::str::Utf8Error) -> MasqueError {
        MasqueError::InvalidChunkBytes
    }
}
