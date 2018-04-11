use clap::ArgMatches;
use failure::Fail;
use ffsend_api::action::exists::{
    Error as ExistsError,
    Exists as ApiExists,
};
use ffsend_api::action::info::{
    Error as InfoError,
    Info as ApiInfo,
};
use ffsend_api::action::metadata::Metadata as ApiMetadata;
use ffsend_api::file::remote_file::{
    FileParseError,
    RemoteFile,
};
use ffsend_api::reqwest::Client;

use cmd::matcher::{
    Matcher,
    info::InfoMatcher,
};
use util::{print_error, ensure_password};

/// A file info action.
pub struct Info<'a> {
    cmd_matches: &'a ArgMatches<'a>,
}

impl<'a> Info<'a> {
    /// Construct a new info action.
    pub fn new(cmd_matches: &'a ArgMatches<'a>) -> Self {
        Self {
            cmd_matches,
        }
    }

    /// Invoke the info action.
    // TODO: create a trait for this method
    pub fn invoke(&self) -> Result<(), Error> {
        // Create the command matchers
        let matcher_info = InfoMatcher::with(self.cmd_matches).unwrap();

        // Get the share URL
        let url = matcher_info.url();

        // Create a reqwest client
        let client = Client::new();

        // Parse the remote file based on the share URL, get the password
        let file = RemoteFile::parse_url(url, matcher_info.owner())?;
        let mut password = matcher_info.password();

        // TODO: show an informative error if the owner token isn't set

        // Check whether the file exists
        let exists = ApiExists::new(&file).invoke(&client)?;
        if !exists.exists() {
            return Err(Error::Expired);
        }

        // Ensure a password is set when required
        ensure_password(&mut password, exists.has_password());

        // Fetch both file info and metadata
        let info = ApiInfo::new(&file, None).invoke(&client)?;
        let metadata = ApiMetadata::new(&file, password, false)
            .invoke(&client)
            .map_err(|err| print_error(err.context(
                "Failed to fetch file metadata, showing limited info",
            )))
            .ok();

        // Print the result
        println!("ID: {}", file.id());
        if let Some(metadata) = metadata {
            println!("File name: {}", metadata.metadata().name());
            println!("MIME type: {}", metadata.metadata().mime());
        }
        println!("Downloads: {} of {}", info.download_count(), info.download_limit());
        println!("TTL: {} ms", info.ttl_millis());

        // TODO: show the file size, fetch TTL from metadata?

        Ok(())
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    /// Failed to parse a share URL, it was invalid.
    /// This error is not related to a specific action.
    #[fail(display = "Invalid share URL")]
    InvalidUrl(#[cause] FileParseError),

    /// An error occurred while checking if the file exists.
    #[fail(display = "Failed to check whether the file exists")]
    Exists(#[cause] ExistsError),

    /// An error occurred while fetching the file information.
    #[fail(display = "Failed to fetch file info")]
    Info(#[cause] InfoError),

    /// The given Send file has expired, or did never exist in the first place.
    #[fail(display = "The file has expired or did never exist")]
    Expired,
}

impl From<FileParseError> for Error {
    fn from(err: FileParseError) -> Error {
        Error::InvalidUrl(err)
    }
}

impl From<ExistsError> for Error {
    fn from(err: ExistsError) -> Error {
        Error::Exists(err)
    }
}

impl From<InfoError> for Error {
    fn from(err: InfoError) -> Error {
        Error::Info(err)
    }
}
