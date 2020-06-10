use super::{PackFile, PackObject};
use crate::packfile::index::PackIndex;
use crate::store::object::{GitObject, GitObjectType};
use crate::utils::sha1_hash_hex;
use byteorder::{BigEndian, ReadBytesExt};
use flate2::{Decompress, FlushDecompress, Status};
use num_traits::cast::FromPrimitive;
use std::io::{Error, ErrorKind, Read, Result as IOResult};
use crate::packfile::HEADER_LENGTH;

#[derive(Debug, PartialEq)]
enum ParseState {
    Init,
    ParseEntryHeader(usize),
    ParseEntryBody(u8, usize, usize, usize),
    ParseCheckSum(usize),
    End,
}

pub struct PackFileParser {
    packfile_data: Vec<u8>,
    lines: usize,
    size: usize,
    version: u32,
    entries: usize,
    state: ParseState,
    checksum: [u8; 20],
    objects: Vec<(usize, u32, PackObject)>,
}

const MAGIC_HEADER: u32 = 1_346_454_347; // "PACK"
const GIT_VERSION: u32 = 2;

impl PackFileParser {
    pub fn new() -> Self {
        PackFileParser {
            packfile_data: Vec::new(),
            lines: 0,
            size: 0,
            version: 0,
            entries: 0,
            state: ParseState::Init,
            checksum: [0; 20],
            objects: vec![],
        }
    }

    pub fn from_contents(contents: &[u8]) -> Self {
        PackFileParser {
            packfile_data: contents.to_vec(),
            lines: 0,
            size: contents.len(),
            version: 0,
            entries: 0,
            state: ParseState::Init,
            checksum: [0; 20],
            objects: vec![],
        }
    }

    pub fn parse(&mut self, dir: Option<&str>, index_opt: Option<PackIndex>) -> IOResult<PackFile> {
        let sha_computed = sha1_hash_hex(&self.checksum);
        let objects = self
            .objects
            .iter()
            .filter_map(|o| match o {
                (s, c, PackObject::Base(obj)) => Some((*s, *c, obj.clone())),
                _ => None,
            })
            .collect();
        let refs_deltas = self.objects.iter().filter_map(|o|{

        })
        let index = index_opt.unwrap_or(PackIndex::from_objects(objects, &sha_computed, dir));
        Ok(PackFile {
            version: self.version,
            num_objects: self.entries,
            encoded_objects: self.packfile_data[HEADER_LENGTH..self.packfile_data.len() - 20].to_vec(),
            hexsha: sha_computed,
            index,
        })
    }
    pub(crate) fn add_line(&mut self, line: &[u8]) -> IOResult<()> {
        match line[0] {
            1 => {
                self.lines += 1;
                self.packfile_data.extend_from_slice(&line[1..]);
                self.size += line.len();
                return self.process_line();
            }
            2 => {
                self.print_remote_message(std::str::from_utf8(&line[1..]).unwrap());
            }
            3 => {
                self.print_remote_error(std::str::from_utf8(&line[1..]).unwrap());
            }
            _ => return Err(Error::new(ErrorKind::Other, "Git server returned error")),
        }
        Ok(())
    }

    pub(crate) fn process_pending_lines(&mut self) -> IOResult<()> {
        while !self.eof() {
            self.process_line()?;
        }
        println!("pack file parse eof");
        Ok(())
    }

    pub(crate) fn slurp(&mut self) -> IOResult<()> {
        return self.process_pending_lines();
    }

    pub(crate) fn process_line(&mut self) -> IOResult<()> {
        println!("process_line state = {:?}", self.state);
        return match self.state {
            ParseState::Init => {
                let mut data: &[u8] = &self.packfile_data[0..12];
                let magic = data.read_u32::<BigEndian>()?;
                if magic != MAGIC_HEADER {
                    return Err(Error::new(ErrorKind::Other, "Magic Header Not Found"));
                }
                self.version = data.read_u32::<BigEndian>()?;
                if self.version != GIT_VERSION {
                    return Err(Error::new(ErrorKind::Other, "Unsupported version"));
                }
                self.entries = data.read_u32::<BigEndian>()? as usize;
                self.state = ParseState::ParseEntryHeader(12);
                Ok(())
            }
            ParseState::ParseEntryHeader(offset) => {
                let mut data: &[u8] = &self.packfile_data[offset..];
                let mut c = data.read_u8()?;
                let type_id = (c >> 4) & 7;
                let mut size: usize = (c & 0x0F) as usize;
                let mut shift: usize = 4;
                let mut pos = offset + 1;
                // Parse the variable length size header for the object.
                // Read the MSB and check if we need to continue
                // consuming bytes to get the object size
                while (c & 0x80) > 0 {
                    c = data.read_u8()?;
                    pos += 1;
                    size += ((c & 0x7f) as usize) << shift;
                    shift += 7;
                }
                assert!(type_id > 0 && type_id <= 7);
                assert_ne!(type_id, 5);
                self.state = ParseState::ParseEntryBody(type_id, offset, pos, size);
                Ok(())
            }
            ParseState::ParseEntryBody(type_id, offset, pos, size) => {
                if self.size > pos {
                    let (obj, consumed) = self.parse_object_content(type_id, pos, size)?;
                    println!(
                        "consume = {}, size = {}, type_id={}, pos={}, offset={}",
                        consumed, size, type_id, pos, offset
                    );
                    self.add_object(offset, obj);
                    if self.entries == self.objects.len() {
                        self.state = ParseState::ParseCheckSum(pos + consumed);
                    } else {
                        self.state = ParseState::ParseEntryHeader(pos + consumed);
                    }
                }
                Ok(())
            }
            ParseState::ParseCheckSum(pos) => {
                let mut data: &[u8] = &self.packfile_data[pos..];
                data.read_exact(&mut self.checksum)?;
                self.state = ParseState::End;
                Ok(())
            }
            _ => Ok(()),
        };
    }

    fn parse_object_content(
        &mut self,
        type_id: u8,
        pos: usize,
        size: usize,
    ) -> IOResult<(PackObject, usize)> {
        let err = &format!("unexpected id: {} for git object", type_id)[..];
        match type_id {
            1 | 2 | 3 | 4 => {
                let (content, consumed) = self.read_object_content(pos, size)?;
                let base_type: GitObjectType =
                    GitObjectType::from_u8(type_id).ok_or(Error::new(ErrorKind::Other, err))?;
                Ok((
                    PackObject::Base(GitObject::new(base_type, content)),
                    consumed,
                ))
            }
            6 => {
                let (ref_offset, consumed1) = self.read_offset(pos)?;
                println!("ref_offset = {}, consumed1 = {}", ref_offset, consumed1);
                let (content, consumed2) = self.read_object_content(pos + consumed1, size)?;
                Ok((
                    PackObject::OfsDelta(ref_offset, content),
                    consumed1 + consumed2,
                ))
            }
            7 => {
                let mut base: [u8; 20] = [0; 20];
                let mut data: &[u8] = &self.packfile_data[pos..];
                data.read_exact(&mut base)?;
                let (content, consumed) = self.read_object_content(pos + 20, size)?;
                Ok((PackObject::RefDelta(base, content), consumed + 20))
            }
            _ => {
                let err = &format!("unexpected id: {} for git object", type_id)[..];
                Err(Error::new(ErrorKind::Other, err))
            }
        }
    }

    fn read_offset(&mut self, pos: usize) -> IOResult<(usize, usize)> {
        let mut data: &[u8] = &self.packfile_data[pos..];
        let mut c = data.read_u8()?;
        let mut offset = (c & 0x7f) as usize;
        let mut consumed = 1;
        while c & 0x80 != 0 {
            c = data.read_u8()?;
            consumed += 1;
            offset += 1;
            offset <<= 7;
            offset += (c & 0x7f) as usize;
        }
        Ok((offset, consumed))
    }

    fn read_object_content(&mut self, offset: usize, size: usize) -> IOResult<(Vec<u8>, usize)> {
        let mut decompressor = Decompress::new(true);
        let mut object_buffer = Vec::with_capacity(size);
        let mut consumed = 0;
        let mut pos = offset;
        loop {
            let last_total_in = decompressor.total_in();
            let res = {
                let zlib_buffer = &self.packfile_data[pos..];
                decompressor.decompress_vec(zlib_buffer, &mut object_buffer, FlushDecompress::None)
            };
            let nread = (decompressor.total_in() - last_total_in) as usize;
            pos += nread;
            consumed += nread;
            match res {
                Ok(Status::StreamEnd) => {
                    if decompressor.total_out() as usize != size {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Size does not match for expected object contents",
                        ));
                    }

                    return Ok((object_buffer, consumed));
                }
                Ok(Status::BufError) => {
                    return Err(Error::new(
                        ErrorKind::Other,
                        "Encountered zlib buffer error",
                    ))
                }
                Ok(Status::Ok) => (),
                Err(e) => {
                    let s = &format!("Encountered zlib decompression error: {}", e)[..];
                    return Err(Error::new(ErrorKind::Other, s));
                }
            }
        }
    }

    fn print_remote_message(&self, msg: &str) {
        println!("{}", msg);
    }

    fn print_remote_error(&self, msg: &str) {
        println!("{}", msg);
    }

    pub fn count_objects(&self) -> usize {
        self.objects.len()
    }

    fn add_object(&mut self, offset: usize, object: PackObject) {
        println!("add_object(offset={}, object_count={})", offset, self.objects.len());
        self.objects.push((offset, object.crc32(), object));
    }

    pub fn eof(&self) -> bool {
        self.state == ParseState::End
    }
}
