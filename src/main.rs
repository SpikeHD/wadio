use std::{fs::File, io::Read, path::PathBuf, sync::{Arc, Mutex}};

use gumdrop::Options;
use manager::Manager;
use tiny_http::{Response, Server, ServerConfig};
use track::get_bitrate;

mod manager;
mod track;

#[derive(Debug, Options)]
struct Args {
  #[options(help = "show this help message")]
  help: bool,

  #[options(help = "show version information")]
  version: bool,

  #[options(help = "path where music is stored")]
  music_path: String,

  #[options(help = "cache the song list, so restarts are faster", default = "true")]
  cache: bool,
}

fn main() {
  let args: Args = Args::parse_args_default_or_exit();

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

  println!("reading music in {}", args.music_path);

  let (tx, rx) = flume::unbounded();
  let mut manager = Arc::new(Mutex::new(Manager::new(&PathBuf::from(args.music_path), tx).expect("failed to create manager")));

  println!("found {} songs", manager.lock().unwrap().songs().len());

  manager.lock().unwrap().songs_to_queue();
  manager.lock().unwrap().shuffle();

  // Master thread to manage the manager
  std::thread::spawn(move || {
    loop {
      let manager = manager.clone();
      let mut manager = manager.lock().unwrap();

      // Get the next song
      let next = manager.next();
      if next.is_none() {
        // Reshuffle a new queue
        manager.songs_to_queue();
        manager.shuffle();
        continue;
      }

      // Drop the lock
      drop(manager);
    }
  });

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
        "/" => { /* Hosted site */ },
        "/mp3" => {
          let clients = clients_recv.clone();
          let mut clients = clients.lock().unwrap();
          let uuid = uuid::Uuid::new_v4();
          let mut writer = req.into_writer();

          println!("new client: {}", uuid);

          // Write the initial header
          writer.write(b"HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\n\r\n").unwrap();

          clients.push((uuid, writer));

          drop(clients);
        }
        // Ignore everything else
        _ => {}
      }
    }
  });

  // DEBUG write test file to clients
  let br = get_bitrate(&PathBuf::from("audio2.mp3")).unwrap();
  let byterate = br / 8;

  let file = File::open("audio2.mp3").unwrap();
  let mut reader = std::io::BufReader::new(file);
  let mut buf = vec![0; byterate as usize];
  
  while let Ok(n) = reader.read(&mut buf) {
    let clients = clients.clone();
    let mut clients = clients.lock().unwrap();
    for (_, ref mut writer) in clients.iter_mut() {
      println!("writing {} bytes", n);
      writer.write(&buf[..n]).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(1000));

    if n == 0 {
      break;
    }
  }
}
