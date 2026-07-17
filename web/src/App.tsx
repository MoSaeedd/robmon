import { useEffect, useState } from 'react';

type RobotState = {
  metadata: {
    robot_id: string;
    hostname: string;
    os: string;
    arch: string;
    ros_version: string;
    agent_version: string;
  };
  metrics: {
    cpu_usage: number;
    cpu_cores: number;
    memory_total_bytes: number;
    memory_used_bytes: number;
    memory_available_bytes: number;
    load_average: {
      one: number;
      five: number;
      fifteen: number;
    };
  };
  last_seen: string;
  command_history: string[];
};

type DeployConfig = {
  image: string;
  ros_domain: number;
  network_interface: string;
  ros_config: string;
  custom_ros_config: string;
};

const MAX_HISTORY = 20;

type SparklineProps = {
  values: number[];
  color: string;
  label: string;
};

function Sparkline({ values, color, label }: SparklineProps) {
  const width = 220;
  const height = 54;
  const points = values.length
    ? values.map((value, index) => {
        const x = values.length === 1 ? width / 2 : (index * width) / (values.length - 1);
        const y = height - Math.min(value, 100) / 100 * height;
        return `${x},${y}`;
      })
    : [`0,${height}`, `${width},${height}`];

  const pointList = points.join(' ');

  return (
    <div className="sparkline">
      <div className="sparkline-label">{label}</div>
      <svg viewBox={`0 0 ${width} ${height}`} className="sparkline-chart" aria-hidden="true">
        <polyline
          fill="none"
          stroke={color}
          strokeWidth="2.5"
          strokeLinejoin="round"
          strokeLinecap="round"
          points={pointList}
        />
        <polygon
          fill={`${color}22`}
          stroke="none"
          points={`${pointList} ${width},${height} 0,${height}`}
        />
      </svg>
    </div>
  );
}

type UsageBarProps = {
  label: string;
  value: number;
  max: number;
  caption: string;
  color: string;
};

function UsageBar({ label, value, max, caption, color }: UsageBarProps) {
  const percentage = max > 0 ? Math.min(100, (value / max) * 100) : 0;

  return (
    <div className="usage-bar-card">
      <div className="usage-bar-title">
        <span>{label}</span>
        <strong>{caption}</strong>
      </div>
      <div className="usage-bar-track" aria-label={`${label} usage`}>
        <div className="usage-bar-fill" style={{ width: `${percentage}%`, background: color }} />
      </div>
      <div className="usage-bar-percent">{percentage.toFixed(0)}%</div>
    </div>
  );
}

function formatBytes(bytes: number) {
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

const IMAGE_OPTIONS = [
  'ghcr.io/rosmesh/robot-agent:latest',
  'ghcr.io/rosmesh/robot-agent:stable',
  'ghcr.io/rosmesh/robot-agent:dev',
];

const NETWORK_OPTIONS = ['rosmesh0', 'eth0', 'wlan0'];
const DOMAIN_OPTIONS = [0, 1, 2, 3, 101];

const ROS_CONFIG_OPTIONS = [
  {
    label: 'Default ROS2 network',
    value: 'ros__parameters:\n  use_sim_time: false\nnetwork:\n  interface: rosmesh0',
  },
  {
    label: 'Secure ROS2 domain',
    value: 'ros__parameters:\n  use_sim_time: false\nsecurity:\n  enabled: true\nnetwork:\n  interface: rosmesh0',
  },
  {
    label: 'Simulation mode',
    value: 'ros__parameters:\n  use_sim_time: true\nnetwork:\n  interface: rosmesh0',
  },
  {
    label: 'Custom config',
    value: 'custom',
  },
];

const DEFAULT_DEPLOY: DeployConfig = {
  image: 'ghcr.io/rosmesh/robot-agent:latest',
  ros_domain: 0,
  network_interface: 'rosmesh0',
  ros_config: ROS_CONFIG_OPTIONS[0].value,
  custom_ros_config: '',
};

function App() {
  const [robots, setRobots] = useState<RobotState[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedRobotId, setSelectedRobotId] = useState<string>('');
  const [detailOpen, setDetailOpen] = useState(false);
  const [deployConfig, setDeployConfig] = useState<DeployConfig>(DEFAULT_DEPLOY);
  const [deployMessage, setDeployMessage] = useState<string>('');
  const [cpuHistory, setCpuHistory] = useState<number[]>([]);
  const [ramHistory, setRamHistory] = useState<number[]>([]);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const fetchRobots = async () => {
      try {
        const response = await fetch('/api/robots');
        if (!response.ok) {
          throw new Error(`Failed to load robots: ${response.status}`);
        }
        const data = await response.json();
        setRobots(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    };

    fetchRobots();
    const interval = setInterval(fetchRobots, 1000);
    return () => clearInterval(interval);
  }, [selectedRobotId]);

  const isRobotOnline = (robot: RobotState) => {
    const lastSeen = new Date(robot.last_seen).getTime();
    return now - lastSeen <= 5000;
  };

  const selectedRobot = robots.find((robot) => robot.metadata.robot_id === selectedRobotId);
  const totalRobots = robots.length;
  const onlineRobots = robots.filter((robot) => isRobotOnline(robot)).length;

  function formatRelativeTime(lastSeen: string) {
    const ms = Date.now() - new Date(lastSeen).getTime();
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) {
      return `${seconds}s ago`;
    }
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) {
      return `${minutes}m ago`;
    }
    const hours = Math.floor(minutes / 60);
    return `${hours}h ago`;
  }

  const averageCpu = totalRobots
    ? robots.reduce((sum, robot) => sum + robot.metrics.cpu_usage, 0) / totalRobots
    : 0;
  const averageMemoryPercent = totalRobots
    ?
        robots.reduce(
          (sum, robot) =>
            sum +
            (robot.metrics.memory_total_bytes > 0
              ? (robot.metrics.memory_used_bytes / robot.metrics.memory_total_bytes) * 100
              : 0),
          0
        ) / totalRobots
    : 0;
  const memoryTotalGB = selectedRobot ? selectedRobot.metrics.memory_total_bytes / 1024 / 1024 / 1024 : 0;
  const memoryUsedGB = selectedRobot ? selectedRobot.metrics.memory_used_bytes / 1024 / 1024 / 1024 : 0;

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);

  const closeDetail = () => setDetailOpen(false);

  const openRobotDetail = (robotId: string) => {
    setSelectedRobotId(robotId);
    setDetailOpen(true);
    setDeployMessage('');
  };

  useEffect(() => {
    if (!selectedRobot) {
      return;
    }

    setCpuHistory((existing) => {
      const next = [...existing, selectedRobot.metrics.cpu_usage];
      return next.slice(-MAX_HISTORY);
    });

    const memoryUsedPercent =
      selectedRobot.metrics.memory_total_bytes > 0
        ? (selectedRobot.metrics.memory_used_bytes / selectedRobot.metrics.memory_total_bytes) * 100
        : 0;

    setRamHistory((existing) => {
      const next = [...existing, memoryUsedPercent];
      return next.slice(-MAX_HISTORY);
    });
  }, [selectedRobot]);

  const handleSelectRobot = (robotId: string) => {
    openRobotDetail(robotId);
  };

  const handleDeploySubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedRobot) {
      setDeployMessage('Select a robot before deploying.');
      return;
    }
    const configLabel = deployConfig.ros_config === 'custom' ? 'custom ROS config' : 'preset ROS config';
    setDeployMessage(
      `Deploying ${deployConfig.image} to ${selectedRobot.metadata.hostname} on ROS domain ${deployConfig.ros_domain} with ${deployConfig.network_interface} and ${configLabel}.`
    );
  };

  return (
    <div className="app-shell">
      <header>
        <h1>ROSMesh Dashboard</h1>
        <p>Unified robot status, telemetry, and deployment control.</p>
      </header>
      <section className="hero-panel">
        <div>
          <h2>Robot visibility, ROS2 health, and fleet readiness in one view.</h2>
          <p>Track CPU, RAM, deployment state, and live connection status while the platform grows toward fleet-level management.</p>
        </div>
        <div className="phase-pill-row">
          <span className="phase-pill">Phase 1: Observability</span>
          <span className="phase-pill">Phase 2: Health</span>
          <span className="phase-pill">Phase 3: Fleet</span>
        </div>
      </section>
      {error && <div className="error">{error}</div>}

      <div className="dashboard-summary">
        <div className="summary-card">
          <span className="summary-label">Robots online</span>
          <strong>{onlineRobots} / {totalRobots}</strong>
        </div>
        <div className="summary-card">
          <span className="summary-label">Avg CPU</span>
          <strong>{averageCpu.toFixed(1)}%</strong>
        </div>
        <div className="summary-card">
          <span className="summary-label">Avg RAM</span>
          <strong>{averageMemoryPercent.toFixed(1)}%</strong>
        </div>
      </div>

      <div className="main-actions">
        <div>
          <h2>Fleet overview</h2>
          <p className="section-copy">Select a robot to view live health, metrics, and deployment options. Robot statuses refresh automatically every second.</p>
        </div>
      </div>

      <div className="fleet-grid">
        {robots.map((robot) => {
          const online = isRobotOnline(robot);
          const ageSeconds = Math.max(0, Math.round((now - new Date(robot.last_seen).getTime()) / 1000));
          return (
            <button
              key={robot.metadata.robot_id}
              className={`robot-card ${online ? 'online' : 'offline'}`}
              onClick={() => openRobotDetail(robot.metadata.robot_id)}
            >
              <div className="robot-card-header">
                <div>
                  <div className="robot-name">{robot.metadata.hostname}</div>
                  <div className="robot-meta">{robot.metadata.robot_id} · {robot.metadata.ros_version} · {robot.metadata.os}</div>
                </div>
                <span className={`status-dot ${online ? 'online' : 'offline'}`} />
              </div>

              <div className="robot-card-body">
                <div className="robot-card-row">
                  <span>CPU</span>
                  <strong>{robot.metrics.cpu_usage.toFixed(1)}%</strong>
                </div>
                <div className="robot-card-row">
                  <span>Memory</span>
                  <strong>{formatBytes(robot.metrics.memory_used_bytes)} / {formatBytes(robot.metrics.memory_total_bytes)}</strong>
                </div>
              </div>

              <div className="robot-card-footer">
                <span>{formatRelativeTime(robot.last_seen)}</span>
                <span>{online ? 'Online' : 'Offline'}</span>
              </div>
            </button>
          );
        })}
      </div>

      {detailOpen && selectedRobot && (
        <div className="overlay" onClick={closeDetail}>
          <div className="detail-modal" onClick={(event) => event.stopPropagation()}>
            <button className="close-btn" onClick={closeDetail} aria-label="Close robot detail">×</button>
            <section className="detail-card">
              <div className="detail-header">
                <div>
                  <h2>{selectedRobot.metadata.hostname}</h2>
                  <p className="subtitle">{selectedRobot.metadata.robot_id} · {selectedRobot.metadata.os} · {selectedRobot.metadata.arch}</p>
                </div>
                <span className={`status-pill ${isRobotOnline(selectedRobot) ? 'online' : 'offline'}`}>
                  {isRobotOnline(selectedRobot) ? 'Live' : 'Offline'}
                </span>
              </div>

              <div className="status-grid">
                <div className="status-card">
                  <span className="label">Last Seen</span>
                  <div>{new Date(selectedRobot.last_seen).toLocaleString()}</div>
                </div>
                <div className="status-card">
                  <span className="label">ROS version</span>
                  <div>{selectedRobot.metadata.ros_version}</div>
                </div>
                <div className="status-card">
                  <span className="label">Agent</span>
                  <div>{selectedRobot.metadata.agent_version}</div>
                </div>
              </div>

              <div className="health-panels">
                <div className="usage-panels">
                  <UsageBar
                    label="CPU utilization"
                    value={selectedRobot.metrics.cpu_usage}
                    max={100}
                    caption={`${selectedRobot.metrics.cpu_usage.toFixed(1)} / 100%`}
                    color="#38bdf8"
                  />
                  <UsageBar
                    label="Memory usage"
                    value={selectedRobot.metrics.memory_used_bytes}
                    max={selectedRobot.metrics.memory_total_bytes}
                    caption={`${formatBytes(selectedRobot.metrics.memory_used_bytes)} / ${formatBytes(selectedRobot.metrics.memory_total_bytes)}`}
                    color="#a855f7"
                  />
                </div>
                <div className="graph-panel">
                  <Sparkline values={cpuHistory} label="CPU trend" color="#38bdf8" />
                  <Sparkline values={ramHistory} label="RAM trend" color="#a855f7" />
                </div>
              </div>

              <div className="metric-group">
                <h3>System Metrics</h3>
                <div className="metric-row">
                  <span>CPU</span>
                  <strong>{selectedRobot.metrics.cpu_usage.toFixed(1)}%</strong>
                </div>
                <div className="metric-row">
                  <span>Cores</span>
                  <strong>{selectedRobot.metrics.cpu_cores}</strong>
                </div>
                <div className="metric-row">
                  <span>Memory total</span>
                  <strong>{formatBytes(selectedRobot.metrics.memory_total_bytes)}</strong>
                </div>
                <div className="metric-row">
                  <span>Memory used</span>
                  <strong>{formatBytes(selectedRobot.metrics.memory_used_bytes)}</strong>
                </div>
                <div className="metric-row">
                  <span>Memory available</span>
                  <strong>{formatBytes(selectedRobot.metrics.memory_available_bytes)}</strong>
                </div>
                <div className="metric-row">
                  <span>Load</span>
                  <strong>{selectedRobot.metrics.load_average.one.toFixed(2)}</strong>
                </div>
              </div>

              <div className="history-group">
                <h3>Recent Command History</h3>
                {selectedRobot.command_history.length ? (
                  <ul>
                    {selectedRobot.command_history.slice(-5).map((entry, index) => (
                      <li key={index}>{entry}</li>
                    ))}
                  </ul>
                ) : (
                  <div className="empty-state">No recent commands executed.</div>
                )}
              </div>
            </section>

            <section className="detail-card deploy-card">
              <h3>Deploy Application</h3>
              <form onSubmit={handleDeploySubmit}>
                <label>
                  Image URL
                  <select
                    value={deployConfig.image}
                    onChange={(event) => setDeployConfig({ ...deployConfig, image: event.target.value })}
                  >
                    {IMAGE_OPTIONS.map((image) => (
                      <option key={image} value={image}>
                        {image}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  ROS 2 Domain ID
                  <select
                    value={deployConfig.ros_domain}
                    onChange={(event) => setDeployConfig({ ...deployConfig, ros_domain: Number(event.target.value) })}
                  >
                    {DOMAIN_OPTIONS.map((domain) => (
                      <option key={domain} value={domain}>
                        {domain}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Network interface
                  <select
                    value={deployConfig.network_interface}
                    onChange={(event) => setDeployConfig({ ...deployConfig, network_interface: event.target.value })}
                  >
                    {NETWORK_OPTIONS.map((iface) => (
                      <option key={iface} value={iface}>
                        {iface}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  ROS2 config preset
                  <select
                    value={deployConfig.ros_config}
                    onChange={(event) => setDeployConfig({ ...deployConfig, ros_config: event.target.value })}
                  >
                    {ROS_CONFIG_OPTIONS.map((option) => (
                      <option key={option.label} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                </label>
                {deployConfig.ros_config === 'custom' && (
                  <label>
                    Custom ROS2 / network config
                    <textarea
                      rows={6}
                      value={deployConfig.custom_ros_config}
                      onChange={(event) => setDeployConfig({ ...deployConfig, custom_ros_config: event.target.value })}
                    />
                  </label>
                )}
                <button className="primary-btn" type="submit">
                  Deploy
                </button>
              </form>
              {deployMessage && <div className="deploy-message">{deployMessage}</div>}
              <div className="deploy-hint">
                Example image: <a href="https://ghcr.io/rosmesh/robot-agent:latest" target="_blank" rel="noreferrer">ghcr.io/rosmesh/robot-agent:latest</a>
              </div>
            </section>
          </div>
        </div>
      )}

      <section className="future-grid">
        <div className="feature-card">
          <h4>Fleet visibility</h4>
          <p>Future support for multi-robot dashboards, topology maps, and group status filtering.</p>
        </div>
        <div className="feature-card">
          <h4>Remote management</h4>
          <p>Planned remote command execution, deployments, and system restart workflows from the dashboard.</p>
        </div>
        <div className="feature-card">
          <h4>Security enforcement</h4>
          <p>Upcoming controls for identity, access, and secure ROS2 communications.</p>
        </div>
        <div className="feature-card">
          <h4>Network observability</h4>
          <p>Next phase adds interface, IP, and bandwidth monitoring for each robot.</p>
        </div>
      </section>
    </div>
  );
}

export default App;
