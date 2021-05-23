use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{offset::Utc, Duration};
use diesel::{dsl::count, expression::dsl::any, expression_methods::BoolExpressionMethods};
use exec_rs::sync::MutexSync;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{
    filters::header::headers_cloned,
    http::{
        header::{self, HeaderMap},
        Response, StatusCode,
    },
    Filter, Rejection, Reply,
};

use crate::{
    acquire_db_connection,
    diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl},
    error::Error,
    model::{NewPrincipal, NewRefreshToken, NewUser, Principal, RefreshToken, User},
    schema::{principal, qr_user, refresh_token},
    DbConnection,
};

/// Struct received by the /login request.
#[derive(Deserialize)]
pub struct LoginRequest {
    pub user_name: String,
    pub password: String,
}

/// Struct returned by the /login and /refresh-login endpoints.
#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expiration_secs: i64,
}

/// Struct received by the /register endpoint used to create a principal and one qr_user.
#[derive(Deserialize)]
pub struct UserRegistration {
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    pub user_name: String,
    pub password: String,
}

/// Struct encoded in the JWT that contains its expiry and subject principal.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: usize,
    sub: String,
}

/// Warp filter for requests that optionally receive the logged in principal from the auth header.
pub fn with_principal_optional(
) -> impl warp::Filter<Extract = (Option<Principal>,), Error = Rejection> + Clone {
    headers_cloned().and_then(get_principal_from_auth_header)
}

/// Warp filter for requests that require a logged in principal provided by the auth header.
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

/// Decodes the user name provided by the JWT if provided and finds the matching Principal.
///
/// Failure to find the Principal would return a QueryError causing a 500 response as the username
/// should always refer to an existing Principal.
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

/// Handler for the /login endpoint that receives a json deserialized to the [`LoginRequest`] struct
/// and returns a [`LoginResponse`] if the credentials are correct or a InvalidCredentialsError, which
/// results in a 403, if the credentials are not correct.
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

    let refresh_token_cookie = create_refresh_token_cookie(&principal, &connection)?;
    create_login_response(&principal, refresh_token_cookie)
}

/// Create a HttpOnly Cookie that may be used to refresh logins by generating a UUID which is persisted
/// to the database as a RefreshToken entity which links the UUID to the Principal.
fn create_refresh_token_cookie(
    principal: &Principal,
    connection: &DbConnection,
) -> Result<String, Rejection> {
    let uuid = Uuid::new_v4();
    let current_utc = Utc::now();
    let expiry = current_utc + Duration::hours(24);

    let new_refresh_token = NewRefreshToken {
        uuid,
        expiry,
        invalidated: false,
        fk_principal: principal.pk,
    };

    let refresh_token = match diesel::insert_into(refresh_token::table)
        .values(&new_refresh_token)
        .get_result::<RefreshToken>(connection)
    {
        Ok(refresh_token) => refresh_token,
        Err(_) => return Err(warp::reject::custom(Error::QueryError)),
    };

    let uuid = refresh_token.uuid.to_string();
    let expiry = refresh_token.expiry.to_rfc2822();

    // TODO set Secure once moving to production
    Ok(format!(
        "refresh_token={}; Expires={}; HttpOnly",
        uuid, expiry
    ))
}

/// Create a [`LoginResponse`] for the provided Principal and add the provided refresh token cookie.
/// Used when a /login or /refresh-login succeeds.
fn create_login_response(
    principal: &Principal,
    refresh_token_cookie: String,
) -> Result<impl Reply, Rejection> {
    let expiration_period = Duration::minutes(15);
    let expiration_secs = expiration_period.num_seconds();
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
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

    let json_response = serde_json::to_vec(&LoginResponse {
        token,
        expiration_secs,
    })
    .map_err(|_| warp::reject::custom(Error::SerialisationError))?;

    let response_body = Response::builder()
        .status(StatusCode::OK)
        .header(header::SET_COOKIE, refresh_token_cookie)
        .body(json_response)
        .map_err(|_| warp::reject::custom(Error::SerialisationError))?;

    Ok(response_body)
}

/// Refreshes a login for the provided refresh token by creating a fresh JWT for the Principal linked
/// to the refresh token and refreshes the refresh token with a new UUID and resets its expiration.
///
/// Returns a [`LoginResponse`] with the new JWT if the refresh token is valid (the UUID exists and
/// the refresh token is not expired) or else returns a InvalidRefreshTokenError which results in a 401.
pub async fn refresh_login_handler(refresh_token: String) -> Result<impl Reply, Rejection> {
    let connection = acquire_db_connection()?;
    let curr_token_uuid = Uuid::parse_str(&refresh_token)
        .map_err(|_| warp::reject::custom(Error::BadRequestError))?;
    let current_utc = Utc::now();

    let refresh_token = refresh_token::table
        .filter(
            refresh_token::uuid
                .eq(&curr_token_uuid)
                .and(refresh_token::expiry.ge(&current_utc))
                .and(refresh_token::invalidated.eq(false)),
        )
        .first::<RefreshToken>(&connection)
        .optional()
        .map_err(|_| warp::reject::custom(Error::QueryError))?
        .ok_or_else(|| warp::reject::custom(Error::InvalidRefreshTokenError))?;

    let principal = principal::table
        .filter(principal::pk.eq(refresh_token.fk_principal))
        .first::<Principal>(&connection)
        .map_err(|_| warp::reject::custom(Error::QueryError))?;

    let expiry = current_utc + Duration::hours(24);
    let new_token = Uuid::new_v4();

    let updated_token = diesel::update(refresh_token::table)
        .filter(refresh_token::pk.eq(refresh_token.pk))
        .set((
            refresh_token::uuid.eq(new_token),
            refresh_token::expiry.eq(expiry),
        ))
        .get_result::<RefreshToken>(&connection)
        .map_err(|_| warp::reject::custom(Error::QueryError))?;

    let uuid = updated_token.uuid.to_string();
    let expiry = updated_token.expiry.to_rfc2822();

    // TODO set Secure once moving to production
    let refresh_token_cookie = format!("refresh_token={}; Expires={}; HttpOnly", uuid, expiry);

    create_login_response(&principal, refresh_token_cookie)
}

lazy_static! {
    static ref USER_NAME_SYNC: MutexSync<String> = MutexSync::new();
}

/// Registers a user by creating a new Principal and one QrUser. This request receives a json that
/// is deserialized to the [`UserRegistration`] struct which contains all information to create a
/// new Principal and one related QrUser.
///
/// If the given user_name already exists the endpoint returns a PrincipalExistsError which results
/// in a 400.
///
/// Creating the Principal is synchronised based on the value of user_name by mapping a mutex to it.
/// This means that concurrent attempts to register the same user_name will be synchronised so that
/// one request is guaranteed to see the Principal created by other, instead of receiving unique
/// constraint violation when committing either transaction.
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
            name: user_registration.name,
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
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
}

/// Creates a new user (creditor) for the currently logged in Principal. This request receives a json
/// that is deserialized to the [`CreateUser`] struct.
pub async fn create_user_handler(
    principal: Principal,
    create_user: CreateUser,
) -> Result<impl Reply, Rejection> {
    let connection = acquire_db_connection()?;

    let new_user = NewUser {
        name: create_user.name,
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

/// Returns all users (creditors) saved by the currently logged in Principal.
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

/// Deletes all users where the pk matches any of the provided keys in the comma separated `user_keys_str`,
/// which is defined by the path parameter. Invalid primary keys that either do not exist or describe
/// entities that do not belong to the current principal are ignored.
pub async fn delete_users_handler(
    principal: Principal,
    user_keys_str: String,
) -> Result<impl Reply, Rejection> {
    let mut keys = Vec::new();
    for key_str in user_keys_str.split(',') {
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
