use std::sync::{Arc, Mutex};

use lofty::{file::TaggedFileExt, picture::MimeType};
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

#[derive(Debug, Serialize, Deserialize)]
struct TrackList {
  tracks: Vec<Song>,
}

pub fn handle_api_request(
  req: Request,
  manager: &Arc<Mutex<Manager>>,
) -> Result<(), Box<dyn std::error::Error>> {
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
        res.add_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());
      }
      Err(err) => {
        eprintln!("Error handling /api/next request: {}", err);
      }
    };
  }

  if req.url() == "/api/cover" {
    match playing_cover(manager) {
      Ok((mime, data)) => {
        res = Response::from_data(data);
        res.add_header(Header::from_bytes(b"Content-Type", mime.as_bytes()).unwrap());
      }
      Err(err) => {
        eprintln!("Error handling /api/cover request: {}", err);
      }
    };
  }

  if req.url() == "/api/queue" {
    match queue(manager) {
      Ok(data) => {
        let data = json::to_string(&data);
        res = Response::from_data(data.into_bytes());
        res.add_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());
      }
      Err(err) => {
        eprintln!("Error handling /api/queue request: {}", err);
      }
    };
  }

  if req.url() == "/api/history" {
    match history(manager) {
      Ok(data) => {
        let data = json::to_string(&data);
        res = Response::from_data(data.into_bytes());
        res.add_header(Header::from_bytes(b"Content-Type", b"application/json").unwrap());
      }
      Err(err) => {
        eprintln!("Error handling /api/history request: {}", err);
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

fn playing(
  manager: &Arc<Mutex<Manager>>,
) -> Result<Box<dyn Serialize>, Box<dyn std::error::Error>> {
  let track = match manager.lock().unwrap().current() {
    Some(track) => track,
    None => {
      return Ok(Box::new(Generic {
        message: "No current song".to_string(),
      }));
    }
  };
  let elapsed = manager.lock().unwrap().elapsed();

  Ok(Box::new(Song {
    name: track.name,
    artist: track.artist,
    album: track.album,
    length: track.length,
    elapsed,
  }))
}

fn playing_cover(
  manager: &Arc<Mutex<Manager>>,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
  let track = match manager.lock().unwrap().current() {
    Some(track) => track,
    None => {
      return Ok(("image/jpeg".to_string(), vec![]));
    }
  };

  let tag = lofty::read_from_path(track.path)?;
  let tag = match tag.primary_tag() {
    Some(tag) => tag,
    None => {
      return Ok(("image/jpeg".to_string(), vec![]));
    }
  };

  let pictures = tag.pictures();
  let cover = match pictures.first() {
    Some(cover) => cover,
    None => {
      return Ok(("image/jpeg".to_string(), vec![]));
    }
  };
  let mime = cover.mime_type().unwrap_or(&MimeType::Jpeg);

  Ok((mime.to_string(), cover.data().to_vec()))
}

fn queue(manager: &Arc<Mutex<Manager>>) -> Result<Box<dyn Serialize>, Box<dyn std::error::Error>> {
  let manager = manager.lock().unwrap();
  let queue = manager.queue();

  Ok(Box::new(TrackList {
    tracks: queue
      .iter()
      .map(|track| Song {
        name: track.name.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        length: track.length,
        elapsed: 0,
      })
      .collect(),
  }))
}

fn history(
  manager: &Arc<Mutex<Manager>>,
) -> Result<Box<dyn Serialize>, Box<dyn std::error::Error>> {
  let manager = manager.lock().unwrap();
  let history = manager.history();

  Ok(Box::new(TrackList {
    tracks: history
      .iter()
      .map(|track| Song {
        name: track.name.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        length: track.length,
        elapsed: 0,
      })
      .collect(),
  }))
}
