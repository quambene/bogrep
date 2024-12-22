use crate::{
    dom::{Node, NodeData, RcDom, SerializableHandle},
    errors::BogrepError,
    UnderlyingType,
};
use html5ever::{
    parse_document,
    serialize::{serialize, SerializeOpts},
    ParseOpts,
};
use log::{debug, trace};
use markup5ever::interface::QualName;
use readability::{extract, ExtractOptions, ScorerOptions};
use regex::Regex;
use reqwest::Url;
use scraper::{Html, Selector};
use std::{borrow::BorrowMut, io::Cursor, rc::Rc, sync::OnceLock};
use tendril::TendrilSink;

static UNLIKELY_CANDIDATES: OnceLock<Regex> = OnceLock::new();
static NEGATIVE_CANDIDATES: OnceLock<Regex> = OnceLock::new();
static POSITIVE_CANDIDATES: OnceLock<Regex> = OnceLock::new();

pub fn filter_html(html: &str) -> Result<String, BogrepError> {
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .map_err(BogrepError::ReadHtml)?;

    let filtered_dom = filter_dom(dom);
    let handle = SerializableHandle::from(filtered_dom.document);

    let mut bytes = vec![];
    serialize(
        &mut bytes,
        &handle,
        SerializeOpts {
            scripting_enabled: true,
            traversal_scope: html5ever::serialize::TraversalScope::ChildrenOnly(None),
            create_missing_parent: true,
        },
    )
    .map_err(BogrepError::SerializeHtml)?;
    let html = String::from_utf8(bytes)?;
    Ok(html)
}

fn filter_dom(dom: RcDom) -> RcDom {
    let mut cloned_dom = dom;
    let mut filtered_children = Vec::new();

    for child in cloned_dom.document.children.borrow().iter() {
        match &child.data {
            NodeData::Element {
                name,
                attrs: _,
                template_contents: _,
                mathml_annotation_xml_integration_point: _,
            } => {
                if is_filtered_tag(name) {
                    continue;
                }

                filtered_children.push(filter_tree(child.clone()));
            }
            _ => filtered_children.push(filter_tree(child.clone())),
        }
    }

    *(cloned_dom.document.borrow_mut().children.borrow_mut()) = filtered_children;

    cloned_dom
}

fn filter_tree(node: Rc<Node>) -> Rc<Node> {
    let mut filtered_children = Vec::new();

    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Element {
                name,
                attrs: _,
                template_contents: _,
                mathml_annotation_xml_integration_point: _,
            } => {
                if is_filtered_tag(name) {
                    continue;
                }

                filtered_children.push(filter_tree(child.clone()));
            }
            _ => filtered_children.push(filter_tree(child.clone())),
        }
    }

    *node.children.borrow_mut() = filtered_children;

    node
}

fn is_filtered_tag(tag_name: &QualName) -> bool {
    tag_name.local.contains("svg")
        || tag_name.local.contains("img")
        || tag_name.local.contains("video")
        || tag_name.local.contains("script")
}

pub fn convert_to_text(html: &str, bookmark_url: &Url) -> Result<String, BogrepError> {
    let mut cursor = Cursor::new(html);
    let options =  ExtractOptions { parse_options: Default::default(), scorer_options: ScorerOptions {
        unlikely_candidates: UNLIKELY_CANDIDATES.get_or_init(|| {
            Regex::new(
                "combx|community|disqus|extra|foot|header|menu|remark|rss|shoutbox|sidebar|sponsor|ad-break|agegate|pagination|pager|popup|tweet|twitter|ssba",
            )
            .unwrap()
        }),
        negative_candidates: NEGATIVE_CANDIDATES.get_or_init(|| {
            Regex::new("combx|contact|foot|footer|footnote|masthead|media|meta|outbrain|promo|related|scroll|shoutbox|sidebar|sponsor|shopping|tags|tool|widget|form|textfield|uiScale|hidden").unwrap()
        }),
        positive_candidates: POSITIVE_CANDIDATES.get_or_init(|| {
            Regex::new("article|body|content|entry|hentry|main|page|pagination|post|blog|story").unwrap()
        }),
        ..Default::default()
    }};
    let product = extract(&mut cursor, bookmark_url, options).map_err(BogrepError::ConvertHtml)?;
    Ok(product.text)
}

pub fn convert_to_markdown(html: &str) -> String {
    html2md::parse_html(html)
}

pub fn select_underlying(
    html: &str,
    underlying_type: &UnderlyingType,
) -> Result<Option<Url>, BogrepError> {
    debug!("Select underlying for underlying type: {underlying_type:?}");
    trace!("Select underlying in\n{html}");

    match underlying_type {
        UnderlyingType::HackerNews => select_underlying_hackernews(html),
        UnderlyingType::Reddit => select_underlying_reddit(html),
        UnderlyingType::None => Ok(None),
    }
}

fn select_underlying_hackernews(html: &str) -> Result<Option<Url>, BogrepError> {
    let document = Html::parse_document(html);
    let span_selector =
        Selector::parse("span.titleline").map_err(|err| BogrepError::ParseHtml(err.to_string()))?;
    let a_selector = Selector::parse("a").map_err(|err| BogrepError::ParseHtml(err.to_string()))?;

    if let Some(span) = document.select(&span_selector).collect::<Vec<_>>().first() {
        if let Some(a) = span.select(&a_selector).collect::<Vec<_>>().first() {
            if let Some(underlying_link) = a.attr("href") {
                // We are ignoring invalid underlying urls, e.g. for "Ask HN"
                // where no underlying is expected.
                if let Ok(underlying_url) = Url::parse(underlying_link) {
                    debug!("Select underlying: {underlying_link}");
                    return Ok(Some(underlying_url));
                }
            }
        }
    }

    Ok(None)
}

fn select_underlying_reddit(html: &str) -> Result<Option<Url>, BogrepError> {
    let document = Html::parse_document(html);
    let a_selector = Selector::parse("a.styled-outbound-link")
        .map_err(|err| BogrepError::ParseHtml(err.to_string()))?;

    if let Some(a) = document.select(&a_selector).collect::<Vec<_>>().first() {
        if let Some(underlying_link) = a.attr("href") {
            let underlying_url = Url::parse(underlying_link)?;
            debug!("Selected underlying: {underlying_url}");
            return Ok(Some(underlying_url));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_underlying_reddit() {
        let html = r#"
            <html>

            <head>
                <title>title_content</title>
                <meta>
                <script>script_content_1</script>
            </head>

            <body>
                <a href="https://url.com"></a>
                <div class="gfk49d">
                    <a href="https://github.com/quambene/bogrep" class="iek49s styled-outbound-link"
                        rel="noopener nofollow ugc" target="_blank" data-testid="outbound-link">github.com/quambe...<i
                            class="icon icon-external_link_fill k239sk">
                        </i>
                    </a>
                </div>
            </body>

            </html>
        "#;
        let res = select_underlying(html, &UnderlyingType::Reddit);
        assert!(res.is_ok());

        let underlying_url = res.unwrap();
        assert_eq!(
            underlying_url,
            Some(Url::parse("https://github.com/quambene/bogrep").unwrap())
        );
    }

    fn filter_whitespaces(html: impl Into<String>) -> String {
        html.into()
            .chars()
            .filter(|char| !char.is_whitespace())
            .collect::<String>()
    }

    #[test]
    fn test_filter_html() {
        let html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
            <script>script_content_1</script>
        </head>

        <body>
            <img>
            <div>
                <p>paragraph_content_1</p>
                <script>script_content_2</script>
                <img>
                <video></video>
                <svg></svg>
                <div>
                    <p>paragraph_content_2</p>
                    <script>script_content_3</script>
                    <img>
                    <video></video>
                    <svg></svg>
                </div>
            </div>
        </body>

        </html>
        "#;

        let expected_html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
        </head>

        <body>
            <div>
                <p>paragraph_content_1</p>
                <div>
                    <p>paragraph_content_2</p>
                </div>
            </div>
        </body>

        </html>
        "#;

        let filter_html = filter_html(html).unwrap();

        assert_eq!(
            filter_whitespaces(filter_html),
            filter_whitespaces(expected_html)
        );
    }

    #[test]
    fn test_convert_to_text() {
        let html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
        </head>

        <body>
            <div>
                <p>paragraph_content_1</p>
                <div>
                    <p>paragraph_content_2</p>
                </div>
            </div>
        </body>

        </html>
        "#;
        let url = Url::parse("https://example.net").unwrap();
        let res = convert_to_text(html, &url);
        assert!(res.is_ok(), "{}", res.unwrap_err());

        let text = res.unwrap();
        // TODO: fix line breaks
        // TODO: fix missing "paragraph_content_2"
        assert_eq!(text, "title_contentparagraph_content_1");
    }

    #[test]
    fn test_convert_to_markdown() {
        let html = r#"
        <html>

        <head>
            <title>title_content</title>
            <meta>
        </head>

        <body>
            <div>
                <p>paragraph_content_1</p>
                <div>
                    <p>paragraph_content_2</p>
                </div>
            </div>
        </body>

        </html>
        "#;
        let expected_markdown = " title_content\n\nparagraph_content_1\n\nparagraph_content_2";

        let markdown = convert_to_markdown(html);
        // TODO: fix superfluous backslashes
        assert_eq!(markdown.replace('\\', ""), expected_markdown);
    }

    #[test]
    fn test_select_underlying_hackernews() {
        let html = r#"
            <html>

            <head>
                <title>title_content</title>
                <meta>
                <script>script_content_1</script>
            </head>

            <body>
                <td class="title">
                    <span class="titleline">
                        <a href="https://github.com/quambene/bogrep">Bogrep – Grep Your Bookmarks</a>
                        <span class="sitebit comhead"> (<a href="from?site=github.com/quambene">
                                <span class="sitestr">github.com/quambene</span></a>)
                        </span>
                    </span>
                </td>
            </body>

            </html>
        "#;
        let res = select_underlying(html, &UnderlyingType::HackerNews);
        assert!(res.is_ok());

        let underlying_url = res.unwrap();
        assert_eq!(
            underlying_url,
            Some(Url::parse("https://github.com/quambene/bogrep").unwrap())
        );
    }
}
