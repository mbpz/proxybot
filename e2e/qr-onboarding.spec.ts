import { test, expect } from "@playwright/test";
import { spawn } from "child_process";
import * as path from "path";
import * as fs from "fs";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const CERT_SERVER_PORT = 19876;
const TEST_LAN_IP = "127.0.0.1";

let server: ReturnType<typeof spawn> | null = null;

test.beforeAll(async () => {
  const helper = path.join(__dirname, "test-helpers", "cert_server_e2e.mjs");
  if (!fs.existsSync(helper)) {
    test.skip(true, `cert_server_e2e.mjs not found at ${helper}`);
    return;
  }
  server = spawn("node", [helper], {
    env: { ...process.env, CERT_SERVER_PORT: String(CERT_SERVER_PORT) },
  });
  await new Promise((r) => setTimeout(r, 500));
});

test.afterAll(async () => {
  if (server) {
    server.kill("SIGTERM");
  }
});

test("GET /ios.mobileconfig returns mobileconfig content-type", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/ios.mobileconfig`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("x-apple-aspen-config");
  const body = await response.text();
  expect(body).toContain('<plist version="1.0">');
  expect(body).toContain("ProxyServer");
});

test("GET /android-setup returns HTML content-type", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/android-setup`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("text/html");
  const body = await response.text();
  expect(body).toContain("ProxyBot Device Setup");
  expect(body).toContain("Android 7+");
});

test("GET /ca.crt still returns the CA cert (regression)", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/ca.crt`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("application/x-x509-ca-cert");
});

test("unknown path returns CA cert (backward compat)", async ({ request }) => {
  const response = await request.get(`http://${TEST_LAN_IP}:${CERT_SERVER_PORT}/some/random/path`);
  expect(response.status()).toBe(200);
  expect(response.headers()["content-type"]).toContain("application/x-x509-ca-cert");
});
