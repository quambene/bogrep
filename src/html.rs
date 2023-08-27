use html5ever::{
    parse_document,
    rcdom::{Node, NodeData, RcDom},
    serialize::{serialize, SerializeOpts},
    tendril::TendrilSink,
    ParseOpts, QualName,
};
use std::{borrow::BorrowMut, rc::Rc};

pub fn filter_html(html: &str) -> Result<String, anyhow::Error> {
    let dom = parse_document(RcDom::default(), ParseOpts::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())?;

    let filtered_dom = filter_dom(dom);

    let mut bytes = vec![];
    serialize(
        &mut bytes,
        &filtered_dom.document,
        SerializeOpts {
            scripting_enabled: true,
            traversal_scope: html5ever::serialize::TraversalScope::ChildrenOnly(None),
            create_missing_parent: true,
        },
    )?;
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
                if is_filtered_tag(&name) {
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
                if is_filtered_tag(&name) {
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
    if tag_name.local.contains("svg")
        || tag_name.local.contains("img")
        || tag_name.local.contains("video")
        || tag_name.local.contains("script")
    {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            filter_html
                .chars()
                .filter(|char| !char.is_whitespace())
                .collect::<String>(),
            expected_html
                .chars()
                .filter(|char| !char.is_whitespace())
                .collect::<String>()
        );
    }
}
