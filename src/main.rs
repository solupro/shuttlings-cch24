mod service;

use std::collections::HashMap;
use actix_files::Files;
use actix_web::middleware::Logger;
use actix_web::{get, web, web::ServiceConfig, HttpResponse};
use service::day12;
use service::day19;
use service::day2;
use service::day23;
use service::day5;
use service::day9;
use service::day16;
use shuttle_actix_web::ShuttleActixWeb;
use std::convert::Into;
use std::sync::{Arc, RwLock};

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres(local_uri = "postgres://root:123456@localhost:5432/test")]
    pool: sqlx::PgPool,
) -> ShuttleActixWeb<impl FnOnce(&mut ServiceConfig) + Send + Clone + 'static> {

    let bucket = Arc::new(day9::TokenBucket::new(5));
    let bucket_clone = bucket.clone();
    let board: Arc<RwLock<day12::Board>> = Default::default();
    let page_map: Arc<RwLock<HashMap<String, usize>>> = Default::default();

    // 启动一个独立的任务来补充令牌
    tokio::spawn(async move {
        bucket_clone.replenish().await;
    });

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let config = move |cfg: &mut ServiceConfig| {
        cfg.service(hello_world)
            .service(scope())
            .service(day2::scope().wrap(Logger::default()))
            .service(day5::scope().wrap(Logger::default()))
            .service(day9::scope().wrap(Logger::default()))
            .service(day12::scope().wrap(Logger::default()))
            .service(day16::scope().wrap(Logger::default()))
            .service(day19::scope().wrap(Logger::default()))
            .service(day23::scope().wrap(Logger::default()));

        cfg.app_data(web::Data::new(bucket.clone()));
        cfg.app_data(web::Data::new(board));
        cfg.app_data(web::Data::new(pool.clone()));
        cfg.app_data(web::Data::new(page_map));

        cfg.service(Files::new("/assets", "assets"));
    };

    Ok(config.into())
}

#[get("/")]
async fn hello_world() -> &'static str {
    "Hello, bird!"
}

async fn seek() -> HttpResponse {
    HttpResponse::Found()
        .insert_header(("Localtion", "https://www.youtube.com/watch?v=9Gc4QTqslN4"))
        .finish()
}

fn scope() -> actix_web::Scope {
    web::scope("-1")
        .route("/seek", web::get().to(seek))
}
