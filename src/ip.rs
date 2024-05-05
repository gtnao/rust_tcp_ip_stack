use anyhow::Result;

pub const IP_ADDRESS_LENGTH: u8 = 4;

#[derive(Debug)]
enum IPVersion {
    IPv4,
    IPv6,
}
#[derive(Debug)]
enum IPProtocol {
    ICMP,
    TCP,
    UDP,
}

#[derive(Debug)]
pub struct IPPacket {
    header: IPHeader,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct IPHeader {
    version: IPVersion,
    ihl: u8,
    precedence: u8,
    delay: bool,
    throughput: bool,
    reliability: bool,
    total_length: u16,
    identification: u16,
    df: bool,
    mf: bool,
    fragment_offset: u16,
    ttl: u8,
    protocol: IPProtocol,
    header_checksum: u16,
    source_ip_address: u32,
    destination_ip_address: u32,
    options: Vec<u8>,
}

impl IPHeader {
    fn parse(data: &[u8]) -> Result<Self> {
        let version = match data[0] >> 4 {
            4 => IPVersion::IPv4,
            6 => IPVersion::IPv6,
            _ => return Err(anyhow::anyhow!("Invalid IP version")),
        };
        let ihl = data[0] & 0x0F;
        let precedence = data[1] >> 5;
        let delay = (data[1] >> 4) & 1 == 1;
        let throughput = (data[1] >> 3) & 1 == 1;
        let reliability = (data[1] >> 2) & 1 == 1;
        let total_length = u16::from_be_bytes([data[2], data[3]]);
        let identification = u16::from_be_bytes([data[4], data[5]]);
        let df = (data[6] >> 6) & 1 == 1;
        let mf = (data[6] >> 5) & 1 == 1;
        let fragment_offset = u16::from_be_bytes([data[6] & 0x1F, data[7]]);
        let ttl = data[8];
        let protocol = match data[9] {
            1 => IPProtocol::ICMP,
            6 => IPProtocol::TCP,
            17 => IPProtocol::UDP,
            _ => return Err(anyhow::anyhow!("Invalid IP protocol")),
        };
        let header_checksum = u16::from_be_bytes([data[10], data[11]]);
        let source_ip_address = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let destination_ip_address = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let options = data[20..(ihl as usize * 4)].to_vec();
        Ok(IPHeader {
            version,
            ihl,
            precedence,
            delay,
            throughput,
            reliability,
            total_length,
            identification,
            df,
            mf,
            fragment_offset,
            ttl,
            protocol,
            header_checksum,
            source_ip_address,
            destination_ip_address,
            options,
        })
    }
    fn data_offset(&self) -> usize {
        (self.ihl << 2) as usize
    }
}

impl IPPacket {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        let header = IPHeader::parse(&data)?;
        let data = data[header.data_offset()..].to_vec();
        Ok(IPPacket { header, data })
    }
}

// pub struct IPController {}
//
// impl IPController {
//     pub fn new() -> Self {
//         IPController {}
//     }
//
//     pub fn input(&self, data: Vec<u8>) {}
// }
