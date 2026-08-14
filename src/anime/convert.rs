use std::collections::HashMap;

use cosmox_plugin_anime::anime::define::Episode as AnimeEpisode;
use cosmox_plugin_anime::anime::define::{
    Anime, AnimeSource, AnimeStatus, Character, Staff, StrList, TitleMap, VoiceActor,
    VoiceActorList,
};

use crate::bangumi::{Episode as BangumiEpisode, Person, RelatedCharacter, RelatedPerson, Subject};

impl From<Subject> for Anime {
    fn from(s: Subject) -> Self {
        let title_main = s
            .name_cn
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| s.name.clone());
        let mut titles = HashMap::new();
        titles.insert("main".to_string(), title_main.clone());
        if let Some(cn) = s.name_cn.clone().filter(|s| !s.is_empty()) {
            titles.insert("cn".to_string(), cn);
        }
        if !s.name.is_empty() {
            titles.insert("ja".to_string(), s.name.clone());
        }
        let official_website = s.infobox.iter().find_map(|info| {
            (info.key == "官方网站")
                .then(|| info.value.as_str())
                .flatten()
                .map(String::from)
        });
        Anime {
            titles: TitleMap(titles),
            genres: StrList::default(),
            start_date: s.date.clone(),
            end_date: None,
            status: AnimeStatus::default(),
            source: AnimeSource::default(),
            studios: StrList::default(),
            rating: s.rating.as_ref().map(|r| r.score as f32),
            official_website,
            eps: s.eps,
            platform: s.platform.clone(),
        }
    }
}

impl From<BangumiEpisode> for AnimeEpisode {
    fn from(e: BangumiEpisode) -> Self {
        AnimeEpisode {
            season_number: None,
            episode_number: e.ep,
            air_date: e.airdate.clone().filter(|s| !s.is_empty()),
            duration_minutes: e.duration_seconds.map(|d| d as f32 / 60.0),
            extra_kind: None,
            extra_title: None,
            extra_tag: None,
        }
    }
}

impl From<RelatedCharacter> for Character {
    fn from(c: RelatedCharacter) -> Self {
        Character {
            voice_actors: VoiceActorList(c.actors.into_iter().map(VoiceActor::from).collect()),
            gender: None,
            birth_date: None,
            role: c.relation,
        }
    }
}

impl From<Person> for VoiceActor {
    fn from(p: Person) -> Self {
        VoiceActor {
            id: p.id as u32,
            name: p.name,
            name_japanese: None,
            birth_date: None,
            nationality: None,
            image_url: p.images.as_ref().and_then(|i| i.large.clone()),
        }
    }
}

impl From<RelatedPerson> for Staff {
    fn from(p: RelatedPerson) -> Self {
        Staff {
            career: StrList(p.career),
            episode_range: p.eps.clone().filter(|s| !s.is_empty()),
        }
    }
}
