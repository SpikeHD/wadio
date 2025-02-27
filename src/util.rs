use std::{fs::File, io::{BufReader, Read, Seek}};

pub fn skip_id3_tags(reader: &mut BufReader<File>) -> std::io::Result<()> {
  let mut buf = [0; 10]; // ID3 header is at least 10 bytes
  reader.read_exact(&mut buf)?;

  // Check if the file starts with an ID3 tag
  if &buf[0..3] == b"ID3" {
    // Calculate the size of the ID3 tag
    let id3_size = ((buf[6] as usize) << 21)
      | ((buf[7] as usize) << 14)
      | ((buf[8] as usize) << 7)
      | (buf[9] as usize);

    // Skip the ID3 tag
    let mut skip_buf = vec![0; id3_size];
    reader.read_exact(&mut skip_buf)?;
  }

  Ok(())
}

pub fn find_mp3_sync_word(reader: &mut BufReader<File>) -> std::io::Result<()> {
  let mut buf = [0; 1];
  loop {
    reader.read_exact(&mut buf)?;

    // Check for the MP3 sync word (0xFF)
    if buf[0] == 0xFF {
      let mut next_byte = [0; 1];
      reader.read_exact(&mut next_byte)?;

      // Check if the next byte starts with 0xF (11-bit sync word)
      if (next_byte[0] & 0xE0) == 0xE0 {
        // Rewind the reader by 2 bytes to include the sync word
        reader.seek(std::io::SeekFrom::Current(-2))?;
        break;
      }
    }
  }

  Ok(())
}
