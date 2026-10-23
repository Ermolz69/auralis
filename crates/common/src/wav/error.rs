#[derive(Debug)]
pub enum Pcm16Error {
    Io(std::io::Error),
    Invalid(&'static str),
}

impl From<std::io::Error> for Pcm16Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl std::fmt::Display for Pcm16Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => std::fmt::Display::fmt(error, formatter),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for Pcm16Error {}
