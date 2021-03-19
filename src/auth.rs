use chrono::{offset::Utc, Duration};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use warp::{Rejection, Reply};

use crate::{
    diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl, RunQueryDsl},
    error::Error,
    model::User,
    schema::{principal::dsl::*, qr_user::dsl::*},
};
use diesel::dsl::exists;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    sub: String,
}

pub async fn login_handler(request: LoginRequest) -> Result<impl Reply, Rejection> {
    let connection = match crate::CONNECTION_POOL.get() {
        Ok(connection) => connection,
        Err(_) => return Err(warp::reject::custom(Error::DatabaseConnectionError)),
    };

    let found_users = qr_user
        .filter(exists(
            principal.filter(
                user_name
                    .eq(&request.user_name)
                    .and(password.eq(&request.password)),
            ),
        ))
        .load::<User>(&connection);

    if let Ok(users) = found_users {
        if users.is_empty() {
            return Err(warp::reject::custom(Error::InvalidCredentials));
        } else if users.len() != 1 {
            panic!(
                "Found multiple users for principal user_name: '{}'",
                &request.user_name
            );
        }
    } else {
        return Err(warp::reject::custom(Error::QueryError));
    };

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(30))
        .expect("Invalid timestamp")
        .timestamp();

    let claims = Claims {
        exp: expiration as usize,
        sub: request.user_name.clone(),
    };

    let header = Header::new(Algorithm::HS512);
    // TODO use private secret
    let token = match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(&34534535_u64.to_be_bytes()),
    ) {
        Ok(token) => token,
        Err(_) => return Err(warp::reject::custom(Error::JwtCreationError)),
    };

    let response = LoginResponse { token };

    Ok(warp::reply::json(&response))
}
