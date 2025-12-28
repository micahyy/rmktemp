// src/main.rs
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals, flash::Flash};
use rmk::keyboard::{Keyboard, KeyboardConfig};

// RMK 会自动生成配置
use keyboard_config::*;

bind_interrupts!(struct Irqs {
    POWER_CLOCK => embassy_nrf::power::InterruptHandler;
    // 其他中断...
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("键盘固件启动");
    
    let p = embassy_nrf::init(Default::default());
    
    // ============== EEPROM 清除部分 ==============
    // 方法1：总是清除（开发用）
    // force_clear_eeprom(&p).await;
    
    // 方法2：条件清除
    if should_clear_eeprom(&p).await {
        info!("正在清除 EEPROM...");
        if let Err(e) = clear_eeprom(&p).await {
            error!("清除失败: {:?}", e);
        } else {
            info!("清除成功，请重新上电");
            // 清除后停止运行，需要重新上电
            loop {
                cortex_m::asm::wfi();
            }
        }
    }
    // ===========================================
    
    // 正常初始化键盘
    let keyboard_config = KeyboardConfig::default();
    let keyboard = Keyboard::new(keyboard_config);
    
    info!("键盘初始化完成");
    keyboard.run().await;
}

/// 强制清除 EEPROM
async fn force_clear_eeprom(p: &peripherals::Peripherals) {
    info!("强制清除 EEPROM");
    
    // 使用 RMK 配置的存储地址
    // 这些值应该与 keyboard.toml 中的 storage 部分匹配
    const EEPROM_START: u32 = 0x000F_0000;
    const EEPROM_SIZE: u32 = 0x0000_4000;  // 16KB
    
    let mut flash = Flash::new();
    
    // 擦除每个扇区（4KB）
    for addr in (EEPROM_START..EEPROM_START + EEPROM_SIZE).step_by(4096) {
        info!("擦除地址: 0x{:08X}", addr);
        
        match flash.erase(addr, addr + 4096).await {
            Ok(_) => info!("擦除成功"),
            Err(e) => {
                error!("擦除失败: {:?}", e);
                break;
            }
        }
    }
}

/// 条件清除：检查是否需要清除
async fn should_clear_eeprom(p: &peripherals::Peripherals) -> bool {
    use embassy_nrf::gpio::{Input, Pull};
    use embassy_time::{Duration, Timer};
    
    // 使用 BOOT 按钮（通常连接到 P0.11 或 P0.13）
    // 检查你的原理图确认按钮引脚
    let clear_button = Input::new(p.P0_11, Pull::Up);
    
    info!("按住 BOOT 按钮 3 秒可清除 EEPROM");
    
    // 检查按钮是否被按住
    for _ in 0..30 {  // 3 秒 = 30 * 100ms
        if clear_button.is_low() {
            Timer::after(Duration::from_millis(100)).await;
        } else {
            return false;
        }
    }
    
    info!("检测到清除请求");
    true
}

/// 清除 EEPROM 的具体实现
async fn clear_eeprom(p: &peripherals::Peripherals) -> Result<(), &'static str> {
    const FLASH_PAGES: u32 = 4;  // 要擦除的页数（每页 4KB）
    
    let mut flash = Flash::new();
    
    // 从 Flash 末尾开始擦除
    // 注意：不要擦除前 0x10000 区域（包含 Bootloader）
    let flash_size = 0x80000;  // nRF52840 有 512KB Flash
    let page_size = 4096;
    
    // 计算要擦除的区域（最后几页）
    let start_page = (flash_size / page_size) - FLASH_PAGES;
    let start_addr = start_page * page_size;
    
    info!("擦除 Flash: 0x{:08X} - 0x{:08X}", 
          start_addr, start_addr + FLASH_PAGES * page_size);
    
    for i in 0..FLASH_PAGES {
        let addr = start_addr + i * page_size;
        match flash.erase(addr, addr + page_size).await {
            Ok(_) => info!("页 {} 擦除成功", i),
            Err(_) => return Err("Flash 擦除失败"),
        }
    }
    
    Ok(())
}
