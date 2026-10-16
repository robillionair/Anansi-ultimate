import { validReadiness, validError } from './validators.mjs';
import { invoke } from '@tauri-apps/api/core';
import type { Readiness } from './generated';
export type { Readiness } from './generated';
export interface Client { readiness(): Promise<Readiness> }
export function decodeReadiness(value: unknown): Readiness {
  if (!validReadiness(value) || value.contractVersion !== 1) throw new Error('The core returned an incompatible response. Update both applications.');
  return value;
}
function failure(value: unknown): Error {
  if (validError(value)) {
    if (value.code === 'UNAUTHORIZED') return new Error('Pairing was rejected. Enter the current session token.');
    if (value.code === 'FORBIDDEN') return new Error('This browser address is not allowed. Open http://127.0.0.1:1420.');
    if (value.code === 'UNSUPPORTED_CONTRACT') return new Error('The core uses a different contract version. Update both applications.');
  }
  return new Error('The core could not complete the connection check. Try again.');
}
export function httpClient(endpoint: string, token: string): Client {
  let url: URL;
  try { url = new URL(endpoint); } catch { throw new Error('Enter a local API address such as http://127.0.0.1:1451.'); }
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port || url.username || url.password || url.pathname !== '/' || url.search || url.hash) throw new Error('Use an HTTP address on 127.0.0.1 with a port and no path.');
  if (!/^[a-f0-9]{64}$/.test(token)) throw new Error('Enter the 64-character token from the current private pairing file.');
  return { async readiness() {
    let response: Response;
    try {
      response = await fetch(`${url.origin}/v1/readiness`, { method: 'POST', credentials: 'omit', cache: 'no-store', redirect: 'error', headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: JSON.stringify({ contractVersion: 1 }), signal: AbortSignal.timeout(5000) });
    } catch { throw new Error('Local core unavailable. Start the API, then retry or pair with its new session.'); }
    let value: unknown;
    try { value = await response.json(); } catch { throw new Error('The core returned an unreadable response. Try again.'); }
    if (!response.ok) throw failure(value);
    return decodeReadiness(value);
  } };
}
export function nativeClient(): Client {
  return { async readiness() {
    let value: unknown;
    try { value = await invoke('get_readiness', { request: { contractVersion: 1 } }); }
    catch (error) { throw failure(error); }
    return decodeReadiness(value);
  } };
}
