use binrw::{BinRead, BinWrite, binrw};

use crate::common::timestamp_secs;

/// Required to implement specializations of `IpcSegment`.
pub trait ReadWriteIpcOpcode<T> {
    /// Returns the opcode that fits `data`.
    fn from_data(data: &T) -> Self;
}

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

    /// Returns the comment for this opcode.
    fn get_comment(&self) -> Option<&'static str>;
}

pub trait IpcSegmentHeader<T> {
    /// Returns the header with `opcode`.
    fn from_opcode(opcode: T) -> Self;

    /// Returns the opcode.
    fn opcode(&self) -> &T;
}

/// Seen in Zone connections. Has an extra field containing server information.
#[binrw]
#[derive(Debug, Clone)]
#[brw(magic = 0x14u16)]
pub struct ServerIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    /// The opcode for this segment.
    pub op_code: OpCode,
    #[brw(pad_before = 4)] // empty
    /// This is the internal server ID. (*Not* the World ID.) This seems to be just for informational purposes, and doesn't affect anything functionally.
    pub server_id: u16,
    /// The timestamp of this packet in seconds since UNIX epoch.
    #[brw(pad_before = 2)] // not sure if always empty
    pub timestamp: u32,
}

impl<OpCode> Default for ServerIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn default() -> Self {
        Self {
            op_code: OpCode::default(),
            server_id: 0,
            timestamp: timestamp_secs(),
        }
    }
}

impl<OpCode> IpcSegmentHeader<OpCode> for ServerIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn from_opcode(opcode: OpCode) -> Self {
        Self {
            op_code: opcode,
            ..Default::default()
        }
    }

    fn opcode(&self) -> &OpCode {
        return &self.op_code;
    }
}

/// Seen in Lobby connections. Only has the timestamp and opcode.
#[binrw]
#[derive(Debug, Clone)]
#[brw(magic = 0x14u16)]
pub struct ServerlessIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    /// The opcode for this segment.
    pub op_code: OpCode,
    /// The timestamp of this packet in seconds since UNIX epoch.
    #[brw(pad_before = 4)] // empty
    #[brw(pad_after = 4)] // empty
    pub timestamp: u32,
}

impl<OpCode> Default for ServerlessIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn default() -> Self {
        Self {
            op_code: OpCode::default(),
            timestamp: timestamp_secs(),
        }
    }
}

impl<OpCode> IpcSegmentHeader<OpCode> for ServerlessIpcSegmentHeader<OpCode>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn from_opcode(opcode: OpCode) -> Self {
        Self {
            op_code: opcode,
            ..Default::default()
        }
    }

    fn opcode(&self) -> &OpCode {
        return &self.op_code;
    }
}

/// An IPC packet segment.
/// When implementing a new connection type, `OpCode` and `Data` can be used to specialize this type.
#[binrw]
#[derive(Debug, Clone)]
#[br(import(size: &u32))]
pub struct IpcSegment<Header, OpCode, Data>
where
    for<'a> Header:
        BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Header:
        BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode, &'a u32)> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    pub header: Header,
    /// The data associated with the opcode.
    #[br(args(&header.opcode(), size))]
    pub data: Data,
}

impl<Header, OpCode, Data> IpcSegment<Header, OpCode, Data>
where
    for<'a> OpCode: ReadWriteIpcOpcode<Data>,
    for<'a> Header:
        BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Header:
        BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode, &'a u32)> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    /// Creates a new IPC segment with the specified `data`.
    pub fn new(data: Data) -> Self {
        Self {
            header: Header::from_opcode(OpCode::from_data(&data)),
            data,
            ..Default::default()
        }
    }
}

impl<Header, OpCode, Data> Default for IpcSegment<Header, OpCode, Data>
where
    for<'a> Header:
        BinRead<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Header:
        BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default + IpcSegmentHeader<OpCode>,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode, &'a u32)> + 'a + std::fmt::Debug + Default,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug + Default,
{
    fn default() -> Self {
        Self {
            header: Header::default(),
            data: Data::default(),
        }
    }
}

pub const IPC_HEADER_SIZE: u32 = 16;
