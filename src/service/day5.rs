use actix_web::{HttpRequest, HttpResponse, HttpMessage, web};
use cargo_manifest::Manifest;
use serde::Deserialize;
use shuttle_runtime::__internals::serde_json;
use toml::Value;

#[derive(Debug)]
enum OrderQuantity {
    U32(u32),
    String(String),
    Other(f64),
}

impl<'de> Deserialize<'de> for OrderQuantity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OrderQuantityVariants {
            U32(u32),
            String(String),
            Other(f64),
        }

        OrderQuantityVariants::deserialize(deserializer).map(|v| match v {
            OrderQuantityVariants::U32(v) => OrderQuantity::U32(v),
            OrderQuantityVariants::String(v) => OrderQuantity::String(v),
            OrderQuantityVariants::Other(v) => OrderQuantity::Other(v),
        })
    }
}

#[derive(Deserialize, Debug)]
struct OrderItem {
    item: Option<String>,
    quantity: Option<OrderQuantity>,
}

#[derive(Deserialize, Debug)]
struct PackageMetadata {
    orders: Option<Vec<OrderItem>>,
}

#[derive(Deserialize, Debug)]
struct Package {
    name: String,
    authors: Option<Vec<String>>,
    keywords: Option<Vec<String>>,
    #[serde(rename = "rust-version")]
    rust_version: Option<String>,
    metadata: Option<PackageMetadata>,
}

#[derive(Deserialize, Debug)]
struct TomlContent {
    package: Package,
}

async fn manifest(body: String, req: HttpRequest) -> HttpResponse {
    let content: Result<Manifest<PackageMetadata, Value>, String> = match req.content_type() {
        "application/toml" => {
            toml::from_str(&body).map_err(|e| format!("TOML parsing error: {}", e))
        }
        "application/yaml" => {
            serde_yaml::from_str(&body).map_err(|e| format!("YAML parsing error: {}", e))
        }
        "application/json" => {
            serde_json::from_str(&body).map_err(|e| format!("JSON parsing error: {}", e))
        }
        _ => return HttpResponse::UnsupportedMediaType().finish(),
    };

    match content {
        Ok(content) => {
            let mut response_body: Option<String> = None;

            if let Some(pkg) = &content.package {
                if !pkg
                    .keywords
                    .as_ref()
                    .and_then(|keywords| keywords.as_ref().as_local())
                    .map_or(false, |kws| kws.contains(&"Christmas 2024".to_string()))
                {
                    return HttpResponse::BadRequest().body("Magic keyword not provided");
                }

                response_body =
                    pkg.metadata
                        .as_ref()
                        .and_then(|v| v.orders.as_ref())
                        .map(|orders| {
                            orders
                                .into_iter()
                                .filter_map(|order| {
                                    // 安全地处理 quantity 字段，避免类型错误导致的崩溃
                                    if let Some(quantity) = &order.quantity {
                                        match quantity {
                                            OrderQuantity::U32(quantity) => Some(format!(
                                                "{}: {}",
                                                order.item.clone().unwrap_or_default(),
                                                quantity
                                            )),
                                            _ => None,
                                        }
                                    } else {
                                        // 如果没有 quantity，忽略该订单
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        });
            }

            match response_body {
                Some(body) if !body.is_empty() => HttpResponse::Ok().body(body),
                _ => HttpResponse::NoContent().finish(),
            }
        }
        Err(err) => {
            eprintln!("Failed to parse: {:?}", err);
            HttpResponse::BadRequest().body("Invalid manifest")
        }
    }
}


pub(crate) fn scope() -> actix_web::Scope {
    web::scope("5")
        .route("/manifest", web::post().to(manifest))
}