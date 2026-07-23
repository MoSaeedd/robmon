# RobMon Mesh VPN — Production Engineering Plan

> Thinking like a Tailscale/Oxide engineer: the mesh must be zero-config, secure by default, work through NAT, and be operationally invisible to the user.

---

## 0. Foundational Architecture Decisions

Before any code, these must be locked:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Tunnel protocol** | WireGuard | Kernel-grade, audited, simple, works on all platforms |
| **Key type** | Curve25519 static + optional ephemeral (Noise IK) | Standard for WG |
| **Control plane role** | Coordination only — never sees or holds private keys | Zero-trust: CP compromise ≠ mesh compromise |
| **Peer discovery** | CP relays `public_key` + `endpoint` + `mesh_ip` tuples | Agent configures WG locally |
| **NAT traversal** | STUN + birthday paradox UDP hole punching + periodic keepalives | Tailscale-proven approach |
| **Relay fallback** | TURN-like TCP relay (only when hole punch fails) | DERP-style from Tailscale |
| **Auth binding** | Mesh identity = JWT-authenticated node_id | Reuses existing login flow |
| **Key rotation** | On every agent restart (optional: periodic rotation) | PFS-like property |

---

## 1. Agent — WireGuard Interface Management (Rust)

### 1.1 Key Generation & Lifecycle
- [ ] **1.1.1** Generate Curve25519 static keypair on first agent start using `x25519-dalek` or `wg` CLI
- [ ] **1.1.2** Persist private key to `agent_state.json` (encrypted at rest with a derived key from machine UUID + salt)
- [ ] **1.1.3** On subsequent starts: load existing keypair; if missing/corrupt, regenerate and re-join mesh
- [ ] **1.1.4** Expose public key via `mesh.public_key` in agent state and API payloads
- [ ] **1.1.5** Optional: support `--regenerate-keys` flag for manual rotation

### 1.2 Interface Creation & Configuration
- [ ] **1.2.1** On mesh join success: create `robmon0` WireGuard interface via `wg-quick` or direct netlink (prefer `wireguard-rs` crate for portability)
- [ ] **1.2.2** Assign the mesh IP (`10.42.0.x/16`) to the interface
- [ ] **1.2.3** Set the private key on the interface
- [ ] **1.2.4** Set the interface MTU to 1280 (avoids IPv6 minimum MTU issues)
- [ ] **1.2.5** Bring interface up; add route for `10.42.0.0/16` via `robmon0`
- [ ] **1.2.6** On agent shutdown / logout: tear down interface, remove routes, clean up WireGuard peers
- [ ] **1.2.7** Handle interface already existing (reuse / reconfigure)

### 1.3 Peer Configuration Engine
- [ ] **1.3.1** On each heartbeat response: diff received peer list against current WireGuard config
- [ ] **1.3.2** For new peers: `wg set robmon0 peer <pubkey> endpoint <ip:port> allowed-ips <mesh_ip/32> persistent-keepalive 25`
- [ ] **1.3.3** For removed peers: `wg set robmon0 peer <pubkey> remove`
- [ ] **1.3.4** For changed endpoints: update endpoint only (no remove/re-add)
- [ ] **1.3.5** Batch peer updates into a single `wg syncconf` call when possible (atomic, no flap)
- [ ] **1.3.6** Log peer changes at INFO level for operational visibility

### 1.4 NAT Traversal Engine
- [ ] **1.4.1** **STUN**: On agent start, query a STUN server (e.g., `stun.l.google.com:19302`) to discover the server-reflexive address. Use `stun-rs` crate.
- [ ] **1.4.2** **Endpoint detection**: Compare STUN-discovered address against `MESH_PUBLIC_IP`. If they differ, the agent is behind NAT.
- [ ] **1.4.3** **Hole punching**: On heartbeat, send a UDP packet to each peer's public endpoint. WireGuard's built-in "birthday paradox" will establish the tunnel once both sides have punched.
- [ ] **1.4.4** **Keepalive**: Set `persistent-keepalive = 25` on all peers to maintain NAT bindings (25s is the industry standard).
- [ ] **1.4.5** **Endpoint update**: If STUN-discovered address changes mid-session, update the control plane via heartbeat and notify peers.
- [ ] **1.4.6** **NAT type detection**: Classify NAT as full-cone, restricted, port-restricted, or symmetric. Log the type for debugging.

### 1.5 Relay Fallback (DERP-style)
- [ ] **1.5.1** If direct WireGuard handshake fails after 5 seconds, fall back to a TCP relay server
- [ ] **1.5.2** Relay server: simple TCP proxy that forwards encrypted WireGuard packets between two connected peers
- [ ] **1.5.3** Agent connects to relay via WebSocket or TCP, authenticates with JWT
- [ ] **1.5.4** Relay assigns a virtual endpoint (e.g., `relay://<relay_id>/<node_id>`)
- [ ] **1.5.5** Agent reports relay endpoint to control plane as fallback address
- [ ] **1.5.6** Peers attempt direct WG first; if that fails, they connect via relay
- [ ] **1.5.7** Relay traffic is encrypted end-to-end (WireGuard already encrypts; relay is blind)
- [ ] **1.5.8** Once direct connection is established (e.g., NAT binding becomes stable), switch from relay to direct and tear down relay connection

### 1.6 Interface Health & Recovery
- [ ] **1.6.1** Periodic `wg show robmon0` to verify interface state
- [ ] **1.6.2** Detect interface down (e.g., after sleep/wake on laptop) and recreate
- [ ] **1.6.3** Detect peer handshake timeout (> 2 minutes) and re-punch
- [ ] **1.6.4** Expose WireGuard stats (bytes transferred, handshake time) in agent metrics

---

## 2. Agent — Mesh Sync Loop (Rust)

### 2.1 Extended Join/Heartbeat Payloads
- [ ] **2.1.1** Add `public_key` field to `MeshJoinPayload`
- [ ] **2.1.2** Add `public_key` field to `MeshHeartbeatPayload`
- [ ] **2.1.3** Add `nat_type` field (derived from STUN) to payloads
- [ ] **2.1.4** Add `relay_endpoint` field (if relay fallback is active)
- [ ] **2.1.5** Update `MeshPeer` model to include `public_key`, `nat_type`, `relay_endpoint`

### 2.2 Peer Reconciliation
- [ ] **2.2.1** On heartbeat response: compute diff between received peers and current WG config
- [ ] **2.2.2** Add missing peers, remove stale peers, update changed endpoints
- [ ] **2.2.3** If a peer's endpoint changed: trigger a new hole punch to that peer
- [ ] **2.2.4** If a peer went offline: keep their WG config for 5 minutes (they may come back), then remove
- [ ] **2.2.5** Log peer count changes: `"Mesh: 3 peers online (added: robot-b, removed: robot-c)"`

### 2.3 Connection Quality Monitoring
- [ ] **2.3.1** Track per-peer handshake latency (time since last handshake)
- [ ] **2.3.2** Track per-peer bytes transferred (rx/tx)
- [ ] **2.3.3** If handshake > 30s stale: log warning, trigger re-punch
- [ ] **2.3.4** If handshake > 120s stale: mark peer as degraded, attempt relay fallback
- [ ] **2.3.5** Expose all metrics via the existing metrics endpoint

---

## 3. Control Plane — Key Distribution & Coordination (Node.js)

### 3.1 Public Key Registry
- [ ] **3.1.1** Add `public_key` field to mesh node storage
- [ ] **3.1.2** Add `nat_type` field to mesh node storage
- [ ] **3.1.3** Add `relay_endpoint` field to mesh node storage
- [ ] **3.1.4** Validate public key format (32 bytes, base64-encoded) on join/heartbeat
- [ ] **3.1.5** Reject join if public key conflicts with existing node (key already in use by different node_id)

### 3.2 Enhanced Mesh Responses
- [ ] **3.2.1** Include `public_key` in `/api/mesh/join` response for all peers
- [ ] **3.2.2** Include `public_key` in `/api/mesh/heartbeat` response for all peers
- [ ] **3.2.3** Include `nat_type` and `relay_endpoint` in peer objects
- [ ] **3.2.4** Add `endpoint` field (computed from `public_ip:public_port`) to peer objects for convenience

### 3.3 NAT-Aware Peer Selection
- [ ] **3.3.1** When returning peers, prefer nodes with known public endpoints
- [ ] **3.3.2** If both peers are behind symmetric NAT, flag them for relay fallback
- [ ] **3.3.3** Expose `/api/mesh/nat-status` endpoint for debugging

### 3.4 Relay Server (built into control plane or separate)
- [ ] **3.4.1** WebSocket-based relay endpoint: `/api/relay`
- [ ] **3.4.2** Authenticate relay connections with JWT
- [ ] **3.4.3** Map connected nodes to virtual relay addresses
- [ ] **3.4.4** Forward encrypted WireGuard packets between connected peers
- [ ] **3.4.5** Rate-limit relay traffic per node (prevent abuse)
- [ ] **3.4.6** Log relay usage metrics (bytes relayed, active relay sessions)

---

## 4. Security Architecture

### 4.1 Identity & Authentication
- [ ] **4.1.1** **Node identity**: `node_id` is authenticated via JWT from login. Mesh join requires valid JWT.
- [ ] **4.1.2** **Key binding**: Control plane maps `node_id ↔ public_key`. If a node tries to re-join with a different public key, require re-authentication.
- [ ] **4.1.3** **Key rotation**: Support `--rotate-keys` flag. On rotation: generate new keypair, re-join mesh, old key is invalidated.
- [ ] **4.1.4** **Node revocation**: Admin can revoke a node's access via API. Revoked nodes are removed from mesh and their key is blacklisted.

### 4.2 Transport Security
- [ ] **4.2.1** **Control plane API**: All HTTP traffic MUST use TLS in production. Document `CERT_FILE` / `KEY_FILE` env vars.
- [ ] **4.2.2** **WireGuard**: All mesh traffic is encrypted with WireGuard's ChaCha20Poly1305. No additional encryption needed.
- [ ] **4.2.3** **Relay traffic**: End-to-end encrypted by WireGuard. Relay is a blind packet forwarder.
- [ ] **4.2.4** **mTLS option**: For high-security deployments, add optional mutual TLS between agent and control plane.

### 4.3 Access Control
- [ ] **4.3.1** **Mesh ACLs**: Control plane enforces which nodes can see each other. Implement `mesh_acl` field per user/role.
- [ ] **4.3.2** **Network segmentation**: Support multiple mesh subnets (e.g., `10.42.0.0/16` for robots, `10.43.0.0/16` for admin devices).
- [ ] **4.3.3** **Rate limiting**: Limit join/heartbeat frequency per node (prevent DoS on IP allocation).

### 4.4 Audit & Observability
- [ ] **4.4.1** **Audit log**: Log all mesh join/leave/key-rotate events with timestamp, node_id, IP
- [ ] **4.4.2** **Metrics**: Expose mesh metrics: connected peers, relay sessions, handshake failures, NAT types
- [ ] **4.4.3** **Alerts**: Alert on: repeated handshake failures, unexpected key changes, relay bandwidth spikes

### 4.5 Defense in Depth
- [ ] **4.5.1** **Private key encryption**: Encrypt private key at rest using AES-256-GCM with key derived from machine-specific secret + user-provided passphrase (optional)
- [ ] **4.5.2** **Firewall rules**: Agent should configure `iptables` / `nftables` to only allow WireGuard traffic on the mesh interface
- [ ] **4.5.3** **Split DNS**: Optionally push DNS records for `.robmon` domain pointing to mesh IPs
- [ ] **4.5.4** **Ephemeral keys**: Support per-session ephemeral keys (re-key on every agent restart)

---

## 5. Testing

### 5.1 Unit Tests (Rust)
- [ ] **5.1.1** Key generation and serialization
- [ ] **5.1.2** Peer diff logic (add/remove/update)
- [ ] **5.1.3** NAT type classification from STUN response
- [ ] **5.1.4** Payload serialization with new fields
- [ ] **5.1.5** Interface config generation (string formatting)

### 5.2 Integration Tests
- [ ] **5.2.1** **Two-node mesh**: Spawn two agents + control plane in Docker. Verify they discover each other and can ping via mesh IP.
- [ ] **5.2.2** **NAT simulation**: Use Docker with `--cap-add=NET_ADMIN` and `iptables` to simulate NAT. Verify hole punching works.
- [ ] **5.2.3** **Relay fallback**: Block direct UDP between two containers. Verify they fall back to relay and can still communicate.
- [ ] **5.2.4** **Key rotation**: Rotate keys on one agent. Verify other agents detect the change and update their WG config.
- [ ] **5.2.5** **Node revocation**: Revoke a node. Verify it's removed from all peers' WG configs.
- [ ] **5.2.6** **Reconnection**: Kill and restart an agent. Verify it rejoins the mesh and re-establishes tunnels.

### 5.3 Chaos Tests
- [ ] **5.3.1** **Network partition**: Isolate two nodes. Verify they detect disconnection and reconnect when partition heals.
- [ ] **5.3.2** **Sleep/wake**: Simulate laptop sleep (pause agent for 30 min). Verify it reconnects on wake.
- [ ] **5.3.3** **IP change**: Change agent's public IP mid-session. Verify STUN detects the change and updates peers.

---

## 6. Cross-Cutting & Operations

### 6.1 Documentation
- [ ] **6.1.1** WireGuard installation guide for all supported platforms
- [ ] **6.1.2** NAT traversal explanation and troubleshooting guide
- [ ] **6.1.3** Security model documentation (threat model, trust boundaries)
- [ ] **6.1.4** Relay server deployment guide
- [ ] **6.1.5** Mesh ACL configuration guide

### 6.2 Docker & Deployment
- [ ] **6.2.1** Add `--cap-add=NET_ADMIN` to agent Dockerfile
- [ ] **6.2.2** Add `--sysctl net.ipv4.ip_forward=1` for routing
- [ ] **6.2.3** Add WireGuard installation to Dockerfile
- [ ] **6.2.4** Document Kubernetes deployment with `securityContext.capabilities`

### 6.3 Monitoring
- [ ] **6.3.1** Add mesh health check to `/health` endpoint
- [ ] **6.3.2** Add Prometheus metrics for mesh state
- [ ] **6.3.3** Add Grafana dashboard template for mesh visualization

---

## Effort Summary

| Phase | Area | Tasks | Effort |
|-------|------|-------|--------|
| **P0** | Key generation & interface | 1.1, 1.2 | 2 days |
| **P0** | Peer config engine | 1.3 | 1.5 days |
| **P0** | Extended join/heartbeat | 2.1, 3.1, 3.2 | 1 day |
| **P0** | Peer reconciliation | 2.2 | 1 day |
| **P1** | STUN & NAT detection | 1.4.1–1.4.6 | 2 days |
| **P1** | Hole punching | 1.4.3–1.4.4 | 1 day |
| **P1** | Relay server | 1.5, 3.4 | 3 days |
| **P1** | Relay fallback logic | 1.5.1–1.5.8 | 1.5 days |
| **P2** | Security hardening | 4.1–4.5 | 3 days |
| **P2** | Connection quality monitoring | 2.3 | 1 day |
| **P2** | Unit tests | 5.1 | 1 day |
| **P2** | Integration tests | 5.2 | 2 days |
| **P3** | Chaos tests | 5.3 | 2 days |
| **P3** | Documentation & ops | 6.1–6.3 | 2 days |
| | **Total** | | **~24 days (5 weeks)** |

### Key Insight
The NAT traversal + relay fallback is the **highest-risk, highest-effort** component (~7.5 days). If all nodes are guaranteed to have public IPs (no NAT), this drops to **~12 days**. The phased approach (P0 → P1 → P2 → P3) lets you ship a working mesh VPN after P0 and iterate on NAT/security.