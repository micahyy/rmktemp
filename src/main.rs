// src/main.rs
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output};
use embassy_nrf::peripherals::{P0_04, P1_11};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};

// 全局蓝牙状态信号
static BLE_STATE: Signal<CriticalSectionRawMutex, BleState> = Signal::new();

// 蓝牙状态枚举
#[derive(Debug, Clone, Copy)]
enum BleState {
    Disconnected,
    Advertising,
    Connected,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("CZMAOWL17 键盘启动");
    
    // 初始化 nRF52840
    let p = embassy_nrf::init(Default::default());
    
    // ==================== 初始化 NumLock LED 作为蓝牙指示灯 ====================
    // 使用 P1_11 引脚（在 keyboard.toml 中配置的 numslock 引脚）
    let numlock_led = Output::new(p.P1_11, Level::Low);
    
    // ==================== 启动蓝牙指示灯任务 ====================
    spawner.spawn(ble_indicator_task(numlock_led)).unwrap();
    
    // ==================== 模拟蓝牙状态变化（测试用） ====================
    spawner.spawn(ble_state_simulator()).unwrap();
    
    // ==================== 初始化键盘（由 RMK 宏处理） ====================
    // 注意：这里不能直接使用 p，需要让 #[rmk_keyboard] 宏处理
    
    // RMK 键盘初始化会在宏中完成
    info!("键盘初始化将由 RMK 宏处理");
}

/// 蓝牙指示灯任务
#[embassy_executor::task]
async fn ble_indicator_task(mut led: Output<'static, P1_11>) {
    info!("蓝牙指示灯任务启动 (P1_11)");
    
    loop {
        // 等待蓝牙状态变化
        let state = BLE_STATE.wait().await;
        
        match state {
            BleState::Connected => {
                // 已连接：常亮
                info!("蓝牙已连接 - LED 常亮");
                led.set_high();
                
                // 保持常亮直到状态变化
                while BLE_STATE.try_signaler() == Some(&BleState::Connected) {
                    Timer::after(Duration::from_millis(100)).await;
                }
            }
            BleState::Advertising => {
                // 广播中：快闪（250ms 间隔）
                info!("蓝牙广播中 - LED 快闪");
                loop {
                    led.set_high();
                    Timer::after(Duration::from_millis(250)).await;
                    led.set_low();
                    Timer::after(Duration::from_millis(250)).await;
                    
                    // 检查状态是否变化
                    if let Some(new_state) = BLE_STATE.try_signaler() {
                        if *new_state != BleState::Advertising {
                            break;
                        }
                    }
                }
            }
            BleState::Disconnected => {
                // 断开：慢闪（1秒间隔）
                info!("蓝牙断开 - LED 慢闪");
                loop {
                    led.set_high();
                    Timer::after(Duration::from_millis(100)).await;
                    led.set_low();
                    Timer::after(Duration::from_millis(900)).await;
                    
                    // 检查状态是否变化
                    if let Some(new_state) = BLE_STATE.try_signaler() {
                        if *new_state != BleState::Disconnected {
                            break;
                        }
                    }
                }
            }
        }
    }
}

/// 模拟蓝牙状态变化（测试用）
#[embassy_executor::task]
async fn ble_state_simulator() {
    info!("蓝牙状态模拟器启动（测试指示灯）");
    
    // 初始状态：广播中
    BLE_STATE.signal(BleState::Advertising);
    
    // 模拟状态循环
    let states = [
        (Duration::from_secs(5), BleState::Advertising),   // 广播5秒
        (Duration::from_secs(10), BleState::Connected),    // 连接10秒
        (Duration::from_secs(5), BleState::Disconnected),  // 断开5秒
        (Duration::from_secs(3), BleState::Advertising),   // 重新广播
        (Duration::from_secs(8), BleState::Connected),     // 重新连接
    ];
    
    for (duration, state) in states.iter().cycle() {
        BLE_STATE.signal(*state);
        Timer::after(*duration).await;
    }
}

// 以下是真正的蓝牙事件处理函数（需要根据你的蓝牙驱动实现）
// 在实际应用中，这些函数应该由蓝牙驱动调用

/// 当蓝牙连接时调用
pub fn on_ble_connected() {
    info!("蓝牙已连接（真实事件）");
    BLE_STATE.signal(BleState::Connected);
}

/// 当蓝牙断开时调用
pub fn on_ble_disconnected() {
    info!("蓝牙已断开（真实事件）");
    BLE_STATE.signal(BleState::Disconnected);
}

/// 当蓝牙开始广播时调用
pub fn on_ble_advertising_started() {
    info!("蓝牙开始广播（真实事件）");
    BLE_STATE.signal(BleState::Advertising);
}
