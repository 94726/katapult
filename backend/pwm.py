import os
import subprocess
import os.path
from os.path import join


class HardwarePWMException(Exception):
    pass


def echo(message: int, file: str) -> None:
    with open(file, 'w') as f:
        f.write(f'{message}\n')


""" 
There are a few for pwm eligable pins, but each of them requires a slightly different setup.
Optimally one should configure it via a correct dtoverlay, but this dict allows the HardwarePWM to set itself up
see table or bash script at: https://gist.github.com/Gadgetoid/b92ad3db06ff8c264eef2abf0e09d569
"""
GPIO_TO_CHANNEL_MAPPING = {
    19: (3, 'a3'),
    18: (2, 'a3'),
    15: (3, 'a0'),
    14: (2, 'a0'),
    13: (1, 'a0'),
    12: (0, 'a0'),
}


class HardwarePWM:
    """
    Control the hardware PWM on the Raspberry Pi. Requires to first add `dtoverlay=pwm-2chan` to `/boot/firmware/config.txt`.

    pwm0 is GPIO pin 18 is physical pin 32 (dtoverlay can be deployed to use GPIO 12 instead)
    pwm1 is GPIO pin 19 is physical pin 33 (dtoverlay can be deployed to use GPIO 13 instead)

    Example
    ----------
    >pwm = HardwarePWM(0, hz=20)
    >pwm.start(100)
    >
    >pwm.change_duty_cycle(50)
    >pwm.change_frequency(50)
    >
    >pwm.stop()

    Notes
    --------
     - For Rpi 1,2,3,4, use chip=0; For Rpi 5, use chip=2
     - For Rpi 1,2,3,4 only channels 0 and 1 are available
     - If you get "write error: Invalid argument" - you have to set duty_cycle to 0 before changing period
     - /sys/ pwm interface described here: https://jumpnowtek.com/rpi/Using-the-Raspberry-Pi-Hardware-PWM-timers.html

    """

    _duty_cycle: float
    _hz: float
    chippath: str

    def __init__(self, gpio_pin: int, hz: float, chip: int = 2) -> None:
        if gpio_pin not in GPIO_TO_CHANNEL_MAPPING:
            raise HardwarePWMException(
                f'Available gpio pins are: {GPIO_TO_CHANNEL_MAPPING.keys()}; got {gpio_pin} instead'
            )
        self.gpio_pin = gpio_pin
        self.gpio_func = GPIO_TO_CHANNEL_MAPPING[gpio_pin][1]

        self.chippath: str = f'/sys/class/pwm/pwmchip{chip}'
        self.pwm_channel = GPIO_TO_CHANNEL_MAPPING[gpio_pin][0]
        self.pwm_dir = f'{self.chippath}/pwm{self.pwm_channel}'
        self._duty_cycle = 0

        if not self.is_overlay_loaded():
            raise HardwarePWMException(
                "Need to add 'dtoverlay=pwm-2chan' to /boot/config.txt and reboot"
            )
        if not self.is_export_writable():
            raise HardwarePWMException(
                f"Need write access to files in '{self.chippath}'"
            )
        if not self.does_pwmX_exists():
            self.create_pwmX()

        while True:
            try:
                self.change_frequency(hz)
                break
            except PermissionError:
                continue

    def is_overlay_loaded(self) -> bool:
        return os.path.isdir(self.chippath)

    def is_export_writable(self) -> bool:
        return os.access(join(self.chippath, 'export'), os.W_OK)

    def does_pwmX_exists(self) -> bool:
        return os.path.isdir(self.pwm_dir)

    def create_pwmX(self) -> None:
        echo(self.pwm_channel, join(self.chippath, 'export'))

    def start(self, initial_duty_cycle_ms: float) -> None:
        """sets up and starts pwm pin"""
        subprocess.run(['pinctrl', 'set', str(self.gpio_pin), self.gpio_func])
        self.change_duty_cycle(initial_duty_cycle_ms)
        echo(1, join(self.pwm_dir, 'enable'))

    def stop(self) -> None:
        self.change_duty_cycle(0)
        echo(0, join(self.pwm_dir, 'enable'))

    def change_duty_cycle(self, duty_cycle_ms: float) -> None:
        self._duty_cycle = duty_cycle_ms
        dc = int(duty_cycle_ms * 1_000_000)
        echo(dc, join(self.pwm_dir, 'duty_cycle'))

    def change_frequency(self, hz: float) -> None:
        if hz < 0.1:
            raise HardwarePWMException("Frequency can't be lower than 0.1 on the Rpi.")
        self._hz = hz

        # we first have to change duty cycle, since https://stackoverflow.com/a/23050835/1895939
        original_duty_cycle = self._duty_cycle
        if self._duty_cycle:
            self.change_duty_cycle(0)

        per = 1 / float(self._hz)
        per *= 1000  # now in milliseconds
        per *= 1_000_000  # now in nanoseconds
        echo(int(per), join(self.pwm_dir, 'period'))

        self.change_duty_cycle(original_duty_cycle)
