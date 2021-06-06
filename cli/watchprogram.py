#!/usr/bin/env python3
import asyncio
import json
import websockets

connected = set()
prog_addr = "3ujtFXCGGRoWpJ1rKeeKBase18qoaCXDX7MCTKaMj89g"
ws_uri = "wss://api.mainnet-beta.solana.com/"
# ws_uri = "ws://localhost:8900/"


def parse_msg(msg):
    if not msg.get("jsonrpc") or not msg.get("params"):
        print(msg)
        return

    print(msg["params"]["result"]["value"]["logs"][7].split()[-1])


async def main():
    connected.add(ws_uri)
    try:
        async with websockets.connect(ws_uri, ssl=True) as ws:  # pylint: disable=E1101
            sub = {
                "jsonrpc": "2.0",
                "id": 42,
                "method": "logsSubscribe",
                "params": [  # ["all"],
                    {
                        "mentions": [prog_addr]
                    }
                ],
            }
            await ws.send(json.dumps(sub))
            while True:
                parse_msg(json.loads(await ws.recv()))
    finally:
        connected.remove(ws_uri)


if __name__ == "__main__":
    while True:
        asyncio.get_event_loop().run_until_complete(main())
