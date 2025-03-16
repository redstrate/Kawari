use binrw::{BinRead, BinWrite, binrw};

pub trait IpcSegmentTrait:
    for<'a> BinRead<Args<'a> = ()> + for<'a> BinWrite<Args<'a> = ()> + std::fmt::Debug + 'static
{
    /// Calculate the size of this Ipc segment *including* the 16 byte header.
    /// When implementing this, please use the size as seen in retail.
    fn calc_size(&self) -> u32;
}

#[binrw]
#[derive(Debug, Clone)]
pub struct IpcSegment<OpCode, Data>
where
    for<'a> OpCode: BinRead<Args<'a> = ()> + 'a + std::fmt::Debug,
    for<'a> OpCode: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug,
    for<'a> Data: BinRead<Args<'a> = (&'a OpCode,)> + 'a + std::fmt::Debug,
    for<'a> Data: BinWrite<Args<'a> = ()> + 'a + std::fmt::Debug,
{
    pub unk1: u8,
    pub unk2: u8,
    #[br(dbg)]
    pub op_code: OpCode,
    #[brw(pad_before = 2)] // empty
    #[br(dbg)]
    pub server_id: u16,
    #[br(dbg)]
    pub timestamp: u32,
    #[brw(pad_before = 4)]
    #[br(args(&op_code))]
    pub data: Data,
}
