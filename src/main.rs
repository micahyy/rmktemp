
// src/main.rs
#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals, gpio::Output};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use core::sync::atomic::{AtomicU8, Ordering};

// 蓝牙状态枚举
#[derive(defmt::Format)]
enum BleState {
    Disconnected,    // 未连接
    Advertising,     // 广播中
    Connected,       // 已连接
    LowBattery,      // 低电量
}

// 全局蓝牙状态
static BLE_STATE: AtomicU8 = AtomicU8::new(0); // 0=断开, 1=广播, 2=连接, 3=低电

bind_interrupts!(struct Irqs {
    POWER_CLOCK => embassy_nrf::power::InterruptHandler;
    // 根据你的需求添加其他中断
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("CZMAOWL17 键盘启动");
    
    let p = embassy_nrf::init(Default::default());
    
    // ==================== 初始化 NumLock LED ====================
    // 根据你的 keyboard.toml: numslock.pin = "P1_11"
    let mut numlock_led = Output::new(p.P1_11, embassy_nrf::gpio::Level::Low);
    info!("NumLock LED 初始化完成 (P1_11)");
    
    // ==================== 启动蓝牙状态指示灯任务 ====================
    _spawner.spawn(ble_indicator_task(numlock_led)).unwrap();
    
    // ==================== 模拟蓝牙状态变化（测试用） ====================
    // 实际使用时，这里应该接收真正的蓝牙事件
    _spawner.spawn(ble_simulator_task()).unwrap();
    
    // ==================== 正常键盘初始化 ====================
    // 这里初始化键盘矩阵、配置等
    info!("键盘初始化完成");
    
    // 主循环保持运行
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// 蓝牙状态指示灯任务
#[embassy_executor::task]
async fn ble_indicator_task(mut led: Output<'static>) {
    info!("蓝牙指示灯任务启动");
    
    loop {
        let state = BLE_STATE.load(Ordering::SeqCst);
        let ble_state = match state {
            0 => BleState::Disconnected,
            1 => BleState::Advertising,
            2 => BleState::Connected,
            3 => BleState::LowBattery,
            _ => BleState::Disconnected,
        };
        
        match ble_state {
            BleState::Disconnected => {
                // 断开状态：慢闪（亮100ms，灭900ms）
                led.set_high();
                Timer::after(Duration::from_millis(100)).await;
                led.set_low();
                Timer::after(Duration::from_millis(900)).await;
            }
            
            BleState::Advertising => {
                // 广播状态：快闪（亮250ms，灭250ms）
                led.set_high();
                Timer::after(Duration::from_millis(250)).await;
                led.set_low();
                Timer::after(Duration::from_millis(250)).await;
            }
            
            BleState::Connected => {
                // 连接状态：常亮
                led.set_high();
                Timer::after(Duration::from_secs(1)).await;
            }
            
            BleState::LowBattery => {
                // 低电量：急促闪烁3次后暂停
                for _ in 0..3 {
                    led.set_high();
                    Timer::after(Duration::from_millis(100)).await;
                    led.set_low();
                    Timer::after(Duration::from_millis(100)).await;
                }
                Timer::after(Duration::from_millis(1000)).await;
            }
        }
    }
}

/// 模拟蓝牙状态变化（测试用）
/// 实际使用时应该用真正的蓝牙事件替换
#[embassy_executor::task]
async fn ble_simulator_task() {
    info!("蓝牙模拟器启动（用于测试指示灯）");
    
    // 初始状态：广播中
    BLE_STATE.store(1, Ordering::SeqCst);
    
    // 模拟状态变化
    let states = [
        (Duration::from_secs(5), 1),  // 广播5秒
        (Duration::from_secs(10), 2), // 连接10秒
        (Duration::from_secs(5), 0),  // 断开5秒
        (Duration::from_secs(3), 3),  // 低电量3秒
        (Duration::from_secs(5), 1),  // 重新广播
        (Duration::from_secs(5), 2),  // 重新连接
    ];
    
    for &(duration, state) in states.iter().cycle() {
        BLE_STATE.store(state, Ordering::SeqCst);
        let state_str = match state {
            0 => "断开",
            1 => "广播",
            2 => "连接",
            3 => "低电",
            _ => "未知",
        };
        info!("蓝牙状态: {}", state_str);
        Timer::after(duration).await;
    }
}

/// 实际的蓝牙事件处理函数
/// 当收到真正的蓝牙事件时调用这个函数
fn handle_ble_event(event: BleEvent) {
    match event {
        BleEvent::Connected(_) => {
            BLE_STATE.store(2, Ordering::SeqCst);
            info!("蓝牙已连接");
        }
        BleEvent::Disconnected(_) => {
            BLE_STATE.store(0, Ordering::SeqCst);
            info!("蓝牙已断开");
        }
        BleEvent::AdvertisingStarted => {
            BLE_STATE.store(1, Ordering::SeqCst);
            info!("蓝牙开始广播");
        }
        BleEvent::BatteryLow => {
            BLE_STATE.store(3, Ordering::SeqCst);
            info!("电池电量低");
        }
        _ => {}
    }
}

// 蓝牙事件枚举（需要根据实际的蓝牙驱动定义）
#[derive(defmt::Format)]
enum BleEvent {
    Connected(u16),
    Disconnected(u16),
    AdvertisingStarted,
    BatteryLow,
    // 其他事件...
}
