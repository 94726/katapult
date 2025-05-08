use std::collections::VecDeque;
use std::thread;
use std::time::Duration;
use std::{collections::HashMap, time::Instant};

use esp_idf_svc::http::server::ws::EspHttpWsDetachedSender;
use esp_idf_svc::ws::FrameType;
use heapless::mpmc::Q64;

use serde::Serialize;
use serde_json::{json, Value};

use crate::servo::Servo;

const MAGNET_POSITION_ANGLE: i32 = 45;
const SERVO_PROJECTILE_RELEASE_TIME: f64 = 150.0;

#[derive(Serialize)]
pub struct Message {
    pub kind: &'static str,
    pub data: Value,
}

impl Message {
    pub fn rpm(rpm: u128) -> Result<String, serde_json::Error> {
        let msg = Self {
            kind: "RPM_UPDATE",
            data: json!({
                "rpm": rpm
            }),
        };
        serde_json::to_string(&msg)
    }
    pub fn update_inidiated(shot_initiated: bool) -> Result<String, serde_json::Error> {
        let msg = Self {
            kind: "INITIATE_UPDATE",
            data: json!({
                "value": shot_initiated
            }),
        };
        serde_json::to_string(&msg)
    }
}

pub enum Event {
    Broadcast(String),
    AddSession(i32, EspHttpWsDetachedSender),
    RemoveSession(i32),
    TurnServo(Option<i32>),
    InitiateShot(i32),
    HallSensorTrigger(Instant),
}

// Global channel for communicating with the background thread
static mut EVENT_QUEUE: Q64<Event> = Q64::new();

pub fn enqueue(msg: Event) {
    #![allow(static_mut_refs)]
    unsafe {
        EVENT_QUEUE.enqueue(msg).ok();
    }
}

pub fn try_dequeue() -> Option<Event> {
    #![allow(static_mut_refs)]
    unsafe { EVENT_QUEUE.dequeue() }
}

// helper function to immediately broadcast from the state loop
fn broadcast(sessions: &mut HashMap<i32, EspHttpWsDetachedSender>, text: String) {
    sessions.retain(
        |id, sender| match sender.send(FrameType::Text(false), text.as_bytes()) {
            Ok(_) => true,
            Err(e) => {
                println!("Send error to {}: {:?}", id, e);
                false
            }
        },
    );
}

fn turn_servo(servo: &mut Servo) {
    if servo.get_angle() != 90 {
        servo.set_angle(90).unwrap();
    } else {
        servo.set_angle(-90).unwrap();
    }
}

fn toggle_shot_initiated(
    sessions: &mut HashMap<i32, EspHttpWsDetachedSender>,
    shot_initiated: &mut bool,
) {
    *shot_initiated = !*shot_initiated;
    broadcast(
        sessions,
        Message::update_inidiated(*shot_initiated).unwrap(),
    );
}

fn calculate_delay_based_on_rpm_and_target(rpm: u128, target_angle: i32) -> f64 {
    let rps = (rpm as f64) / 60.0; // frequency
    if rps <= 0.0 {
        return 0.0;
    }

    let period_seconds = 1.0 / rps;

    let angle_delta = (target_angle - MAGNET_POSITION_ANGLE).rem_euclid(360);

    let ratio = (angle_delta as f64) / 360.0;

    let delay = (ratio * period_seconds) - SERVO_PROJECTILE_RELEASE_TIME;

    delay.max(0.0)
}

pub fn setup_state_thread(mut servo: Servo<'static>) {
    thread::spawn(move || {
        let mut sessions: HashMap<i32, EspHttpWsDetachedSender> = HashMap::new();

        let mut last_pulse_time = Instant::now();
        let mut intervals: VecDeque<u128> = VecDeque::with_capacity(5); // Rolling average over last 5 pulses
        let mut current_rpm: u128 = 0;

        let mut shot_initiated = false;
        let mut shot_target_angle = 45;

        loop {
            if let Some(event) = try_dequeue() {
                match event {
                    Event::TurnServo(angle) => {
                        if let Some(angle) = angle {
                            servo.set_angle(angle).unwrap()
                        } else {
                            turn_servo(&mut servo);
                        }
                    }
                    Event::InitiateShot(target_angle) => {
                        shot_target_angle = target_angle;
                        toggle_shot_initiated(&mut sessions, &mut shot_initiated);
                    }
                    Event::HallSensorTrigger(now) => {
                        let delta = now.duration_since(last_pulse_time).as_millis();

                        if delta > 0 {
                            if intervals.len() == intervals.capacity() {
                                intervals.pop_front();
                            }
                            intervals.push_back(delta);

                            let avg: u128 =
                                intervals.iter().sum::<u128>() / intervals.len() as u128;
                            current_rpm = 60_000 / avg;
                        }

                        if shot_initiated {
                            thread::sleep(Duration::from_secs_f64(
                                calculate_delay_based_on_rpm_and_target(
                                    current_rpm,
                                    shot_target_angle,
                                ),
                            ));
                            turn_servo(&mut servo);
                            toggle_shot_initiated(&mut sessions, &mut shot_initiated);
                        }
                        println!("Smoothed RPM: {}", current_rpm);
                        broadcast(&mut sessions, Message::rpm(current_rpm).unwrap());

                        last_pulse_time = now;
                    }
                    Event::Broadcast(text) => {
                        broadcast(&mut sessions, text);
                    }
                    Event::AddSession(id, sender) => {
                        sessions.insert(id, sender);
                        println!("Added session: {}", id);
                    }
                    Event::RemoveSession(id) => {
                        sessions.remove(&id);
                        println!("Removed session: {}", id);
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    });
}
