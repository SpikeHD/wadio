use std::{borrow::Cow, fs::File, path::PathBuf};

use lofty::{file::TaggedFileExt, read_from_path, tag::Accessor};
use symphonia::core::{io::MediaSourceStream, probe::Hint, units::TimeBase};

/// Hold metadata for a music track, such as path, name, artist, album, etc.
#[derive(Debug, Clone)]
pub struct Track {
  pub path: PathBuf,
  pub name: String,
  pub artist: String,
  pub album: String,
  pub length: u64,
  pub bitrate: u64,
}

impl Track {
  pub fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
    let (length, bitrate) = get_length_bitrate(&path)?;
    let metadata = read_from_path(&path)?;
    let tag = metadata.primary_tag();

    let tag = match tag {
      Some(tag) => tag,
      None => return Err("no tag found".into()),
    };
    let name = tag.title().unwrap_or(Cow::Borrowed("unknown")).to_string();
    let artist = tag.artist().unwrap_or(Cow::Borrowed("unknown")).to_string();
    let album = tag.album().unwrap_or(Cow::Borrowed("unknown")).to_string();

    Ok(Self {
      path,
      name,
      artist,
      album,
      length,
      bitrate,
    })
  }
}

// https://codeberg.org/obsoleszenz/librecdj/src/branch/main/crates/libplayer/src/sample_loader.rs#L195
pub fn get_length_bitrate(path: &PathBuf) -> Result<(u64, u64), Box<dyn std::error::Error>> {
  let file = File::open(path)?;
  let filesize = file.metadata()?.len();
  let mss = MediaSourceStream::new(Box::new(file), Default::default());
  let mut hint = Hint::new();
  let hint = hint.with_extension("mp3");
  let probe =
    symphonia::default::get_probe().format(hint, mss, &Default::default(), &Default::default())?;
  let format = probe.format;
  let track = format.default_track().ok_or("no default track")?;
  let n_frames = track.codec_params.n_frames.unwrap_or_default();
  let time_base = match track.codec_params.time_base {
    Some(time_base) => time_base,
    None => TimeBase::new(1, 1),
  };
  let length = time_base.calc_time(n_frames).seconds;
  let bitrate = (filesize as f64 * 8.0) / length as f64;
  let bitrate = bitrate as u64;

  Ok((length * 1000, bitrate))
}
