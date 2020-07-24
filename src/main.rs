use std::collections::HashMap;
use std::env;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, DirEntry, OpenOptions};
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Error, Result};
use rayon::prelude::*;
use structopt::StructOpt;

mod encoding;
mod grouper;
mod identifier;
mod recording;

use crate::grouper::{FileGrouper, Grouper};
use crate::recording::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "gopro-join")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: Option<PathBuf>,

    #[structopt(parse(from_os_str))]
    output: Option<PathBuf>,
}

impl Opt {
    fn get_input(&self) -> Result<PathBuf> {
        let wd = env::current_dir()?;
        let path = match &self.input {
            Some(path) => wd.join(path).canonicalize()?,
            None => wd,
        };

        Ok(path)
    }

    fn get_output(&self) -> Result<PathBuf> {
        match &self.output {
            Some(out) => Ok(out.clone()),
            None => self.get_input(),
        }
    }
}

fn main() -> Result<()> {
    let opt: Opt = Opt::from_args();

    let input = opt.get_input()?;
    let output = opt.get_output()?;

    let mut recordings = get_recordings_grouped(&input)?;
    let paths_for_cleanup = process_recordings(input, output, &mut recordings)?;
    println!("Done for all");

    println!("Cleaning up");
    paths_for_cleanup
        .into_iter()
        .map(|f| fs::remove_file(&f).with_context(|| format!("removing file {:?}", f)))
        .collect::<Result<_>>()?;

    println!("Cleaned up");

    println!("Bye");

    Ok(())
}

fn get_recordings_grouped(at: &PathBuf) -> Result<HashMap<Group, Vec<Recording>>> {
    let files = fs::read_dir(at)?.collect::<io::Result<Vec<_>>>();
    let grp = FileGrouper::new();
    Ok(grp.group(files?.into_iter()))
}

fn process_recordings(
    input_dir: PathBuf,
    output_dir: PathBuf,
    recordings: &mut HashMap<Group, Vec<Recording>>,
) -> Result<Vec<PathBuf>> {
    let pairs = recordings
        .drain()
        .into_iter()
        .map(|(grp, mut v)| {
            v.sort_by(|a, b| a.chapter.value.cmp(&b.chapter.value));
            (grp, v)
        })
        .collect::<Vec<_>>();

    let paths_for_cleanup = pairs
        .into_par_iter()
        .map(|(grp, v): (Group, Vec<Recording>)| {
            let tmp_input_files_path =
                env::temp_dir().join(format!("{}.txt", grp.file.representation));

            if tmp_input_files_path.exists() {
                fs::remove_file(&tmp_input_files_path)?;
            }

            let mut tmp_input_files = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&tmp_input_files_path)
                .with_context(|| {
                    format!("creating tmp input file at {:?}", &tmp_input_files_path)
                })?;

            for rec in &v {
                let line = format!(
                    "file '{}'\r\n",
                    input_dir.join(rec.name()).to_str().unwrap()
                );
                tmp_input_files
                    .write(line.as_bytes())
                    .with_context(|| "writing input file contents")?;
            }

            println!(
                "Joining {:?} to {}",
                v.iter().map(|r| r.name()).collect::<Vec<_>>(),
                grp.name()
            );

            // https://trac.ffmpeg.org/wiki/Concatenate
            let mut child = Command::new("ffmpeg")
                .args(&[
                    "-f",
                    "concat",
                    "-safe",
                    "0",
                    "-y",
                    "-i",
                    tmp_input_files_path
                        .as_os_str()
                        .to_str()
                        .unwrap_or_default(),
                    "-c",
                    "copy",
                    output_dir
                        .join(grp.name())
                        .as_os_str()
                        .to_str()
                        .unwrap_or_default(),
                ])
                .spawn()?;

            if !child.wait()?.success() {
                return Err(Error::msg(format!("failed to convert {}", grp.name())));
            }

            println!("Done {}", grp.name());
            Ok(tmp_input_files_path)
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(paths_for_cleanup)
}

#[cfg(test)]
mod tests {
    use super::*;
}
