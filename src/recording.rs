use std::convert::TryFrom;
use std::fmt;

use thiserror::Error;

use crate::encoding::Encoding;
use crate::identifier::Identifier;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file name {0}. Valid GoPro file names formats can be found here: https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#")]
    InvalidFileName(String),

    #[error(":?")]
    Inner(#[from] anyhow::Error),
}

pub type RecordingGroups = Vec<RecordingGroup>;

#[derive(Debug, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct RecordingFingerprint {
    pub encoding: Encoding,
    pub file: Identifier,
    pub extension: String,
}

impl PartialEq for RecordingFingerprint {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding
            && self.file == other.file
            && self.extension == other.extension
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Clone)]
pub struct RecordingGroup {
    pub fingerprint: RecordingFingerprint,
    pub chapters: Vec<Identifier>,
}

impl RecordingGroup {
    pub fn name(&self) -> String {
        format!(
            "{}00{}.{}",
            self.fingerprint.encoding, self.fingerprint.file, self.fingerprint.extension
        )
    }
}

pub struct Recording {
    pub fingerprint: RecordingFingerprint,
    pub chapter: Identifier,
}

impl<'a> TryFrom<&'a str> for Recording {
    type Error = Error;

    fn try_from(name: &'a str) -> std::result::Result<Self, Self::Error> {
        // https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#
        let mut iter = name.rsplitn(2, '.').collect::<Vec<_>>().into_iter();

        let invalid_file_name_error = |name: &'a str| || Error::InvalidFileName(name.into());
        let name = iter.next().ok_or_else(invalid_file_name_error(name))?;
        if name.len() != 8 {
            return Err(Error::InvalidFileName(name.into()).into());
        }

        let ext = iter.next().ok_or_else(invalid_file_name_error(name))?;

        let encoding = Encoding::try_from(&name)?;
        let file = Identifier::try_from(&name[4..])?;
        let chapter = Identifier::try_from(&name[2..4])?;

        let recording = Recording {
            fingerprint: RecordingFingerprint {
                encoding,
                file: file.clone(),
                extension: ext.into(),
            },
            chapter,
        };

        Ok(recording)
    }
}

impl fmt::Display for Recording {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}.{}",
            self.fingerprint.encoding,
            self.chapter,
            self.fingerprint.file,
            self.fingerprint.extension
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoding_try_parse() {
        let ok_input = vec![
            (
                "GH010034.mp4",
                Recording {
                    group: RecordingGroup {
                        encoding: Encoding::AVC,
                        file: Identifier {
                            representation: "0034".into(),
                            value: 34,
                        },
                        extension: "mp4".into(),
                    },
                    chapter: Identifier {
                        representation: "01".into(),
                        value: 1,
                    },
                },
            ),
            (
                "GX111134.flv",
                Recording {
                    group: RecordingGroup {
                        encoding: Encoding::HEVC,
                        file: Identifier {
                            representation: "1134".into(),
                            value: 1134,
                        },
                        extension: "flv".into(),
                    },
                    chapter: Identifier {
                        representation: "11".into(),
                        value: 11,
                    },
                },
            ),
        ];
        ok_input.into_iter().for_each(|(input, expected)| {
            let parsed = Recording::try_parse(input).unwrap();
            assert_eq!(expected, parsed);
        });

        let not_ok_input = vec!["invalid_dots_amount..", "name_longer_than_8_chars_.mp4"];
        not_ok_input.into_iter().for_each(|input| {
            assert!(Recording::try_parse(input).is_err());
        });
    }
}
