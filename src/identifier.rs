use anyhow::Result;

use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct Identifier {
    pub value: usize,
    pub representation: String,
}

impl Identifier {
    pub fn try_from(v: &str) -> Result<Self> {
        Ok(Identifier {
            value: v.parse()?,
            representation: v.into(),
        })
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.representation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_try_from() {
        let ok = vec![("000001", 1), ("022", 22), ("000033", 33)];
        ok.into_iter().for_each(|(st, num)| {
            let id = Identifier::try_from(st);
            assert!(id.is_ok());
            let id = id.unwrap();
            assert_eq!(st, id.representation);
            assert_eq!(num, id.value);
        });

        let non_ok = vec!["fdafda", "", "aaa22", "090909ff"];
        non_ok
            .into_iter()
            .for_each(|st| assert!(Identifier::try_from(st).is_err()));
    }
}
