use crate::{
    args::RemoveArgs,
    bookmark_reader::{ReadTarget, WriteTarget},
    utils, Config, TargetBookmarks,
};
use anyhow::anyhow;
use log::debug;
use url::Url;

pub async fn remove(config: Config, args: RemoveArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let mut target_reader = utils::open_file_in_read_mode(&config.target_bookmark_file)?;
    let mut target_writer = utils::open_and_truncate_file(&config.target_bookmark_lock_file)?;
    let urls = args
        .urls
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;

    if !urls.is_empty() {
        remove_urls(&urls, &mut target_reader, &mut target_writer)?;
    } else {
        return Err(anyhow!("Invalid argument: Specify the URLs to be removed"));
    }

    utils::close_and_rename(
        (target_writer, &config.target_bookmark_lock_file),
        (target_reader, &config.target_bookmark_file),
    )?;

    Ok(())
}

fn remove_urls(
    urls: &[Url],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let mut counter = 0;
    let mut target_bookmarks = TargetBookmarks::default();

    target_reader.read(&mut target_bookmarks)?;

    for url in urls {
        if target_bookmarks.remove(&url).is_some() {
            counter += 1;
        }
    }

    target_writer.write(&target_bookmarks)?;

    println!("Removed {} bookmarks", counter);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmarks::Action, json, JsonBookmarks, SourceType, TargetBookmark};
    use chrono::{DateTime, Utc};
    use std::{
        collections::HashSet,
        io::{Cursor, Write},
    };

    fn create_target_bookmark(url: &Url, now: DateTime<Utc>) -> TargetBookmark {
        let mut sources = HashSet::new();
        sources.insert(SourceType::Internal);
        TargetBookmark::new(
            url.to_owned(),
            now,
            None,
            sources,
            HashSet::new(),
            Action::None,
        )
    }

    #[test]
    fn test_remove_urls() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();

        let mut expected_urls = HashSet::new();
        expected_urls.insert(url1.clone());

        let mut target_bookmarks = TargetBookmarks::default();
        target_bookmarks.insert(create_target_bookmark(&url1, now));
        target_bookmarks.insert(create_target_bookmark(&url2, now));
        target_bookmarks.insert(create_target_bookmark(&url3, now));
        let bookmarks_json = JsonBookmarks::from(&target_bookmarks);
        let buf = json::serialize(bookmarks_json).unwrap();

        let mut target_reader: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());

        let urls = vec![url2, url3];

        let res = remove_urls(&urls, &mut target_reader, &mut target_writer);
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
        assert_eq!(actual_bookmarks.len(), 1);
        assert!(actual_bookmarks
            .iter()
            .all(|bookmark| bookmark.last_cached.is_none()));
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
