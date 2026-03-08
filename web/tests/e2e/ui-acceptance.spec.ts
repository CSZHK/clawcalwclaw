import { expect, test, type Page, type TestInfo } from '@playwright/test';

const PAIRING_CODE = '246810';
const TOKEN = 'ui-acceptance-token';
const STATUS_FIXTURE = {
  provider: 'openrouter',
  model: 'anthropic/claude-sonnet-4.6',
  temperature: 0.7,
  uptime_seconds: 1234,
  gateway_port: 5555,
  locale: 'en',
  memory_backend: 'sqlite',
  paired: true,
  channels: {
    slack: true,
    telegram: false,
  },
  health: {
    pid: 4242,
    updated_at: '2026-03-08T00:00:00Z',
    uptime_seconds: 1234,
    components: {
      gateway: {
        status: 'ok',
        updated_at: '2026-03-08T00:00:00Z',
        last_ok: '2026-03-08T00:00:00Z',
        last_error: null,
        restart_count: 0,
      },
      runtime: {
        status: 'ok',
        updated_at: '2026-03-08T00:00:00Z',
        last_ok: '2026-03-08T00:00:00Z',
        last_error: null,
        restart_count: 0,
      },
    },
  },
};

const COST_FIXTURE = {
  session_cost_usd: 0.0123,
  daily_cost_usd: 0.0456,
  monthly_cost_usd: 1.2345,
  total_tokens: 12345,
  request_count: 42,
  by_model: {
    'anthropic/claude-sonnet-4.6': {
      model: 'anthropic/claude-sonnet-4.6',
      cost_usd: 1.2345,
      total_tokens: 12345,
      request_count: 42,
    },
  },
};

const TOOLS_FIXTURE = {
  tools: [
    {
      name: 'shell',
      description: 'Execute a deterministic shell command in the workspace.',
      parameters: {
        type: 'object',
        properties: {
          command: { type: 'string' },
        },
      },
    },
    {
      name: 'memory_recall',
      description: 'Recall memory snippets from the deterministic fixture store.',
      parameters: {
        type: 'object',
        properties: {
          query: { type: 'string' },
        },
      },
    },
  ],
};

const CLI_TOOLS_FIXTURE = {
  cli_tools: [
    {
      name: 'rg',
      path: '/usr/bin/rg',
      version: '14.1.0',
      category: 'search',
    },
  ],
};

const browserEvidence = new Map<string, { console: string[]; network: string[] }>();

const INITIAL_CONFIG = [
  'default_provider = "openrouter"',
  'default_model = "anthropic/claude-sonnet-4.6"',
  'default_temperature = 0.7',
  '',
  '[provider]',
  'reasoning_level = "medium"',
  '',
  '[memory]',
  'backend = "sqlite"',
].join('\n');

async function installGatewayFixtures(page: Page): Promise<{ getCurrentConfig: () => string }> {
  let currentConfig = INITIAL_CONFIG;

  await page.addInitScript(() => {
    class MockWebSocket {
      static CONNECTING = 0;
      static OPEN = 1;
      static CLOSING = 2;
      static CLOSED = 3;

      readyState = MockWebSocket.CONNECTING;
      onopen: ((event: Event) => void) | null = null;
      onclose: ((event: CloseEvent) => void) | null = null;
      onerror: ((event: Event) => void) | null = null;
      onmessage: ((event: MessageEvent<string>) => void) | null = null;

      constructor(_url: string, _protocols?: string | string[]) {
        setTimeout(() => {
          this.readyState = MockWebSocket.OPEN;
          this.onopen?.(new Event('open'));
          this.onmessage?.(
            new MessageEvent('message', {
              data: JSON.stringify({
                type: 'history',
                messages: [
                  {
                    role: 'assistant',
                    content: 'Fixture session restored.',
                  },
                ],
              }),
            }),
          );
        }, 25);
      }

      send(payload: string): void {
        const parsed = JSON.parse(payload) as { type?: string; content?: string };
        if (parsed.type !== 'message') {
          return;
        }

        setTimeout(() => {
          this.onmessage?.(
            new MessageEvent('message', {
              data: JSON.stringify({
                type: 'tool_call',
                name: 'shell',
                args: { command: 'echo fixture' },
              }),
            }),
          );
        }, 15);

        setTimeout(() => {
          this.onmessage?.(
            new MessageEvent('message', {
              data: JSON.stringify({
                type: 'tool_result',
                output: 'fixture result',
              }),
            }),
          );
        }, 30);

        setTimeout(() => {
          this.onmessage?.(
            new MessageEvent('message', {
              data: JSON.stringify({
                type: 'done',
                content: `Echo: ${parsed.content ?? ''}`,
              }),
            }),
          );
        }, 45);
      }

      close(): void {
        this.readyState = MockWebSocket.CLOSED;
        this.onclose?.(new CloseEvent('close', { code: 1000, reason: 'mock close' }));
      }
    }

    Object.defineProperty(window, 'WebSocket', {
      configurable: true,
      writable: true,
      value: MockWebSocket,
    });
  });

  await page.route('**/health', async (route) => {
    const url = route.request().url();
    if (url.endsWith('/api/health')) {
      await route.fulfill({ json: { health: STATUS_FIXTURE.health } });
      return;
    }

    await route.fulfill({ json: { require_pairing: true, paired: false } });
  });

  await page.route('**/pair', async (route) => {
    const code = route.request().headers()['x-pairing-code'];
    if (code !== PAIRING_CODE) {
      await route.fulfill({ status: 403, body: 'invalid pairing code' });
      return;
    }

    await route.fulfill({ json: { token: TOKEN } });
  });

  await page.route('**/api/status', async (route) => {
    await route.fulfill({ json: STATUS_FIXTURE });
  });

  await page.route('**/api/cost', async (route) => {
    await route.fulfill({ json: { cost: COST_FIXTURE } });
  });

  await page.route('**/api/tools', async (route) => {
    await route.fulfill({ json: TOOLS_FIXTURE });
  });

  await page.route('**/api/cli-tools', async (route) => {
    await route.fulfill({ json: CLI_TOOLS_FIXTURE });
  });

  await page.route('**/api/config', async (route) => {
    if (route.request().method() === 'PUT') {
      currentConfig = route.request().postData() ?? currentConfig;
      await route.fulfill({ status: 204 });
      return;
    }

    await route.fulfill({ json: { format: 'toml', content: currentConfig } });
  });

  return {
    getCurrentConfig: () => currentConfig,
  };
}

async function attachEvidence(testInfo: TestInfo, name: string, lines: string[]): Promise<void> {
  if (lines.length === 0) {
    return;
  }

  await testInfo.attach(name, {
    body: lines.join('\n'),
    contentType: 'text/plain',
  });
}

async function pairIntoDashboard(page: Page): Promise<void> {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'clawclawclaw' })).toBeVisible();
  await page.getByLabel('Pairing code').fill(PAIRING_CODE);
  await page.getByRole('button', { name: 'Pair' }).click();
  await expect(page.getByText('Provider / Model')).toBeVisible();
  await expect(page.getByText('Cost Overview')).toBeVisible();
}

test.describe('web P0 acceptance gate', () => {
  test.beforeEach(async ({ page }, testInfo) => {
    const state = { console: [] as string[], network: [] as string[] };
    browserEvidence.set(testInfo.testId, state);

    page.on('console', (message) => {
      state.console.push(`[${message.type()}] ${message.text()}`);
    });
    page.on('requestfailed', (request) => {
      state.network.push(`${request.method()} ${request.url()} :: ${request.failure()?.errorText ?? 'request failed'}`);
    });
    page.on('response', (response) => {
      if (response.status() >= 400) {
        state.network.push(`${response.request().method()} ${response.url()} -> ${response.status()}`);
      }
    });
  });

  test.afterEach(async ({}, testInfo) => {
    const state = browserEvidence.get(testInfo.testId);
    if (!state) {
      return;
    }

    if (testInfo.status !== testInfo.expectedStatus) {
      await attachEvidence(testInfo, 'console-log.txt', state.console);
      await attachEvidence(testInfo, 'network-log.txt', state.network);
    }

    browserEvidence.delete(testInfo.testId);
  });

  test('pairs and reaches the dashboard critical path', async ({ page }) => {
    await installGatewayFixtures(page);
    await pairIntoDashboard(page);

    await expect(page.getByText('anthropic/claude-sonnet-4.6')).toBeVisible();
    await expect(page.getByText(':5555')).toBeVisible();
    await expect(page.getByText('Component Health')).toBeVisible();
  });

  test('covers chat tool chain end to end with deterministic websocket frames', async ({ page }) => {
    await installGatewayFixtures(page);
    await pairIntoDashboard(page);

    await page.getByRole('link', { name: 'Agent' }).click();
    await expect(page.getByText('Fixture session restored.')).toBeVisible();
    await expect(page.getByText('Connected')).toBeVisible();

    await page.getByLabel('Chat message').fill('status please');
    await page.getByLabel('Chat message').press('Enter');

    await expect(page.getByText('status please')).toBeVisible();
    await expect(page.getByText('[Tool Call] shell({"command":"echo fixture"})')).toBeVisible();
    await expect(page.getByText('[Tool Result] fixture result')).toBeVisible();
    await expect(page.getByText('Echo: status please')).toBeVisible();
  });

  test('covers tools discovery and config save with deterministic fixtures', async ({ page }) => {
    const fixtures = await installGatewayFixtures(page);
    await pairIntoDashboard(page);

    await page.getByRole('link', { name: 'Tools' }).click();
    await expect(page.getByRole('heading', { name: /Agent Tools \(2\)/ })).toBeVisible();
    await page.getByLabel('Search tools').fill('shell');
    await expect(page.getByRole('button', { name: /shell/i })).toBeVisible();
    await page.getByRole('button', { name: /shell/i }).click();
    await expect(page.getByText('Parameter Schema')).toBeVisible();
    await expect(page.getByText('Execute a deterministic shell command in the workspace.')).toBeVisible();

    await page.getByRole('link', { name: 'Configuration' }).click();
    await expect(page.getByText('Sensitive fields are masked')).toBeVisible();
    await page.getByLabel('Default Provider').fill('openai');
    await page.getByRole('button', { name: 'Save' }).click();
    await expect(page.getByText('Configuration saved successfully.')).toBeVisible();
    await expect.poll(fixtures.getCurrentConfig).toContain('default_provider = "openai"');
  });
});
