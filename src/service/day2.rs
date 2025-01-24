use std::net::Ipv6Addr;
use std::str::FromStr;
use actix_web::{web};
use actix_web::web::Query;
use serde::Deserialize;

#[derive(Deserialize)]
struct DestParams {
    from: String,
    key: String,
}

async fn dest(params: Query<DestParams>) -> String {
    let from_nums: Vec<u8> = params
        .from
        .split(".")
        .map(|s| s.parse::<u8>().unwrap())
        .collect();
    let key_nums: Vec<u8> = params
        .key
        .split(".")
        .map(|s| s.parse::<u8>().unwrap())
        .collect();

    let r_nums: Vec<u8> = from_nums
        .iter()
        .zip(key_nums.iter())
        .map(|(a, b)| a.wrapping_add(*b))
        .collect();
    r_nums
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(".")
}

#[derive(Deserialize)]
struct ToParams {
    from: String,
    to: String,
}

async fn to(params: Query<ToParams>) -> String {
    let from_nums: Vec<u8> = params
        .from
        .split(".")
        .map(|s| s.parse::<u8>().unwrap())
        .collect();
    let to_nums: Vec<u8> = params
        .to
        .split(".")
        .map(|s| s.parse::<u8>().unwrap())
        .collect();

    let r_nums: Vec<u8> = from_nums
        .iter()
        .zip(to_nums.iter())
        .map(|(a, b)| b.wrapping_sub(*a))
        .collect();
    r_nums
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(".")
}

fn xor_ipv6(a: String, b: String) -> String {
    let a_addr = Ipv6Addr::from_str(a.as_str()).expect("'a' invalid ipv6 address");
    let b_addr = Ipv6Addr::from_str(b.as_str()).expect("'b' invalid ipv6 address");

    let a_arr: Vec<u16> = a_addr
        .octets()
        .chunks(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    let b_arr: Vec<u16> = b_addr
        .octets()
        .chunks(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    let xor_arr: Vec<u16> = a_arr.iter().zip(b_arr.iter()).map(|(a, b)| a ^ b).collect();
    let xor_addr = Ipv6Addr::new(
        xor_arr[0], xor_arr[1], xor_arr[2], xor_arr[3], xor_arr[4], xor_arr[5], xor_arr[6],
        xor_arr[7],
    );
    xor_addr.to_string()
}

async fn dest_v6(params: Query<DestParams>) -> String {
    xor_ipv6(params.from.clone(), params.key.clone())
}

async fn to_v6(params: Query<ToParams>) -> String {
    xor_ipv6(params.from.clone(), params.to.clone())
}

pub(crate) fn scope() -> actix_web::Scope {
    web::scope("/2")
        .route("/dest", web::get().to(dest))
        .route("/key", web::get().to(to))
        .route("/v6/dest", web::get().to(dest_v6))
        .route("/v6/key", web::get().to(to_v6))
}