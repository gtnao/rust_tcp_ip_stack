use std::{process, sync::Arc, thread, time::Duration};

use anyhow::Result;
use rust_tcp_ip_stack::net::{NetDeviceContext, NetDeviceType};
use signal_hook::{consts::TERM_SIGNALS, iterator::Signals};

fn main() -> Result<()> {
    env_logger::init();

    let net_device_context = Arc::new(NetDeviceContext::new());
    net_device_context.net_init()?;
    net_device_context.net_device_register(NetDeviceType::Dummy)?;
    net_device_context.net_run()?;

    let net_device_context_clone = net_device_context.clone();

    let mut signals = Signals::new(TERM_SIGNALS)?;
    thread::spawn(move || {
        signals.forever().for_each(|_| {
            net_device_context_clone.net_shutdown().unwrap();
            process::exit(0);
        });
    });

    loop {
        net_device_context.output(0, "hello".to_string())?;
        thread::sleep(Duration::from_secs(1));
    }
}
