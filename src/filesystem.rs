use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::fs;
use std::fmt;
use std::io::{Seek, Read, SeekFrom};

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
    mainfile: MainFile
}

#[derive(Debug)]
pub struct MainFile {
    file: Option<File>
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

        let file = File::open(mainfile_path).ok();
        Ok(FileSystem{path: path, mainfile: MainFile{file: file}})
    }

    /// Gets the mainfile, that is, the main_file_cache.dat2 entry in the folder
    /// that holds the actual binary data of the filesystem entries.
    pub fn mainfile(&self) -> &MainFile {
        &self.mainfile
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
