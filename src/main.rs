extern crate actix_threadpool;
extern crate dotenv;
#[macro_use]
extern crate dotenv_codegen;
extern crate git2;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempfile;
extern crate thiserror;
extern crate urlencoding;
extern crate uuid;
extern crate validator;
#[macro_use]
extern crate validator_derive;

use std::env;

use actix_cors::Cors;
use actix_web::{http, middleware::Logger, web, App, HttpServer};
use dotenv::dotenv;
use env_logger::Env;
use listenfd::ListenFd;

mod errors;
pub mod file;
mod utils;
mod validators;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    std::env::set_var("RUST_LOG", "actix_web=debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(web::scope("/api/v1").service(file::create_file))
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
