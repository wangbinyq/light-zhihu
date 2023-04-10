use std::str::FromStr;

use axum::{
    extract::{Path, Query},
    response::{AppendHeaders, IntoResponse, Redirect, Response},
};
use http::{header, StatusCode, Uri};
use maud::{Markup, PreEscaped};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::Value;

use crate::{
    parser,
    types::{Comment, Error, Paging, Question, SearchItem, TimelineItem},
    views,
};

macro_rules! headers {
	{ $($key:expr => $value:expr),+ } => {
		{
			let mut m = http::HeaderMap::new();
			$(
				if let Ok(val) = http::header::HeaderValue::from_str($value) {
					m.insert($key, val);
				}
			)+
			m
		}
	 };
}
static CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .default_headers(headers! {
            "cookie" => &(std::env::var("ZHIHU_COOKIE").unwrap_or_else(|_|format!("d_c0=AHAYQW4aKRaPTkeCwzmIwGqf-AOazW4-dpM=|1673495514")))
        })
        .build()
        .unwrap()
});

pub async fn index() -> impl IntoResponse {
    Redirect::to("/recommend")
}

pub async fn recommend() -> Result<Markup, Error> {
    let response = CLIENT
        .get("https://www.zhihu.com/api/v3/feed/topstory/recommend")
        .send()
        .await?
        .json()
        .await?;

    let results = parser::parse_timeline(&response);

    Ok(layout(
        html! {
            h2 class="p-4 mb-2 bg-white text-base" {
                "推荐"
                a class="ml-2 text-sm underline text-gray-500" href="" { "刷新" }
            }
            @for item in &results.data {
                (views::timeline(item))
            }
            a href="" {
                div class="p-4 mb-2 bg-white mt-2 text-center font-base" { "刷新" }
            }
        },
        Some("推荐"),
    ))
}

pub async fn question(
    qid: Path<(String,)>,
    query: Query<Value>,
    uri: Uri,
) -> Result<Markup, Error> {
    let qid = qid.0 .0;
    let mut query = query.0;

    if !query["include"].is_string() {
        query["include"] = json!("data[*].is_normal,admin_closed_comment,reward_info,is_collapsed,annotation_action,annotation_detail,collapse_reason,is_sticky,collapsed_by,suggest_edit,comment_count,can_comment,content,attachment,voteup_count,reshipment_settings,comment_permission,created_time,updated_time,review_info,relevant_info,question,excerpt,is_labeled,paid_info,paid_info_content,reaction_instruction,relationship.is_authorized,is_author,voting,is_thanked,is_nothelp,is_recognized;data[*].mark_infos[*].url;data[*].author.follower_count,vip_info,badge[*].topics;data[*].settings.table_of_content.enable");
    }

    let html = CLIENT
        .get(format!("https://www.zhihu.com/question/{}", qid))
        .send()
        .await?
        .text()
        .await?;

    let results: Value = CLIENT
        .get(format!(
            "https://www.zhihu.com/api/v4/questions/{}/feeds",
            qid
        ))
        .query(&query)
        .send()
        .await?
        .json()
        .await?;

    let question: Question =
        parser::parse_entity_data(&html, "questions", &qid).unwrap_or_else(|| {
            debug!("unable to find question of {}", qid);
            Default::default()
        });
    let results = parser::parse_timeline(&results);

    Ok(layout(
        html! {
            (views::question(&question, false))

            (render_prev(&results.paging, &uri.path(), html! {
                div class="p-4 mb-2 bg-white text-center font-bold" { "查看上一页" }
            }))

            @for item in &results.data {
                (views::answer(item, true))
            }

            (render_next(&results.paging, &uri.path(), html! {
                div class="p-4 my-2 bg-white text-center font-base" { "查看更多 " (question.answer_count) " 个答案" }
            }))

        },
        Some(&format!("问题: {}", question.title)),
    ))
}

pub async fn answer(p: Path<(String, String)>, query: Query<Value>) -> Result<Markup, Error> {
    let (qid, aid) = p.0;
    let query = query.0;

    let html = CLIENT
        .get(format!(
            "https://www.zhihu.com/question/{}/answer/{}",
            qid, aid
        ))
        .query(&query)
        .send()
        .await?
        .text()
        .await?;

    let initial_data = parser::parse_inital_data(&html).unwrap_or_default();
    let que = initial_data["initialState"]["entities"]["questions"][&qid].clone();
    let answer = initial_data["initialState"]["entities"]["answers"][&aid].clone();

    let que: Question = serde_json::from_value(que)?;
    let answer: TimelineItem = serde_json::from_value(answer)?;

    let q_href = format!("/question/{}", que.id);
    let check_more = html! {
        a href=(q_href) {
            div class="p-4 mb-2 bg-white text-center font-base" {
                "查看全部 "
                (que.answer_count)
                 " 个回答"
            }
        }
    };

    Ok(layout(
        html! {
            (views::question(&que, true))
            (check_more)
            (views::answer(&answer, true))
            (check_more)
        },
        Some(&format!(
            "问题: {:?}, {:?} 的回答",
            que.title,
            answer.author.map(|a| a.name),
        )),
    ))
}

pub async fn article(aid: Path<(String,)>) -> Result<Markup, Error> {
    let aid = aid.0 .0;

    let html = CLIENT
        .get(format!("https://zhuanlan.zhihu.com/p/{}", aid))
        .send()
        .await?
        .text()
        .await?;

    let mut article: TimelineItem = parser::parse_entity_data(&html, "articles", &aid)
        .unwrap_or_else(|| {
            debug!("unable to find aritlce of {}", aid);
            Default::default()
        });

    let author = article.author.take().unwrap();

    let title = article.title.clone().unwrap_or_default();

    Ok(layout(
        html! {
            div class="p-4 mb-2 bg-white" {
                @if !article.image_url.is_empty() {
                    img class="w-full mb-4" src=(article.image_url) alt=(title);
                }
                h2 class="text-xl font-bold" {(title)}
                div class="flex items-center mt-4" {
                    img class="mr-2 w-8 h-8 object-cover rounded-sm" src=(author.avatar_url) alt=(author.name);
                    div {
                        div { (author.name) }
                        div class="text-sm text-gray-600" {
                            (PreEscaped(author.headline))
                        }
                    }
                }
                div class="text-gray-500 mt-4 text-sm" {
                    @if article.voteup_count > 0{
                        span class="mr-2" {
                            (article.voteup_count) " 赞同"
                        }
                    }
                    @if  article.comment_count > 0 {
                        a href=(format!("/comment/root/{}?type=articles", article.id)) {
                            span class="mr-2" {
                                (article.comment_count) " 条评论"
                            }
                        }
                    }
                    @if let Some(updated_time) = article.updated_time {
                        span class="mx-1" {
                            "编辑于 " (views::time(updated_time))
                        }
                    }
                }
            }
            (views::answer(&article, true))
        },
        Some(&format!("专栏文章: {}", title)),
    ))
}

pub async fn root_comment(
    aid: Path<(String,)>,
    query: Query<Value>,
    uri: Uri,
) -> Result<Markup, Error> {
    let aid = aid.0 .0;
    let query = query.0;

    let type_ = query["type"].as_str().unwrap_or("answers");

    let results: Value = CLIENT
        .get(format!(
            "https://www.zhihu.com/api/v4/comment_v5/{}/{}/root_comment",
            type_, aid
        ))
        .query(&query)
        .send()
        .await?
        .json()
        .await?;

    let paging: Paging = serde_json::from_value(results["paging"].clone())?;
    let data: Vec<Comment> = serde_json::from_value(results["data"].clone())?;

    Ok(layout(
        html! {
            (render_prev(&paging, &uri.path(), html! {
                div class="p-4 mb-2 bg-white text-center font-bold" { "查看上一页" }
            }))

            ul {
                @for comment in &data {
                    li class="p-4 mb-2 bg-white" {
                        (views::comment(comment, true))
                    }
                }
            }

            (render_next(&paging, &uri.path(), html! {
                div class="p-4 my-2 bg-white text-center font-base" { "查看下一页" }
            }))
        },
        Some("评论"),
    ))
}

pub async fn child_comment(
    cid: Path<(String,)>,
    query: Query<Value>,
    uri: Uri,
) -> Result<Markup, Error> {
    let cid = cid.0 .0;
    let query = query.0;

    let results: Value = CLIENT
        .get(format!(
            "https://www.zhihu.com/api/v4/comment_v5/comment/{}/child_comment",
            cid
        ))
        .query(&query)
        .send()
        .await?
        .json()
        .await?;

    let paging: Paging = serde_json::from_value(results["paging"].clone())?;
    let data: Vec<Comment> = serde_json::from_value(results["data"].clone())?;
    let root: Comment = serde_json::from_value(results["root"].clone())?;

    Ok(layout(
        html! {
            div class="p-4 mb-2 bg-white" {
                (views::comment(&root, false))
            }



            ul class="p-4" {
                (render_prev(&paging, &uri.path(), html! {
                    div class="p-4 mb-2 bg-white text-center font-bold" { "查看上一页" }
                }))

                @for comment in &data {
                    li class="p-4 mb-2 bg-white" {
                        (views::comment(comment, true))
                    }
                }

                (render_next(&paging, &uri.path(), html! {
                    div class="p-4 my-2 bg-white text-center font-base" { "查看下一页" }
                }))
            }
        },
        Some("评论"),
    ))
}

pub async fn search(query: Query<Value>) -> Result<Markup, Error> {
    let query = query.0;
    let q = query["q"].as_str().unwrap_or_default();

    let results = if !q.is_empty() {
        let response: Value = CLIENT
            .get("https://www.zhihu.com/api/v4/search_v3")
            .query(&query)
            .header("x-zse-93", "101_3_3.0")
            .header("x-zse-96", "2.0_")
            .send()
            .await?
            .json()
            .await?;
        parser::parse_search(&response)
    } else {
        Default::default()
    };

    Ok(layout(
        html! {
            div class="p-4 mb-2 bg-white" {
                form class="flex mb-0" action="/search" {
                    @for (name, val) in [("type", "content"), ("limit", "20"), ("show_all_topics", "1")] {
                        input class="hidden" type="text" name=(name) value=(val);
                    }
                    input placeholder="请输入搜索内容" class="h-8 border border-gray-200 px-1" type="search" name="q" value=(q) autocomplete="off";
                    button type="submit" class="bg-gray-200 h-8 px-4 ml-2 rounded-sm" { "搜索" }
                }
            }

            @if results.data.is_empty() {
                div class="p-4 mb-2 bg-white text-center font-bold" { "暂无数据"}
            } @else {
                ul {
                    (render_prev(&results.paging, "/search", html!{
                        div class="p-4 mb-2 bg-white text-center font-bold" { "查看上一页" }
                    }))
                    @for item in &results.data {
                        @match item {
                            SearchItem::Unknown => {}
                            SearchItem::RelevantQuery(q) => {
                                (views::relevant_query(q))
                            }
                            SearchItem::SearchResult(r) => {
                                (views::timeline(r))
                            }
                        }
                    }
                    (render_next(&results.paging, "/search", html!{
                        div class="p-4 mb-2 bg-white text-center font-bold" { "查看更多" }
                    }))
                }
            }

        },
        Some("搜索"),
    ))
}

pub fn error(status: StatusCode, err: &Error) -> Markup {
    layout(
        html! {
            div class="p-4 mb-2 bg-white" {
                p { (format!("{}: {}", status, err)) }
            }
        },
        Some("Error"),
    )
}

pub async fn default(uri: Uri) -> Response {
    let path = uri.path();

    if path.starts_with("/favicon") {
        (
            StatusCode::OK,
            AppendHeaders([
                (header::CONTENT_TYPE, "image/png"),
                (header::CACHE_CONTROL, "max-age: 31536000"),
            ]),
            include_bytes!("../public/favicon.png"),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            layout(
                html! {
                    div class="p-4 mb-2 bg-white" {
                        p { (format!("404 not found: {}", path)) }
                    }
                },
                Some("404 Not Found"),
            ),
        )
            .into_response()
    }
}

fn layout(body: Markup, title: Option<&str>) -> Markup {
    let title = title.unwrap_or("Light Zhihu - 一个轻量知乎客户端");

    html! {
        head {
            title { (title) }
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="shortcut icon" href="/favicon.png" type="image/png";
            script src="https://unpkg.com/petite-vue" defer init {}
            script src="https://cdn.twind.style" crossorigin {}
            script {
                (PreEscaped(r#"
                twind.install({
                    hash: false,
                    presets: [
                      {
                        rules: [
                          ['line-clamp-none', { '-webkit-line-clamp': 'unset' }],
                          [
                            'line-clamp-(\\d+)',
                            ({ 1: _ }) => lineClamp(_),
                          ],
                        ]
                      }
                    ]
                  })
                  
                  function lineClamp(lines) {
                    return {
                      overflow: 'hidden',
                      display: '-webkit-box',
                      '-webkit-box-orient': 'vertical',
                      '-webkit-line-clamp': `${lines}`,
                    }
                  }
                "#))
            }
        }

        body class="min-h-screen text-base bg-gray-100" {
            header class="bg-white mb-2 px-2 py-4 flex justify-center" {
                nav class="flex flex-grow max-w-2xl" {
                    a class="font-bold text-gray-500 mr-auto" href="/" { "Light Zhihu" }
                    a class="ml-2 underline" href="/recommend" { "推荐" }
                    a class="ml-2 underline" href="/search" { "搜索" }
                }
            }
        }

        main class="max-w-2xl mx-auto" {
            (body)
        }
    }
}

fn paging_href(href: &str, path: &str) -> String {
    if let Ok(url) = Uri::from_str(href) {
        let query = url.query().unwrap_or_default();

        format!("{}?{}", path, query)
    } else {
        path.to_string()
    }
}

fn render_prev(paging: &Paging, path: &str, child: Markup) -> Markup {
    if let Some(is_start) = paging.is_start {
        if !is_start {
            return html! {
                a href=(paging_href(&paging.previous, path)) {
                    (child)
                }
            };
        }
    }
    html! {}
}

fn render_next(paging: &Paging, path: &str, child: Markup) -> Markup {
    if let Some(is_end) = paging.is_end {
        if !is_end {
            return html! {
                a href=(paging_href(&paging.next, path)) {
                    (child)
                }
            };
        }
    }
    html! {}
}
