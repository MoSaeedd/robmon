const net = require('net');
const { spawn } = require('child_process');
const path = require('path');

const SERVER_HOST = '127.0.0.1';
const SERVER_PATH = path.resolve(__dirname, '../server.js');
const ADMIN_CREDENTIALS = { username: 'admin', password: 'password' };
const TEST_USER = { username: 'testuser', password: 'testpass', role: 'user' };

function findFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, SERVER_HOST, () => {
      const port = server.address().port;
      server.close(err => {
        if (err) return reject(err);
        resolve(port);
      });
    });
    server.on('error', reject);
  });
}

function buildUrl(port, path) {
  return `http://${SERVER_HOST}:${port}${path}`;
}

function waitForServer(url, timeout = 5000) {
  const start = Date.now();
  return new Promise((resolve, reject) => {
    const tryRequest = () => {
      fetch(url)
        .then(res => (res.ok ? resolve() : retry()))
        .catch(retry);

      function retry() {
        if (Date.now() - start > timeout) {
          reject(new Error('Server did not start in time'));
        } else {
          setTimeout(tryRequest, 100);
        }
      }
    };
    tryRequest();
  });
}

async function runTest() {
  const serverPort = await findFreePort();
  const serverUrl = buildUrl(serverPort, '');
  const dbPath = path.resolve(__dirname, '../data/db.json');
  if (require('fs').existsSync(dbPath)) {
    require('fs').unlinkSync(dbPath);
  }

  console.log('Starting control-plane server on port', serverPort);
  const serverProcess = spawn('node', [SERVER_PATH], {
    cwd: path.dirname(SERVER_PATH),
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PORT: String(serverPort) },
  });

  serverProcess.stdout.on('data', data => process.stdout.write(`[server] ${data}`));
  serverProcess.stderr.on('data', data => process.stderr.write(`[server] ${data}`));

  let cleanupCalled = false;
  const cleanup = () => {
    if (cleanupCalled) return;
    cleanupCalled = true;
    serverProcess.kill();
  };
  process.on('exit', cleanup);
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);

  try {
    await waitForServer(`${serverUrl}/health`);
    console.log('Server is reachable. Logging in as admin...');

    const loginResponse = await fetch(`${serverUrl}/api/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(ADMIN_CREDENTIALS),
    });
    if (!loginResponse.ok) {
      throw new Error(`Admin login failed: ${loginResponse.status}`);
    }
    const { accessToken } = await loginResponse.json();
    if (!accessToken) {
      throw new Error('Admin login response missing accessToken');
    }
    console.log('Admin login succeeded. Creating test user...');

    const createResponse = await fetch(`${serverUrl}/api/users`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${accessToken}`,
      },
      body: JSON.stringify(TEST_USER),
    });
    if (![200, 201].includes(createResponse.status)) {
      const payload = await createResponse.text();
      throw new Error(`Create user failed: ${createResponse.status} ${payload}`);
    }
    console.log('Test user created. Logging in as test user...');

    const userLogin = await fetch(`${serverUrl}/api/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        username: TEST_USER.username,
        password: TEST_USER.password,
      }),
    });
    if (!userLogin.ok) {
      throw new Error(`Test user login failed: ${userLogin.status}`);
    }

    const userLoginPayload = await userLogin.json();
    if (!userLoginPayload.accessToken) {
      throw new Error('Test user login response missing accessToken');
    }

    console.log('Test user login successful. Verifying authenticated endpoint...');
    const serviceResponse = await fetch(`${serverUrl}/api/services`, {
      headers: {
        Authorization: `Bearer ${userLoginPayload.accessToken}`,
      },
    });
    if (!serviceResponse.ok) {
      throw new Error(`Expected successful /api/services access for authenticated user, got ${serviceResponse.status}`);
    }

    console.log('Authenticated check passed. Testing logout...');
    const logoutResponse = await fetch(`${serverUrl}/api/logout`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${userLoginPayload.accessToken}`,
      },
    });
    if (!logoutResponse.ok) {
      throw new Error(`Expected successful logout, got ${logoutResponse.status}`);
    }

    const revokedResponse = await fetch(`${serverUrl}/api/services`, {
      headers: {
        Authorization: `Bearer ${userLoginPayload.accessToken}`,
      },
    });
    if (![401, 403].includes(revokedResponse.status)) {
      throw new Error(`Expected revoked token to be rejected, got ${revokedResponse.status}`);
    }

    console.log('Logout invalidated the token. Test completed successfully.');
  } catch (error) {
    console.error('Test failed:', error);
    process.exitCode = 1;
  } finally {
    cleanup();
  }
}

runTest();
