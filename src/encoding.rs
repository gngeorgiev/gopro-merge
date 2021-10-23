use derive_more::Display;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid encoding for file {0}. Supported encodings are AVC(GH), HEVC(GX): https://community.gopro.com/t5/en/GoPro-Camera-File-Naming-Convention/ta-p/390220#")]
    InvalidEncoding(String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Display)]
pub enum Encoding {
    #[display(fmt = "GH")]
    Avc,
    #[display(fmt = "GX")]
    Hevc,
}

impl Encoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Encoding::Avc => "GH",
            Encoding::Hevc => "GX",
        }
    }
}

impl TryFrom<&str> for Encoding {
    type Error = Error;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        if name.starts_with(Encoding::Avc.as_str()) {
            Ok(Encoding::Avc)
        } else if name.starts_with(Encoding::Hevc.as_str()) {
            Ok(Encoding::Hevc)
        } else {
            Err(Error::InvalidEncoding(name.into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_try_from() {
        let ok = vec!["GH", "GX"];
        ok.into_iter()
            .for_each(|i| assert!(Encoding::try_from(i).is_ok()));

        let non_ok = vec!["gh", "gh", "", "faasda"];
        non_ok
            .into_iter()
            .for_each(|i| assert!(Encoding::try_from(i).is_err()));
    }

    #[test]
    fn encoding_as_str() {
        assert_eq!("GH", Encoding::Avc.as_str());
        assert_eq!("GX", Encoding::Hevc.as_str());
    }
}
