
extern crate erfs_rt;

pub mod erfs_gensrc;

#[cfg(test)]
mod tests {
    use super::*;
    use erfs_rt::*;

    #[test]
    fn test_read() {
        let fs = erfs_gensrc::erfs_root();
        let file = "/tests/erfsgen_it.rs";
        let data = read(fs, file).expect("read error");
        let data = String::from_utf8_lossy(data);
        println!("read content of {}: {:?}", file, data);
    }

    #[test]
    fn test_open_file() {
        let fs = erfs_gensrc::erfs_root();
        let file = "/src/lib.rs";
        let (handle, size) = open(fs, file).expect("open error");
        println!("size of {}: {}", file, size);

        let name = entry_name(fs, handle).unwrap();
        let name = String::from_utf8_lossy(name);
        println!("name of {}: {}", file, name);

        let flags = entry_flags(handle).unwrap();
        println!("flags of {}: {}", file, flags);
    }


    #[test]
    fn test_open_file1() {
        let fs = erfs_gensrc::erfs_root();
        let file = "/tests/erfsgen_it.rs";
        let (handle, size) = open(fs, file).expect("open error");
        println!("size of {}: {}", file, size);

        let name = entry_name(fs, handle).unwrap();
        let name = String::from_utf8_lossy(name);
        println!("name of {}: {}", file, name);

        let flags = entry_flags(handle).unwrap();
        println!("flags of {}: {}", file, flags);

        let data = read_file(fs, handle).unwrap();
        let data = String::from_utf8_lossy(data);
        println!("content of {}: {}", file, data);
    }

    #[test]
    fn test_open_dir() {
        let fs = erfs_gensrc::erfs_root();
        let file = "/src";
        let (handle, size) = open(fs, file).expect("open error");
        println!("size of {}: {}", file, size);

        let name = entry_name(fs, handle).unwrap();
        let name = String::from_utf8_lossy(name);
        println!("name of {}: {}", file, name);

        let flags = entry_flags(handle).unwrap();
        println!("flags of {}: {}", file, flags);

        // read first child
        let entry = read_dir(fs, handle, 0).unwrap();

        let name = entry_name(fs, entry).unwrap();
        let name = String::from_utf8_lossy(name);
        println!("name of child [0]: {}", name);

        let flags = entry_flags(entry).unwrap();
        println!("flags of child [0]: {}", flags);
    }
}