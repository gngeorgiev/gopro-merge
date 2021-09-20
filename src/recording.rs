use std::fmt;

use anyhow::Result;
use thiserror::Error;

use crate::encoding::Encoding;
use crate::identifier::Identifier;
use std::fmt::Formatter;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file name {0}. Valid GoPro file names formats can be found here: https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#")]
    InvalidFileName(String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Group {
    pub encoding: Encoding,
    pub file: Identifier,
    pub extension: String,
}

impl Group {
    pub fn name(&self) -> String {
        format!("{}00{}.{}", self.encoding, self.file, self.extension)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Recording {
    pub group: Group,
    pub chapter: Identifier,
}

impl Recording {
    pub fn try_parse(name: &str) -> Result<Self> {
        // https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#
        let mut iter = name.rsplitn(2, '.').collect::<Vec<_>>().into_iter();
        let (ext, name) = (
            iter.next()
                .ok_or_else(|| Error::InvalidFileName(name.into()))?,
            iter.next()
                .ok_or_else(|| Error::InvalidFileName(name.into()))?,
        );

        if name.len() != 8 {
            return Err(Error::InvalidFileName(name.into()).into());
        }

        let encoding = Encoding::try_from(&name)?;
        let file = Identifier::try_from(&name[4..])?;
        let chapter = Identifier::try_from(&name[2..4])?;

        let group = Group {
            encoding,
            file: file.clone(),
            extension: ext.into(),
        };

        Ok(Recording { group, chapter })
    }
}

impl fmt::Display for Recording {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}.{}",
            self.group.encoding, self.chapter, self.group.file, self.group.extension
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
                    group: Group {
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
                    group: Group {
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
