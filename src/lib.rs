use cosmox_api::api::plugin;

mod anime;
mod bangumi;

#[plugin]
mod bangumi_plugin {
    use cosmox_api::api::bindings::cosmox::plugin::cosmox_types;
    use cosmox_api::api::bindings::exports::cosmox::plugin::{
        command_handler, core_lifecycle, host_notifier, telemetry_reporter,
    };
    use cosmox_api::event::payloads::OnMetadataLocalTreeReadyEventContext;
    use cosmox_api::handle::{MetadataView, PathMappingView, TagView};

    use crate::anime::action::rebuild_metadata_tree;

    #[on_load]
    fn on_load(config: core_lifecycle::ConfigData) -> core_lifecycle::PluginResult {
        log::info!(
            "on loading bangumi plugin: id={}, name={}",
            config.id,
            config.name
        );
        core_lifecycle::PluginResult::Ok
    }

    #[on_event(OnMetadataLocalTreeReady({ r#type: vec!["Anime".into()], ..Default::default() }))]
    fn handle_metadata_local_ready(
        _data: OnMetadataLocalTreeReadyEventContext,
        metadata_view: MetadataView,
        path_mapping_view: PathMappingView,
        tag_view: TagView,
    ) -> host_notifier::PluginResult {
        log::info!("metadata local tree ready, enriching");
        rebuild_metadata_tree(metadata_view, path_mapping_view, tag_view);
        host_notifier::PluginResult::Ok
    }

    #[health]
    fn get_health() -> telemetry_reporter::PluginHealthStatus {
        telemetry_reporter::PluginHealthStatus {
            status: cosmox_types::PluginStatus::Ok,
            message: None,
            metrics: None,
        }
    }

    #[on_command]
    fn execute_command(command_name: String, args: Vec<String>) -> command_handler::PluginResult {
        log::info!("execute command: {command_name} args: {args:?}");
        command_handler::PluginResult::Ok
    }
}
