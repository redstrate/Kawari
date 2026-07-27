use binrw::binrw;

use crate::common::{read_bool_from, write_bool_as};

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherChange {
    /// Index into the Weather Excel sheet.
    pub weather_id: u8,
    /// Whether this resets the current forced weather.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub disables_override: bool,
    /// Presumably how long the weather takes to change, but haven't played with this yet.
    #[brw(pad_before = 2)] // not read
    pub fade_length: f32,
}
