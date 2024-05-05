use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc, Mutex, RwLock},
};

use anyhow::Result;
use log::{debug, error, info};

use crate::irq::{raise_irq, IRQContext};

const DUMMY_IRQ: i32 = 35;
const LOOPBACK_IRQ: i32 = 36;

pub struct NetDeviceContext {
    current_index: AtomicU32,
    net_devices: RwLock<Vec<RwLock<NetDevice>>>,
    irq_device_map: RwLock<HashMap<i32, u32>>,
    irq_context: RwLock<IRQContext>,
}

impl NetDeviceContext {
    pub fn new() -> Result<Arc<NetDeviceContext>> {
        let context = Arc::new(NetDeviceContext {
            current_index: AtomicU32::new(0),
            net_devices: RwLock::new(Vec::new()),
            irq_device_map: RwLock::new(HashMap::new()),
            irq_context: RwLock::new(IRQContext::new()),
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
    pub fn register(&self, net_device_type: NetDeviceType) -> Result<()> {
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
        let net_device = NetDevice::new(name, net_device_type);
        self.net_devices
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
            .push(RwLock::new(net_device));
        self.current_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
    pub fn transmit(&self, index: u32, data: String) -> Result<()> {
        if let Some(net_device) = self
            .net_devices
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?
            .get(index as usize)
        {
            net_device
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                .transmit(data)?;
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
}

struct NetDevice {
    name: String,
    net_device_type: NetDeviceType,
    flags: u16,
}
impl NetDevice {
    const FLAG_UP: u16 = 0x0001;

    pub fn new(name: String, net_device_type: NetDeviceType) -> NetDevice {
        NetDevice {
            name,
            net_device_type,
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
    pub fn transmit(&mut self, data: String) -> Result<()> {
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
                queue.push(data);
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
                // pop all
                while let Some(data) = queue.pop() {
                    debug!(
                        "queue popped (num:{}), dev={}, type={:?}, len={}",
                        queue.len(),
                        self.name,
                        self.net_device_type,
                        data.len()
                    );
                    debug!("data={}", data);
                    self.input(data)?;
                }
            }
        }
        Ok(())
    }
    fn input(&self, data: String) -> Result<()> {
        debug!(
            "dev={}, type={:?}, len={}",
            self.name,
            self.net_device_type,
            data.len()
        );
        debug!("data={}", data);
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
    queue: Mutex<Vec<String>>,
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
