use std::collections::HashMap;
use std::fs::DirEntry;
use std::io;

use anyhow::Result;

use crate::recording::{Group, Recording};

pub trait Grouper<T> {
    fn group(&self, items: impl Iterator<Item = T>) -> HashMap<Group, Vec<Recording>>;
}

pub struct FileGrouper {}

impl FileGrouper {
    pub fn new() -> Self {
        FileGrouper {}
    }
}

impl Grouper<DirEntry> for FileGrouper {
    fn group(&self, items: impl Iterator<Item = DirEntry>) -> HashMap<Group, Vec<Recording>> {
        items
            .into_iter()
            .filter_map(|f| match Recording::try_parse(f.file_name().to_str()?) {
                Ok(r) => Some(r),
                Err(err) => {
                    println!("{}", err);
                    None
                }
            })
            .fold(HashMap::<Group, Vec<Recording>>::new(), |mut acc, rec| {
                acc.entry(rec.group()).or_default().push(rec);
                acc
            })
    }
}

#[cfg(test)]
mod tests {}
