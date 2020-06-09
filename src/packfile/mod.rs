pub mod refs;
pub mod index;
pub mod packfile_parser;
use std::io::{Error, ErrorKind, Read, Result as IOResult};
use byteorder::{BigEndian, ReadBytesExt};
use flate2::{Decompress, FlushDecompress, Status};
use crate::store::object::{GitObject, GitObjectType};
use num_traits::cast::FromPrimitive;
use crc::crc32;
use index::PackIndex;

pub struct PackFile {
    version: u32,
    num_objects: usize,
    encoded_objects: Vec<u8>,
    hexsha: String,
    index: PackIndex,
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

