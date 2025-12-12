use itertools::Itertools;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use thiserror::Error;

pub const DIGEST_BYTES: usize = 16;
pub const DIGEST_HEX_DIGITS: usize = 2 * DIGEST_BYTES;

pub type DigestBuffer = [u8; DIGEST_BYTES];

#[derive(Error, Debug, PartialEq)]
pub enum DigestError {
    #[error("Invalid char found; support only [0-9a-fA-f]. Found {0}")]
    InvalidChar(char),

    #[error("Invalid length")]
    InvalidLength,

    #[error("Invalid alignment")]
    InvalidAlignment,

    #[error("Invalid value")]
    InvalidValue,
}

macro_rules! impl_digest_output {
    ($output : ident) => {
        paste::paste! {
            #[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
            pub struct $output(DigestBuffer);

            impl $output {
                pub fn to_binary(&self) -> &DigestBuffer {
                    &self.0
                }

                pub fn boxed_binary(&self) -> Box<[u8]> {
                    Box::new(self.0.clone())
                }
            }

            impl Display for $output {
                fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                    for &x in &self.0 {
                        f.write_fmt(format_args!("{:02x}", x))?;
                    }
                    Ok(())
                }
            }

            impl From<DigestBuffer> for $output {
                fn from(value: DigestBuffer) -> Self {
                    $output(value)
                }
            }

            impl TryFrom<&[u8]> for $output {
                type Error = DigestError;

                fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
                    if value.len() != DIGEST_BYTES {
                        return Err(DigestError::InvalidLength);
                    }

                    let mut digest = [0u8; DIGEST_BYTES];
                    digest.copy_from_slice(value);
                    Ok(digest.into())
                }
            }

            impl TryFrom<Vec<u8>> for $output {
                type Error = DigestError;

                fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                    value.as_slice().try_into()
                }
            }

            impl TryFrom<String> for $output {
                type Error = DigestError;

                fn try_from(value: String) -> Result<Self, Self::Error> {
                    value.as_str().try_into()
                }
            }

            impl TryFrom<&str> for $output {
                type Error = DigestError;

                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    if value.len() != DIGEST_HEX_DIGITS {
                        return Err(DigestError::InvalidLength);
                    }

                    let mut digest = [0u8; DIGEST_BYTES];

                    for ((a, b), target) in value.chars().tuples().zip(digest.iter_mut()) {
                        let Some(num_a) = a.to_digit(16) else {
                            return Err(DigestError::InvalidChar(a));
                        };
                        let Some(num_b) = b.to_digit(16) else {
                            return Err(DigestError::InvalidChar(b));
                        };

                        *target = ((num_a as u8) << 4) | (num_b as u8);
                    }

                    Ok(digest.into())
                }
            }

            #[derive(Default, Debug)]
            pub struct [<$output Builder >] {
                buffer: DigestBuffer,
                bits: usize,
            }

            impl [<$output Builder >] {
                pub fn push_u4(&mut self, value: u8) -> Result<&mut Self, DigestError> {
                    if self.bits % 4 != 0 {
                        return Err(DigestError::InvalidAlignment);
                    }

                    if self.bits + 4 > 8 * DIGEST_BYTES {
                        return Err(DigestError::InvalidLength);
                    }

                    if value > 0xf {
                        return Err(DigestError::InvalidValue);
                    }

                    self.buffer[self.bits / 8] <<= 4;
                    self.buffer[self.bits / 8] |= value;
                    self.bits += 4;
                    Ok(self)
                }

                pub fn push_u8(&mut self, value: u8) -> Result<&mut Self, DigestError> {
                    if self.bits % 8 != 0 {
                        return Err(DigestError::InvalidAlignment);
                    }
                    if self.bits + 8 > 8 * DIGEST_BYTES {
                        return Err(DigestError::InvalidLength);
                    }

                    self.buffer[self.bits / 8] = value;
                    self.bits += 8;
                    Ok(self)
                }

                pub fn push_u32(&mut self, value: u32) -> Result<&mut Self, DigestError> {
                    if self.bits % 8 != 0 {
                        return Err(DigestError::InvalidAlignment);
                    }

                    if self.bits + 32 > 8 * DIGEST_BYTES {
                        return Err(DigestError::InvalidLength);
                    }

                    self.push_u8((value >> 24) as u8)?;
                    self.push_u8((value >> 16) as u8)?;
                    self.push_u8((value >> 8) as u8)?;
                    self.push_u8((value >> 0) as u8)
                }

                pub fn push_slice(&mut self, value: &[u8]) -> Result<&mut Self, DigestError> {
                    if self.bits + value.len() * 8 > DIGEST_BYTES * 8 {
                        return Err(DigestError::InvalidLength);
                    }

                    for &x in value {
                        self.push_u8(x)?;
                    }
                    Ok(self)
                }

                pub fn build(&self) -> Result<$output, DigestError> {
                    if self.bits != DIGEST_BYTES * 8 {
                        return Err(DigestError::InvalidLength);
                    }
                    Ok(self.buffer.into())
                }
            }

            impl Serialize for $output {
                fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    ser.serialize_str(format!("{}", self).as_str())
                }
            }

            impl<'de> Deserialize<'de> for $output {
                fn deserialize<D>(de: D) -> Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let s = String::deserialize(de)?;

                    if s.len() != DIGEST_HEX_DIGITS {
                        return Err(D::Error::invalid_length(
                            s.len(),
                            &"hex digest with exactly 20 characters",
                        ));
                    }

                    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(D::Error::invalid_value(
                            Unexpected::Str(&s),
                            &"a valid hex string",
                        ));
                    }

                    if let Ok(digest) = s.as_str().try_into() {
                        Ok(digest)
                    } else {
                        Err(D::Error::invalid_value(
                            Unexpected::Str(&s),
                            &"Unexpected string",
                        ))
                    }
                }
            }

            #[cfg(test)]
            mod [<test_ $output:snake>] {
                use super::*;
                type TestDigest = $output;
                type TestBuilder = [<$output Builder >];

                #[test]
                fn digest_string_serde() {
                    let string = (0..DIGEST_HEX_DIGITS)
                        .map(|i| format!("{:x}", i % 16))
                        .collect::<String>();
                    assert_eq!(string.len(), DIGEST_HEX_DIGITS);

                    let digest_string = TestDigest::try_from(string).unwrap();

                    let serialized = serde_json::to_string(&digest_string).unwrap();
                    let deserialized: TestDigest = serde_json::from_str(&serialized).unwrap();

                    assert_eq!(digest_string, deserialized);

                    assert!(serde_json::from_str::<TestDigest>("\"xasdf\"").is_err());
                }

                #[test]
                fn digest_deserialize_wrong_length() {
                    assert!(serde_json::from_str::<TestDigest>("\"0123\"").is_err());
                }

                #[test]
                fn digest_deserialize_invalid_chars() {
                    assert!(serde_json::from_str::<TestDigest>("\"01234567890123456789012345678931\"").is_ok());
                    assert!(
                        serde_json::from_str::<TestDigest>("\"0123456789012345678901234567893z\"").is_err()
                    );
                }

                #[test]
                fn digest_string() {
                    let mut string = (0..DIGEST_HEX_DIGITS)
                        .map(|i| format!("{:x}", i % 16))
                        .collect::<String>();
                    assert_eq!(string.len(), DIGEST_HEX_DIGITS);

                    assert_eq!(
                        &TestDigest::try_from(string.as_str()).unwrap().to_string(),
                        &string
                    );

                    string.pop();
                    assert!(TestDigest::try_from(string.as_str()).is_err());

                    string.push('X'); // this is not a hex digit ;)
                    assert!(TestDigest::try_from(string.as_str()).is_err());

                    string.pop();
                    string.push('F');
                    let mut lower = string.clone();
                    lower.make_ascii_lowercase();

                    assert_eq!(
                        &TestDigest::try_from(string.as_str()).unwrap().to_string(),
                        &lower,
                    );
                }

                #[test]
                fn digest_roundtrip() {
                    let mut original_buffer: DigestBuffer = [0u8; DIGEST_BYTES];
                    let mut i = 1u64;

                    for _ in 0..1000 {
                        for b in original_buffer.iter_mut() {
                            *b = i as u8;
                            i += (3 * i + 27) & 0xFF;
                        }

                        let sd: TestDigest = original_buffer.into();

                        {
                            let reconstructed_buffer = sd.to_binary();
                            assert_eq!(&original_buffer, reconstructed_buffer);
                        }

                        {
                            let reconstructed_buffer = sd.boxed_binary();
                            assert_eq!(original_buffer.as_slice(), &reconstructed_buffer[..]);
                        }
                    }
                }

                #[test]
                fn digest_from_binary_wrong_length() {
                    let buffer = [0u8; DIGEST_HEX_DIGITS / 2 - 1];
                    assert!(TestDigest::try_from(buffer.as_slice()).is_err());

                    let buffer = [0u8; DIGEST_HEX_DIGITS / 2 + 1];
                    assert!(TestDigest::try_from(buffer.as_slice()).is_err());
                }

                const BUFFER: DigestBuffer = [
                    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
                    0x1F,
                ];

                #[test]
                fn builder_push_slice_full() {
                    let digest = TestBuilder::default()
                        .push_slice(&BUFFER)
                        .unwrap()
                        .build()
                        .unwrap();
                    assert_eq!(digest.to_binary(), &BUFFER);
                }

                #[test]
                fn builder_push_slice_size_error() {
                    assert_eq!(
                        TestBuilder::default()
                            .push_slice(&BUFFER[1..])
                            .unwrap()
                            .push_slice(&BUFFER[1..])
                            .err(),
                        Some(DigestError::InvalidLength)
                    );
                }

                #[test]
                fn builder_push_u4() {
                    let digest = TestBuilder::default()
                        .push_u4(5)
                        .unwrap()
                        .push_u4(8)
                        .unwrap()
                        .push_slice(&BUFFER[1..])
                        .unwrap()
                        .build()
                        .unwrap();

                    assert_eq!(digest.to_binary()[..4], [0x58, 0x11, 0x12, 0x13])
                }

                #[test]
                fn builder_push_u4_alignment() {
                    assert_eq!(
                        TestBuilder::default().push_u4(5).unwrap().push_u8(8).err(),
                        Some(DigestError::InvalidAlignment)
                    );
                }

                #[test]
                fn builder_push_u8() {
                    let digest = TestBuilder::default()
                        .push_u8(5)
                        .unwrap()
                        .push_u8(8)
                        .unwrap()
                        .push_slice(&BUFFER[..14])
                        .unwrap()
                        .build()
                        .unwrap();

                    assert_eq!(digest.to_binary()[..4], [0x5, 0x8, 0x10, 0x11])
                }

                #[test]
                fn builder_push_u32() {
                    let digest = TestBuilder::default()
                        .push_u32(0xFFEEDDCC)
                        .unwrap()
                        .push_slice(&BUFFER[..12])
                        .unwrap()
                        .build()
                        .unwrap();

                    assert_eq!(digest.to_binary()[..5], [0xFF, 0xEE, 0xDD, 0xCC, 0x10])
                }
            }
        }
    };
}

impl_digest_output!(InstanceDigest);
impl_digest_output!(SolutionDigest);
impl_digest_output!(FileDigest);
