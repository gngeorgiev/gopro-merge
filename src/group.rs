use std::{collections::HashMap, convert::TryFrom};

use crate::recording::{Recording, RecordingFingerprint, RecordingGroup, RecordingGroups};
use std::fs::read_dir;
use std::io;
use std::path::PathBuf;

use anyhow::Result;

pub fn recording_groups(path: &PathBuf) -> Result<RecordingGroups> {
    let files = read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    let recordings = files
        .into_iter()
        .filter_map(|f| f.file_name().to_str().map(|f| f.to_string()))
        .filter_map(|rec| Recording::try_from(rec.as_str()).ok());

    Ok(groups_from_recordings(recordings))
}

fn groups_from_recordings(recordings: impl Iterator<Item = Recording>) -> RecordingGroups {
    recordings
        .fold(
            HashMap::<RecordingFingerprint, RecordingGroup>::new(),
            |mut acc, rec| {
                let group = acc
                    .entry(rec.fingerprint.clone())
                    .or_insert_with(|| RecordingGroup {
                        fingerprint: rec.fingerprint,
                        chapters: vec![],
                    });
                group.chapters.push(rec.chapter);
                acc
            },
        )
        .drain()
        .into_iter()
        .map(|(_, mut v)| {
            v.chapters.sort_by(|a, b| a.value.cmp(&b.value));
            v
        })
        .collect::<RecordingGroups>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_recordings() {
        let rec1 = Recording::try_parse("GH011234.mp4").unwrap();
        let rec2 = Recording::try_parse("GH021234.mp4").unwrap();
        let rec3 = Recording::try_parse("GH011235.mp4").unwrap();

        let input = vec![rec1.clone(), rec2.clone(), rec3.clone()].into_iter();
        let res = groups_from_recordings(input);
        assert_eq!(res.len(), 2);

        let grp1 = &res[0];
        assert_eq!(grp1, &(rec1.group.clone(), vec![rec1.clone(), rec2]));

        let grp2 = &res[1];
        assert_eq!(grp2, &(rec3.group.clone(), vec![rec3]));
    }
}
