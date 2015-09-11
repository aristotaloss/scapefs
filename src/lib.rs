pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
	let fs = FileSystem::new("C:\\cache_osrs_92").unwrap();
	println!("AYYYYY LMAOOO {:?}", fs);
	let mf = fs.mainfile();
	println!("Wow the mainfile {:?}", mf);
}
