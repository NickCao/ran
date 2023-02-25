use nom::branch::alt;
use nom::combinator::opt;
use nom::multi::many0;
use nom::number::streaming::le_u64;
use nom::sequence::{delimited, pair, preceded, terminated};
use nom::{bytes::streaming::*, IResult};

#[derive(Debug)]
pub enum Entry<'a> {
    Regular(bool, &'a [u8]),
    Symlink(&'a [u8]),
    Directory(Vec<(&'a [u8], Entry<'a>)>),
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
        many0(delimited(
            pair(padded_tag("entry"), padded_tag("(")),
            pair(
                preceded(padded_tag("name"), padded_bytes),
                preceded(padded_tag("node"), entry),
            ),
            padded_tag(")"),
        )),
    )(i)
    .map(|(i, entries)| (i, Entry::Directory(entries)))
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
        padded_bytes,
    )(i)
    .map(|(i, target)| (i, Entry::Symlink(target)))
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
        dbg!(nar(include_bytes!("../hello.nar")).is_ok());
    }
}
