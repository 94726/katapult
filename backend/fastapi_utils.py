import json
from typing import Any

from fastapi import WebSocket
from fastapi.encoders import jsonable_encoder
from fastapi.websockets import WebSocketState
from pydantic import BaseModel, ConfigDict
from pydantic.alias_generators import to_camel


class BaseSchema(BaseModel):
    """Converts camelcase to snake_case and vice-versa, as python and javascript follow different conventions/code-styles"""

    model_config = ConfigDict(
        alias_generator=to_camel,
        populate_by_name=True,
        from_attributes=True,
    )


class WebsocketManager:
    """
    A simple controller for managing websocket connections.
    Collects all active connections and sends messages between them.
    """

    def __init__(self):
        self.active_connections: set[WebSocket] = set()

    async def connect(self, websocket: WebSocket):
        """Should be called to initially connect a websocket"""
        await websocket.accept()
        self.active_connections.add(websocket)

    def disconnect(self, websocket: WebSocket):
        self.active_connections.remove(websocket)

    async def send_personal_message(self, message: str, websocket: WebSocket):
        await websocket.send_text(message)

    async def broadcast(self, message: Any):
        """Sends a message to all connections"""
        for connection in self.active_connections:
            if connection.client_state != WebSocketState.CONNECTED:
                self.active_connections.remove(connection)
                continue
            try:
                await connection.send_text(json.dumps(jsonable_encoder(message)))
            except Exception:
                try:
                    for connection in self.active_connections:
                        await connection.close()
                except Exception:
                    pass
                finally:
                    self.active_connections.clear()
                    break
