use serde::Serialize;
use thiserror::Error;
use warp::{hyper::StatusCode, reject::Reject, Rejection, Reply};

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid credentials")]
    InvalidCredentialsError,
    #[error("Could not establish database connection")]
    DatabaseConnectionError,
    #[error("There has been an error executing a query")]
    QueryError,
    #[error("There has been an error creating the JWT token")]
    JwtCreationError,
    #[error("There has been an error encrypting / decrypting a password")]
    EncryptionError,
    #[error("There already exists a principal with the given identifier: '{0}'")]
    PrincipalExistsError(String),
    #[error("Failed to decode request header as valid utf8")]
    UtfEncodingError,
    #[error("The auth header is not formatted correctly (expected JWT 'Bearer ' header)")]
    InvalidAuthHeaderError,
    #[error("No auth header provided")]
    MissingAuthHeaderError,
    #[error("The request is not formatted correctly")]
    BadRequestError,
    #[error("The JWT is not or no longer valid")]
    InvalidJwtError,
    #[error("Failed to serialise data")]
    SerialisationError,
    #[error("The provided refresh token is invalid")]
    InvalidRefreshTokenError,
    #[error("An error occurred when evaluating a python script: '{0}'")]
    PythonError(String),
    #[error("An error occurred while generating a QR-Code: '{0}'")]
    QrCodeError(String),
    #[error("An error occurred while rendering a tera template: '{0}'")]
    TeraError(String),
    #[error("An error occurred while building a pdf file: '{0}'")]
    PdfError(String),
    #[error("An IO error occurred: '{0}'")]
    IoError(String),
    #[error("The request input could not be validated: '{0}'")]
    InvalidRequestInputError(String),
}

impl Reject for Error {}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

/// Creates a Rejection response for the given error and logs internal server errors.
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(e) = err.find::<Error>() {
        let (code, message) = match e {
            Error::InvalidCredentialsError => (StatusCode::FORBIDDEN, e.to_string()),
            Error::MissingAuthHeaderError
            | Error::InvalidJwtError
            | Error::InvalidRefreshTokenError => (StatusCode::UNAUTHORIZED, e.to_string()),
            Error::PrincipalExistsError(_)
            | Error::UtfEncodingError
            | Error::InvalidAuthHeaderError
            | Error::BadRequestError
            | Error::InvalidRequestInputError(_) => (StatusCode::BAD_REQUEST, e.to_string()),
            Error::DatabaseConnectionError
            | Error::QueryError
            | Error::JwtCreationError
            | Error::EncryptionError
            | Error::SerialisationError
            | Error::PythonError(_)
            | Error::QrCodeError(_)
            | Error::TeraError(_)
            | Error::PdfError(_)
            | Error::IoError(_) => {
                log::error!("Encountered internal server error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        };

        let err_response = ErrorResponse {
            message,
            status: code.to_string(),
        };

        let json = warp::reply::json(&err_response);

        Ok(warp::reply::with_status(json, code))
    } else {
        return Err(err);
    }
}
