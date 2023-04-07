use axum::response::{IntoResponse, Response};
use http::StatusCode;
use serde::Deserialize;
use serde_json::Value;
use serde_this_or_that::as_string;
use thiserror::Error;

use crate::routes;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        error!("{:?}", self);
        routes::error(StatusCode::INTERNAL_SERVER_ERROR, &self).into_response()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Paging {
    pub is_end: Option<bool>,
    pub is_start: Option<bool>,
    pub next: String,
    pub previous: String,
    pub totals: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiResults<T> {
    pub data: Vec<T>,
    pub paging: Paging,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Question {
    #[serde(deserialize_with = "as_string")]
    pub id: String,
    #[serde(alias = "name")]
    pub title: String,
    pub detail: String,

    #[serde(alias = "answerCount")]
    pub answer_count: u64,
    #[serde(alias = "commentCount")]
    pub comment_count: u64,
    #[serde(alias = "voteupCount")]
    pub voteup_count: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum SearchItem {
    #[default]
    Unknown,
    RelevantQuery(Value),
    SearchResult(TimelineItem),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TimelineItem {
    #[serde(deserialize_with = "as_string")]
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,

    #[serde(alias = "imageUrl")]
    pub image_url: String,
    pub thumbnail: Option<String>,
    pub excerpt: Option<String>,
    pub content: Option<String>,
    pub title: Option<String>,
    pub question: Option<Question>,
    pub author: Option<Author>,
    pub attachment: Option<Attachment>,
    #[serde(alias = "createdTime")]
    pub created_time: Option<i64>,
    #[serde(alias = "updatedTime", alias = "updated")]
    pub updated_time: Option<i64>,
    #[serde(alias = "voteupCount")]
    pub voteup_count: u64,
    #[serde(alias = "commentCount")]
    pub comment_count: u64,

    #[serde(flatten)]
    pub ext: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Author {
    #[serde(alias = "avatarUrl")]
    pub avatar_url: String,
    pub name: String,
    pub headline: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Attachment {
    #[serde(rename = "type")]
    pub type_: String,
    pub video: AttachmentVideo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AttachmentVideo {
    pub title: String,
    #[serde(alias = "playCount")]
    pub play_count: u32,
    #[serde(alias = "videoInfo")]
    pub video_info: VideoInfo,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoInfo {
    pub thumbnail: String,
    pub playlist: VideoUrls,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoUrls {
    pub ld: Option<VideoUrl>,
    pub sd: Option<VideoUrl>,
    pub hd: Option<VideoUrl>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VideoUrl {
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Comment {
    pub id: String,
    pub author: Author,
    pub reply_to_author: Option<Author>,
    pub content: String,
    #[serde(alias = "createdTime")]
    pub created_time: i64,
    pub like_count: u32,
    pub dislike_count: u32,
    pub child_comments: Vec<Comment>,
    pub child_comment_count: u32,
}
