use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use physis::{
    common::{Language, Platform},
    layer::{LayerEntryData, LayerGroup},
    lvb::Lvb,
    resource::{Resource, SqPackResource},
};

fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let args: Vec<String> = std::env::args().collect();
    let zone_id: u16 = args[1].parse().unwrap();

    tracing::info!("Generating navmesh for zone {zone_id}!");

    let mut sqpack_resource =
        SqPackResource::from_existing(Platform::Win32, &config.filesystem.game_path);
    let sheet = TerritoryTypeSheet::read_from(&mut sqpack_resource, Language::None).unwrap();
    let Some(row) = sheet.get_row(zone_id as u32) else {
        tracing::error!("Invalid zone id {zone_id}!");
        return;
    };

    // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
    let bg_path = row.Bg().into_string().unwrap();

    let path = format!("bg/{}.lvb", &bg_path);
    let lvb_file = sqpack_resource.read(&path).unwrap();
    let lvb = Lvb::from_existing(&lvb_file).unwrap();

    for path in &lvb.scns[0].header.path_layer_group_resources {
        if path.contains("bg.lgb") {
            tracing::info!("Processing {path}");

            let lgb_file = sqpack_resource.read(path).unwrap();
            let lgb = LayerGroup::from_existing(&lgb_file);
            let Some(lgb) = lgb else {
                tracing::error!(
                    "Failed to parse {path}, this is most likely a bug in Physis and should be reported somewhere!"
                );
                return;
            };

            // TODO: i think we know which layer is specifically used for navmesh gen, better check that LVB
            for chunk in &lgb.chunks {
                for layer in &chunk.layers {
                    for object in &layer.objects {
                        if let LayerEntryData::BG(bg) = &object.data {
                            if !bg.collision_asset_path.value.is_empty() {
                                tracing::info!("Considering {} for navimesh", object.instance_id);

                                tracing::info!("- Loading {}", bg.collision_asset_path.value);
                            }
                        }
                    }
                }
            }
        }
    }
}
