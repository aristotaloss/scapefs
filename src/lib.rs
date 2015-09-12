pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let ref mut fs = FileSystem::new("fstest").unwrap();
    println!("FileSystem: {:?}", fs);
    {
        let ref mut mf = fs.mainfile();
        assert!(mf.exists());
        assert!(mf.num_blocks().unwrap() == 88216);
        let data = mf.read_block(1);
        print_vec(data.unwrap());
    }

    let index = fs.index(&mut 2).unwrap();
    println!("Index 2: {:?} {}", index, index.last_entry());
}

fn print_vec(v: [u8; 520]) {
    for i in v.iter() {
        print!("{0:X} ", *i);
    }
    println!("");
}
