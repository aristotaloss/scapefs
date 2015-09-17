pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let mut fs = FileSystem::new("/home/bart/eocache/data 845").unwrap();
    println!("{:?}", fs.index(2).unwrap().entry(69));
}

fn print_vec(v: [u8; 520]) {
    for i in v.iter() {
        print!("{0:X} ", *i);
    }
    println!("");
}
