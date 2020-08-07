use super::object::GitObject;
use chrono::naive::NaiveDateTime;
use chrono::{DateTime, FixedOffset};
use nom::character::complete::{digit1, line_ending, newline, space0, space1};
use nom::combinator::rest;
use std::str::from_utf8;
use std::str::{self, FromStr};

#[derive(Debug, Clone)]
pub struct Person<'a> {
    name: &'a str,
    email: &'a str,
    timestamp: DateTime<FixedOffset>,
}

#[derive(Debug, Clone)]
pub struct Commit<'a> {
    pub tree: &'a str,
    pub parents: Vec<&'a str>,
    author: Person<'a>,
    committer: Person<'a>,
    message: &'a str,
    raw: &'a GitObject,
}

impl<'a> Commit<'a> {

    pub fn has_parents(&self) -> bool {
        !self.parents.is_empty()
    }

    pub fn from_raw(obj: &'a GitObject) -> Option<Self> {
        parse_commit_inner(&obj.content).ok().map(|(_, raw_parts)| {
            let (tree, parents, author, committer, message) = raw_parts;
            Commit {
                tree,
                parents,
                author,
                committer,
                message,
                raw: obj,
            }
        })
    }

    pub fn get_message(&self) -> String {
        self.message.to_string()
    }
}

named!(u64_digit(&[u8]) -> u64,
    map_res!(
        dbg!(map_res!(
            digit1,
            std::str::from_utf8
        )),
    FromStr::from_str)
);

named!(i32_digit(&[u8]) -> i32,
    map_res!(
        map_res!(
            digit1,
            from_utf8
        ),
    FromStr::from_str)
);

named!(parse_person(&[u8]) -> Person,
    do_parse!(
        name: map_res!(take_until!(" <"), from_utf8) >>
        take!(2) >>
        email: map_res!(take_until!("> "), from_utf8) >>
        take!(2) >> space0 >>
        ts: u64_digit >>
        space1 >>
        sign: alt!(char!('+') | char!('-')) >>
        tz: i32_digit >>
        newline >>
        ({
            let sgn = if sign == '-' {
                -1
            } else {
                1
            };
            let naive = NaiveDateTime::from_timestamp(ts as i64, 0);
            let offset = FixedOffset::east(sgn * tz/100 * 3600);
            let datetime = DateTime::from_utc(naive, offset);
            Person {
                name: name,
                email: email,
                timestamp: datetime
            }
        })
    )
);

named!(parse_commit_inner(&[u8]) -> (&str, Vec<&str>, Person, Person, &str),
  do_parse!(
    tag!("tree ") >>
    tree: map_res!(take!(40), from_utf8) >>
    newline >>
    parents: many0!(
        do_parse!(
            tag!("parent ") >>
            parent: map_res!(take!(40), from_utf8) >>
            newline >>
            ( parent )
        )
    ) >>
    tag!("author ") >>
    author: parse_person >>
    tag!("committer ") >>
    committer: parse_person >>
    line_ending >>
    message: map_res!(rest, from_utf8) >>
    ( {
        (tree, parents, author, committer, message)
    }
    )
  )
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::object::{GitObject, GitObjectType};
    use nom::IResult;

    #[test]
    fn test_person_parsing() {
        let input = b"The Author <author@devs.com> 1353116070 +1100\n";

        if let IResult::Ok((_, person)) = parse_person(&input[..]) {
            assert_eq!(person.name, "The Author");
            assert_eq!(person.email, "author@devs.com");
        }
    }

    #[test]
    fn test_commit_parsing() {
        let input = b"tree asdf456789012345678901234567890123456789\n\
            parent parentone012345678901234567890123456789a\n\
            parent parenttwo012345678901234567890123456789b\n\
            author The Author <author@devs.com> 1353116070 +1100\n\
            committer The Committer <commiter@devs.com> 1353116070 +1100\n\
            \n\
            Bump version to 1.6";
        let input2 = b"tree 9f5829a852fcd8e3381e343b45cb1c9ff33abf56\nauthor Christian Briones <christian@whisper.sh> 1418004896 -0800\ncommitter Christian Briones <christian@whisper.sh> 1418004914 -0800\n\ninit\n";
        let object = GitObject::new(GitObjectType::Commit, (&input[..]).to_owned());
        if let Some(commit) = Commit::from_raw(&object) {
            assert_eq!(commit.tree, "asdf456789012345678901234567890123456789");
            let parents = vec![
                "parentone012345678901234567890123456789a",
                "parenttwo012345678901234567890123456789b",
            ];
            assert_eq!(commit.parents, parents);
            assert_eq!(commit.message, "Bump version to 1.6");
        } else {
            panic!("Failed to parse commit.");
        }

        let object2 = GitObject::new(GitObjectType::Commit, (&input2[..]).to_owned());
        assert!(Commit::from_raw(&object2).is_some())
    }
}
