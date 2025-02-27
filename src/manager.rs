use std::{
  path::{Path, PathBuf},
  sync::{Arc, Mutex, MutexGuard},
  time::SystemTime,
};

use rand::seq::SliceRandom;

use crate::track::Track;

#[derive(Debug, Clone)]
pub struct Manager {
  path: PathBuf,
  songs: Arc<Mutex<Vec<Track>>>,
  queue: Arc<Mutex<Vec<Track>>>,
  current: Option<Track>,
  song_start: SystemTime,
}

impl Manager {
  pub fn new(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
    // read all tracks recursively
    let songs = Arc::new(Mutex::new(find_songs(path)?));
    let queue = Arc::new(Mutex::new(vec![]));

    Ok(Self {
      path: path.clone(),
      songs,
      queue,
      current: None,
      song_start: SystemTime::now(),
    })
  }

  pub fn refresh(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    self.songs = Arc::new(Mutex::new(find_songs(&self.path)?));
    Ok(())
  }

  pub fn current(&self) -> Option<Track> {
    self.current.clone()
  }

  pub fn songs(&self) -> MutexGuard<Vec<Track>> {
    self.songs.lock().unwrap()
  }

  // pub fn queue(&self) -> MutexGuard<Vec<Track>> {
  //   self.queue.lock().unwrap()
  // }

  pub fn songs_to_queue(&mut self) {
    let songs = self.songs.lock().unwrap();
    self.queue.lock().unwrap().extend(songs.clone());
  }

  pub fn shuffle(&mut self) {
    let mut queue = self.queue.lock().unwrap();
    queue.shuffle(&mut rand::rng());
  }

  // pub fn add_to_queue(&mut self, track: Track) {
  //   let mut queue = self.queue.lock().unwrap();
  //   queue.push(track);
  // }

  pub fn next(&mut self) -> bool {
    let mut queue = self.queue.lock().unwrap();
    self.song_start = SystemTime::now();
    self.current = queue.pop();
    self.current.is_some()
  }

  // pub fn elapsed(&self) -> u64 {
  //   self.song_start.elapsed().unwrap().as_millis() as u64
  // }
}

fn find_songs(path: &Path) -> Result<Vec<Track>, Box<dyn std::error::Error>> {
  let mut songs = vec![];

  for entry in std::fs::read_dir(path)? {
    let entry = entry?;
    let path = entry.path();
    if path.is_file() {
      let track = Track::new(path);
      if let Ok(track) = track {
        songs.push(track);
      }
    } else {
      songs.extend(find_songs(&path)?);
    }
  }

  Ok(songs)
}
