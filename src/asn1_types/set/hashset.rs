#![cfg(feature = "std")]
use crate::*;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::hash::Hash;

impl<T> Tagged for HashSet<T> {
    const TAG: Tag = Tag::Set;
}

impl<'a, T> TryFrom<Any<'a>> for HashSet<T>
where
    T: FromBer<'a>,
    T: Hash + Eq,
{
    type Error = Error;

    fn try_from(any: Any<'a>) -> Result<Self> {
        any.tag().assert_eq(Self::TAG)?;
        any.header.assert_constructed()?;
        let items = SetIterator::<T, BerParser>::new(any.data).collect::<Result<HashSet<T>>>()?;
        Ok(items)
    }
}

impl<T> CheckDerConstraints for HashSet<T>
where
    T: CheckDerConstraints,
{
    fn check_constraints(any: &Any) -> Result<()> {
        any.tag().assert_eq(Self::TAG)?;
        any.header.assert_constructed()?;
        for item in SetIterator::<Any, DerParser>::new(any.data) {
            let item = item?;
            T::check_constraints(&item)?;
        }
        Ok(())
    }
}

/// manual impl of FromDer, so we do not need to require TryFrom<Any> + CheckDerConstraints
impl<'a, T, E> FromDer<'a, E> for HashSet<T>
where
    T: FromDer<'a, E>,
    T: Hash + Eq,
    E: From<Error>,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<'a, Self, E> {
        let (rem, any) = Any::from_der(bytes).map_err(Err::convert)?;
        any.tag()
            .assert_eq(Self::TAG)
            .map_err(|e| nom::Err::Error(e.into()))?;
        any.header
            .assert_constructed()
            .map_err(|e| nom::Err::Error(e.into()))?;
        let items = SetIterator::<T, DerParser, E>::new(any.data)
            .collect::<Result<HashSet<T>, E>>()
            .map_err(nom::Err::Error)?;
        Ok((rem, items))
    }
}

impl<T> ToDer for HashSet<T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        let mut len = 0;
        for t in self.iter() {
            len += t.to_der_len()?;
        }
        let header = Header::new(Class::Universal, true, Self::TAG, Length::Definite(len));
        Ok(header.to_der_len()? + len)
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut len = 0;
        for t in self.iter() {
            len += t.to_der_len().map_err(|_| SerializeError::InvalidLength)?;
        }
        let header = Header::new(Class::Universal, true, Self::TAG, Length::Definite(len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut sz = 0;
        for t in self.iter() {
            sz += t.write_der(writer)?;
        }
        Ok(sz)
    }
}
