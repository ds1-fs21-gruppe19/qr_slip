use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{offset::Utc, Duration};
use exec_rs::sync::MutexSync;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use warp::{
    filters::header::headers_cloned,
    http::header::{self, HeaderMap},
    Filter, Rejection, Reply,
};

use crate::{
    acquire_db_connection,
    diesel::{ExpressionMethods, QueryDsl, RunQueryDsl},
    error::Error,
    model::{NewPrincipal, NewUser, Principal, User},
    schema::{principal, qr_user},
};
use diesel::{dsl::count, expression::dsl::any, expression_methods::BoolExpressionMethods};

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
    pub first_name: Option<String>,
    pub last_name: Option<String>,
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

pub fn with_principal_optional(
) -> impl warp::Filter<Extract = (Option<Principal>,), Error = Rejection> + Clone {
    headers_cloned().and_then(get_principal_from_auth_header)
}

pub fn with_principal() -> impl warp::Filter<Extract = (Principal,), Error = Rejection> + Clone {
    headers_cloned().and_then(require_principal_from_auth_header)
}

async fn require_principal_from_auth_header(header_map: HeaderMap) -> Result<Principal, Rejection> {
    match get_principal_from_auth_header(header_map).await {
        Ok(Some(principal)) => Ok(principal),
        Ok(None) => Err(warp::reject::custom(Error::MissingAuthHeaderError)),
        Err(e) => Err(e),
    }
}

async fn get_principal_from_auth_header(
    header_map: HeaderMap,
) -> Result<Option<Principal>, Rejection> {
    const JWT_BEARER_PREFIX: &str = "Bearer ";
    let auth_header = match header_map.get(header::AUTHORIZATION) {
        Some(h) => match std::str::from_utf8(h.as_bytes()) {
            Ok(v) => v,
            Err(_) => return Err(warp::reject::custom(Error::UtfEncodingError)),
        },
        None => return Ok(None),
    };

    if !auth_header.starts_with(JWT_BEARER_PREFIX) {
        return Err(warp::reject::custom(Error::InvalidAuthHeaderError));
    }

    let jwt_token = auth_header.trim_start_matches(JWT_BEARER_PREFIX);
    let token_data = decode::<Claims>(
        jwt_token,
        &DecodingKey::from_secret(&crate::JWT_SECRET.to_be_bytes()),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|_| warp::reject::custom(Error::InvalidJwtError))?;
    let claims = &token_data.claims;

    let connection = acquire_db_connection()?;
    match principal::table
        .filter(principal::user_name.eq(&claims.sub))
        .first::<Principal>(&connection)
    {
        Ok(principal) => Ok(Some(principal)),
        Err(_) => Err(warp::reject::custom(Error::QueryError)),
    }
}

pub async fn login_handler(request: LoginRequest) -> Result<impl Reply, Rejection> {
    let connection = acquire_db_connection()?;

    let found_principal = principal::table
        .filter(principal::user_name.eq(&request.user_name))
        .first::<Principal>(&connection);
    let principal = match found_principal {
        Ok(principal) => {
            let hashed_password = &principal.password;
            match verify(&request.password, hashed_password) {
                Ok(valid) => {
                    if valid {
                        principal
                    } else {
                        return Err(warp::reject::custom(Error::InvalidCredentialsError));
                    }
                }
                Err(_) => return Err(warp::reject::custom(Error::EncryptionError)),
            }
        }
        Err(diesel::NotFound) => return Err(warp::reject::custom(Error::InvalidCredentialsError)),
        Err(_) => return Err(warp::reject::custom(Error::QueryError)),
    };

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(30))
        .expect("Invalid timestamp")
        .timestamp();

    let claims = Claims {
        exp: expiration as usize,
        sub: principal.user_name.clone(),
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

lazy_static! {
    static ref USER_NAME_SYNC: MutexSync<String> = MutexSync::new();
}

pub async fn register_handler(
    user_registration: UserRegistration,
) -> Result<impl Reply, Rejection> {
    // synchronise principal creation based on user_name
    USER_NAME_SYNC.evaluate(user_registration.user_name.clone(), || {
        let connection = acquire_db_connection()?;

        let existing_count: Result<i64, _> = principal::table
            .select(count(principal::pk))
            .filter(principal::user_name.eq(&user_registration.user_name))
            .first(&connection);

        match existing_count {
            Ok(count) => {
                if count != 0 {
                    return Err(warp::reject::custom(Error::PrincipalExistsError(
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
    })
}

#[derive(Deserialize)]
pub struct CreateUser {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
}

pub async fn create_user_handler(
    principal: Principal,
    create_user: CreateUser,
) -> Result<impl Reply, Rejection> {
    let connection = acquire_db_connection()?;

    let new_user = NewUser {
        first_name: create_user.first_name,
        last_name: create_user.last_name,
        address: create_user.address,
        zip_code: create_user.zip_code,
        city: create_user.city,
        iban: create_user.iban,
        country: create_user.country,
        fk_principal: principal.pk,
    };

    match diesel::insert_into(qr_user::table)
        .values(&new_user)
        .get_result::<User>(&connection)
    {
        Ok(_) => Ok(warp::reply::reply()),
        Err(_) => Err(warp::reject::custom(Error::QueryError)),
    }
}

pub async fn get_users_handler(principal: Principal) -> Result<impl Reply, Rejection> {
    let connection = acquire_db_connection()?;

    match qr_user::table
        .filter(qr_user::fk_principal.eq(&principal.pk))
        .load::<User>(&connection)
    {
        Ok(users) => Ok(warp::reply::json(&users)),
        Err(_) => Err(warp::reject::custom(Error::QueryError)),
    }
}

pub async fn delete_users_handler(
    principal: Principal,
    user_keys_str: String,
) -> Result<impl Reply, Rejection> {
    let mut keys = Vec::new();
    for key_str in user_keys_str.split(",") {
        if let Ok(key) = key_str.trim().parse::<i32>() {
            keys.push(key);
        } else {
            return Err(warp::reject::custom(Error::BadRequestError));
        }
    }

    let connection = acquire_db_connection()?;

    let deleted_users = diesel::delete(
        qr_user::table.filter(
            qr_user::fk_principal
                .eq(principal.pk)
                .and(qr_user::pk.eq(any(keys))),
        ),
    )
    .get_results::<User>(&connection);

    match deleted_users {
        Ok(deleted) => Ok(warp::reply::json(&deleted)),
        Err(_) => Err(warp::reject::custom(Error::QueryError)),
    }
}
