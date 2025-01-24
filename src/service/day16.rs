use std::collections::HashSet;
use actix_web::cookie::Cookie;
use actix_web::{web, HttpMessage, HttpResponse};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode, decode, Validation, Algorithm};
use jsonwebtoken::errors::ErrorKind;
use serde::{Deserialize, Serialize};
use shuttle_runtime::__internals::serde_json;

const SECRET_KEY: &[u8] = b"not-a-secret";
const SANTAS_PUB_KEY: &[u8] = include_bytes!("../../assets/day16_santa_public_key.pem");

#[derive(Serialize, Deserialize)]
struct Claims {
    gift: serde_json::Value,
    exp: u64,
}

async fn wrap(body: String, req: actix_web::HttpRequest) -> HttpResponse {
    let content: Result<serde_json::Value, String> = match req.content_type() {
        "application/json" => {
            serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {}", e))
        }
        _ => return HttpResponse::BadRequest().finish(),
    };

    let c = Claims {
        gift: content.unwrap(),
        exp: u64::MAX,
    };
    let Ok(token) = encode(
        &Header::default(),
        &c,
        &EncodingKey::from_secret(SECRET_KEY),
    ) else {
        return HttpResponse::InternalServerError().finish();
    };

    HttpResponse::Ok()
        .cookie(Cookie::new("gift", token))
        .finish()
}

async fn unwrap(req: actix_web::HttpRequest) -> HttpResponse {
    let Some(gift) = req.cookie("gift") else {
        return HttpResponse::BadRequest().finish();
    };

    let Ok(token_data) =
        decode::<Claims>(gift.value(), &DecodingKey::from_secret(SECRET_KEY), &Validation::default())
    else {
        return HttpResponse::BadRequest().finish();
    };

    HttpResponse::Ok().json(token_data.claims.gift)
}

async fn decode_token(body: String) -> HttpResponse {
    let mut validation = Validation::default();
    validation.algorithms = vec![Algorithm::RS256, Algorithm::RS512];
    validation.required_spec_claims = HashSet::default();

    match decode::<serde_json::Value>(&body, &DecodingKey::from_rsa_pem(SANTAS_PUB_KEY).unwrap(), &validation) {
        Ok(token_data) => HttpResponse::Ok().json(token_data.claims),
        Err(e) => {
            match e.kind() {
                ErrorKind::InvalidSignature => HttpResponse::Unauthorized().finish(),
                _ => HttpResponse::BadRequest().finish(),
            }
        }
    }
}

pub(crate) fn scope() -> actix_web::Scope {
    web::scope("16")
        .route("/wrap", web::post().to(wrap))
        .route("/unwrap", web::get().to(unwrap))
        .route("/decode", web::post().to(decode_token))
}
