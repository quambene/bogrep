use crate::{
    args::RemoveArgs,
    bookmark_reader::{ReadTarget, TargetReaderWriter, WriteTarget},
    bookmarks::{BookmarkManager, RunMode, ServiceConfig},
    Config,
};
use anyhow::anyhow;
use log::debug;
use url::Url;

pub async fn remove(config: Config, args: RemoveArgs) -> Result<(), anyhow::Error> {
    debug!("{args:?}");

    let target_reader_writer = TargetReaderWriter::new(
        &config.target_bookmark_file,
        &config.target_bookmark_lock_file,
    )?;
    let urls = args
        .urls
        .iter()
        .map(|url| Url::parse(url))
        .collect::<Result<Vec<_>, _>>()?;

    if !urls.is_empty() {
        let service_config = ServiceConfig::new(RunMode::RemoveUrls(urls.clone()), vec![]);

        remove_urls(
            service_config,
            &urls,
            &mut target_reader_writer.reader(),
            &mut target_reader_writer.writer(),
        )?;
    } else {
        return Err(anyhow!("Invalid argument: Specify the URLs to be removed"));
    }

    target_reader_writer.close()?;

    Ok(())
}

// TODO: Use `BookmarkService`.
fn remove_urls(
    config: ServiceConfig,
    urls: &[Url],
    target_reader: &mut impl ReadTarget,
    target_writer: &mut impl WriteTarget,
) -> Result<(), anyhow::Error> {
    let mut bookmark_manager = BookmarkManager::new();

    target_reader.read(bookmark_manager.target_bookmarks_mut())?;

    bookmark_manager.remove_urls(urls)?;
    bookmark_manager.print_report(&vec![], config.run_mode());
    bookmark_manager.finish();

    target_writer.write(bookmark_manager.target_bookmarks())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bookmarks::TargetBookmarkBuilder, json, JsonBookmarks, SourceType, TargetBookmarks,
    };
    use chrono::Utc;
    use std::{
        collections::HashSet,
        io::{Cursor, Write},
    };

    #[test]
    fn test_remove_urls() {
        let now = Utc::now();
        let url1 = Url::parse("https://url1.com").unwrap();
        let url2 = Url::parse("https://url2.com").unwrap();
        let url3 = Url::parse("https://url3.com").unwrap();

        let mut expected_urls = HashSet::new();
        expected_urls.insert(url1.clone());

        let mut target_bookmarks = TargetBookmarks::default();
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url1.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url2.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url3.clone(), now)
                .add_source(SourceType::Internal)
                .build(),
        );
        let bookmarks_json = JsonBookmarks::from(&target_bookmarks);
        let buf = json::serialize(bookmarks_json).unwrap();

        let mut target_reader: Cursor<Vec<u8>> = Cursor::new(Vec::new());
        target_reader.write_all(&buf).unwrap();
        // Set cursor position to the start again to prepare cursor for reading.
        target_reader.set_position(0);
        let mut target_writer = Cursor::new(Vec::new());
        let urls = vec![url2, url3];
        let run_config = ServiceConfig::default();

        let res = remove_urls(run_config, &urls, &mut target_reader, &mut target_writer);
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
