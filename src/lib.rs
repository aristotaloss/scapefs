pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let mut fs = FileSystem::new("fstest").unwrap();
    let varbit_entry = fs.index(2).unwrap().entry(69).unwrap();
    let varbit_header = fs.mainfile().read_header(varbit_entry);
    println!("Entry header: {:?}", varbit_header);
}

fn print_vec(v: [u8; 520]) {
    for i in v.iter() {
        print!("{0:X} ", *i);
    }
    println!("");
}
