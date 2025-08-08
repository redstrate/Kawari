use binrw::{BinRead, BinWrite, binrw};

use crate::common::timestamp_secs;

/// Required to implement for specializations of `IpcSegment`. These should be read/writeable, however for client packets you can leave calc_size() unimplemented.
pub trait ReadWriteIpcSegment:
    for<'a> BinRead<Args<'a> = (&'a u32,)>
    + for<'a> BinWrite<Args<'a> = ()>
    + std::fmt::Debug
    + 'static
    + Default
{
    /// Calculate the size of this Ipc segment *including* the 16 byte header.
    /// When implementing this, please use the size seen in retail instead of guessing.
    fn calc_size(&self) -> u32;

    /// Returns a human-readable name of the opcode.
    fn get_name(&self) -> &'static str;

    /// Returns the integer opcode.
    fn get_opcode(&self) -> u16;
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
#[br(import(size: &u32))]
pub struct IpcSegment<OpCode, Data>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode, &'a u32)> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    /// Unknown purpose, but usually 20.
    pub unk1: u8,
    /// Unknown purpose, but usually 0.
    pub unk2: u8,
    /// The opcode for this segment.
    pub op_code: OpCode,
    #[brw(pad_before = 4)] // empty
    /// Unknown purpose, but safe to keep 0.
    pub option: u16,
    /// The timestamp of this packet in seconds since UNIX epoch.
    #[brw(pad_before = 2)]
    pub timestamp: u32,
    /// The data associated with the opcode.
    #[br(args(&op_code, size))]
    pub data: Data,
}

impl<OpCode, Data> Default for IpcSegment<OpCode, Data>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode, &'a u32)> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn default() -> Self {
        Self {
            unk1: 20,
            unk2: 0,
            op_code: OpCode::default(),
            option: 15,
            timestamp: timestamp_secs(),
            data: Data::default(),
        }
    }
}

pub const IPC_HEADER_SIZE: u32 = 16;
