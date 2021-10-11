use std::convert::TryFrom;
use std::io;
use std::{collections::HashMap, path::Path};

use log::*;
use thiserror::Error;

use crate::identifier::Identifier;
use crate::recording::{self, Fingerprint, Recording};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Recording(#[from] recording::Error),

    #[error(transparent)]
    IO(#[from] io::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Eq, Clone, PartialOrd, Ord)]
pub struct RecordingGroup {
    pub fingerprint: Fingerprint,
    pub chapters: Vec<Identifier>,
}

impl RecordingGroup {
    pub fn name(&self) -> String {
        self.file_name("00")
    }

    pub fn chapter_file_name(&self, chapter: &Identifier) -> String {
        self.file_name(chapter.to_string().as_str())
    }

    fn file_name(&self, chapter: &str) -> String {
        format!(
            "{}{}{}.{}",
            self.fingerprint.encoding, chapter, self.fingerprint.file, self.fingerprint.extension
        )
    }
}

impl PartialEq for RecordingGroup {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
    }
}

pub type RecordingGroups = Vec<RecordingGroup>;

pub fn recordings(path: &Path) -> Result<RecordingGroups> {
    let recordings = collect_recordings(path)?;
    Ok(groups_from_recordings(recordings))
}

fn collect_recordings(path: &Path) -> Result<impl Iterator<Item = Recording>> {
    let files = path
        .read_dir()?
        .into_iter()
        .map(|f| f.map_err(From::from))
        .collect::<Result<Vec<_>>>()?;

    let recordings = files.into_iter().filter_map(|rec| {
        let file_name = rec.file_name();
        let name = file_name.to_str().unwrap();
        debug!("trying to parse file with name {}", name);
        let parsed = Recording::try_from(name).ok();
        debug!("parsed file with name {}: {:?}", name, parsed);
        parsed
    });

    Ok(recordings)
}

fn groups_from_recordings(recordings: impl Iterator<Item = Recording>) -> RecordingGroups {
    recordings
        .fold(HashMap::new(), |mut acc, rec| {
            let group = acc
                .entry(rec.fingerprint.clone())
                .or_insert_with(|| RecordingGroup {
                    fingerprint: rec.fingerprint.clone(),
                    chapters: vec![],
                });
            group.chapters.push(rec.chapter);
            acc
        })
        .drain()
        .map(|(_, mut v)| {
            v.chapters.sort();
            v
        })
        .collect::<RecordingGroups>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    use crate::encoding::Encoding;

    #[derive(Debug)]
    struct Fs(PathBuf, Vec<PathBuf>);

    struct Test<T> {
        files: Vec<&'static str>,
        expected: Vec<T>,

        fs: Option<Fs>,
    }

    impl<T> Test<T> {
        fn new(files: Vec<&'static str>, expected: Vec<T>) -> Self {
            Test {
                files: files,
                expected: expected,
                fs: None,
            }
        }

        fn setup_fs(&mut self, dir_postfix: &str) {
            let tmp = env::temp_dir().join(format!("goprotest_group_{}", dir_postfix));
            fs::create_dir_all(&tmp).unwrap();
            fs::read_dir(&tmp).unwrap().for_each(|f| {
                fs::remove_file(f.unwrap().path()).unwrap();
            });

            let paths = self
                .files
                .iter()
                .map(|f| {
                    let p = tmp.join(f);
                    fs::File::create(&p).unwrap();
                    p
                })
                .collect();

            self.fs = Fs(tmp, paths).into();
        }
    }

    #[test]
    fn test_collect_recordings() {
        let tests = vec![
            Test::new(
                vec!["GH011234.mp4"],
                vec![Recording {
                    fingerprint: Fingerprint {
                        encoding: Encoding::AVC,
                        file: Identifier::try_from("1234").unwrap(),
                        extension: "mp4".into(),
                    },
                    chapter: Identifier::try_from("01").unwrap(),
                }],
            ),
            Test::new(
                vec!["GH011234.mp4", "GH021234.mp4"],
                vec![
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("01").unwrap(),
                    },
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("02").unwrap(),
                    },
                ],
            ),
            Test::new(
                vec![
                    "GH011234.mp4",
                    "GH021234.mp4",
                    "file.png",
                    "random.mp4",
                    "aaaa",
                    "111111",
                ],
                vec![
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("01").unwrap(),
                    },
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("02").unwrap(),
                    },
                ],
            ),
            Test::new(
                vec!["GHAA0001.mp4", "GHAA0002.mp4"],
                vec![
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("0001").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("AA").unwrap(),
                    },
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("0002").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("AA").unwrap(),
                    },
                ],
            ),
            Test::new(
                vec!["GH011234.mp4", "GX011234.mp4"],
                vec![
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("01").unwrap(),
                    },
                    Recording {
                        fingerprint: Fingerprint {
                            encoding: Encoding::HEVC,
                            file: Identifier::try_from("1234").unwrap(),
                            extension: "mp4".into(),
                        },
                        chapter: Identifier::try_from("01").unwrap(),
                    },
                ],
            ),
        ];

        tests.into_iter().for_each(|mut test| {
            test.setup_fs("test_collect_recordings");

            let fs = test.fs.as_ref().unwrap();
            let mut recordings = collect_recordings(&fs.0).unwrap().collect::<Vec<_>>();
            recordings.sort();

            test.expected.sort();

            assert_eq!(
                test.expected, recordings,
                "collected recordings didn't match"
            );
        });
    }

    #[test]
    fn test_recordings() {
        let tests = vec![
            Test::new(
                vec!["GH011234.mp4", "GH021234.mp4"],
                vec![RecordingGroup {
                    fingerprint: Fingerprint {
                        encoding: Encoding::AVC,
                        extension: "mp4".into(),
                        file: "1234".try_into().unwrap(),
                    },
                    chapters: vec![
                        Identifier::try_from("01").unwrap(),
                        Identifier::try_from("02").unwrap(),
                    ],
                }],
            ),
            Test::new(
                vec![
                    "GH011234.mp4",
                    "GH021234.mp4",
                    "GX011235.flv",
                    "GH001111.mp4",
                ],
                vec![
                    RecordingGroup {
                        fingerprint: Fingerprint {
                            encoding: Encoding::AVC,
                            extension: "mp4".into(),
                            file: "1234".try_into().unwrap(),
                        },
                        chapters: vec![
                            Identifier::try_from("01").unwrap(),
                            Identifier::try_from("02").unwrap(),
                        ],
                    },
                    RecordingGroup {
                        fingerprint: Fingerprint {
                            encoding: Encoding::HEVC,
                            extension: "flv".into(),
                            file: "1235".try_into().unwrap(),
                        },
                        chapters: vec![Identifier::try_from("01").unwrap()],
                    },
                ],
            ),
        ];

        tests.into_iter().for_each(|mut t| {
            t.setup_fs("test_recordings");

            let fs = t.fs.as_ref().unwrap();
            let mut result = recordings(&fs.0).unwrap();
            result.sort();
            assert_eq!(t.expected, result);
        });
    }
}
