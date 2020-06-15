pub mod index;
pub mod packfile_parser;
pub mod refs;
use crate::packfile::packfile_parser::PackFileParser;
use crate::store::object::GitObject;
use crate::utils::sha1_hash;
use byteorder::{BigEndian, WriteBytesExt};
use crc::crc32;
use index::PackIndex;
use nom::lib::std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Result as IOResult, Write};
use std::path::{Path, PathBuf};

pub const MAGIC_HEADER: u32 = 1_346_454_347; // "PACK"
const HEADER_LENGTH: usize = 12; // Magic + Len + Version

pub struct PackFile {
    version: u32,
    num_objects: usize,
    encoded_objects: Vec<u8>,
    hexsha: String,
    pub index: PackIndex,
    objects: HashMap<String, GitObject>,
    //offset_objects: HashMap<usize, GitObject>,
}

impl PackFile {
    #[allow(dead_code)]
    pub fn open<P: AsRef<Path>>(p: P) -> IOResult<Self> {
        let path = p.as_ref();
        let mut contents = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut contents)?;

        let idx_path = path.with_extension("idx");
        let idx = PackIndex::open(idx_path)?;
        PackFile::parse_with_index(&contents, idx, None)
    }

    fn parse_with_index(
        contents: &[u8],
        idx: Option<PackIndex>,
        dir: Option<&str>,
    ) -> IOResult<Self> {
        let mut parser = PackFileParser::from_contents(contents);
        parser.slurp()?;
        parser.parse(dir, idx)
    }

    pub fn write(&self, dir: &str) -> IOResult<()> {
        let mut root = PathBuf::new();
        root.push(dir);
        root.push(".git");
        self.write_to_path(&root)
    }

    fn write_to_path(&self, root: &PathBuf) -> IOResult<()> {
        let mut path = root.clone();
        path.push("objects/pack");
        fs::create_dir_all(&path)?;
        path.push(format!("pack-{}", self.sha()));
        path.set_extension("pack");

        let mut pack_file = File::create(&path)?;

        let pack = self.encode()?;
        pack_file.write_all(&pack)?;

        Ok(())
    }

    pub fn encode(&self) -> IOResult<Vec<u8>> {
        let mut encoded = Vec::with_capacity(HEADER_LENGTH + self.encoded_objects.len());
        encoded.write_u32::<BigEndian>(MAGIC_HEADER)?;
        encoded.write_u32::<BigEndian>(self.version)?;
        encoded.write_u32::<BigEndian>(self.num_objects as u32)?;
        encoded.write_all(&self.encoded_objects[..])?;
        let checksum = sha1_hash(&encoded);
        encoded.write_all(&checksum[..])?;
        Ok(encoded)
    }

    pub fn sha(&self) -> &str {
        &self.hexsha
    }

    pub fn find_by_sha(&self, sha: &str) -> IOResult<Option<GitObject>> {
        Ok(self.objects.get(sha).cloned())
    }


}

#[derive(Debug)]
pub enum PackObject {
    Base(GitObject),
    OfsDelta(usize, Vec<u8>),
    RefDelta([u8; 20], Vec<u8>),
}

impl PackObject {
    pub fn crc32(&self) -> u32 {
        let content = match *self {
            PackObject::Base(ref b) => &b.content[..][..],
            PackObject::RefDelta(_, ref c) => &c[..],
            PackObject::OfsDelta(_, ref c) => &c[..],
        };
        crc32::checksum_ieee(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    static PACK_FILE: &'static str =
        "tests/data/packs/pack-79f006bb5e8d079fdbe07e7ce41f97f4db7d341c.pack";

    static BASE_SHA: &'static str = "7e690abcc93718dbf26ddea5c6ede644a63a5b34";
    // We need to test reading an object with a non-trivial delta
    // chain (4).
    static DELTA_SHA: &'static str = "9b104dc31028e46f2f7d0b8a29989ab9a5155d41";
    static DELTA_CONTENT: &'static str =
        "This is a test repo, used for testing the capabilities of the rgit tool. \
        rgit is a implementation of\n\
        the Git version control tool written in Rust.\n\n\
        This line was added on test branch.\n";

    fn read_pack() -> PackFile {
        PackFile::open(PACK_FILE).unwrap()
    }

    #[test]
    fn reading_a_packfile() {
        read_pack();
    }

    #[test]
    fn read_and_encode_should_be_inverses() {
        let pack = read_pack();
        let encoded = pack.encode().unwrap();
        let mut on_disk = Vec::with_capacity(encoded.len());
        let mut file = File::open(PACK_FILE).unwrap();
        file.read_to_end(&mut on_disk).unwrap();

        assert_eq!(on_disk, encoded);
    }


    #[test]
    fn reading_a_packed_object_by_sha() {
        let pack = read_pack();
        // Read a base object
        pack.find_by_sha(BASE_SHA).unwrap().unwrap();
        // Read a deltified object
        pack.find_by_sha(DELTA_SHA).unwrap().unwrap();
    }

    #[test]
    fn reading_delta_objects_should_resolve_them_correctly() {
        use std::str;
        let pack = read_pack();
        let delta = pack.find_by_sha(DELTA_SHA).unwrap().unwrap();
        let content = str::from_utf8(&delta.content[..]).unwrap();
        assert_eq!(content, DELTA_CONTENT);
    }
}
