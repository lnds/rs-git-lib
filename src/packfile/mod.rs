pub mod index;
pub mod packfile_parser;
pub mod refs;
use crate::packfile::packfile_parser::PackFileParser;
use crate::store::object::{GitObject, GitObjectType};
use crate::utils::sha1_hash;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crc::crc32;
use flate2::{Decompress, FlushDecompress, Status};
use index::PackIndex;
use rustc_serialize::hex::{FromHex, ToHex};
use std::fs::File;
use std::io::{Read, Result as IOResult, Write};
use std::path::{Path, PathBuf};
use std::{cmp, fs};

pub const MAGIC_HEADER: u32 = 1_346_454_347; // "PACK"
const HEADER_LENGTH: usize = 12; // Magic + Len + Version

pub struct PackFile {
    version: u32,
    num_objects: usize,
    encoded_objects: Vec<u8>,
    hexsha: String,
    pub index: PackIndex,
}

impl PackFile {
    pub fn open<P: AsRef<Path>>(p: P) -> IOResult<Self> {
        let path = p.as_ref();
        let mut contents = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut contents)?;

        let idx_path = path.with_extension("idx");
        let idx = PackIndex::open(idx_path)?;
        println!("idx read");
        PackFile::parse_with_index(&contents, idx, None)
    }

    fn parse_with_index(
        contents: &[u8],
        idx: Option<PackIndex>,
        dir: Option<&str>,
    ) -> IOResult<Self> {
        let mut parser = PackFileParser::from_contents(contents);
        println!("parser");
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
        println!("find by sha {}", sha);
        let off = sha.from_hex().ok().and_then(|s| self.index.find(&s));
        println!("off = {:?}", off);
        match off {
            Some(offset) => self.find_by_offset(offset),
            None => Ok(None),
        }
    }

    fn find_by_sha_unresolved(&self, sha: &str) -> IOResult<Option<PackObject>> {
        let off = sha.from_hex().ok().and_then(|ref s| self.index.find(s));
        match off {
            Some(offset) => Ok(Some(self.read_at_offset(offset)?)),
            None => Ok(None),
        }
    }

    fn find_by_offset(&self, mut offset: usize) -> IOResult<Option<GitObject>> {
        println!("find by offset {}", offset);
        // Read the initial offset.
        //
        // If it is a base object, return the enclosing object.
        let mut object = self.read_at_offset(offset)?;
        println!("object read");
        if let PackObject::Base(base) = object {
            return Ok(Some(base));
        };
        // Otherwise we will have to recreate the delta object.
        //
        // To do this, we accumulate the entire delta chain into a vector by repeatedly
        // following the references to the next base object.
        //
        // We need to keep track of all the offsets so they are correct.
        let mut patches = Vec::new();

        while !object.is_base() {
            let next;
            match object {
                PackObject::OfsDelta(delta_offset, patch) => {
                    patches.push(patch);
                    // This offset is *relative* to its own position
                    // We don't need to store multiple chains because a delta chain
                    // will either be offsets or shas but not both.
                    offset -= delta_offset;
                    next = Some(self.read_at_offset(offset)?);
                }
                PackObject::RefDelta(sha, patch) => {
                    patches.push(patch);
                    next = self.find_by_sha_unresolved(&sha.to_hex())?;
                }
                _ => unreachable!(),
            }
            match next {
                Some(o) => object = o,
                None => return Ok(None),
            }
        }
        // The patches then look like: vec![patch3, patch2, patch1]
        //
        // These patches are then popped off the end, applied in turn to create the desired object.
        // We could cache these results along the way in some offset cache to avoid repeatedly
        // recreating the chain for any object along it, but this shouldn't be necessary
        // for most operations since we will only be concerned with the tip of the chain.
        while let Some(patch) = patches.pop() {
            object = object.patch(&patch).unwrap();
            // TODO: Cache here
        }
        Ok(Some(object.unwrap()))
    }

    fn read_at_offset(&self, offset: usize) -> IOResult<PackObject> {
        let total_offset = offset - HEADER_LENGTH ;
        let contents = &self.encoded_objects[total_offset..];
        println!("read_at_offset, total_offset = {}", total_offset);
        let mut reader = ObjectReader::new(contents);
        println!("reader!");
        reader.read_object()
    }
}

#[derive(Debug)]
pub enum PackObject {
    Base(GitObject),
    OfsDelta(usize, Vec<u8>),
    RefDelta([u8; 20], Vec<u8>),
}

impl PackObject {
    pub fn unwrap(self) -> GitObject {
        match self {
            PackObject::Base(b) => b,
            _ => panic!("Called `GitObject::unwrap` on a deltified object"),
        }
    }

    pub fn crc32(&self) -> u32 {
        let content = match *self {
            PackObject::Base(ref b) => &b.content[..][..],
            PackObject::RefDelta(_, ref c) => &c[..],
            PackObject::OfsDelta(_, ref c) => &c[..],
        };
        crc32::checksum_ieee(content)
    }

    pub fn is_base(&self) -> bool {
        if let PackObject::Base(_) = *self {
            true
        } else {
            false
        }
    }

    pub fn patch(&self, delta: &[u8]) -> Option<Self> {
        if let PackObject::Base(ref b) = *self {
            Some(PackObject::Base(b.patch(delta)))
        } else {
            None
        }
    }
}

const BUFFER_SIZE: usize = 4 * 1024;

pub struct ObjectReader<R> {
    inner: R,
    pos: usize,
    cap: usize,
    consumed_bytes: usize,
    buf: [u8; BUFFER_SIZE],
}

impl<R> ObjectReader<R>
where
    R: Read,
{
    pub fn new(inner: R) -> Self {
        ObjectReader {
            inner,
            pos: 0,
            cap: 0,
            consumed_bytes: 0,
            buf: [0; BUFFER_SIZE],
        }
    }

    pub fn read_object(&mut self) -> IOResult<PackObject> {
        let mut c = self.read_u8()?;
        let type_id = (c >> 4) & 7;

        let mut size: usize = (c & 0xf) as usize;
        let mut shift: usize = 4;

        // Parse the variable length size header for the object.
        // Read the MSB and check if we need to continue
        // consuming bytes to get the object size
        while c & 0x80 > 0 {
            c = self.read_u8()?;
            size += ((c & 0x7f) as usize) << shift;
            shift += 7;
        }
        println!("type_id = {}, size={}", type_id, size);

        match type_id {
            1 | 2 | 3 | 4 => {
                let content = self.read_object_content(size)?;
                let base_type = match type_id {
                    1 => GitObjectType::Commit,
                    2 => GitObjectType::Tree,
                    3 => GitObjectType::Blob,
                    4 => GitObjectType::Tag,
                    _ => unreachable!(),
                };
                Ok(PackObject::Base(GitObject::new(base_type, content)))
            }
            6 => {
                let offset = self.read_offset()?;
                let content = self.read_object_content(size)?;
                Ok(PackObject::OfsDelta(offset, content))
            }
            7 => {
                let mut base: [u8; 20] = [0; 20];
                self.read_exact(&mut base)?;
                let content = self.read_object_content(size)?;
                Ok(PackObject::RefDelta(base, content))
            }
            _ => panic!("Unexpected id : {} for git object", type_id),
        }
    }

    // Offset encoding.
    // n bytes with MSB set in all but the last one.
    // The offset is then the number constructed
    // by concatenating the lower 7 bits of each byte, and
    // for n >= 2 adding 2^7 + 2^14 + ... + 2^(7*(n-1))
    // to the result.
    fn read_offset(&mut self) -> IOResult<usize> {
        let mut c = self.read_u8()?;
        let mut offset = (c & 0x7f) as usize;
        while c & 0x80 != 0 {
            c = self.read_u8()?;
            offset += 1;
            offset <<= 7;
            offset += (c & 0x7f) as usize;
        }
        Ok(offset)
    }

    pub fn consumed_bytes(&self) -> usize {
        self.consumed_bytes
    }

    fn read_object_content(&mut self, size: usize) -> IOResult<Vec<u8>> {
        let mut decompressor = Decompress::new(true);
        let mut object_buffer = Vec::with_capacity(size);

        loop {
            let last_total_in = decompressor.total_in();
            let res = {
                let zlib_buffer = self.fill_buffer()?;
                decompressor.decompress_vec(zlib_buffer, &mut object_buffer, FlushDecompress::None)
            };
            let nread = decompressor.total_in() - last_total_in;
            self.consume(nread as usize);

            match res {
                Ok(Status::StreamEnd) => {
                    if decompressor.total_out() as usize != size {
                        panic!("Size does not match for expected object contents");
                    }

                    return Ok(object_buffer);
                }
                Ok(Status::BufError) => panic!("Encountered zlib buffer error"),
                Ok(Status::Ok) => (),
                Err(e) => panic!("Encountered zlib decompression error: {}", e),
            }
        }
    }

    fn fill_buffer(&mut self) -> IOResult<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the underlying reader.
        if self.pos == self.cap {
            self.cap = self.inner.read(&mut self.buf)?;
            self.pos = 0;
        }
        Ok(&self.buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.consumed_bytes += amt;
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}

impl<R: Read> Read for ObjectReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> IOResult<usize> {
        // If we don't have any buffered data and we're doing a massive read
        // (larger than our internal buffer), bypass our internal buffer
        // entirely.
        if self.pos == self.cap && buf.len() >= self.buf.len() {
            let nread = self.inner.read(buf)?;
            // We still want to keep track of the correct offset so
            // we consider this consumed.
            self.consumed_bytes += nread;
            return Ok(nread);
        }
        let nread = {
            let mut rem = self.fill_buffer()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    static PACK_FILE: &'static str =
        "tests/data/packs/pack-79f006bb5e8d079fdbe07e7ce41f97f4db7d341c.pack";

    static BASE_OFFSET: usize = 2154;
    static BASE_SHA: &'static str = "7e690abcc93718dbf26ddea5c6ede644a63a5b34";
    // We need to test reading an object with a non-trivial delta
    // chain (4).
    static DELTA_SHA: &'static str = "9b104dc31028e46f2f7d0b8a29989ab9a5155d41";
    static DELTA_OFFSET: usize = 2461;
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
    fn reading_a_packed_object_by_offset() {
        let pack = read_pack();
        // Read a base object
        pack.find_by_offset(BASE_OFFSET).unwrap().unwrap();
        // Read a deltified object
        pack.find_by_offset(DELTA_OFFSET).unwrap().unwrap();
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
        println!("pack read");
        let delta = pack.find_by_sha(DELTA_SHA).unwrap().unwrap();
        println!("after delta");
        let content = str::from_utf8(&delta.content[..]).unwrap();
        assert_eq!(content, DELTA_CONTENT);
    }
}
