use crate::{errors::BogrepError, json, JsonBookmarks, SourceType, TargetBookmarks};
use std::io::{Read, Seek};
use url::Url;

/// Extension trait for [`Read`] and [`Seek`] to read target bookmarks.
pub trait ReadTarget {
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError>;
}

impl<T> ReadTarget for T
where
    T: Read + Seek,
{
    fn read(&mut self, target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
        let mut buf = Vec::new();
        self.read_to_end(&mut buf).map_err(BogrepError::ReadFile)?;

        // Rewind after reading.
        self.rewind().map_err(BogrepError::RewindFile)?;

        let bookmarks = json::deserialize::<JsonBookmarks>(&buf)?;

        for bookmark in bookmarks {
            target_bookmarks.insert(bookmark.try_into()?);
        }

        convert_underlyings(target_bookmarks)?;

        Ok(())
    }
}

pub fn convert_underlyings(target_bookmarks: &mut TargetBookmarks) -> Result<(), BogrepError> {
    let underlying_bookmarks = target_bookmarks
        .values()
        .filter_map(|bookmark| {
            if bookmark
                .sources()
                .iter()
                .any(|source| matches!(source, SourceType::Underlying(_)))
            {
                Some(bookmark.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for underlying_bookmark in underlying_bookmarks {
        let underlying_source = underlying_bookmark
            .sources()
            .iter()
            .find(|source| matches!(source, SourceType::Underlying(_)));
        let underlying_url = match underlying_source {
            Some(SourceType::Underlying(underlying_url)) => {
                let underlying_url = Url::parse(underlying_url)?;
                Some(underlying_url)
            }
            _ => None,
        };

        if let Some(underlying_url) = underlying_url {
            if let Some(bookmark) = target_bookmarks.get_mut(&underlying_url) {
                bookmark.set_underlying_url(underlying_bookmark.url().clone());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bookmarks::TargetBookmarkBuilder, CacheMode, UnderlyingType};
    use chrono::Utc;

    #[test]
    fn test_convert_underlyings() {
        let now = Utc::now();
        let mut target_bookmarks = TargetBookmarks::default();
        let url1 = Url::parse("https://news.ycombinator.com/item?id=00000000").unwrap();
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url1.clone(), now)
                .add_source(SourceType::Internal)
                .add_cache_mode(CacheMode::Text)
                .build(),
        );
        let url2 = Url::parse("https://github.com/some_project").unwrap();
        target_bookmarks.insert(
            TargetBookmarkBuilder::new(url2.clone(), now)
                .add_source(SourceType::Underlying(
                    "https://news.ycombinator.com/item?id=00000000".to_owned(),
                ))
                .add_cache_mode(CacheMode::Text)
                .build(),
        );

        let res = convert_underlyings(&mut target_bookmarks);
        assert!(
            res.is_ok(),
            "Can't convert underlyings: {}",
            res.unwrap_err()
        );

        let bookmark1 = target_bookmarks.get(&url1).unwrap();
        assert_eq!(bookmark1.underlying_url(), Some(&url2));
        assert_eq!(bookmark1.underlying_type(), &UnderlyingType::HackerNews);
        let bookmark2 = target_bookmarks.get(&url2).unwrap();
        assert!(bookmark2.underlying_url().is_none());
        assert_eq!(bookmark2.underlying_type(), &UnderlyingType::None);
    }
}
