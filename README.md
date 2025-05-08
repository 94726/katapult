# Katapult :)

This is the source code for a school project about a motor-driven, rotating catapult

## 🧩 Structure and Prerequesites

### 1. RaspberryPi setup.
Originally this project was meant to run on a esp32, but mine got fried :/

So heres the RaspberryPi (Pi5) setup:
The only special requirement is to enable the PWM overlay by adding `dtoverlay=pwm-2chan` to `/boot/firmware/config.txt`.

### 2. Backend

The backend can be found inside `/backend` or `/backend-esp32` (not tested).
Python and all of the necessary dependencies are managed using [pdm](https://pdm-project.org/latest/).

### 3. Frontend
To visualize and controll the model, a frontend was made using solid-js.
The frontend can be found inside `/frontend`.

The javascript project is managed using [bun](https://bun.sh/) as package manager.

