pub mod refs;
pub mod index;
pub mod packfile_parser;
use std::io::{Error, ErrorKind, Read, Write, Result as IOResult};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::{Decompress, FlushDecompress, Status};
use crate::store::object::{GitObject, GitObjectType};
use num_traits::cast::FromPrimitive;
use crc::crc32;
use index::PackIndex;
use std::path::PathBuf;
use std::fs;
use std::fs::File;
use crate::utils::sha1_hash;

pub const MAGIC_HEADER: u32 = 1_346_454_347; // "PACK"
const HEADER_LENGTH: usize = 12; // Magic + Len + Version

pub struct PackFile {
    version: u32,
    num_objects: usize,
    encoded_objects: Vec<u8>,
    hexsha: String,
    index: PackIndex,
}

impl PackFile {

    pub fn write(&self, dir: &str) -> IOResult<()>{
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

