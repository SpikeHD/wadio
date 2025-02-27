use std::{
  fs::File,
  io::Read,
  path::PathBuf,
  sync::{Arc, Mutex},
};

use gumdrop::Options;
use manager::Manager;
use rand::Rng;
use tiny_http::{Response, Server, ServerConfig};
use track::get_bitrate;
use util::{find_mp3_sync_word, skip_id3_tags};
use uuid::fmt::Braced;

mod manager;
mod track;
mod util;

#[derive(Debug, Options)]
struct Args {
  #[options(help = "show this help message")]
  help: bool,

  #[options(help = "show version information")]
  version: bool,

  #[options(help = "path where music is stored")]
  music_path: String,

  #[options(help = "automatically rescan the music path for new music whenever the current playlist is finished", default = "true")]
  auto_refresh: bool,
}

fn main() {
  let args: Args = Args::parse_args_default_or_exit();
  let autorefresh = args.auto_refresh;

  if args.help {
    println!("{}", Args::usage());
    return;
  }

  if args.version {
    println!(
      "wadio {} ({})",
      env!("CARGO_PKG_VERSION"),
      option_env!("GIT_HASH").unwrap_or("unknown revision")
    );
    return;
  }

  println!("Reading music in {}", args.music_path);

  let master = Arc::new(Mutex::new(
    Manager::new(&PathBuf::from(args.music_path)).expect("failed to create manager"),
  ));

  println!("Found {} songs", master.lock().unwrap().songs().len());

  master.lock().unwrap().songs_to_queue();
  master.lock().unwrap().shuffle();

  let server = Server::http("0.0.0.0:7887").expect("failed to create HTTP server");
  let clients_recv = Arc::new(Mutex::new(vec![]));
  let clients = clients_recv.clone();

  // Start the server thread
  std::thread::spawn(move || {
    loop {
      let req = match server.try_recv() {
        Ok(Some(req)) => req,
        Ok(None) => continue,
        Err(err) => {
          println!("error: {}", err);
          continue;
        }
      };

      match req.url() {
        "/mp3" => {
          let clients = clients_recv.clone();
          let mut clients = clients.lock().unwrap();
          let uuid = uuid::Uuid::new_v4();

          println!("New client: {} ({:?})", uuid, req.remote_addr());

          let mut writer = req.into_writer();

          // Write the initial header
          writer
            .write(b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\n\r\n")
            .unwrap();

          clients.push((uuid, writer));

          drop(clients);
        }
        // Ignore everything else
        _ => {}
      }
    }
  });

  println!("Listening on http://0.0.0.0:7887");
  println!("MP3 stream available at http://0.0.0.0:7887/mp3");

  loop {
    // Get next song
    if !master.lock().unwrap().next() {
      println!("No more songs, shuffling");
      // Reshuffle a new queue
      if autorefresh {
        match master.lock().unwrap().refresh() {
          Ok(_) => {}
          Err(err) => {
            eprintln!("Failed to refresh music path: {}", err);
          }
        };
      }
      master.lock().unwrap().songs_to_queue();
      master.lock().unwrap().shuffle();
      continue;
    }

    let song = match master.lock().unwrap().current() {
      Some(song) => song,
      None => {
        eprintln!("No current song, this should never happen!");
        break;
      }
    };

    println!("Playing {}", song.name);

    let path = song.path;
    let bitrate = match get_bitrate(&path) {
      Ok(bitrate) => bitrate,
      Err(err) => {
        eprintln!("Failed to get bitrate for {}: {}", song.name, err);
        continue;
      }
    };
    // Add a bit of wiggle room so any discrepancies don't cause lag/stutter/choppy streams
    // This does make it so someone could skip ahead (and then lag while it waits for the next song)
    // but whatever.
    // This number comes out of nowhere, it is not based on any math or rigorous testing.
    let byterate = (bitrate / 8) + 1200;
    let file = match File::open(path) {
      Ok(file) => file,
      Err(err) => {
        eprintln!("Failed to open file {}: {}", song.name, err);
        continue;
      }
    };
    let mut reader = std::io::BufReader::new(file);
    let mut buf = vec![0; byterate as usize];

    // If we don't skip these, some programs (like browsers) get confused.
    // This is because they read the file metadata, which tells it the length
    // of the MP3, but we are obviously just going to keep playing MP3 data, so
    // it doesn't match
    if skip_id3_tags(&mut reader).is_err() {
      eprintln!("Failed to skip ID3 tags for {}", song.name);
      continue;
    }

    if find_mp3_sync_word(&mut reader).is_err() {
      eprintln!("Failed to find MP3 sync word for {}", song.name);
      continue;
    }

    while let Ok(n) = reader.read(&mut buf) {
      let clients = clients.clone();
      let mut clients = clients.lock().unwrap();

      let mut idx = 0;

      // We do the loop like this because we need to be able to remove clients on the fly
      while idx < clients.len() {
        let (uuid, ref mut writer) = clients[idx];
        match writer.write(&buf[..n]) {
          Ok(_) => {}
          Err(err) => {
            println!(
              "Error writing to client (likely disconnected) {}: {}",
              uuid, err
            );
            clients.retain(|(u, _)| *u != uuid);
          }
        };
        idx += 1;
      }

      std::thread::sleep(std::time::Duration::from_millis(1000));

      if n == 0 {
        break;
      }
    }

    // Short pause between songs
    std::thread::sleep(std::time::Duration::from_millis(1000));

    println!("Finished playing {}", song.name);
  }
}
