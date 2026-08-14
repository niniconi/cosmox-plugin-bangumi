use url::{Position, Url};
use wasi::{
    http::{
        outgoing_handler,
        types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme},
    },
    io::streams::StreamError,
};

use serde::{Deserialize, Serialize};

/// Image URLs of a subject at several sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectImages {
    pub small: Option<String>,
    pub medium: Option<String>,
    pub large: Option<String>,
    pub common: Option<String>,
    pub grid: Option<String>,
}

/// One key/value row of a subject's `infobox` (staff, airing info, ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfoboxItem {
    pub key: String,
    pub value: serde_json::Value,
}

/// Per-rating-bucket counts (`1..=10`), flattened onto the struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingCount {
    #[serde(flatten)]
    pub counts: std::collections::HashMap<String, u32>,
}

/// Rating summary of a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rating {
    pub score: f64,
    pub total: u32,
    pub rank: u32,
    pub count: RatingCount,
}

/// Users' collection state of a subject (wish / collect / doing / ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub wish: u32,
    pub collect: u32,
    pub doing: u32,
    pub dropped: u32,
    pub on_hold: u32,
}

/// A user tag attached to a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub count: u32,
    pub total_count: u32,
}

/// An anime subject (series or movie) as returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subject {
    pub id: u64,
    #[serde(rename = "type")]
    pub subject_type: u8,
    pub name: String,
    pub name_cn: Option<String>,
    pub summary: Option<String>,
    pub date: Option<String>,
    pub images: SubjectImages,
    pub image: Option<String>,
    pub infobox: Vec<InfoboxItem>,
    pub tags: Vec<Tag>,
    pub rating: Option<Rating>,
    pub collection: Option<Collection>,
    pub eps: Option<u32>,
    pub volumes: Option<u32>,
    pub series: Option<bool>,
    pub locked: Option<bool>,
    pub platform: Option<String>,
    pub nsfw: Option<bool>,
    pub meta_tags: Option<Vec<String>>,
    pub total_episodes: Option<u32>,
}

/// Paged result of a subject search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedSubject {
    pub data: Vec<Subject>,
    pub limit: u32,
    pub offset: u32,
    pub total: u64,
}

/// A single episode of a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: u64,
    #[serde(rename = "type")]
    pub ep_type: u8,
    pub name: String,
    pub name_cn: Option<String>,
    pub sort: u32,
    pub ep: Option<u32>,
    pub duration: Option<String>,
    pub duration_seconds: Option<u32>,
    pub airdate: Option<String>,
    pub desc: Option<String>,
    pub status: Option<String>,
    pub subject_id: Option<u64>,
}

/// Paged result of an episode listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedEpisode {
    pub data: Vec<Episode>,
    pub limit: u32,
    pub offset: u32,
    pub total: u64,
}

/// A subject related to another subject (sequel / prequel / ...).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectRelation {
    pub id: u64,
    pub name: String,
    pub name_cn: Option<String>,
    pub relation: String,
    pub images: Option<SubjectImages>,
}

/// Image URLs of a person (actor / staff) at several sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonImages {
    pub large: Option<String>,
    pub medium: Option<String>,
    pub small: Option<String>,
    pub grid: Option<String>,
}

/// A person (actor / staff) attached to a subject's character (`actors`
/// list) or returned by the subject's persons endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub id: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub person_type: u8,
    pub career: Vec<String>,
    pub images: Option<PersonImages>,
    pub short_summary: Option<String>,
    pub locked: Option<bool>,
}

/// A character of a subject, with its voice actors (CV) in `actors`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedCharacter {
    pub id: u64,
    pub name: String,
    pub summary: Option<String>,
    #[serde(rename = "type")]
    pub character_type: u8,
    pub images: Option<PersonImages>,
    pub relation: Option<String>,
    #[serde(default)]
    pub actors: Vec<Person>,
}

/// A person (producer / director / seiyu / ...) related to a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedPerson {
    pub id: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub person_type: u8,
    pub career: Vec<String>,
    pub images: Option<PersonImages>,
    pub relation: Option<String>,
    pub eps: Option<String>,
}

const API_BASE: &str = "https://api.bgm.tv";

/// Set a request header, mapping WASI errors to a `String`.
fn set_header(headers: &Fields, name: &str, value: &[u8]) -> Result<(), String> {
    headers.set(name, &[value.to_vec()]).map_err(|e| {
        log::warn!("send_request: set {name} failed: {e}");
        format!("set {name} failed: {e}")
    })
}

pub fn send_request(method: Method, url: &str, payload: Option<&str>) -> Result<Vec<u8>, String> {
    log::debug!("send_request => method={method:?} url={url}");
    if let Some(p) = payload {
        log::trace!("send_request payload = {p}");
    }

    let url = Url::parse(url).map_err(|e| {
        log::warn!("send_request: invalid url {url}: {e}");
        format!("invalid url {url}: {e}")
    })?;
    log::trace!(
        "send_request: parsed => scheme={:?} host={:?} path_with_query={:?}",
        url.scheme(),
        url.host_str(),
        &url[Position::BeforePath..]
    );

    log::trace!("send http request (WASI Preview 2 Client)...");

    let headers = Fields::new();
    set_header(
        &headers,
        "User-Agent",
        b"Mozilla/5.0 (X11; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0",
    )?;
    set_header(
        &headers,
        "Accept",
        b"text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    )?;
    set_header(&headers, "Accept-Language", b"en-US,en;q=0.5")?;
    // Connection / TE are hop-by-hop headers, which WASI HTTP forbids
    // (they would fail with HeaderError::Forbidden), so they are not set.
    set_header(&headers, "Upgrade-Insecure-Requests", b"1")?;
    set_header(&headers, "Sec-Fetch-Dest", b"document")?;
    set_header(&headers, "Sec-Fetch-Mode", b"navigate")?;
    set_header(&headers, "Sec-Fetch-Site", b"none")?;
    set_header(&headers, "Sec-Fetch-User", b"?1")?;
    set_header(&headers, "Priority", b"u=0, i")?;
    if matches!(method, Method::Post) {
        set_header(&headers, "Content-Type", b"application/json")?;
        if let Some(p) = payload {
            set_header(&headers, "Content-Length", p.len().to_string().as_bytes())?;
        }
    }

    log::trace!(
        "send headers => \n{:#?}",
        headers
            .entries()
            .iter()
            .map(|(key, value)| format!("{key}:{}", String::from_utf8_lossy(value)))
            .collect::<Vec<_>>()
    );

    let request = OutgoingRequest::new(headers);

    request.set_method(&method).map_err(|_| {
        log::warn!("send_request: set method {method:?} failed");
        "set method failed".to_string()
    })?;
    request.set_scheme(Some(&Scheme::Https)).map_err(|_| {
        log::warn!("send_request: set scheme https failed");
        "set scheme failed".to_string()
    })?;
    request.set_authority(url.host_str()).map_err(|_| {
        log::warn!("send_request: set authority {:?} failed", url.host_str());
        "set authority failed".to_string()
    })?;
    request
        .set_path_with_query(Some(&url[Position::BeforePath..]))
        .map_err(|_| {
            log::warn!(
                "send_request: set path_with_query {:?} failed",
                &url[Position::BeforePath..]
            );
            "set path failed".to_string()
        })?;

    let request_body = request.body().map_err(|e| {
        log::warn!("send_request: get outgoing body failed: {e:?}");
        format!("get body failed: {e:?}")
    })?;

    if let Some(payload) = payload {
        let stream = request_body.write().map_err(|e| {
            log::warn!("send_request: open body stream failed: {e:?}");
            format!("open body stream failed: {e:?}")
        })?;
        stream
            .blocking_write_and_flush(payload.as_bytes())
            .map_err(|e| {
                log::warn!("send_request: write body failed: {e}");
                format!("write body failed: {e}")
            })?;
        log::trace!("send_request: body written ({} bytes)", payload.len());
        drop(stream);
    }

    log::trace!("send_request: calling outgoing_handler::handle ...");
    let future_response = outgoing_handler::handle(request, None).map_err(|e| {
        log::warn!("send_request: outgoing_handler::handle failed: {e}");
        format!("send request failed: {e}")
    })?;
    log::trace!("send_request: handle returned future_response");

    OutgoingBody::finish(request_body, None).map_err(|e| {
        log::warn!("send_request: finish outgoing body failed: {e:?}");
        format!("finish outgoing body failed: {e:?}")
    })?;
    log::trace!("send_request: outgoing body finished");

    let incoming_response = loop {
        if let Some(result) = future_response.get() {
            log::trace!("send_request: future_response ready");
            break result
                .map_err(|e| {
                    log::warn!("send_request: http error code: {e:?}");
                    format!("http error code: {e:?}")
                })?
                .map_err(|e| {
                    log::warn!("send_request: request rejected: {e:?}");
                    format!("request rejected: {e:?}")
                })?;
        }
        future_response.subscribe().block();
    };
    log::trace!("send_request: got incoming_response");

    let status = incoming_response.status();
    log::debug!("send_request: recv status code: {status}");

    let response_body = incoming_response.consume().map_err(|e| {
        log::warn!("send_request: consume response body failed: {e:?}");
        format!("consume response body failed: {e:?}")
    })?;
    let stream = response_body.stream().map_err(|e| {
        log::warn!("send_request: open response stream failed: {e:?}");
        format!("open response stream failed: {e:?}")
    })?;

    let mut response_data = Vec::new();

    loop {
        match stream.blocking_read(4096) {
            Ok(bytes) => {
                if bytes.is_empty() {
                    break;
                }
                response_data.extend_from_slice(&bytes);
            }
            Err(StreamError::Closed) => break,
            Err(e) => {
                log::warn!("send_request: read response stream failed: {e:?}");
                return Err(format!("read response stream failed: {e:?}"));
            }
        }
    }

    log::debug!(
        "send_request: response content length: {} Bytes",
        response_data.len()
    );

    if !(200..300).contains(&status) {
        log::warn!(
            "send_request: bangumi api returned status {status}: {}",
            String::from_utf8_lossy(&response_data)
        );
        return Err(format!(
            "bangumi api returned status {status}: {}",
            String::from_utf8_lossy(&response_data)
        ));
    }

    Ok(response_data)
}

pub fn search_subjects(keyword: &str, subject_type: u8) -> Result<Vec<Subject>, String> {
    log::debug!("searching {keyword}");
    let payload = serde_json::json!({
        "keyword": keyword,
        "sort": "match",
        "filter": { "type": [subject_type] }
    });
    let payload =
        serde_json::to_string(&payload).map_err(|e| format!("encode search body: {e}"))?;
    log::debug!("sending --> \n {payload}");
    let response_data = send_request(
        Method::Post,
        &format!("{API_BASE}/v0/search/subjects?limit=10&offset=0"),
        Some(&payload),
    )?;
    let data: PagedSubject = serde_json::from_slice(&response_data).map_err(|e| {
        log::warn!("search_subjects: decode response failed: {e}");
        format!("decode search response: {e}")
    })?;
    log::debug!("search_subjects found {} entries", data.total);
    Ok(data.data)
}

pub fn get_subject(subject_id: u64) -> Result<Subject, String> {
    let response_data = send_request(
        Method::Get,
        &format!("{API_BASE}/v0/subjects/{subject_id}"),
        None,
    )?;
    serde_json::from_slice(&response_data).map_err(|e| {
        log::warn!("get_subject({subject_id}): decode response failed: {e}");
        format!("decode subject response: {e}")
    })
}

pub fn get_episodes(subject_id: u64, ep_type: u8) -> Result<Vec<Episode>, String> {
    let mut episodes = Vec::new();
    let mut offset = 0u32;
    loop {
        let url = format!(
            "{API_BASE}/v0/episodes?subject_id={subject_id}&type={ep_type}&limit=100&offset={offset}"
        );
        let response_data = send_request(Method::Get, &url, None)?;
        let page: PagedEpisode = serde_json::from_slice(&response_data).map_err(|e| {
            log::warn!("get_episodes({subject_id}): decode response failed: {e}");
            format!("decode episodes response: {e}")
        })?;
        let count = page.data.len();
        episodes.extend(page.data);
        if count < 100 {
            break;
        }
        offset += count as u32;
    }
    Ok(episodes)
}

pub fn get_subject_relations(subject_id: u64) -> Result<Vec<SubjectRelation>, String> {
    let response_data = send_request(
        Method::Get,
        &format!("{API_BASE}/v0/subjects/{subject_id}/subjects"),
        None,
    )?;
    serde_json::from_slice(&response_data).map_err(|e| {
        log::warn!("get_subject_relations({subject_id}): decode response failed: {e}");
        format!("decode relations response: {e}")
    })
}

pub fn get_subject_characters(subject_id: u64) -> Result<Vec<RelatedCharacter>, String> {
    let response_data = send_request(
        Method::Get,
        &format!("{API_BASE}/v0/subjects/{subject_id}/characters"),
        None,
    )?;
    serde_json::from_slice(&response_data).map_err(|e| {
        log::warn!("get_subject_characters({subject_id}): decode response failed: {e}");
        format!("decode characters response: {e}")
    })
}

pub fn get_subject_persons(subject_id: u64) -> Result<Vec<RelatedPerson>, String> {
    let response_data = send_request(
        Method::Get,
        &format!("{API_BASE}/v0/subjects/{subject_id}/persons"),
        None,
    )?;
    serde_json::from_slice(&response_data).map_err(|e| {
        log::warn!("get_subject_persons({subject_id}): decode response failed: {e}");
        format!("decode persons response: {e}")
    })
}
