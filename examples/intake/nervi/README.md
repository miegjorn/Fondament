# Nèrvi

Nèrvi is the async subscription fabric for the Occitan stack. It deploys NATS
JetStream on the cluster and exposes `nervi_publish` and `nervi_subscribe` MCP
tools so agents can exchange machine-readable signals without synchronous
coordination or a Matrix room.

## What it is not

Nèrvi is not Charradissa (the human-chat layer) and is not a general
application message queue. It carries intra-stack agent signals only. The first
sensor is an SRE log monitor that publishes anomalies to `ops.sre.alerts`.

## Occitan stack position

| Depends on | Farga (optional — signal writes) |
|---|---|
| Exposes to | Any agent with the Nèrvi MCP tools |
| Namespace | occitan-system |
| Port | 8080 (MCP), 4222 (NATS) |
