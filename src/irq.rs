use anyhow::Result;
use log::info;
use signal_hook::{consts::SIGHUP, iterator::Signals, low_level};
use std::{
    sync::{Arc, RwLock, Weak},
    thread,
};

use crate::net::NetDeviceContext;

pub struct IRQEntry {
    irq: i32,
}

pub struct IRQContext {
    net_device_context: Weak<NetDeviceContext>,
    irq_entries: Arc<RwLock<Vec<RwLock<IRQEntry>>>>,
}
impl Default for IRQContext {
    fn default() -> Self {
        IRQContext {
            net_device_context: Weak::new(),
            irq_entries: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl IRQContext {
    const AVAILABLE_IRQ_MIN: i32 = 35;
    const AVAILABLE_IRQ_MAX: i32 = 64;
    pub fn new() -> IRQContext {
        Self::default()
    }
    pub fn set_net_device_context(&mut self, net_device_context: Arc<NetDeviceContext>) {
        self.net_device_context = Arc::downgrade(&net_device_context);
    }
    pub fn init(&self) -> Result<()> {
        info!("initialized");
        Ok(())
    }
    pub fn register(&self, irq: i32) -> Result<()> {
        let irq_entry = IRQEntry { irq };
        self.irq_entries
            .write()
            .unwrap()
            .push(RwLock::new(irq_entry));
        Ok(())
    }
    pub fn run(&self) -> Result<()> {
        let available_irqs =
            (Self::AVAILABLE_IRQ_MIN..Self::AVAILABLE_IRQ_MAX).collect::<Vec<i32>>();
        let mut signal_list = vec![SIGHUP];
        signal_list.extend(&available_irqs);
        let mut signals = Signals::new(&signal_list)?;
        let net_device_context_clone = self.net_device_context.upgrade();
        let irq_entries_clone = self.irq_entries.clone();
        thread::spawn(move || {
            for signal in signals.forever() {
                if signal == SIGHUP {
                    break;
                } else {
                    for irq_entry in irq_entries_clone.read().unwrap().iter() {
                        if irq_entry.read().unwrap().irq == signal {
                            if let Some(net_device_context) = &net_device_context_clone {
                                net_device_context.isr(signal).unwrap();
                            }
                        }
                    }
                }
            }
        });
        Ok(())
    }
    pub fn shutdown(&self) -> Result<()> {
        // TODO:
        info!("shutdown");
        Ok(())
    }
}

pub fn raise_irq(irq: i32) -> Result<()> {
    low_level::raise(irq)?;
    Ok(())
}
