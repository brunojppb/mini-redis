use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{self, BufReader, Read, Seek},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt};

type ByteString = Vec<u8>;
type ByteStr = [u8];

// #[derive(Debug, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: ByteString,
    pub value: ByteString,
}

const CHECKSUM_CHECKER: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);

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
        // but place it into our buffer to we can split it up later
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
}
