use std::sync::{Arc, Mutex};

use miniserde::{json, Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response};

use crate::manager::Manager;

#[derive(Debug, Serialize)]
struct Generic {
  message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Song {
  name: String,
  artist: String,
  album: String,
  elapsed: u64,
  length: u64,
}

pub fn handle_api_request(req: Request, manager: &Arc<Mutex<Manager>>) -> Result<(), Box<dyn std::error::Error>> {
  if req.method() != &Method::Get {
    return Ok(());
  }

  // 404
  let mut res = Response::from_string(json::to_string(&Generic {
    message: "Not found".to_string(),
  }));
  res.add_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());

  if req.url() == "/api/current" {
    match playing(manager) {
      Ok(data) => {
        let data = json::to_string(&data);
        res = Response::from_data(data.into_bytes());
      }
      Err(err) => {
        eprintln!("Error handling /api/next request: {}", err);
      }
    };
  }

  match req.respond(res) {
    Ok(_) => {}
    Err(err) => {
      eprintln!("Error responding to API request: {}", err);
    }
  };

  Ok(())
}

pub fn playing(manager: &Arc<Mutex<Manager>>) -> Result<Box<dyn Serialize>, Box<dyn std::error::Error>> {
  let track = match manager.lock().unwrap().current() {
    Some(track) => track,
    None => {
      return Ok(Box::new(
        Generic {
          message: "No current song".to_string(),
        }
      ));
    }
  };
  let elapsed = manager.lock().unwrap().elapsed();

  Ok(
    Box::new(
      Song {
        name: track.name,
        artist: track.artist,
        album: track.album,
        length: track.length,
        elapsed,
      }
    )
  )
}
