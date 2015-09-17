pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let mut fs = FileSystem::new("/home/bart/eocache/data 845").unwrap();
    let mut x = fs.index(2).unwrap();
    let mut y = x.entry(69);
    {
    //    println!("{:?}", fs.index(2).as_mut().unwrap().entry(69));
    }
    //println!("Index 2: {:?} {}", index, index.last_entry());

    //println!("{:?}", mutfile.entry(69));
}

fn print_vec(v: [u8; 520]) {
    for i in v.iter() {
        print!("{0:X} ", *i);
    }
    println!("");
}
