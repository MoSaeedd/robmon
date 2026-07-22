const express = require('express');
const cors = require('cors');
const jwt = require('jsonwebtoken');
const bcrypt = require('bcryptjs');
const fs = require('fs');
const path = require('path');
const { LowSync } = require('lowdb');
const { JSONFileSync } = require('lowdb/node');

const app = express();
app.use(cors());
app.use(express.json());

const SECRET = process.env.JWT_SECRET || 'robmon-dev-secret';
const dataDir = path.join(__dirname, 'data');
fs.mkdirSync(dataDir, { recursive: true });

const adapter = new JSONFileSync(path.join(dataDir, 'db.json'));
const db = new LowSync(adapter, { users: [] });
db.read();
db.data = db.data || { users: [] };

function getUser(username) {
  return db.data.users.find(u => u.username === username);
}

function saveUser(username, password, role = 'user') {
  const passwordHash = bcrypt.hashSync(password, 10);
  db.data.users.push({ username, passwordHash, role });
  db.write();
  return getUser(username);
}

function ensureAdminUser() {
  if (!db.data.users || db.data.users.length === 0) {
    const adminPassword = process.env.ADMIN_PASSWORD || 'password';
    saveUser('admin', adminPassword, 'admin');
    console.log('Created default admin user. Set ADMIN_PASSWORD to a strong value.');
  }
}

ensureAdminUser();

const robots = new Map();
const services = new Map();
const revokedTokens = new Set();

function generateAccessToken(user) {
  return jwt.sign(user, SECRET, { expiresIn: '2h' });
}

function authenticateToken(req, res, next) {
  const authHeader = req.headers['authorization'];
  const token = authHeader && authHeader.split(' ')[1];
  if (!token) {
    return res.status(401).json({ error: 'Missing auth token' });
  }

  if (revokedTokens.has(token)) {
    return res.status(401).json({ error: 'Auth token revoked' });
  }

  jwt.verify(token, SECRET, (err, user) => {
    if (err) {
      return res.status(403).json({ error: 'Invalid auth token' });
    }
    req.user = user;
    next();
  });
}

function normalizeRobot(robot) {
  const metadata = robot.metadata || {};
  const metrics = robot.metrics || {};
  return {
    metadata: {
      robot_id: metadata.robot_id || robot.robot_id || 'unknown',
      hostname: metadata.hostname || robot.hostname || 'unknown',
      os: metadata.os || robot.os || 'unknown',
      arch: metadata.arch || robot.arch || 'unknown',
      ros_version: metadata.ros_version || robot.ros_version || 'unknown',
      agent_version: metadata.agent_version || robot.agent_version || 'unknown',
    },
    metrics: {
      cpu_usage: metrics.cpu_usage ?? 0,
      cpu_cores: metrics.cpu_cores ?? 0,
      memory_total_bytes: metrics.memory_total_bytes ?? metrics.memory_total_kb ?? 0,
      memory_used_bytes: metrics.memory_used_bytes ?? metrics.memory_used_kb ?? 0,
      memory_available_bytes: metrics.memory_available_bytes ?? metrics.memory_available_kb ?? 0,
      load_average: {
        one: metrics.load_average?.one ?? 0,
        five: metrics.load_average?.five ?? 0,
        fifteen: metrics.load_average?.fifteen ?? 0,
      },
    },
    last_seen: robot.last_seen || new Date().toISOString(),
    command_history: robot.command_history || [],
  };
}

app.post('/api/login', (req, res) => {
  const { username, password } = req.body;
  const user = getUser(username);
  if (!user || !bcrypt.compareSync(password, user.passwordHash)) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }
  const accessToken = generateAccessToken({ username: user.username, role: user.role });
  return res.json({ accessToken });
});

app.post('/api/logout', authenticateToken, (req, res) => {
  const authHeader = req.headers['authorization'];
  const token = authHeader && authHeader.split(' ')[1];
  if (!token) {
    return res.status(400).json({ error: 'Missing token' });
  }

  revokedTokens.add(token);
  return res.json({ ok: true, message: 'Logged out' });
});

app.post('/api/users', authenticateToken, (req, res) => {
  if (!req.user || req.user.role !== 'admin') {
    return res.status(403).json({ error: 'Admin role required' });
  }

  const { username, password, role } = req.body;
  if (!username || !password) {
    return res.status(400).json({ error: 'Missing username or password' });
  }
  if (getUser(username)) {
    return res.status(409).json({ error: 'User already exists' });
  }

  saveUser(username, password, role || 'user');
  return res.status(201).json({ ok: true, username });
});

app.post('/api/services', authenticateToken, (req, res) => {
  const service = req.body;
  if (!service || !service.serviceName || !service.host || !service.port) {
    return res.status(400).json({ error: 'Invalid service payload' });
  }

  services.set(service.serviceName, normalizeService(service));
  return res.status(200).json({ ok: true });
});

app.get('/api/services', authenticateToken, (req, res) => {
  return res.json(Array.from(services.values()));
});

app.get('/api/services/:serviceName', authenticateToken, (req, res) => {
  const service = services.get(req.params.serviceName);
  if (!service) {
    return res.status(404).json({ error: 'Service not found' });
  }
  return res.json(service);
});

app.post('/api/robots', (req, res) => {
  const robot = req.body;
  if (!robot || !robot.metadata || !robot.metadata.robot_id) {
    return res.status(400).json({ error: 'Invalid robot payload' });
  }

  robots.set(robot.metadata.robot_id, normalizeRobot(robot));
  return res.status(200).json({ ok: true });
});

app.get('/api/robots', (req, res) => {
  return res.json(Array.from(robots.values()));
});

app.get('/api/robots/:robotId', (req, res) => {
  const robot = robots.get(req.params.robotId);
  if (!robot) {
    return res.status(404).json({ error: 'Robot not found' });
  }
  return res.json(robot);
});

app.get('/api/robots/:robotId/commands', authenticateToken, (req, res) => {
  const robotId = req.params.robotId;
  if (!robots.has(robotId)) {
    return res.status(404).json({ error: 'Robot not found' });
  }
  return res.json({ commands: [] });
});

app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    robots: robots.size,
    services: services.size,
  });
});

function normalizeService(service) {
  return {
    serviceName: service.serviceName,
    host: service.host,
    port: service.port,
    protocol: service.protocol || 'http',
    created_at: service.created_at || new Date().toISOString(),
    meta: service.meta || {},
  };
}

const port = process.env.PORT || 8080;
app.listen(port, () => {
  console.log(`Control plane listening on http://localhost:${port}`);
});
