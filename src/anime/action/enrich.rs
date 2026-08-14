//! Cast / staff / episode enrichment helpers for the Bangumi source.
//!
//! These functions annotate the metadata tree with data fetched from the
//! Bangumi API. They follow the anime plugin's clone style: views are
//! passed by value and cloned internally, which is cheap because a view
//! only holds a single handle reference.

use cosmox_api::api::bindings::cosmox::plugin::context::MetadataQuery;
use cosmox_api::handle::{MetadataView, PathMappingView};
use cosmox_api::metadata::MetadataType;
use cosmox_plugin_anime::anime::define::{Character, Episode, Staff};

use crate::anime::define::{BangumiCharacter, BangumiStaff};
use crate::bangumi;

/// Find the rid of the first child of `parent_id` named `name` (if any).
/// Used to keep cast/staff trees idempotent across rebuilds.
fn find_child_by_name(view: &MetadataView, parent_id: u64, name: &str) -> Option<u64> {
    view.children(&MetadataQuery::Id(parent_id))
        .into_iter()
        .find(|&child| view.name(&MetadataQuery::Id(child)).as_deref() == Some(name))
}

fn preferred_image(images: &Option<bangumi::PersonImages>) -> Option<String> {
    images
        .as_ref()
        .and_then(|img| img.large.clone().or_else(|| img.medium.clone()))
}

/// Create one virtual node per character of `subject_id` under a
/// `Cast` container node placed under `parent_id`. Each character node
/// carries its name, summary, relation and the list of voice actors.
pub fn enrich_cast(view: MetadataView, path: &PathMappingView, parent_id: u64, subject_id: u64) {
    let Ok(characters) = bangumi::get_subject_characters(subject_id) else {
        log::warn!("fetch characters of {subject_id} failed");
        return;
    };
    log::debug!("subject {subject_id} has {} characters", characters.len());
    if characters.is_empty() {
        return;
    }

    // Idempotency: re-running this plugin must not duplicate the tree.
    let cast_container = match find_child_by_name(&view, parent_id, "Cast") {
        Some(rid) => rid,
        None => match view
            .create_node(&MetadataQuery::Id(parent_id), "Cast", MetadataType::Virtual)
            .map(MetadataQuery::Id)
        {
            Some(MetadataQuery::Id(rid)) => rid,
            _ => return,
        },
    };

    for character in characters {
        let node = match view
            .create_node(
                &MetadataQuery::Id(cast_container),
                &character.name,
                MetadataType::Virtual,
            )
            .map(MetadataQuery::Id)
        {
            Some(MetadataQuery::Id(rid)) => rid,
            _ => continue,
        };
        if let Some(summary) = &character.summary
            && !summary.is_empty()
        {
            view.modify(&MetadataQuery::Id(node), "description", summary);
        }
        let cover = preferred_image(&character.images);
        view.write_extend(
            &MetadataQuery::Id(node),
            BangumiCharacter { id: character.id },
        );
        view.write_extend(&MetadataQuery::Id(node), Character::from(character));
        push_cover(path, node, cover);
    }
}

/// Create one virtual node per person of `subject_id` under a `Staff`
/// container node placed under `parent_id`. Each person node carries
/// its career list and episode range.
pub fn enrich_staff(view: MetadataView, path: &PathMappingView, parent_id: u64, subject_id: u64) {
    let Ok(persons) = bangumi::get_subject_persons(subject_id) else {
        log::warn!("fetch persons of {subject_id} failed");
        return;
    };
    log::debug!("subject {subject_id} has {} persons", persons.len());
    if persons.is_empty() {
        return;
    }

    let staff_container = match find_child_by_name(&view, parent_id, "Staff") {
        Some(rid) => rid,
        None => match view
            .create_node(
                &MetadataQuery::Id(parent_id),
                "Staff",
                MetadataType::Virtual,
            )
            .map(MetadataQuery::Id)
        {
            Some(MetadataQuery::Id(rid)) => rid,
            _ => return,
        },
    };

    for person in persons {
        let node = match view
            .create_node(
                &MetadataQuery::Id(staff_container),
                &person.name,
                MetadataType::Virtual,
            )
            .map(MetadataQuery::Id)
        {
            Some(MetadataQuery::Id(rid)) => rid,
            _ => continue,
        };
        let cover = preferred_image(&person.images);
        view.write_extend(&MetadataQuery::Id(node), BangumiStaff { id: person.id });
        view.write_extend(&MetadataQuery::Id(node), Staff::from(person));
        push_cover(path, node, cover);
    }
}

/// Write the cover URL (if any) into the node's path mapping under
/// `cover_file_map_id`. No-op when `cover` is empty or `None`.
pub fn push_cover(path: &PathMappingView, id: u64, cover: Option<String>) {
    let Some(url) = cover.filter(|u| !u.is_empty()) else {
        return;
    };
    if let Err(err) = path.push(id, "cover_file_map_id", &url) {
        log::warn!("push cover for {id} failed: {err:?}");
    }
}

/// Collect `(rid, episode_number)` pairs from the season node's children,
/// recursing into a nested virtual `unknown` node.
pub fn collect_episode_files(view: MetadataView, season_id: u64) -> Vec<(u64, u32)> {
    let mut files = Vec::new();
    for child in view.children(&MetadataQuery::Id(season_id)) {
        collect_episode_file(view.clone(), child, &mut files);
    }
    files
}

fn collect_episode_file(view: MetadataView, id: u64, out: &mut Vec<(u64, u32)>) {
    if let Some(episode) = view.read_extend::<Episode>(&MetadataQuery::Id(id))
        && let Some(n) = episode.episode_number
    {
        out.push((id, n));
        return;
    }
    // The anime plugin groups season-internal files without an episode
    // number under a nested `unknown` node; recurse into it to find them.
    let name = view.name(&MetadataQuery::Id(id)).unwrap_or_default();
    if name == "unknown" {
        log::debug!("recursing into unknown node {id} for episode files");
        for grandchild in view.children(&MetadataQuery::Id(id)) {
            collect_episode_file(view.clone(), grandchild, out);
        }
    }
}

pub fn enrich_episode(
    view: MetadataView,
    file_id: u64,
    episode_number: u32,
    episodes: &[bangumi::Episode],
) {
    let Some(bgm_ep) = episodes.iter().find(|e| e.ep == Some(episode_number)) else {
        log::warn!("episode {episode_number} not found in subject episodes");
        return;
    };
    let mut episode = Episode::from(bgm_ep.clone());
    episode.episode_number = Some(episode_number);
    if let Some(desc) = bgm_ep.desc.as_ref().filter(|s| !s.is_empty()) {
        view.modify(&MetadataQuery::Id(file_id), "description", desc);
    }
    view.write_extend(&MetadataQuery::Id(file_id), episode);
}
