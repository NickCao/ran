use nom::branch::alt;
use nom::combinator::map;
use nom::combinator::opt;
use nom::multi::many0;
use nom::number::streaming::le_u64;
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{bytes::streaming::*, IResult};
use std::ffi::OsStr;
use std::fmt::Display;
use std::os::unix::prelude::OsStrExt;
use std::path::Path;

#[derive(Debug)]
pub enum Entry<'a> {
    Regular(bool, &'a [u8]),
    Symlink(&'a Path),
    Directory(Vec<(&'a Path, Entry<'a>)>),
}

impl Display for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regular(executable, _) => write!(f, "regular (executable {})", executable),
            Self::Symlink(target) => write!(f, "symlink (target {:?})", target),
            Self::Directory(entries) => {
                write!(f, "(directory (")?;
                for entry in entries {
                    write!(f, "((name {:?}) ({}))", entry.0, entry.1)?;
                }
                write!(f, "))")
            }
        }
    }
}

pub fn padding(n: usize) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    let n = if n % 8 == 0 { 0 } else { 8 - n % 8 };
    move |i: &[u8]| take(n)(i)
}

pub fn padded_bytes(i: &[u8]) -> IResult<&[u8], &[u8]> {
    let (i, len) = le_u64(i)?;
    let (i, dat) = take(len)(i)?;
    let (i, _) = padding(len as usize)(i)?;
    Ok((i, dat))
}

pub fn padded_tag(t: &str) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> + '_ {
    let t = t.clone();
    move |i: &[u8]| {
        delimited(
            tag((t.len() as u64).to_le_bytes()),
            tag(t),
            padding(t.len()),
        )(i)
    }
}

fn directory(i: &[u8]) -> IResult<&[u8], Entry> {
    preceded(
        pair(padded_tag("type"), padded_tag("directory")),
        map(
            many0(delimited(
                pair(padded_tag("entry"), padded_tag("(")),
                pair(
                    map(preceded(padded_tag("name"), padded_bytes), |x| {
                        Path::new(OsStr::from_bytes(x))
                    }),
                    preceded(padded_tag("node"), entry),
                ),
                padded_tag(")"),
            )),
            Entry::Directory,
        ),
    )(i)
}

fn entry(i: &[u8]) -> IResult<&[u8], Entry> {
    delimited(
        padded_tag("("),
        alt((regular, symlink, directory)),
        padded_tag(")"),
    )(i)
}

fn symlink(i: &[u8]) -> IResult<&[u8], Entry> {
    preceded(
        pair(padded_tag("type"), padded_tag("symlink")),
        preceded(
            padded_tag("target"),
            map(padded_bytes, |target| {
                Entry::Symlink(&Path::new(OsStr::from_bytes(target)))
            }),
        ),
    )(i)
}

fn regular(i: &[u8]) -> IResult<&[u8], Entry> {
    preceded(
        pair(padded_tag("type"), padded_tag("regular")),
        pair(
            opt(terminated(padded_tag("executable"), padded_tag(""))),
            preceded(padded_tag("contents"), padded_bytes),
        ),
    )(i)
    .map(|(i, (executable, content))| (i, Entry::Regular(executable.is_some(), content)))
}

pub fn nar(i: &[u8]) -> IResult<&[u8], Entry> {
    preceded(padded_tag("nix-archive-1"), entry)(i)
}

#[cfg(test)]
mod test {
    use crate::*;
    #[test]
    fn demo() {
        println!("{}", nar(include_bytes!("../hello.nar")).unwrap().1);
    }
}
