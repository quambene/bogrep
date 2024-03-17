use super::{ParsedBookmarks, ReadSource, SeekRead};
use log::debug;
use std::io::{BufRead, BufReader};

/// Reader for txt files.
#[derive(Debug)]
pub struct TextReader;

impl ReadSource for TextReader {
    fn extension(&self) -> Option<&str> {
        Some("txt")
    }

    fn read_and_parse<'a>(
        &self,
        reader: &'a mut dyn SeekRead,
    ) -> Result<ParsedBookmarks<'a>, anyhow::Error> {
        debug!("Read file with extension: {:?}", self.extension());

        let buf_reader = BufReader::new(reader);
        let lines = buf_reader.lines();
        Ok(ParsedBookmarks::Text(lines))
    }
}
