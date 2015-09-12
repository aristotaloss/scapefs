pub mod filesystem;

pub use filesystem::{FileSystem, FsError, MainFile};

#[test]
fn it_works() {
    let mut fs = FileSystem::new("fstest").unwrap();
    println!("FileSystem: {:?}", fs);
    let ref mut mf = fs.mainfile();
    assert!(mf.exists());
    assert!(mf.num_blocks().unwrap() == 88216);
    let data = mf.read_block(1);
    print_vec(data.unwrap());
}

fn print_vec(v: [u8; 520]) {
    for i in v.iter() {
        println!("{0:x}", *i)
    }
}
