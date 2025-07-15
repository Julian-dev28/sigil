import { describe, expect, it, vi } from 'vitest';
import { ApiError, SigilClient } from './client';
import type { VerifyCredentialResponse } from './client';

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

const sampleVerify: VerifyCredentialResponse = {
  valid: true,
  checks: { signature: true, notExpired: true, notRevoked: true },
  issuer: 'did:web:issuer.example',
  subject: 'did:key:z6MkExample',
  credential: {
    '@context': ['https://www.w3.org/2018/credentials/v1'],
    id: 'urn:uuid:1111',
    type: ['VerifiableCredential', 'EmploymentCredential'],
    issuer: 'did:web:issuer.example',
    issuanceDate: '2024-01-01T00:00:00Z',
    credentialSubject: { id: 'did:key:z6MkExample', role: 'Engineer' },
  },
};

describe('SigilClient', () => {
  it('posts to the verify endpoint with a JSON body and returns typed data', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(sampleVerify));
    const client = new SigilClient({
      baseUrl: 'http://api.test',
      fetch: fetchMock as unknown as typeof fetch,
    });

    const result = await client.verifyCredential({ jwt: 'a.b.c' });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('http://api.test/api/v1/credentials/verify');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string)).toEqual({ jwt: 'a.b.c' });
    expect((init.headers as Record<string, string>)['Content-Type']).toBe(
      'application/json',
    );
    expect(result.valid).toBe(true);
    expect(result.checks.notRevoked).toBe(true);
  });

  it('builds list query params and strips a trailing slash from the base URL', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(jsonResponse({ items: [], total: 0 }));
    const client = new SigilClient({
      baseUrl: 'http://api.test/',
      fetch: fetchMock as unknown as typeof fetch,
    });

    await client.listCredentials(10, 20);

    const [url] = fetchMock.mock.calls[0] as [string];
    expect(url).toBe('http://api.test/api/v1/credentials?limit=10&offset=20');
  });

  it('throws a typed ApiError carrying the server error envelope', async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse(
        { error: { code: 'verification_failed', message: 'bad signature' } },
        400,
      ),
    );
    const client = new SigilClient({
      baseUrl: 'http://api.test',
      fetch: fetchMock as unknown as typeof fetch,
    });

    await expect(client.verifyCredential({ jwt: 'x.y.z' })).rejects.toMatchObject({
      name: 'ApiError',
      status: 400,
      code: 'verification_failed',
      message: 'bad signature',
    });
  });

  it('wraps network failures in an ApiError', async () => {
    const fetchMock = vi.fn().mockRejectedValue(new TypeError('connection refused'));
    const client = new SigilClient({
      baseUrl: 'http://api.test',
      fetch: fetchMock as unknown as typeof fetch,
    });

    const error = await client.health().catch((e: unknown) => e);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).code).toBe('internal');
  });
});
