use ::pn532::error::CommError as GenericCommError;
use ::i2cdev::linux::LinuxI2CError as I2CError;

pub type CommError = GenericCommError<I2CError, I2CError>;

#[derive(Debug)]
pub enum SetupError {
    I2C(I2CError),
    PN532(CommError),
}

impl From<I2CError> for SetupError {
    fn from(e: I2CError) -> Self {
        SetupError::I2C(e)
    }
}

impl From<CommError> for SetupError {
    fn from(e: CommError) -> Self {
        SetupError::PN532(e)
    }
}

#[derive(Debug)]
pub enum TagError {
    Comm(CommError),
    InvalidTag,
}

impl From<CommError> for TagError {
    fn from(e: CommError) -> Self {
        TagError::Comm(e)
    }
}

#[derive(Debug)]
pub enum AuthError<E> {
    Tag(CommError),
    Other(E),
    InvalidCredentials,
}

impl<E: ::std::fmt::Display> ::std::fmt::Display for AuthError<E> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            AuthError::Tag(ref e) => write!(f, "communication with tag failed: {}", e),
            AuthError::Other(ref e) => e.fmt(f),
            AuthError::InvalidCredentials => write!(f, "invalid credentials"),
        }
    }
}

impl<E: ::std::error::Error> ::std::error::Error for AuthError<E> {
    fn description(&self) -> &str {
        "authentication error"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}

impl<E> From<CommError> for AuthError<E> {
    fn from(e: CommError) -> Self {
        AuthError::Tag(e)
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    InvalidSector,
    InvalidLength,
}

impl ::std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use self::DatabaseError::*;

        match *self {
            InvalidSector => write!(f, "Data in database contain invalid sector number."),
            InvalidLength => write!(f, "Data in database has invalid length."),
        }
    }
}

impl ::std::error::Error for DatabaseError {
    fn description(&self) -> &str {
        "invalid data in database"
    }

    fn cause(&self) -> Option<&::std::error::Error> {
        None
    }
}
