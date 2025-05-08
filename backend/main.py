from __future__ import annotations
import asyncio
from contextlib import asynccontextmanager
from pathlib import Path
import time
from typing import TYPE_CHECKING, Optional
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from gpiozero.mixins import deque

from .fastapi_utils import BaseSchema, WebsocketManager
from fastapi import FastAPI, Request, WebSocket, WebSocketDisconnect
from gpiozero import DigitalInputDevice
import warnings

from .voltage_control import VoltageControl

SENSOR_PIN = 17
SERVO_PIN = 18

MAGNET_POSITION_ANGLE = 45
target_angle = 45

# SERVO_FULL_OPEN_TIME = 0.2 # seconds
SERVO_PROJECTILE_RELEASE_TIME = 0.15 # seconds that the projectile roughly takes to leave the compartment once servo starts opening

ws_manager = WebsocketManager()

voltage: VoltageControl
if (
    not TYPE_CHECKING
):  # otherwise causes issues with importing in tests, as not every environment has gpio
    voltage = VoltageControl(gpio_pin=SERVO_PIN)
    
class Controls:
    @staticmethod
    async def turn():
        voltage.turn()

    @staticmethod
    async def reset():
        voltage.turn_to('idle')



class RPMCounter:
    def __init__(self):
        self.last_time = None
        self.rpm_history = deque(maxlen=3)

    def pulse_detected(self):
        current_time = time.time()
        if self.last_time is not None:
            time_diff = current_time - self.last_time  # seconds per revolution
            if time_diff > 0:
                rpm = 60 / time_diff
                self.rpm_history.append(rpm)
                print(
                    f'Current RPM: {rpm:.2f}, Averaged RPM: {self.get_average_rpm():.2f}'
                )
        self.last_time = current_time

    def get_average_rpm(self):
        if self.rpm_history:
            return sum(self.rpm_history) / len(self.rpm_history)
        return 0.0


async def delay_based_on_rpm_and_target(target_deg: int):
    rpm = counter.get_average_rpm()
    rps = rpm / 60 # frequency
    if rps <= 0: 
        return
    period_seconds = 1 / rps

    angle_delta = (target_deg - MAGNET_POSITION_ANGLE) % 360

    ratio = angle_delta / 360

    delay = (ratio * period_seconds) - SERVO_PROJECTILE_RELEASE_TIME
    delay = max(delay, 0)

    await asyncio.sleep(delay)

    
counter = RPMCounter()

async def rpm_trigger():
    global shot_initiated
    if shot_initiated:
        await delay_based_on_rpm_and_target(target_angle)
        await Controls.turn()
        await set_shot_initiated(False)

    counter.pulse_detected()
    await ws_manager.broadcast(
        {'kind': 'RPM_UPDATE', 'data': {'rpm': counter.get_average_rpm()}}
    )

@asynccontextmanager
async def lifespan(app: FastAPI):  # runs when fastapi-server started
    voltage.turn_to('idle')

    sensor = DigitalInputDevice(SENSOR_PIN, bounce_time=0.001)


    sensor.when_deactivated = lambda: asyncio.run(rpm_trigger()) # enter
    sensor.when_activated = lambda: print('exit') # exit

    yield  # Clean up
    voltage.close()


# split server setup to make sure, all static-files (frontend) are served, unless it's a request to /api
app = FastAPI(lifespan=lifespan)  # the actual root server
api = FastAPI()  # the sub-route handler for /api


frontend_files_path = Path(__file__).parent / '../frontend/dist'


# serves index.html, as StaticFiles doesn't handle the conversion from index.html to /
@app.get('/', response_class=FileResponse)
async def index(request: Request):
    return frontend_files_path / 'index.html'


app.mount('/api', api)

if frontend_files_path.exists():
    app.mount(
        '/', StaticFiles(directory=frontend_files_path), name='static'
    )  # serves assests of the frontend
else:
    warnings.warn(
        'Static files not found, please run "bun generate" to build nuxt frontend, see README.md'
    )


# websocket to allow synchronized communication between multiple clients
@api.websocket('/ws')
async def websocket_endpoint(websocket: WebSocket):
    await ws_manager.connect(websocket)
    try:
        while True:
            data = await websocket.receive_text()
            await websocket.send_text(f'Message text was: {data}')
    except WebSocketDisconnect:
        ws_manager.disconnect(websocket)


shot_initiated = False

async def set_shot_initiated(value: bool):
    global shot_initiated
    shot_initiated = value
    return await ws_manager.broadcast({
        "kind": "INITIATE_UPDATE",
        "data": {
            "value": shot_initiated
        }
    })


class InitiateBody(BaseSchema):
    angle: Optional[int] | None = None

@api.post('/trigger/initiate')
async def post_initiate(body: InitiateBody):
    global target_angle
    if not shot_initiated:
        target_angle = body.angle
    await set_shot_initiated(not shot_initiated)

@api.post('/trigger/turn')
async def post_turn():
    return await Controls.turn()


@api.post('/reset')
async def post_reset():
    return await Controls.reset()

