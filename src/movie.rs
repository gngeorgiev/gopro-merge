use std::convert::TryFrom;

use crate::encoding::{self, Encoding};
use crate::identifier::{self, Identifier};

use derive_more::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid file name {0}. Valid GoPro file names formats can be found here: https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#")]
    InvalidFileName(String),

    #[error("Invalid movie file number 0. Non loop file numbers should be numeric in the range of 01-99")]
    InvalidMovieFileNumberZero,

    #[error("Invalid movie chapter number 0. Non loop file numbers should be numeric in the range of 0001-9999")]
    InvalidMovieChapterNumberZero,

    #[error(transparent)]
    Identifier(#[from] identifier::Error),

    #[error(transparent)]
    Encoding(#[from] encoding::Error),
}

#[derive(Debug, Eq, PartialOrd, PartialEq, Ord, Hash, Clone)]
pub struct Fingerprint {
    pub encoding: Encoding,
    pub file: Identifier,
    pub extension: String,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Display)]
#[display(
    fmt = "{}{}{}.{}",
    "fingerprint.encoding",
    chapter,
    "fingerprint.file",
    "fingerprint.extension"
)]
pub struct Movie {
    pub fingerprint: Fingerprint,
    pub chapter: Identifier,
}

impl<'a> TryFrom<&'a str> for Movie {
    type Error = Error;

    fn try_from(name: &'a str) -> std::result::Result<Self, Self::Error> {
        // https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#
        let mut iter = name.rsplitn(2, '.').collect::<Vec<_>>().into_iter();

        let invalid_file_name_error = |name: &'a str| || Error::InvalidFileName(name.into());
        let ext = iter.next().ok_or_else(invalid_file_name_error(name))?;
        let name = iter.next().ok_or_else(invalid_file_name_error(name))?;
        if name.len() != 8 {
            return Err(Error::InvalidFileName(name.into()));
        }

        let encoding = Encoding::try_from(name)?;
        let file = Identifier::try_from(&name[4..])?;
        if let Ok(0) = file.numeric() {
            return Err(Error::InvalidMovieFileNumberZero);
        }

        let chapter = Identifier::try_from(&name[2..4])?;
        if let Ok(0) = chapter.numeric() {
            return Err(Error::InvalidMovieChapterNumberZero);
        }

        let movie = Movie {
            fingerprint: Fingerprint {
                encoding,
                file,
                extension: ext.into(),
            },
            chapter,
        };

        Ok(movie)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoding_try_from_format() {
        let ok_input = vec![
            (
                "GH010034.mp4",
                Movie {
                    fingerprint: Fingerprint {
                        encoding: Encoding::Avc,
                        file: Identifier::try_from("0034").unwrap(),
                        extension: "mp4".into(),
                    },
                    chapter: Identifier::try_from("01").unwrap(),
                },
            ),
            (
                "GX111134.flv",
                Movie {
                    fingerprint: Fingerprint {
                        encoding: Encoding::Hevc,
                        file: Identifier::try_from("1134").unwrap(),
                        extension: "flv".into(),
                    },
                    chapter: Identifier::try_from("11").unwrap(),
                },
            ),
        ];
        ok_input.into_iter().for_each(|(input, expected)| {
            let parsed = Movie::try_from(input).unwrap();
            assert_eq!(input, &parsed.to_string());
            assert_eq!(expected, parsed);
        });
    }

    #[test]
    fn movie_try_from_err() {
        let not_ok_input = vec![
            "invalid_dots_amount..",
            "name_longer_than_8_chars_.mp4",
            "picture.png",
            "0",
            "",
            "1111111111111111",
            "GY111134.flv",
            "GPAA0000.mp4",
            "GX000000.mp4",
            "GH010000.mp4",
            "GH000001.mp4",
        ];
        not_ok_input.into_iter().for_each(|input| {
            assert!(Movie::try_from(input).is_err(), "{} isn't error", input,);
        });
    }
}
