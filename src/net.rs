use std::sync::{atomic::AtomicU32, RwLock};

use anyhow::Result;
use log::{debug, error, info};

pub struct NetDeviceContext {
    current_index: AtomicU32,
    net_devices: RwLock<Vec<RwLock<NetDevice>>>,
}
impl Default for NetDeviceContext {
    fn default() -> Self {
        NetDeviceContext {
            current_index: AtomicU32::new(0),
            net_devices: RwLock::new(Vec::new()),
        }
    }
}

impl NetDeviceContext {
    pub fn new() -> NetDeviceContext {
        NetDeviceContext::default()
    }
    pub fn net_device_register(&self, net_device_type: NetDeviceType) -> Result<()> {
        let index = self.current_index.load(std::sync::atomic::Ordering::SeqCst);
        let name = format!("net{}", index);
        let net_device = NetDevice::new(name, net_device_type);
        self.net_devices
            .write()
            .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
            .push(RwLock::new(net_device));
        self.current_index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
    pub fn net_run(&self) -> Result<()> {
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
    pub fn net_shutdown(&self) -> Result<()> {
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
        Ok(())
    }
    pub fn net_init(&self) -> Result<()> {
        info!("initialized");
        Ok(())
    }
    pub fn output(&self, index: u32, data: String) -> Result<()> {
        let net_devices = self
            .net_devices
            .read()
            .map_err(|_| anyhow::anyhow!("Failed to read lock"))?;
        if let Some(net_device) = net_devices.get(index as usize) {
            net_device
                .write()
                .map_err(|_| anyhow::anyhow!("Failed to write lock"))?
                .output(data)?;
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
            NetDeviceType::Dummy => {
                // TODO:
            }
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
            NetDeviceType::Dummy => {
                // TODO:
            }
        }
        self.flags &= !Self::FLAG_UP;
        info!("dev={}, state={}", self.name, self.state());
        Ok(())
    }
    pub fn output(&mut self, data: String) -> Result<()> {
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
            NetDeviceType::Dummy => {
                // TODO:
            }
        }
        Ok(())
    }
    fn mtu(&self) -> u16 {
        match &self.net_device_type {
            NetDeviceType::Dummy => u16::MAX,
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
}
