#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::XskMap,
    programs::XdpContext,
};

#[map]
static XSK_MAP: XskMap = XskMap::with_max_entries(64, 0);

#[xdp]
pub fn flight_redirect(ctx: XdpContext) -> u32 {
    match try_redirect(&ctx) {
        Ok(action) => action,
        Err(_) => xdp_action::XDP_PASS,
    }
}

fn try_redirect(ctx: &XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Ethernet(14) + IPv4(20) + UDP(8) = 42 bytes required
    if data + 42 > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    let buf = data as *const u8;

    unsafe {
        // EtherType == 0x0800 (IPv4)?
        if *buf.add(12) != 0x08 || *buf.add(13) != 0x00 {
            return Ok(xdp_action::XDP_PASS);
        }

        // Protocol == UDP (17)?
        if *buf.add(23) != 17 {
            return Ok(xdp_action::XDP_PASS);
        }
    }

    // redirect to AF_XDP socket
    // flags's lower 2 bits: XDP action to return on redirect failure
    let queue_id = unsafe { (*ctx.ctx).rx_queue_index };
    XSK_MAP
        .redirect(queue_id, xdp_action::XDP_PASS as u64)
        .map_err(|_| ())
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
