use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::ethernet::{ETHERNET_ADDRESS_LENGTH, ETHERNET_TYPE_IP};
use crate::ip::IP_ADDRESS_LENGTH;

const ARP_HARDWARE_TYPE_ETHERNET: u16 = 0x0001;
const ARP_PROTOCOL_TYPE_IP: u16 = ETHERNET_TYPE_IP;
const ARP_OPCODE_REQUEST: u16 = 0x0001;
const ARP_OPCODE_REPLY: u16 = 0x0002;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ARPHeader {
    hardware_type: u16,
    protocol_type: u16,
    hardware_length: u8,
    protocol_length: u8,
    opcode: u16,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ARPPacket<const T: usize, const U: usize> {
    header: ARPHeader,
    sender_hardware_address: [u8; T],
    sender_protocol_address: [u8; U],
    target_hardware_address: [u8; T],
    target_protocol_address: [u8; U],
}

const ETHERNET_HARDWARE_LENGTH_USIZE: usize = ETHERNET_ADDRESS_LENGTH as usize;
const IP_PROTOCOL_LENGTH_USIZE: usize = IP_ADDRESS_LENGTH as usize;
type ARPEthernetIPPacket = ARPPacket<ETHERNET_HARDWARE_LENGTH_USIZE, IP_PROTOCOL_LENGTH_USIZE>;

const ARP_CACHE_TIMEOUT_SECONDS: u64 = 30;
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum ARPCacheState {
    Free,
    Incomplete,
    Resolved,
    Static,
}
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ARPCacheEntry<const T: usize, const U: usize> {
    hardware_address: [u8; T],
    protocol_address: [u8; U],
    state: ARPCacheState,
    timeout: u64,
}
type ARPEthernetIPCacheEntry =
    ARPCacheEntry<ETHERNET_HARDWARE_LENGTH_USIZE, IP_PROTOCOL_LENGTH_USIZE>;

#[derive(Debug)]
struct ARPContext<const T: usize, const U: usize> {
    cache: RwLock<HashMap<[u8; U], ARPCacheEntry<T, U>>>,
}
type ARPEthernetIPContext = ARPContext<ETHERNET_HARDWARE_LENGTH_USIZE, IP_PROTOCOL_LENGTH_USIZE>;
impl<const T: usize, const U: usize> ARPContext<T, U> {
    fn new() -> Self {
        ARPContext {
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn lookup(&self, protocol_address: [u8; U]) -> Result<Option<[u8; T]>> {
        let cache = self
            .cache
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        if let Some(entry) = cache.get(&protocol_address) {
            if entry.state != ARPCacheState::Free {
                return Ok(Some(entry.hardware_address));
            }
        }
        Ok(None)
    }

    fn insert(&self, hardware_address: [u8; T], protocol_address: [u8; U]) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?;
        cache.insert(
            protocol_address,
            ARPCacheEntry {
                hardware_address,
                protocol_address,
                state: ARPCacheState::Resolved,
                timeout: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
                    + ARP_CACHE_TIMEOUT_SECONDS,
            },
        );
        Ok(())
    }

    fn update(&self, hardware_address: [u8; T], protocol_address: [u8; U]) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?;
        if let Some(entry) = cache.get_mut(&protocol_address) {
            entry.hardware_address = hardware_address;
            entry.state = ARPCacheState::Resolved;
            entry.timeout =
                SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + ARP_CACHE_TIMEOUT_SECONDS;
        }
        Ok(())
    }

    fn delete(&self, protocol_address: [u8; U]) -> Result<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?;
        if let Some(entry) = cache.get_mut(&protocol_address) {
            entry.hardware_address = [0; T];
            entry.state = ARPCacheState::Free;
            entry.timeout = 0;
        }
        Ok(())
    }
}
