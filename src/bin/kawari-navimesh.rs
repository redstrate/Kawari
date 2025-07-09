use icarus::TerritoryType::TerritoryTypeSheet;
use kawari::config::get_config;
use physis::{
    common::{Language, Platform},
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
        }
    }
}
