use std::path::PathBuf;
use std::fs::File;
use std::error::Error;
use std::fs;
use std::fmt;

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

    pub fn file(&self) -> &Option<File> {
		&self.file
	}
}
