use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct WeatherChange {
    pub weather_id: u16,
    #[brw(pad_before = 3)]
    pub transistion_time: f32,
}
