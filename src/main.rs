use std::{
    process,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use log::{debug, error, info};
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

struct DeviceManager {
    current_index: u32,
    devices: Vec<NetDevice>,
}

impl DeviceManager {
    fn new() -> DeviceManager {
        DeviceManager {
            current_index: 0,
            devices: Vec::new(),
        }
    }
    fn net_device_register(&mut self, net_device_type: NetDeviceType) {
        let index = self.current_index;
        let name = format!("net{}", index);
        let dev = NetDevice::new(index, name, net_device_type);
        self.devices.push(dev);
        self.current_index += 1;
    }
    fn net_run(&mut self) -> Result<()> {
        debug!("open all devices...");
        for dev in &mut self.devices {
            dev.open()?;
        }
        debug!("runnning...");
        Ok(())
    }
    fn net_shutdown(&mut self) -> Result<()> {
        debug!("close all devices...");
        for dev in &mut self.devices {
            dev.close()?;
        }
        debug!("shutting donw");
        Ok(())
    }
    fn net_init(&self) {
        info!("initialized");
    }
}

struct NetDevice {
    index: u32,
    name: String,
    net_device_type: NetDeviceType,
    flags: u16,
}
impl NetDevice {
    const FLAG_UP: u16 = 0x0001;

    fn new(index: u32, name: String, net_device_type: NetDeviceType) -> NetDevice {
        NetDevice {
            index,
            name,
            net_device_type,
            flags: 0,
        }
    }
    fn open(&mut self) -> Result<()> {
        if self.is_up() {
            error!("already opened, dev={}", self.name);
            return Err(anyhow!("already opened"));
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
    fn close(&mut self) -> Result<()> {
        if !self.is_up() {
            error!("not opend, dev={}", self.name);
            return Err(anyhow!("not opened"));
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
    fn output(&mut self, data: String) -> Result<()> {
        if !self.is_up() {
            error!("not opened, dev={}", self.name);
            return Err(anyhow!("not opened"));
        }
        if data.len() > self.mtu() as usize {
            error!(
                "too long, dev={}, mtu={}, len={}",
                self.name,
                self.mtu(),
                data.len()
            );
            return Err(anyhow!("too long"));
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
enum NetDeviceType {
    Dummy,
}

fn main() -> Result<()> {
    env_logger::init();

    let mut device_manager = DeviceManager::new();
    device_manager.net_init();
    device_manager.net_device_register(NetDeviceType::Dummy);
    device_manager.net_run()?;

    let device_manager = Arc::new(Mutex::new(device_manager));
    let device_manager_clone = device_manager.clone();

    let mut signals = Signals::new(TERM_SIGNALS)?;
    thread::spawn(move || {
        signals.forever().for_each(|_| {
            device_manager_clone.lock().unwrap().net_shutdown().unwrap();
            process::exit(0);
        });
    });

    loop {
        device_manager
            .lock()
            .map_err(|e| anyhow!("lock failed: {:?}", e))?
            .devices[0]
            .output("hello".to_string())?;
        thread::sleep(Duration::from_secs(1));
    }
}
