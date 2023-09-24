use crate::{cache::CacheMode, utils, Cache, Caching, Config, TargetBookmarks};
use anyhow::anyhow;
use colored::Colorize;
use log::info;
use regex::{Captures, Regex};
use std::{
    borrow::Cow,
    io::{self, BufRead},
};

/// Maximum number of characters per line displayed in the search result.
const MAX_COLUMNS: usize = 1000;

pub fn search(
    pattern: String,
    config: &Config,
    cache_mode: &Option<CacheMode>,
) -> Result<(), anyhow::Error> {
    if config.verbosity >= 1 {
        info!("{pattern:?}");
    }

    let mut target_bookmark_file = utils::open_file(&config.target_bookmark_file)?;
    let target_bookmarks = TargetBookmarks::read(&mut target_bookmark_file)?;
    let cache = Cache::new(&config.cache_path, cache_mode);

    if target_bookmarks.bookmarks.is_empty() {
        Err(anyhow!("Missing bookmarks, run `bogrep import` first"))
    } else {
        search_bookmarks(pattern, &target_bookmarks, &cache)?;
        Ok(())
    }
}

#[allow(clippy::comparison_chain)]
fn search_bookmarks(
    pattern: String,
    bookmarks: &TargetBookmarks,
    cache: &impl Caching,
) -> Result<(), anyhow::Error> {
    let re = format!("(?i){pattern}");
    let regex = Regex::new(&re)?;

    for bookmark in &bookmarks.bookmarks {
        if let Some(cache_file) = cache.open(bookmark)? {
            let reader = io::BufReader::new(cache_file);
            let matched_lines = find_matches(reader, &regex)?;

            if matched_lines.len() == 1 {
                println!("Match in bookmark: {}", bookmark.url.blue());
            } else if matched_lines.len() > 1 {
                println!("Matches in bookmark: {}", bookmark.url.blue());
            }

            for matched_line in &matched_lines {
                println!("{}", color_matches(matched_line, &regex));
            }
        }
    }

    Ok(())
}

/// Find the matched lines for the regex in a file.
fn find_matches(reader: impl BufRead, regex: &Regex) -> Result<Vec<String>, anyhow::Error> {
    let mut matched_lines = vec![];

    for line in reader.lines() {
        let start_index;
        let end_index;
        let line = line?;

        if regex.is_match(&line) {
            if line.len() >= MAX_COLUMNS {
                let first_match = regex.find(&line).ok_or(anyhow!("Can't find first match"))?;
                let match_start = first_match.start();
                let match_end = first_match.end();
                let half_max = MAX_COLUMNS / 2;
                start_index = match_start.saturating_sub(half_max);
                end_index = (match_end + half_max).min(line.len());
            } else {
                start_index = 0;
                end_index = line.len();
            }

            let truncated_line = &line[start_index..end_index];

            matched_lines.push(truncated_line.to_owned());
        }
    }

    Ok(matched_lines)
}

/// Display search pattern in bold red.
fn color_matches<'a>(matched_line: &'a str, regex: &Regex) -> Cow<'a, str> {
    regex.replace_all(matched_line, |caps: &Captures| {
        caps[0].bold().red().to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_find_matches() {
        let content = r#"To avoid errors, you once again add extra information. Here, you send they-value that corresponds to another predeterminedx-coordinate. If the three points do not fall on the same line, there’s an error. And to figure out where the error is, you just send one more value — meaning you’ve sent four numbers total, rather than the six required by the previous method.The advantage grows with the size of the message. Let’s say you want to send a longer message — 1,000 numbers. The less efficient code would require sending 2,000 numbers to identify an error, and 3,000 to correct it. But if you use the code that involves interpolating a polynomial through given points, you only need 1,001 numbers to find the error, and 1,002 to correct it. (You can add more points to identify and correct more potential errors.) As the length of your message increases, the difference in efficiency between the two codes grows starker.The more efficient code is called a Reed-Solomon code. Since its introduction in 1960, mathematicians have made further breakthroughs, developing algorithms that can correct more errors with greater efficiency. “It’s very elegant, clean, concrete,” saidSwastik Kopparty, a mathematician and computer scientist at the University of Toronto. “It can be taught to a second-year undergraduate in half an hour.”Reed-Solomon codes have been particularly useful for storing and transmitting information electronically. But the same concept has also been essential in cryptography and distributed computing.Take secret sharing: Let’s say you want to distribute a secret among several parties such that no one person can access the entire secret, but together they can. (Imagine an encryption key, for instance, or a missile launch code.) You encode the numbers in a polynomial, evaluate that polynomial at a predetermined set of points, and distribute each of the results to a different person.Most recently, Reed-Solomon codes have been employed in areas like cloud computing and blockchain technology. Say you need to run a computation that’s too complicated for your laptop, so you have a large computational cluster run it — but now you need to verify that the computation you get back is correct. Reed-Solomon codes let you ask for additional information that the cluster likely won’t be able to produce if it hasn’t done the computation correctly. “This works magically,” saidJade Nardi, a research fellow at the Mathematics Institute of Rennes in France. “This process is really wonderful, and the way it relies on [these codes] blows my mind.”But Reed-Solomon codes also have an important constraint. They’re constructed in such a way that you can only evaluate your polynomial at a fixed (and usually relatively small) set of values. That is, you’re limited to using a certain set of numbers to encode your message. The size of that set, or alphabet, in turn restricts the length of the messages you can send — and the bigger you try to make your alphabet, the more computational power you’ll need to decode those messages.And so mathematicians sought an even more optimal code.Future CodesA more general, more powerful code would allow you to store or send longer messages without needing to increase the size of your alphabet. To do this, mathematicians devised codes that involve interpolating a function — which lives in a special space associated to a more complicated curve — through given points on that curve. These so-called algebraic geometry codes “came out of nowhere, and they’re better than any other code we know how to make [with a smaller alphabet],” Kopparty said. “This beats everything. It was a real shock.”There’s just one problem. In practice, implementing a Reed-Solomon code is much, much easier than implementing an algebraic geometry code. “This is state-of-the-art, but it’s still under investigation to really turn into something practical,” said the cryptologistSimon Abelard. “It involves quite abstract mathematics, and it’s hard to handle these codes on a computer.”For now, that’s not worrisome: In real-world applications, Reed-Solomon codes and related forms of error correction are sufficient. But that might not always be the case. For instance, if powerful quantum computers become available in the future, they’ll be able tobreak today’s cryptography protocols. As a result, researchers have been searching for schemes that can resist quantum attacks. One top contender for such schemes would require something stronger than Reed-Solomon codes. Certain versions of algebraic geometry codes might just work. Other researchers are hopeful about the role algebraic geometry codes might play in cloud computing.But even in the absence of such potential uses, “in the history of mathematics, sometimes you discover new things that really don’t have applications nowadays,” saidElena Berardini, a researcher at Eindhoven University of Technology in the Netherlands who works on algebraic geometry codes. “But then after 50 years, you find that it might be useful for something completely unexpected” — just like the ancient problem of interpolation itself."#;
        let cursor = Cursor::new(content);
        let re = format!("(?i)reed-solomon code");
        let regex = Regex::new(&re).unwrap();

        let res = find_matches(cursor, &regex);
        assert!(res.is_ok());
        let matched_lines = res.unwrap();
        assert_eq!(
            matched_lines,
            vec!["— 1,000 numbers. The less efficient code would require sending 2,000 numbers to identify an error, and 3,000 to correct it. But if you use the code that involves interpolating a polynomial through given points, you only need 1,001 numbers to find the error, and 1,002 to correct it. (You can add more points to identify and correct more potential errors.) As the length of your message increases, the difference in efficiency between the two codes grows starker.The more efficient code is called a Reed-Solomon code. Since its introduction in 1960, mathematicians have made further breakthroughs, developing algorithms that can correct more errors with greater efficiency. “It’s very elegant, clean, concrete,” saidSwastik Kopparty, a mathematician and computer scientist at the University of Toronto. “It can be taught to a second-year undergraduate in half an hour.”Reed-Solomon codes have been particularly useful for storing and transmitting information electronically. But the same concept has also b".to_owned()]
        );
    }
}
