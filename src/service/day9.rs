use std::sync::Arc;
use std::time::Duration;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use shuttle_runtime::__internals::serde_json;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub struct TokenBucket {
    capacity: u32,
    tokens: Mutex<u32>,
}

impl TokenBucket {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            tokens: Mutex::new(capacity),
        }
    }

    pub async fn replenish(&self) {
        loop {
            sleep(Duration::from_secs(1)).await;
            let mut tokens = self.tokens.lock().await;
            if *tokens < self.capacity {
                *tokens += 1;
            }
        }
    }

    pub async fn consume(&self) -> bool {
        let mut tokens = self.tokens.lock().await;
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }

    pub async fn refill(&self) {
        let mut tokens = self.tokens.lock().await;
        *tokens = self.capacity;
    }
}

#[derive(Deserialize, Debug, Serialize)]
struct VolumeUnit {
    #[serde(skip_serializing_if = "Option::is_none")]
    liters: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gallons: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    litres: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pints: Option<f32>,
}


async fn milk(body: String, req: HttpRequest, bucket: actix_web::web::Data<Arc<TokenBucket>>) -> impl Responder {
    if !bucket.consume().await {
        return HttpResponse::TooManyRequests().body("No milk available\n")
    }

    // print body
    println!("{}", body);

    let content: Result<VolumeUnit, String> = match req.content_type() {
        "application/json" => serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {}", e)),
        _ => return HttpResponse::Ok().body("Milk withdrawn\n"),
    };

    if let Ok(VolumeUnit { liters, gallons, litres, pints }) = content {
        let mut v = VolumeUnit { liters: None, gallons: None, litres: None, pints: None };
        if (liters.is_some() && gallons.is_none() && litres.is_none() && pints.is_none()) ||
            (liters.is_none() && gallons.is_some() && litres.is_none() && pints.is_none()) {
            if let Some(liters) = liters {
                // liters to gallons
                let gallons = liters / 3.78541;
                v.gallons = Some(gallons);
            }
            if let Some(gallons) = gallons {
                // gallons to liters
                let liters = gallons * 3.78541;
                v.liters = Some(liters);
            }
            return HttpResponse::Ok().json(v);
        } else if (litres.is_some() && pints.is_none() && liters.is_none() && gallons.is_none())
            || (litres.is_none() && pints.is_some() && liters.is_none() && gallons.is_none()) {
            if let Some(litres) = litres {
                // litres to pints
                let pints = litres * 1.75975;
                v.pints = Some(pints);
            }
            if let Some(pints) = pints {
                // pints to litres
                let litres = pints / 1.75975;
                v.litres = Some(litres);
            }
            return HttpResponse::Ok().json(v);
        }
    }

    HttpResponse::BadRequest().finish()
}

async fn refill(bucket: web::Data<Arc<TokenBucket>>) -> impl Responder {
    bucket.refill().await;
    HttpResponse::Ok().finish()
}


pub(crate) fn scope() -> actix_web::Scope {
    web::scope("9")
        .route("/milk", web::post().to(milk))
        .route("/refill", web::post().to(refill))
}