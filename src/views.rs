use std::collections::HashMap;

use lol_html::{element, html_content::ContentType};
use maud::{Markup, PreEscaped};
use once_cell::sync::OnceCell;
use regex::Regex;
use serde_json::Value;

use crate::types::{Attachment, Comment, Question, TimelineItem};

static EMOJI: OnceCell<HashMap<String, String>> = OnceCell::new();
static EMOJI_RE: OnceCell<regex::Regex> = OnceCell::new();
static HREF_RE: OnceCell<Regex> = OnceCell::new();

macro_rules! css {
    ($css:expr, $class:expr) => {
        element!($css, |el| {
            let class = if let Some(cls) = el.get_attribute("class") {
                format!("{} {}", cls, $class)
            } else {
                $class.to_string()
            };
            el.set_attribute("class", &class).ok();
            Ok(())
        })
    };
}

fn render_html(html: &str) -> PreEscaped<String> {
    let emoji =
        EMOJI.get_or_init(|| serde_json::from_str(include_str!("../data/emoji.json")).unwrap());
    let emoji_re = EMOJI_RE.get_or_init(|| {
        let re: Vec<String> = emoji.keys().map(|t| format!("(\\[{t}\\])")).collect();
        let re = re.join("|");

        regex::Regex::new(&re).unwrap()
    });
    let href_re = HREF_RE.get_or_init(|| Regex::new(r#"https?://(.*?).zhihu.com/(.*)"#).unwrap());

    let html = emoji_re.replace_all(html, |caps: &regex::Captures| {
        let key = caps[0].trim_matches('[').trim_matches(']');
        if let Some(src) = emoji.get(key) {
            format!(r#"<img class="w-5 h-5 !my-0 mx-1 inline-block align-text-bottom" src="{src}"></img>"#)
        } else {
            key.to_string()
        }
    });

    let html = lol_html::rewrite_str(
        &html,
        lol_html::Settings {
            element_content_handlers: vec![
                css!("*", "mt-2 break-all"),
                css!(".invisible", "hidden"),
                css!("em", "text-red-400 not-italic"),
                css!("pre", "p-2 bg-gray-200 overflow-auto"),
                css!("blockquote", "pl-4 border-l-4 text-gray-500"),
                css!("hr", "!my-12 w-1/2 mx-auto"),
                css!("ol", "list-decimal pl-4"),
                css!("h2", "text-lg font-bold"),
                css!("h3", "text-base"),
                css!("figure", "!my-6 flex flex-col-reverse"),
                css!("figcaption", "text-sm text-gray-400 text-center"),
                element!("figure > noscript", |el| {
                    el.remove();
                    Ok(())
                }),
                element!("a[href]", |a| {
                    let href = a.get_attribute("href").unwrap_or_default();
                    let href = href_re.replace(&href, |caps: &regex::Captures| {
                        let host = &caps[1];
                        let path = &caps[2];
                        if host == "link" {
                            a.set_attribute("class", "border-b-1 border-gray-400").ok();
                            urlencoding::decode(
                                &path.replace("?target=", "")
                            ).unwrap().to_string()
                        } else {
                            a.set_attribute("class", "text-blue-600 underline").ok();
                            format!("/{path}")
                        }
                    });

                    a.set_attribute("href", &href).ok();
                    Ok(())
                }),
                element!("figure img", |img| {
                    img.set_attribute("loading", "lazy").ok();
                    img.set_attribute("class", "mx-auto").ok();

                    let actual_src = img.get_attribute("data-actualsrc");
                    let original_src = img
                        .get_attribute("data-originalsrc")
                        .or_else(|| img.get_attribute("data-original"));

                    let img_src = actual_src.or(original_src.clone()).unwrap_or_default();
                    img.set_attribute("src", &img_src).ok();

                    img.before(
                        &format!(
                            "<a target=\"_blank\" href=\"{}\">",
                            original_src.unwrap_or(img_src)
                        ),
                        ContentType::Html,
                    );
                    img.after("</a>", ContentType::Html);

                    Ok(())
                }),
                element!(
                    r#"a[class="comment_sticker"], a[class="comment_img"]"#,
                    |a| {
                        let aclass = a.get_attribute("class").unwrap_or_default();
                        let src = a.get_attribute("href").unwrap_or_default();

                        let img = format!(r#"<img loading="lazy" src={src} #width# />"#);
                        let img = if aclass == "comment_sticker" {
                            img.replace("#width#", "width=\"80\"")
                        } else {
                            img.replace("#width#", "max-width=\"100%\"")
                        };

                        a.replace(&img, ContentType::Html);

                        Ok(())
                    }
                ),
                element!(r#"a[data-draft-type="link-card"]"#, |a| {
                    a.set_attribute("target", "_blank").ok();
                    a.set_attribute("class", "flex w-96 max-w-full p-4 bg-gray-100 rounded my-4 mx-auto no-underline cursor-pointer line-clamp-2").ok();

                    a.prepend(r#"<div class="mr-auto text-sm text-gray-800"> "#, ContentType::Html);

                    a.append(
                        &if let Some(img_src) = a.get_attribute("data-image") {
                            format!(r#"</div><img class="w-16 h-16 object-cover rounded ml-1" loading="lazy" src="{}" />"#, img_src)
                        } else {
                            "</div>".to_string()
                        },
                        ContentType::Html,
                    );
                    Ok(())
                }),
            ],
            ..Default::default()
        },
    )
    .unwrap();
    PreEscaped(html)
}

fn attachment(attachment: &Option<Attachment>) -> Markup {
    if let Some(attachment) = attachment {
        if attachment.type_ == "video" {
            let video = &attachment.video;
            let info = video.video_info.clone();
            let sd_url = info.playlist.sd.or(info.playlist.ld).map(|v| v.url);
            let hd_url = info.playlist.hd.map(|v| v.url).or(sd_url.clone());
            return html! {
                div  {
                    video controls loading="lazy" poster=(info.thumbnail) src=[sd_url];
                    a href=[hd_url] {
                        div class="p-2 bg-gray-200 rounded-b-sm" {
                            h3 class="text-sm" { (video.title) }
                            div class="text-gray-400 text-xs" {
                                (format!("{} 次播放", video.play_count))
                            }
                        }
                    }
                }
            };
        }
        debug!("unsupport answer attachment: {}", attachment.type_);
    }
    html! {}
}

pub fn answer(answer: &TimelineItem, show_all: bool) -> Markup {
    let content = answer
        .content
        .as_ref()
        .or(answer.excerpt.as_ref())
        .map(String::as_str)
        .unwrap_or_default();

    html! {
        div class="p-4 pb-0 mb-2 bg-white"
            v-scope=(format!("{{show_all: {show_all} }}")) {
            @if let Some(author) = &answer.author {
                div class="flex items-center" {
                    img class="mr-2 w-8 h-8 object-cover rounded-sm" src=(author.avatar_url) alt=(author.name);
                    div {
                        div class="text-sm" { (author.name) }
                        div class="text-xs text-gray-600" { (author.headline) }
                    }
                }
            }

            div class="relative" v-bind:class="show_all ? '' : 'line-clamp-5 max-h-40'" {
                (render_html(&content))

                template v-if="!show_all" {
                    div v-on:click="show_all = true"
                        class="absolute cursor-pointer bg-gradient-to-t from-white to-transparent h-full w-full top-0 flex justify-center items-end" {
                        div class="text-gray-500" { "展开阅读全文" }
                    }
                }
            }

            (attachment(&answer.attachment))

            @if let Some(updated_time) = answer.updated_time {
                div class="text-gray-400 mt-2 text-sm" {
                    "编辑于 " (time(updated_time))
                }
            }

            div class="flex text-sm text-gray-500 bg-white p-4 -mx-4"
                v-bind:class="show_all ? 'sticky bottom-0' : ''" {
                @if answer.voteup_count > 0 {
                    span class="mr-2" {
                        (answer.voteup_count) " 赞同"
                    }
                }
                @if answer.comment_count > 0 {
                    a href=(format!("/comment/root/{}{}", answer.id, if answer.type_ == "article" {"?type=articles"} else {""} )) {
                        span class="mr-2" {
                            (answer.comment_count) " 条评论"
                        }
                    }
                }

                template v-if="show_all" {
                    button v-on:click="show_all=false" class="ml-auto" { "收起" }
                }
            }
        }
    }
}

pub fn timeline(item: &TimelineItem) -> Markup {
    let title = &item
        .title
        .as_ref()
        .or_else(|| item.question.as_ref().map(|q| &q.title))
        .map(String::as_str)
        .unwrap_or_default();
    let content = item
        .excerpt
        .as_ref()
        .or(item.content.as_ref())
        .map(String::as_str)
        .unwrap_or_default();
    let thumbnail = item.thumbnail.as_ref().map(|s| s.as_str());
    let thumbnail = item
        .ext
        .pointer("/thumbnail_info/thumbnails/0/url")
        .and_then(|v| v.as_str())
        .or(thumbnail);
    let (body_href, title_href) = if item.type_ == "answer" {
        let question = item.question.as_ref().unwrap();

        (
            format!("/question/{}/answer/{}", question.id, item.id),
            format!("/question/{}", question.id),
        )
    } else {
        let href = format!("/p/{}", item.id);
        (href.clone(), href)
    };

    html! {
        div class="p-4 mb-2 bg-white" {
            a href=(title_href) {
                h3 class="text-base font-bold mb-1" {
                    (render_html(title))
                }
            }
            a class="flex" href=(body_href) {
                div class="mr-auto" {
                    div class="text-sm line-clamp-3" {
                        @if let Some(author) = item.author.as_ref() {
                            span class="font-bold" {
                                (PreEscaped(&author.name))":"
                            }
                        }
                        span {
                            (render_html(&content))
                        }
                    }
                    div class="mt-2 text-xs text-gray-500" {
                        @if item.voteup_count > 0{
                            span class="mr-2" {
                                (item.voteup_count) " 赞同"
                            }
                        }
                        @if  item.comment_count > 0{
                            span class="mr-2" {
                                (item.comment_count) " 条评论"
                            }
                        }
                        @if let Some(created_time) = item.created_time {
                            span class="mx-1" {
                                (time(created_time))
                            }
                        }
                    }
                }
                @if let Some(thumbnail) = thumbnail {
                    img class="flex-grow-0 ml-2 w-auto max-w-[25%] h-16 object-cover rounded" src=(thumbnail);
                }
            }
        }
    }
}

pub fn question(question: &Question, show_all: bool) -> Markup {
    let has_detail = !question.detail.is_empty();

    html! {
        div class="p-4 pb-0 my-1 bg-white"
            v-scope=(format!("{{show_all: {show_all} }}")) {
            h3 class="text-base font-bold text-lg" { (question.title )}

            @if has_detail {
                div class="relative text-sm text-gray-600" v-bind:class="show_all ? '' : 'line-clamp-5 max-h-40'" {
                    (render_html(&question.detail))

                    template v-if="!show_all" {
                        div v-on:click="show_all = true"
                            class="absolute cursor-pointer bg-gradient-to-t from-white to-transparent h-full w-full top-0 flex justify-center items-end" {
                            div class="text-gray-500" { "展开阅读全文" }
                        }
                    }
                }
            }

            div class="flex text-sm text-gray-400 bg-white p-4 -mx-4"
                v-bind:class="show_all ? 'sticky bottom-0' : ''" {
                @if question.answer_count > 0 {
                    span class="mr-2" { (question.answer_count) " 回答" }
                }
                @if question.comment_count > 0 {
                    a href=(format!("/comment/root/{}?type=questions", question.id)) {
                        span class="mr-2" { (question.comment_count) " 条评论" }
                    }
                }
                @if question.voteup_count > 0 {
                    span class="mr-2" { (question.voteup_count) " 好问题" }
                }

                @if has_detail {
                    template v-if="show_all" {
                        button v-on:click="show_all=false" class="ml-auto" { "收起" }
                    }
                }
            }
        }
    }
}

pub fn comment(comment: &Comment, show_more: bool) -> Markup {
    html! {
        div class="flex items-start" {
            img class="flex-shrink-0 w-8 h-8 rounded-sm object-cover mr-2" src=(comment.author.avatar_url) alt=(comment.author.name);
            div class="flex-grow" {
                div class="flex items-center font-bold" {
                    (comment.author.name)
                    @for tag in &comment.author_tag {
                        span class="font-normal text-gray-400 text-xs mx-1 border-1 px-1 rounded-sm" {
                            (tag.text)
                        }
                    }
                    @if let Some(reply_to_author) = comment.reply_to_author.as_ref() {
                        " > " (reply_to_author.name)
                    }
                }
                div {
                    (render_html(&comment.content))
                }
                div class="flex mt-2 text-xs text-gray-400" {
                    span class="mr-auto" {
                        (time(comment.created_time))
                    }

                    @if comment.like_count > 0 {
                        span class="ml-2" {
                            (comment.like_count) " 赞"
                        }
                    }

                    @if comment.dislike_count > 0 {
                        span class="ml-2" {
                            (comment.dislike_count) " 踩"
                        }
                    }
                }

                @for child in &comment.child_comments {
                    div class="mt-2" {
                        (self::comment(child, true))
                    }
                }

                @if show_more && comment.child_comment_count > 2 {
                    div class="mt-2" {
                        a href=(format!("/comment/child/{}", comment.id)) {
                            span class="rounded cursor-pointer py-1 px-2 text-gray-500 bg-gray-200" {
                                "查看全部 " (comment.child_comment_count) " 条回复"
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn relevant_query(querys: &Value) -> Markup {
    if let Some(query_list) = querys.as_array() {
        html! {
            div class="p-4 mb-2 bg-white" {
                h3 class="text-base font-bold" { "相关搜索" }
                div class="mt-2 grid grid-cols-2" {
                    @for q in query_list {
                        @let q = q["query"].as_str().unwrap();
                        a class="p-2 mr-1 mb-1 bg-gray-200 rounded" href=(format!("/search?q={}", q))  {
                            span {
                                (q)
                            }
                        }
                    }
                }
            }
        }
    } else {
        html! {}
    }
}

pub fn time(seconds: i64) -> String {
    let time = chrono::NaiveDateTime::from_timestamp_opt(seconds, 0).unwrap();
    time.format("%Y-%m-%d").to_string()
}
