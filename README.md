# RobMon: a robot management platform for monitoring, deployment and connectivity. It aims to provide a robust and scalable secure mesh network connection between a fleet of robots. It also provides an easy way for user access management as well as ROS topic and service permissions.

> **License**: [PolyForm Noncommercial 1.0.0](LICENSE) — free for non-commercial use.  
> Commercial licenses available by contacting the licensor.

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

Build and run all services:

```bash
docker compose build
docker compose up
```

- Control plane: http://localhost:8080
- Web dashboard: http://localhost:5173

To build and run only the agent container (with the control plane already running locally):

```bash
docker build -t robmon-agent ./agent
docker run --rm \
  -e CONTROL_PLANE_URL=http://host.docker.internal:8080 \
  -e RUST_LOG=info \
  robmon-agent
```

On macOS, `host.docker.internal` lets the container reach services running on your machine. If your control plane uses a different host or port, set `CONTROL_PLANE_URL` accordingly.

---

## Project overview

This repository contains a lightweight observability platform with:

- a Node.js control plane
- a Vite/React web dashboard
- a Rust agent that publishes robot state
- automated auth and login tests

## Current status

Implemented pieces include local observability, robot state publishing, authentication, and logout support for the agent/control plane.

The detailed roadmap and future feature planning now live in [FUTURE_MAP.md](FUTURE_MAP.md).

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

## Mesh network (peer discovery)

Authenticated agents can join a private mesh and discover other online nodes. Each node gets a stable private IP from `10.42.0.0/16` (gateway `10.42.0.1`). Public endpoints are advertised directly; NAT hole punching is not required for this step.

### Agent configuration

Set the reachable public endpoint before starting the agent:

```bash
export MESH_PUBLIC_IP=203.0.113.10
export MESH_PUBLIC_PORT=51820   # optional, defaults to 51820
cd agent
cargo run
```

The agent logs in, joins the mesh, heartbeats every 10 seconds, and publishes its mesh IP plus discovered peers in robot state.

### Join the mesh

```bash
curl -X POST http://localhost:8080/api/mesh/join \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer $TOKEN' \
  -d '{"node_id":"robot-a","hostname":"robot-a","public_ip":"203.0.113.10","public_port":51820}'
```

### List online peers

```bash
curl -H 'Authorization: Bearer $TOKEN' 'http://localhost:8080/api/mesh/peers?exclude=robot-a'
```

Peers that miss a heartbeat for 30 seconds are treated as offline.

---

## Current feature checklist

- [x] Local robot observability dashboard with live fleet overview and robot detail modal
- [x] Rust agent that publishes state to the control plane
- [x] Private mesh IP assignment and authenticated peer discovery
- [ ] ROS 2 discovery, graph visualization, and remote operations
- [ ] Software deployment, rollback, and mesh networking with NAT traversal
- [ ] SSO / 2FA authentication and advanced fleet security policies
- [ ] Zero-trust identity, certificate-based robot identity, and secure access control

## Future roadmap

The detailed roadmap and feature map have been moved to [FUTURE_MAP.md](FUTURE_MAP.md).
