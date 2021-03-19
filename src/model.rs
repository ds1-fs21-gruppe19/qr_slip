use diesel::{Associations, Identifiable, Insertable, Queryable};

use crate::schema::{principal, qr_user};

#[derive(Identifiable, Insertable, Queryable)]
#[table_name = "qr_user"]
#[primary_key(pk)]
pub struct User {
    pub pk: i32,
    pub first_name: String,
    pub last_name: String,
    pub address: String,
    pub zip_code: String,
    pub city: String,
    pub iban: String,
    pub country: String,
}

#[derive(Associations, Identifiable, Insertable, Queryable)]
#[belongs_to(User, foreign_key = "fk_user")]
#[table_name = "principal"]
#[primary_key(pk)]
pub struct Principal {
    pub pk: i32,
    pub user_name: String,
    pub password: String,
    pub fk_user: i32,
}
