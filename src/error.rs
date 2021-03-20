use serde::Serialize;
use thiserror::Error;
use warp::{hyper::StatusCode, reject::Reject, Rejection, Reply};

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("Could not establish database connection")]
    DatabaseConnectionError,
    #[error("There has been an error executing a query")]
    QueryError,
    #[error("There has been an error creating the JWT token")]
    JwtCreationError,
    #[error("There has been an error encrypting / decrypting a password")]
    EncryptionError,
    #[error("There already exists a principal with the given identifier: '{0}'")]
    PrincipalExists(String),
}

impl Reject for Error {}

#[derive(Serialize, Debug)]
struct ErrorResponse {
    message: String,
    status: String,
}

pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(e) = err.find::<Error>() {
        let (code, message) = match e {
            Error::InvalidCredentials => (StatusCode::FORBIDDEN, e.to_string()),
            Error::PrincipalExists(_) => (StatusCode::BAD_REQUEST, e.to_string()),
            Error::DatabaseConnectionError
            | Error::QueryError
            | Error::JwtCreationError
            | Error::EncryptionError => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
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
