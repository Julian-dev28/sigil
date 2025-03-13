/**
 * Typed HTTP client for the Sigil server.
 *
 * The TypeScript types below mirror the shared domain model and the
 * `/api/v1` HTTP surface exactly (see the project build contract). Keep them in
 * sync with `crates/sigil-core` and `crates/sigil-server`.
 */

/* ------------------------------------------------------------------ */
/* Domain model (mirrors W3C VC Data Model 1.1 / JWT-VC)              */
/* ------------------------------------------------------------------ */

/** StatusList2021Entry embedded in a credential's `credentialStatus`. */
export interface CredentialStatus {
  id: string;
  type: 'StatusList2021Entry';
  statusPurpose: string;
  statusListIndex: string;
  statusListCredential: string;
}

/** A subject is a bag of claims plus an optional DID identifier. */
export interface CredentialSubject {
  id?: string;
  [claim: string]: unknown;
}

/** A W3C Verifiable Credential (JSON form embedded in the JWT `vc` claim). */
export interface VerifiableCredential {
  '@context': string[];
  id: string;
  type: string[];
  issuer: string;
  issuanceDate: string;
  expirationDate?: string;
  credentialStatus?: CredentialStatus;
  credentialSubject: CredentialSubject;
}

/* ------------------------------------------------------------------ */
/* Request / response payloads                                        */
/* ------------------------------------------------------------------ */

export interface IssueCredentialRequest {
  subjectId: string;
  type: string;
  claims: Record<string, unknown>;
  expiresInDays?: number;
}

export interface IssueCredentialResponse {
  id: string;
  jwt: string;
  credential: VerifiableCredential;
}

export interface VerifyCredentialRequest {
  jwt: string;
}

export interface VerificationChecks {
  signature: boolean;
  notExpired: boolean;
  notRevoked: boolean;
}

export interface VerifyCredentialResponse {
  valid: boolean;
  checks: VerificationChecks;
  issuer: string;
  subject: string;
  credential: VerifiableCredential;
}

/** Summary row returned by the credential list endpoint. */
export interface CredentialSummary {
  id: string;
  type: string;
  subjectId: string;
  issuedAt: string;
  revoked: boolean;
}

export interface CredentialListResponse {
  items: CredentialSummary[];
  total: number;
}

/** Full stored credential record, including the encoded JWT. */
export interface CredentialRecord {
  id: string;
  type: string;
  subjectId: string;
  issuedAt: string;
  expiresAt?: string;
  revoked: boolean;
  jwt: string;
  credential: VerifiableCredential;
}

export interface RevokeResponse {
  id: string;
  revoked: boolean;
}

export interface HealthResponse {
  status: string;
}

/* ------------------------------------------------------------------ */
/* Error handling                                                     */
/* ------------------------------------------------------------------ */

export type ApiErrorCode =
  | 'invalid_request'
  | 'not_found'
  | 'verification_failed'
  | 'internal';

/** Shape of the server error envelope: `{ "error": { code, message } }`. */
export interface ApiErrorBody {
  error: {
    code: ApiErrorCode | string;
    message: string;
  };
}

/** Thrown for any non-2xx response (or a network/parse failure). */
export class ApiError extends Error {
  readonly status: number;
  readonly code: ApiErrorCode | string;

  constructor(status: number, code: ApiErrorCode | string, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
  }
}

function isApiErrorBody(value: unknown): value is ApiErrorBody {
  if (typeof value !== 'object' || value === null) return false;
  const err = (value as { error?: unknown }).error;
  return (
    typeof err === 'object' &&
    err !== null &&
    typeof (err as { message?: unknown }).message === 'string'
  );
}

/* ------------------------------------------------------------------ */
/* Client                                                             */
/* ------------------------------------------------------------------ */

export interface SigilClientOptions {
  /** Server origin, e.g. `http://localhost:8080`. */
  baseUrl?: string;
  /** Injectable fetch (defaults to global `fetch`); useful for tests. */
  fetch?: typeof fetch;
}

const DEFAULT_BASE_URL = 'http://localhost:8080';

/** Resolve the configured base URL from Vite env, with a safe default. */
export function resolveBaseUrl(): string {
  const fromEnv =
    typeof import.meta !== 'undefined' ? import.meta.env?.VITE_API_URL : undefined;
  return (fromEnv ?? DEFAULT_BASE_URL).replace(/\/+$/, '');
}

export class SigilClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: SigilClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? resolveBaseUrl()).replace(/\/+$/, '');
    // Bind so a passed-in global `fetch` keeps its correct `this`.
    const f = options.fetch ?? fetch;
    this.fetchImpl = f.bind(globalThis);
  }

  private async request<T>(path: string, init?: RequestInit): Promise<T> {
    const url = `${this.baseUrl}${path}`;

    let response: Response;
    try {
      response = await this.fetchImpl(url, {
        ...init,
        headers: {
          Accept: 'application/json',
          ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
          ...init?.headers,
        },
      });
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Network request failed';
      throw new ApiError(0, 'internal', `Could not reach the Sigil server: ${message}`);
    }

    const text = await response.text();
    const body: unknown = text ? safeJsonParse(text) : undefined;

    if (!response.ok) {
      if (isApiErrorBody(body)) {
        throw new ApiError(response.status, body.error.code, body.error.message);
      }
      throw new ApiError(
        response.status,
        'internal',
        `Request to ${path} failed with status ${response.status}`,
      );
    }

    return body as T;
  }

  /* ----- endpoints ----- */

  health(): Promise<HealthResponse> {
    return this.request<HealthResponse>('/health');
  }

  issueCredential(req: IssueCredentialRequest): Promise<IssueCredentialResponse> {
    return this.request<IssueCredentialResponse>('/api/v1/credentials/issue', {
      method: 'POST',
      body: JSON.stringify(req),
    });
  }

  verifyCredential(req: VerifyCredentialRequest): Promise<VerifyCredentialResponse> {
    return this.request<VerifyCredentialResponse>('/api/v1/credentials/verify', {
      method: 'POST',
      body: JSON.stringify(req),
    });
  }

  listCredentials(limit = 50, offset = 0): Promise<CredentialListResponse> {
    const params = new URLSearchParams({
      limit: String(limit),
      offset: String(offset),
    });
    return this.request<CredentialListResponse>(`/api/v1/credentials?${params.toString()}`);
  }

  getCredential(id: string): Promise<CredentialRecord> {
    return this.request<CredentialRecord>(`/api/v1/credentials/${encodeURIComponent(id)}`);
  }

  revokeCredential(id: string): Promise<RevokeResponse> {
    return this.request<RevokeResponse>(
      `/api/v1/credentials/${encodeURIComponent(id)}/revoke`,
      { method: 'POST' },
    );
  }

  getStatusList(
    listId: string | number,
  ): Promise<{ jwt: string; credential: VerifiableCredential }> {
    // The status list endpoint returns the signed StatusList2021 JWT-VC plus
    // its decoded credential object.
    return this.request(`/api/v1/status/${encodeURIComponent(String(listId))}`);
  }
}

function safeJsonParse(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    // Some endpoints (status list JWT) may return a bare string.
    return text;
  }
}

/** Shared default client instance for app code. */
export const apiClient = new SigilClient();
