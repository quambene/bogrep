use crate::{
    args::AddArgs,
    bookmark_reader::{ReadTarget, WriteTarget},
    bookmarks::Action,
    utils, Config, SourceType, TargetBookmark, TargetBookmarks,
};
use anyhow::anyhow;
use chrono::Utc;
use log::debug;
use std::collections::HashSet;
use url::Url;

pub async fn add(config: Config, args: AddArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    let mut target_writer = utils::open_and_truncate_file(&config.target_bookmark_lock_file)?;
    let urls = args
        .urls
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;

    if !urls.is_empty() {
        add_urls(&urls, &mut target_reader, &mut target_writer)?;
    } else {
        return Err(anyhow!("Invalid argument: Specify the URLs to be added"));
    }

    utils::close_and_rename(
        (target_writer, &config.target_bookmark_lock_file),
        (target_reader, &config.target_bookmark_file),
    )?;

    Ok(())
}

fn add_urls(
    urls: &[Url],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let now = Utc::now();
    let cache_modes = HashSet::new();
    let mut sources = HashSet::new();
    sources.insert(SourceType::Internal);
    let mut target_bookmarks = TargetBookmarks::default();

    target_reader.read(&mut target_bookmarks)?;

    for url in urls {
        let bookmark = TargetBookmark::new(
            url.clone(),
            now,
            None,
            sources.clone(),
            cache_modes.clone(),
            Action::None,
        );
        target_bookmarks.insert(bookmark);
    }

    target_writer.write(&target_bookmarks)?;

    println!("Added {} bookmarks", urls.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;
    use crate::{json, JsonBookmarks};
    use std::io::{Cursor, Write};

    #[test]
    fn test_add_urls() {
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut expected_urls = HashSet::new();
        expected_urls.insert(url1.to_owned());
        expected_urls.insert(url2.to_owned());

        let target_bookmarks = TargetBookmarks::default();
        let bookmarks_json = JsonBookmarks::from(&target_bookmarks);
        let buf = json::serialize(bookmarks_json).unwrap();

        let mut target_reader: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        let urls = vec![url1, url2];

        let res = add_urls(&urls, &mut target_reader, &mut target_writer);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<JsonBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
        assert_eq!(actual_bookmarks.len(), 2);
        assert!(
            actual_bookmarks
                .iter()
                .all(|bookmark| bookmark.last_cached.is_none()
                    && bookmark.sources.contains(&SourceType::Internal)),
            "actual: {actual_bookmarks:#?}"
        );
        assert_eq!(
            actual_bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_urls
                .into_iter()
                .map(|url| url.to_string())
                .collect(),
        );
    }

    #[test]
    fn test_add_urls_existing() {
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let mut expected_urls = HashSet::new();
        expected_urls.insert(url1.to_owned());
        expected_urls.insert(url2.to_owned());

        let mut target_bookmarks = TargetBookmarks::default();
        target_bookmarks.insert(TargetBookmark::new(
            url1.clone(),
            Utc::now(),
            None,
            HashSet::new(),
            HashSet::new(),
            Action::FetchAndAdd,
        ));
        target_bookmarks.insert(TargetBookmark::new(
            url2.clone(),
            Utc::now(),
            None,
            HashSet::new(),
            HashSet::new(),
            Action::FetchAndAdd,
        ));
        let bookmarks_json = JsonBookmarks::from(&target_bookmarks);
        let buf = json::serialize(bookmarks_json).unwrap();

        let mut target_reader: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        let urls = vec![url1.to_owned(), url2.to_owned()];

        let res = add_urls(&urls, &mut target_reader, &mut target_writer);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let actual = target_writer.get_ref();
        let actual_bookmarks = json::deserialize::<JsonBookmarks>(actual);
        assert!(
            actual_bookmarks.is_ok(),
            "{}\n{}",
            actual_bookmarks.unwrap_err(),
            String::from_utf8(actual.to_owned()).unwrap()
        );

        let actual_bookmarks = actual_bookmarks.unwrap();
        assert_eq!(actual_bookmarks.len(), 2);
        assert!(
            actual_bookmarks
                .iter()
                .all(|bookmark| bookmark.last_cached.is_none()
                    && bookmark.sources.contains(&SourceType::Internal)),
            "actual: {actual_bookmarks:#?}"
        );
        assert_eq!(
            actual_bookmarks
                .iter()
                .map(|bookmark| bookmark.url.clone())
                .collect::<HashSet<_>>(),
            expected_urls
                .into_iter()
                .map(|url| url.to_string())
                .collect(),
        );
    }
}
