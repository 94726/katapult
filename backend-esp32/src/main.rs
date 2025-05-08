#![feature(path_add_extension)]

mod config;
mod server;
mod servo;
mod state;

use anyhow::{self};
use embedded_svc::http::Method;
use embedded_svc::wifi::{AccessPointConfiguration, AuthMethod, Configuration::AccessPoint};
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::ledc::config::TimerConfig;
use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::io::EspIOError;
use esp_idf_svc::http::server::{Configuration, EspHttpServer};
use esp_idf_svc::io::vfs::MountedLittlefs;
use esp_idf_svc::mdns::EspMdns;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;

use esp_idf_svc::fs::littlefs::Littlefs;

use esp_idf_hal::prelude::*;
use server::server_handle_ui;
use std::ffi::CString;
use std::str::FromStr;
use std::time::Instant;
use std::{thread::sleep, time::Duration};

use esp_idf_hal::gpio;
use esp_idf_hal::ledc::*;
use esp_idf_hal::peripheral::Peripheral;

use serde::Deserialize;

#[derive(Deserialize)]
struct AngleRequest {
    angle: i32,
}

fn spawn_hall_thread<T: gpio::InputPin>(pin: impl Peripheral<P = T> + 'static) {
    let do_pin = PinDriver::input(pin).unwrap();
    let mut last_state = false;

    // todo maybe move into start of state loop, so that enqueued msgs instantly get transmitted
    std::thread::spawn(move || loop {
        let current = do_pin.is_low();

        // leading edge
        if current && !last_state {
            state::enqueue(state::Event::HallSensorTrigger(Instant::now()));

            println!("Magnet detected (leading edge)!");
        }
        // falling edge
        if !current && last_state {
            println!("Magnet removed (trailing edge)!");
        }

        last_state = current;
        std::thread::sleep(Duration::from_millis(10));
    });
}

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs))?;
    let config = config::Config::load()?;

    wifi.set_configuration(&AccessPoint(AccessPointConfiguration {
        ssid: heapless::String::from_str(config.wifi_ssid)
            .map_err(|_| anyhow::anyhow!("SSID is too long."))?,
        password: heapless::String::from_str(config.wifi_pass)
            .map_err(|_| anyhow::anyhow!("Wifi password is too long."))?,
        auth_method: AuthMethod::WPA2WPA3Personal,
        ..Default::default()
    }))?;

    let mut mdns = EspMdns::take().expect("Could not get mdns");

    mdns.set_hostname("katapult").expect("failed hostname");
    wifi.start()?;

    let littlefs: Littlefs<CString>;
    let _mounted_littlefs: MountedLittlefs<Littlefs<CString>>;

    unsafe {
        littlefs = Littlefs::new_partition("lfs")?;
        _mounted_littlefs = MountedLittlefs::mount(littlefs, "/frontend")?;
    }

    let ledc_timer = LedcTimerDriver::new(
        peripherals.ledc.timer0,
        &TimerConfig::default()
            .frequency(50.Hz())
            .resolution(Resolution::Bits14),
    )?;

    let pwm = LedcDriver::new(
        peripherals.ledc.channel0,
        ledc_timer,
        peripherals.pins.gpio32,
    )?;

    state::setup_state_thread(servo::Servo::standard(pwm)?);
    spawn_hall_thread(peripherals.pins.gpio14);

    let mut server = EspHttpServer::new(&Configuration {
        uri_match_wildcard: true,
        ..Default::default()
    })?;

    server.fn_handler(
        "/api/trigger/initiate",
        Method::Post,
        move |mut request| -> core::result::Result<(), EspIOError> {
            let mut buf = [0u8; 128];
            let read = request.read(&mut buf)?;
            let body = &buf[..read];

            let body: AngleRequest = match serde_json::from_slice(body) {
                Ok(val) => val,
                Err(_e) => {
                    let _ = request.into_status_response(400)?;
                    return Ok(());
                }
            };

            state::enqueue(state::Event::InitiateShot(body.angle));
            request.into_ok_response()?;
            Ok(())
        },
    )?;
    server.fn_handler(
        "/api/trigger/turn",
        Method::Post,
        move |request| -> core::result::Result<(), EspIOError> {
            state::enqueue(state::Event::TurnServo(None));
            request.into_ok_response()?;
            Ok(())
        },
    )?;
    server.fn_handler(
        "/api/reset",
        Method::Post,
        move |request| -> core::result::Result<(), EspIOError> {
            state::enqueue(state::Event::TurnServo(Some(90)));
            request.into_ok_response()?;
            Ok(())
        },
    )?;

    server_handle_ui(&mut server)?;

    println!("Server awaiting connection");

    loop {
        sleep(Duration::from_millis(1000));
    }
}
