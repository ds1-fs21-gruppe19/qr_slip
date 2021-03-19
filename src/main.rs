#[macro_use]
extern crate diesel;

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
        dotenv().ok();

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
}

#[tokio::main]
async fn main() {
    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(auth::login_handler);

    let hello_world = warp::path("hello")
        .and(warp::get())
        .map(|| String::from("hello"));

    let routes = login_route.or(hello_world).recover(error::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
}
