use chrono::{offset::Utc, DateTime};
use diesel::{Associations, Identifiable, Insertable, Queryable};
use serde::Serialize;
use uuid::Uuid;

use crate::schema::{principal, qr_user, refresh_token};

#[derive(Associations, Identifiable, Queryable, Serialize)]
#[belongs_to(Principal, foreign_key = "fk_principal")]
#[table_name = "qr_user"]
#[primary_key(pk)]
pub struct User {
    pub pk: i32,
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    #[serde(skip_serializing)]
    pub fk_principal: i32,
}

#[derive(Insertable)]
#[table_name = "qr_user"]
pub struct NewUser {
    pub name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
    pub fk_principal: i32,
}

#[derive(Identifiable, Queryable)]
#[table_name = "principal"]
#[primary_key(pk)]
pub struct Principal {
    pub pk: i32,
    pub user_name: String,
    pub password: String,
}

#[derive(Insertable)]
#[table_name = "principal"]
pub struct NewPrincipal {
    pub user_name: String,
    pub password: String,
}

#[derive(Associations, Identifiable, Queryable)]
#[belongs_to(Principal, foreign_key = "fk_principal")]
#[table_name = "refresh_token"]
#[primary_key(pk)]
pub struct RefreshToken {
    pub pk: i32,
    pub uuid: Uuid,
    pub expiry: DateTime<Utc>,
    pub invalidated: bool,
    pub fk_principal: i32,
}

#[derive(Insertable)]
#[table_name = "refresh_token"]
pub struct NewRefreshToken {
    pub uuid: Uuid,
    pub expiry: DateTime<Utc>,
    pub invalidated: bool,
    pub fk_principal: i32,
}
