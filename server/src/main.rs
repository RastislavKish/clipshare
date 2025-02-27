/*
* Copyright (C) 2023 Rastislav Kish
*
* This program is free software: you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation, version 3.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
* GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License
* along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Instant, Duration};
use std::sync::LazyLock;

use tokio::sync::Mutex;

use anyhow::bail;
use axum::{
    extract::{DefaultBodyLimit, Path},
    http::StatusCode,
    routing::{get},
    Router,
    };
use axum_server::tls_rustls::RustlsConfig;
use redis::AsyncCommands;
use regex::Regex;

static CLIPBOARD_ID_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(
    r"^[a-zA-Z0-9_\-]{32,128}$"
    ).unwrap());
static CLIPBOARD_CONTENT_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(
    r"^[a-zA-Z0-9+/]+$"
    ).unwrap());
static SIZE_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(
    r"^(?<value>\d+)(?<unit>B|K|M|G|T)?$"
    ).unwrap());
/*
static TIME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(
    r"^(?<value>\d+)(?<unit>S|M|H|Y)?$"
    ).unwrap());
*/
static CLIPBOARD_MONITOR: LazyLock<Mutex<ClipboardMonitor>> = LazyLock::new(|| Mutex::new(ClipboardMonitor::new()));
static REDIS_HOST: LazyLock<redis::ConnectionInfo> = LazyLock::new(|| {
    if let Ok(host)=std::env::var("REDIS_HOST") {
        if let Ok(connection_info)=redis::ConnectionInfo::from_str(&host) {
            return connection_info;
            }
        }

    redis::ConnectionInfo::from_str("redis://127.0.0.1/").unwrap()
    });
static REDIS_CLIENT: LazyLock<redis::Client> = LazyLock::new(|| {
    redis::Client::open(REDIS_HOST.clone()).unwrap()
    });
static CERT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    if let Ok(v)=env::var("CERT_DIR") {
        let cert_dir=PathBuf::from(&v);

        if !cert_dir.exists() {
            panic!("Directory {v} set in CERT_DIR environment variable does not exist.");
            }
        if !cert_dir.is_dir() {
            panic!("Directory {v} set in CERT_DIR environment variable is not a directory.");
            }

        return cert_dir;
        }

    panic!("Error: CERT_DIR environment variable not set.");
    });
static SERVER_PORT: LazyLock<u16> = LazyLock::new(|| {
    if let Ok(v)=env::var("SERVER_PORT") {
        match v.parse::<u16>() {
            Ok(port) => return port,
            Err(_) => {
                eprintln!("Warning: Invalid value in SERVER_PORT environment variable. Using the default setting.");
                },
            }
        }

    3127
    });
static RESTRICTED_TO: LazyLock<Vec<String>> = LazyLock::new(|| {
    if let Ok(id_list)=std::env::var("RESTRICTED_TO") {
        return id_list.split(',')
        .filter(|id| CLIPBOARD_ID_REGEX.is_match(id))
        .map(|id| id.to_string())
        .collect();
        }

    Vec::new()
    });
static MAX_CLIPBOARD_COUNT: LazyLock<usize> = LazyLock::new(|| {
    if let Ok(v)=env::var("MAX_CLIPBOARD_COUNT") {
        match v.parse::<usize>() {
            Ok(val) => return val,
            Err(e) => eprintln!("Warning: Invalid content in MAX_CLIPBOARD_COUNT, using the default value. {e}"),
            };
        }

    10000
    });
static MAX_USED_SPACE: LazyLock<usize> = LazyLock::new(|| {
    if let Ok(v)=env::var("MAX_USED_SPACE") {
        match parse_size(&v) {
            Ok(val) => return val,
            Err(e) => eprintln!("Warning: Invalid content in MAX_USED_SPACE, using the default value. {e}"),
            };
        }

    parse_size("500M").unwrap()
    });
static CLIPBOARD_CONTENT_EXPIRATION_TIME: LazyLock<Duration> = LazyLock::new(|| {
    if let Ok(v)=env::var("CLIPBOARD_CONTENT_EXPIRATION_TIME") {
        match parse_duration(&v) {
            Ok(duration) => return duration,
            Err(e) => eprintln!("Warning: Invalid duration in CLIPBOARD_CONTENT_EXPIRATION_TIME. {e} Using the default value."),
            };
        }

    parse_duration("5M").unwrap()
    });
static CLIPBOARD_CONTENT_MAX_SIZE: LazyLock<usize> = LazyLock::new(|| {
    if let Ok(v)=env::var("CLIPBOARD_CONTENT_MAX_SIZE") {
        match parse_size(&v) {
            Ok(size) => return size,
            Err(e) => eprintln!("Warning: Invalid size in CLIPBOARD_CONTENT_MAX_SIZE. {e} Using the default value."),
            };
        }

    parse_size("5M").unwrap()
    });

#[derive(Clone)]
pub struct Clipboard {
    created_at: Instant,
    size: usize,
    }
impl Clipboard {

    pub fn new(created_at: Instant, size: usize) -> Clipboard {
        Clipboard { created_at, size }
        }

    pub fn created_at(&self) -> Instant {
        self.created_at
        }
    pub fn size(&self) -> usize {
        self.size
        }

    pub fn valid(&self) -> bool {
        let current_time=Instant::now();

        if current_time.duration_since(self.created_at)>=*CLIPBOARD_CONTENT_EXPIRATION_TIME {
            return false;
            }

        true
        }
    }

pub struct ClipboardMonitor {
    clipboards: HashMap<String, Clipboard>,
    total_used_space: usize,
    }
impl ClipboardMonitor {

    pub fn new() -> ClipboardMonitor {
        let clipboards=HashMap::with_capacity(*MAX_CLIPBOARD_COUNT);
        let total_used_space=0_usize;

        ClipboardMonitor { clipboards, total_used_space }
        }

    pub fn reserve_clipboard(&mut self, id: &str, size: usize) -> Result<(), anyhow::Error> {
        if !self.clipboard_fits(id, size) {
            self.garbage_collect();

            if !self.clipboard_fits(id, size) {
                bail!("Storage full");
                }
            }

        if self.clipboards.contains_key(id) {
            self.update_clipboard(id, Clipboard::new(Instant::now(), size));
            }
        else {
            self.add_clipboard(id, Clipboard::new(Instant::now(), size));
            }

        Ok(())
        }

    fn add_clipboard(&mut self, id: &str, clipboard: Clipboard) {
        self.total_used_space+=clipboard.size;
        self.clipboards.insert(id.to_string(), clipboard);
        }
    fn update_clipboard(&mut self, id: &str, clipboard: Clipboard) {
        self.total_used_space+=clipboard.size-self.clipboards[id].size();
        self.clipboards.insert(id.to_string(), clipboard);
        }
    fn clipboard_fits(&self, id: &str, size: usize) -> bool {
        let (count_difference, space_difference)=if self.clipboards.contains_key(id) {
            (0, size-self.clipboards[id].size())
            }
        else {
            (1, size)
            };
        self.clipboards.len()+count_difference<=*MAX_CLIPBOARD_COUNT && self.total_used_space+space_difference<=*MAX_USED_SPACE
        }
    fn garbage_collect(&mut self) {
        self.clipboards=self.clipboards.iter()
        .filter(|(_, clipboard)| clipboard.valid())
        .map(|(id, clipboard)| (id.clone(), clipboard.clone()))
        .collect();

        self.total_used_space=self.clipboards.values()
        .fold(0, |total_used_space, clipboard| total_used_space+clipboard.size());
        }
    }
impl Default for ClipboardMonitor {

    fn default() -> ClipboardMonitor {
        ClipboardMonitor::new()
        }
    }
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app=Router::new()
    .route("/", get(landing_page))
    .route("/clipboard/:id", get(get_clipboard).post(set_clipboard))
    .layer(DefaultBodyLimit::max(*CLIPBOARD_CONTENT_MAX_SIZE));

    let mut public_cert=CERT_DIR.clone();
    public_cert.push("fullchain.pem");
    if !public_cert.exists() {
        panic!("Error: Unable to locate the public certificate in {}", public_cert.display());
        }

    let mut private_cert=CERT_DIR.clone();
    private_cert.push("privkey.pem");
    if !private_cert.exists() {
        panic!("Error: Unable to locate the private certificate in {}", private_cert.display());
        }

    let rustls_config=RustlsConfig::from_pem_file(public_cert, private_cert).await.unwrap();

    let addr=SocketAddr::from(([0, 0, 0, 0], *SERVER_PORT));
    tracing::debug!("Listening on {}", addr);
    axum_server::bind_rustls(addr, rustls_config)
    .serve(app.into_make_service())
    .await
    .unwrap();
    }

async fn landing_page() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("landing_page.html"))
    }

async fn get_clipboard(Path(id): Path<String>) -> (StatusCode, String) {
    if !CLIPBOARD_ID_REGEX.is_match(&id) {
        return (StatusCode::BAD_REQUEST, String::from("Invalid clipboard ID"));
        }
    if !RESTRICTED_TO.is_empty() && !RESTRICTED_TO.contains(&id) {
        return (StatusCode::UNAUTHORIZED, String::from("Unauthorized clipboard ID"));
        }

    if let Ok(mut connection)=REDIS_CLIENT.get_async_connection().await {
        if let Ok(clipboard_content)=connection.get::<String, String>(format!("clipboard::{id}")).await {
            if !clipboard_content.is_empty() {
                return (StatusCode::OK, clipboard_content);
                }
            }

        return (StatusCode::NOT_FOUND, String::from("Clipboard empty"));
        }

    (StatusCode::INTERNAL_SERVER_ERROR, String::from("Internal server error"))
    }
async fn set_clipboard(Path(id): Path<String>, body: String) -> (StatusCode, String) {
    if !CLIPBOARD_ID_REGEX.is_match(&id) {
        return (StatusCode::BAD_REQUEST, String::from("Invalid clipboard ID"));
        }
    if !RESTRICTED_TO.is_empty() && RESTRICTED_TO.contains(&id) {
        return (StatusCode::UNAUTHORIZED, String::from("Unauthorised ID"));
        }
    if body.len()>*CLIPBOARD_CONTENT_MAX_SIZE || !CLIPBOARD_CONTENT_REGEX.is_match(&body) {
        return (StatusCode::BAD_REQUEST, String::from("Invalid clipboard content"));
        }

    if let Ok(mut connection)=REDIS_CLIENT.get_async_connection().await {
        let mut clipboard_monitor=CLIPBOARD_MONITOR.lock().await;
        if let Err(e)=clipboard_monitor.reserve_clipboard(&id, body.len()) {
            return (StatusCode::TOO_MANY_REQUESTS, format!("{e}"));
            }
        drop(clipboard_monitor);

        if connection.set_ex(&format!("clipboard::{id}"), &body, CLIPBOARD_CONTENT_EXPIRATION_TIME.as_secs() as usize).await==Ok(()) {
            return (StatusCode::OK, String::new());
            }
        }

    (StatusCode::INTERNAL_SERVER_ERROR, String::from("Internal server error"))
    }

fn parse_size(size: &str) -> Result<usize, anyhow::Error> {
    let size=size.to_uppercase();

    if let Some(caps)=SIZE_REGEX.captures(&size) {
        let mut value: usize=caps["value"].parse().unwrap();

        value=match &caps["unit"] {
            "B" => value,
            "K" => 1000*value,
            "M" => 1000000*value,
            "G" => 1000000000*value,
            "T" => 1000000000000*value,
            _ => value,
            };

        return Ok(value);
        }

    bail!("Invalid size {size}");
    }
fn parse_duration(duration: &str) -> Result<Duration, anyhow::Error> {
    let duration=duration.to_uppercase();

    if let Some(caps)=SIZE_REGEX.captures(&duration) {
        let mut value: u64=caps["value"].parse().unwrap();

        value=match &caps["unit"] {
            "S" => value,
            "M" => 60*value,
            "H" => 60*60*value,
            "D" => 24*60*60*value,
            "W" => 7*24*60*60*value,
            "Y" => 365*24*60*60*value,
            _ => value,
            };

        return Ok(Duration::from_secs(value));
        }

    bail!("Invalid duration {duration}");
    }
