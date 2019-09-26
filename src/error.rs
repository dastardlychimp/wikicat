use serde_json;
use hyper;
use wikiquery::error::WikiError;

use std::ops::Deref;


#[derive(Debug, PartialEq)]
pub enum Error
{
    NoMembers,
    Hyper(HyperError),
    Wiki(WikiError),
    Serde(SerdeError),
}


#[derive(Debug)]
pub struct HyperError(hyper::error::Error);

impl Deref for HyperError
{
    type Target = hyper::error::Error;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl PartialEq<HyperError> for HyperError
{
    fn eq(&self, _other: &HyperError) -> bool
    {
        true
    }
}

#[derive(Debug)]
pub struct SerdeError(serde_json::error::Error);

impl Deref for SerdeError
{
    type Target = serde_json::error::Error;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl PartialEq<SerdeError> for SerdeError
{
    fn eq(&self, _other: &SerdeError) -> bool
    {
        true
    }
}

impl Error
{
    pub fn is_hyper(&self) -> bool
    {
        match self
        {
            Error::Hyper(_) => true,
            _ => false,
        }
    }

    pub fn is_wiki(&self) -> bool
    {
        match self
        {
            Error::Wiki(_) => true,
            _ => false,
        }
    }

    pub fn is_serde(&self) -> bool
    {
        match self
        {
            Error::Serde(_) => true,
            _ => false,
        }
    }
}


impl From<hyper::error::Error> for Error
{
    fn from(error: hyper::error::Error) -> Error
    {
        Error::Hyper(HyperError(error))
    }
}

impl From<serde_json::error::Error> for Error
{
    fn from(error: serde_json::error::Error) -> Error
    {
        Error::Serde(SerdeError(error))
    }
}