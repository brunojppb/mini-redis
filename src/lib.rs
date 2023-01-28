use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

pub type ByteString = Vec<u8>;
pub type ByteStr = [u8];

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: ByteString,
    pub value: ByteString,
}

const CHECKSUM_CHECKER: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);

/// Store structured data using the Bitcask format
///
/// Here is how the layout of an entry looks like:
///
///                                                                                                                               
///                             12 bytes header                                     variable length contents                   
///                                  |                                                          |                               
/// |--------------------------------|---------------------------------|  |---------------------|----------------------         
/// |                                                                  |  |                                           |         
/// |                                                                  |  |                                           |         
/// |      Checksum               key length             value length  |  |         key                   value       |         
/// +------+------+------+ +------+------+------+ +------+------+------+  +--------------------+ +--------------------+         
/// |      |      |      | |      |      |      | |      |      |      |  |                    | |                    |         
/// |      |      |      | |      |      |      | |      |      |      |  |                    | |                    |         
/// +------+------+------+ +------+------+------+ +------+------+------+  +--------------------+ +--------------------+         
/// |                    | |                    | |                    |  |                    | |                    |         
/// |----------|---------| |----------|---------| |----------|---------|  |----------|---------| |----------|---------|         
/// |                      |                      |                       |                      |                   
///           u32                    u32                    u32              [u8, key length]      [u8, value length]           

#[derive(Debug)]
pub struct MiniRedis {
    f: File,
    pub index: HashMap<ByteString, u64>,
}

impl MiniRedis {
    pub fn open(path: &Path) -> io::Result<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;

        let index = HashMap::new();
        Ok(MiniRedis { f, index })
    }

    pub fn load(&mut self) -> io::Result<()> {
        let mut f = BufReader::new(&mut self.f);

        loop {
            let position = f.seek(io::SeekFrom::Current(0))?;
            let maybe_kv = MiniRedis::process_record(&mut f);
            let kv = match maybe_kv {
                Ok(kv) => kv,
                Err(err) => match err.kind() {
                    io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    _ => return Err(err),
                },
            };

            self.index.insert(kv.key, position);
        }

        Ok(())
    }

    fn process_record<R: Read>(f: &mut R) -> io::Result<KeyValuePair> {
        // We need to make sure that the byte order written to disk
        // is consistent across platforms. The byteorder crate helps
        // us to use little endian across all systems.
        let saved_checksum = f.read_u32::<LittleEndian>()?;
        let key_len = f.read_u32::<LittleEndian>()?;
        let value_len = f.read_u32::<LittleEndian>()?;
        let data_len = key_len + value_len;

        let mut data = ByteString::with_capacity(data_len as usize);

        // read the data payload from the reader
        // but place it into our buffer so we can split it up later
        {
            f.by_ref().take(data_len as u64).read_to_end(&mut data)?;
        }

        debug_assert_eq!(data.len(), data_len as usize);

        // make sure the checksum header matches with the computed checksum
        // bail otherwise.
        let checksum = CHECKSUM_CHECKER.checksum(&data);
        if checksum != saved_checksum {
            panic!(
                "data corruption encountered ({:08x} != {:08x})",
                checksum, saved_checksum
            );
        }

        let value = data.split_off(key_len as usize);
        let key = data;

        Ok(KeyValuePair { key, value })
    }

    pub fn seek_to_end(&mut self) -> io::Result<u64> {
        self.f.seek(io::SeekFrom::End(0))
    }

    pub fn get(&mut self, key: &ByteStr) -> io::Result<Option<ByteString>> {
        let position = match self.index.get(key) {
            None => return Ok(None),
            Some(pos) => *pos,
        };

        let kv = self.get_at(position)?;
        Ok(Some(kv.value))
    }

    pub fn get_at(&mut self, position: u64) -> io::Result<KeyValuePair> {
        let mut f = BufReader::new(&mut self.f);
        f.seek(io::SeekFrom::Start(position))?;
        let kv = MiniRedis::process_record(&mut f)?;
        Ok(kv)
    }

    pub fn find(&mut self, target: &ByteStr) -> io::Result<Option<(u64, ByteString)>> {
        let mut f = BufReader::new(&mut self.f);
        let mut found: Option<(u64, ByteString)> = None;

        loop {
            let position = f.seek(io::SeekFrom::Current(0))?;
            let maybe_kv = MiniRedis::process_record(&mut f);
            let kv = match maybe_kv {
                Ok(kv) => kv,
                Err(err) => match err.kind() {
                    io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    _ => return Err(err),
                },
            };

            if kv.key == target {
                found = Some((position, kv.value));
            }
            // Do not break here. As this is a append-only data structure
            // we need to keep looping through the file until the end
            // so if a value was overwritten, we would return the latest.
        }

        Ok(found)
    }

    pub fn insert(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<()> {
        let position = self.insert_but_ignore_index(key, value)?;
        self.index.insert(key.to_vec(), position);
        Ok(())
    }

    pub fn insert_but_ignore_index(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<u64> {
        let key_len = key.len();
        let value_len = value.len();
        let mut tmp = ByteString::with_capacity(key_len + value_len);

        for byte in key {
            tmp.push(*byte);
        }

        for byte in value {
            tmp.push(*byte);
        }

        let checksum = CHECKSUM_CHECKER.checksum(&tmp);

        let mut f = BufWriter::new(&mut self.f);
        let next_byte = SeekFrom::End(0);
        // keep track of the current position in the stream
        // so we can return that to the caller.
        let current_position = f.seek(io::SeekFrom::Current(0))?;
        // Move the needle to the end of the stream so we
        // append the new value to the stream.
        f.seek(next_byte)?;
        // write the header first
        f.write_u32::<LittleEndian>(checksum)?;
        f.write_u32::<LittleEndian>(key_len as u32)?;
        f.write_u32::<LittleEndian>(value_len as u32)?;
        // write the content
        f.write_all(&tmp)?;

        // The caller will use this position to index
        // the key/value pair that were just added.
        Ok(current_position)
    }

    #[inline]
    pub fn update(&mut self, key: &ByteStr, value: &ByteStr) -> io::Result<()> {
        self.insert(key, value)
    }

    #[inline]
    pub fn delete(&mut self, key: &ByteStr) -> io::Result<()> {
        self.insert(key, b"")
    }
}
