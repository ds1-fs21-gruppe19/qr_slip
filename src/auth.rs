use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{offset::Utc, Duration};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use warp::{Rejection, Reply};

use crate::{
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    error::Error,
    model::{NewPrincipal, NewUser, Principal, User},
    schema::{principal, qr_user},
};
use diesel::dsl::count;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
}

#[derive(Deserialize)]
pub struct UserRegistration {
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    pub user_name: String,
    pub password: String,
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

    let found_principal = principal::table
        .filter(principal::user_name.eq(&request.user_name))
        .load::<Principal>(&connection);
    match found_principal {
        Ok(principals) => {
            if principals.is_empty() {
                return Err(warp::reject::custom(Error::InvalidCredentials));
            } else if principals.len() == 1 {
                let hashed_password = &principals[0].password;
                match verify(&request.password, hashed_password) {
                    Ok(valid) => {
                        if !valid {
                            return Err(warp::reject::custom(Error::InvalidCredentials));
                        }
                    }
                    Err(_) => return Err(warp::reject::custom(Error::EncryptionError)),
                }
            } else {
                panic!(
                    "Found multiple principal entries for user_name: '{}'",
                    &request.user_name
                );
            }
        }
        Err(_) => return Err(warp::reject::custom(Error::QueryError)),
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
    let token = match encode(
        &header,
        &claims,
        &EncodingKey::from_secret(&crate::JWT_SECRET.to_be_bytes()),
    ) {
        Ok(token) => token,
        Err(_) => return Err(warp::reject::custom(Error::JwtCreationError)),
    };

    let response = LoginResponse { token };

    Ok(warp::reply::json(&response))
}

pub async fn register_handler(
    user_registration: UserRegistration,
) -> Result<impl Reply, Rejection> {
    let connection = match crate::CONNECTION_POOL.get() {
        Ok(connection) => connection,
        Err(_) => return Err(warp::reject::custom(Error::DatabaseConnectionError)),
    };

    let existing_count: Result<i64, _> = principal::table
        .select(count(principal::pk))
        .filter(principal::user_name.eq(&user_registration.user_name))
        .first(&connection);

    match existing_count {
        Ok(count) => {
            if count != 0 {
                return Err(warp::reject::custom(Error::PrincipalExists(
                    user_registration.user_name,
                )));
            }
        }
        Err(_) => return Err(warp::reject::custom(Error::QueryError)),
    };

    let hashed_password = match hash(&user_registration.password, DEFAULT_COST) {
        Ok(hashed_password) => hashed_password,
        Err(_) => return Err(warp::reject::custom(Error::EncryptionError)),
    };

    let new_principal = NewPrincipal {
        user_name: user_registration.user_name,
        password: hashed_password,
    };

    let principal = match diesel::insert_into(principal::table)
        .values(&new_principal)
        .get_result::<Principal>(&connection)
    {
        Ok(principal) => principal,
        Err(_) => return Err(warp::reject::custom(Error::QueryError)),
    };

    let new_user = NewUser {
        first_name: user_registration.first_name,
        last_name: user_registration.last_name,
        address: user_registration.address,
        zip_code: user_registration.zip_code,
        city: user_registration.city,
        iban: user_registration.iban,
        country: user_registration.country,
        fk_principal: principal.pk,
    };

    if diesel::insert_into(qr_user::table)
        .values(&new_user)
        .get_result::<User>(&connection)
        .is_err()
    {
        return Err(warp::reject::custom(Error::QueryError));
    }

    Ok(warp::reply::reply())
}
