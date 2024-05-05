use std::{process, thread, time::Duration};

use anyhow::Result;
use rust_tcp_ip_stack::{
    ip::IPPacket,
    net::{LoopbackNetDevice, NetDeviceContext, NetDeviceType, NET_PROTOCOL_IP},
};
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

    // test
    let packet: Vec<u8> = vec![
        0x45, 0x00, 0x00, 0x14, 0x00, 0x01, 0x40, 0x00, 0x40, 0x06, 0x00, 0x00, 0xc0, 0xa8, 0x01,
        0x01, 0xc0, 0xa8, 0x01, 0x02,
    ];
    println!("{:?}", IPPacket::parse(packet)?);
    let packet: Vec<u8> = vec![
        0x46,       // バージョン4, ヘッダ長6
        0b10111000, // ToS: 優先度5, D=1, T=1, R=1
        0x00, 0x37, // 全長
        0x00, 0x01, // 識別子
        0b00100000, 0x64, // フラグ: MF, フラグメントオフセット100
        0x40, // TTL
        0x06, // プロトコル (TCP)
        0x00, 0x00, // チェックサム (再計算が必要)
        0xc0, 0xa8, 0x01, 0x01, // 送信元IPアドレス
        0xc0, 0xa8, 0x01, 0x02, // 宛先IPアドレス
        0x01, 0x02, 0x03, 0x04, // オプション (例としてノップ)
        0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        0x21, // データ
    ];
    println!("{:?}", IPPacket::parse(packet)?);

    thread::sleep(Duration::from_secs(1));
    loop {
        net_device_context.transmit(0, NET_PROTOCOL_IP, "hello".to_string())?;
        thread::sleep(Duration::from_secs(1));
    }
}
