const express = require('express');
const cors = require('cors');

const app = express();
app.use(cors());
app.use(express.json());

const robots = new Map();

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

app.get('/health', (req, res) => {
  res.json({ status: 'ok', robots: robots.size });
});

const port = process.env.PORT || 8080;
app.listen(port, () => {
  console.log(`Control plane listening on http://localhost:${port}`);
});
