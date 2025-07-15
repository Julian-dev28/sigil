import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '../test/utils';
import { VerifierPage } from './VerifierPage';
import { apiClient } from '../api/client';
import type { VerifyCredentialResponse } from '../api/client';

const validResult: VerifyCredentialResponse = {
  valid: true,
  checks: { signature: true, notExpired: true, notRevoked: true },
  issuer: 'did:web:issuer.example',
  subject: 'did:key:z6MkHolder',
  credential: {
    '@context': ['https://www.w3.org/2018/credentials/v1'],
    id: 'urn:uuid:abc',
    type: ['VerifiableCredential', 'EmploymentCredential'],
    issuer: 'did:web:issuer.example',
    issuanceDate: '2024-01-01T00:00:00Z',
    credentialSubject: { id: 'did:key:z6MkHolder', role: 'Engineer' },
  },
};

describe('VerifierPage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the form with a labelled JWT input and a verify button', () => {
    renderWithProviders(<VerifierPage />);
    expect(
      screen.getByRole('heading', { name: /verify a credential/i }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText(/jwt-vc/i)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: /verify credential/i }),
    ).toBeInTheDocument();
  });

  it('shows a validation error for a malformed JWT without calling the API', async () => {
    const spy = vi.spyOn(apiClient, 'verifyCredential');
    const user = userEvent.setup();
    renderWithProviders(<VerifierPage />);

    await user.type(screen.getByLabelText(/jwt-vc/i), 'not-a-jwt');
    await user.click(screen.getByRole('button', { name: /verify credential/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent(/compact jwt/i);
    expect(spy).not.toHaveBeenCalled();
  });

  it('verifies a JWT and renders per-check pass badges', async () => {
    const spy = vi
      .spyOn(apiClient, 'verifyCredential')
      .mockResolvedValue(validResult);
    const user = userEvent.setup();
    renderWithProviders(<VerifierPage />);

    await user.type(screen.getByLabelText(/jwt-vc/i), 'aaa.bbb.ccc');
    await user.click(screen.getByRole('button', { name: /verify credential/i }));

    await waitFor(() => {
      expect(screen.getByTestId('verify-result')).toBeInTheDocument();
    });

    expect(spy).toHaveBeenCalledWith({ jwt: 'aaa.bbb.ccc' });
    expect(
      screen.getByRole('heading', { name: /credential is valid/i }),
    ).toBeInTheDocument();
    expect(screen.getByText('Signature valid')).toBeInTheDocument();
    expect(screen.getByText('Not revoked')).toBeInTheDocument();
    expect(screen.getAllByText('Pass')).toHaveLength(3);
  });

  it('surfaces an API error message', async () => {
    vi.spyOn(apiClient, 'verifyCredential').mockRejectedValue(
      Object.assign(new Error('bad signature'), { name: 'ApiError' }),
    );
    const user = userEvent.setup();
    renderWithProviders(<VerifierPage />);

    await user.type(screen.getByLabelText(/jwt-vc/i), 'aaa.bbb.ccc');
    await user.click(screen.getByRole('button', { name: /verify credential/i }));

    expect(await screen.findByRole('alert')).toHaveTextContent(/bad signature/i);
  });
});
