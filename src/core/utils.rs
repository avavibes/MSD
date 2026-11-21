use embassy_time::Timer;

/// Used to allow the messages to print over usb instead of fully panicking. 
pub async fn panic_wait(msg: &str) -> !{
    log::error!("===PANIC===");
    log::error!("{}", msg);
    loop {
        Timer::after_millis(1000).await;
    }
}