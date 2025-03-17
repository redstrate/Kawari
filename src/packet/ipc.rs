use binrw::{BinRead, BinWrite, binrw};

pub trait IpcSegmentTrait:
    for<'a> BinRead<Args<'a> = ()> + for<'a> BinWrite<Args<'a> = ()> + std::fmt::Debug + 'static
{
    /// Calculate the size of this Ipc segment *including* the 16 byte header.
    /// When implementing this, please use the size as seen in retail.
    fn calc_size(&self) -> u32;
}

/// An IPC packet segment.
/// When implementing a new connection type, `OpCode` and `Data` can be used to specialize this type:
/// ```
/// # use binrw::binrw;
/// # use kawari::packet::IpcSegment;
/// #
/// # #[binrw]
/// # #[brw(repr = u16)]
/// # #[derive(Clone, PartialEq, Debug)]
/// # pub enum ClientLobbyIpcType {
/// #     Dummy = 0x1,
/// # }
/// #
/// # #[binrw]
/// # #[br(import(magic: &ClientLobbyIpcType))]
/// # #[derive(Debug, Clone)]
/// # pub enum ClientLobbyIpcData {
/// #     Dummy()
/// # }
/// #
/// pub type ClientLobbyIpcSegment = IpcSegment<ClientLobbyIpcType, ClientLobbyIpcData>;
/// ```
#[binrw]
#[derive(Debug, Clone)]
pub struct IpcSegment<OpCode, Data>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode,)> + 'a + std::fmt::Debug,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug,
{
    /// Unknown purpose, but usually 20.
    pub unk1: u8,
    /// Unknown purpose, but usually 0.
    pub unk2: u8,
    /// The opcode for this segment.
    pub op_code: OpCode,
    #[brw(pad_before = 2)] // empty
    /// Unknown purpose, but safe to keep 0.
    pub server_id: u16,
    /// The timestamp of this packet in seconds since UNIX epoch.
    pub timestamp: u32,
    /// The data associated with the opcode.
    #[brw(pad_before = 4)]
    #[br(args(&op_code))]
    pub data: Data,
}
