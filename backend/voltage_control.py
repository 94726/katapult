from typing import Literal
from .pwm import HardwarePWM


DUTY_CYCLE_POINTS = {"negative": 0.3, "positive": 1.4, "idle": 0.3}


class VoltageControl:
    """
    Uses custom pwm to avoid jitter
    see servo specs: https://www.electronicoscaldas.com/datasheet/MG90S_Tower-Pro.pdf
    """
    def __init__(self, gpio_pin: int, hz=50, chip=2):
        self.pwm = HardwarePWM(gpio_pin=gpio_pin, hz=hz, chip=chip)
        self.start()

    def start(self):
        self.pwm.start(DUTY_CYCLE_POINTS['idle'])

    def close(self):
        self.turn_to('idle')
        self.pwm.stop()

    @property
    def is_positive(self) -> bool:
        return self.pwm._duty_cycle == DUTY_CYCLE_POINTS["positive"]

    @property
    def is_negative(self) -> bool:
        return self.pwm._duty_cycle == DUTY_CYCLE_POINTS["negative"]

    @property
    def is_idle(self) -> bool:
        return self.pwm._duty_cycle == DUTY_CYCLE_POINTS["idle"]

    @property
    def status_num(self) -> int:
        if self.is_positive:
            return 1
        elif self.is_negative:
            return -1
        else:
            return 0

    def turn_to(
        self, target: Literal["positive"] | Literal["negative"] | Literal["idle"]
    ):
        self.pwm.change_duty_cycle(DUTY_CYCLE_POINTS[target])

    def turn(self):
        if self.is_negative:
            return self.turn_to("positive")
        return self.turn_to("negative")
