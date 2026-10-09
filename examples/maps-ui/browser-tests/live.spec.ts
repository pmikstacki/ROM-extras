import { test, expect } from '@playwright/test';
test.skip(process.env.ROM_LIVE_FIXTURE !== '1', 'Explicit controlled live host required');
test('empty geocoding clears the previous suggestion without a page error', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Connected');
  await page.getByRole('button', { name: 'Query host geocoder', exact: true }).click();
  await expect(page.getByTestId('live-suggestion')).toContainText('Fixture; Credit: Fixture');
  await page.route('**/geocode', route => {
    const body = route.request().postDataJSON();
    return route.fulfill({ json: { generation: body.generation, suggestions: [] } });
  });
  await page.getByRole('button', { name: 'Query host geocoder', exact: true }).click();
  await expect(page.getByTestId('live-suggestion')).toHaveText('No results');
  expect(errors).toEqual([]);
});
test('refused controls show safe feedback and keep revoked disclosure cleared', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Connected');
  await page.route('**/geocode', route => route.fulfill({ status: 429, body: 'private response' }));
  await page.getByRole('button', { name: 'Query host geocoder', exact: true }).click();
  await expect(page.getByTestId('live-suggestion')).toHaveText('Geocoding unavailable');
  await page.route('**/fixture/unknown', route => route.abort());
  await page.getByRole('button', { name: 'Simulate unknown host outcome', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Host control outcome unknown');
  await page.route('**/fixture/revoke', route => route.fulfill({ status: 503, body: 'private response' }));
  await page.getByRole('button', { name: 'Revoke host fixture', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Local access cleared; host revocation unknown');
  await expect(page.locator('.maplibregl-marker')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Query host geocoder', exact: true })).toBeDisabled();
  expect(await page.content()).not.toContain('private response');
  expect(errors).toEqual([]);
});
test('browser reads ROM, selects exact key, queries provider and revokes disclosure', async ({ page }) => {
  const errors: string[] = [];
  const externalRequests: string[] = [];
  const credentialRequests: string[] = [];
  page.on('request', request => {
    const url = new URL(request.url());
    if (['http:', 'https:'].includes(url.protocol) && url.origin !== 'http://127.0.0.1:55467') externalRequests.push(url.origin);
    if (request.postData()?.includes('synthetic-secret')) credentialRequests.push(url.pathname);
  });
  await page.addInitScript(() => {
    Reflect.set(window, '__mapGeolocationCalls', 0);
    const record = () => Reflect.set(window, '__mapGeolocationCalls', Reflect.get(window, '__mapGeolocationCalls') + 1);
    Object.defineProperty(navigator, 'geolocation', { value: {getCurrentPosition:record, watchPosition:record, clearWatch:()=>{}} });
  });
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/?live');
  await expect(page.locator('.maplibregl-marker')).toHaveCount(0);
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Connected');
  await page.getByRole('button', { name: 'Approved place', exact: true }).click();
  await expect(page.getByTestId('selection')).toHaveText('Selected: ę/0001:Straße');
  await page.getByRole('button', { name: 'Query host geocoder', exact: true }).click();
  await expect(page.getByTestId('live-suggestion')).toHaveText('Fixture; Credit: Fixture');
  expect(await page.content()).not.toContain('synthetic-secret');
  expect(await page.evaluate(() => document.cookie)).not.toContain('synthetic-session');
  await page.getByRole('button', { name: 'Revoke host fixture', exact: true }).click();
  await expect(page.locator('.maplibregl-marker')).toHaveCount(0);
  await expect(page.getByTestId('selection')).toHaveText('Selected: none');
  expect(errors).toEqual([]);
  expect(externalRequests).toEqual([]);
  expect(credentialRequests).toEqual([]);
  expect(await page.evaluate(() => Reflect.get(window, '__mapGeolocationCalls'))).toBe(0);
});
test('live host unknown outcome blocks selection until explicit session recovery', async ({ page }) => {
  await page.goto('/?live');
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await expect(page.getByTestId('host-status')).toHaveText('Connected');
  await Promise.all([
    page.waitForResponse(response => response.url().endsWith('/fixture/unknown') && response.status() === 204),
    page.getByRole('button', { name: 'Simulate unknown host outcome', exact: true }).click(),
  ]);
  const marker = page.getByRole('button', { name: 'Approved place', exact: true });
  await marker.click();
  await expect(marker).toBeDisabled();
  await expect(page.getByText('The selection outcome is unknown. Ask the host to recover.', { exact: true })).toBeVisible();
  await expect(page.getByTestId('selection')).toHaveText('Selected: none');
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await expect(marker).toBeEnabled();
  await marker.click();
  await expect(page.getByTestId('selection')).toHaveText('Selected: ę/0001:Straße');
});
test('delayed session response cannot restore revoked browser authority', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  let release!: () => void;
  let ready!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; });
  const responseReady = new Promise<void>(resolve => { ready = resolve; });
  await page.route('**/fixture/session', async route => {
    const response = await route.fetch();
    ready();
    await gate;
    await route.fulfill({ response });
  });
  await page.goto('/?live');
  await page.getByRole('button', { name: 'Connect host fixture', exact: true }).click();
  await responseReady;
  await Promise.all([
    page.waitForResponse(response => response.url().endsWith('/fixture/revoke')),
    page.getByRole('button', { name: 'Revoke host fixture', exact: true }).click(),
  ]);
  const lateReply = page.waitForResponse(response => response.url().endsWith('/fixture/session'));
  release();
  await lateReply;
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  await expect(page.getByTestId('host-status')).toHaveText('Revoked');
  await expect(page.getByRole('button', { name: 'Query host geocoder', exact: true })).toBeDisabled();
  await expect(page.locator('.maplibregl-marker')).toHaveCount(0);
  expect(errors).toEqual([]);
});
