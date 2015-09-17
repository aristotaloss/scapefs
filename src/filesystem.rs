use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::fs;
use std::fmt;
use std::io::{Seek, Read, SeekFrom};
use std::collections::HashMap;

#[derive(Debug)]
pub enum FsError {
    FileNotFound,
    InvalidDirectory
}
impl Error for FsError {
    fn description(&self) -> &str {
        match *self {
            FsError::FileNotFound => "the folder does not exist or cannot be read from",
            FsError::InvalidDirectory => "the specified directory is not a valid directory",
        }
    }
}
impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FsError::FileNotFound => write!(f, "the folder specified could not be found or read from"),
            FsError::InvalidDirectory => write!(f, "the specified directory is not a valid directory"),
        }
    }
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
    file: File
}

#[derive(Debug,Clone)]
pub struct IndexEntry {
    id: u32,
    size: u32,
    offset: u64
}

impl IndexEntry {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl IndexFile {
    pub fn last_entry(&self) -> u64 {
         self.file.metadata().unwrap().len() / 6u64
    }

    pub fn entry(&mut self, id: u32) -> Option<&mut IndexEntry> {
        let ref mut file = self.file;
        let mut tmp: [u8; 6] = [0; 6];

        // Seek to the proper position and read into the temp buffer
        // TODO this must be done safer.. what if it doesn't exist?
        file.seek(SeekFrom::Start(id as u64 * 6u64));
        file.read(&mut tmp);

        // Decode the size and offset from the temp buffer
        let size: u32 = ((tmp[0] as u32) << 16) | ((tmp[1] as u32) << 8) | (tmp[2] as u32);
        let offset: u64 = ((tmp[3] as u64) << 16) | ((tmp[4] as u64) << 8) | (tmp[5] as u64);

        Some(IndexEntry {id: id, size: size, offset: offset}).as_mut()
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
                indices.insert(idx, IndexFile {file: File::open(e.path()).unwrap()});
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
    pub fn index(&mut self, index: u32) -> Option<&IndexFile> {
        self.indices.get(&index)
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
        file.seek(SeekFrom::Start(block as u64 * 520u64));
        file.read(&mut data);

        return Some(data);
    }
}
