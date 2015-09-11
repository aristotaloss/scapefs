pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let fs = FileSystem::new("fstest").unwrap();
    println!("FileSystem: {:?}", fs);
    let mf = fs.mainfile();
}
