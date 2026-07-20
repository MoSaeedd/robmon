# ROSMesh OS — Detailed Feature Requirements and Development Breakdown

## Implementation Strategy

Development should proceed from:

1. **Local robot observability**
2. **ROS 2 awareness**
3. **Fleet visibility**
4. **Remote management**
5. **Security enforcement**
6. **Distributed networking**
7. **Full robot fleet operating system**

Each phase should produce a usable demonstration.

---

## Getting Started

### Local development

1. Install Node.js and npm:
   ```bash
   brew install node
   ```
2. Start the control plane:
   ```bash
   cd control-plane
   npm install
   npm start
   ```
3. Start the dashboard:
   ```bash
   cd web
   npm install
   npm run dev
   ```
4. Run the Rust agent:
   ```bash
   cd agent
   cargo run
   ```

### Testing

The project keeps package tests in dedicated folders, not inside application source files.

- Control plane tests are located in `control-plane/test/`
- The automation test script is `control-plane/test/auth-login.test.js`
- Run it from the control plane package:

```bash
cd control-plane
npm install
npm test
```

You can add future package tests in similar folders, for example `web/test/` or `agent/tests/`.

### Docker

1. Build all services:
   ```bash
   docker compose build
   ```
2. Run all services:
   ```bash
   docker compose up
   ```

- Control plane: http://localhost:8080
- Web dashboard: http://localhost:5173

---

# Phase 1 — ROSMesh Agent Foundation

## Goal

Create a lightweight Rust daemon running on every robot.

Agent responsibilities:

* Collect robot information
* Communicate with control plane
* Maintain local state
* Execute commands

---

## Feature 1.1 — Agent Runtime

### Description

Create a background service running on Linux.

### Requirements

The agent must:

* Start automatically using systemd.
* Run without user interaction.
* Maintain a persistent connection to the server.
* Recover automatically after crashes.
* Store local state.

### Technical requirements

Language:

* Rust

Runtime:

* Tokio async runtime

Service:

* systemd unit

Example:

```
rosmesh-agent.service

Status:
active (running)

PID:
1423
```

---

## Feature 1.2 — Robot Registration

### Description

Each robot registers with the ROSMesh platform.

### Requirements

The agent sends:

* Robot hostname
* Operating system
* ROS version
* CPU architecture
* Unique robot ID

Example:

```
Robot ID:
warehouse_robot_01

OS:
Ubuntu 22.04

ROS:
Jazzy

Agent:
v0.1.0
```

---

# Phase 2 — System Health Monitoring

## Goal

Monitor robot hardware and operating system state.

---

## Feature 2.1 — CPU Monitoring

Requirements:

Collect:

* CPU usage
* CPU temperature
* Number of cores
* Load average

Example:

```
CPU

Usage:
43%

Temperature:
58 C
```

---

## Feature 2.2 — Memory Monitoring

Collect:

* Total RAM
* Used RAM
* Available RAM
* Memory pressure

Example:

```
Memory

8 GB total

Used:
5.2 GB
```

---

## Feature 2.3 — Disk Monitoring

Collect:

* Disk usage
* Available storage
* Log directory size

Alerts:

```
WARNING:

Disk usage >90%
```

---

## Feature 2.4 — Network Monitoring

Collect:

* Network interfaces
* IP addresses
* Bandwidth usage
* Packet statistics

Example:

```
eth0

RX:
25 Mbps

TX:
5 Mbps
```

---

# Docker

Build the Docker image in the project root:

```bash
cd /Users/mohamedaboeljereed/Projects/robmon
docker build -t rosmesh-agent .
```

Run the container and point the agent at the host machine control plane:

```bash
docker run --rm \
  -e CONTROL_PLANE_URL=http://host.docker.internal:8080 \
  -e RUST_LOG=info \
  rosmesh-agent
```

On macOS, `host.docker.internal` lets the container reach services running on your machine.

If your control plane uses a different host or port, set `CONTROL_PLANE_URL` accordingly.

---

## Authentication and service discovery

The control plane now supports a lightweight login flow for service registration.

### Login

Request a token using:

```bash
curl -X POST http://localhost:8080/api/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"password"}'
```

The response contains an `accessToken` for service registration and discovery.

### Register a service

Use the token to register a backend service:

```bash
curl -X POST http://localhost:8080/api/services \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer $TOKEN' \
  -d '{"serviceName":"robmon:80","host":"10.0.0.5","port":80}'
```

### Discover services

Then list discovered mesh-backed services:

```bash
curl -H 'Authorization: Bearer $TOKEN' http://localhost:8080/api/services
```

---

## Current feature checklist

- [x] Local robot observability dashboard with live fleet overview and robot detail modal
- [x] Rust agent that publishes state to the control plane
- [ ] ROS 2 discovery, graph visualization, and remote operations
- [ ] Software deployment, rollback, and mesh networking with NAT traversal
- [ ] SSO / 2FA authentication and advanced fleet security policies
- [ ] Zero-trust identity, certificate-based robot identity, and secure access control

## Future roadmap

The detailed roadmap and feature map have been moved to [FUTURE_MAP.md](FUTURE_MAP.md).
