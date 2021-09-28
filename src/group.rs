use anyhow::Result;
use std::convert::TryFrom;
use std::{collections::HashMap, path::Path};

use crate::identifier::Identifier;
use crate::recording::{Fingerprint, Recording};

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

    let recordings = files
        .into_iter()
        .filter_map(|rec| Recording::try_from(rec.file_name().to_str().unwrap()).ok());

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
    use crate::encoding::Encoding;

    use super::*;

    // fn setup_fs(files: Vec<&'static str>) -> VfsPath {
    //     let root: VfsPath = vfs::MemoryFS::new().into();
    //     files.into_iter().for_each(|f| {
    //         root.join(f).unwrap().create_file().unwrap();
    //     });

    //     root
    // }

    // #[test]
    // fn test_collect_recordings() {
    //     struct Test {
    //         fs: VfsPath,
    //         expected: Vec<Recording>,
    //     }

    //     let tests = vec![
    //         Test {
    //             fs: setup_fs(vec!["GH011234.mp4"]),
    //             expected: vec![Recording {
    //                 fingerprint: Fingerprint {
    //                     encoding: Encoding::AVC,
    //                     file: Identifier::try_from("1234").unwrap(),
    //                     extension: "mp4".into(),
    //                 },
    //                 chapter: Identifier::try_from("01").unwrap(),
    //             }],
    //         },
    //         Test {
    //             fs: setup_fs(vec!["GH011234.mp4", "GH021234.mp4"]),
    //             expected: vec![
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("01").unwrap(),
    //                 },
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("02").unwrap(),
    //                 },
    //             ],
    //         },
    //         Test {
    //             fs: setup_fs(vec![
    //                 "GH011234.mp4",
    //                 "GH021234.mp4",
    //                 "file.png",
    //                 "random.mp4",
    //                 "aaaa",
    //                 "111111",
    //             ]),
    //             expected: vec![
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("01").unwrap(),
    //                 },
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("02").unwrap(),
    //                 },
    //             ],
    //         },
    //         Test {
    //             fs: setup_fs(vec!["GHAA0001.mp4", "GHAA0002.mp4"]),
    //             expected: vec![
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("0001").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("AA").unwrap(),
    //                 },
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("0002").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("AA").unwrap(),
    //                 },
    //             ],
    //         },
    //         Test {
    //             fs: setup_fs(vec!["GH011234.mp4", "GX011234.mp4"]),
    //             expected: vec![
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("01").unwrap(),
    //                 },
    //                 Recording {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::HEVC,
    //                         file: Identifier::try_from("1234").unwrap(),
    //                         extension: "mp4".into(),
    //                     },
    //                     chapter: Identifier::try_from("01").unwrap(),
    //                 },
    //             ],
    //         },
    //     ];

    //     tests.into_iter().for_each(|mut test| {
    //         let mut recordings = collect_recordings(&test.fs).unwrap().collect::<Vec<_>>();
    //         recordings.sort();

    //         test.expected.sort();

    //         assert_eq!(
    //             test.expected, recordings,
    //             "collected recordings didn't match"
    //         );
    //     });
    // }

    // #[test]
    // fn test_recordings() {
    //     struct Test {
    //         fs: VfsPath,
    //         expected: RecordingGroups,
    //     }

    //     let tests = vec![
    //         Test {
    //             fs: setup_fs(vec!["GH011234.mp4", "GH021234.mp4"]),
    //             expected: vec![RecordingGroup {
    //                 fingerprint: Fingerprint {
    //                     encoding: Encoding::AVC,
    //                     extension: "mp4".into(),
    //                     file: "1234".try_into().unwrap(),
    //                 },
    //                 chapters: vec![
    //                     Identifier::try_from("01").unwrap(),
    //                     Identifier::try_from("02").unwrap(),
    //                 ],
    //             }],
    //         },
    //         Test {
    //             fs: setup_fs(vec![
    //                 "GH011234.mp4",
    //                 "GH021234.mp4",
    //                 "GX011235.flv",
    //                 "GH001111.mp4",
    //             ]),
    //             expected: vec![
    //                 RecordingGroup {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::AVC,
    //                         extension: "mp4".into(),
    //                         file: "1234".try_into().unwrap(),
    //                     },
    //                     chapters: vec![
    //                         Identifier::try_from("01").unwrap(),
    //                         Identifier::try_from("02").unwrap(),
    //                     ],
    //                 },
    //                 RecordingGroup {
    //                     fingerprint: Fingerprint {
    //                         encoding: Encoding::HEVC,
    //                         extension: "flv".into(),
    //                         file: "1235".try_into().unwrap(),
    //                     },
    //                     chapters: vec![Identifier::try_from("01").unwrap()],
    //                 },
    //             ],
    //         },
    //     ];

    //     tests.into_iter().for_each(|t| {
    //         let mut result = recordings(&t.fs).unwrap();
    //         result.sort();
    //         assert_eq!(t.expected, result);
    //     });
    // }
}
