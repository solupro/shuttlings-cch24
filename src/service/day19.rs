use actix_web::{web, web::Query, HttpMessage, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use shuttle_runtime::__internals::serde_json;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::types::Uuid;
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Deserialize, Serialize, FromRow)]
struct Quote {
    #[serde(default = "Uuid::new_v4")]
    id: Uuid,
    author: String,
    quote: String,
    #[serde(default)]
    created_at: DateTime<Utc>,
    #[serde(default)]
    version: i32,
}

#[derive(Deserialize, Serialize)]
struct QuoteResp {
    quotes: Vec<Quote>,
    page: usize,
    next_token: Option<String>,
}

async fn draft(body: String, req: HttpRequest, pool: web::Data<sqlx::PgPool>) -> HttpResponse {
    let content: Result<Quote, String> = match req.content_type() {
        "application/json" => {
            serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {}", e))
        }
        _ => return HttpResponse::BadRequest().finish(),
    };

    if let Ok(q) = content {
        let id = Uuid::new_v4();
        let new_quote = sqlx::query_as!(
            Quote,
            "INSERT INTO quotes
        (id, author, quote)
        values ($1, $2, $3)
        RETURNING id, author, quote, created_at, version;",
            id,
            q.author,
            q.quote
        )
        .fetch_one(pool.get_ref())
        .await
        .unwrap();

        return HttpResponse::Created().json(new_quote);
    }

    HttpResponse::BadRequest().finish()
}

async fn remove(id: web::Path<Uuid>, pool: web::Data<sqlx::PgPool>) -> HttpResponse {
    let id = id.into_inner();
    let quote: Option<Quote> = get_quote(id, &pool).await;
    if quote.is_none() {
        return HttpResponse::NotFound().finish();
    }

    sqlx::query!(r#"DELETE FROM quotes WHERE id = $1"#, id)
        .execute(pool.get_ref())
        .await
        .unwrap();

    HttpResponse::Ok().json(quote.unwrap())
}

async fn undo(
    body: String,
    req: HttpRequest,
    id: web::Path<String>,
    pool: web::Data<sqlx::PgPool>,
) -> HttpResponse {
    let query_id = match Uuid::try_parse(&id.into_inner()) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let quote: Option<Quote> = get_quote(query_id, &pool).await;
    if quote.is_none() {
        return HttpResponse::NotFound().finish();
    }
    let content: Result<Quote, String> = match req.content_type() {
        "application/json" => {
            serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {}", e))
        }
        _ => return HttpResponse::BadRequest().finish(),
    };

    if let Ok(q) = content {
        let _rs = sqlx::query!(
            "UPDATE quotes
        SET
        author = $1,
        quote = $2,
        version = version + 1
        WHERE id = $3;",
            q.author,
            q.quote,
            query_id
        )
        .execute(pool.get_ref())
        .await
        .unwrap();

        let new_quote = get_quote(query_id, &pool).await.unwrap();
        return HttpResponse::Ok().json(new_quote);
    }

    HttpResponse::BadRequest().finish()
}

async fn cite(id: web::Path<Uuid>, pool: web::Data<sqlx::PgPool>) -> HttpResponse {
    let quote: Option<Quote> = get_quote(id.into_inner(), &pool).await;
    if quote.is_none() {
        return HttpResponse::NotFound().finish();
    }

    HttpResponse::Ok().json(quote.unwrap())
}

async fn reset(pool: web::Data<sqlx::PgPool>) -> HttpResponse {
    let _ = sqlx::query!(r#"TRUNCATE quotes;"#)
        .execute(pool.get_ref())
        .await
        .map_err(|_e| -> HttpResponse { HttpResponse::InternalServerError().finish() });

    HttpResponse::Ok().finish()
}

async fn get_quote(id: Uuid, pool: &web::Data<sqlx::PgPool>) -> Option<Quote> {
    sqlx::query_as!(Quote, "SELECT * FROM quotes WHERE id = $1", id)
        .fetch_optional(pool.get_ref())
        .await
        .unwrap()
}

#[derive(Deserialize)]
struct Params {
    token: Option<String>,
}
async fn list(
    p: Query<Params>,
    page_map: web::Data<Arc<RwLock<HashMap<String, usize>>>>,
    pool: web::Data<sqlx::PgPool>,
) -> HttpResponse {

    let page = match p.into_inner().token {
        Some(token) => {
            let page_map = page_map.read().unwrap();
            if let Some(page) = page_map.get(&token) {
                *page
            } else {
                return HttpResponse::BadRequest().finish();
            }
        }
        None => 0
    };

    let Ok(quotes) = sqlx::query_as!(
        Quote,
        "SELECT * FROM quotes ORDER BY created_at LIMIT 3 OFFSET $1",
        (page * 3) as i64
    )
    .fetch_all(pool.get_ref())
    .await
    else {
        return HttpResponse::InternalServerError().finish();
    };

    #[derive(FromRow)]
    struct Row {
        count: Option<i64>,
    }
    let total = sqlx::query_as!(Row, "SELECT COUNT(*) FROM quotes")
        .fetch_one(pool.get_ref())
        .await
        .unwrap()
        .count
        .unwrap_or(0);

    let page = page + 1;
    let next_token = if total > (page * 3) as i64 {
        let hex = super::generate_token(16);
        page_map.write().unwrap().insert(hex.clone(), page);
        Some(hex)
    } else {
        None
    };

    HttpResponse::Ok().json(QuoteResp {
        quotes,
        page,
        next_token
    })
}

pub(crate) fn scope() -> actix_web::Scope {
    web::scope("19")
        .route("/reset", web::post().to(reset))
        .route("/cite/{id}", web::get().to(cite))
        .route("/remove/{id}", web::delete().to(remove))
        .route("/undo/{id}", web::put().to(undo))
        .route("/draft", web::post().to(draft))
        .route("/list", web::get().to(list))
}
