use std::num;

use derive_more::Display;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Kind {
    // GH01xxxx.mp4
    File,
    // GHxx0001.mp4
    Chapter,
    // GHxx0001.mp4
    Loop,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid identifier len {0}")]
    InvalidIdentifierLen(usize),

    #[error(transparent)]
    ParseInt(#[from] num::ParseIntError),
}

impl TryFrom<&str> for Kind {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let kind = match value.len() {
            4 => Kind::File,
            2 => match value.parse::<usize>().is_ok() {
                true => Kind::Chapter,
                false => Kind::Loop,
            },
            len @ _ => return Err(Error::InvalidIdentifierLen(len)),
        };

        Ok(kind)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Hash, Clone, Display)]
#[display(fmt = "{}", "self.string()")]
pub struct Identifier {
    raw_value: String,
    kind: Kind,
}

impl Ord for Identifier {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let numeric1 = self.numeric();
        let numeric2 = other.numeric();
        if numeric1.is_ok() && numeric2.is_ok() {
            numeric1.unwrap().cmp(&numeric2.unwrap())
        } else {
            self.string().cmp(&other.string())
        }
    }
}

impl TryFrom<&str> for Identifier {
    type Error = Error;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        Ok(Identifier {
            raw_value: v.into(),
            kind: Kind::try_from(v)?,
        })
    }
}

impl Identifier {
    pub fn numeric(&self) -> Result<usize, Error> {
        self.raw_value.parse().map_err(From::from)
    }

    fn string(&self) -> String {
        match self.kind {
            Kind::Chapter => format!("{:0>2}", self.raw_value),
            Kind::File => format!("{:0>4}", self.raw_value),
            Kind::Loop => self.raw_value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_try_from_ok() {
        struct Test {
            input: &'static str,
            expected_string: &'static str,
            expected_kind: Kind,
            assert_numeric: Box<dyn Fn(Result<usize, Error>)>,
        }

        let tests = vec![
            Test {
                input: "0001",
                expected_string: "0001",
                expected_kind: Kind::File,
                assert_numeric: Box::new(|res: Result<usize, Error>| assert_eq!(1, res.unwrap())),
            },
            Test {
                input: "22",
                expected_string: "22",
                expected_kind: Kind::Chapter,
                assert_numeric: Box::new(|res: Result<usize, Error>| assert_eq!(22, res.unwrap())),
            },
            Test {
                input: "02",
                expected_string: "02",
                expected_kind: Kind::Chapter,
                assert_numeric: Box::new(|res: Result<usize, Error>| assert_eq!(2, res.unwrap())),
            },
            Test {
                input: "AA",
                expected_string: "AA",
                expected_kind: Kind::Loop,
                assert_numeric: Box::new(|res: Result<usize, Error>| match res {
                    Err(Error::ParseInt(_)) => {}
                    err @ _ => panic!("invalid numeric parsing result {:?}", err),
                }),
            },
        ];

        tests.into_iter().for_each(|test| {
            let id = Identifier::try_from(test.input).unwrap();
            assert_eq!(test.expected_string, id.string());
            assert_eq!(test.expected_kind, id.kind);
            (test.assert_numeric)(id.numeric());
        });
    }

    #[test]
    fn identifier_try_from_err() {
        let non_ok = vec![
            "fdafda",
            "",
            "aaa22",
            "090909ff",
            "1",
            "222",
            "2222222222",
            "0",
        ];
        non_ok
            .into_iter()
            .for_each(|st| assert!(Identifier::try_from(st).is_err()));
    }
}
