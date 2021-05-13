#[macro_use]
extern crate diesel;
#[cfg(feature = "auto_migration")]
#[macro_use]
extern crate diesel_migrations;

use std::str::FromStr;

use diesel::{
    pg::PgConnection,
    r2d2::{self, ConnectionManager, Pool, PooledConnection},
};
use dotenv::dotenv;
use lazy_static::lazy_static;
use pyo3::prelude::*;
use warp::{http::header, Filter};

use error::Error;

pub mod auth;
pub mod error;
pub mod model;
pub mod schema;
pub mod templating;

pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

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
    pub static ref USE_PY_QR_GENERATOR: bool = {
        std::env::var("USE_PY_QR_GENERATOR").map_or(false, |val| {
            bool::from_str(&val).expect("USE_PY_QR_GENERATOR is not a valid bool value")
        })
    };
}

pub const QR_GENERATOR_MODULE: &str = "qr_generator";
const QR_GENERATOR_SCRIPT: &str = std::include_str!("resources/py/qr_generator.py");

#[cfg(feature = "auto_migration")]
diesel_migrations::embed_migrations!();

#[tokio::main]
async fn main() {
    dotenv().ok();

    // initialise certain lazy statics on startup
    lazy_static::initialize(&CONNECTION_POOL);
    lazy_static::initialize(&JWT_SECRET);
    lazy_static::initialize(&USE_PY_QR_GENERATOR);
    lazy_static::initialize(&templating::QR_SLIP_TEMPLATES);
    lazy_static::initialize(&templating::PDF_APPLICATION_WORKER);

    setup_logger();

    if *USE_PY_QR_GENERATOR {
        // compile qr generator module
        Python::with_gil(|py| {
            PyModule::from_code(
                py,
                QR_GENERATOR_SCRIPT,
                "resources/py/qr_generator.py",
                QR_GENERATOR_MODULE,
            )
            .expect("Could not compile qr generator python module");
        });
    }

    #[cfg(feature = "auto_migration")]
    {
        log::info!("Running diesel migrations");
        let connection = acquire_db_connection().expect("Failed to acquire database connection");
        if let Err(e) = embedded_migrations::run_with_output(&connection, &mut std::io::stdout()) {
            panic!("Failed running db migrations: {}", e);
        }
        log::info!("Done running diesel migrations");
    }

    let login_route = warp::path("login")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(auth::login_handler);

    let refresh_login_router = warp::path("refresh-login")
        .and(warp::post())
        .and(warp::cookie("refresh_token"))
        .and_then(auth::refresh_login_handler);

    let register_route = warp::path("register")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(auth::register_handler);

    let create_user_route = warp::path("create-user")
        .and(warp::post())
        .and(auth::with_principal())
        .and(warp::body::json())
        .and_then(auth::create_user_handler);

    let get_users_route = warp::path("users")
        .and(warp::get())
        .and(auth::with_principal())
        .and_then(auth::get_users_handler);

    let delete_users_route = warp::path("delete-users")
        .and(warp::delete())
        .and(auth::with_principal())
        .and(warp::path::param())
        .and_then(auth::delete_users_handler);

    let generate_qr_slip_route = warp::path("generate-slip")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(templating::generate_slip_handler)
        .map(|reply| warp::reply::with_header(reply, header::CONTENT_TYPE, "application/pdf"));

    #[cfg(debug_assertions)]
    let dbg_qr_pdf_route = warp::path("dbg-qr-pdf")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(templating::dbg_qr_pdf_handler);

    #[cfg(debug_assertions)]
    let dbg_qr_html_route = warp::path("dbg-qr-html")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(templating::dbg_qr_html_handler);

    #[cfg(debug_assertions)]
    let dbg_qr_svg_route = warp::path("dbg-qr-svg")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(templating::dbg_qr_svg_handler);

    let routes = login_route
        .or(refresh_login_router)
        .or(register_route)
        .or(create_user_route)
        .or(get_users_route)
        .or(delete_users_route)
        .or(generate_qr_slip_route);

    #[cfg(debug_assertions)]
    let all_routes = routes
        .or(dbg_qr_pdf_route)
        .or(dbg_qr_html_route)
        .or(dbg_qr_svg_route);

    #[cfg(not(debug_assertions))]
    let all_routes = routes;

    let filter = all_routes
        .recover(error::handle_rejection)
        .with(warp::log("qr_slip::api"));
    warp::serve(filter).run(([0, 0, 0, 0], 8000)).await;
}

pub fn acquire_db_connection() -> Result<DbConnection, warp::Rejection> {
    CONNECTION_POOL
        .get()
        .map_err(|_| warp::reject::custom(Error::DatabaseConnectionError))
}

fn setup_logger() {
    // create logs dir as fern does not appear to handle that itself
    if !std::path::Path::new("logs/").exists() {
        std::fs::create_dir("logs").expect("Failed to create logs/ directory");
    }

    let (logging_level, api_logging_level) = if cfg!(debug_assertions) {
        (log::LevelFilter::Debug, log::LevelFilter::Debug)
    } else {
        (log::LevelFilter::Info, log::LevelFilter::Warn)
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}]{}[{}] {}",
                record.level(),
                chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                record.target(),
                message
            ))
        })
        .level(logging_level)
        .level_for("qr_slip::api", api_logging_level)
        .chain(std::io::stdout())
        .chain(fern::DateBased::new("logs/", "logs_%Y-%W.log"))
        .apply()
        .expect("Failed to set up logging");
}
