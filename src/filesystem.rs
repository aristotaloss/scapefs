use std::{io::ErrorKind, path::PathBuf};
use std::fs::File;
use std::error::Error;
use std::fs;
use std::fmt;
use std::io::{Seek, Read, SeekFrom};
use std::collections::HashMap;
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;

#[derive(Debug)]
pub enum FsError {
    FileNotFound,
    InvalidDirectory,
    NoFileHandle,
    MalformedDataSequence,
    CorruptedData,
}
impl Error for FsError {
    fn description(&self) -> &str {
        match *self {
            FsError::FileNotFound => "the folder does not exist or cannot be read from",
            FsError::InvalidDirectory => "the specified directory is not a valid directory",
            FsError::NoFileHandle => "the filesystem did not load a file yet",
            FsError::MalformedDataSequence => "the data sequence did not complete correctly",
            FsError::CorruptedData => "the data was corrupt",
        }
    }
}
impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FsError::FileNotFound => write!(f, "the folder specified could not be found or read from"),
            FsError::InvalidDirectory => write!(f, "the specified directory is not a valid directory"),
            FsError::NoFileHandle => write!(f, "the filesystem did not load a file yet"),
            FsError::MalformedDataSequence => write!(f, "the data sequence did not complete correctly"),
            FsError::CorruptedData => write!(f, "the data was corrupt"),
        }
    }
}
impl From<FsError> for std::io::Error {
    fn from(other: FsError) -> std::io::Error {
        std::io::Error::new(ErrorKind::Other, other)
    }
}

#[derive(Debug,Clone)]
pub enum CompressionType {
    /// The archive is not compressed and the raw data is the real data.
    None,
    /// The archive is compressed with the Bzip2 compression algorithm.
    Bzip2,
    /// The archive is compressed with (a slightly modified, headerless) Gzip codec.
    Gzip,
    /// The archive is compressed with a modified LZMA2 variant (without size field).
    Lzma
}


#[derive(Debug)]
pub struct FileSystem {
    path: PathBuf,
    mainfile: MainFile,
    indices: HashMap<u32, IndexFile>
}

#[derive(Debug)]
pub struct MainFile {
    file: Option<File>
}

#[derive(Debug)]
pub struct IndexFile {
    id: u32,
    file: File
}

#[derive(Debug,Clone)]
pub struct IndexEntry {
    index: u8,
    id: u32,
    size: u32,
    offset: u64
}

#[derive(Debug,Clone)]
pub struct EntryHeader {
    raw_size: u32,
    real_size: u32,
    compression: CompressionType
}

impl EntryHeader {

    pub fn from_bytes(bytes: [u8; 9]) -> Result<EntryHeader, std::io::Error> {
        // Parse the 9 bytes of important info
        let compression_type = bytes[0];
        let raw_size: u32 = ((bytes[1] as u32) << 24) | ((bytes[2] as u32) << 16) | ((bytes[3] as u32) << 8) | (bytes[4] as u32);
        let real_size: u32 = ((bytes[5] as u32) << 24) | ((bytes[6] as u32) << 16) | ((bytes[7] as u32) << 8) | (bytes[8] as u32);

        // Return the new entry header
        return Ok(EntryHeader {
            raw_size: raw_size,
            real_size: real_size,
            compression: CompressionType::from_code(compression_type)
        });
    }

}

#[derive(Debug,Clone)]
pub struct BlockHeader {
    big: bool,
    entry_id: u32,
    index_id: u8,

    next_seq: i32,
    next_block: u32
}

impl CompressionType {
    /// Fetches the appropriate type of compression based on the header field
    /// in the archive header.
    pub fn from_code(code: u8) -> CompressionType {
        match code {
            1 => CompressionType::Bzip2,
            2 => CompressionType::Gzip,
            3 => CompressionType::Lzma,
            _ => CompressionType::None
        }
    }
}

impl BlockHeader {
    pub fn from_block(big: bool, data: [u8; 520]) -> BlockHeader {
        match big {
            true => {
                BlockHeader {
                    big: true,
                    entry_id: ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | (data[3] as u32),
                    next_seq: (((data[4] as u32) << 8) | (data[5] as u32)) as i32,
                    next_block: ((data[6] as u32) << 16) | ((data[7] as u32) << 8) | (data[8] as u32),
                    index_id: data[9] as u8
                }
            },
            false => {
                BlockHeader {
                    big: false,
                    entry_id: ((data[0] as u32) << 8) | (data[1] as u32),
                    next_seq: (((data[2] as u32) << 8) | (data[3] as u32)) as i32,
                    next_block: ((data[4] as u32) << 16) | ((data[5] as u32) << 8) | (data[6] as u32),
                    index_id: data[7] as u8
                }
            }
        }
    }
}

impl IndexEntry {
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Gets the id of this block.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Gets the absolute size of the entry data, not counting the 8-10
    /// byte header in the blocks.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Gets the absolute offset of the very first 520-byte block of this
    /// entry in the main data file.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn block(&self) -> u32 {
        (self.offset / 520u64) as u32
    }
}

impl IndexFile {
    pub fn last_entry(&self) -> u64 {
         self.file.metadata().unwrap().len() / 6u64
    }

    pub fn entry(&mut self, id: u32) -> Option<IndexEntry> {
        let ref mut file = self.file;
        let mut tmp: [u8; 6] = [0; 6];

        // Seek to the proper position and read into the temp buffer
        let seek_offset = id as u64 * 6u64;
        let res1 = file.seek(SeekFrom::Start(seek_offset));
        let res2 = file.read(&mut tmp);

        // Check if the seek and read operation succeeded
        if res1.is_err() || res2.is_err() {
            return None;
        }

        // Check if the operations returned the expected results
        if res1.unwrap() != seek_offset || res2.unwrap() != 6 {
            return None;
        }

        // Decode the size and offset from the temp buffer
        let size: u32 = ((tmp[0] as u32) << 16) | ((tmp[1] as u32) << 8) | (tmp[2] as u32);
        let offset: u64 = ((tmp[3] as u64) << 16) | ((tmp[4] as u64) << 8) | (tmp[5] as u64);

        Some(IndexEntry {index: self.id as u8, id: id, size: size, offset: offset * 520u64})
    }
}

impl FileSystem {
    pub fn new(string: &'static str) -> Result<FileSystem, FsError> {
        // Declare some nice variables!!!
        let path = PathBuf::from(string);
        let metadata = fs::metadata(&path);

        // Make sure the folder exists
        if !metadata.is_ok() {
            return Err(FsError::FileNotFound);
        }

        // Make sure it is a directory
        if !metadata.unwrap().is_dir() {
            return Err(FsError::InvalidDirectory);
        }

        // Create mainfile path
        let mut mainfile_path = PathBuf::from(string);
        mainfile_path.push("main_file_cache.dat2");

        // Find all valid index files
        let mut indices: HashMap<u32, IndexFile> = HashMap::new();
        let entries = fs::read_dir(&path).unwrap();
        for entry in entries {
            let e = entry.unwrap();
            let fname = e.file_name().into_string().unwrap();

            // Is this an index?
            if fname.starts_with("main_file_cache.idx") {
                // Parse the index id into an integer
                let idx = fname[19..].parse::<u32>().unwrap();

                // Add the index file to our map with indices
                indices.insert(idx, IndexFile {id: idx, file: File::open(e.path()).unwrap()});
            }
        }

        // Create the filesystem object and return it
        let file = File::open(mainfile_path).ok();
        let mainfile = MainFile{file: file};

        Ok(FileSystem {path: path, mainfile: mainfile, indices: indices})
    }

    /// Gets the mainfile, that is, the main_file_cache.dat2 entry in the folder
    /// that holds the actual binary data of the filesystem entries.
    pub fn mainfile(&mut self) -> &mut MainFile {
        &mut self.mainfile
    }

    /// Gets an index with a specific id if it exists. The index can only exist if the file exists
    /// on the file system.
    pub fn index(&mut self, index: u32) -> Option<&mut IndexFile> {
        self.indices.get_mut(&index)
    }
}

impl MainFile {
    /// Checks if the file exists.
    pub fn exists(&self) -> bool {
        self.file.is_some()
    }

    /// Gets the backing file, if existant. Returns a new instance with a fresh seek pointer.
    pub fn file(&mut self) -> Option<&mut File> {
        self.file.as_mut()
    }

    /// Calculates the number of data blocks in the mainfile (if existant). This is done by
    /// taking the file size and dividing that by 520 (rouding up), because each block
    /// takes up 520 bytes of data.
    pub fn num_blocks(&self) -> Option<u64> {
        match self.file {
            Some(ref x) => Some((x.metadata().unwrap().len() + 519u64) / 520u64),
            None => None
        }
    }

    /// Reads a block of data, specified by the block id. The data is read at 520 * block_id
    /// and is exactly 520 bytes big. It is not guaranteed all 520 bytes are occupied if the
    /// block is the last one, thus possible to be trimmed.
    pub fn read_block(&mut self, block: u32) -> Option<[u8; 520]> {
        // Do we have a valid file?
        if self.file.is_none() {
            return None;
        }

        let mut data: [u8; 520] = [0; 520];
        let file = self.file().unwrap();

        // Seek to the right position and read the data
        file.seek(SeekFrom::Start(block as u64 * 520u64)).unwrap();
        file.read(&mut data).unwrap();

        return Some(data);
    }

    pub fn read_header(&mut self, entry: IndexEntry) -> Option<EntryHeader> {
        // Do we have a valid file?
        if self.file.is_none() {
            return None;
        }

        let mut hdr: [u8; 9] = [0; 9];
        let file = self.file().unwrap();

        // Seek to the right position and read the data, skipping the block header at start
        let block_header_len = if entry.id() > 0xFFFF { 10 } else { 8 };
        file.seek(SeekFrom::Start(entry.offset() + block_header_len)).unwrap();
        file.read(&mut hdr).unwrap();

        return Some(EntryHeader::from_bytes(hdr).unwrap());
    }

    pub fn read_entry(&mut self, entry: IndexEntry) -> Result<Vec<u8>, FsError> {
        // Do we have a valid file?
        if self.file.is_none() {
            return Err(FsError::NoFileHandle);
        }

        // Create a vec with what we assume is the size. If not, the vec will
        // perfectly resize itself, so it's only an estimation to help us speed up.
        let mut data: Vec<u8> = Vec::with_capacity(entry.size() as usize);

        let mut current_block = entry.block();
        let mut remaining = entry.size();
        let mut current_seq = 0; // We expect a next part to be '1'

        while remaining > 0 {
            let block_data = self.read_block(current_block).unwrap();
            let block_info = BlockHeader::from_block(entry.id() > 65535, block_data);

            let header_size = if block_info.big {10} else {8};
            let available_data = 520 - header_size;
            let consumable = if remaining > available_data {available_data} else {remaining};

            if block_info.big {
                data.extend(&block_data[10..10+consumable as usize]);
            } else {
                data.extend(&block_data[8..8+consumable as usize]);
            }

            remaining -= consumable;

            // Do some checks to validate this block.
            if remaining > 0 {
                if block_info.index_id != entry.index() || block_info.next_seq != current_seq {
                    return Err(FsError::MalformedDataSequence);
                }
            }

            current_block += 1;
            current_seq += 1;
        }

        Ok(data)
    }

    pub fn read_decompressed(&mut self, entry: IndexEntry) -> Result<Vec<u8>, FsError> {
        let mut data = self.read_entry(entry.clone())?;
        let header = self.read_header(entry).unwrap();

        match header.compression {
            CompressionType::None => {
                Ok(data[5usize..(header.raw_size+5) as usize].to_vec())
            }
            CompressionType::Gzip => {
                let mut cursor = std::io::Cursor::new(&mut data);
                cursor.seek(SeekFrom::Current(9)).unwrap();

                let mut decoder = GzDecoder::new(cursor);
                let mut out = Vec::<u8>::new();
                out.resize(header.real_size as usize, 0);

                match decoder.read_exact(out.as_mut_slice()) {
                    Err(_) => return Err(FsError::CorruptedData),
                    Ok(_) => return Ok(out),
                }
            }
            CompressionType::Lzma => {
                panic!("Lzma compression unsupported")
            }
            CompressionType::Bzip2 => {
                // Patch the data so that the prefix is present
                data[5] = b'B';
                data[6] = b'Z';
                data[7] = b'h';
                data[8] = b'1';

                let mut cursor = std::io::Cursor::new(&mut data);
                cursor.seek(SeekFrom::Current(5)).unwrap();

                let mut decoder = BzDecoder::new(cursor);
                let mut out = Vec::<u8>::new();
                out.resize(header.real_size as usize, 0);

                match decoder.read_exact(out.as_mut_slice()) {
                    Err(_) => return Err(FsError::CorruptedData),
                    Ok(_) => return Ok(out),
                }
            }
        }
    }

}
