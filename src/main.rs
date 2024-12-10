// not on OS , not have std library
#![no_std]
#![no_main]

// logging
use defmt::*;
use defmt_rtt as _;

// async entry point
// use embassy_executor::Spawner;

// sync entry point
use cortex_m_rt::entry;

use k256::elliptic_curve::sec1::ToEncodedPoint;
// panic handler
use panic_probe as _;

// stm32
use embassy_stm32;
use embassy_stm32::{bind_interrupts, peripherals, rng};
use embassy_stm32::rng::Rng;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::time::Hertz;
use embassy_stm32::Config;
use embassy_time::Delay;

use embassy_stm32::fmc::Fmc;

use embedded_alloc::LlffHeap as Heap;
use embedded_hal::delay::DelayNs;

extern crate alloc;

mod is42s32800j;
mod signer;

bind_interrupts!(struct Irqs {
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

const SDRAM_PTR: usize = 0xD000_0000;
const SDRAM_SIZE: usize = 32 * 1024 * 1024;

#[global_allocator]
static HEAP: Heap = Heap::empty();

// use entry point
// use cortex_m_rt::entry;
#[entry]
fn main() -> ! {
    let p = init_stm32();
    info!("Hello, world!");

    mpu_config();

    // sdram init
    let mut sdram = Fmc::sdram_a12bits_d32bits_4banks_bank2(
        p.FMC,
        // A0-A11
        p.PF0, p.PF1, p.PF2, p.PF3, p.PF4, p.PF5, p.PF12, p.PF13, p.PF14, p.PF15, p.PG0, p.PG1,
        // BA0-BA1
        p.PG4, p.PG5,
        // D0-D31
        p.PD14, p.PD15, p.PD0, p.PD1, p.PE7, p.PE8, p.PE9, p.PE10, p.PE11, p.PE12, p.PE13, p.PE14, p.PE15, p.PD8, p.PD9, p.PD10,
        p.PH8, p.PH9, p.PH10, p.PH11, p.PH12, p.PH13, p.PH14, p.PH15, p.PI0, p.PI1, p.PI2, p.PI3, p.PI6, p.PI7, p.PI9, p.PI10,
        // NBL0 - NBL3
        p.PE0, p.PE1, p.PI4, p.PI5,
        p.PH7,  // SDCKE1
        p.PG8,  // SDCLK
        p.PG15, // SDNCAS
        p.PH6,  // SDNE1 (!CS)
        p.PF11, // SDRAS
        p.PH5,  // SDNWE, change to p.PH5 for EVAL boards
        is42s32800j::Is42s32800j {},
    );

    let mut delay = Delay;

    unsafe {
        // Initialise controller and SDRAM
        let ptr: *mut u32 = sdram.init(&mut delay) as *mut _;

        // initialize the allocator
        HEAP.init(ptr as usize, SDRAM_SIZE);
    };

    // we already initialized the sdram and global allocator
    // now we can use heap, alloc memory
    // test_heap();

    let digest = signer::digest(b"hello world");
    let s = hex::encode(digest);
    info!("digest sha(hello world): {}", s.as_str());

    let mut rng = Rng::new(p.RNG, Irqs);

    let key = signer::keygen(&mut rng);

    let s = hex::encode(key.to_bytes());
    info!("private key: {}", s.as_str());

    let b = key.verifying_key().to_encoded_point(false);
    let s = hex::encode(b.as_bytes());
    info!("public key: {}", s.as_str());

    let sig = signer::sign(b"hello world", &key);

    let s = hex::encode(sig);
    info!("signature: {}", s.as_str());

    // stm32h747i-disco red led
    let mut led = Output::new(p.PI14, Level::High, Speed::Low);


    let mut delay = Delay;
    loop {
        led.set_high();
        delay.delay_ms(500);

        led.set_low();
        delay.delay_ms(500);
    }
}

// use async function
// #[embassy_executor::main]
// async fn main(_spawner: Spawner) {

//     let p = init_stm32();
//     info!("Hello, world!");

//     let mut led = Output::new(p.PI14, Level::High, Speed::Low);

//     loop {
//         info!("high");
//         led.set_high();
//         Timer::after_millis(500).await;

//         info!("low");
//         led.set_low();
//         Timer::after_millis(500).await;
//     }
// }

fn init_stm32() -> embassy_stm32::Peripherals {
    let mut config = Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: Hertz::mhz(25),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV5,
            mul: PllMul::MUL160,
            divp: Some(PllDiv::DIV2), // 480 MHz
            divq: Some(PllDiv::DIV8),
            divr: None,
        });
        config.rcc.hsi = None;
        config.rcc.csi = false;
        config.rcc.voltage_scale = VoltageScale::Scale1;
        config.rcc.supply_config = SupplyConfig::DirectSMPS;
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.d1c_pre = AHBPrescaler::DIV1;
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 240 MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 120 MHz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 120 MHz
    }

    let p = embassy_stm32::init(config);
    p
}

fn mpu_config() {
    let mut core_peri = cortex_m::Peripherals::take().unwrap();

    // taken from stm32h7xx-hal
    core_peri.SCB.enable_icache();
    // See Errata Sheet 2.2.1
    // core_peri.SCB.enable_dcache(&mut core_peri.CPUID);
    core_peri.DWT.enable_cycle_counter();
    // ----------------------------------------------------------
    // Configure MPU for external SDRAM
    // MPU config for SDRAM write-through

    {
        let mpu = core_peri.MPU;
        let scb = &mut core_peri.SCB;
        let size = SDRAM_SIZE;
        // Refer to ARM®v7-M Architecture Reference Manual ARM DDI 0403
        // Version E.b Section B3.5
        const MEMFAULTENA: u32 = 1 << 16;

        unsafe {
            /* Make sure outstanding transfers are done */
            cortex_m::asm::dmb();

            scb.shcsr.modify(|r| r & !MEMFAULTENA);

            /* Disable the MPU and clear the control register*/
            mpu.ctrl.write(0);
        }

        const REGION_NUMBER0: u32 = 0x00;
        const REGION_BASE_ADDRESS: u32 = 0xD000_0000;

        const REGION_FULL_ACCESS: u32 = 0x03;
        const REGION_CACHEABLE: u32 = 0x01;
        const REGION_WRITE_BACK: u32 = 0x01;
        const REGION_ENABLE: u32 = 0x01;

        crate::assert_eq!(
            size & (size - 1),
            0,
            "SDRAM memory region size must be a power of 2"
        );
        crate::assert_eq!(
            size & 0x1F,
            0,
            "SDRAM memory region size must be 32 bytes or more"
        );
        fn log2minus1(sz: u32) -> u32 {
            for i in 5..=31 {
                if sz == (1 << i) {
                    return i - 1;
                }
            }
            crate::panic!("Unknown SDRAM memory region size!");
        }

        //info!("SDRAM Memory Size 0x{:x}", log2minus1(size as u32));

        // Configure region 0
        //
        // Cacheable, outer and inner write-back, no write allocate. So
        // reads are cached, but writes always write all the way to SDRAM
        unsafe {
            mpu.rnr.write(REGION_NUMBER0);
            mpu.rbar.write(REGION_BASE_ADDRESS);
            mpu.rasr.write(
                (REGION_FULL_ACCESS << 24)
                    | (REGION_CACHEABLE << 17)
                    | (REGION_WRITE_BACK << 16)
                    | (log2minus1(size as u32) << 1)
                    | REGION_ENABLE,
            );
        }

        const MPU_ENABLE: u32 = 0x01;
        const MPU_DEFAULT_MMAP_FOR_PRIVILEGED: u32 = 0x04;

        // Enable
        unsafe {
            mpu.ctrl
                .modify(|r| r | MPU_DEFAULT_MMAP_FOR_PRIVILEGED | MPU_ENABLE);

            scb.shcsr.modify(|r| r | MEMFAULTENA);

            // Ensure MPU settings take effect
            cortex_m::asm::dsb();
            cortex_m::asm::isb();
        }
    }
}

fn test_heap() {
        // testing heap
    // create a vector with capacity 1000
    let mut v = alloc::vec::Vec::with_capacity(1000);

    // fill the vector
    v.extend((0..1000).into_iter());

    // log the vector var safe rust
    for i in v.iter() {
        info!("safe rust value: {}", i);
    }

    // log the vector unsafe rust, with raw pointer
    unsafe {
        let ptr = SDRAM_PTR as *mut i32;

        for i in 0..1000 {
            info!("unsafe rust value: {}", *ptr.offset(i as isize));
        }

    }
}