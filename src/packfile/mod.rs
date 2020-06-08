pub mod refs;
use std::io::{Error, ErrorKind, Read, Result as IOResult};
use byteorder::{BigEndian, ReadBytesExt};
use url::form_urlencoded::Parse;


#[derive(Debug, PartialEq)]
enum ParseState {
    Init,
    ParseEntryHeader(usize),
    ParseEntryBody(u8, usize, usize),
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
    compressed_objects: usize,
    checksum: Vec<u8>,
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
            compressed_objects: 0,
            checksum: vec![0; 20],
        }
    }

    pub(crate) fn add_line(&mut self, line: &[u8]) -> IOResult<()> {
        match line[0] {
            1 => {
                self.lines += 1;
                self.packfile_data.extend_from_slice(&line[1..]);
                self.size += line.len();
                return self.parse();
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

    pub(crate) fn parse(&mut self) -> IOResult<()> {
        println!("PARSE, state = {:?}", self.state);
        match self.state {
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
                self.state = ParseState::ParseEntryHeader(13);
                return Ok(())
                }
            ParseState::ParseEntryHeader(offset) => {
                let mut data: &[u8] = &self.packfile_data[offset..];
                let mut c = data.read_u8()?;
                let type_id = (c >> 4) & 7;
                let mut size: usize = (c & 0x0F) as usize;
                let mut shift: usize = 4;
                // Parse the variable length size header for the object.
                // Read the MSB and check if we need to continue
                // consuming bytes to get the object size
                while c & 0x80 > 0 {
                    c = data.read_u8()?;
                    size += ((c & 0x7f) as usize) << shift;
                    shift += 7;
                }
                assert!(type_id > 0 && type_id <= 7);
                assert_ne!(type_id, 5);
                self.state = ParseState::ParseEntryBody(type_id, offset + 3, size);
                return Ok(());
            }
            ParseState::ParseEntryBody(type_id, offset, size) => {
                println!("PARSE ENTRY BODY {}, {}, {}", type_id, offset, size);
                if self.size >= offset + size {
                    let mut data: &[u8] = &self.packfile_data[offset..offset + size + 1];
                    let mut pkt = vec![0u8; size];
                    data.read_exact(&mut pkt)?;
                    self.add_compressed_object(type_id, &pkt);
                    if self.entries == self.compressed_objects {
                        self.state = ParseState::ParseCheckSum(offset+size+1);
                    } else {
                        self.state = ParseState::ParseEntryHeader(offset+size+1);
                    }
                }
                return Ok(())
            }
            ParseState::ParseCheckSum(offset) => {
                println!("CHECKSUM ({})", offset);
                let mut data: &[u8] = &self.packfile_data[offset..];
                data.read_exact(&mut self.checksum)?;
                self.state = ParseState::End;
                return Ok(());
            }
            _ => return Ok(())
        }

    }

    fn print_remote_message(&self, msg: &str) {
        println!("{}", msg);
    }

    fn print_remote_error(&self, msg: &str) {
        println!("{}", msg);
    }

    pub fn count_objects(&self) -> usize {
        self.compressed_objects
    }

    fn add_compressed_object(&mut self, type_id: u8, data: &[u8]) {
        println!("add_compresed_object! type_id = {}, data.len = {}", type_id, data.len());
        self.compressed_objects += 1;
    }

    pub fn eof(&self) -> bool {
        self.state == ParseState::End
    }
}
