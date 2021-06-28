use crate::{
    BerParser, DerParser, FromBer, FromDer, ParseResult, Result, Set, SetIterator, Tag, Tagged,
    ToDer,
};
use std::borrow::Cow;

#[derive(Debug)]
pub struct SetOf<T> {
    items: Vec<T>,
}

impl<T> SetOf<T> {
    pub const fn new(items: Vec<T>) -> Self {
        SetOf { items }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<'a, T> AsRef<[T]> for SetOf<T> {
    fn as_ref(&self) -> &[T] {
        &self.items
    }
}

impl<'a, T> FromBer<'a> for SetOf<T>
where
    T: FromBer<'a>,
{
    fn from_ber(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, set) = Set::from_ber(bytes)?;
        let data = match set.content {
            Cow::Borrowed(b) => b,
            // Since 'any' is built from 'bytes', it is borrowed by construction
            _ => unreachable!(),
        };
        let v = SetIterator::<T, BerParser>::new(data).collect::<Result<Vec<T>>>()?;
        Ok((rem, SetOf::new(v)))
    }
}

impl<'a, T> FromDer<'a> for SetOf<T>
where
    T: FromDer<'a>,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, set) = Set::from_der(bytes)?;
        let data = match set.content {
            Cow::Borrowed(b) => b,
            // Since 'any' is built from 'bytes', it is borrowed by construction
            _ => unreachable!(),
        };
        let v = SetIterator::<T, DerParser>::new(data).collect::<Result<Vec<T>>>()?;
        Ok((rem, SetOf::new(v)))
    }
}

impl<T> From<SetOf<T>> for Vec<T> {
    fn from(set: SetOf<T>) -> Self {
        set.items
    }
}

impl<T> Tagged for SetOf<T> {
    const TAG: Tag = Tag::Set;
}

impl<T> ToDer for SetOf<T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        self.items.to_der_len()
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> crate::SerializeResult<usize> {
        self.items.write_der_header(writer)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> crate::SerializeResult<usize> {
        self.items.write_der_content(writer)
    }
}