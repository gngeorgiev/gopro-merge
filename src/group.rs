use std::collections::HashMap;

use crate::recording::{Group, Recording};
use std::fs::read_dir;
use std::io;
use std::path::PathBuf;

use anyhow::Result;

pub fn recording_groups(path: &PathBuf) -> Result<Vec<(Group, Vec<Recording>)>> {
    let files = read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    let recordings = files
        .into_iter()
        .filter_map(|f| f.file_name().to_str().map(|f| f.to_string()))
        .filter_map(|rec| Recording::try_parse(&rec).ok());

    Ok(groups_from_recordings(recordings))
}

fn groups_from_recordings(
    recordings: impl Iterator<Item = Recording>,
) -> Vec<(Group, Vec<Recording>)> {
    let mut v = recordings
        .fold(HashMap::<Group, Vec<Recording>>::new(), |mut acc, rec| {
            acc.entry(rec.group.clone()).or_default().push(rec);
            acc
        })
        .drain()
        .into_iter()
        .map(|(grp, mut v)| {
            v.sort_by(|a, b| a.chapter.value.cmp(&b.chapter.value));
            (grp, v)
        })
        .collect::<Vec<_>>();

    v.sort_by(|a, b| a.0.file.cmp(&b.0.file));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_recordings() {
        let rec1 = Recording::try_parse("GH011234.mp4").unwrap();
        let rec2 = Recording::try_parse("GH021234.mp4").unwrap();
        let rec3 = Recording::try_parse("GH011235.mp4").unwrap();

        let input = vec![rec1, rec2, rec3].into_iter();
        let res = groups_from_recordings(input);
        assert_eq!(res.len(), 2);

        let grp1 = &res[0];
        assert_eq!(grp1, &(rec1.group.clone(), vec![rec1.clone(), rec2]));

        let grp2 = &res[1];
        assert_eq!(grp2, &(rec3.group.clone(), vec![rec3]));
    }
}
