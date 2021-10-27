use std::io::{BufRead, BufReader, Read};
use std::ops::Add;
use std::str::Split;
use std::time::Duration;

use crate::merge::Result;
use crate::progress::Progress;

use log::*;

struct CharToU64Iter<'a>(Split<'a, char>);

impl<'a> Iterator for CharToU64Iter<'a> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|c| c.parse::<u64>().unwrap_or_default())
    }
}

impl<'a> CharToU64Iter<'a> {
    fn next_default(&mut self) -> <Self as Iterator>::Item {
        self.next().unwrap_or_default()
    }
}

pub trait CommandStreamDurationParser<T: Read, V: Default> {
    fn parse(&mut self) -> Result<V>;
}

pub struct FFprobeDurationParser<T: Read> {
    stream: Option<T>,
}

impl<T: Read> CommandStreamDurationParser<T, Duration> for FFprobeDurationParser<T> {
    fn parse(&mut self) -> Result<Duration> {
        let duration = parse_command_stream(self.stream.take().unwrap(), |name, value| {
            if name != "duration" {
                return None;
            }

            let mut split = CharToU64Iter(value.split('.'));
            let seconds = Duration::from_secs(split.next_default());
            let micros = Duration::from_micros(split.next_default());

            Some(seconds.add(micros))
        })?;

        Ok(duration)
    }
}

impl<T: Read> FFprobeDurationParser<T> {
    pub fn new(stream: T) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

pub struct FFmpegDurationProgressParser<'a, T: Read, P: Progress> {
    stream: Option<T>,
    pb: &'a mut P,
}

impl<'a, T: Read, P: Progress> CommandStreamDurationParser<T, ()>
    for FFmpegDurationProgressParser<'a, T, P>
{
    fn parse(&mut self) -> Result<()> {
        parse_command_stream(self.stream.take().unwrap(), |name, value| match name {
            "out_time" => {
                self.pb.update(self.parse_timestamp_match(value));
                None
            }
            _ => None,
        })?;

        Ok(())
    }
}

impl<'a, T: Read, P: Progress> FFmpegDurationProgressParser<'a, T, P> {
    pub fn new(stream: T, pb: &'a mut P) -> Self {
        Self {
            stream: stream.into(),
            pb,
        }
    }

    fn parse_timestamp_match(&self, input: &str) -> Duration {
        let mut micros_split = input.split('.');
        let mut secs_split = CharToU64Iter(micros_split.next().unwrap_or("0:0:0").split(':'));

        let hours = Duration::from_secs(secs_split.next_default() * 60 * 60);
        let minutes = Duration::from_secs(secs_split.next_default() * 60);
        let seconds = Duration::from_secs(secs_split.next_default());
        let micros = Duration::from_micros(CharToU64Iter(micros_split).next_default());

        [hours, minutes, seconds, micros].iter().sum()
    }
}

fn parse_command_stream<V: Default>(
    stream: impl Read,
    mut parse: impl FnMut(&str, &str) -> Option<V>,
) -> Result<V> {
    let stdout_reader = BufReader::new(stream);
    let mut lines = stdout_reader.lines();

    while let Some(Ok(line)) = lines.next() {
        trace!("get_duration_from_command_stream line {}", &line);

        let mut split = line.split('=');
        match (split.next(), split.next()) {
            (Some(name), Some(value)) => match parse(name, value) {
                Some(parsed) => return Ok(parsed),
                _ => continue,
            },
            _ => continue,
        }
    }

    Ok(Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Write;

    #[test]
    fn test_ffmpeg_parse_duration() {
        #[derive(Clone)]
        struct MockProgress {}
        impl Progress for MockProgress {
            fn update(&mut self, _: Duration) {}
            fn set_len(&mut self, _: Duration) {}
            fn finish(&self) {}
        }

        [
            (
                "00:06:49.00",
                Duration::from_secs(6 * 60).add(Duration::from_secs(49)),
            ),
            (
                "00:06:49.100",
                [
                    Duration::from_secs(6 * 60),
                    Duration::from_secs(49),
                    Duration::from_micros(100),
                ]
                .into_iter()
                .sum::<Duration>(),
            ),
            (
                "01:06:49.100",
                [
                    Duration::from_secs(60 * 60),
                    Duration::from_secs(6 * 60),
                    Duration::from_secs(49),
                    Duration::from_micros(100),
                ]
                .into_iter()
                .sum::<Duration>(),
            ),
            (
                "02:06:49.100",
                [
                    Duration::from_secs(2 * 60 * 60),
                    Duration::from_secs(6 * 60),
                    Duration::from_secs(49),
                    Duration::from_micros(100),
                ]
                .into_iter()
                .sum::<Duration>(),
            ),
            ("00:00:00.000", Duration::default()),
            ("000:0000:0.000000", Duration::default()),
        ]
        .into_iter()
        .for_each(|(input, expected)| {
            let s = String::new();
            let mut p = MockProgress {};
            let parser = FFmpegDurationProgressParser::new(s.as_bytes(), &mut p);

            let result = parser.parse_timestamp_match(input);
            assert_eq!(expected, result);
        });
    }

    #[test]
    fn test_ffmpeg_parse_duration_stream() {
        #[derive(Clone, Default)]
        struct MockProgress {
            total_duration: Duration,
        }

        impl Progress for MockProgress {
            fn update(&mut self, v: Duration) {
                self.total_duration = self.total_duration.add(v)
            }

            fn set_len(&mut self, _: Duration) {}

            fn finish(&self) {}
        }

        fn stream_data(values: &[&'static str]) -> String {
            let mut d = String::new();
            values.iter().for_each(|v| {
                writeln!(d, "out_time={}", v).unwrap();
                writeln!(d, "other_key_name={}", v).unwrap();
            });

            d
        }

        [(
            stream_data(&["01:00:00.0", "2:0:0.0", "0:01:00.0", "0:01:01.100"]),
            [
                Duration::from_secs(60 * 60),
                Duration::from_secs(2 * 60 * 60),
                Duration::from_secs(60),
                Duration::from_secs(60),
                Duration::from_secs(1),
                Duration::from_micros(100),
            ]
            .into_iter()
            .sum::<Duration>(),
        )]
        .into_iter()
        .for_each(|(stream, expected)| {
            let mut p = MockProgress::default();
            let mut parser = FFmpegDurationProgressParser::new(stream.as_bytes(), &mut p);

            parser.parse().unwrap();

            assert_eq!(expected, p.total_duration);
        });
    }

    #[test]
    fn test_ffprobe_duration_parse_stream() {
        fn stream_data(v: &'static str) -> String {
            let mut d = String::new();
            writeln!(d, "duration={}", v).unwrap();
            writeln!(d, "other_key_name={}", v).unwrap();
            d
        }

        [
            (stream_data("5.0"), Duration::from_secs(5)),
            (
                stream_data("99.10"),
                Duration::from_secs(99).add(Duration::from_micros(10)),
            ),
            (
                stream_data("100.10000"),
                Duration::from_secs(100).add(Duration::from_micros(10000)),
            ),
            (stream_data("0000.0000"), Duration::default()),
            (stream_data("1111."), Duration::from_secs(1111)),
            (stream_data(".1"), Duration::from_micros(1)),
        ]
        .into_iter()
        .for_each(|(input, expected)| {
            let result = FFprobeDurationParser::new(input.as_bytes())
                .parse()
                .unwrap();

            assert_eq!(expected, result);
        })
    }
}
