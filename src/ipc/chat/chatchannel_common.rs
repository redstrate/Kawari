use binrw::binrw;

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, Copy, Debug, Default)]
pub enum ChatChannelType {
    #[default]
    CWLinkshellOrNone = 0,
    Party = 1,
    Linkshell = 2,
    FreeCompany = 3,
    NoviceNetwork = 4,

    // These technically don't belong here, but if we ever internally represent zone connection chats with ChatChannels (e.g. for multiple worlds, or maybe routing zone chat based on the channel number maybe being a zone id?), these are good to have. These only directly show up as u16s in zone connection chat messages.
    Say = 10,
    Shout = 11,
    CustomEmote = 28,
    Yell = 30,
}

#[binrw]
#[derive(Clone, Copy, Debug, Default)]
pub struct ChatChannel {
    pub channel_number: u32,
    pub channel_type: ChatChannelType,
    pub world_id: u16,
}
