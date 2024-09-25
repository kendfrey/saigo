use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
};

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use confy::ConfyError;
use image::ImageError;
use nokhwa::NokhwaError;

/// The errors that can be returned by the API.
#[derive(Debug)]
pub enum SaigoError {
    InvalidProfileName(String),
    NonexistentProfile(String),
    Confy(ConfyError),
    Image(ImageError),
    IO(io::Error),
    Nokhwa(NokhwaError),
}

impl Display for SaigoError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SaigoError::InvalidProfileName(profile) => {
                write!(f, "'{}' is not a valid profile name.", profile)
            }
            SaigoError::NonexistentProfile(profile) => {
                write!(f, "Profile '{}' does not exist.", profile)
            }
            SaigoError::Confy(error) => write!(f, "Confy error: {}", error),
            SaigoError::Image(error) => write!(f, "Image error: {}", error),
            SaigoError::IO(error) => write!(f, "IO error: {}", error),
            SaigoError::Nokhwa(error) => write!(f, "Nokhwa error: {}", error),
        }
    }
}

impl Error for SaigoError {}

impl From<ConfyError> for SaigoError {
    fn from(error: ConfyError) -> Self {
        SaigoError::Confy(error)
    }
}

impl From<ImageError> for SaigoError {
    fn from(error: ImageError) -> Self {
        SaigoError::Image(error)
    }
}

impl From<io::Error> for SaigoError {
    fn from(error: io::Error) -> Self {
        SaigoError::IO(error)
    }
}

impl From<NokhwaError> for SaigoError {
    fn from(error: NokhwaError) -> Self {
        SaigoError::Nokhwa(error)
    }
}

impl IntoResponse for SaigoError {
    fn into_response(self) -> Response<Body> {
        let status = match self {
            SaigoError::InvalidProfileName(_) => StatusCode::BAD_REQUEST,
            SaigoError::NonexistentProfile(_) => StatusCode::BAD_REQUEST,
            SaigoError::Confy(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SaigoError::Image(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SaigoError::IO(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SaigoError::Nokhwa(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }
}
