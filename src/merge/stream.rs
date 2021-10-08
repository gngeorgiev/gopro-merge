use std::io::Read;
use std::io::{BufRead, BufReader};
use std::ops::Add;
use std::time::Duration;

use crate::merge::Error::{self};
use crate::merge::Result;
use crate::progress::Progress;

use log::*;

pub trait CommandStreamDurationParser<T: Read, V: Default> {
    fn parse(&mut self) -> Result<V>;
}

pub struct FfprobeDurationParser<T: Read> {
    stream: Option<T>,
}

impl<T: Read> CommandStreamDurationParser<T, Duration> for FfprobeDurationParser<T> {
    fn parse(&mut self) -> Result<Duration> {
        let duration =
            parse_command_stream(self.stream.take().unwrap(), |name: &str, value: &str| {
                if name != "duration" {
                    return Ok(None);
                }

                let mut split = value.split('.');
                let seconds = Duration::from_secs(
                    split
                        .next()
                        .ok_or_else(|| Error::InvalidOutputLine(value.into()))?
                        .parse()?,
                );
                let micros = Duration::from_micros(
                    split
                        .next()
                        .ok_or_else(|| Error::InvalidOutputLine(value.into()))?
                        .parse()?,
                );

                Ok(Some(Duration::default().add(seconds).add(micros)))
            })?;

        Ok(duration)
    }
}

impl<T: Read> FfprobeDurationParser<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

pub struct FfmpegDurationProgressParser<'a, T: Read, P: Progress> {
    stream: Option<T>,
    pb: &'a mut P,
}

impl<'a, T: Read, P: Progress> CommandStreamDurationParser<T, ()>
    for FfmpegDurationProgressParser<'a, T, P>
{
    fn parse(&mut self) -> Result<()> {
        parse_command_stream(
            self.stream.take().unwrap(),
            |name: &str, value: &str| match name {
                "out_time" => {
                    let progress = self.parse_timestamp_match(value)?;
                    self.pb.update(progress);
                    Ok(None)
                }
                _ => Ok(None),
            },
        )?;

        Ok(())
    }
}

impl<'a, T: Read, P: Progress> FfmpegDurationProgressParser<'a, T, P> {
    pub fn new(stream: T, pb: &'a mut P) -> Self {
        Self {
            stream: stream.into(),
            pb,
        }
    }

    fn parse_timestamp_match(&self, input: &str) -> Result<Duration> {
        macro_rules! parse {
            ($iter:expr) => {
                $iter.next().unwrap_or("0").parse::<u64>()?
            };
        }

        let mut millis_split = input.split('.');
        let mut secs_split = millis_split
            .next()
            .ok_or_else(|| Error::InvalidOutputLine(input.into()))?
            .split(':');
        let hours_duration = Duration::from_secs(parse!(secs_split) * 60 * 60);
        let minutes_duration = Duration::from_secs(parse!(secs_split) * 60);
        let seconds_duration = Duration::from_secs(parse!(secs_split));
        let millis_duration = Duration::from_micros(parse!(millis_split));

        Ok(Duration::default()
            .add(hours_duration)
            .add(minutes_duration)
            .add(seconds_duration)
            .add(millis_duration))
    }
}

fn parse_command_stream<V: Default>(
    stream: impl Read,
    mut parse: impl FnMut(&str, &str) -> Result<Option<V>>,
) -> Result<V> {
    let stdout_reader = BufReader::new(stream);
    let mut lines = stdout_reader.lines();

    while let Some(Ok(line)) = lines.next() {
        trace!("get_duration_from_command_stream line {}", &line);
        let mut split = line.split("=");

        let output_field_name = match split.next() {
            Some(name) => name,
            None => continue,
        };

        let output_field_value = match split.next() {
            Some(value) => value,
            None => continue,
        };

        if let Some(value) = parse(output_field_name, output_field_value)? {
            return Ok(value);
        }
    }

    Ok(Default::default())
}
