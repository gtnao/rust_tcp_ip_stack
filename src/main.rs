use std::{process, thread, time::Duration};

use anyhow::Result;
use rust_tcp_ip_stack::net::{LoopbackNetDevice, NetDeviceContext, NetDeviceType, NET_PROTOCOL_IP};
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

fn main() -> Result<()> {
    env_logger::init();

    let net_device_context = NetDeviceContext::new()?;
    net_device_context.init()?;
    // net_device_context.register(NetDeviceType::Dummy)?;
    net_device_context.register(
        NetDeviceType::Loopback(LoopbackNetDevice::new()),
        net_device_context.clone(),
    )?;
    net_device_context.register_protocol(NET_PROTOCOL_IP)?;
    net_device_context.run()?;

    let net_device_context_clone = net_device_context.clone();
    let mut signals = Signals::new(TERM_SIGNALS)?;
    thread::spawn(move || {
        signals.forever().for_each(|_| {
            net_device_context_clone.shutdown().unwrap();
            process::exit(0);
        });
    });

    thread::sleep(Duration::from_secs(1));
    loop {
        net_device_context.transmit(0, NET_PROTOCOL_IP, "hello".to_string())?;
        thread::sleep(Duration::from_secs(1));
    }
}
