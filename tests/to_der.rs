use asn1_rs::*;
use hex_literal::hex;
// use nom::HexDisplay;
use std::convert::{TryFrom, TryInto};

#[test]
fn to_der_length() {
    // indefinite length
    let length = Length::Indefinite;
    let v = length.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x80]);
    // definite, short form
    let length = Length::Definite(3);
    let v = length.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x03]);
    // definite, long form
    let length = Length::Definite(250);
    let v = length.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0xfa, 0x01]);
}

#[test]
fn to_der_tag() {
    // short tag, UNIVERSAL
    let v = (Class::Universal, false, Tag(0x1a))
        .to_der_vec()
        .expect("serialization failed");
    assert_eq!(&v, &[0x1a]);
    // short tag, APPLICATION
    let v = (Class::Application, false, Tag(0x1a))
        .to_der_vec()
        .expect("serialization failed");
    assert_eq!(&v, &[0x1a | (0b01 << 6)]);
    // short tag, constructed
    let v = (Class::Universal, true, Tag(0x10))
        .to_der_vec()
        .expect("serialization failed");
    assert_eq!(&v, &[0x30]);
    // long tag, UNIVERSAL
    let v = (Class::Universal, false, Tag(0x1a1a))
        .to_der_vec()
        .expect("serialization failed");
    assert_eq!(&v, &[0b1_1111, 0x9a, 0x34]);
}

#[test]
fn to_der_header() {
    // simple header
    let header = Header::new_simple(Tag::Integer);
    let v = header.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x2, 0x0]);
    // indefinite length
    let header = Header::new(Class::Universal, false, Tag::Integer, Length::Indefinite);
    let v = header.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x2, 0x80]);
}

#[test]
fn to_der_any() {
    let header = Header::new_simple(Tag::Integer);
    let any = Any::new(header, &hex!("02"));
    let v = any.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x02, 0x01, 0x02]);
}

#[test]
fn to_der_any_raw() {
    let header = Header::new(Class::Universal, false, Tag::Integer, Length::Definite(3));
    let any = Any::new(header, &hex!("02"));
    // to_vec should compute the length
    let v = any.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x02, 0x01, 0x02]);
    // to_vec_raw will use the header as provided
    let v = any.to_der_vec_raw().expect("serialization failed");
    assert_eq!(&v, &[0x02, 0x03, 0x02]);
}

#[test]
fn to_der_bitstring() {
    let bitstring = BitString::new(6, &hex!("6e 5d c0"));
    let v = bitstring.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("03 04 06 6e 5d c0"));
    let (_, result) = BitString::from_der(&v).expect("parsing failed");
    assert!(bitstring.eq(&result));
}

#[test]
fn to_der_bool() {
    let v = Boolean::new(0xff)
        .to_der_vec()
        .expect("serialization failed");
    assert_eq!(&v, &[0x01, 0x01, 0xff]);
    //
    let v = false.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x01, 0x01, 0x00]);
    //
    let v = true.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x01, 0x01, 0xff]);
    // raw value (not 0 of 0xff)
    let v = Boolean::new(0x8a)
        .to_der_vec_raw()
        .expect("serialization failed");
    assert_eq!(&v, &[0x01, 0x01, 0x8a]);
}

#[test]
fn to_der_generalizedtime() {
    // date without millisecond
    let dt = ASN1DateTime::new(1999, 12, 31, 23, 59, 59, None, ASN1TimeZone::Z);
    let time = GeneralizedTime::new(dt);
    let v = time.to_der_vec().expect("serialization failed");
    assert_eq!(&v[..2], &hex!("18 0f"));
    assert_eq!(&v[2..], b"19991231235959Z");
    let (_, time2) = GeneralizedTime::from_der(&v).expect("decoding serialized object failed");
    assert!(time.eq(&time2));
    //
    // date with millisecond
    let dt = ASN1DateTime::new(1999, 12, 31, 23, 59, 59, Some(123), ASN1TimeZone::Z);
    let time = GeneralizedTime::new(dt);
    let v = time.to_der_vec().expect("serialization failed");
    assert_eq!(&v[..2], &hex!("18 13"));
    assert_eq!(&v[2..], b"19991231235959.123Z");
    let (_, time2) = GeneralizedTime::from_der(&v).expect("decoding serialized object failed");
    assert!(time.eq(&time2));
}

fn encode_decode_assert_int<T>(t: T, expected: &[u8])
where
    T: ToDer + std::fmt::Debug + Eq,
    for<'a> T: TryFrom<Integer<'a>, Error = Error>,
{
    let v = t.to_der_vec().expect("serialization failed");
    assert_eq!(&v, expected);
    let (_, obj) = Integer::from_der(&v).expect("decoding serialized object failed");
    let t2: T = obj.try_into().unwrap();
    assert_eq!(t, t2);
}

#[test]
fn to_der_integer() {
    let int = Integer::new(&hex!("02"));
    let v = int.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x02, 0x01, 0x02]);
    // from_u32
    let int = Integer::from_u32(2);
    let v = int.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &[0x02, 0x01, 0x02]);
    // impl ToDer for primitive types
    encode_decode_assert_int(2u32, &[0x02, 0x01, 0x02]);
    // signed i32 (> 0)
    encode_decode_assert_int(4, &[0x02, 0x01, 0x04]);
    // signed i32 (< 0)
    encode_decode_assert_int(-4, &[0x02, 0x05, 0x00, 0xff, 0xff, 0xff, 0xfc]);
}

#[test]
fn to_der_octetstring() {
    let bytes: &[u8] = &hex!("01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f");
    let s = OctetString::from(bytes);
    let v = s.to_der_vec().expect("serialization failed");
    assert_eq!(&v[..2], &hex!("04 0f"));
    assert_eq!(&v[2..], bytes);
    let (_, s2) = OctetString::from_der(&v).expect("decoding serialized object failed");
    assert!(s.eq(&s2));
}

#[test]
fn to_der_real_binary() {
    // base = 2, value = 4
    let r = Real::binary(2.0, 2, 1);
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 03 80 02 01"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!((r.f64() - result.f64()).abs() < f64::EPSILON);
    //
    // base = 2, value = 0.5
    let r = Real::binary(0.5, 2, 0);
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 03 80 ff 01"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!((r.f64() - result.f64()).abs() < f64::EPSILON);
    //
    // base = 2, value = 3.25, but change encoding base (8)
    let r = Real::binary(3.25, 2, 0).with_enc_base(8);
    let v = r.to_der_vec().expect("serialization failed");
    // note: this encoding has a scale factor (not DER compliant)
    assert_eq!(&v, &hex!("09 03 94 ff 0d"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!((r.f64() - result.f64()).abs() < f64::EPSILON);
    //
    // base = 2, value = 0.00390625, but change encoding base (16)
    let r = Real::binary(0.00390625, 2, 0).with_enc_base(16);
    let v = r.to_der_vec().expect("serialization failed");
    // note: this encoding has a scale factor (not DER compliant)
    assert_eq!(&v, &hex!("09 03 a0 fe 01"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!((r.f64() - result.f64()).abs() < f64::EPSILON);
    //
    // 2 octets for exponent, negative exponent and abs(exponent) is all 1's and fills the whole octet(s)
    let r = Real::binary(3.0, 2, -1020);
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 04 81 fc 04 03"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!((r.f64() - result.f64()).abs() < f64::EPSILON);
    //
    // 3 octets for exponent, and
    // check that first 9 bits for exponent are not all 1's
    let r = Real::binary(1.0, 2, 262140);
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 05 82 03 ff fc 01"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    // XXX value cannot be represented as f64 (inf)
    assert!(result.f64().is_infinite());
    //
    // >3 octets for exponent, and
    // mantissa < 0
    let r = Real::binary(-1.0, 2, 76354972);
    let v = r.to_der_vec().expect("serialization failed");
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert_eq!(&v, &hex!("09 07 c3 04 04 8d 15 9c 01"));
    // XXX value cannot be represented as f64 (-inf)
    assert!(result.f64().is_infinite());
}

#[test]
fn to_der_real_special() {
    // ZERO
    let r = Real::Zero;
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 00"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!(r.eq(&result));
    // INFINITY
    let r = Real::Infinity;
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 01 40"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!(r.eq(&result));
    // MINUS INFINITY
    let r = Real::NegInfinity;
    let v = r.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("09 01 41"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!(r.eq(&result));
}

#[test]
fn to_der_real_string() {
    //  non-zero value, base 10
    let r = Real::new(1.2345);
    let v = r.to_der_vec().expect("serialization failed");
    // assert_eq!(&v, &hex!("09 00"));
    let (_, result) = Real::from_der(&v).expect("parsing failed");
    assert!(r.eq(&result));
}

#[test]
fn to_der_sequence() {
    let it = [2, 3, 4].iter();
    let seq = Sequence::from_iter_to_der(it).unwrap();
    let v = seq.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("30 09 02 01 02 02 01 03 02 01 04"));
    let (_, seq2) = Sequence::from_der(&v).expect("decoding serialized object failed");
    assert_eq!(seq, seq2);
    // Vec<T>::ToDer
    let v = vec![2, 3, 4].to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("30 09 02 01 02 02 01 03 02 01 04"));
}

#[test]
fn to_der_tagged_explicit() {
    let tagged = TaggedValue::new_explicit(Class::ContextSpecific, 1, 2u32);
    let v = tagged.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("a1 03 02 01 02"));
    let (_, t2) =
        TaggedValue::<Explicit, u32>::from_der(&v).expect("decoding serialized object failed");
    assert!(tagged.eq(&t2));
}

#[test]
fn to_der_tagged_implicit() {
    let tagged = TaggedValue::new_implicit(Class::ContextSpecific, false, 1, 2u32);
    let v = tagged.to_der_vec().expect("serialization failed");
    assert_eq!(&v, &hex!("81 01 02"));
    let (_, t2) =
        TaggedValue::<Implicit, u32>::from_der(&v).expect("decoding serialized object failed");
    assert!(tagged.eq(&t2));
}

#[test]
fn to_der_utctime() {
    let dt = ASN1DateTime::new(99, 12, 31, 23, 59, 59, None, ASN1TimeZone::Z);
    let time = UtcTime::new(dt);
    let v = time.to_der_vec().expect("serialization failed");
    assert_eq!(&v[..2], &hex!("17 0d"));
    assert_eq!(&v[2..], b"991231235959Z");
    let (_, time2) = UtcTime::from_der(&v).expect("decoding serialized object failed");
    assert!(time.eq(&time2));
}

#[test]
fn to_der_utf8string() {
    let s = Utf8String::from("abcdef");
    let v = s.to_der_vec().expect("serialization failed");
    assert_eq!(&v[..2], &hex!("0c 06"));
    assert_eq!(&v[2..], b"abcdef");
    let (_, s2) = Utf8String::from_der(&v).expect("decoding serialized object failed");
    assert!(s.eq(&s2));
}
