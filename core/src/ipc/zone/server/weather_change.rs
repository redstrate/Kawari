use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherChange {
    /// Index into the Weather Excel sheet.
    pub weather_id: u16,
    /// Presumably how long the weather takes to change, but haven't played with this yet.
    #[brw(pad_before = 2)]
    pub transistion_time: f32,
}
