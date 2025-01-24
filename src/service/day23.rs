use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use futures::StreamExt;
use askama_escape::{escape, Html};
use serde::Deserialize;

async fn star() -> impl Responder {
    HttpResponse::Ok().body("<div id=\"star\" class=\"lit\"></div>")
}

async fn present(path: web::Path<String>) -> impl Responder {
    let color = path.into_inner();
    let next_color = match color.as_str() {
        "red" => "blue",
        "blue" => "purple",
        "purple" => "red",
        _ => return HttpResponse::ImATeapot().finish(),
    };

    let html = format!("<div class=\"present {color}\" hx-get=\"/23/present/{next_color}\" hx-swap=\"outerHTML\"><div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div></div>");
    HttpResponse::Ok().body(html)
}

async fn ornament(path: web::Path<(String, String)>) -> impl Responder {
    let (state, n) = path.into_inner();
    let next_state = match state.as_str() {
        "on" => "off",
        "off" => "on",
        _ => return HttpResponse::ImATeapot().finish(),
    };
    let n = escape(&n, Html);

    let html = format!(
        r#"<div class="ornament{}" id="ornament{n}" hx-trigger="load delay:2s once" hx-get="/23/ornament/{next_state}/{n}" hx-swap="outerHTML"></div>"#,
        match state.as_str() {
            "on" => " on",
            _ => "",
        }
    );
    HttpResponse::Ok().body(html)
}

#[derive(Deserialize, Debug)]
struct Lockfile {
    #[serde(rename = "package")]
    packages: Vec<Package>,
}

#[derive(Deserialize, Debug)]
struct Package {
    checksum: Option<String>,
}
async fn lockfile(mut payload: Multipart) -> impl Responder {
    let mut bytes = Vec::new();
    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                // 获取字段的 Content-Disposition
                let content_disposition = field.content_disposition();
                if let Some(filename) = content_disposition.get_filename() {
                    println!("Uploading file: {}", filename);
                    while let Some(chunk) = field.next().await {
                        let data = chunk.unwrap();
                        bytes.extend_from_slice(&data);
                    }
                }
            }
            Err(e) => {
                println!("Error while receiving file: {}", e);
                return HttpResponse::BadRequest().finish();
            }
        }
    }

    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UTF-8"),
    };

    let packages = match toml::from_str::<Lockfile>(&content) {
        Ok(Lockfile { packages }) => packages,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let mut html = String::default();
    for package in packages {
        let Some(checksum) = &package.checksum else {
            continue;
        };
        if checksum.len() < 10 {
            return HttpResponse::UnprocessableEntity().finish();
        }
        if hex::decode(checksum).is_err() {
            return HttpResponse::UnprocessableEntity().finish();
        }
        let color = &checksum[0..6];
        let top = u32::from_str_radix(&checksum[6..8], 16).expect("Hex code");
        let left = u32::from_str_radix(&checksum[8..10], 16).expect("Hex code");

        html.push_str(&format!(
            "<div style=\"background-color:#{color};top:{top}px;left:{left}px;\"></div>"
        ));
    }

    HttpResponse::Ok().body(html)
}

pub(crate) fn scope() -> actix_web::Scope {
    web::scope("23")
        .route("/star", web::get().to(star))
        .route("/present/{color}", web::get().to(present))
        .route("/ornament/{state}/{n}", web::get().to(ornament))
        .route("/lockfile", web::post().to(lockfile))
}