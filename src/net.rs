use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc, Mutex, RwLock},
};

use anyhow::Result;
use log::{debug, error, info};
use signal_hook::consts::SIGUSR1;

use crate::irq::{raise_irq, IRQContext};

const DUMMY_IRQ: i32 = 35;
const LOOPBACK_IRQ: i32 = 36;

pub const NET_PROTOCOL_IP: u16 = 0x0800;

pub struct NetDeviceContext {
    current_index: AtomicU32,
    net_devices: RwLock<Vec<RwLock<NetDevice>>>,
    irq_device_map: RwLock<HashMap<i32, u32>>,
    irq_context: RwLock<IRQContext>,
    protocols: RwLock<Vec<NetProtocol>>,
}

impl NetDeviceContext {
    pub fn new() -> Result<Arc<NetDeviceContext>> {
        let context = Arc::new(NetDeviceContext {
            current_index: AtomicU32::new(0),
            net_devices: RwLock::new(Vec::new()),
            irq_device_map: RwLock::new(HashMap::new()),
            irq_context: RwLock::new(IRQContext::new()),
            protocols: RwLock::new(Vec::new()),
        });
        context
            .irq_context
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
            .set_net_device_context(context.clone());
        Ok(context)
    }
    pub fn init(&self) -> Result<()> {
        self.irq_context
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .init()?;
        info!("initialized");
        Ok(())
    }
    pub fn register(
        &self,
        net_device_type: NetDeviceType,
        context: Arc<NetDeviceContext>,
    ) -> Result<()> {
        let index = self.current_index.load(std::sync::atomic::Ordering::SeqCst);
        let name = format!("net{}", index);
        match net_device_type {
            NetDeviceType::Dummy => {
                self.irq_context
                    .read()
                    .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
                    .register(DUMMY_IRQ)?;
                self.irq_device_map
                    .write()
                    .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                    .insert(DUMMY_IRQ, index);
            }
            NetDeviceType::Loopback(_) => {
                self.irq_context
                    .read()
                    .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
                    .register(LOOPBACK_IRQ)?;
                self.irq_device_map
                    .write()
                    .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                    .insert(LOOPBACK_IRQ, index);
            }
        }
        let net_device = NetDevice::new(name, net_device_type, context);
        self.net_devices
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
            .push(RwLock::new(net_device));
        self.current_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    pub fn register_protocol(&self, protocol_type: u16) -> Result<()> {
        let protocol = NetProtocol {
            protocol_type,
            queue: Mutex::new(Vec::new()),
        };
        self.protocols
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
            .push(protocol);
        Ok(())
    }
    pub fn run(&self) -> Result<()> {
        self.irq_context
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .run()?;
        debug!("open all devices...");
        let net_devices = self
            .net_devices
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        for net_device in &*net_devices {
            net_device
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                .open()?;
        }
        debug!("runnning...");
        Ok(())
    }
    pub fn shutdown(&self) -> Result<()> {
        debug!("close all devices...");
        let net_devices = self
            .net_devices
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        for net_device in &*net_devices {
            net_device
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                .close()?;
        }
        debug!("shutting donw");
        self.irq_context
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .shutdown()?;
        Ok(())
    }
    pub fn transmit(&self, index: u32, net_protocol_type: u16, data: String) -> Result<()> {
        if let Some(net_device) = self
            .net_devices
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .get(index as usize)
        {
            net_device
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                .transmit(net_protocol_type, data)?;
        }
        Ok(())
    }
    pub fn isr(&self, irq: i32) -> Result<()> {
        if let Some(net_device_index) = self
            .irq_device_map
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .get(&irq)
        {
            if let Some(net_device) = self
                .net_devices
                .read()
                .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
                .get(*net_device_index as usize)
            {
                net_device
                    .write()
                    .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                    .isr(irq)?;
            }
        }
        Ok(())
    }
    pub fn software_isr(&self) -> Result<()> {
        let protocols = self
            .protocols
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        for protocol in &*protocols {
            while let Some(data) = protocol
                .queue
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock"))?
                .pop()
            {
                match protocol.protocol_type {
                    NET_PROTOCOL_IP => {
                        debug!("software isr, protocol=IP, data={}", data);
                    }
                    _ => {
                        error!(
                            "software isr, unknown protocol, type={}",
                            protocol.protocol_type
                        );
                    }
                }
            }
        }
        Ok(())
    }
    pub fn input(&self, protocol_type: u16, data: String) -> Result<()> {
        let protocols = self
            .protocols
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        for protocol in &*protocols {
            if protocol.protocol_type == protocol_type {
                protocol
                    .queue
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock"))?
                    .push(data);
                raise_irq(SIGUSR1)?;
                break;
            }
        }
        Ok(())
    }
}

struct NetDevice {
    name: String,
    net_device_type: NetDeviceType,
    net_device_context: Arc<NetDeviceContext>,
    flags: u16,
}
impl NetDevice {
    const FLAG_UP: u16 = 0x0001;

    pub fn new(
        name: String,
        net_device_type: NetDeviceType,
        net_device_context: Arc<NetDeviceContext>,
    ) -> NetDevice {
        NetDevice {
            name,
            net_device_type,
            net_device_context,
            flags: 0,
        }
    }
    pub fn open(&mut self) -> Result<()> {
        if self.is_up() {
            error!("already opened, dev={}", self.name);
            return Err(anyhow::anyhow!("already opened"));
        }
        match &self.net_device_type {
            NetDeviceType::Dummy => {}
            NetDeviceType::Loopback(_) => {}
        }
        self.flags |= Self::FLAG_UP;
        info!("dev={}, state={}", self.name, self.state());
        Ok(())
    }
    pub fn close(&mut self) -> Result<()> {
        if !self.is_up() {
            error!("not opend, dev={}", self.name);
            return Err(anyhow::anyhow!("not opened"));
        }
        match &self.net_device_type {
            NetDeviceType::Dummy => {}
            NetDeviceType::Loopback(_) => {}
        }
        self.flags &= !Self::FLAG_UP;
        info!("dev={}, state={}", self.name, self.state());
        Ok(())
    }
    pub fn transmit(&mut self, net_protocol_type: u16, data: String) -> Result<()> {
        if !self.is_up() {
            error!("not opened, dev={}", self.name);
            return Err(anyhow::anyhow!("not opened"));
        }
        if data.len() > self.mtu() as usize {
            error!(
                "too long, dev={}, mtu={}, len={}",
                self.name,
                self.mtu(),
                data.len()
            );
            return Err(anyhow::anyhow!("too long"));
        }
        debug!(
            "dev={}, type={:?}, len={}",
            self.name,
            self.net_device_type,
            data.len()
        );
        debug!("data={}", data);
        match &self.net_device_type {
            NetDeviceType::Dummy => raise_irq(DUMMY_IRQ)?,
            NetDeviceType::Loopback(net_device) => {
                let mut queue = net_device
                    .queue
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock"))?;
                queue.push(LoopbackNetDeviceQueueEntry {
                    net_protocol_type,
                    data,
                });
                debug!(
                    "queue pushed (num:{}), dev={}, type={:?}",
                    queue.len(),
                    self.name,
                    self.net_device_type,
                );
                raise_irq(LOOPBACK_IRQ)?
            }
        }
        Ok(())
    }
    pub fn isr(&mut self, irq: i32) -> Result<()> {
        debug!("dev={}, irq={}", self.name, irq);
        match &self.net_device_type {
            NetDeviceType::Dummy => {}
            NetDeviceType::Loopback(net_device) => {
                let mut queue = net_device
                    .queue
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Failed to lock"))?;
                while let Some(entry) = queue.pop() {
                    debug!(
                        "queue popped (num:{}), dev={}, type={:?}, len={}",
                        queue.len(),
                        self.name,
                        entry.net_protocol_type,
                        entry.data.len()
                    );
                    debug!("data={}", entry.data);
                    self.net_device_context
                        .input(entry.net_protocol_type, entry.data)?;
                }
            }
        }
        Ok(())
    }
    fn mtu(&self) -> u16 {
        match &self.net_device_type {
            NetDeviceType::Dummy => u16::MAX,
            NetDeviceType::Loopback(_) => u16::MAX,
        }
    }
    fn is_up(&self) -> bool {
        self.flags & Self::FLAG_UP != 0
    }
    fn state(&self) -> String {
        if self.is_up() {
            "up".to_string()
        } else {
            "down".to_string()
        }
    }
}

#[derive(Debug)]
pub enum NetDeviceType {
    Dummy,
    Loopback(LoopbackNetDevice),
}

#[derive(Debug)]
pub struct LoopbackNetDevice {
    queue: Mutex<Vec<LoopbackNetDeviceQueueEntry>>,
}
impl Default for LoopbackNetDevice {
    fn default() -> Self {
        LoopbackNetDevice {
            queue: Mutex::new(Vec::new()),
        }
    }
}
impl LoopbackNetDevice {
    pub fn new() -> LoopbackNetDevice {
        Self::default()
    }
}
#[derive(Debug)]
struct LoopbackNetDeviceQueueEntry {
    net_protocol_type: u16,
    data: String,
}

struct NetProtocol {
    protocol_type: u16,
    queue: Mutex<Vec<String>>,
}
