#[macro_use]
extern crate diesel;

use std::str::FromStr;

use diesel::{
    pg::PgConnection,
    r2d2::{self, ConnectionManager, Pool},
};
use dotenv::dotenv;
use lazy_static::lazy_static;
use warp::Filter;

pub mod auth;
pub mod error;
pub mod model;
pub mod schema;

lazy_static! {
    pub static ref CONNECTION_POOL: Pool<ConnectionManager<PgConnection>> = {
        let database_url = std::env::var("DATABASE_URL")
            .expect("Missing environment variable DATABASE_URL must be set to connect to postgres");
        let database_connection_manager =
            r2d2::ConnectionManager::<PgConnection>::new(database_url);
        r2d2::Builder::new()
            .min_idle(Some(10))
            .max_size(50)
            .build(database_connection_manager)
            .expect("Failed to initialise connection pool")
    };
    pub static ref JWT_SECRET: u64 = {
        let secret_str = std::env::var("JWT_SECRET")
            .expect("Missing environment variable JWT_SECRET must be set to generate JWT tokens.");
        u64::from_str(&secret_str).expect("JWT_SECRET var is not a valid u64 value")
    };
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(auth::login_handler);

    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(auth::register_handler);

    let hello_world = warp::path("hello")
        .and(warp::get())
        .map(|| String::from("hello"));

    let routes = login_route
        .or(register_route)
        .or(hello_world)
        .recover(error::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
