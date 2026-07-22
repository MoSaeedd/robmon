# RobMon Future Roadmap

This file contains the detailed roadmap and feature map for the RobMon project.

# Phase 1 — RobMon Agent Foundation

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
robmon-agent.service

Status:
active (running)

PID:
1423
```

---

## Feature 1.2 — Robot Registration

### Description

Each robot registers with the RobMon platform.

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

# Phase 3 — ROS 2 Discovery Monitoring

## Goal

Understand what is running inside the robot.

---

## Feature 3.1 — ROS Node Discovery

Requirements:

Detect:

* Node names
* Namespaces
* Node lifetime

Example:

```
Robot:

Nodes:

/navigation
/localization
/camera_driver
```

---

## Feature 3.2 — Topic Discovery

Requirements:

Track:

* Topic names
* Message types
* Publishers
* Subscribers

Example:

```
Topic:

/cmd_vel

Publisher:

navigation_node

Subscriber:

controller_node
```

---

## Feature 3.3 — Topic Activity Monitoring

Requirements:

Track:

* Last message timestamp
* Message frequency
* Message count

Example:

```
Topic:

/odom

Rate:

50 Hz

Last update:

12 ms ago
```

---

## Feature 3.4 — ROS Graph Storage

Requirements:

Store historical graph state.

Examples:

* Node appeared
* Node disappeared
* Topic created
* Topic removed

Example:

```
14:32

navigation_node started publishing:

/cmd_vel
```

---

# Phase 4 — Fleet Dashboard

## Goal

Visualize all robots.

---

## Feature 4.1 — Fleet Overview

Display:

* Robot list
* Online/offline status
* Health state

Example:

```
Fleet

Robot 01   ONLINE
Robot 02   WARNING
Robot 03   OFFLINE
```

---

## Feature 4.2 — Robot Detail Page

Display:

Hardware:

* CPU
* Memory
* Disk

ROS:

* Nodes
* Topics
* Activity

Software:

* Agent version
* ROS version

---

## Feature 4.3 — ROS Graph Visualization

Display:

Interactive graph:

```
camera_node

    |
    |
/camera/image

    |
    |

perception_node
```

---

# Phase 5 — Remote Operations

## Goal

Allow engineers to manage robots remotely.

---

## Feature 5.1 — Log Collection

Requirements:

Collect:

* system logs
* ROS logs
* application logs

Support:

* search
* filtering
* download

---

## Feature 5.2 — Remote Command Execution

Requirements:

Execute approved commands:

Examples:

```
restart navigation node

collect diagnostics

restart service
```

Security:

* authenticated requests
* audit trail

---

## Feature 5.3 — Remote Terminal

Provide:

* secure shell access
* session logging
* permission control

Example:

```
Engineer

  |
Dashboard

  |
Robot terminal
```

---

# Phase 6 — Software Deployment

## Goal

Manage robot software versions.

---

## Feature 6.1 — Application Inventory

Track:

* ROS packages
* Docker images
* Git commits
* configuration versions

Example:

```
Robot 01

Navigation:
v2.1.5

Camera:
v1.4.0
```

---

## Feature 6.2 — Software Deployment

Requirements:

Support:

* package installation
* Docker deployment
* configuration updates

---

## Feature 6.3 — Rollback

Requirements:

Store previous versions.

Example:

```
Deployment failed

Rollback:

v2.1.5 -> v2.1.4
```

---

# Phase 7 — Robot Identity and Security

## Goal

Move from device trust to zero trust.

---

## Feature 7.1 — Cryptographic Robot Identity

Requirements:

Each robot has:

* public/private key pair
* identity certificate
* registration state

---

## Feature 7.1.1 — Two-factor authentication

Requirements:

* Strong user authentication for dashboard and remote actions
* One-time passwords or hardware-backed second factor
* Login flows that require both credentials and a second verification step
* Audit logs for 2FA validation events

---

## Feature 7.2 — Access Policies

Policies apply to:

* robots
* users
* processes
* ROS nodes
* topics

Example:

```yaml
robot:
 warehouse_01

allow:

 navigation:
   publish:
     - /cmd_vel
```

---

## Feature 7.3 — ROS Topic Authorization

Requirements:

Allow:

* topic allow lists
* topic deny lists
* publisher restrictions

Example:

```
Allowed:

navigation_node
   -> /cmd_vel


Denied:

unknown_node
   -> /cmd_vel
```

---

## Feature 7.4 — Process Identity

Map:

```
Linux process

      ↓

ROS node

      ↓

Topic permissions
```

Collect:

* PID
* executable
* container ID
* user

---

# Phase 8 — Mesh Networking Layer

## Goal

Build the Tailscale/ZeroTier-like foundation.

---

## Feature 8.1 — Virtual Network Interface

Requirements:

Create:

```
robmon0
```

Capabilities:

* virtual IP addressing
* packet routing
* encrypted traffic

---

## Feature 8.2 — Peer Discovery

Requirements:

Nodes discover:

* available peers
* public endpoints
* connectivity state

---

## Feature 8.3 — Encrypted Tunnel

Requirements:

Support:

* authenticated encryption
* key exchange
* session rotation

---

## Feature 8.4 — Full Mesh Connectivity

Requirements:

Support:

* many-to-many communication
* direct peer connections
* relay fallback

Example:

```
Robot A
 |
 +--- Robot B
 |
 +--- Robot C
```

---

# Phase 9 — Advanced Fleet Security

## Goal

Create a robotics-native zero trust network.

---

## Feature 9.1 — Network Policies

Control:

* robot-to-robot communication
* service access
* external access

---

## Feature 9.2 — ROS Security Policies

Control:

* who publishes topics
* who subscribes
* which processes can command robots

---

## Feature 9.3 — Security Audit Dashboard

Display:

* blocked actions
* authentication events
* policy changes

---

# Final Milestone Demo

A complete demonstration should show:

## Three simulated robots

Each robot:

* Runs ROS 2
* Runs RobMon agent
* Joins secure mesh

Demonstrate:

1. Dashboard discovers robots.
2. ROS graph appears automatically.
3. Health metrics update.
4. Remote logs collected.
5. Software deployment succeeds.
6. Unauthorized `/cmd_vel` publisher is blocked.
7. Robots communicate through encrypted mesh.
8. Network topology is visualized.

---

# Final Capability

The finished system demonstrates:

* Rust systems programming
* Linux networking
* Distributed systems
* ROS 2 middleware
* Security engineering
* Cloud control planes
* Fleet operations
