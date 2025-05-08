use crate::state;
use embedded_svc::{http::Method, io::Write};
use esp_idf_hal::io::EspIOError;
use esp_idf_svc::{http::server::EspHttpServer, ws::FrameType};
use esp_idf_sys::EspError;
use std::{fs, path::PathBuf};

pub fn server_handle_ui(server: &mut EspHttpServer<'static>) -> anyhow::Result<()> {
    server.ws_handler("/api/ws", move |ws| {
        let session_id = ws.session();
        if ws.is_new() {
            let sender = ws.create_detached_sender().unwrap();
            state::enqueue(state::Event::AddSession(session_id, sender));
            ws.send(FrameType::Text(false), b"Connected to WebSocket!")?;
        } else if ws.is_closed() {
            state::enqueue(state::Event::RemoveSession(session_id));
        }

        Ok::<(), EspError>(())
    })?;

    server.fn_handler(
        "/",
        Method::Get,
        |request| -> core::result::Result<(), EspIOError> {
            let full_path = "/frontend/index.html.gz";
            let html = fs::read(full_path).unwrap();

            let mut response =
                request.into_response(200, Some(""), &get_content_headers(full_path))?;
            response.write_all(&html)?;
            Ok(())
        },
    )?;

    server.fn_handler(
        "/*",
        Method::Get,
        |request| -> core::result::Result<(), EspIOError> {
            println!("{}", request.uri());
            match path_from_uri(request.uri()) {
                Some(full_path) => {
                    println!(
                        "path: {}, mime: {}",
                        &full_path.to_str().unwrap(),
                        get_mime_type(full_path.to_str().unwrap())
                    );
                    if let Ok(contents) = fs::read(&full_path) {
                        let mut response = request.into_response(
                            200,
                            Some(""),
                            &get_content_headers(full_path.to_str().unwrap()),
                        )?;
                        response.write_all(&contents)?;
                    }
                }
                None => {
                    let mut response = request.into_ok_response()?;
                    response.write_all(format!("404 Not Found").as_bytes())?;
                }
            }

            Ok(())
        },
    )?;

    Ok(())
}

fn try_path(pathbuf: &PathBuf) -> Option<PathBuf> {
    match pathbuf.try_exists() {
        Ok(_) => Some(pathbuf.to_path_buf()),
        _ => None,
    }
}

fn try_encoding_endings(pathbuf: &PathBuf) -> Option<PathBuf> {
    try_path(&pathbuf.with_added_extension("gz"))
        .or_else(|| try_path(&pathbuf.with_added_extension("br")))
        .or_else(|| try_path(&pathbuf))
}

fn path_from_uri(uri: &str) -> Option<PathBuf> {
    let request_path = uri.trim_start_matches('/'); // Extract filename
    let mut full_path = PathBuf::from("/frontend");
    full_path.push(&request_path);

    try_encoding_endings(&full_path).or_else(|| try_encoding_endings(&full_path.join("index.html")))
}

fn get_mime_type(file_name: &str) -> &str {
    match file_name.rsplit('.').next() {
        Some("html") => "text/html",
        Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("txt") => "text/plain",
        Some("pdf") => "application/pdf",
        Some("wasm") => "application/wasm",
        Some("xml") => "application/xml",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "application/octet-stream", // default for unknown types
    }
}

fn get_content_encoding(file_name: &str) -> (&str, &str) {
    match file_name.rsplit_once('.') {
        Some((file_name, "gz")) => (file_name, "gzip"),
        Some((file_name, "br")) => (file_name, "br"),
        _ => (file_name, "identity"),
    }
}

fn get_content_headers(file_name: &str) -> [(&str, &str); 2] {
    let (file_name, content_encoding) = get_content_encoding(file_name);
    let content_type = get_mime_type(file_name);

    [
        ("Content-Encoding", content_encoding),
        ("Content-Type", content_type),
    ]
}
