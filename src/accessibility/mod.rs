use crate::errors::{Error, Result};
use quick_xml::{Reader, events::Event};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: usize,
    pub parent: Option<usize>,
    pub text: String,
    pub description: String,
    pub resource_id: String,
    pub class: String,
    pub package: String,
    pub bounds: Option<[i32; 4]>,
    pub clickable: bool,
    pub enabled: bool,
    pub password: bool,
}

pub fn parse(output: &str) -> Result<Vec<Node>> {
    let start = output.find("<hierarchy").ok_or_else(|| {
        Error::new(
            "ui_unavailable",
            "Android did not return an accessibility hierarchy; unlock the device and try again",
        )
    })?;
    let end = output
        .rfind("</hierarchy>")
        .ok_or_else(|| Error::new("ui_unavailable", "Incomplete accessibility hierarchy"))?
        + "</hierarchy>".len();
    let mut reader = Reader::from_str(&output[start..end]);
    reader.config_mut().expand_empty_elements = true;
    let mut nodes = Vec::new();
    let mut parents = Vec::new();
    loop {
        match reader
            .read_event()
            .map_err(|e| Error::new("invalid_ui_tree", e.to_string()))?
        {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"node" => {
                if nodes.len() >= 10000 {
                    return Err(Error::new(
                        "output_limit",
                        "UI hierarchy exceeds 10000 nodes",
                    ));
                }
                let mut node = Node {
                    id: nodes.len(),
                    parent: parents.last().copied(),
                    text: String::new(),
                    description: String::new(),
                    resource_id: String::new(),
                    class: String::new(),
                    package: String::new(),
                    bounds: None,
                    clickable: false,
                    enabled: false,
                    password: false,
                };
                for attr in e.attributes() {
                    let attr = attr.map_err(|e| Error::new("invalid_ui_tree", e.to_string()))?;
                    let value = attr
                        .decode_and_unescape_value(reader.decoder())
                        .map_err(|e| Error::new("invalid_ui_tree", e.to_string()))?
                        .into_owned();
                    match attr.key.as_ref() {
                        b"text" => node.text = value,
                        b"content-desc" => node.description = value,
                        b"resource-id" => node.resource_id = value,
                        b"class" => node.class = value,
                        b"package" => node.package = value,
                        b"clickable" => node.clickable = value == "true",
                        b"enabled" => node.enabled = value == "true",
                        b"password" => node.password = value == "true",
                        b"bounds" => {
                            let numbers: std::result::Result<Vec<i32>, _> = value
                                .split(['[', ']', ','])
                                .filter(|s| !s.is_empty())
                                .map(str::parse)
                                .collect();
                            node.bounds = numbers.ok().and_then(|n| n.try_into().ok());
                        }
                        _ => {}
                    }
                }
                if node.password {
                    node.text.clear();
                    node.description.clear();
                }
                // Expanding empty elements makes the parent stack uniform.
                parents.push(node.id);
                nodes.push(node);
            }
            Event::End(e) if e.name().as_ref() == b"node" => {
                parents.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(nodes)
}
