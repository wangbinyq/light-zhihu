use once_cell::sync::OnceCell;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::{ApiResults, SearchItem, TimelineItem};

static JS_INIITAL_DATA_RE: OnceCell<regex::Regex> = OnceCell::new();

pub fn parse_timeline(value: &Value) -> ApiResults<TimelineItem> {
    let mut results = ApiResults::default();

    results.paging = serde_json::from_value(value["paging"].clone()).unwrap_or_default();

    let data = value["data"].as_array();
    if let Some(data) = data {
        for item in data {
            let str = item["target"].to_string();
            let jd = &mut serde_json::Deserializer::from_str(&str);

            let object: TimelineItem = match serde_path_to_error::deserialize(jd) {
                Ok(object) => object,
                Err(err) => {
                    error!("parse timteline item error: {:?}", err);
                    continue;
                }
            };

            match object.type_.as_str() {
                "answer" | "article" => {
                    results.data.push(object);
                }
                ty => {
                    debug!("find unsupport type: {}", ty);
                }
            }
        }
    }

    results
}

pub fn parse_search(value: &Value) -> ApiResults<SearchItem> {
    let mut results = ApiResults::default();

    results.paging = serde_json::from_value(value["paging"].clone()).unwrap_or_default();

    let data = value["data"].as_array();
    if let Some(data) = data {
        for item in data {
            let ty = &item["type"];
            if ty == "relevant_query" {
                results
                    .data
                    .push(SearchItem::RelevantQuery(item["query_list"].clone()));
                continue;
            } else if ty == "search_result" {
                let str = item["object"].to_string();
                let jd = &mut serde_json::Deserializer::from_str(&str);

                let object: TimelineItem = match serde_path_to_error::deserialize(jd) {
                    Ok(object) => object,
                    Err(err) => {
                        error!("parse search result error: {:?}", err);
                        continue;
                    }
                };

                match object.type_.as_str() {
                    "answer" | "article" => {
                        results.data.push(SearchItem::SearchResult(object));
                    }
                    ty => {
                        debug!("find unsupport type: {}", ty);
                    }
                }
            }
        }
    }

    results
}

pub fn parse_inital_data(html: &str) -> Option<Value> {
    let re = JS_INIITAL_DATA_RE.get_or_init(|| {
        regex::RegexBuilder::new(r#"<script id="js-initialData" type="text/json">(.*?)</script>"#)
            .multi_line(true)
            .build()
            .unwrap()
    });

    let m = re.captures(html)?;

    let json = m.get(1)?.as_str();

    serde_json::from_str(json).ok()?
}
pub fn parse_entity_data<T>(html: &str, entity: &str, id: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    let re = JS_INIITAL_DATA_RE.get_or_init(|| {
        regex::RegexBuilder::new(r#"<script id="js-initialData" type="text/json">(.*?)</script>"#)
            .multi_line(true)
            .build()
            .unwrap()
    });

    let m = re.captures(html)?;

    let json = m.get(1)?.as_str();

    let json: Value = serde_json::from_str(json).ok()?;

    let value = &json["initialState"]["entities"][entity][id];

    serde_json::from_value(value.clone()).ok()
}
