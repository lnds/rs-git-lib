use crate::packfile::refs::{Ref, Refs};
use std::collections::HashMap;
use std::io::{Error, ErrorKind, Read, Result as IOResult};
use crate::packfile::packfile_parser::PackFileParser;

pub(crate) const GIT_UPLOAD_PACK_HEADER: &[u8; 26] = b"# service=git-upload-pack\n";
pub(crate) const GIT_FLUSH_HEADER: &[u8; 4] = b"0000";

pub(crate) fn read_packet_line<R: Read>(reader: &mut R) -> IOResult<Option<Vec<u8>>> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length_str = std::str::from_utf8(&header[..]).unwrap_or("");
    let length = u64::from_str_radix(length_str, 16).unwrap_or(0);
    if length <= 4 {
        Ok(None)
    } else {
        let mut pkt = vec![0; (length - 4) as usize];
        reader.read_exact(&mut pkt)?;
        Ok(Some(pkt))
    }
}

pub(crate) fn read_flush_packet<R: Read>(reader: &mut R) -> IOResult<Option<Vec<u8>>> {
    let mut flush: [u8; 4] = [0; 4];
    reader.read_exact(&mut flush)?;
    Ok(Some(flush.to_vec()))
}

pub(crate) fn receive_packet<R: Read>(reader: &mut R) -> IOResult<Vec<String>> {
    let mut lines = vec![];
    loop {
        match read_packet_line(reader) {
            Ok(Some(line)) => {
                let s: String = std::str::from_utf8(&line[..]).unwrap().to_owned();
                lines.push(s)
            }
            Ok(None) => return Ok(lines),
            Err(e) => return Err(e),
        }
    }
}

pub(crate) fn parse_refs_lines(lines: &[String]) -> IOResult<Refs> {
    if lines.len() <= 1 {
        return Err(Error::new(
            ErrorKind::Other,
            "parse_refs_lines need at least 1 line",
        ));
    }

    let mut iter = lines.iter().map(|s| s.trim_end());

    // First line contains capabilities separated by '\0'
    let mut parsed = Vec::new();
    let first = iter.next().unwrap();
    let (_capabilities, first_ref) = parse_ref_first_line(first);
    parsed.push(first_ref);
    for line in iter {
        parsed.push(parse_ref_line(line))
    }
    Ok(parsed)
}

fn parse_ref_first_line(line: &str) -> (Vec<String>, Ref) {
    let split = line.split('\0').collect::<Vec<_>>();
    let the_ref = parse_ref_line(split[0]);
    let capabilities = split[1]
        .split(' ')
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();
    (capabilities, the_ref)
}

fn parse_ref_line(line: &str) -> Ref {
    let split = line.split(' ').collect::<Vec<_>>();

    let (obj_id, name) = (split[0], split[1]);
    Ref {
        id: obj_id.to_owned(),
        name: name.to_owned(),
    }
}

pub(crate) fn create_packfile_negotiation_request(capabilities: &[&str], refs: &[Ref]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(refs.len());
    let mut ids: HashMap<String, ()> = HashMap::new();
    for (i, r) in refs.iter().enumerate() {
        let &Ref { id: ref o, .. } = r;
        if ids.contains_key(&r.id) {
            continue;
        }
        ids.insert(r.id.to_string(), ());
        if i == 0 {
            let caps = capabilities.join(" ");
            // if this is a space it is correctly multiplexed
            let line: String = ["want ", &o[..], " ", &caps[..], "\n"].concat();
            lines.push(packet_line(&line[..]));
        } else {
            let line: String = ["want ", &o[..], "\n"].concat();
            lines.push(packet_line(&line[..]));
        }
    }
    lines.push(flush_packet());
    lines.push(packet_line("done\n"));
    lines.concat()
}

fn packet_line(msg: &str) -> String {
    format!("{:04x}{}", 4 + msg.len(), msg)
}

fn flush_packet() -> String {
    format!("{:04x}", 0)
}

pub(crate) fn receive_packet_file_with_sideband<R: Read>(
    reader: &mut R,
) -> IOResult<PackFileParser> {
    let mut parser = PackFileParser::new();
    while let Some(line) = read_packet_line(reader)? {
        if &line[..] != b"NAK\n" {
            parser.add_line(&line)?;
        }
    }
    parser.process_pending_lines()?;
    Ok(parser)
}
