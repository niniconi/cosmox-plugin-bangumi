//! Tree rebuild flow: search the Bangumi API for each series node, write
//! series/season metadata, and enrich cast, staff and episodes.
//!
//! Follows the anime plugin's clone style: views are passed by value and
//! cloned internally (a view only holds a single handle reference, so
//! cloning is cheap). All errors are logged, never silently ignored.

use cosmox_api::api::bindings::cosmox::plugin::context::MetadataQuery;
use cosmox_api::handle::{MetadataView, PathMappingView, TagView};
use cosmox_plugin_anime::anime::define::{Anime, Season};

use crate::anime::action::enrich::{
    collect_episode_files, enrich_cast, enrich_episode, enrich_staff, push_cover,
};
use crate::anime::define::{BangumiAnime, BangumiSeason};
use crate::bangumi;

const ANIME_TYPE: u8 = 2;

/// Search Bangumi for an anime subject and return the best match.
fn search_anime(keyword: &str) -> Option<bangumi::Subject> {
    match bangumi::search_subjects(keyword, ANIME_TYPE) {
        Ok(mut list) => {
            log::debug!("search '{keyword}' returned {} candidates", list.len());
            list.sort_by_key(|s| {
                std::cmp::Reverse(s.collection.as_ref().map(|c| c.collect).unwrap_or(0))
            });
            let best = list.into_iter().next();
            if let Some(s) = &best {
                log::info!(
                    "pick subject id={} name={} cn={:?}",
                    s.id,
                    s.name,
                    s.name_cn
                );
            }
            best
        }
        Err(err) => {
            log::warn!("search '{keyword}' FAILED: {err}");
            None
        }
    }
}

fn apply_subject_metadata(
    view: MetadataView,
    path: &PathMappingView,
    tags: &TagView,
    id: u64,
    subject: &bangumi::Subject,
) {
    let name_cn = subject.name_cn.clone().unwrap_or_default();
    if !name_cn.is_empty() {
        log::debug!("chinese name = {name_cn}");
        view.modify(&MetadataQuery::Id(id), "name", &name_cn);
    }

    if let Some(summary) = &subject.summary {
        view.modify(&MetadataQuery::Id(id), "description", summary);
    }
    view.write_extend(&MetadataQuery::Id(id), Anime::from(subject.clone()));
    view.write_extend(&MetadataQuery::Id(id), BangumiAnime { id: subject.id });
    if let Some(meta_tags) = &subject.meta_tags {
        log::debug!("tags = {meta_tags:#?}");
        let tags_list = meta_tags
            .iter()
            .map(|x| ("common".to_string(), x.clone()))
            .collect::<Vec<_>>();
        if let Err(err) = tags.add_tags(id, tags_list.as_slice()) {
            log::warn!("add tags for {id} failed: {err}");
        }
    }
    push_cover(
        path,
        id,
        subject
            .images
            .large
            .clone()
            .or_else(|| subject.image.clone()),
    );
}

/// Locate the subject for season `n` of `series`:
/// - season 1 uses the series subject itself;
/// - later seasons walk the "续集" relation chain, falling back to a search.
fn find_season_subject(
    series_subject: &bangumi::Subject,
    series_name: &str,
    season: u32,
) -> Option<bangumi::Subject> {
    if season <= 1 {
        log::info!(
            "season {season} -> use series subject id={}",
            series_subject.id
        );
        return Some(series_subject.clone());
    }

    log::debug!(
        "season {season}: walk relation chain from series id={} name={}",
        series_subject.id,
        series_subject.name
    );
    let mut next = Some(series_subject.clone());
    for _ in 1..season {
        let Some(current) = next else { break };
        let Ok(relations) = bangumi::get_subject_relations(current.id) else {
            log::warn!(
                "season {season}: get_subject_relations({}) failed",
                current.id
            );
            next = None;
            break;
        };
        log::debug!(
            "season {season}: subject {} has {} relations",
            current.id,
            relations.len()
        );
        let Some(sequel) = relations
            .into_iter()
            .find(|r| r.relation == "续集" || r.relation == "sequel")
        else {
            log::warn!(
                "season {season}: no sequel relation for subject {}",
                current.id
            );
            next = None;
            break;
        };
        log::debug!(
            "season {season}: sequel -> id={} name={}",
            sequel.id,
            sequel.name
        );
        next = bangumi::get_subject(sequel.id).ok();
    }
    if let Some(subject) = next {
        log::info!(
            "season {season}: relation chain resolved to id={} name={}",
            subject.id,
            subject.name
        );
        return Some(subject);
    }

    log::warn!("season {season}: relation chain broken, fall back to search");
    for keyword in [
        format!("{series_name} 第{season}季"),
        format!("{series_name} S{season}"),
        format!("{series_name} Season {season}"),
    ] {
        if let Some(subject) = search_anime(&keyword) {
            log::info!(
                "season {season}: fallback search '{keyword}' -> id={}",
                subject.id
            );
            return Some(subject);
        }
    }
    None
}

/// Enrich one `Season N` node: resolve its Bangumi subject, fetch its
/// episodes, and annotate every episode file inside it.
pub fn season_metadata(
    view: MetadataView,
    path: &PathMappingView,
    season_id: u64,
    series_subject: &bangumi::Subject,
    series_name: &str,
) {
    let name = view.name(&MetadataQuery::Id(season_id)).unwrap_or_default();
    let season = name
        .trim()
        .strip_prefix("Season")
        .and_then(|rest| rest.trim().parse::<u32>().ok())
        .or_else(|| name.trim().parse::<u32>().ok());

    let Some(season) = season else {
        return;
    };
    log::info!("processing season {season} of {series_name}");

    let Some(subject) = find_season_subject(series_subject, series_name, season) else {
        log::warn!("no subject found for {series_name} season {season}");
        return;
    };
    log::info!(
        "season {season} resolved to subject id={} name={}",
        subject.id,
        subject.name
    );

    let season_data = Season {
        season_number: Some(season),
    };
    if let Some(summary) = &subject.summary {
        view.modify(&MetadataQuery::Id(season_id), "description", summary);
    }
    view.write_extend(&MetadataQuery::Id(season_id), season_data);
    view.write_extend(
        &MetadataQuery::Id(season_id),
        BangumiSeason { id: subject.id },
    );
    enrich_cast(view.clone(), path, season_id, subject.id);
    enrich_staff(view.clone(), path, season_id, subject.id);

    let episodes = match bangumi::get_episodes(subject.id, 0) {
        Ok(eps) => eps,
        Err(err) => {
            log::warn!("fetch episodes of {} failed: {err}", subject.id);
            return;
        }
    };
    log::debug!("subject {} has {} episodes", subject.id, episodes.len());

    let files = collect_episode_files(view.clone(), season_id);
    log::debug!(
        "season {season} has {} episode files to enrich",
        files.len()
    );
    for (file_id, episode_number) in files {
        enrich_episode(view.clone(), file_id, episode_number, &episodes);
    }
}

/// Enrich one series node: search Bangumi, write series metadata, then
/// recurse into its `Season N` children.
pub fn rebuild_series(view: MetadataView, path: &PathMappingView, tags: &TagView, node: u64) {
    let Some(name) = view.name(&MetadataQuery::Id(node)) else {
        log::warn!("cannot read name of node {node}, skip");
        return;
    };
    log::info!("processing series {name}");
    log::debug!("search keyword = '{name}'");

    let Some(subject) = search_anime(&name) else {
        log::warn!("no subject found for {name}");
        return;
    };
    log::info!(
        "matched subject {} ({}), cn={:?}",
        subject.id,
        subject.name,
        subject.name_cn
    );
    apply_subject_metadata(view.clone(), path, tags, node, &subject);
    enrich_cast(view.clone(), path, node, subject.id);
    enrich_staff(view.clone(), path, node, subject.id);

    let season_children = view.children(&MetadataQuery::Id(node));
    log::debug!(
        "series {name} has {} season children",
        season_children.len()
    );
    for child in season_children {
        season_metadata(view.clone(), path, child, &subject, &name);
    }
}

pub fn rebuild_metadata_tree(view: MetadataView, path: PathMappingView, tags: TagView) {
    let roots = view.children(&MetadataQuery::Id(0));
    log::info!("rebuild_metadata_tree: {} root nodes", roots.len());
    for child in roots {
        rebuild_series(view.clone(), &path, &tags, child);
    }
    log::info!("rebuild_metadata_tree finished");
}
